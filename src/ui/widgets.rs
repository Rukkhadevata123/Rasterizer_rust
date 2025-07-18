use egui::{Color32, Context, RichText, Vec2};
use std::sync::atomic::Ordering;

use super::animation::AnimationMethods;
use super::app::RasterizerApp;
use super::core::CoreMethods;
use super::render_ui::RenderUIMethods;
use crate::core::renderer::Renderer;
use crate::geometry::camera::ProjectionType;
use crate::io::config_loader::TomlConfigLoader;
use crate::io::render_settings::{AnimationType, RotationAxis, parse_point3, parse_vec3};
use crate::material_system::light::Light;
use crate::utils::render_utils::calculate_rotation_parameters;

/// UI组件和工具提示相关方法的特质
pub trait WidgetMethods {
    /// 绘制UI的侧边栏
    fn draw_side_panel(&mut self, ctx: &Context, ui: &mut egui::Ui);

    /// 显示错误对话框
    fn show_error_dialog_ui(&mut self, ctx: &Context);

    /// 显示工具提示
    fn add_tooltip(response: egui::Response, ctx: &Context, text: &str) -> egui::Response;

    // === 面板函数接口 ===

    /// 绘制文件与输出设置面板
    fn ui_file_output_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制渲染属性设置面板
    fn ui_render_properties_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制物体变换控制面板
    fn ui_object_transform_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制背景与环境设置面板
    fn ui_background_settings(app: &mut RasterizerApp, ui: &mut egui::Ui);

    /// 绘制相机设置面板
    fn ui_camera_settings_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制光照设置面板
    fn ui_lighting_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制PBR材质设置面板
    fn ui_pbr_material_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制Phong材质设置面板
    fn ui_phong_material_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制动画设置面板
    fn ui_animation_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制按钮控制面板
    fn ui_button_controls_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context);

    /// 绘制渲染信息面板
    fn ui_render_info_panel(app: &mut RasterizerApp, ui: &mut egui::Ui);
}

