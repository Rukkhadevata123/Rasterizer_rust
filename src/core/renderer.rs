use crate::core::rasterizer::{TextureSource, TriangleData, VertexRenderData, rasterize_triangle};
use crate::core::shadow_mapping::ShadowMapper; // 添加阴影映射器引用
use crate::geometry::camera::Camera;
use crate::geometry::culling::{is_backface, should_cull_small_triangle};
use crate::geometry::transform::transform_pipeline_batch;
use crate::io::render_settings::RenderSettings;
use crate::material_system::materials::{Material, MaterialView, ModelData, Vertex};
use crate::scene::scene_object::SceneObject;
use crate::scene::scene_utils::Scene;
use atomic_float::AtomicF32;
use nalgebra::{Point2, Point3, Vector3};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

/// 帧缓冲区实现，存储渲染结果
pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    /// 存储正深度值，数值越小表示越近。使用原子类型以支持并行写入。
    pub depth_buffer: Vec<AtomicF32>,
    /// 存储RGB颜色值 [0, 255]，类型为u8。使用原子类型以支持并行写入。
    pub color_buffer: Vec<AtomicU8>,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let num_pixels = width * height;

        // 为深度缓冲区创建原子浮点数向量
        let depth_buffer = (0..num_pixels)
            .map(|_| AtomicF32::new(f32::INFINITY))
            .collect();

        // 使用迭代器创建颜色缓冲区，避免使用vec!宏
        let color_buffer = (0..num_pixels * 3).map(|_| AtomicU8::new(0)).collect();

        FrameBuffer {
            width,
            height,
            depth_buffer,
            color_buffer,
        }
    }

    /// 清除所有缓冲区，并根据配置绘制背景和地面
    pub fn clear(&self, settings: &RenderSettings, skip_ground: bool) {
        // 重置深度缓冲区，使用原子操作避免数据竞争
        self.depth_buffer.par_iter().for_each(|atomic_depth| {
            atomic_depth.store(f32::INFINITY, Ordering::Relaxed);
        });

        // 根据设置绘制背景和地面
        (0..self.height).into_par_iter().for_each(|y| {
            for x in 0..self.width {
                let buffer_index = y * self.width + x;
                let color_index = buffer_index * 3;

                // 计算标准化屏幕坐标 (0-1)
                let t_y = y as f32 / (self.height - 1) as f32;
                let t_x = x as f32 / (self.width - 1) as f32;

                // 1. 首先绘制背景图片（如果启用）- 最低层级
                let mut final_color =
                    if settings.use_background_image && settings.background_image.is_some() {
                        // 使用背景图片
                        let background = settings.background_image.as_ref().unwrap();

                        // 计算纹理坐标 (纹理坐标中y轴方向与屏幕坐标相反)
                        let tex_x = t_x;
                        let tex_y = 1.0 - t_y; // 翻转Y轴以匹配图片坐标系

                        // 获取对应的像素颜色，并转换为所需类型
                        background.sample(tex_x, tex_y).into()
                    } else if settings.enable_gradient_background {
                        // 2. 无背景图片时绘制渐变背景 - 或者渐变背景覆盖图片
                        settings.gradient_top_color_vec * (1.0 - t_y)
                            + settings.gradient_bottom_color_vec * t_y
                    } else {
                        // 默认黑色背景
                        Vector3::new(0.0, 0.0, 0.0)
                    };

                // 如果启用了渐变背景且已经有背景图片，则混合渐变
                if settings.use_background_image
                    && settings.background_image.is_some()
                    && settings.enable_gradient_background
                {
                    // 渐变背景覆盖在图片上
                    let gradient_color = settings.gradient_top_color_vec * (1.0 - t_y)
                        + settings.gradient_bottom_color_vec * t_y;
                    // 简单混合 - 渐变具有半透明效果
                    final_color = final_color * 0.3 + gradient_color * 0.7;
                }

                // 3. 如果启用地面平面且未跳过地面渲染，在下半部分应用地面效果
                if settings.enable_ground_plane && !skip_ground {
                    // 获取地面在世界空间中的Y坐标（高度）
                    let ground_y_world = settings.ground_plane_height;

                    // 如果像素在视图下半部分，考虑应用地面效果
                    let ground_factor = if t_y > 0.5 {
                        // 从像素创建射线，以获得精确的网格线
                        let aspect_ratio = self.width as f32 / self.height as f32;
                        let camera_ref = &Camera::default();
                        let fov_rad = camera_ref.fov_y;
                        let ndc_x = (x as f32 + 0.5) / self.width as f32 * 2.0 - 1.0;
                        let ndc_y = 1.0 - (y as f32 + 0.5) / self.height as f32 * 2.0;

                        let view_x = ndc_x * aspect_ratio * (fov_rad / 2.0).tan();
                        let view_y = ndc_y * (fov_rad / 2.0).tan();
                        let view_dir = Vector3::new(view_x, view_y, -1.0).normalize();

                        let view_to_world = camera_ref
                            .view_matrix
                            .try_inverse()
                            .unwrap_or_else(nalgebra::Matrix4::identity);
                        let world_ray_dir = view_to_world.transform_vector(&view_dir).normalize();
                        let world_ray_origin = camera_ref.position;

                        // 计算与地面平面的交点
                        let ground_normal = Vector3::y();
                        let denominator = ground_normal.dot(&world_ray_dir);

                        if denominator.abs() <= 1e-6 {
                            // 射线几乎与地面平行，使用屏幕空间计算作为后备
                            let ground_influence = (t_y - 0.5) * 2.0;
                            let depth_enhanced = ground_influence.powf(1.2);

                            // 使用屏幕空间坐标创建网格
                            let perspective_factor = 1.0 + (t_y - 0.5) * 3.0;
                            let grid_density = 16.0 * perspective_factor;

                            let grid_x = (t_x * grid_density) % 1.0;
                            let grid_z = (t_y * grid_density * 1.2) % 1.0;

                            let line_width = 0.05 / perspective_factor.max(0.2);
                            let x_distance = (grid_x - 0.5).abs() - (0.5 - line_width);
                            let z_distance = (grid_z - 0.5).abs() - (0.5 - line_width);

                            let is_grid_line = x_distance > 0.0 || z_distance > 0.0;
                            let line_strength = if is_grid_line {
                                0.3 * (1.0 / perspective_factor).min(1.0)
                            } else {
                                0.0
                            };

                            // 边缘淡出
                            let center_x = 0.5;
                            let dx = t_x - center_x;
                            let distance_factor =
                                ((dx * 1.4).powi(2) + (t_y - 0.9).powi(2) * 0.7).sqrt() * 1.3;
                            let edge_softness = 0.2;
                            let edge_factor =
                                (1.0 - distance_factor.min(1.0)).powf(1.0 / edge_softness);

                            edge_factor * depth_enhanced * (1.0 - line_strength * 0.9)
                        } else {
                            // 保留原有的地面渲染逻辑...
                            let t = (Point3::new(0.0, ground_y_world, 0.0) - world_ray_origin)
                                .dot(&ground_normal)
                                / denominator;

                            if t < 0.0 {
                                // 交点在相机后方
                                0.0
                            } else {
                                // 计算交点
                                let intersection = world_ray_origin + t * world_ray_dir;

                                // 网格大小参数
                                let grid_size = 1.0; // 物理单位中的网格大小

                                // 计算网格坐标
                                let grid_x = (intersection.x / grid_size).abs() % 1.0;
                                let grid_z = (intersection.z / grid_size).abs() % 1.0;

                                // 创建更加清晰的网格线
                                let is_grid_line = !(0.05..=0.95).contains(&grid_x)
                                    || !(0.05..=0.95).contains(&grid_z);
                                let grid_factor = if is_grid_line { 0.4 } else { 0.0 };

                                // 计算距离中心的衰减
                                let distance_from_center =
                                    (intersection.x.powi(2) + intersection.z.powi(2)).sqrt();
                                let max_distance = 50.0;
                                let distance_factor =
                                    (distance_from_center / max_distance).min(1.0);

                                // 计算最终的地面因子
                                let ground_influence = (t_y - 0.5) * 2.0;
                                let depth_enhanced = ground_influence.powf(1.2);

                                // 应用所有效果
                                (1.0 - distance_factor).powf(0.5)
                                    * depth_enhanced
                                    * (1.0 - grid_factor * 0.95)
                            }
                        }
                    } else {
                        0.0
                    };

                    if ground_factor > 0.0 {
                        // 混合地面颜色和背景色
                        let mut ground_color = settings.ground_plane_color_vec;

                        // 使用更微妙的颜色变化
                        let t_x_centered = (t_x - 0.5) * 2.0; // -1.0 到 1.0

                        // 添加轻微的色调变化，创建更自然的地面外观
                        ground_color.x *= 1.0 + t_x_centered * 0.05; // 红色分量变化
                        ground_color.y *= 1.0 - t_x_centered.abs() * 0.03; // 绿色分量变化

                        // 远处颜色略微偏蓝，模拟大气透视
                        let distance_from_center =
                            ((t_x - 0.5).powi(2) + (t_y - 0.75).powi(2)).sqrt();
                        let atmospheric_factor = distance_from_center * 0.2;
                        ground_color = ground_color * (1.0 - atmospheric_factor)
                            + Vector3::new(0.6, 0.7, 0.9) * atmospheric_factor;

                        // 应用微弱的反射效果
                        let sky_reflection = settings.gradient_top_color_vec * 0.08;
                        ground_color =
                            ground_color + sky_reflection * (1.0 - (t_y - 0.5) * 1.2).max(0.0);

                        // 使用平滑过渡进行颜色混合
                        final_color =
                            final_color * (1.0 - ground_factor) + ground_color * ground_factor;
                    }
                }

                // 转换为u8颜色并保存到缓冲区
                let color_u8 = crate::material_system::color::linear_rgb_to_u8(
                    &final_color,
                    settings.use_gamma,
                );
                self.color_buffer[color_index].store(color_u8[0], Ordering::Relaxed);
                self.color_buffer[color_index + 1].store(color_u8[1], Ordering::Relaxed);
                self.color_buffer[color_index + 2].store(color_u8[2], Ordering::Relaxed);
            }
        });
    }

    /// 获取颜色缓冲区的字节数据，用于保存图像
    pub fn get_color_buffer_bytes(&self) -> Vec<u8> {
        self.color_buffer
            .iter()
            .map(|atomic_color| atomic_color.load(Ordering::Relaxed))
            .collect()
    }

    /// 获取深度缓冲区的浮点数据，用于保存深度图
    pub fn get_depth_buffer_f32(&self) -> Vec<f32> {
        self.depth_buffer
            .iter()
            .map(|atomic_depth| atomic_depth.load(Ordering::Relaxed))
            .collect()
    }
}

