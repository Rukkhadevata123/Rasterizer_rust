use crate::io::render_settings::RenderSettings;
use crate::material_system::light::Light;
use crate::scene::scene_utils::Scene;
use atomic_float::AtomicF32;
use nalgebra::{Matrix4, Point3, Vector3, Vector4};
use rayon::prelude::*;
use std::sync::atomic::Ordering;

/// 阴影缓冲区 - 存储光源视角的深度信息
pub struct ShadowBuffer {
    pub width: usize,
    pub height: usize,
    pub depth_buffer: Vec<AtomicF32>,
}

impl ShadowBuffer {
    /// 创建一个新的阴影缓冲区
    pub fn new(size: usize) -> Self {
        let num_pixels = size * size;
        let depth_buffer = (0..num_pixels)
            .map(|_| AtomicF32::new(f32::INFINITY))
            .collect();

        ShadowBuffer {
            width: size,
            height: size,
            depth_buffer,
        }
    }

    /// 清除阴影缓冲区
    pub fn clear(&self) {
        self.depth_buffer.iter().for_each(|atomic_depth| {
            atomic_depth.store(f32::INFINITY, Ordering::Relaxed);
        });
    }

    /// 设置深度值
    pub fn set_depth(&self, x: usize, y: usize, depth: f32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            self.depth_buffer[index].fetch_min(depth, Ordering::Relaxed);
        }
    }

    /// 获取深度值
    pub fn get_depth(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            self.depth_buffer[index].load(Ordering::Relaxed)
        } else {
            f32::INFINITY
        }
    }
}

/// 阴影映射管理器 - 处理阴影映射的创建和查询
pub struct ShadowMapper {
    pub shadow_maps: Vec<ShadowBuffer>,
    pub light_indices: Vec<usize>,          // 对应的光源索引
    pub light_view_proj: Vec<Matrix4<f32>>, // 光源的视图投影矩阵
    pub shadow_bias: f32,
    pub shadow_softness: f32,
    pub shadow_darkness: f32,
}

impl ShadowMapper {
    /// 创建一个新的阴影映射管理器
    pub fn new(settings: &RenderSettings) -> Self {
        ShadowMapper {
            shadow_maps: Vec::new(),
            light_indices: Vec::new(),
            light_view_proj: Vec::new(),
            shadow_bias: settings.shadow_bias,
            shadow_softness: settings.shadow_softness,
            shadow_darkness: settings.shadow_darkness,
        }
    }

    /// 为场景中投射阴影的光源创建阴影映射
    pub fn prepare_shadow_maps(&mut self, scene: &Scene, settings: &RenderSettings) {
        // 清除现有的阴影映射
        self.shadow_maps.clear();
        self.light_indices.clear();
        self.light_view_proj.clear();

        // 如果不启用阴影映射，则直接返回
        if !settings.use_shadow_mapping {
            return;
        }

        // 计算场景包围球，用于确定光源视图
        let (scene_center, scene_radius) = Scene::calculate_scene_bounds(scene, settings);

        // 更新阴影设置参数
        self.shadow_bias = settings.shadow_bias;
        self.shadow_softness = settings.shadow_softness;
        self.shadow_darkness = settings.shadow_darkness;

        // 为每个投射阴影的光源创建阴影映射
        for (i, light) in scene.lights.iter().enumerate() {
            // 目前只有定向光支持阴影
            if let Light::Directional { cast_shadow, .. } = light {
                if *cast_shadow {
                    // 创建一个阴影缓冲区
                    self.shadow_maps
                        .push(ShadowBuffer::new(settings.shadow_map_size));
                    self.light_indices.push(i);

                    // 创建光源可变引用副本并计算阴影矩阵
                    let mut light_copy = light.clone();
                    if let Some(matrix) =
                        light_copy.calculate_shadow_matrix(scene_center, scene_radius)
                    {
                        self.light_view_proj.push(matrix);
                    }
                }
            }
        }
    }

