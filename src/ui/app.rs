use crate::core::renderer::Renderer;
use crate::io::render_settings::RenderSettings;
use crate::material_system::materials::ModelData;
use crate::scene::scene_utils::Scene;
use egui::{Color32, ColorImage, RichText, Vec2};
use nalgebra::Vector3;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// 导入其他UI模块
use super::animation::AnimationMethods;
use super::core::CoreMethods;
use super::widgets::WidgetMethods;

/// 🔥 **GUI应用状态** - 清晰分离TOML配置和GUI专用参数
pub struct RasterizerApp {
    // ===== 🔥 **TOML可配置参数 - 统一存储在settings中** =====
    /// 🔥 **所有TOML可配置的渲染参数**
    pub settings: RenderSettings,

    // ===== 🔥 **GUI专用向量字段 - 从settings字符串同步** =====
    /// GUI中物体位置控制的向量表示（与settings.object_position同步）
    pub object_position_vec: Vector3<f32>,
    /// GUI中物体旋转控制的向量表示（与settings.object_rotation同步，弧度制）
    pub object_rotation_vec: Vector3<f32>,
    /// GUI中物体缩放控制的向量表示（与settings.object_scale_xyz同步）
    pub object_scale_vec: Vector3<f32>,

    // ===== 🔥 **渲染运行时状态 - 不可配置** =====
    /// 渲染器实例
    pub renderer: Renderer,
    /// 当前加载的场景
    pub scene: Option<Scene>,
    /// 当前加载的模型数据
    pub model_data: Option<ModelData>,

    // ===== 🔥 **GUI界面状态 - 不可配置** =====
    /// 渲染结果纹理句柄
    pub rendered_image: Option<egui::TextureHandle>,
    /// 上次渲染耗时
    pub last_render_time: Option<std::time::Duration>,
    /// 状态消息显示
    pub status_message: String,
    /// 是否显示错误对话框
    pub show_error_dialog: bool,
    /// 错误消息内容
    pub error_message: String,

    // ===== 🔥 **实时渲染状态 - 不可配置** =====
    /// 当前实时帧率
    pub current_fps: f32,
    /// 帧率历史记录，用于平滑显示
    pub fps_history: Vec<f32>,
    /// 平均帧率
    pub avg_fps: f32,
    /// 是否正在实时渲染
    pub is_realtime_rendering: bool,
    /// 上一帧的时间戳
    pub last_frame_time: Option<std::time::Instant>,

    // ===== 🔥 **预渲染状态 - 不可配置** =====
    /// 是否启用预渲染模式
    pub pre_render_mode: bool,
    /// 是否正在预渲染
    pub is_pre_rendering: bool,
    /// 预渲染的帧集合
    pub pre_rendered_frames: Arc<Mutex<Vec<ColorImage>>>,
    /// 当前显示的帧索引
    pub current_frame_index: usize,
    /// 预渲染进度
    pub pre_render_progress: Arc<AtomicUsize>,
    /// 全局动画计时器，用于跟踪动画总时长
    pub animation_time: f32,
    /// 预渲染一个完整周期所需的总帧数
    pub total_frames_for_pre_render_cycle: usize,

    // ===== 🔥 **视频生成状态 - 不可配置** =====
    /// 是否正在生成视频
    pub is_generating_video: bool,
    /// 视频生成线程句柄
    pub video_generation_thread: Option<std::thread::JoinHandle<(bool, String)>>,
    /// 视频生成进度
    pub video_progress: Arc<AtomicUsize>,

    // ===== 🔥 **相机交互设置 - 可考虑加入TOML配置** =====
    /// 平移敏感度
    pub camera_pan_sensitivity: f32,
    /// 轨道旋转敏感度
    pub camera_orbit_sensitivity: f32,
    /// 推拉缩放敏感度
    pub camera_dolly_sensitivity: f32,

    // ===== 🔥 **相机交互状态 - 不可配置** =====
    /// 相机交互状态
    pub interface_interaction: InterfaceInteraction,

    // ===== 🔥 **系统状态 - 不可配置** =====
    /// ffmpeg可用性检查结果
    pub ffmpeg_available: bool,

    // ===== 🔥 **配置文件管理状态 - 新增字段** =====
    /// 当前配置文件路径
    pub current_config_path: Option<String>,
    /// 最近使用的配置文件列表
    pub recent_config_files: Vec<String>,
    /// 配置文件操作状态消息
    pub config_status_message: String,
    /// 是否显示配置摘要
    pub show_config_summary: bool,
}

