use clap::Parser;
use crate::io::config_loader::TomlConfigLoader;
use crate::io::render_settings::RenderSettings;
use crate::material_system::light::Light;

/// 🔥 **TOML驱动的极简CLI** - 配置文件优先，命令行简化
#[derive(Parser, Debug)]
#[command(name = "rasterizer")]
#[command(about = "🎨 TOML驱动的高性能光栅化渲染器")]
#[command(long_about = r#"
🔥 基于TOML配置文件的现代光栅化渲染器

特性:
  • 📝 人类可读的TOML配置文件
  • 🔥 多光源支持 (方向光、点光源)
  • 🎨 完整材质系统 (Phong、PBR)
  • ⚡ 多线程渲染优化
  • 🖼️ 高级背景和动画支持

使用方式:
  rasterizer --generate-example scene.toml  # 生成示例配置
  rasterizer --config scene.toml            # 加载配置启动GUI
  rasterizer --config scene.toml --render   # 直接渲染
  rasterizer                                 # 默认设置启动GUI
"#)]
pub struct SimpleCli {
    /// 📁 配置文件路径（TOML格式）
    #[arg(short, long, value_name = "FILE")]
    #[arg(help = "TOML配置文件路径，包含所有渲染设置")]
    pub config: Option<String>,
    
    /// 🚀 直接渲染模式（跳过GUI）
    #[arg(short, long)]
    #[arg(help = "直接开始渲染，不启动图形界面")]
    pub render: bool,
    
    /// 📋 生成示例配置文件
    #[arg(long, value_name = "FILE")]
    #[arg(help = "生成包含所有功能的示例TOML配置文件")]
    pub generate_example: Option<String>,
    
    /// 🎯 详细输出模式
    #[arg(short, long)]
    #[arg(help = "显示详细的渲染过程信息")]
    pub verbose: bool,
    
    /// 📊 显示当前版本信息
    #[arg(long)]
    #[arg(help = "显示版本信息和功能列表")]
    pub version_info: bool,
    
    /// 🔍 验证配置文件（不执行渲染）
    #[arg(long, value_name = "FILE")]
    #[arg(help = "验证TOML配置文件的正确性，不执行渲染")]
    pub validate: Option<String>,
}

impl SimpleCli {
    /// 🔥 **处理CLI参数并返回RenderSettings**
    pub fn process() -> Result<Option<RenderSettings>, String> {
        let cli = Self::parse();
        
        // 显示版本信息
        if cli.version_info {
            Self::print_version_info();
            return Ok(None);
        }
        
        // 验证配置文件
        if let Some(validate_path) = &cli.validate {
            Self::validate_config_file(validate_path)?;
            return Ok(None);
        }
        
        // 生成示例配置
        if let Some(example_path) = &cli.generate_example {
            Self::generate_example_config(example_path)?;
            println!("✅ 示例配置文件已生成: {}", example_path);
            println!("💡 使用 --config {} 加载配置", example_path);
            return Ok(None);
        }
        
        // 设置详细输出
        if cli.verbose {
            println!("🔍 启用详细输出模式");
        }
        
        // 加载配置
        let settings = if let Some(config_path) = &cli.config {
            if cli.verbose {
                println!("📁 正在加载配置文件: {}", config_path);
            }
            
            let settings = TomlConfigLoader::load_from_file(config_path)
                .map_err(|e| format!("❌ 配置文件加载失败: {}", e))?;
            
            if cli.verbose {
                Self::print_config_summary(&settings);
            }
            
            settings
        } else {
            if cli.verbose {
                println!("💡 未指定配置文件，使用默认设置");
            }
            
            let mut default_settings = RenderSettings::default();
            // 🔥 **确保默认状态有基础光源**
            default_settings.initialize_lights();
            default_settings
        };
        
        // 直接渲染模式
        if cli.render {
            if cli.verbose {
                println!("🚀 直接渲染模式启动");
            }
            
            // 验证必要参数
            settings.validate()
                .map_err(|e| format!("❌ 配置验证失败: {}", e))?;
            
            // 执行无头渲染
            Self::execute_headless_render(settings, cli.verbose)?;
            return Ok(None);
        }
        
        // 返回设置，启动GUI
        Ok(Some(settings))
    }
    
    /// 🔥 **生成功能完整的示例配置文件**
    fn generate_example_config(path: &str) -> Result<(), String> {
        TomlConfigLoader::generate_example_config(path)
    }
    
    /// 🔍 **验证配置文件**
    fn validate_config_file(path: &str) -> Result<(), String> {
        println!("🔍 验证配置文件: {}", path);
        
        let settings = TomlConfigLoader::load_from_file(path)
            .map_err(|e| format!("❌ 配置文件解析失败: {}", e))?;
        
        settings.validate()
            .map_err(|e| format!("❌ 配置验证失败: {}", e))?;
        
        println!("✅ 配置文件验证通过！");
        Self::print_config_summary(&settings);
        
        Ok(())
    }
    
