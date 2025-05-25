use std::fs;
use std::time::Instant;

// 声明模块
mod core;
mod geometry;
mod io;
mod material_system;
mod scene;
mod ui;
mod utils;

// 导入语句
use core::renderer::Renderer;
use io::model_loader::ModelLoader;
use io::render_settings::RenderSettings;
use io::simple_cli::SimpleCli;
use utils::render_utils::{render_single_frame, run_animation_loop};

// 🔥 **修复：正确的类型导入**
use crate::scene::scene_utils::Scene;
use crate::material_system::materials::ModelData;

fn main() -> Result<(), String> {
    // 🔥 **使用SimpleCli处理所有命令行参数**
    match SimpleCli::process()? {
        Some(settings) => {
            // 🔥 **CLI返回了配置，启动应用程序逻辑**
            run_application(settings)
        }
        None => {
            // 🔥 **CLI已经处理完成（如生成示例、验证配置等），直接退出**
            Ok(())
        }
    }
}

/// 🔥 **核心应用逻辑** - 根据配置决定启动GUI或直接渲染
fn run_application(mut settings: RenderSettings) -> Result<(), String> {
    let start_time = Instant::now();

    // 🔥 **确保光源已初始化**
    if settings.lights.is_empty() {
        settings.initialize_lights();
    }

    // 🔥 **检查是否应该启动GUI模式**
    if settings.should_start_gui() {
        println!("🎨 启动GUI模式...");
        println!("💡 配置摘要:");
        println!("{}", settings.get_config_summary());
        
        // 检查潜在问题并给出建议
        let issues = settings.check_potential_issues();
        if !issues.is_empty() {
            println!("⚠️  潜在问题提醒:");
            for issue in issues {
                println!("   • {}", issue);
            }
            println!();
        }

        if let Err(err) = ui::start_gui(settings) {
            return Err(format!("❌ GUI启动失败: {}", err));
        }
        return Ok(());
    }

    // 🔥 **命令行渲染模式**
    println!("🚀 启动命令行渲染模式...");
    
    // 验证配置
    settings.validate()
        .map_err(|e| format!("❌ 配置验证失败: {}", e))?;

    // 获取OBJ文件路径（此时已通过验证，可以安全unwrap）
    let obj_path = settings.obj.as_ref().unwrap();

    // 🔥 **前期准备工作**
    prepare_rendering_environment(&settings)?;

    // 🔥 **显示渲染信息**
    print_rendering_info(&settings);

    // 🔥 **加载模型和创建场景**
    let (mut scene, model_data) = load_scene_with_feedback(obj_path, &settings)?;

    // 🔥 **创建渲染器**
    let mut renderer = create_renderer_with_feedback(&settings);

    // 🔥 **执行渲染**
    execute_rendering(&mut scene, &mut renderer, &settings)?;

    println!("✅ 总执行时间：{:?}", start_time.elapsed());
    Ok(())
}

/// 🔥 **准备渲染环境**
fn prepare_rendering_environment(settings: &RenderSettings) -> Result<(), String> {
    // 确保输出目录存在
    println!("📁 准备输出目录: {}", settings.output_dir);
    fs::create_dir_all(&settings.output_dir)
        .map_err(|e| format!("❌ 创建输出目录 '{}' 失败：{}", settings.output_dir, e))?;

    // 🔥 **验证所有资源**
    println!("🔍 验证资源文件...");
    if let Err(e) = ModelLoader::validate_resources(settings) {
        println!("⚠️  资源验证警告: {}", e);
        println!("💡 继续执行渲染，部分功能可能受限");
    } else {
        println!("✅ 所有资源验证通过");
    }

    Ok(())
}

/// 🔥 **显示详细的渲染信息**
fn print_rendering_info(settings: &RenderSettings) {
    println!("\n🎨 ========== 渲染配置信息 ==========");
    
    // 基础设置
    println!("📐 输出尺寸: {}x{} 像素", settings.width, settings.height);
    println!("📊 投影模式: {}", settings.projection);
    println!("📦 模型文件: {}", settings.obj.as_ref().unwrap());
    
    // 材质和光照
    println!("🎨 着色模型: {}", settings.get_lighting_description());
    if settings.use_lighting {
        println!("💡 光照系统: 启用");
        println!("   🌟 环境光强度: {:.2}", settings.ambient);
        println!("   🔦 光源数量: {} 个", settings.lights.len());
        println!("   ✨ 启用光源: {} 个", settings.get_enabled_light_count());
        
        // 显示每个光源的简要信息
        for (i, light) in settings.lights.iter().enumerate() {
            match light {
                crate::material_system::light::Light::Directional { enabled, intensity, .. } => {
                    println!("     光源{}: 方向光 (强度: {:.2}, {})", 
                        i + 1, intensity, if *enabled { "启用" } else { "禁用" });
                }
                crate::material_system::light::Light::Point { enabled, intensity, .. } => {
                    println!("     光源{}: 点光源 (强度: {:.2}, {})", 
                        i + 1, intensity, if *enabled { "启用" } else { "禁用" });
                }
            }
        }
    } else {
        println!("💡 光照系统: 禁用");
    }
    
    // 纹理设置
    if settings.use_texture {
        if let Some(texture) = &settings.texture {
            println!("🖼️  纹理贴图: 指定文件 ({})", texture);
        } else {
            println!("🖼️  纹理贴图: 使用MTL文件设置");
        }
    } else {
        println!("🖼️  纹理贴图: 禁用");
    }
    
    // 背景设置
    if settings.use_background_image {
        if let Some(bg_path) = &settings.background_image_path {
            println!("🌄 背景图片: {}", bg_path);
        }
    }
    if settings.enable_gradient_background {
        println!("🌈 渐变背景: 启用");
    }
    if settings.enable_ground_plane {
        println!("🏔️  地面平面: 启用 (高度: {:.2})", settings.ground_plane_height);
    }
    
    // 渲染优化
    println!("⚡ 性能优化:");
    println!("   🧵 多线程渲染: {}", if settings.use_multithreading { "启用" } else { "禁用" });
    println!("   🔍 背面剔除: {}", if settings.backface_culling { "启用" } else { "禁用" });
    println!("   📏 小三角形剔除: {}", if settings.cull_small_triangles { "启用" } else { "禁用" });
    println!("   💾 Z缓冲: {}", if settings.use_zbuffer { "启用" } else { "禁用" });
    
    // 输出设置
    println!("📁 输出设置:");
    println!("   📂 输出目录: {}", settings.output_dir);
    println!("   📄 文件名: {}.png", settings.output);
    if settings.save_depth {
        println!("   🗺️  深度图: {}_depth.png", settings.output);
    }
    
    // 动画设置
    if settings.animate {
        println!("🎬 动画模式: 启用");
        println!("   📺 帧率: {} fps", settings.fps);
        println!("   🔄 旋转圈数: {:.1}", settings.rotation_cycles);
        println!("   ⚡ 旋转速度: {:.1}x", settings.rotation_speed);
        println!("   🎯 动画类型: {:?}", settings.animation_type);
    } else {
        println!("🎬 动画模式: 单帧渲染");
    }
    
    println!("=====================================\n");
}