impl WidgetMethods for RasterizerApp {
    /// 重构后的侧边栏
    fn draw_side_panel(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        // 主题切换控件（放在侧边栏顶部）
        ui.horizontal(|ui| {
            ui.label("主题：");
            egui::ComboBox::from_id_salt("theme_switch")
                .selected_text(if self.is_dark_theme {
                    "深色"
                } else {
                    "浅色"
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut self.is_dark_theme, true, "深色")
                        .clicked()
                    {
                        ctx.set_visuals(egui::Visuals::dark());
                    }
                    if ui
                        .selectable_value(&mut self.is_dark_theme, false, "浅色")
                        .clicked()
                    {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                });
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // === 核心设置组 ===
            ui.collapsing("📁 文件与输出", |ui| {
                Self::ui_file_output_panel(self, ui, ctx);
            });

            ui.collapsing("🎨 场景与视觉", |ui| {
                // 合并渲染属性和背景设置
                ui.group(|ui| {
                    ui.label(RichText::new("渲染设置").size(14.0).strong());
                    Self::ui_render_properties_panel(self, ui, ctx);
                });

                ui.separator();

                ui.group(|ui| {
                    ui.label(RichText::new("背景设置").size(14.0).strong());
                    Self::ui_background_settings(self, ui);
                });
            });

            // === 3D变换组 ===
            ui.collapsing("🔄 3D变换与相机", |ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("物体变换").size(14.0).strong());
                    Self::ui_object_transform_panel(self, ui, ctx);
                });

                ui.separator();

                ui.group(|ui| {
                    ui.label(RichText::new("相机控制").size(14.0).strong());
                    Self::ui_camera_settings_panel(self, ui, ctx);
                });
            });

            // === 材质与光照组 ===
            ui.collapsing("💡 光照与材质", |ui| {
                // 先显示光照和通用材质属性
                Self::ui_lighting_panel(self, ui, ctx);

                ui.separator();

                // 然后根据着色模型显示专用设置
                if self.settings.use_pbr {
                    ui.group(|ui| {
                        ui.label(RichText::new("✨ PBR专用参数").size(14.0).strong());
                        Self::ui_pbr_material_panel(self, ui, ctx);
                    });
                }

                if self.settings.use_phong {
                    ui.group(|ui| {
                        ui.label(RichText::new("✨ Phong专用参数").size(14.0).strong());
                        Self::ui_phong_material_panel(self, ui, ctx);
                    });
                }
            });

            // === 动画与渲染组 ===
            ui.collapsing("🎬 动画与渲染", |ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("动画设置").size(14.0).strong());
                    Self::ui_animation_panel(self, ui, ctx);
                });

                ui.separator();

                ui.group(|ui| {
                    ui.label(RichText::new("渲染控制").size(14.0).strong());
                    Self::ui_button_controls_panel(self, ui, ctx);
                });
            });

            // === 信息显示组 ===
            ui.collapsing("📊 渲染信息", |ui| {
                Self::ui_render_info_panel(self, ui);
            });
        });
    }

    /// 显示错误对话框
    fn show_error_dialog_ui(&mut self, ctx: &Context) {
        if self.show_error_dialog {
            egui::Window::new("错误")
                .fixed_size([400.0, 150.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(&self.error_message)
                                .color(Color32::from_rgb(230, 50, 50))
                                .size(16.0),
                        );
                        ui.add_space(20.0);
                        if ui.button(RichText::new("确定").size(16.0)).clicked() {
                            self.show_error_dialog = false;
                        }
                    });
                });
        }
    }

    /// 显示工具提示
    fn add_tooltip(response: egui::Response, _ctx: &Context, text: &str) -> egui::Response {
        response.on_hover_ui(|ui| {
            ui.add(egui::Label::new(
                RichText::new(text).size(14.0).color(Color32::DARK_GRAY),
            ));
        })
    }

    /// 文件与输出设置面板
    fn ui_file_output_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label("OBJ文件：");
            let mut obj_text = app.settings.obj.clone().unwrap_or_default();
            let response = ui.text_edit_singleline(&mut obj_text);
            if response.changed() {
                if obj_text.is_empty() {
                    app.settings.obj = None;
                } else {
                    app.settings.obj = Some(obj_text);
                }

                // OBJ路径变化需要重新加载场景
                app.interface_interaction.anything_changed = true;
                app.scene = None; // 清除现有场景，强制重新加载
                app.rendered_image = None; // 清除渲染结果
            }
            Self::add_tooltip(response, ctx, "选择要渲染的3D模型文件（.obj格式）");
            if ui.button("浏览").clicked() {
                app.select_obj_file();
            }
        });

        // 配置文件管理
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("配置文件：");
            if ui.button("📁 加载配置").clicked() {
                app.load_config_file();
            }
            if ui.button("💾 保存配置").clicked() {
                app.save_config_file();
            }
            if ui.button("📋 示例配置").clicked() {
                // 创建示例配置并应用
                match TomlConfigLoader::create_example_config("temp_example_for_gui.toml") {
                    Ok(_) => {
                        match TomlConfigLoader::load_from_file("temp_example_for_gui.toml") {
                            Ok(example_settings) => {
                                app.apply_loaded_config(example_settings);
                                app.status_message = "示例配置已应用".to_string();
                                // 删除临时文件
                                let _ = std::fs::remove_file("temp_example_for_gui.toml");
                            }
                            Err(e) => {
                                app.set_error(format!("加载示例配置失败: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        app.set_error(format!("创建示例配置失败: {e}"));
                    }
                }
            }
        });
        ui.small("💡 提示：加载配置会覆盖当前所有设置");

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("输出目录：");
            let response = ui.text_edit_singleline(&mut app.settings.output_dir);
            Self::add_tooltip(response, ctx, "选择渲染结果保存的目录");
            if ui.button("浏览").clicked() {
                app.select_output_dir();
            }
        });

        ui.horizontal(|ui| {
            ui.label("输出文件名：");
            let response = ui.text_edit_singleline(&mut app.settings.output);
            Self::add_tooltip(response, ctx, "渲染结果的文件名（不含扩展名）");
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("宽度：");
            let old_width = app.settings.width;
            let response = ui.add(
                egui::DragValue::new(&mut app.settings.width)
                    .speed(1)
                    .range(1..=4096),
            );
            if app.settings.width != old_width {
                // 分辨率变化需要重新创建渲染器
                app.renderer = Renderer::new(app.settings.width, app.settings.height);
                app.rendered_image = None;
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(response, ctx, "渲染图像的宽度（像素）");
        });

        ui.horizontal(|ui| {
            ui.label("高度：");
            let old_height = app.settings.height;
            let response = ui.add(
                egui::DragValue::new(&mut app.settings.height)
                    .speed(1)
                    .range(1..=4096),
            );
            if app.settings.height != old_height {
                app.renderer = Renderer::new(app.settings.width, app.settings.height);
                app.rendered_image = None;
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(response, ctx, "渲染图像的高度（像素）");
        });

        let response = ui.checkbox(&mut app.settings.save_depth, "保存深度图");
        Self::add_tooltip(response, ctx, "同时保存深度图（深度信息可视化）");
    }

    /// 渲染属性设置面板
    fn ui_render_properties_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label("投影类型：");
            let old_projection = app.settings.projection.clone();
            let resp1 = ui.radio_value(
                &mut app.settings.projection,
                "perspective".to_string(),
                "透视",
            );
            let resp2 = ui.radio_value(
                &mut app.settings.projection,
                "orthographic".to_string(),
                "正交",
            );
            if app.settings.projection != old_projection {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp1, ctx, "使用透视投影（符合人眼观察方式）");
            Self::add_tooltip(resp2, ctx, "使用正交投影（无透视变形）");
        });

        ui.separator();

        // 深度缓冲
        let old_zbuffer = app.settings.use_zbuffer;
        let resp1 = ui.checkbox(&mut app.settings.use_zbuffer, "深度缓冲");
        if app.settings.use_zbuffer != old_zbuffer {
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(resp1, ctx, "启用Z缓冲进行深度测试，处理物体遮挡关系");

        // 表面颜色设置
        ui.horizontal(|ui| {
            ui.label("表面颜色：");

            let old_texture = app.settings.use_texture;
            let old_colorize = app.settings.colorize;

            let texture_response = ui.radio_value(&mut app.settings.use_texture, true, "使用纹理");
            if texture_response.clicked() && app.settings.use_texture {
                app.settings.colorize = false;
            }

            let face_color_response =
                ui.radio_value(&mut app.settings.colorize, true, "使用面颜色");
            if face_color_response.clicked() && app.settings.colorize {
                app.settings.use_texture = false;
            }

            let material_color_response = ui.radio(
                !app.settings.use_texture && !app.settings.colorize,
                "使用材质颜色",
            );
            if material_color_response.clicked() {
                app.settings.use_texture = false;
                app.settings.colorize = false;
            }

            if app.settings.use_texture != old_texture || app.settings.colorize != old_colorize {
                app.interface_interaction.anything_changed = true;
            }

            Self::add_tooltip(
                texture_response,
                ctx,
                "使用模型的纹理贴图（如果有）\n优先级最高，会覆盖面颜色设置",
            );
            Self::add_tooltip(
                face_color_response,
                ctx,
                "为每个面分配随机颜色\n仅在没有纹理或纹理被禁用时生效",
            );
            Self::add_tooltip(
                material_color_response,
                ctx,
                "使用材质的基本颜色（如.mtl文件中定义）\n在没有纹理且不使用面颜色时生效",
            );
        });

        // 着色模型设置
        ui.horizontal(|ui| {
            ui.label("着色模型：");
            let old_phong = app.settings.use_phong;
            let old_pbr = app.settings.use_pbr;

            let phong_response = ui.radio_value(&mut app.settings.use_phong, true, "Phong着色");
            if phong_response.clicked() && app.settings.use_phong {
                app.settings.use_pbr = false;
            }

            let pbr_response = ui.radio_value(&mut app.settings.use_pbr, true, "PBR渲染");
            if pbr_response.clicked() && app.settings.use_pbr {
                app.settings.use_phong = false;
            }

            if app.settings.use_phong != old_phong || app.settings.use_pbr != old_pbr {
                app.interface_interaction.anything_changed = true;
            }

            Self::add_tooltip(phong_response, ctx, "使用 Phong 着色（逐像素着色）和 Blinn-Phong 光照模型\n提供高质量的光照效果，适合大多数场景");
            Self::add_tooltip(pbr_response, ctx, "使用基于物理的渲染（PBR）\n提供更真实的材质效果，但需要更多的参数调整");
        });

        ui.separator();

        // 修改原有的增强光照效果组，添加阴影映射
        ui.group(|ui| {

            // 阴影映射设置
            let old_shadow_mapping = app.settings.enable_shadow_mapping;
            let resp = ui.checkbox(&mut app.settings.enable_shadow_mapping, "地面阴影映射");
            if app.settings.enable_shadow_mapping != old_shadow_mapping {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(
                resp,
                ctx,
                "启用简单阴影映射，在地面显示物体阴影\n需要至少一个方向光源\n相比软阴影更真实但需要更多计算"
            );

            if app.settings.enable_shadow_mapping {
                ui.group(|ui| {
                    ui.label(RichText::new("阴影映射参数").size(12.0).strong());

                    ui.horizontal(|ui| {
                        ui.label("阴影贴图尺寸:");
                        let old_size = app.settings.shadow_map_size;
                        let resp = ui.add(
                            egui::DragValue::new(&mut app.settings.shadow_map_size)
                                .speed(128)
                                .range(128..=10240)
                        );
                        if app.settings.shadow_map_size != old_size {
                            app.interface_interaction.anything_changed = true;
                        }
                        Self::add_tooltip(resp, ctx, "输入阴影贴图分辨率（如4096），越大越清晰但越慢");
                    });

                    ui.horizontal(|ui| {
                        ui.label("阴影偏移:");
                        let old_bias = app.settings.shadow_bias;
                        let resp = ui.add(
                            egui::Slider::new(&mut app.settings.shadow_bias, 0.0001..=0.01)
                                .step_by(0.0001)
                                .custom_formatter(|n, _| format!("{n:.4}"))
                        );
                        if (app.settings.shadow_bias - old_bias).abs() > f32::EPSILON {
                            app.interface_interaction.anything_changed = true;
                        }
                        Self::add_tooltip(resp, ctx, "防止阴影痤疮的偏移值\n值太小会出现自阴影，值太大会使阴影分离");
                    });

                    ui.horizontal(|ui| {
                        ui.label("阴影距离:");
                        let old_distance = app.settings.shadow_distance;
                        let resp = ui.add(
                            egui::Slider::new(&mut app.settings.shadow_distance, 1.0..=100.0)
                                .suffix(" 单位")
                        );
                        if (app.settings.shadow_distance - old_distance).abs() > f32::EPSILON {
                            app.interface_interaction.anything_changed = true;
                        }
                        Self::add_tooltip(resp, ctx, "阴影渲染的最大距离\n距离越大覆盖范围越广，但阴影精度可能降低");
                    });

                    // 是否启用PCF
                    let old_enable_pcf = app.settings.enable_pcf;
                    let resp = ui.checkbox(&mut app.settings.enable_pcf, "启用PCF软阴影");
                    if app.settings.enable_pcf != old_enable_pcf {
                        app.interface_interaction.anything_changed = true;
                    }
                    Self::add_tooltip(resp, ctx, "开启后阴影边缘会变软，抗锯齿但性能消耗增加");

                    if app.settings.enable_pcf {
                        // PCF类型选择
                        let old_pcf_type = app.settings.pcf_type.clone();
                        egui::ComboBox::from_id_salt("pcf_type_combo")
                            .selected_text(&app.settings.pcf_type)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut app.settings.pcf_type, "Box".to_string(), "Box");
                                ui.selectable_value(&mut app.settings.pcf_type, "Gauss".to_string(), "Gauss");
                            });
                        if app.settings.pcf_type != old_pcf_type {
                            app.interface_interaction.anything_changed = true;
                        }

                        // kernel参数
                        let old_kernel = app.settings.pcf_kernel;
                        let resp = ui.add(
                            egui::Slider::new(&mut app.settings.pcf_kernel, 1..=10)
                                .text("PCF窗口(kernel)")
                        );
                        if app.settings.pcf_kernel != old_kernel {
                            app.interface_interaction.anything_changed = true;
                        }
                        Self::add_tooltip(resp, ctx, "采样窗口半径，越大越软，性能消耗也越高");

                        // Gauss类型时显示sigma
                        if app.settings.pcf_type == "Gauss" {
                            let old_sigma = app.settings.pcf_sigma;
                            let resp = ui.add(
                                egui::Slider::new(&mut app.settings.pcf_sigma, 0.1..=10.0)
                                    .text("高斯σ")
                            );
                            if (app.settings.pcf_sigma - old_sigma).abs() > f32::EPSILON {
                                app.interface_interaction.anything_changed = true;
                            }
                            Self::add_tooltip(resp, ctx, "高斯采样的σ参数，影响软化范围");
                        }
                    }
                });

                // 阴影映射状态提示
                if app.settings.lights.iter().any(|light| matches!(light, Light::Directional { enabled: true, .. })) {
                    ui.label(RichText::new("✅ 检测到方向光源，阴影映射可用").color(Color32::LIGHT_GREEN).size(12.0));
                } else {
                    ui.label(RichText::new("⚠️ 需要至少一个启用的方向光源").color(Color32::DARK_GRAY).size(12.0));
                }
            }
        });

        ui.separator();
        let old_gamma = app.settings.use_gamma;
        let resp7 = ui.checkbox(&mut app.settings.use_gamma, "Gamma校正");
        if app.settings.use_gamma != old_gamma {
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(resp7, ctx, "应用伽马校正，使亮度显示更准确");

        // ACES色调映射开关
        let old_aces = app.settings.enable_aces;
        let resp = ui.checkbox(&mut app.settings.enable_aces, "启用ACES色调映射");
        if app.settings.enable_aces != old_aces {
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(
            resp,
            ctx,
            "让高动态范围颜色更自然，避免过曝和死黑，推荐开启",
        );

        let old_backface = app.settings.backface_culling;
        let resp8 = ui.checkbox(&mut app.settings.backface_culling, "背面剔除");
        if app.settings.backface_culling != old_backface {
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(resp8, ctx, "剔除背向相机的三角形面，提高渲染效率");

        let old_wireframe = app.settings.wireframe;
        let resp9 = ui.checkbox(&mut app.settings.wireframe, "线框模式");
        if app.settings.wireframe != old_wireframe {
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(resp9, ctx, "仅渲染三角形边缘，显示为线框");

        // 小三角形剔除设置
        ui.horizontal(|ui| {
            let old_cull = app.settings.cull_small_triangles;
            let resp = ui.checkbox(&mut app.settings.cull_small_triangles, "剔除小三角形");
            if app.settings.cull_small_triangles != old_cull {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "忽略投影后面积很小的三角形，提高性能");

            if app.settings.cull_small_triangles {
                let old_area = app.settings.min_triangle_area;
                let resp = ui.add(
                    egui::DragValue::new(&mut app.settings.min_triangle_area)
                        .speed(0.0001)
                        .range(0.0..=1.0)
                        .prefix("面积阈值："),
                );
                if (app.settings.min_triangle_area - old_area).abs() > f32::EPSILON {
                    app.interface_interaction.anything_changed = true;
                }
                Self::add_tooltip(resp, ctx, "小于此面积的三角形将被剔除（范围0.0-1.0）");
            }
        });

        ui.separator();

        // 纹理设置
        ui.horizontal(|ui| {
            ui.label("纹理文件 (覆盖MTL)：");
            let mut texture_path_str = app.settings.texture.clone().unwrap_or_default();
            let resp = ui.text_edit_singleline(&mut texture_path_str);
            Self::add_tooltip(resp.clone(), ctx, "选择自定义纹理，将覆盖MTL中的定义");

            if resp.changed() {
                if texture_path_str.is_empty() {
                    app.settings.texture = None;
                } else {
                    app.settings.texture = Some(texture_path_str);
                }

                // 纹理变化应该立即触发重绘
                app.interface_interaction.anything_changed = true;
            }

            if ui.button("浏览").clicked() {
                app.select_texture_file(); // 调用 render_ui.rs 中的方法
            }
        });
    }

    /// 物体变换控制面板
    fn ui_object_transform_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        // 位置控制
        ui.group(|ui| {
            ui.label("物体位置 (x,y,z)：");
            let old = app.settings.object_position.clone();
            let resp = ui.text_edit_singleline(&mut app.settings.object_position);
            if app.settings.object_position != old {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "输入物体的世界坐标，例如 0,0,0");
        });

        // 旋转控制（度）
        ui.group(|ui| {
            ui.label("物体旋转 (x,y,z，度)：");
            let old = app.settings.object_rotation.clone();
            let resp = ui.text_edit_singleline(&mut app.settings.object_rotation);
            if app.settings.object_rotation != old {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "输入旋转角度（度），例如 0,45,0");
        });

        // 缩放控制
        ui.group(|ui| {
            ui.label("物体缩放 (x,y,z)：");
            let old = app.settings.object_scale_xyz.clone();
            let resp = ui.text_edit_singleline(&mut app.settings.object_scale_xyz);
            if app.settings.object_scale_xyz != old {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "输入缩放比例，例如 1,1,1");
            ui.horizontal(|ui| {
                ui.label("全局缩放:");
                let old_scale = app.settings.object_scale;
                let resp = ui.add(
                    egui::Slider::new(&mut app.settings.object_scale, 0.1..=5.0)
                        .logarithmic(true)
                        .text("倍率"),
                );
                if app.settings.object_scale != old_scale {
                    app.interface_interaction.anything_changed = true;
                }
                Self::add_tooltip(resp, ctx, "整体缩放倍率，影响所有轴");
            });
        });
    }

    /// 背景与环境设置面板
    fn ui_background_settings(app: &mut RasterizerApp, ui: &mut egui::Ui) {
        // 背景图片选项
        let old_bg_image = app.settings.use_background_image;
        ui.checkbox(&mut app.settings.use_background_image, "使用背景图片");
        if app.settings.use_background_image != old_bg_image {
            app.interface_interaction.anything_changed = true;
            app.renderer.frame_buffer.invalidate_background_cache(); // 失效背景缓存
        }

        if app.settings.use_background_image {
            ui.horizontal(|ui| {
                let mut path_text = app
                    .settings
                    .background_image_path
                    .clone()
                    .unwrap_or_default();
                ui.label("背景图片:");
                let response = ui.text_edit_singleline(&mut path_text);

                if response.changed() {
                    if path_text.is_empty() {
                        app.settings.background_image_path = None;
                    } else {
                        app.settings.background_image_path = Some(path_text.clone());
                        app.status_message = format!("背景图片路径已设置: {path_text}");
                    }

                    app.interface_interaction.anything_changed = true;
                    app.renderer.frame_buffer.invalidate_background_cache(); // 失效背景缓存
                }

                if ui.button("浏览...").clicked() {
                    app.select_background_image();
                }
            });
        }

        // 渐变背景设置
        let old_gradient = app.settings.enable_gradient_background;
        ui.checkbox(&mut app.settings.enable_gradient_background, "使用渐变背景");
        if app.settings.enable_gradient_background != old_gradient {
            app.interface_interaction.anything_changed = true;
            app.renderer.frame_buffer.invalidate_background_cache(); // 失效背景缓存
        }

        if app.settings.enable_gradient_background {
            if app.settings.use_background_image && app.settings.background_image_path.is_some() {
                ui.label(
                    egui::RichText::new("注意：渐变背景将覆盖在背景图片上")
                        .color(Color32::DARK_GRAY),
                );
            }

            // 使用按需计算的颜色值
            let top_color = app.settings.get_gradient_top_color_vec();
            let mut top_color_array = [top_color.x, top_color.y, top_color.z];
            if ui.color_edit_button_rgb(&mut top_color_array).changed() {
                app.settings.gradient_top_color = format!(
                    "{},{},{}",
                    top_color_array[0], top_color_array[1], top_color_array[2]
                );

                app.interface_interaction.anything_changed = true;
                app.renderer.frame_buffer.invalidate_background_cache(); // 失效背景缓存
            }
            ui.label("渐变顶部颜色");

            let bottom_color = app.settings.get_gradient_bottom_color_vec();
            let mut bottom_color_array = [bottom_color.x, bottom_color.y, bottom_color.z];
            if ui.color_edit_button_rgb(&mut bottom_color_array).changed() {
                app.settings.gradient_bottom_color = format!(
                    "{},{},{}",
                    bottom_color_array[0], bottom_color_array[1], bottom_color_array[2]
                );

                app.interface_interaction.anything_changed = true;
                app.renderer.frame_buffer.invalidate_background_cache(); // 失效背景缓存
            }
            ui.label("渐变底部颜色");
        }

        // 地面平面设置
        let old_ground = app.settings.enable_ground_plane;
        ui.checkbox(&mut app.settings.enable_ground_plane, "显示地面平面");
        if app.settings.enable_ground_plane != old_ground {
            app.interface_interaction.anything_changed = true;
        }

        if app.settings.enable_ground_plane {
            if app.settings.use_background_image && app.settings.background_image_path.is_some() {
                ui.label(
                    RichText::new("注意：地面平面将覆盖在背景图片上").color(Color32::DARK_GRAY),
                );
            }

            // 使用按需计算的地面颜色
            let ground_color = app.settings.get_ground_plane_color_vec();
            let mut ground_color_array = [ground_color.x, ground_color.y, ground_color.z];
            if ui.color_edit_button_rgb(&mut ground_color_array).changed() {
                app.settings.ground_plane_color = format!(
                    "{},{},{}",
                    ground_color_array[0], ground_color_array[1], ground_color_array[2]
                );

                app.interface_interaction.anything_changed = true;
            }
            ui.label("地面颜色");

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Slider::new(&mut app.settings.ground_plane_height, -10.0..=5.0)
                            .text("地面高度")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    app.interface_interaction.anything_changed = true;
                }

                // 自动适配按钮
                if ui.button("自动适配").clicked() {
                    if let Some(optimal_height) = app.calculate_optimal_ground_height() {
                        app.settings.ground_plane_height = optimal_height;

                        app.interface_interaction.anything_changed = true;
                        app.status_message = format!("地面高度已自动调整为 {optimal_height:.2}");
                    } else {
                        app.status_message = "无法计算地面高度：请先加载模型".to_string();
                    }
                }
            });
        }
    }

    fn ui_camera_settings_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label("相机位置 (x,y,z)：");
            let old = app.settings.camera_from.clone();
            let resp = ui.text_edit_singleline(&mut app.settings.camera_from);
            if app.settings.camera_from != old {
                if let Some(scene) = &mut app.scene {
                    if let Ok(from) = parse_point3(&app.settings.camera_from) {
                        scene.active_camera.params.position = from;
                        scene.active_camera.update_matrices();
                        app.interface_interaction.anything_changed = true;
                    }
                }
            }
            Self::add_tooltip(resp, ctx, "相机的位置坐标，格式为x,y,z");
        });

        ui.horizontal(|ui| {
            ui.label("相机目标 (x,y,z)：");
            let old = app.settings.camera_at.clone();
            let resp = ui.text_edit_singleline(&mut app.settings.camera_at);
            if app.settings.camera_at != old {
                if let Some(scene) = &mut app.scene {
                    if let Ok(at) = parse_point3(&app.settings.camera_at) {
                        scene.active_camera.params.target = at;
                        scene.active_camera.update_matrices();
                        app.interface_interaction.anything_changed = true;
                    }
                }
            }
            Self::add_tooltip(resp, ctx, "相机看向的目标点坐标，格式为x,y,z");
        });

        ui.horizontal(|ui| {
            ui.label("相机上方向 (x,y,z)：");
            let old = app.settings.camera_up.clone();
            let resp = ui.text_edit_singleline(&mut app.settings.camera_up);
            if app.settings.camera_up != old {
                if let Some(scene) = &mut app.scene {
                    if let Ok(up) = parse_vec3(&app.settings.camera_up) {
                        scene.active_camera.params.up = up.normalize();
                        scene.active_camera.update_matrices();
                        app.interface_interaction.anything_changed = true;
                    }
                }
            }
            Self::add_tooltip(resp, ctx, "相机的上方向向量，格式为x,y,z");
        });

        ui.horizontal(|ui| {
            ui.label("视场角 (度)：");
            let old_fov = app.settings.camera_fov;
            let resp = ui.add(egui::Slider::new(
                &mut app.settings.camera_fov,
                10.0..=120.0,
            ));
            if (app.settings.camera_fov - old_fov).abs() > 0.1 {
                if let Some(scene) = &mut app.scene {
                    if let ProjectionType::Perspective { fov_y_degrees, .. } =
                        &mut scene.active_camera.params.projection
                    {
                        *fov_y_degrees = app.settings.camera_fov;
                        scene.active_camera.update_matrices();
                        app.interface_interaction.anything_changed = true;
                    }
                }
            }
            Self::add_tooltip(resp, ctx, "相机视场角，值越大视野范围越广（鱼眼效果）");
        });
        ui.separator();

        // 相机交互控制设置（敏感度设置不需要立即响应，它们只影响交互行为）
        ui.group(|ui| {
            ui.label(RichText::new("相机交互控制").size(16.0).strong());
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("平移敏感度:");
                let resp = ui.add(
                    egui::Slider::new(&mut app.camera_pan_sensitivity, 0.1..=5.0)
                        .step_by(0.1)
                        .text("倍率"),
                );
                Self::add_tooltip(
                    resp,
                    ctx,
                    "鼠标拖拽时的平移敏感度\n数值越大，鼠标移动相同距离时相机移动越快",
                );
            });

            ui.horizontal(|ui| {
                ui.label("旋转敏感度:");
                let resp = ui.add(
                    egui::Slider::new(&mut app.camera_orbit_sensitivity, 0.1..=5.0)
                        .step_by(0.1)
                        .text("倍率"),
                );
                Self::add_tooltip(
                    resp,
                    ctx,
                    "Shift+拖拽时的轨道旋转敏感度\n数值越大，鼠标移动相同距离时相机旋转角度越大",
                );
            });

            ui.horizontal(|ui| {
                ui.label("缩放敏感度:");
                let resp = ui.add(
                    egui::Slider::new(&mut app.camera_dolly_sensitivity, 0.1..=5.0)
                        .step_by(0.1)
                        .text("倍率"),
                );
                Self::add_tooltip(
                    resp,
                    ctx,
                    "鼠标滚轮的推拉缩放敏感度\n数值越大，滚轮滚动相同距离时相机前后移动越快",
                );
            });

            // 重置按钮
            ui.horizontal(|ui| {
                if ui.button("重置交互敏感度").clicked() {
                    app.camera_pan_sensitivity = 1.0;
                    app.camera_orbit_sensitivity = 1.0;
                    app.camera_dolly_sensitivity = 1.0;
                }

                // 预设敏感度按钮
                if ui.button("精确模式").clicked() {
                    app.camera_pan_sensitivity = 0.3;
                    app.camera_orbit_sensitivity = 0.3;
                    app.camera_dolly_sensitivity = 0.3;
                }

                if ui.button("快速模式").clicked() {
                    app.camera_pan_sensitivity = 2.0;
                    app.camera_orbit_sensitivity = 2.0;
                    app.camera_dolly_sensitivity = 2.0;
                }
            });

            // 交互说明
            ui.group(|ui| {
                ui.label(RichText::new("交互说明:").size(14.0).strong());
                ui.label("• 拖拽 - 平移相机视角");
                ui.label("• Shift + 拖拽 - 围绕目标旋转");
                ui.label("• 鼠标滚轮 - 推拉缩放");
                ui.label(
                    RichText::new("注意: 需要在中央渲染区域操作")
                        .size(12.0)
                        .color(Color32::DARK_GRAY),
                );
            });
        });
    }

    /// 光照设置面板
    fn ui_lighting_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        // 总光照开关
        let resp = ui
            .checkbox(&mut app.settings.use_lighting, "启用光照")
            .on_hover_text("总光照开关，关闭则仅使用环境光");
        if resp.changed() {
            app.interface_interaction.anything_changed = true;
        }

        ui.separator();

        // 环境光设置
        ui.horizontal(|ui| {
            ui.label("环境光颜色:");
            let ambient_color_vec = app.settings.get_ambient_color_vec();
            let mut ambient_color_rgb = [
                ambient_color_vec.x,
                ambient_color_vec.y,
                ambient_color_vec.z,
            ];
            let resp = ui.color_edit_button_rgb(&mut ambient_color_rgb);
            if resp.changed() {
                app.settings.ambient_color = format!(
                    "{},{},{}",
                    ambient_color_rgb[0], ambient_color_rgb[1], ambient_color_rgb[2]
                );
                app.interface_interaction.anything_changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("环境光强度:");
            let resp = ui.add(egui::Slider::new(&mut app.settings.ambient, 0.0..=1.0));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
        });
        ui.separator();

        // 统一的材质通用属性控制
        ui.group(|ui| {
    ui.label(RichText::new("🎨 材质通用属性").size(16.0).strong());
    ui.separator();

    // 基础颜色（通用于PBR和Phong）
    ui.horizontal(|ui| {
        ui.label("基础颜色 (Base Color / Diffuse):");
        let base_color_vec = if app.settings.use_pbr {
            parse_vec3(&app.settings.base_color)
        } else {
            parse_vec3(&app.settings.diffuse_color)
        }.unwrap_or_else(|_| nalgebra::Vector3::new(0.8, 0.8, 0.8));

        let mut base_color_rgb = [base_color_vec.x, base_color_vec.y, base_color_vec.z];
        let resp = ui.color_edit_button_rgb(&mut base_color_rgb);
        if resp.changed() {
            let color_str = format!(
                "{:.3},{:.3},{:.3}",
                base_color_rgb[0], base_color_rgb[1], base_color_rgb[2]
            );

            // 同时更新PBR和Phong的颜色设置
            if app.settings.use_pbr {
                app.settings.base_color = color_str;
            } else {
                app.settings.diffuse_color = color_str;
            }
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(
            resp,
            ctx,
            "材质的基础颜色\nPBR模式下为Base Color，Phong模式下为Diffuse Color",
        );
    });

    // 透明度控制（通用于PBR和Phong）
    ui.horizontal(|ui| {
        ui.label("透明度 (Alpha)：");
        let resp = ui.add(egui::Slider::new(&mut app.settings.alpha, 0.0..=1.0));
        if resp.changed() {
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(
            resp,
            ctx,
            "材质透明度，0为完全透明，1为完全不透明\n适用于PBR和Phong着色模型\n调整此值可立即看到透明效果",
        );
    });

    // 自发光控制（通用于PBR和Phong）
    ui.horizontal(|ui| {
        ui.label("自发光颜色 (Emissive):");
        let emissive_color_vec = parse_vec3(&app.settings.emissive)
            .unwrap_or_else(|_| nalgebra::Vector3::new(0.0, 0.0, 0.0));
        let mut emissive_color_rgb = [
            emissive_color_vec.x,
            emissive_color_vec.y,
            emissive_color_vec.z,
        ];
        let resp = ui.color_edit_button_rgb(&mut emissive_color_rgb);
        if resp.changed() {
            app.settings.emissive = format!(
                "{:.3},{:.3},{:.3}",
                emissive_color_rgb[0], emissive_color_rgb[1], emissive_color_rgb[2]
            );
            app.interface_interaction.anything_changed = true;
        }
        Self::add_tooltip(
            resp,
            ctx,
            "材质的自发光颜色，表示材质本身发出的光\n不受光照影响，适用于发光物体",
        );
    });
});

        ui.separator();

        // 直接光源管理
        if app.settings.use_lighting {
            ui.horizontal(|ui| {
                if ui.button("➕ 添加方向光").clicked() {
                    app.settings.lights.push(Light::directional(
                        nalgebra::Vector3::new(0.0, -1.0, -1.0),
                        nalgebra::Vector3::new(1.0, 1.0, 1.0),
                        0.8, // 直接使用合理的默认强度
                    ));
                    app.interface_interaction.anything_changed = true;
                }

                if ui.button("➕ 添加点光源").clicked() {
                    app.settings.lights.push(Light::point(
                        nalgebra::Point3::new(0.0, 2.0, 0.0),
                        nalgebra::Vector3::new(1.0, 1.0, 1.0),
                        1.0, // 直接使用合理的默认强度
                        Some((1.0, 0.09, 0.032)),
                    ));
                    app.interface_interaction.anything_changed = true;
                }

                ui.separator();
                ui.label(format!("光源总数: {}", app.settings.lights.len()));
            });

            ui.separator();

            // 可编辑的光源列表
            let mut to_remove = Vec::new();
            for (i, light) in app.settings.lights.iter_mut().enumerate() {
                let mut light_changed = false;

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // 删除按钮
                        if ui.button("🗑").on_hover_text("删除此光源").clicked() {
                            to_remove.push(i);
                            app.interface_interaction.anything_changed = true;
                        }

                        // 光源类型和编号
                        match light {
                            Light::Directional { .. } => {
                                ui.label(format!("🔦 方向光 #{}", i + 1));
                            }
                            Light::Point { .. } => {
                                ui.label(format!("💡 点光源 #{}", i + 1));
                            }
                        }
                    });

                    // 光源参数编辑
                    match light {
                        Light::Directional {
                            enabled,
                            direction_str,
                            color_str,
                            intensity,
                            ..
                        } => {
                            ui.horizontal(|ui| {
                                let resp = ui.checkbox(enabled, "启用");
                                if resp.changed() {
                                    light_changed = true;
                                }

                                if *enabled {
                                    // 独立的强度控制
                                    let resp = ui.add(
                                        egui::Slider::new(intensity, 0.0..=3.0)
                                            .text("强度")
                                            .step_by(0.1),
                                    );
                                    if resp.changed() {
                                        light_changed = true;
                                    }
                                }
                            });

                            if *enabled {
                                ui.horizontal(|ui| {
                                    ui.label("方向 (x,y,z):");
                                    let resp = ui.text_edit_singleline(direction_str);
                                    if resp.changed() {
                                        light_changed = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("颜色:");
                                    let color_vec = parse_vec3(color_str)
                                        .unwrap_or_else(|_| nalgebra::Vector3::new(1.0, 1.0, 1.0));
                                    let mut color_rgb = [color_vec.x, color_vec.y, color_vec.z];
                                    let resp = ui.color_edit_button_rgb(&mut color_rgb);
                                    if resp.changed() {
                                        *color_str = format!(
                                            "{},{},{}",
                                            color_rgb[0], color_rgb[1], color_rgb[2]
                                        );
                                        light_changed = true;
                                    }
                                });
                            }
                        }
                        Light::Point {
                            enabled,
                            position_str,
                            color_str,
                            intensity,
                            constant_attenuation,
                            linear_attenuation,
                            quadratic_attenuation,
                            ..
                        } => {
                            ui.horizontal(|ui| {
                                let resp = ui.checkbox(enabled, "启用");
                                if resp.changed() {
                                    light_changed = true;
                                }

                                if *enabled {
                                    // 独立的强度控制
                                    let resp = ui.add(
                                        egui::Slider::new(intensity, 0.0..=10.0)
                                            .text("强度")
                                            .step_by(0.1),
                                    );
                                    if resp.changed() {
                                        light_changed = true;
                                    }
                                }
                            });

                            if *enabled {
                                ui.horizontal(|ui| {
                                    ui.label("位置 (x,y,z):");
                                    let resp = ui.text_edit_singleline(position_str);
                                    if resp.changed() {
                                        light_changed = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("颜色:");
                                    let color_vec = parse_vec3(color_str)
                                        .unwrap_or_else(|_| nalgebra::Vector3::new(1.0, 1.0, 1.0));
                                    let mut color_rgb = [color_vec.x, color_vec.y, color_vec.z];
                                    let resp = ui.color_edit_button_rgb(&mut color_rgb);
                                    if resp.changed() {
                                        *color_str = format!(
                                            "{},{},{}",
                                            color_rgb[0], color_rgb[1], color_rgb[2]
                                        );
                                        light_changed = true;
                                    }
                                });

                                // 衰减设置
                                ui.collapsing("衰减参数", |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("常数:");
                                        let resp = ui.add(
                                            egui::DragValue::new(constant_attenuation)
                                                .speed(0.05)
                                                .range(0.0..=10.0),
                                        );
                                        if resp.changed() {
                                            light_changed = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("线性:");
                                        let resp = ui.add(
                                            egui::DragValue::new(linear_attenuation)
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if resp.changed() {
                                            light_changed = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("二次:");
                                        let resp = ui.add(
                                            egui::DragValue::new(quadratic_attenuation)
                                                .speed(0.001)
                                                .range(0.0..=0.5),
                                        );
                                        if resp.changed() {
                                            light_changed = true;
                                        }
                                    });
                                    ui.small("💡 推荐值: 常数=1.0, 线性=0.09, 二次=0.032");
                                });
                            }
                        }
                    }
                });

                if light_changed {
                    let _ = light.update_runtime_fields();
                    app.interface_interaction.anything_changed = true;
                }
            }

            // 删除标记的光源
            for &index in to_remove.iter().rev() {
                app.settings.lights.remove(index);
            }

            // 如果没有光源，显示提示
            if app.settings.lights.is_empty() {
                ui.group(|ui| {
                    ui.label("💡 提示：当前没有光源");
                    ui.label("点击上方的「➕ 添加」按钮来添加光源");
                });
            }
        }
    }

    /// PBR材质设置面板
    fn ui_pbr_material_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label("金属度 (Metallic)：");
            let resp = ui.add(egui::Slider::new(&mut app.settings.metallic, 0.0..=1.0));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "材质的金属特性，0为非金属，1为纯金属");
        });

        ui.horizontal(|ui| {
            ui.label("粗糙度 (Roughness)：");
            let resp = ui.add(egui::Slider::new(&mut app.settings.roughness, 0.0..=1.0));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "材质的粗糙程度，影响高光的散射");
        });

        ui.horizontal(|ui| {
            ui.label("环境光遮蔽 (AO)：");
            let resp = ui.add(egui::Slider::new(
                &mut app.settings.ambient_occlusion,
                0.0..=1.0,
            ));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "环境光遮蔽程度，模拟凹陷处的阴影");
        });
    }

    /// 简化后的Phong材质设置面板
    fn ui_phong_material_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label("镜面反射颜色：");
            let specular_color_vec = parse_vec3(&app.settings.specular_color)
                .unwrap_or_else(|_| nalgebra::Vector3::new(0.5, 0.5, 0.5));
            let mut specular_color_rgb = [
                specular_color_vec.x,
                specular_color_vec.y,
                specular_color_vec.z,
            ];
            let resp = ui.color_edit_button_rgb(&mut specular_color_rgb);
            if resp.changed() {
                app.settings.specular_color = format!(
                    "{:.3},{:.3},{:.3}",
                    specular_color_rgb[0], specular_color_rgb[1], specular_color_rgb[2]
                );
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "高光的颜色");
        });

        ui.horizontal(|ui| {
            ui.label("漫反射强度：");
            let resp = ui.add(egui::Slider::new(
                &mut app.settings.diffuse_intensity,
                0.0..=2.0,
            ));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "漫反射光的强度倍数");
        });

        ui.horizontal(|ui| {
            ui.label("镜面反射强度：");
            let resp = ui.add(egui::Slider::new(
                &mut app.settings.specular_intensity,
                0.0..=2.0,
            ));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "高光的强度倍数");
        });

        ui.horizontal(|ui| {
            ui.label("光泽度：");
            let resp = ui.add(egui::Slider::new(&mut app.settings.shininess, 1.0..=100.0));
            if resp.changed() {
                app.interface_interaction.anything_changed = true;
            }
            Self::add_tooltip(resp, ctx, "高光的锐利程度，值越大越集中");
        });
    }

    /// 动画设置面板
    fn ui_animation_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label("旋转圈数:");
            let resp = ui.add(
                egui::DragValue::new(&mut app.settings.rotation_cycles)
                    .speed(0.1)
                    .range(0.1..=10.0),
            );
            Self::add_tooltip(resp, ctx, "动画完成的旋转圈数，影响生成的总帧数");
        });

        ui.horizontal(|ui| {
            ui.label("视频生成及预渲染帧率 (FPS):");
            let resp = ui.add(
                egui::DragValue::new(&mut app.settings.fps)
                    .speed(1)
                    .range(1..=60),
            );
            Self::add_tooltip(resp, ctx, "生成视频的每秒帧数");
        });

        let (_, seconds_per_rotation, frames_per_rotation) =
            calculate_rotation_parameters(app.settings.rotation_speed, app.settings.fps);
        let total_frames = (frames_per_rotation as f32 * app.settings.rotation_cycles) as usize;
        let total_seconds = seconds_per_rotation * app.settings.rotation_cycles;

        ui.label(format!(
            "估计总帧数: {total_frames} (视频长度: {total_seconds:.1}秒)"
        ));

        // 动画类型选择
        ui.horizontal(|ui| {
            ui.label("动画类型:");
            let current_animation_type = app.settings.animation_type.clone();
            egui::ComboBox::from_id_salt("animation_type_combo")
                .selected_text(match current_animation_type {
                    AnimationType::CameraOrbit => "相机轨道旋转",
                    AnimationType::ObjectLocalRotation => "物体局部旋转",
                    AnimationType::None => "无动画",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.settings.animation_type,
                        AnimationType::CameraOrbit,
                        "相机轨道旋转",
                    );
                    ui.selectable_value(
                        &mut app.settings.animation_type,
                        AnimationType::ObjectLocalRotation,
                        "物体局部旋转",
                    );
                    ui.selectable_value(
                        &mut app.settings.animation_type,
                        AnimationType::None,
                        "无动画",
                    );
                });
        });

        // 旋转轴选择 (仅当动画类型不是 None 时显示)
        if app.settings.animation_type != AnimationType::None {
            ui.horizontal(|ui| {
                ui.label("旋转轴:");
                let current_rotation_axis = app.settings.rotation_axis.clone();
                egui::ComboBox::from_id_salt("rotation_axis_combo")
                    .selected_text(match current_rotation_axis {
                        RotationAxis::X => "X 轴",
                        RotationAxis::Y => "Y 轴",
                        RotationAxis::Z => "Z 轴",
                        RotationAxis::Custom => "自定义轴",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.settings.rotation_axis,
                            RotationAxis::X,
                            "X 轴",
                        );
                        ui.selectable_value(
                            &mut app.settings.rotation_axis,
                            RotationAxis::Y,
                            "Y 轴",
                        );
                        ui.selectable_value(
                            &mut app.settings.rotation_axis,
                            RotationAxis::Z,
                            "Z 轴",
                        );
                        ui.selectable_value(
                            &mut app.settings.rotation_axis,
                            RotationAxis::Custom,
                            "自定义轴",
                        );
                    });
            });

            if app.settings.rotation_axis == RotationAxis::Custom {
                ui.horizontal(|ui| {
                    ui.label("自定义轴 (x,y,z):");
                    let resp = ui.text_edit_singleline(&mut app.settings.custom_rotation_axis);
                    Self::add_tooltip(resp, ctx, "输入自定义旋转轴，例如 1,0,0 或 0.707,0.707,0");
                });
            }
        }
        Self::add_tooltip(
            ui.label(""),
            ctx,
            "选择实时渲染和视频生成时的动画效果和旋转轴",
        );

        // 简化预渲染模式复选框逻辑
        let pre_render_enabled = app.can_toggle_pre_render();
        let mut pre_render_value = app.pre_render_mode;

        let pre_render_resp = ui.add_enabled(
            pre_render_enabled,
            egui::Checkbox::new(&mut pre_render_value, "启用预渲染模式"),
        );

        if pre_render_resp.changed() && pre_render_value != app.pre_render_mode {
            app.toggle_pre_render_mode();
        }
        Self::add_tooltip(
            pre_render_resp,
            ctx,
            "启用后，首次开始实时渲染时会预先计算所有帧，\n然后以选定帧率无卡顿播放。\n要求更多内存，但播放更流畅。",
        );

        ui.horizontal(|ui| {
            ui.label("旋转速度 (实时渲染):");
            let resp = ui.add(egui::Slider::new(
                &mut app.settings.rotation_speed,
                0.1..=5.0,
            ));
            Self::add_tooltip(resp, ctx, "实时渲染中的旋转速度倍率");
        });
    }

    /// 按钮控制面板
    fn ui_button_controls_panel(app: &mut RasterizerApp, ui: &mut egui::Ui, ctx: &Context) {
        ui.add_space(20.0);

        // 计算按钮的统一宽度
        let available_width = ui.available_width();
        let spacing = ui.spacing().item_spacing.x;

        // 第一行：2个按钮等宽
        let button_width_row1 = (available_width - spacing) / 2.0;

        // 第二行：2个按钮等宽
        let button_width_row2 = (available_width - spacing) / 2.0;

        // 第三行：2个按钮等宽
        let button_width_row3 = (available_width - spacing) / 2.0;

        let button_height = 40.0;

        // === 第一行：恢复默认值 + 开始渲染 ===
        ui.horizontal(|ui| {
            // 恢复默认值按钮
            let reset_button = ui.add_sized(
                [button_width_row1, button_height],
                egui::Button::new(RichText::new("恢复默认值").size(15.0)),
            );

            if reset_button.clicked() {
                app.reset_to_defaults();
            }

            Self::add_tooltip(
                reset_button,
                ctx,
                "重置所有渲染参数为默认值，保留文件路径设置",
            );

            // 渲染按钮
            let render_button = ui.add_sized(
                [button_width_row1, button_height],
                egui::Button::new(RichText::new("开始渲染").size(18.0).strong()),
            );

            if render_button.clicked() {
                app.render(ctx);
            }

            Self::add_tooltip(render_button, ctx, "快捷键: Ctrl+R");
        });

        ui.add_space(10.0);

        // === 第二行：动画渲染 + 截图 ===
        ui.horizontal(|ui| {
            // 动画渲染按钮
            let realtime_button_text = if app.is_realtime_rendering {
                "停止动画渲染"
            } else if app.pre_render_mode {
                "开始动画渲染 (预渲染模式)"
            } else {
                "开始动画渲染 (实时模式)"
            };

            let realtime_button = ui.add_enabled(
                app.can_render_animation(),
                egui::Button::new(RichText::new(realtime_button_text).size(15.0))
                    .min_size(Vec2::new(button_width_row2, button_height)),
            );

            if realtime_button.clicked() {
                // 如果当前在播放预渲染帧，点击时只是停止播放
                if app.is_realtime_rendering && app.pre_render_mode {
                    app.is_realtime_rendering = false;
                    app.status_message = "已停止动画渲染".to_string();
                }
                // 否则切换实时渲染状态
                else if !app.is_realtime_rendering {
                    // 使用CoreMethods中的开始动画渲染方法
                    if let Err(e) = app.start_animation_rendering() {
                        app.set_error(e);
                    }
                } else {
                    // 使用CoreMethods中的停止动画渲染方法
                    app.stop_animation_rendering();
                }
            }

            // 更新工具提示文本
            let tooltip_text = if app.pre_render_mode {
                "启动动画渲染（预渲染模式）\n• 首次启动会预先计算所有帧\n• 然后以目标帧率流畅播放\n• 需要更多内存但播放更流畅"
            } else {
                "启动动画渲染（实时模式）\n• 每帧实时计算和渲染\n• 帧率取决于硬件性能\n• 内存占用较少"
            };

            Self::add_tooltip(realtime_button, ctx, tooltip_text);

            // 截图按钮
            let screenshot_button = ui.add_enabled(
                app.rendered_image.is_some(),
                egui::Button::new(RichText::new("截图").size(15.0))
                    .min_size(Vec2::new(button_width_row2, button_height)),
            );

            if screenshot_button.clicked() {
                match app.take_screenshot() {
                    Ok(path) => {
                        app.status_message = format!("截图已保存至 {path}");
                    }
                    Err(e) => {
                        app.set_error(format!("截图失败: {e}"));
                    }
                }
            }

            Self::add_tooltip(screenshot_button, ctx, "保存当前渲染结果为图片文件");
        });

        ui.add_space(10.0);

        // === 第三行：生成视频 + 清空缓冲区 ===
        ui.horizontal(|ui| {
            let video_button_text = if app.is_generating_video {
                let progress = app.video_progress.load(Ordering::SeqCst);

                // 使用通用函数计算实际帧数
                let (_, _, frames_per_rotation) =
                    calculate_rotation_parameters(app.settings.rotation_speed, app.settings.fps);
                let total_frames =
                    (frames_per_rotation as f32 * app.settings.rotation_cycles) as usize;

                let percent = (progress as f32 / total_frames as f32 * 100.0).round();
                format!("生成视频中... {percent}%")
            } else if app.ffmpeg_available {
                "生成视频".to_string()
            } else {
                "生成视频 (需ffmpeg)".to_string()
            };

            let is_video_button_enabled = app.can_generate_video();

            // 视频生成按钮
            let video_button_response = ui.add_enabled(
                is_video_button_enabled,
                egui::Button::new(RichText::new(video_button_text).size(15.0))
                    .min_size(Vec2::new(button_width_row3, button_height)),
            );

            if video_button_response.clicked() {
                app.start_video_generation(ctx);
            }
            Self::add_tooltip(
                video_button_response,
                ctx,
                "在后台渲染多帧并生成MP4视频。\n需要系统安装ffmpeg。\n生成过程不会影响UI使用。",
            );

            // 清空缓冲区按钮
            let is_clear_buffer_enabled = app.can_clear_buffer();

            let clear_buffer_response = ui.add_enabled(
                is_clear_buffer_enabled,
                egui::Button::new(RichText::new("清空缓冲区").size(15.0))
                    .min_size(Vec2::new(button_width_row3, button_height)),
            );

            if clear_buffer_response.clicked() {
                // 使用CoreMethods实现
                app.clear_pre_rendered_frames();
            }
            Self::add_tooltip(
                clear_buffer_response,
                ctx,
                "清除已预渲染的动画帧，释放内存。\n请先停止动画渲染再清除缓冲区。",
            );
        });
    }

    /// 渲染信息面板
    fn ui_render_info_panel(app: &mut RasterizerApp, ui: &mut egui::Ui) {
        // 渲染信息
        if let Some(time) = app.last_render_time {
            ui.separator();
            ui.label(format!("渲染耗时: {time:.2?}"));

            // 显示场景统计信息（直接使用SceneStats）
            if let Some(scene) = &app.scene {
                let stats = scene.get_scene_stats();
                ui.label(format!("网格数量: {}", stats.mesh_count));
                ui.label(format!("三角形数量: {}", stats.triangle_count));
                ui.label(format!("顶点数量: {}", stats.vertex_count));
                ui.label(format!("材质数量: {}", stats.material_count));
                ui.label(format!("光源数量: {}", stats.light_count));
            }
        }

        // FPS显示
        if app.is_realtime_rendering {
            let (fps_text, fps_color) = app.get_fps_display();
            ui.separator();
            ui.label(RichText::new(fps_text).color(fps_color).size(16.0));
        }
    }
}