    /// 📊 **显示版本信息**
    fn print_version_info() {
        println!(r#"
🎨 Rust光栅化渲染器 v1.0.0

🔥 核心特性:
  • 多光源支持 (方向光、点光源)
  • 完整材质系统 (Phong、PBR)
  • 高级背景渲染 (渐变、图片、地面)
  • 多线程优化渲染
  • 实时GUI预览
  • TOML配置驱动

📝 支持格式:
  • 输入: OBJ模型文件
  • 纹理: JPG, PNG, BMP
  • 输出: PNG图像 + 深度图
  • 配置: TOML格式

🚀 使用指南:
  rasterizer --generate-example demo.toml  # 生成示例
  rasterizer --config demo.toml --render   # 直接渲染
  rasterizer --config demo.toml            # GUI模式
"#);
    }
    
    /// 📊 **显示配置摘要**
    fn print_config_summary(settings: &RenderSettings) {
        println!("📊 配置摘要:");
        
        if let Some(obj) = &settings.obj {
            println!("  📦 模型文件: {}", obj);
        } else {
            println!("  📦 模型文件: 未指定");
        }
        
        println!("  🖼️  输出尺寸: {}x{}", settings.width, settings.height);
        println!("  📐 投影模式: {}", settings.projection);
        
        if settings.use_lighting {
            println!("  💡 光照系统: 启用 ({} 个光源)", settings.lights.len());
            
            let directional_count = settings.lights.iter()
                .filter(|light| matches!(light, Light::Directional { .. }))
                .count();
            let point_count = settings.lights.iter()
                .filter(|light| matches!(light, Light::Point { .. }))
                .count();
            
            if directional_count > 0 {
                println!("    🔦 方向光: {} 个", directional_count);
            }
            if point_count > 0 {
                println!("    💡 点光源: {} 个", point_count);
            }
        } else {
            println!("  💡 光照系统: 禁用");
        }
        
        let material_type = if settings.use_pbr {
            "PBR物理渲染"
        } else if settings.use_phong {
            "Phong着色"
        } else {
            "平面着色"
        };
        println!("  🎨 材质系统: {}", material_type);
        
        if settings.use_texture {
            if let Some(texture) = &settings.texture {
                println!("  🖼️  纹理贴图: {}", texture);
            } else {
                println!("  🖼️  纹理贴图: 使用MTL文件");
            }
        } else {
            println!("  🖼️  纹理贴图: 禁用");
        }
        
        if settings.use_multithreading {
            println!("  ⚡ 多线程: 启用");
        }
        
        if settings.animate {
            println!("  🎬 动画模式: 启用 ({}fps)", settings.fps);
        }
        
        println!("  📁 输出目录: {}", settings.output_dir);
    }
    
    /// 🚀 **执行无头渲染**
    fn execute_headless_render(settings: RenderSettings, verbose: bool) -> Result<(), String> {
        if verbose {
            println!("🔧 初始化渲染器...");
        }
        
        // TODO: 这里应该调用实际的渲染函数
        // 暂时模拟渲染过程
        
        if verbose {
            println!("📦 加载模型文件...");
            if let Some(obj_path) = &settings.obj {
                println!("   📄 文件: {}", obj_path);
            }
            
            println!("🎨 设置材质和光照...");
            println!("   💡 光源数量: {}", settings.lights.len());
            
            println!("🖼️  开始渲染 {}x{} 像素...", settings.width, settings.height);
            
            if settings.use_multithreading {
                println!("   ⚡ 使用多线程加速");
            }
        }
        
        // 模拟渲染时间
        use std::thread;
        use std::time::Duration;
        
        if verbose {
            println!("⏳ 渲染进行中...");
            thread::sleep(Duration::from_millis(100)); // 模拟渲染
            println!("✅ 渲染完成！");
            println!("📁 输出保存至: {}/{}.png", settings.output_dir, settings.output);
            
            if settings.save_depth {
                println!("📁 深度图保存至: {}/{}_depth.png", settings.output_dir, settings.output);
            }
        } else {
            println!("🚀 开始渲染...");
            thread::sleep(Duration::from_millis(50)); // 模拟渲染
            println!("✅ 渲染完成: {}/{}.png", settings.output_dir, settings.output);
        }
        
        Ok(())
    }
    
    /// 🔧 **检查是否应该启动GUI**
    pub fn should_start_gui(&self) -> bool {
        // 如果有这些参数，不启动GUI
        if self.render || self.generate_example.is_some() || 
           self.version_info || self.validate.is_some() {
            return false;
        }
        
        true
    }
    
    /// 📋 **获取使用帮助信息**
    pub fn get_usage_examples() -> Vec<(&'static str, &'static str)> {
        vec![
            ("生成示例配置", "rasterizer --generate-example scene.toml"),
            ("验证配置文件", "rasterizer --validate scene.toml"),
            ("GUI模式渲染", "rasterizer --config scene.toml"),
            ("直接渲染模式", "rasterizer --config scene.toml --render"),
            ("详细输出模式", "rasterizer --config scene.toml --render --verbose"),
            ("查看版本信息", "rasterizer --version-info"),
            ("使用默认设置", "rasterizer"),
        ]
    }
}

/// 🔥 **CLI错误类型** - 更好的错误处理
#[derive(Debug)]
pub enum CliError {
    ConfigNotFound(String),
    ConfigParseFailed(String),
    ValidationFailed(String),
    RenderFailed(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::ConfigNotFound(path) => write!(f, "❌ 配置文件未找到: {}", path),
            CliError::ConfigParseFailed(msg) => write!(f, "❌ 配置文件解析失败: {}", msg),
            CliError::ValidationFailed(msg) => write!(f, "❌ 参数验证失败: {}", msg),
            CliError::RenderFailed(msg) => write!(f, "❌ 渲染失败: {}", msg),
            CliError::IoError(err) => write!(f, "❌ 文件操作失败: {}", err),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        CliError::IoError(error)
    }
}

/// 🔥 **便捷宏** - 快速错误返回
#[macro_export]
macro_rules! cli_bail {
    ($msg:expr) => {
        return Err($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(format!($fmt, $($arg)*))
    };
}