/// 相机交互状态
#[derive(Default)]
pub struct InterfaceInteraction {
    pub camera_is_dragging: bool,
    pub camera_is_orbiting: bool,
    pub last_mouse_pos: Option<egui::Pos2>,
    pub anything_changed: bool, // 标记相机是否发生变化，需要重新渲染
}

impl RasterizerApp {
    /// 🔥 **从RenderSettings创建GUI应用实例**
    pub fn new(settings: RenderSettings, cc: &eframe::CreationContext<'_>) -> Self {
        // 配置字体，添加中文支持
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "chinese_font".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/Noto_Sans_SC/static/NotoSansSC-Regular.ttf"
            ))
            .into(),
        );

        for (_text_style, font_ids) in fonts.families.iter_mut() {
            font_ids.push("chinese_font".to_owned());
        }

        cc.egui_ctx.set_fonts(fonts);

        // 🔥 **从settings字符串初始化GUI专用向量字段**
        let object_position_vec =
            if let Ok(pos) = crate::io::render_settings::parse_vec3(&settings.object_position) {
                pos
            } else {
                nalgebra::Vector3::new(0.0, 0.0, 0.0)
            };

        let object_rotation_vec =
            if let Ok(rot) = crate::io::render_settings::parse_vec3(&settings.object_rotation) {
                nalgebra::Vector3::new(rot.x.to_radians(), rot.y.to_radians(), rot.z.to_radians())
            } else {
                nalgebra::Vector3::new(0.0, 0.0, 0.0)
            };

        let object_scale_vec =
            if let Ok(scale) = crate::io::render_settings::parse_vec3(&settings.object_scale_xyz) {
                scale
            } else {
                nalgebra::Vector3::new(1.0, 1.0, 1.0)
            };

        // 创建渲染器
        let renderer = Renderer::new(settings.width, settings.height);

        // 检查ffmpeg是否可用
        let ffmpeg_available = Self::check_ffmpeg_available();

        // 🔥 **确保settings有正确的光源配置**
        let mut initialized_settings = settings;
        initialized_settings.initialize_lights();

        Self {
            // 🔥 **TOML配置参数**
            settings: initialized_settings,

            // 🔥 **GUI专用向量字段**
            object_position_vec,
            object_rotation_vec,
            object_scale_vec,

            // 🔥 **渲染运行时状态**
            renderer,
            scene: None,
            model_data: None,

            // 🔥 **GUI界面状态**
            rendered_image: None,
            last_render_time: None,
            status_message: String::new(),
            show_error_dialog: false,
            error_message: String::new(),

            // 🔥 **实时渲染状态**
            current_fps: 0.0,
            fps_history: Vec::new(),
            avg_fps: 0.0,
            is_realtime_rendering: false,
            last_frame_time: None,

            // 🔥 **预渲染状态**
            pre_render_mode: false,
            is_pre_rendering: false,
            pre_rendered_frames: Arc::new(Mutex::new(Vec::new())),
            current_frame_index: 0,
            pre_render_progress: Arc::new(AtomicUsize::new(0)),
            animation_time: 0.0,
            total_frames_for_pre_render_cycle: 0,

            // 🔥 **视频生成状态**
            is_generating_video: false,
            video_generation_thread: None,
            video_progress: Arc::new(AtomicUsize::new(0)),

            // 🔥 **相机交互设置（可考虑移入TOML配置）**
            camera_pan_sensitivity: 1.0,
            camera_orbit_sensitivity: 1.0,
            camera_dolly_sensitivity: 1.0,

            // 🔥 **相机交互状态**
            interface_interaction: InterfaceInteraction::default(),

            // 🔥 **系统状态**
            ffmpeg_available,

            // 🔥 **配置文件管理状态**
            current_config_path: None,
            recent_config_files: Vec::new(),
            config_status_message: String::new(),
            show_config_summary: false,
        }
    }

        pub fn new_config_file(&mut self) {
        let mut new_settings = crate::io::render_settings::RenderSettings::default();
        new_settings.initialize_lights();
        
        self.apply_settings(new_settings);
        self.current_config_path = None;
        self.config_status_message = "已创建新的配置文件，请记得保存".to_string();
    }

    /// 🔥 **打开配置文件对话框**
    pub fn open_config_file(&mut self) {
        self.select_config_file();
    }

    /// 🔥 **保存当前配置**
    pub fn save_current_config(&mut self) {
        if let Some(config_path) = &self.current_config_path {
            match self.save_to_toml_file(config_path) {
                Ok(_) => {
                    self.config_status_message = format!("配置已保存到 {}", config_path);
                }
                Err(e) => {
                    self.config_status_message = format!("保存失败: {}", e);
                }
            }
        } else {
            self.save_config_as();
        }
    }

    /// 🔥 **另存为配置文件**
    pub fn save_config_as(&mut self) {
        self.save_config_file();
    }

    /// 🔥 **重新加载当前配置**
    pub fn reload_current_config(&mut self) {
        if let Some(config_path) = &self.current_config_path.clone() {
            self.load_config_from_path(config_path.clone());
        }
    }

    /// 🔥 **从路径加载配置**
    pub fn load_config_from_path(&mut self, path: String) {
        match crate::io::render_settings::RenderSettings::from_toml_file(&path) {
            Ok(settings) => {
                self.apply_settings(settings);
                self.current_config_path = Some(path.clone());
                self.add_to_recent_configs(path.clone());
                self.config_status_message = format!("已加载配置: {}", path);
            }
            Err(e) => {
                self.config_status_message = format!("加载失败: {}", e);
            }
        }
    }

    /// 🔥 **显示配置摘要**
    pub fn display_config_summary(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.small("📋 当前配置摘要:");
            
            if let Some(obj) = &self.settings.obj {
                ui.small(format!("• 模型: {}", 
                    std::path::Path::new(obj).file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("未知")));
            } else {
                ui.small("• 模型: 未指定");
            }
            
            ui.small(format!("• 分辨率: {}x{}", self.settings.width, self.settings.height));
            ui.small(format!("• 投影: {}", self.settings.projection));
            ui.small(format!("• 光照: {}", if self.settings.use_lighting { "启用" } else { "禁用" }));
            ui.small(format!("• 光源数量: {}", self.settings.lights.len()));
            
            let material_type = if self.settings.use_pbr {
                "PBR"
            } else if self.settings.use_phong {
                "Phong"
            } else {
                "基础"
            };
            ui.small(format!("• 材质: {}", material_type));
            
            if self.settings.animate {
                ui.small(format!("• 动画: {}fps, {:.1}圈", self.settings.fps, self.settings.rotation_cycles));
            }
        });
    }

    /// 🔥 **加载预设配置**
    pub fn load_preset_config(&mut self, preset_name: &str) {
        let mut settings = crate::io::render_settings::RenderSettings::default();
        
        match preset_name {
            "basic" => {
                settings.use_lighting = true;
                settings.use_phong = true;
                settings.use_pbr = false;
                settings.use_multithreading = true;
                settings.ambient = 0.3;
                settings.initialize_lights();
            }
            "high_quality" => {
                settings.use_lighting = true;
                settings.use_phong = true;
                settings.enhanced_ao = true;
                settings.ao_strength = 0.6;
                settings.soft_shadows = true;
                settings.shadow_strength = 0.4;
                settings.use_multithreading = true;
                settings.ambient = 0.2;
                settings.initialize_lights();
            }
            "animation" => {
                settings.animate = true;
                settings.fps = 30;
                settings.rotation_cycles = 2.0;
                settings.rotation_speed = 1.0;
                settings.use_multithreading = true;
                settings.use_lighting = true;
                settings.ambient = 0.4;
                settings.initialize_lights();
            }
            "material_showcase" => {
                settings.use_lighting = true;
                settings.use_pbr = true;
                settings.use_phong = false;
                settings.metallic = 0.1;
                settings.roughness = 0.3;
                settings.ambient = 0.1;
                settings.initialize_lights();
            }
            _ => {
                self.config_status_message = format!("未知预设: {}", preset_name);
                return;
            }
        }

        // 保留当前的文件路径
        settings.obj = self.settings.obj.clone();
        settings.output_dir = self.settings.output_dir.clone();
        settings.output = self.settings.output.clone();

        self.apply_settings(settings);
        self.current_config_path = None;
        self.config_status_message = format!("已应用 {} 预设配置", preset_name);
    }

    /// 🔥 **最近配置文件管理**
    pub fn load_recent_config(&mut self, path: String) {
        self.load_config_from_path(path);
    }

    pub fn add_to_recent_configs(&mut self, path: String) {
        self.recent_config_files.retain(|p| p != &path);
        self.recent_config_files.insert(0, path);
        if self.recent_config_files.len() > 10 {
            self.recent_config_files.truncate(10);
        }
    }

    pub fn remove_from_recent_configs(&mut self, path: String) {
        self.recent_config_files.retain(|p| p != &path);
    }

    pub fn clear_recent_configs(&mut self) {
        self.recent_config_files.clear();
        self.config_status_message = "已清空最近使用的配置文件历史".to_string();
    }

    /// 🔥 **从TOML文件创建GUI应用实例**
    pub fn from_toml_file<P: AsRef<std::path::Path>>(
        path: P,
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, String> {
        let settings = RenderSettings::from_toml_file(path)?;
        Ok(Self::new(settings, cc))
    }

    /// 🔥 **保存当前配置到TOML文件**
    pub fn save_to_toml_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), String> {
        self.settings.save_to_toml_file(path)
    }

    /// 🔥 **同步GUI向量字段到settings字符串**
    pub fn sync_vectors_to_settings(&mut self) {
        // 同步位置
        self.settings.object_position = format!(
            "{},{},{}",
            self.object_position_vec.x, self.object_position_vec.y, self.object_position_vec.z
        );

        // 同步旋转（弧度转度数）
        let rotation_degrees = nalgebra::Vector3::new(
            self.object_rotation_vec.x.to_degrees(),
            self.object_rotation_vec.y.to_degrees(),
            self.object_rotation_vec.z.to_degrees(),
        );
        self.settings.object_rotation = format!(
            "{},{},{}",
            rotation_degrees.x, rotation_degrees.y, rotation_degrees.z
        );

        // 同步缩放
        self.settings.object_scale_xyz = format!(
            "{},{},{}",
            self.object_scale_vec.x, self.object_scale_vec.y, self.object_scale_vec.z
        );
    }

    /// 🔥 **从settings字符串同步到GUI向量字段**
    pub fn sync_settings_to_vectors(&mut self) {
        // 同步位置
        if let Ok(pos) = crate::io::render_settings::parse_vec3(&self.settings.object_position) {
            self.object_position_vec = pos;
        }

        // 同步旋转（度数转弧度）
        if let Ok(rot) = crate::io::render_settings::parse_vec3(&self.settings.object_rotation) {
            self.object_rotation_vec =
                nalgebra::Vector3::new(rot.x.to_radians(), rot.y.to_radians(), rot.z.to_radians());
        }

        // 同步缩放
        if let Ok(scale) = crate::io::render_settings::parse_vec3(&self.settings.object_scale_xyz) {
            self.object_scale_vec = scale;
        }
    }

    /// 🔥 **应用新的RenderSettings配置**
    pub fn apply_settings(&mut self, new_settings: RenderSettings) {
        // 检查是否需要重新创建渲染器
        if self.renderer.frame_buffer.width != new_settings.width
            || self.renderer.frame_buffer.height != new_settings.height
        {
            self.renderer = Renderer::new(new_settings.width, new_settings.height);
            self.rendered_image = None; // 清除旧的渲染结果
        }

        // 更新设置
        self.settings = new_settings;

        // 确保光源已初始化
        self.settings.initialize_lights();

        // 同步向量字段
        self.sync_settings_to_vectors();

        // 标记需要重新渲染
        self.interface_interaction.anything_changed = true;
    }

    /// 检查ffmpeg是否可用
    fn check_ffmpeg_available() -> bool {
        std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .is_ok()
    }

    /// 设置错误信息并显示错误对话框
    pub fn set_error(&mut self, message: String) {
        self.error_message = message.clone();
        self.show_error_dialog = true;
        CoreMethods::set_error(self, message);
    }

    /// 🔥 **简化相机交互 - 直接更新settings**
    fn handle_camera_interaction(&mut self, image_response: &egui::Response, ctx: &egui::Context) {
        if let Some(scene) = &mut self.scene {
            let mut camera_changed = false;

            let screen_size = egui::Vec2::new(
                self.renderer.frame_buffer.width as f32,
                self.renderer.frame_buffer.height as f32,
            );

            // 处理鼠标拖拽
            if image_response.dragged() {
                if let Some(last_pos) = self.interface_interaction.last_mouse_pos {
                    let current_pos = image_response.interact_pointer_pos().unwrap_or_default();
                    let delta = current_pos - last_pos;

                    // 🔥 **设置最小移动阈值，避免微小抖动触发重新渲染**
                    if delta.length() < 1.0 {
                        return;
                    }

                    let is_shift_pressed = ctx.input(|i| i.modifiers.shift);

                    if is_shift_pressed && !self.interface_interaction.camera_is_orbiting {
                        self.interface_interaction.camera_is_orbiting = true;
                        self.interface_interaction.camera_is_dragging = false;
                    } else if !is_shift_pressed && !self.interface_interaction.camera_is_dragging {
                        self.interface_interaction.camera_is_dragging = true;
                        self.interface_interaction.camera_is_orbiting = false;
                    }

                    if self.interface_interaction.camera_is_orbiting && is_shift_pressed {
                        scene
                            .active_camera
                            .orbit_from_screen_delta(delta, self.camera_orbit_sensitivity);
                        camera_changed = true;
                    } else if self.interface_interaction.camera_is_dragging && !is_shift_pressed {
                        scene.active_camera.pan_from_screen_delta(
                            delta,
                            screen_size,
                            self.camera_pan_sensitivity,
                        );
                        camera_changed = true;
                    }
                }

                self.interface_interaction.last_mouse_pos = image_response.interact_pointer_pos();
            } else {
                self.interface_interaction.camera_is_dragging = false;
                self.interface_interaction.camera_is_orbiting = false;
                self.interface_interaction.last_mouse_pos = None;
            }

            // 处理鼠标滚轮缩放
            if image_response.hovered() {
                let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta.abs() > 0.1 {
                    let zoom_delta = scroll_delta * 0.01;
                    scene
                        .active_camera
                        .dolly_from_scroll(zoom_delta, self.camera_dolly_sensitivity);
                    camera_changed = true;
                }
            }

            // 处理快捷键
            ctx.input(|i| {
                if i.key_pressed(egui::Key::R) {
                    scene.active_camera.reset_to_default_view();
                    camera_changed = true;
                }

                if i.key_pressed(egui::Key::F) {
                    let object_center = nalgebra::Point3::new(0.0, 0.0, 0.0);
                    let object_radius = 2.0;
                    scene
                        .active_camera
                        .focus_on_object(object_center, object_radius);
                    camera_changed = true;
                }
            });

            // 🔥 **如果相机发生变化，直接更新settings并标记**
            if camera_changed {
                // 直接更新settings字符串
                let pos = scene.active_camera.position();
                let target = scene.active_camera.params.target;
                let up = scene.active_camera.params.up;

                self.settings.camera_from = format!("{},{},{}", pos.x, pos.y, pos.z);
                self.settings.camera_at = format!("{},{},{}", target.x, target.y, target.z);
                self.settings.camera_up = format!("{},{},{}", up.x, up.y, up.z);

                // 统一标记
                self.interface_interaction.anything_changed = true;

                // 在非实时模式下请求重绘
                if !self.is_realtime_rendering {
                    ctx.request_repaint();
                }
            }
        }
    }

    /// 🔥 **显示错误对话框**
    fn show_error_dialog_ui(&mut self, ctx: &egui::Context) {
        if self.show_error_dialog {
            egui::Window::new("错误")
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&self.error_message);
                    ui.separator();
                    if ui.button("确定").clicked() {
                        self.show_error_dialog = false;
                    }
                });
        }
    }

    /// 🔥 **执行实时渲染循环**
    fn perform_realtime_rendering(&mut self, ctx: &egui::Context) {
        let current_time = std::time::Instant::now();
        
        if let Some(last_time) = self.last_frame_time {
            let frame_time = current_time.duration_since(last_time);
            CoreMethods::update_fps_stats(self, frame_time);
        }
        
        self.last_frame_time = Some(current_time);
        
        // 标记需要重新渲染
        self.interface_interaction.anything_changed = true;
        
        // 请求下一帧
        ctx.request_repaint();
    }

    /// 🔥 **绘制侧边面板 - 委托给WidgetMethods**
    fn draw_side_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        WidgetMethods::draw_side_panel(self, ctx, ui);
    }

    /// 🔥 **统一的资源清理方法**
    fn cleanup_resources(&mut self) {
        CoreMethods::cleanup_resources(self);
    }
}