/// 渲染器结构体 - 负责高层次渲染流程
pub struct Renderer {
    pub frame_buffer: FrameBuffer,
    pub shadow_mapper: Option<ShadowMapper>, // 添加阴影映射器
}

impl Renderer {
    /// 创建一个新的渲染器实例
    pub fn new(width: usize, height: usize) -> Self {
        Renderer {
            frame_buffer: FrameBuffer::new(width, height),
            shadow_mapper: None, // 初始无阴影映射器
        }
    }

    /// 渲染一个场景，包含多个模型和对象
    pub fn render_scene(&mut self, scene: &Scene, settings: &RenderSettings) {
        // 初始化阴影映射器
        if settings.use_shadow_mapping {
            // 如果没有阴影映射器或需要重新创建
            if self.shadow_mapper.is_none() {
                self.shadow_mapper = Some(ShadowMapper::new(settings));
            }

            // 获取阴影映射器的可变引用
            if let Some(shadow_mapper) = &mut self.shadow_mapper {
                // 为场景中的光源准备阴影映射
                shadow_mapper.prepare_shadow_maps(scene, settings);

                // 将场景渲染到阴影映射
                shadow_mapper.render_scene_to_shadow_maps(scene);

                println!(
                    "阴影映射已准备完成 - 软化程度: {}",
                    settings.shadow_softness
                );
            }
        } else if self.shadow_mapper.is_some() {
            // 如果禁用了阴影映射，释放资源
            self.shadow_mapper = None;
        }

        // 清除帧缓冲并绘制背景
        // 使用阴影映射时跳过地面绘制，稍后单独处理地面及其阴影
        let skip_ground = settings.use_shadow_mapping && settings.enable_ground_plane;
        self.frame_buffer.clear(settings, skip_ground);

        // 逐个渲染场景中的每个对象
        for object in &scene.objects {
            // 获取该对象引用的模型数据
            if object.model_id < scene.models.len() {
                let model = &scene.models[object.model_id];
                self.render(model, object, &scene.active_camera, settings);
            } else {
                println!("警告：对象引用了无效的模型 ID {}", object.model_id);
            }
        }

        // 绘制地面（如果启用且使用阴影映射）
        if settings.enable_ground_plane && settings.use_shadow_mapping {
            self.render_ground_plane(scene, settings);
        }
    }

