use clap::Parser;
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
use io::render_settings::RenderSettings;
use io::resource_loader::ResourceLoader; // 引入新的资源加载器
use utils::render_utils::{render_single_frame, run_animation_loop};

fn main() -> Result<(), String> {
    // 解析命令行参数
    let mut settings = RenderSettings::parse();

    // 判断是否应该启动GUI模式
    if settings.should_start_gui() {
        println!("启动GUI模式...");
        if let Err(err) = ui::start_gui(settings) {
            return Err(format!("GUI启动失败: {}", err));
        }
        return Ok(());
    }

    // 如果代码执行到这里，说明有OBJ文件路径，进入命令行渲染模式
    let start_time = Instant::now();

    // 获取OBJ文件路径（此时我们确定obj是Some，所以可以安全unwrap）
    let obj_path = settings.obj.as_ref().unwrap();

    // 确保输出目录存在
    fs::create_dir_all(&settings.output_dir)
        .map_err(|e| format!("创建输出目录 '{}' 失败：{}", settings.output_dir, e))?;

    // 使用ResourceLoader加载模型和创建场景
    let (mut scene, _model_data) =
        ResourceLoader::load_model_and_create_scene(obj_path, &settings)?;

    // 使用ResourceLoader加载背景图片
    if let Err(e) = ResourceLoader::load_background_image_if_enabled(&mut settings) {
        println!("背景图片加载问题: {}", e);
        // 继续执行，不中断渲染过程
    }

    // --- 创建渲染器 ---
    let renderer = Renderer::new(settings.width, settings.height);

    // --- 渲染动画或单帧 ---
    if settings.animate {
        run_animation_loop(&mut scene, &renderer, &settings)?;
    } else {
        println!("--- 准备单帧渲染 ---");

        // 打印配置摘要
        println!("--- 渲染配置摘要 ---");
        // 这里可以添加配置摘要打印，如果需要的话
        println!("分辨率: {}x{}", settings.width, settings.height);
        println!("投影类型: {}", settings.projection);
        println!("使用光照: {}", settings.use_lighting);
        println!("-------------------");

        println!("使用{}", settings.get_lighting_description());
        render_single_frame(&mut scene, &renderer, &settings, &settings.output)?;
    }

    println!("总执行时间：{:?}", start_time.elapsed());
    Ok(())
}
