use crate::io::render_settings::{AnimationType, RenderSettings, RotationAxis};
use crate::material_system::light::Light;
use nalgebra::{Point3, Vector3};
use std::path::Path;
use toml::Value;

/// 🔥 **TOML配置管理器** - 统一处理所有配置的读写
pub struct TomlConfigLoader;

impl TomlConfigLoader {
    /// 🔥 **从TOML文件加载完整配置**
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<RenderSettings, String> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("读取配置文件失败: {}", e))?;

        Self::load_from_content(&content)
    }

    /// 🔥 **从TOML内容字符串加载配置**
    pub fn load_from_content(content: &str) -> Result<RenderSettings, String> {
        let toml_value: Value =
            toml::from_str(content).map_err(|e| format!("解析TOML失败: {}", e))?;

        Self::parse_toml_to_settings(toml_value)
    }

    /// 🔥 **保存配置到TOML文件**
    pub fn save_to_file<P: AsRef<Path>>(settings: &RenderSettings, path: P) -> Result<(), String> {
        let toml_content = Self::settings_to_toml(settings)?;
        std::fs::write(path, toml_content).map_err(|e| format!("写入配置文件失败: {}", e))
    }

    /// 🔥 **生成示例配置文件**
    pub fn generate_example_config<P: AsRef<Path>>(path: P) -> Result<(), String> {
        let example_settings = Self::create_example_settings();
        Self::save_to_file(&example_settings, path)
    }

    // ===== 🔥 **TOML -> RenderSettings 转换** =====

    fn parse_toml_to_settings(toml: Value) -> Result<RenderSettings, String> {
        let mut settings = RenderSettings::default();

        // [files] 部分
        if let Some(files) = toml.get("files").and_then(|v| v.as_table()) {
            Self::parse_files_section(&mut settings, files)?;
        }

        // [render] 部分
        if let Some(render) = toml.get("render").and_then(|v| v.as_table()) {
            Self::parse_render_section(&mut settings, render)?;
        }

        // [camera] 部分
        if let Some(camera) = toml.get("camera").and_then(|v| v.as_table()) {
            Self::parse_camera_section(&mut settings, camera)?;
        }

        // [object] 部分
        if let Some(object) = toml.get("object").and_then(|v| v.as_table()) {
            Self::parse_object_section(&mut settings, object)?;
        }

        // [lighting] 部分
        if let Some(lighting) = toml.get("lighting").and_then(|v| v.as_table()) {
            Self::parse_lighting_section(&mut settings, lighting)?;
        }

        // 🔥 **[[light]] 数组 - 多光源支持**
        settings.lights = Self::parse_lights_array(&toml)?;

        // [material] 部分
        if let Some(material) = toml.get("material").and_then(|v| v.as_table()) {
            Self::parse_material_section(&mut settings, material)?;
        }

        // [background] 部分
        if let Some(background) = toml.get("background").and_then(|v| v.as_table()) {
            Self::parse_background_section(&mut settings, background)?;
        }

        // [animation] 部分
        if let Some(animation) = toml.get("animation").and_then(|v| v.as_table()) {
            Self::parse_animation_section(&mut settings, animation)?;
        }

        Ok(settings)
    }

    // ===== 🔥 **各个section的解析方法** =====

    fn parse_files_section(
        settings: &mut RenderSettings,
        files: &toml::Table,
    ) -> Result<(), String> {
        if let Some(obj) = files.get("obj").and_then(|v| v.as_str()) {
            settings.obj = Some(obj.to_string());
        }
        if let Some(output) = files.get("output").and_then(|v| v.as_str()) {
            settings.output = output.to_string();
        }
        if let Some(output_dir) = files.get("output_dir").and_then(|v| v.as_str()) {
            settings.output_dir = output_dir.to_string();
        }
        if let Some(texture) = files.get("texture").and_then(|v| v.as_str()) {
            settings.texture = Some(texture.to_string());
        }
        if let Some(bg_image) = files.get("background_image").and_then(|v| v.as_str()) {
            settings.background_image_path = Some(bg_image.to_string());
        }
        Ok(())
    }

    fn parse_render_section(
        settings: &mut RenderSettings,
        render: &toml::Table,
    ) -> Result<(), String> {
        if let Some(width) = render.get("width").and_then(|v| v.as_integer()) {
            settings.width = width as usize;
        }
        if let Some(height) = render.get("height").and_then(|v| v.as_integer()) {
            settings.height = height as usize;
        }
        if let Some(projection) = render.get("projection").and_then(|v| v.as_str()) {
            settings.projection = projection.to_string();
        }
        if let Some(use_zbuffer) = render.get("use_zbuffer").and_then(|v| v.as_bool()) {
            settings.use_zbuffer = use_zbuffer;
        }
        if let Some(colorize) = render.get("colorize").and_then(|v| v.as_bool()) {
            settings.colorize = colorize;
        }
        if let Some(use_texture) = render.get("use_texture").and_then(|v| v.as_bool()) {
            settings.use_texture = use_texture;
        }
        if let Some(use_gamma) = render.get("use_gamma").and_then(|v| v.as_bool()) {
            settings.use_gamma = use_gamma;
        }
        if let Some(backface_culling) = render.get("backface_culling").and_then(|v| v.as_bool()) {
            settings.backface_culling = backface_culling;
        }
        if let Some(wireframe) = render.get("wireframe").and_then(|v| v.as_bool()) {
            settings.wireframe = wireframe;
        }
        if let Some(use_multithreading) = render.get("use_multithreading").and_then(|v| v.as_bool())
        {
            settings.use_multithreading = use_multithreading;
        }
        if let Some(cull_small_triangles) =
            render.get("cull_small_triangles").and_then(|v| v.as_bool())
        {
            settings.cull_small_triangles = cull_small_triangles;
        }
        if let Some(min_triangle_area) = render.get("min_triangle_area").and_then(|v| v.as_float())
        {
            settings.min_triangle_area = min_triangle_area as f32;
        }
        if let Some(save_depth) = render.get("save_depth").and_then(|v| v.as_bool()) {
            settings.save_depth = save_depth;
        }
        Ok(())
    }

    fn parse_camera_section(
        settings: &mut RenderSettings,
        camera: &toml::Table,
    ) -> Result<(), String> {
        if let Some(from) = camera.get("from").and_then(|v| v.as_str()) {
            settings.camera_from = from.to_string();
        }
        if let Some(at) = camera.get("at").and_then(|v| v.as_str()) {
            settings.camera_at = at.to_string();
        }
        if let Some(up) = camera.get("up").and_then(|v| v.as_str()) {
            settings.camera_up = up.to_string();
        }
        if let Some(fov) = camera.get("fov").and_then(|v| v.as_float()) {
            settings.camera_fov = fov as f32;
        }
        Ok(())
    }

    fn parse_object_section(
        settings: &mut RenderSettings,
        object: &toml::Table,
    ) -> Result<(), String> {
        if let Some(position) = object.get("position").and_then(|v| v.as_str()) {
            settings.object_position = position.to_string();
        }
        if let Some(rotation) = object.get("rotation").and_then(|v| v.as_str()) {
            settings.object_rotation = rotation.to_string();
        }
        if let Some(scale_xyz) = object.get("scale_xyz").and_then(|v| v.as_str()) {
            settings.object_scale_xyz = scale_xyz.to_string();
        }
        if let Some(scale) = object.get("scale").and_then(|v| v.as_float()) {
            settings.object_scale = scale as f32;
        }
        Ok(())
    }

    fn parse_lighting_section(
        settings: &mut RenderSettings,
        lighting: &toml::Table,
    ) -> Result<(), String> {
        if let Some(use_lighting) = lighting.get("use_lighting").and_then(|v| v.as_bool()) {
            settings.use_lighting = use_lighting;
        }
        if let Some(ambient) = lighting.get("ambient").and_then(|v| v.as_float()) {
            settings.ambient = ambient as f32;
        }
        if let Some(ambient_color) = lighting.get("ambient_color").and_then(|v| v.as_str()) {
            settings.ambient_color = ambient_color.to_string();
        }
        Ok(())
    }

    /// 🔥 **多光源解析 - 支持 [[light]] 数组语法**
    fn parse_lights_array(toml: &Value) -> Result<Vec<Light>, String> {
        let mut lights = Vec::new();

        if let Some(lights_array) = toml.get("light").and_then(|v| v.as_array()) {
            for (i, light_value) in lights_array.iter().enumerate() {
                if let Some(light_table) = light_value.as_table() {
                    let light = Self::parse_single_light(light_table)
                        .map_err(|e| format!("第{}个光源解析失败: {}", i + 1, e))?;
                    lights.push(light);
                }
            }
        }

        Ok(lights)
    }

    fn parse_single_light(light_table: &toml::Table) -> Result<Light, String> {
        let light_type = light_table
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("光源缺少type字段")?;

        let enabled = light_table
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let intensity = light_table
            .get("intensity")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;

        let color_str = light_table
            .get("color")
            .and_then(|v| v.as_str())
            .unwrap_or("1,1,1");

        let color_vec = crate::io::render_settings::parse_vec3(color_str)
            .map_err(|e| format!("解析光源颜色失败: {}", e))?;

        match light_type {
            "directional" => {
                let direction_str = light_table
                    .get("direction")
                    .and_then(|v| v.as_str())
                    .ok_or("方向光缺少direction字段")?;

                let direction_vec = crate::io::render_settings::parse_vec3(direction_str)
                    .map_err(|e| format!("解析方向光方向失败: {}", e))?;

                let mut light = Light::directional(direction_vec, color_vec, intensity);
                if let Light::Directional {
                    enabled: ref mut light_enabled,
                    ..
                } = light
                {
                    *light_enabled = enabled;
                }
                Ok(light)
            }
            "point" => {
                let position_str = light_table
                    .get("position")
                    .and_then(|v| v.as_str())
                    .ok_or("点光源缺少position字段")?;

                let position_point = crate::io::render_settings::parse_point3(position_str)
                    .map_err(|e| format!("解析点光源位置失败: {}", e))?;

                let constant = light_table
                    .get("constant_attenuation")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.0) as f32;
                let linear = light_table
                    .get("linear_attenuation")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.09) as f32;
                let quadratic = light_table
                    .get("quadratic_attenuation")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.032) as f32;

                let mut light = Light::point(
                    position_point,
                    color_vec,
                    intensity,
                    Some((constant, linear, quadratic)),
                );
                if let Light::Point {
                    enabled: ref mut light_enabled,
                    ..
                } = light
                {
                    *light_enabled = enabled;
                }
                Ok(light)
            }
            _ => Err(format!("未知的光源类型: {}", light_type)),
        }
    }

    fn parse_material_section(
        settings: &mut RenderSettings,
        material: &toml::Table,
    ) -> Result<(), String> {
        if let Some(use_phong) = material.get("use_phong").and_then(|v| v.as_bool()) {
            settings.use_phong = use_phong;
        }
        if let Some(use_pbr) = material.get("use_pbr").and_then(|v| v.as_bool()) {
            settings.use_pbr = use_pbr;
        }
        if let Some(diffuse_color) = material.get("diffuse_color").and_then(|v| v.as_str()) {
            settings.diffuse_color = diffuse_color.to_string();
        }
        if let Some(specular) = material.get("specular").and_then(|v| v.as_float()) {
            settings.specular = specular as f32;
        }
        if let Some(shininess) = material.get("shininess").and_then(|v| v.as_float()) {
            settings.shininess = shininess as f32;
        }
        if let Some(base_color) = material.get("base_color").and_then(|v| v.as_str()) {
            settings.base_color = base_color.to_string();
        }
        if let Some(metallic) = material.get("metallic").and_then(|v| v.as_float()) {
            settings.metallic = metallic as f32;
        }
        if let Some(roughness) = material.get("roughness").and_then(|v| v.as_float()) {
            settings.roughness = roughness as f32;
        }
        if let Some(ambient_occlusion) =
            material.get("ambient_occlusion").and_then(|v| v.as_float())
        {
            settings.ambient_occlusion = ambient_occlusion as f32;
        }
        if let Some(emissive) = material.get("emissive").and_then(|v| v.as_str()) {
            settings.emissive = emissive.to_string();
        }
        if let Some(enhanced_ao) = material.get("enhanced_ao").and_then(|v| v.as_bool()) {
            settings.enhanced_ao = enhanced_ao;
        }
        if let Some(ao_strength) = material.get("ao_strength").and_then(|v| v.as_float()) {
            settings.ao_strength = ao_strength as f32;
        }
        if let Some(soft_shadows) = material.get("soft_shadows").and_then(|v| v.as_bool()) {
            settings.soft_shadows = soft_shadows;
        }
        if let Some(shadow_strength) = material.get("shadow_strength").and_then(|v| v.as_float()) {
            settings.shadow_strength = shadow_strength as f32;
        }
        Ok(())
    }

    fn parse_background_section(
        settings: &mut RenderSettings,
        background: &toml::Table,
    ) -> Result<(), String> {
        if let Some(use_background_image) = background
            .get("use_background_image")
            .and_then(|v| v.as_bool())
        {
            settings.use_background_image = use_background_image;
        }
        if let Some(enable_gradient_background) = background
            .get("enable_gradient_background")
            .and_then(|v| v.as_bool())
        {
            settings.enable_gradient_background = enable_gradient_background;
        }
        if let Some(gradient_top_color) = background
            .get("gradient_top_color")
            .and_then(|v| v.as_str())
        {
            settings.gradient_top_color = gradient_top_color.to_string();
        }
        if let Some(gradient_bottom_color) = background
            .get("gradient_bottom_color")
            .and_then(|v| v.as_str())
        {
            settings.gradient_bottom_color = gradient_bottom_color.to_string();
        }
        if let Some(enable_ground_plane) = background
            .get("enable_ground_plane")
            .and_then(|v| v.as_bool())
        {
            settings.enable_ground_plane = enable_ground_plane;
        }
        if let Some(ground_plane_color) = background
            .get("ground_plane_color")
            .and_then(|v| v.as_str())
        {
            settings.ground_plane_color = ground_plane_color.to_string();
        }
        if let Some(ground_plane_height) = background
            .get("ground_plane_height")
            .and_then(|v| v.as_float())
        {
            settings.ground_plane_height = ground_plane_height as f32;
        }
        Ok(())
    }

    fn parse_animation_section(
        settings: &mut RenderSettings,
        animation: &toml::Table,
    ) -> Result<(), String> {
        if let Some(animate) = animation.get("animate").and_then(|v| v.as_bool()) {
            settings.animate = animate;
        }
        if let Some(fps) = animation.get("fps").and_then(|v| v.as_integer()) {
            settings.fps = fps as usize;
        }
        if let Some(rotation_speed) = animation.get("rotation_speed").and_then(|v| v.as_float()) {
            settings.rotation_speed = rotation_speed as f32;
        }
        if let Some(rotation_cycles) = animation.get("rotation_cycles").and_then(|v| v.as_float()) {
            settings.rotation_cycles = rotation_cycles as f32;
        }
        if let Some(animation_type) = animation.get("animation_type").and_then(|v| v.as_str()) {
            settings.animation_type = match animation_type {
                "CameraOrbit" => AnimationType::CameraOrbit,
                "ObjectLocalRotation" => AnimationType::ObjectLocalRotation,
                "None" => AnimationType::None,
                _ => return Err(format!("未知的动画类型: {}", animation_type)),
            };
        }
        if let Some(rotation_axis) = animation.get("rotation_axis").and_then(|v| v.as_str()) {
            settings.rotation_axis = match rotation_axis {
                "X" => RotationAxis::X,
                "Y" => RotationAxis::Y,
                "Z" => RotationAxis::Z,
                "Custom" => RotationAxis::Custom,
                _ => return Err(format!("未知的旋转轴: {}", rotation_axis)),
            };
        }
        if let Some(custom_rotation_axis) = animation
            .get("custom_rotation_axis")
            .and_then(|v| v.as_str())
        {
            settings.custom_rotation_axis = custom_rotation_axis.to_string();
        }
        Ok(())
    }

    // ===== 🔥 **RenderSettings -> TOML 转换** =====

    fn settings_to_toml(settings: &RenderSettings) -> Result<String, String> {
        let mut content = String::new();

        // 文件头注释
        content.push_str("# 🔥 光栅化渲染器配置文件\n");
        content.push_str("# 支持多光源、完整材质系统和高级渲染特性\n\n");

        // [files] 部分
        content.push_str("[files]\n");
        if let Some(obj) = &settings.obj {
            content.push_str(&format!("obj = \"{}\"\n", obj));
        }
        content.push_str(&format!("output = \"{}\"\n", settings.output));
        content.push_str(&format!("output_dir = \"{}\"\n", settings.output_dir));
        if let Some(texture) = &settings.texture {
            content.push_str(&format!("texture = \"{}\"\n", texture));
        }
        if let Some(bg_image) = &settings.background_image_path {
            content.push_str(&format!("background_image = \"{}\"\n", bg_image));
        }
        content.push_str("\n");

        // [render] 部分
        content.push_str("[render]\n");
        content.push_str(&format!("width = {}\n", settings.width));
        content.push_str(&format!("height = {}\n", settings.height));
        content.push_str(&format!("projection = \"{}\"\n", settings.projection));
        content.push_str(&format!("use_zbuffer = {}\n", settings.use_zbuffer));
        content.push_str(&format!("colorize = {}\n", settings.colorize));
        content.push_str(&format!("use_texture = {}\n", settings.use_texture));
        content.push_str(&format!("use_gamma = {}\n", settings.use_gamma));
        content.push_str(&format!(
            "backface_culling = {}\n",
            settings.backface_culling
        ));
        content.push_str(&format!("wireframe = {}\n", settings.wireframe));
        content.push_str(&format!(
            "use_multithreading = {}\n",
            settings.use_multithreading
        ));
        content.push_str(&format!(
            "cull_small_triangles = {}\n",
            settings.cull_small_triangles
        ));
        content.push_str(&format!(
            "min_triangle_area = {}\n",
            settings.min_triangle_area
        ));
        content.push_str(&format!("save_depth = {}\n", settings.save_depth));
        content.push_str("\n");

        // [camera] 部分
        content.push_str("[camera]\n");
        content.push_str(&format!("from = \"{}\"\n", settings.camera_from));
        content.push_str(&format!("at = \"{}\"\n", settings.camera_at));
        content.push_str(&format!("up = \"{}\"\n", settings.camera_up));
        content.push_str(&format!("fov = {}\n", settings.camera_fov));
        content.push_str("\n");

        // [object] 部分
        content.push_str("[object]\n");
        content.push_str(&format!("position = \"{}\"\n", settings.object_position));
        content.push_str(&format!("rotation = \"{}\"\n", settings.object_rotation));
        content.push_str(&format!("scale_xyz = \"{}\"\n", settings.object_scale_xyz));
        content.push_str(&format!("scale = {}\n", settings.object_scale));
        content.push_str("\n");

        // [lighting] 部分
        content.push_str("[lighting]\n");
        content.push_str(&format!("use_lighting = {}\n", settings.use_lighting));
        content.push_str(&format!("ambient = {}\n", settings.ambient));
        content.push_str(&format!("ambient_color = \"{}\"\n", settings.ambient_color));
        content.push_str("\n");

        // 🔥 **[[light]] 数组 - 多光源**
        if !settings.lights.is_empty() {
            content.push_str("# 🔥 多光源配置 - 支持任意数量\n");
            for light in &settings.lights {
                content.push_str("[[light]]\n");
                match light {
                    Light::Directional {
                        enabled,
                        direction_str,
                        color_str,
                        intensity,
                        ..
                    } => {
                        content.push_str("type = \"directional\"\n");
                        content.push_str(&format!("enabled = {}\n", enabled));
                        content.push_str(&format!("direction = \"{}\"\n", direction_str));
                        content.push_str(&format!("color = \"{}\"\n", color_str));
                        content.push_str(&format!("intensity = {}\n", intensity));
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
                        content.push_str("type = \"point\"\n");
                        content.push_str(&format!("enabled = {}\n", enabled));
                        content.push_str(&format!("position = \"{}\"\n", position_str));
                        content.push_str(&format!("color = \"{}\"\n", color_str));
                        content.push_str(&format!("intensity = {}\n", intensity));
                        content.push_str(&format!(
                            "constant_attenuation = {}\n",
                            constant_attenuation
                        ));
                        content.push_str(&format!("linear_attenuation = {}\n", linear_attenuation));
                        content.push_str(&format!(
                            "quadratic_attenuation = {}\n",
                            quadratic_attenuation
                        ));
                    }
                }
                content.push_str("\n");
            }
        }

        // [material] 部分
        content.push_str("[material]\n");
        content.push_str(&format!("use_phong = {}\n", settings.use_phong));
        content.push_str(&format!("use_pbr = {}\n", settings.use_pbr));
        content.push_str(&format!("diffuse_color = \"{}\"\n", settings.diffuse_color));
        content.push_str(&format!("specular = {}\n", settings.specular));
        content.push_str(&format!("shininess = {}\n", settings.shininess));
        content.push_str(&format!("base_color = \"{}\"\n", settings.base_color));
        content.push_str(&format!("metallic = {}\n", settings.metallic));
        content.push_str(&format!("roughness = {}\n", settings.roughness));
        content.push_str(&format!(
            "ambient_occlusion = {}\n",
            settings.ambient_occlusion
        ));
        content.push_str(&format!("emissive = \"{}\"\n", settings.emissive));
        content.push_str(&format!("enhanced_ao = {}\n", settings.enhanced_ao));
        content.push_str(&format!("ao_strength = {}\n", settings.ao_strength));
        content.push_str(&format!("soft_shadows = {}\n", settings.soft_shadows));
        content.push_str(&format!("shadow_strength = {}\n", settings.shadow_strength));
        content.push_str("\n");

        // [background] 部分
        content.push_str("[background]\n");
        content.push_str(&format!(
            "use_background_image = {}\n",
            settings.use_background_image
        ));
        content.push_str(&format!(
            "enable_gradient_background = {}\n",
            settings.enable_gradient_background
        ));
        content.push_str(&format!(
            "gradient_top_color = \"{}\"\n",
            settings.gradient_top_color
        ));
        content.push_str(&format!(
            "gradient_bottom_color = \"{}\"\n",
            settings.gradient_bottom_color
        ));
        content.push_str(&format!(
            "enable_ground_plane = {}\n",
            settings.enable_ground_plane
        ));
        content.push_str(&format!(
            "ground_plane_color = \"{}\"\n",
            settings.ground_plane_color
        ));
        content.push_str(&format!(
            "ground_plane_height = {}\n",
            settings.ground_plane_height
        ));
        content.push_str("\n");

        // [animation] 部分
        content.push_str("[animation]\n");
        content.push_str(&format!("animate = {}\n", settings.animate));
        content.push_str(&format!("fps = {}\n", settings.fps));
        content.push_str(&format!("rotation_speed = {}\n", settings.rotation_speed));
        content.push_str(&format!("rotation_cycles = {}\n", settings.rotation_cycles));
        content.push_str(&format!(
            "animation_type = \"{:?}\"\n",
            settings.animation_type
        ));
        content.push_str(&format!(
            "rotation_axis = \"{:?}\"\n",
            settings.rotation_axis
        ));
        content.push_str(&format!(
            "custom_rotation_axis = \"{}\"\n",
            settings.custom_rotation_axis
        ));
        content.push_str("\n");

        Ok(content)
    }

    // ===== 🔥 **示例配置生成** =====

    fn create_example_settings() -> RenderSettings {
        let mut settings = RenderSettings::default();

        // 示例文件路径
        settings.obj = Some("obj/simple/bunny.obj".to_string());
        settings.output = "rendered_scene".to_string();
        settings.output_dir = "output".to_string();
        settings.texture = Some("textures/wood.jpg".to_string());
        settings.background_image_path = Some("backgrounds/sky.jpg".to_string());

        // 高质量渲染设置
        settings.width = 1920;
        settings.height = 1080;
        settings.use_zbuffer = true;
        settings.use_texture = true;
        settings.use_gamma = true;
        settings.use_multithreading = true;
        settings.save_depth = true;

        // 示例相机位置
        settings.camera_from = "0,0,5".to_string();
        settings.camera_at = "0,0,0".to_string();
        settings.camera_up = "0,1,0".to_string();
        settings.camera_fov = 45.0;

        // 示例光源配置 - 多种光源展示
        settings.lights = vec![
            // 主要方向光
            Light::directional(
                Vector3::new(0.0, -1.0, -1.0),
                Vector3::new(1.0, 1.0, 1.0),
                0.8,
            ),
            // 温暖的点光源
            Light::point(
                Point3::new(2.0, 2.0, 2.0),
                Vector3::new(1.0, 0.8, 0.6),
                1.2,
                Some((1.0, 0.09, 0.032)),
            ),
            // 蓝色辅助方向光（默认禁用）
            {
                let mut light = Light::directional(
                    Vector3::new(-1.0, 0.0, 0.0),
                    Vector3::new(0.5, 0.5, 1.0),
                    0.4,
                );
                if let Light::Directional {
                    enabled: ref mut light_enabled,
                    ..
                } = light
                {
                    *light_enabled = false;
                }
                light
            },
        ];

        // 启用PBR材质
        settings.use_phong = false;
        settings.use_pbr = true;
        settings.metallic = 0.1;
        settings.roughness = 0.3;
        settings.enhanced_ao = true;
        settings.soft_shadows = true;

        // 渐变背景
        settings.enable_gradient_background = true;
        settings.gradient_top_color = "0.5,0.7,1.0".to_string();
        settings.gradient_bottom_color = "0.1,0.2,0.4".to_string();
        settings.enable_ground_plane = true;

        // 动画设置
        settings.fps = 30;
        settings.rotation_speed = 1.0;
        settings.rotation_cycles = 2.0;
        settings.animation_type = AnimationType::CameraOrbit;
        settings.rotation_axis = RotationAxis::Y;

        settings
    }
}