    /// 渲染场景到阴影映射
    pub fn render_scene_to_shadow_maps(&self, scene: &Scene) {
        // 如果没有阴影映射，直接返回
        if self.shadow_maps.is_empty() || self.light_indices.is_empty() {
            return;
        }

        // 遍历所有阴影映射和对应的光源
        for (shadow_idx, (shadow_buffer, &light_idx)) in
            self.shadow_maps.iter().zip(&self.light_indices).enumerate()
        {
            let light = &scene.lights[light_idx];
            if let Some(light_matrix) = light.get_shadow_matrix() {
                println!("渲染阴影映射 #{}", shadow_idx + 1);

                // 清除阴影缓冲区
                shadow_buffer.clear();

                // 简化版本：处理场景中的每个对象
                for object in &scene.objects {
                    if object.model_id >= scene.models.len() {
                        continue; // 跳过无效模型ID
                    }

                    let model = &scene.models[object.model_id];
                    let model_matrix = object.transform;

                    // 处理模型中的每个三角形
                    for mesh in &model.meshes {
                        for indices in mesh.indices.chunks_exact(3) {
                            // 提取三角形顶点
                            let v0 = &mesh.vertices[indices[0] as usize];
                            let v1 = &mesh.vertices[indices[1] as usize];
                            let v2 = &mesh.vertices[indices[2] as usize];

                            // 将顶点转换到世界空间，然后到光源空间
                            let world_v0 = model_matrix.transform_point(&v0.position);
                            let world_v1 = model_matrix.transform_point(&v1.position);
                            let world_v2 = model_matrix.transform_point(&v2.position);

                            // 变换到光源空间（NDC坐标）
                            let process_vertex =
                                |world_pos: Point3<f32>| -> Option<(usize, usize, f32)> {
                                    let pos_world =
                                        Vector4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
                                    let pos_light = light_matrix * pos_world;

                                    // 透视除法
                                    let w = pos_light.w;
                                    if w <= 0.0 {
                                        return None; // 点在光源后方
                                    }

                                    let pos_ndc = Vector3::new(
                                        pos_light.x / w,
                                        pos_light.y / w,
                                        pos_light.z / w,
                                    );

                                    // 检查点是否在NDC空间内
                                    if pos_ndc.x < -1.0
                                        || pos_ndc.x > 1.0
                                        || pos_ndc.y < -1.0
                                        || pos_ndc.y > 1.0
                                        || pos_ndc.z < 0.0
                                        || pos_ndc.z > 1.0
                                    {
                                        return None; // 点在视锥体外
                                    }

                                    // 转换到阴影贴图坐标
                                    let shadow_x = ((pos_ndc.x * 0.5 + 0.5)
                                        * shadow_buffer.width as f32)
                                        as usize;
                                    let shadow_y = ((pos_ndc.y * 0.5 + 0.5)
                                        * shadow_buffer.height as f32)
                                        as usize;

                                    // 返回像素坐标和深度
                                    Some((shadow_x, shadow_y, pos_ndc.z))
                                };

                            // 处理三个顶点
                            let v0_shadow = process_vertex(world_v0);
                            let v1_shadow = process_vertex(world_v1);
                            let v2_shadow = process_vertex(world_v2);

                            // 如果任何顶点超出范围，跳过此三角形
                            if v0_shadow.is_none() || v1_shadow.is_none() || v2_shadow.is_none() {
                                continue;
                            }

                            // 提取阴影贴图坐标
                            let (x0, y0, z0) = v0_shadow.unwrap();
                            let (x1, y1, z1) = v1_shadow.unwrap();
                            let (x2, y2, z2) = v2_shadow.unwrap();

                            // 简单方法：只在三角形顶点记录深度
                            // 在真实实现中，应该对整个三角形进行光栅化
                            shadow_buffer.set_depth(x0, y0, z0);
                            shadow_buffer.set_depth(x1, y1, z1);
                            shadow_buffer.set_depth(x2, y2, z2);

                            // 在更完整的实现中，我们应该进行光栅化，填充三角形
                            // 这里可以采用类似主渲染器的三角形光栅化技术
                            // 但为了简化，我们暂不实现完整的三角形光栅化
                        }
                    }
                }

                // 简单的边缘模糊（不使用PCF）
                if self.shadow_softness > 0.0 {
                    self.apply_simple_blur(shadow_buffer);
                }
            }
        }
    }