    /// 渲染地面平面，支持接收阴影
    fn render_ground_plane(&self, scene: &Scene, settings: &RenderSettings) {
        if !settings.use_shadow_mapping || self.shadow_mapper.is_none() {
            return;
        }

        println!("绘制地面平面并计算阴影...");

        let shadow_mapper = self.shadow_mapper.as_ref().unwrap();
        let ground_y = settings.ground_plane_height;
        let height = self.frame_buffer.height;
        let width = self.frame_buffer.width;

        // 处理下半部分屏幕
        (height / 2..height).into_par_iter().for_each(|y| {
            for x in 0..width {
                let pixel_index = y * width + x;
                let t_y = y as f32 / (height - 1) as f32;
                let t_x = x as f32 / (width - 1) as f32;

                // 跳过上半部分屏幕
                if t_y <= 0.5 {
                    continue;
                }

                // 计算世界坐标和地面因子 - 从FrameBuffer.clear复制相关代码
                let aspect_ratio = width as f32 / height as f32;
                let camera = &scene.active_camera;
                let fov_rad = camera.fov_y;
                let ndc_x = (x as f32 + 0.5) / width as f32 * 2.0 - 1.0;
                let ndc_y = 1.0 - (y as f32 + 0.5) / height as f32 * 2.0;

                let view_x = ndc_x * aspect_ratio * (fov_rad / 2.0).tan();
                let view_y = ndc_y * (fov_rad / 2.0).tan();
                let view_dir = Vector3::new(view_x, view_y, -1.0).normalize();

                let view_to_world = camera
                    .view_matrix
                    .try_inverse()
                    .unwrap_or_else(nalgebra::Matrix4::identity);
                let world_ray_dir = view_to_world.transform_vector(&view_dir).normalize();
                let world_ray_origin = camera.position;

                // 计算与地面的交点
                let ground_normal = Vector3::y();
                let denominator = ground_normal.dot(&world_ray_dir);

                // 检查是否与地面相交
                if denominator.abs() <= 1e-6 {
                    // 射线几乎与地面平行，使用屏幕空间计算
                    // 保留原有代码...
                    continue;
                }

                let t = (Point3::new(0.0, ground_y, 0.0) - world_ray_origin).dot(&ground_normal)
                    / denominator;

                if t < 0.0 {
                    continue; // 交点在相机后方
                }

                // 1. 计算世界空间中的地面交点
                let ground_point = world_ray_origin + t * world_ray_dir;

                // 2. 计算地面颜色 (从FrameBuffer.clear复制)
                let mut ground_color = settings.ground_plane_color_vec;

                // 应用地面颜色变化 (复制原有效果)
                let t_x_centered = (t_x - 0.5) * 2.0;
                ground_color.x *= 1.0 + t_x_centered * 0.05;
                ground_color.y *= 1.0 - t_x_centered.abs() * 0.03;

                // 应用大气透视效果
                let distance_from_center = ((t_x - 0.5).powi(2) + (t_y - 0.75).powi(2)).sqrt();
                let atmospheric_factor = distance_from_center * 0.2;
                ground_color = ground_color * (1.0 - atmospheric_factor)
                    + Vector3::new(0.6, 0.7, 0.9) * atmospheric_factor;

                // 应用反射效果
                let sky_reflection = settings.gradient_top_color_vec * 0.08;
                ground_color = ground_color + sky_reflection * (1.0 - (t_y - 0.5) * 1.2).max(0.0);

                // 3. 计算阴影因子
                let shadow_factor = shadow_mapper.calculate_soft_shadow(&ground_point);

                // 4. 应用阴影到地面颜色
                let shadow_darkness = settings.shadow_darkness * 0.7;
                let min_shadow_value = 1.0 - shadow_darkness;
                let final_shadow_factor = shadow_factor.min(0.95).max(min_shadow_value);

                // 阴影处理
                ground_color *= final_shadow_factor;

                // 5. 获取背景色
                let color_index = pixel_index * 3;
                let mut final_color = Vector3::new(
                    self.frame_buffer.color_buffer[color_index].load(Ordering::Relaxed) as f32
                        / 255.0,
                    self.frame_buffer.color_buffer[color_index + 1].load(Ordering::Relaxed) as f32
                        / 255.0,
                    self.frame_buffer.color_buffer[color_index + 2].load(Ordering::Relaxed) as f32
                        / 255.0,
                );

                // 6. 混合背景和地面
                let ground_factor = 1.0; // 因为我们知道这是地面区域，可以使用距离中心等因素调整
                final_color = final_color * (1.0 - ground_factor) + ground_color * ground_factor;

                // 7. 转换为u8并存储
                let color_u8 = crate::material_system::color::linear_rgb_to_u8(
                    &final_color,
                    settings.use_gamma,
                );

                // 计算地面点的距离
                let distance = (ground_point - world_ray_origin).magnitude();

                // 关键修改: 进行深度测试
                let current_depth =
                    self.frame_buffer.depth_buffer[pixel_index].load(Ordering::Relaxed);

                if distance < current_depth {
                    // 更新深度缓冲区
                    self.frame_buffer.depth_buffer[pixel_index].store(distance, Ordering::Relaxed);

                    // 更新颜色缓冲区
                    let color_index = pixel_index * 3;
                    self.frame_buffer.color_buffer[color_index]
                        .store(color_u8[0], Ordering::Relaxed);
                    self.frame_buffer.color_buffer[color_index + 1]
                        .store(color_u8[1], Ordering::Relaxed);
                    self.frame_buffer.color_buffer[color_index + 2]
                        .store(color_u8[2], Ordering::Relaxed);
                }
            }
        });
    }