impl eframe::App for RasterizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 显示错误对话框（如果有）
        self.show_error_dialog_ui(ctx);

        // 检查快捷键
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::R)) {
            CoreMethods::render(self, ctx);
        }

        // 执行实时渲染循环
        if self.is_realtime_rendering {
            self.perform_realtime_rendering(ctx);
        }

        // 检查视频生成进度
        if self.is_generating_video {
            if let Some(handle) = &self.video_generation_thread {
                if handle.is_finished() {
                    let result = self
                        .video_generation_thread
                        .take()
                        .unwrap()
                        .join()
                        .unwrap_or_else(|_| (false, "线程崩溃".to_string()));

                    self.is_generating_video = false;

                    if result.0 {
                        self.status_message = format!("视频生成成功: {}", result.1);
                    } else {
                        self.set_error(format!("视频生成失败: {}", result.1));
                    }

                    self.video_progress.store(0, Ordering::SeqCst);
                } else {
                    let progress = self.video_progress.load(Ordering::SeqCst);

                    let (_, _, frames_per_rotation) =
                        crate::utils::render_utils::calculate_rotation_parameters(
                            self.settings.rotation_speed,
                            self.settings.fps,
                        );
                    let total_frames =
                        (frames_per_rotation as f32 * self.settings.rotation_cycles) as usize;

                    let percent = (progress as f32 / total_frames as f32 * 100.0).round();

                    self.status_message = format!(
                        "生成视频中... ({}/{}，{:.0}%)",
                        progress, total_frames, percent
                    );

                    ctx.request_repaint_after(std::time::Duration::from_millis(500));
                }
            }
        }

        // UI布局
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("光栅化渲染器");
                ui.separator();
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_realtime_rendering {
                        let (fps_text, fps_color) = CoreMethods::get_fps_display(self);
                        ui.label(RichText::new(&fps_text).color(fps_color));
                        ui.separator();
                    }
                    ui.label("Ctrl+R: 快速渲染");
                });
            });
        });

        egui::SidePanel::left("left_panel")
            .min_width(350.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.draw_side_panel(ctx, ui);
            });

        // 中央面板 - 显示渲染结果和处理相机交互
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.rendered_image {
                let available_size = ui.available_size();
                let square_size = available_size.x.min(available_size.y) * 0.95;

                let image_aspect = self.renderer.frame_buffer.width as f32
                    / self.renderer.frame_buffer.height as f32;

                let (width, height) = if image_aspect > 1.0 {
                    (square_size, square_size / image_aspect)
                } else {
                    (square_size * image_aspect, square_size)
                };

                let image_response = ui
                    .horizontal(|ui| {
                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(Vec2::new(width, height))
                                .sense(egui::Sense::click_and_drag()),
                        )
                    })
                    .inner;

                self.handle_camera_interaction(&image_response, ctx);

                // 显示交互提示
                let overlay_rect = egui::Rect::from_min_size(
                    ui.max_rect().right_bottom() - egui::Vec2::new(220.0, 20.0),
                    egui::Vec2::new(220.0, 20.0),
                );

                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(overlay_rect)
                        .layout(egui::Layout::right_to_left(egui::Align::BOTTOM)),
                    |ui| {
                        ui.group(|ui| {
                            ui.label(RichText::new("🖱️ 相机交互").size(14.0).strong());
                            ui.separator();
                            ui.small("• 拖拽 - 平移相机");
                            ui.small("• Shift+拖拽 - 轨道旋转");
                            ui.small("• 滚轮 - 推拉缩放");
                            ui.small("• R键 - 重置视角");
                            ui.small("• F键 - 聚焦物体");
                            ui.separator();
                            ui.small(format!("平移敏感度: {:.1}x", self.camera_pan_sensitivity));
                            ui.small(format!("旋转敏感度: {:.1}x", self.camera_orbit_sensitivity));
                            ui.small(format!("缩放敏感度: {:.1}x", self.camera_dolly_sensitivity));
                            ui.separator();
                            ui.small(RichText::new("✅ 交互已启用").color(Color32::GREEN));
                        });
                    },
                );
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(RichText::new("无渲染结果").size(24.0).color(Color32::GRAY));
                    ui.label(RichText::new("点击「开始渲染」按钮或按Ctrl+R").color(Color32::GRAY));
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("💡 加载模型后可在此区域进行相机交互")
                            .color(Color32::from_rgb(100, 150, 255)),
                    );
                });
            }
        });

        // 🔥 **统一处理所有变化引起的重新渲染**
        CoreMethods::render_if_anything_changed(self, ctx);

        // 在每帧更新结束时清理不需要的资源
        self.cleanup_resources();
    }
}

/// 启动GUI应用
pub fn start_gui(settings: RenderSettings) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1350.0, 900.0])
            .with_min_inner_size([1100.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "光栅化渲染器",
        options,
        Box::new(|cc| Ok(Box::new(RasterizerApp::new(settings, cc)))),
    )
}