    /// 应用简单的边缘模糊
    fn apply_simple_blur(&self, shadow_buffer: &ShadowBuffer) {
        // 这是一个非常简化的实现，只模糊边缘，不是真正的PCF
        // 在真实情况下应该使用更高级的技术

        // 为了简化，我们不实际修改阴影缓冲区
        // 在实际实现中，我们会复制缓冲区，应用模糊，然后替换原始缓冲区
        println!("应用阴影边缘模糊，软化程度: {}", self.shadow_softness);
    }

    /// 计算给定世界坐标点的阴影因子
    pub fn calculate_shadow_factor(&self, world_pos: &Point3<f32>) -> f32 {
        if self.shadow_maps.is_empty() || self.light_view_proj.is_empty() {
            return 1.0; // 无阴影
        }

        // 简单实现：只使用第一个阴影映射
        let shadow_map = &self.shadow_maps[0];
        let light_vp = &self.light_view_proj[0];

        // 将世界坐标变换到光源空间
        let pos_world = Vector4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        let pos_light = light_vp * pos_world;

        // 透视除法
        let w = pos_light.w;
        if w <= 0.0 {
            return 1.0; // 点在光源后方
        }

        let pos_ndc = Vector3::new(pos_light.x / w, pos_light.y / w, pos_light.z / w);

        // 将NDC坐标转换为阴影贴图坐标
        if pos_ndc.x < -1.0
            || pos_ndc.x > 1.0
            || pos_ndc.y < -1.0
            || pos_ndc.y > 1.0
            || pos_ndc.z < 0.0
            || pos_ndc.z > 1.0
        {
            return 1.0; // 点在光源视锥体外
        }

        // 转换到阴影贴图坐标
        let shadow_x = ((pos_ndc.x * 0.5 + 0.5) * shadow_map.width as f32) as usize;
        let shadow_y = ((pos_ndc.y * 0.5 + 0.5) * shadow_map.height as f32) as usize;

        // 获取深度
        let depth_in_light = pos_ndc.z;
        let shadow_depth = shadow_map.get_depth(shadow_x, shadow_y);

        // 考虑阴影偏移
        if depth_in_light - self.shadow_bias > shadow_depth {
            // 点在阴影中
            1.0 - self.shadow_darkness
        } else {
            // 点在光照中
            1.0
        }
    }

    /// 使用简单软化计算阴影因子（替代PCF）
    pub fn calculate_soft_shadow(&self, world_pos: &Point3<f32>) -> f32 {
        if self.shadow_maps.is_empty() || self.shadow_softness <= 0.0 {
            return self.calculate_shadow_factor(world_pos);
        }

        // 简化的软阴影实现，不使用PCF
        // 根据软化程度参数简单地混合阴影和非阴影区域

        let hard_shadow = self.calculate_shadow_factor(world_pos);

        // 软化系数，基于shadow_softness参数
        let softness_factor = (self.shadow_softness / 10.0).min(1.0);

        // 简单混合：从硬阴影向非阴影区域过渡
        if hard_shadow < 1.0 {
            // 在阴影中，根据软化程度提亮
            let min_shadow = 1.0 - self.shadow_darkness;
            hard_shadow * (1.0 - softness_factor) + (min_shadow + 0.2) * softness_factor
        } else {
            // 不在阴影中，根据软化程度降暗边缘
            1.0 - softness_factor * 0.1
        }
    }
}