    /// 渲染一个场景对象
    pub fn render(
        &self,
        model_data: &ModelData,
        scene_object: &SceneObject,
        camera: &Camera,
        settings: &RenderSettings,
    ) {
        let start_time = Instant::now();

        println!("渲染场景对象...");

        // --- 材质准备 ---
        let material_override = if let Some(material_id) = scene_object.material_id {
            if material_id < model_data.materials.len() {
                Some(&model_data.materials[material_id])
            } else {
                println!("警告: 场景对象指定的材质ID {} 无效", material_id);
                None
            }
        } else {
            None
        };

        // --- 几何变换 ---
        println!("变换顶点...");
        let transform_start_time = Instant::now();

        // 使用优化的几何变换函数
        let (
            all_pixel_coords,
            all_view_coords,
            all_view_normals,
            all_world_coords,
            mesh_vertex_offsets,
        ) = self.transform_geometry(model_data, scene_object, camera, settings);

        let transform_duration = transform_start_time.elapsed();

        // 打印视图空间Z范围（调试用）
        if !all_view_coords.is_empty() {
            println!(
                "视图空间Z范围: [{:.3}, {:.3}]",
                all_view_coords
                    .iter()
                    .map(|p| p.z)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0),
                all_view_coords
                    .iter()
                    .map(|p| p.z)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0)
            );
        }

        // --- 三角形准备 ---
        println!("准备三角形数据...");
        let triangles_to_render = self.prepare_triangles(
            model_data,
            &all_pixel_coords,
            &all_view_coords,
            &all_view_normals,
            &all_world_coords, // 添加世界坐标
            &mesh_vertex_offsets,
            material_override,
            settings,
        );

        // --- 光栅化 ---
        println!("光栅化网格...");
        let raster_start_time = Instant::now();

        // 光栅化三角形 - 使用配置的多线程设置
        if settings.use_multithreading {
            // 并行处理所有三角形
            triangles_to_render.par_iter().for_each(|triangle_data| {
                rasterize_triangle(
                    triangle_data,
                    self.frame_buffer.width,
                    self.frame_buffer.height,
                    &self.frame_buffer.depth_buffer,
                    &self.frame_buffer.color_buffer,
                    settings,
                );
            });
        } else {
            // 串行处理所有三角形
            triangles_to_render.iter().for_each(|triangle_data| {
                rasterize_triangle(
                    triangle_data,
                    self.frame_buffer.width,
                    self.frame_buffer.height,
                    &self.frame_buffer.depth_buffer,
                    &self.frame_buffer.color_buffer,
                    settings,
                );
            });
        }

        // --- 性能统计 ---
        let raster_duration = raster_start_time.elapsed();
        let total_duration = start_time.elapsed();

        println!(
            "渲染完成. 变换: {:?}, 光栅化: {:?}, 总时间: {:?}",
            transform_duration, raster_duration, total_duration
        );
        println!("渲染了 {} 个三角形。", triangles_to_render.len());
    }

    /// 准备三角形数据 - 将几何和材质数据组织为光栅化器可以使用的形式
    #[allow(clippy::too_many_arguments)]
    fn prepare_triangles<'a>(
        &'a self,
        model_data: &'a ModelData,
        all_pixel_coords: &[Point2<f32>],
        all_view_coords: &[Point3<f32>],
        all_view_normals: &[Vector3<f32>],
        all_world_coords: &[Point3<f32>], // 添加世界坐标
        mesh_vertex_offsets: &[usize],
        material_override: Option<&'a Material>,
        settings: &'a RenderSettings,
    ) -> Vec<TriangleData<'a>> {
        // 准备环境光和光源数据
        let ambient_intensity = settings.ambient;
        let ambient_color = settings.ambient_color_vec;
        let lights = &settings.lights; // 使用RenderSettings中的光源

        // 获取阴影映射器引用
        let shadow_mapper = if settings.use_shadow_mapping {
            self.shadow_mapper.as_ref()
        } else {
            None
        };

        // 创建要渲染的三角形列表
        model_data
            .meshes
            .par_iter() // 并行处理网格
            .enumerate()
            .flat_map(|(mesh_idx, mesh)| {
                let vertex_offset = mesh_vertex_offsets[mesh_idx];
                let model_materials = &model_data.materials;

                // 检查是否使用自定义材质
                let material_opt: Option<&Material> =
                    if let Some(material_reference) = material_override {
                        // 使用场景对象覆盖的材质
                        Some(material_reference)
                    } else {
                        // 使用网格默认材质
                        mesh.material_id.and_then(|id| model_materials.get(id))
                    };

                // 处理三角形索引（每3个一组）
                mesh.indices
                    .chunks_exact(3)
                    .enumerate() // 添加枚举以获取面索引
                    .filter_map(move |(face_idx, indices)| {
                        // --- 提取顶点索引 ---
                        let i0 = indices[0] as usize;
                        let i1 = indices[1] as usize;
                        let i2 = indices[2] as usize;

                        // --- 提取顶点数据 ---
                        let v0 = &mesh.vertices[i0];
                        let v1 = &mesh.vertices[i1];
                        let v2 = &mesh.vertices[i2];

                        // --- 计算全局索引 ---
                        let global_i0 = vertex_offset + i0;
                        let global_i1 = vertex_offset + i1;
                        let global_i2 = vertex_offset + i2;

                        // --- 生成面的唯一ID ---
                        // 使用mesh_idx * 1000 + face_idx作为基础，确保不同网格的面索引不会冲突
                        let global_face_index = (mesh_idx * 1000 + face_idx) as u64;

                        // --- 检查索引有效性 ---
                        if global_i0 >= all_pixel_coords.len()
                            || global_i1 >= all_pixel_coords.len()
                            || global_i2 >= all_pixel_coords.len()
                        {
                            println!("警告: 网格 {} 中的顶点索引无效!", mesh_idx);
                            return None;
                        }

                        // --- 获取坐标和法线 ---
                        let pix0 = all_pixel_coords[global_i0];
                        let pix1 = all_pixel_coords[global_i1];
                        let pix2 = all_pixel_coords[global_i2];

                        let view_pos0 = all_view_coords[global_i0];
                        let view_pos1 = all_view_coords[global_i1];
                        let view_pos2 = all_view_coords[global_i2];

                        // 获取世界坐标 (用于阴影计算)
                        let world_pos0 = all_world_coords[global_i0];
                        let world_pos1 = all_world_coords[global_i1];
                        let world_pos2 = all_world_coords[global_i2];

                        // --- 背面剔除 ---
                        if settings.backface_culling
                            && is_backface(&view_pos0, &view_pos1, &view_pos2)
                        {
                            return None; // 剔除背面
                        }

                        // --- 小三角形剔除 ---
                        if settings.cull_small_triangles
                            && should_cull_small_triangle(
                                &pix0,
                                &pix1,
                                &pix2,
                                settings.min_triangle_area,
                            )
                        {
                            return None; // 剔除小三角形
                        }

                        // --- 确定纹理源 ---
                        let texture_source = self.determine_texture_source(
                            settings,
                            material_opt,
                            global_face_index,
                        );

                        // --- 确定基础颜色 ---
                        let base_color =
                            self.determine_base_color(settings, &texture_source, material_opt);

                        // --- 创建材质视图 ---
                        let material_view = material_opt.map(|m| {
                            if settings.use_pbr {
                                MaterialView::PBR(m)
                            } else {
                                MaterialView::BlinnPhong(m)
                            }
                        });

                        // --- 创建顶点渲染数据 ---
                        let vertex_data = [
                            self.create_vertex_render_data(
                                &pix0,
                                view_pos0,
                                world_pos0, // 添加世界坐标
                                v0,
                                global_i0,
                                &texture_source,
                                all_view_normals,
                            ),
                            self.create_vertex_render_data(
                                &pix1,
                                view_pos1,
                                world_pos1, // 添加世界坐标
                                v1,
                                global_i1,
                                &texture_source,
                                all_view_normals,
                            ),
                            self.create_vertex_render_data(
                                &pix2,
                                view_pos2,
                                world_pos2, // 添加世界坐标
                                v2,
                                global_i2,
                                &texture_source,
                                all_view_normals,
                            ),
                        ];

                        // --- 创建TriangleData ---
                        Some(TriangleData {
                            vertices: vertex_data,
                            base_color,
                            texture_source,
                            material_view,
                            lights, // 使用多光源引用
                            ambient_intensity,
                            ambient_color,
                            is_perspective: settings.is_perspective(),
                            shadow_mapper, // 添加阴影映射器引用
                        })
                    })
                    .collect::<Vec<_>>() // 在展平前先收集这个网格的所有三角形
            })
            .collect()
    }

    /// 创建顶点渲染数据
    fn create_vertex_render_data(
        &self,
        pix: &Point2<f32>,
        view_pos: Point3<f32>,
        world_pos: Point3<f32>, // 添加世界坐标
        vertex: &Vertex,
        global_index: usize,
        texture_source: &TextureSource,
        all_view_normals: &[Vector3<f32>],
    ) -> VertexRenderData {
        VertexRenderData {
            pix: Point2::new(pix.x, pix.y),
            z_view: view_pos.z,
            texcoord: if matches!(texture_source, TextureSource::Image(_)) {
                Some(vertex.texcoord)
            } else {
                None
            },
            normal_view: Some(all_view_normals[global_index]),
            position_view: Some(view_pos),
            position_world: Some(world_pos), // 存储世界坐标
        }
    }

    /// 确定纹理来源
    fn determine_texture_source<'a>(
        &self,
        settings: &RenderSettings,
        material_opt: Option<&'a Material>,
        global_face_index: u64,
    ) -> TextureSource<'a> {
        // 首先判断是否启用纹理功能
        if !settings.use_texture {
            // 未启用纹理功能时：
            // 检查是否启用面颜色模式，即使未启用纹理也可以应用面颜色
            if settings.colorize {
                return TextureSource::FaceColor(global_face_index);
            }
            return TextureSource::None;
        }

        // 已启用纹理功能时，遵循优先级：PNG材质 > 面随机颜色 > SolidColor

        // 1. 优先使用PNG材质（如果存在）
        if let Some(tex) = material_opt.and_then(|m| m.texture.as_ref()) {
            return TextureSource::Image(tex);
        }

        // 2. 其次检查是否应用面随机颜色
        if settings.colorize {
            return TextureSource::FaceColor(global_face_index);
        }

        // 3. 最后使用材质颜色作为固体纹理
        let color = material_opt.map_or_else(
            || Vector3::new(0.7, 0.7, 0.7), // 默认灰色
            |m| m.diffuse(),                // 使用材质的漫反射颜色
        );
        TextureSource::SolidColor(color)
    }

    /// 确定基础颜色
    fn determine_base_color(
        &self,
        _settings: &RenderSettings,     // 添加下划线避免未使用警告
        texture_source: &TextureSource, // 改为借用而非所有权转移
        material_opt: Option<&Material>,
    ) -> Vector3<f32> {
        match texture_source {
            TextureSource::FaceColor(_) => {
                // 对于面随机颜色，使用白色作为基础，实际颜色会在光栅化器中生成
                Vector3::new(1.0, 1.0, 1.0)
            }
            TextureSource::None | TextureSource::Image(_) | TextureSource::SolidColor(_) => {
                // 使用材质的漫反射颜色
                material_opt.map_or_else(
                    || Vector3::new(0.7, 0.7, 0.7), // 默认灰色
                    |m| m.diffuse(),                // 使用材质的漫反射颜色
                )
            }
        }
    }

    /// 获取ModelData中的总顶点数
    fn estimate_vertex_count(model_data: &ModelData) -> usize {
        model_data
            .meshes
            .iter()
            .map(|mesh| mesh.vertices.len())
            .sum()
    }

    /// 执行几何变换，现在也返回世界空间坐标
    fn transform_geometry(
        &self,
        model_data: &ModelData,
        scene_object: &SceneObject,
        camera: &Camera,
        settings: &RenderSettings,
    ) -> (
        Vec<Point2<f32>>,  // 屏幕坐标
        Vec<Point3<f32>>,  // 视图空间坐标
        Vec<Vector3<f32>>, // 视图空间法线
        Vec<Point3<f32>>,  // 世界空间坐标 (新增)
        Vec<usize>,        // 网格顶点偏移量
    ) {
        // 获取对象的模型矩阵
        let model_matrix = scene_object.transform;

        // 收集所有顶点和法线以进行批量变换
        let vertex_count = Self::estimate_vertex_count(model_data);
        let mut all_vertices_model: Vec<Point3<f32>> = Vec::with_capacity(vertex_count);
        let mut all_normals_model: Vec<Vector3<f32>> = Vec::with_capacity(vertex_count);
        let mut mesh_vertex_offsets: Vec<usize> = vec![0];

        for mesh in &model_data.meshes {
            all_vertices_model.extend(mesh.vertices.iter().map(|v| v.position));
            all_normals_model.extend(mesh.vertices.iter().map(|v| v.normal));
            mesh_vertex_offsets.push(all_vertices_model.len());
        }

        // 获取相机变换矩阵
        let view_matrix = camera.get_view_matrix();
        // 使用projection字段来确定投影类型
        let projection_type = if settings.is_perspective() {
            "perspective"
        } else {
            "orthographic"
        };
        let projection_matrix = camera.get_projection_matrix(projection_type);

        // 计算世界空间坐标 (用于阴影计算)
        let all_world_coords: Vec<Point3<f32>> = all_vertices_model
            .iter()
            .map(|vertex| model_matrix.transform_point(vertex))
            .collect();

        // 调用 transform.rs 中的变换函数获取屏幕和视图空间坐标
        let (all_pixel_coords, all_view_coords, all_view_normals) = transform_pipeline_batch(
            &all_vertices_model,
            &all_normals_model,
            &model_matrix,
            &view_matrix,
            &projection_matrix,
            self.frame_buffer.width,
            self.frame_buffer.height,
        );

        (
            all_pixel_coords,
            all_view_coords,
            all_view_normals,
            all_world_coords, // 返回世界空间坐标
            mesh_vertex_offsets,
        )
    }
}