/// 🔥 **加载场景并提供反馈**
fn load_scene_with_feedback(obj_path: &str, settings: &RenderSettings) -> Result<(Scene, ModelData), String> {
    println!("📦 正在加载模型文件...");
    println!("   📄 文件路径: {}", obj_path);
    
    let load_start = Instant::now();
    let result = ModelLoader::load_and_create_scene(obj_path, settings)?;
    let load_time = load_start.elapsed();
    
    println!("✅ 模型加载完成 (耗时: {:?})", load_time);
    
    // 🔥 **修复：通过场景统计获取信息**
    let (ref scene, ref model_data) = result;
    let scene_stats = scene.get_scene_stats();
    
    println!("📊 场景统计:");
    println!("   🔺 三角形数量: {}", scene_stats.triangle_count);
    println!("   📍 顶点数量: {}", scene_stats.vertex_count);
    println!("   🎨 材质数量: {}", scene_stats.material_count);
    println!("   📦 网格数量: {}", scene_stats.mesh_count);
    
    Ok(result)
}

/// 🔥 **创建渲染器并提供反馈**
fn create_renderer_with_feedback(settings: &RenderSettings) -> Renderer {
    println!("🔧 正在初始化渲染器...");
    println!("   📐 分辨率: {}x{}", settings.width, settings.height);
    
    let init_start = Instant::now();
    let renderer = Renderer::new(settings.width, settings.height);
    let init_time = init_start.elapsed();
    
    println!("✅ 渲染器初始化完成 (耗时: {:?})", init_time);
    
    // 计算缓冲区大小
    let buffer_size = settings.width * settings.height;
    let memory_mb = (buffer_size * 4 * 3) as f64 / (1024.0 * 1024.0); // RGB + Z缓冲
    println!("💾 缓冲区内存占用: {:.1} MB", memory_mb);
    
    renderer
}

/// 🔥 **执行渲染流程**
fn execute_rendering(
    scene: &mut Scene, 
    renderer: &mut Renderer, 
    settings: &RenderSettings
) -> Result<(), String> {
    
    if settings.animate {
        println!("🎬 开始动画渲染...");
        println!("   📺 目标帧率: {} fps", settings.fps);
        println!("   🔄 总圈数: {:.1}", settings.rotation_cycles);
        
        let animation_start = Instant::now();
        run_animation_loop(scene, renderer, settings)?;
        let animation_time = animation_start.elapsed();
        
        println!("✅ 动画渲染完成!");
        println!("   ⏱️  总耗时: {:?}", animation_time);
        
        // 计算动画统计
        let total_frames = (settings.fps as f32 * settings.rotation_cycles * 360.0 / (settings.rotation_speed * 360.0 / settings.rotation_cycles)) as usize;
        if total_frames > 0 {
            let avg_frame_time = animation_time.as_secs_f64() / total_frames as f64;
            println!("   📊 平均帧时间: {:.3}s", avg_frame_time);
            println!("   🎯 实际帧率: {:.1} fps", 1.0 / avg_frame_time);
        }
        
    } else {
        println!("🖼️  开始单帧渲染...");
        
        let render_start = Instant::now();
        render_single_frame(scene, renderer, settings, &settings.output)?;
        let render_time = render_start.elapsed();
        
        println!("✅ 单帧渲染完成!");
        println!("   ⏱️  渲染耗时: {:?}", render_time);
        
        // 计算渲染统计
        let total_pixels = settings.width * settings.height;
        let pixels_per_second = total_pixels as f64 / render_time.as_secs_f64();
        println!("   📊 渲染速度: {:.0} 像素/秒", pixels_per_second);
        
        // 显示输出文件信息
        println!("📁 输出文件:");
        println!("   🖼️  主图像: {}/{}.png", settings.output_dir, settings.output);
        if settings.save_depth {
            println!("   🗺️  深度图: {}/{}_depth.png", settings.output_dir, settings.output);
        }
    }
    
    Ok(())
}