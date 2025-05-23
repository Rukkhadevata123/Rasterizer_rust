use crate::io::render_settings::{parse_point3, parse_vec3};
use clap::ValueEnum;
use nalgebra::{Matrix4, Point3, Vector3};

/// 光照预设模式
#[derive(Debug, Clone, Default, PartialEq, Eq, ValueEnum)]
pub enum LightingPreset {
    /// 单一方向光源（默认）
    #[default]
    SingleDirectional,
    /// 三面方向光源（更均匀的照明）
    ThreeDirectional,
    /// 一个方向光源加四个点光源（更生动的照明）
    MixedComplete,
    /// 无光照
    None,
}

/// 光源类型
#[derive(Debug, Clone)]
pub enum Light {
    /// 定向光，direction表示朝向光源的方向
    Directional {
        direction: Vector3<f32>,
        color: Vector3<f32>,
        intensity: f32,
        // 阴影相关字段
        cast_shadow: bool,
        shadow_matrix: Option<Matrix4<f32>>,
    },
    /// 点光源，带位置和衰减因子
    Point {
        position: Point3<f32>,
        color: Vector3<f32>,
        intensity: f32,
        /// 衰减因子: (常数项, 一次项, 二次项)
        attenuation: (f32, f32, f32),
    },
}

impl Light {
    /// 创建定向光源
    pub fn directional(direction: Vector3<f32>, color: Vector3<f32>, intensity: f32) -> Self {
        Light::Directional {
            direction: direction.normalize(),
            color,
            intensity,
            cast_shadow: false, // 默认不投射阴影
            shadow_matrix: None,
        }
    }

    /// 创建点光源
    pub fn point(
        position: Point3<f32>,
        color: Vector3<f32>,
        intensity: f32,
        attenuation: Option<(f32, f32, f32)>,
    ) -> Self {
        Light::Point {
            position,
            color,
            intensity,
            attenuation: attenuation.unwrap_or((1.0, 0.09, 0.032)),
        }
    }

    /// 从字符串创建方向光源
    pub fn directional_from_str(
        direction: &str,
        color: &str,
        intensity: f32,
    ) -> Result<Self, String> {
        let dir = parse_vec3(direction)?.normalize();
        let col = parse_vec3(color)?;
        Ok(Self::directional(dir, col, intensity))
    }

    /// 从字符串创建点光源
    pub fn point_from_str(
        position: &str,
        color: &str,
        intensity: f32,
        constant: f32,
        linear: f32,
        quadratic: f32,
    ) -> Result<Self, String> {
        let pos = parse_point3(position)?;
        let col = parse_vec3(color)?;
        Ok(Self::point(
            pos,
            col,
            intensity,
            Some((constant, linear, quadratic)),
        ))
    }

    /// 获取光源的方向（从表面点到光源）
    pub fn get_direction(&self, point: &Point3<f32>) -> Vector3<f32> {
        match self {
            Light::Directional { direction, .. } => -direction,
            Light::Point { position, .. } => (position - point).normalize(),
        }
    }

    /// 计算光源在给定点的强度（考虑衰减和颜色）
    pub fn get_intensity(&self, point: &Point3<f32>) -> Vector3<f32> {
        match self {
            Light::Directional {
                color, intensity, ..
            } => Vector3::new(
                color.x * intensity,
                color.y * intensity,
                color.z * intensity,
            ),
            Light::Point {
                position,
                color,
                intensity,
                attenuation,
            } => {
                let distance = (position - point).magnitude();
                let (constant, linear, quadratic) = *attenuation;
                let attenuation_factor =
                    1.0 / (constant + linear * distance + quadratic * distance * distance);

                Vector3::new(
                    color.x * intensity * attenuation_factor,
                    color.y * intensity * attenuation_factor,
                    color.z * intensity * attenuation_factor,
                )
            }
        }
    }

    /// 设置是否投射阴影
    pub fn with_shadow(mut self) -> Self {
        match &mut self {
            Light::Directional { cast_shadow, .. } => *cast_shadow = true,
            _ => {} // 点光源暂不支持阴影
        }
        self
    }

    /// 检查此光源是否投射阴影
    pub fn casts_shadow(&self) -> bool {
        match self {
            Light::Directional { cast_shadow, .. } => *cast_shadow,
            _ => false, // 点光源暂不支持阴影
        }
    }

    /// 启用或禁用阴影投射
    pub fn set_cast_shadow(&mut self, value: bool) {
        if let Light::Directional { cast_shadow, .. } = self {
            *cast_shadow = value;
        }
    }

    /// 计算光源视图矩阵和阴影投影矩阵
    pub fn calculate_shadow_matrix(
        &mut self,
        scene_center: Point3<f32>,
        scene_radius: f32,
    ) -> Option<Matrix4<f32>> {
        match self {
            Light::Directional {
                direction,
                cast_shadow,
                shadow_matrix,
                ..
            } => {
                if !*cast_shadow {
                    return None;
                }

                // 将光源定位在场景中心上方足够远处
                // 通过场景半径确定光源与场景的距离
                let light_position = scene_center - *direction * (scene_radius * 2.0);

                // 创建从光源视角的视图矩阵
                let light_target = scene_center;
                let up = if direction.x.abs() < 0.9 && direction.z.abs() < 0.9 {
                    Vector3::new(1.0, 0.0, 0.0) // 如果光源方向不太靠近x或z轴，使用x轴作为上方向
                } else {
                    Vector3::new(0.0, 1.0, 0.0) // 否则使用y轴
                };

                // 构建从光源出发的视图矩阵
                let light_view = Matrix4::look_at_rh(&light_position, &light_target, &up);

                // 创建正交投影矩阵，覆盖整个场景
                // 使用场景半径确定正交视锥体的大小
                let ortho_size = scene_radius * 1.5; // 适当增加以覆盖整个场景
                let near = 0.1;
                let far = scene_radius * 4.0;

                let light_projection = Matrix4::new_orthographic(
                    -ortho_size,
                    ortho_size,
                    -ortho_size,
                    ortho_size,
                    near,
                    far,
                );

                // 组合视图和投影矩阵
                let light_vp_matrix = light_projection * light_view;

                // 存储并返回矩阵
                *shadow_matrix = Some(light_vp_matrix.clone());
                Some(light_vp_matrix)
            }
            _ => None, // 点光源暂不支持阴影
        }
    }

    /// 获取阴影矩阵
    pub fn get_shadow_matrix(&self) -> Option<&Matrix4<f32>> {
        match self {
            Light::Directional { shadow_matrix, .. } => shadow_matrix.as_ref(),
            _ => None,
        }
    }
}

/// 单个方向光源的配置（仅用于UI和命令行交互）
#[derive(Debug, Clone)]
pub struct DirectionalLightConfig {
    pub enabled: bool,
    pub direction: String,
    pub color: String,
    pub intensity: f32,
    pub cast_shadow: bool, // 控制是否投射阴影
}

impl Default for DirectionalLightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            direction: "0,-1,-1".to_string(),
            color: "1.0,1.0,1.0".to_string(),
            intensity: 0.8,
            cast_shadow: false, // 默认不投射阴影
        }
    }
}

impl DirectionalLightConfig {
    /// 转换为Light实例
    pub fn to_light(&self) -> Result<Light, String> {
        if !self.enabled {
            return Err("光源未启用".to_string());
        }

        let mut light = Light::directional_from_str(&self.direction, &self.color, self.intensity)?;

        // 如果配置了投射阴影，设置light.cast_shadow为true
        if self.cast_shadow {
            light = light.with_shadow();
        }

        Ok(light)
    }
}

/// 单个点光源的配置（仅用于UI和命令行交互）
#[derive(Debug, Clone)]
pub struct PointLightConfig {
    pub enabled: bool,
    pub position: String,
    pub color: String,
    pub intensity: f32,
    pub constant_attenuation: f32,
    pub linear_attenuation: f32,
    pub quadratic_attenuation: f32,
}

impl Default for PointLightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: "0,5,5".to_string(),
            color: "1.0,1.0,1.0".to_string(),
            intensity: 1.0,
            constant_attenuation: 1.0,
            linear_attenuation: 0.09,
            quadratic_attenuation: 0.032,
        }
    }
}

impl PointLightConfig {
    /// 转换为Light实例
    pub fn to_light(&self) -> Result<Light, String> {
        if !self.enabled {
            return Err("光源未启用".to_string());
        }
        Light::point_from_str(
            &self.position,
            &self.color,
            self.intensity,
            self.constant_attenuation,
            self.linear_attenuation,
            self.quadratic_attenuation,
        )
    }
}

/// 从预设生成光源配置（用于UI和设置）
pub fn create_light_configs_from_preset(
    preset: LightingPreset,
    main_intensity: f32,
    use_shadows: bool,
) -> (Vec<DirectionalLightConfig>, Vec<PointLightConfig>) {
    let mut directional_lights = Vec::new();
    let mut point_lights = Vec::new();

    match preset {
        LightingPreset::SingleDirectional => {
            // 添加一个默认的方向光源
            directional_lights.push(DirectionalLightConfig {
                enabled: true,
                direction: "0,-1,-1".to_string(),
                color: "1.0,1.0,1.0".to_string(),
                intensity: main_intensity,
                cast_shadow: use_shadows, // 阴影设置
            });
        }
        LightingPreset::ThreeDirectional => {
            // 添加三个方向光源，从不同角度照亮场景
            directional_lights.push(DirectionalLightConfig {
                enabled: true,
                direction: "0,-1,-1".to_string(),
                color: "1.0,1.0,1.0".to_string(),
                intensity: main_intensity * 0.7,
                cast_shadow: use_shadows, // 主光源使用阴影映射设置
            });
            directional_lights.push(DirectionalLightConfig {
                enabled: true,
                direction: "-1,-0.5,0.2".to_string(),
                color: "0.9,0.9,1.0".to_string(),
                intensity: main_intensity * 0.5,
                cast_shadow: false, // 辅助光源不使用阴影
            });
            directional_lights.push(DirectionalLightConfig {
                enabled: true,
                direction: "1,-0.5,0.2".to_string(),
                color: "1.0,0.9,0.8".to_string(),
                intensity: main_intensity * 0.3,
                cast_shadow: false, // 辅助光源不使用阴影
            });
        }
        LightingPreset::MixedComplete => {
            // 添加一个主方向光源
            directional_lights.push(DirectionalLightConfig {
                enabled: true,
                direction: "0,-1,-1".to_string(),
                color: "1.0,1.0,1.0".to_string(),
                intensity: main_intensity * 0.6,
                cast_shadow: use_shadows, // 主光源使用阴影映射设置
            });

            // 添加四个点光源
            let point_configs = [
                ("2,3,2", "1.0,0.8,0.6"),   // 暖色调
                ("-2,3,2", "0.6,0.8,1.0"),  // 冷色调
                ("2,3,-2", "0.8,1.0,0.8"),  // 绿色调
                ("-2,3,-2", "1.0,0.8,1.0"), // 紫色调
            ];

            for (pos, color) in &point_configs {
                point_lights.push(PointLightConfig {
                    enabled: true,
                    position: pos.to_string(),
                    color: color.to_string(),
                    intensity: main_intensity * 0.5,
                    constant_attenuation: 1.0,
                    linear_attenuation: 0.09,
                    quadratic_attenuation: 0.032,
                });
            }
        }
        LightingPreset::None => {
            // 不添加任何光源
        }
    }

    (directional_lights, point_lights)
}

/// 重构根据预设创建光源的函数，使用配置函数
pub fn create_lights_from_preset(
    preset: LightingPreset,
    main_intensity: f32,
    use_shadows: bool,
) -> Vec<Light> {
    // 先获取配置
    let (directional_configs, point_configs) =
        create_light_configs_from_preset(preset, main_intensity, use_shadows);

    // 然后从配置创建实际的光源
    create_lights_from_configs(&directional_configs, &point_configs, use_shadows)
}

/// 从配置列表创建光源集合
pub fn create_lights_from_configs(
    directional_lights: &[DirectionalLightConfig],
    point_lights: &[PointLightConfig],
    use_shadows: bool, // 添加阴影控制参数
) -> Vec<Light> {
    let mut lights = Vec::new();

    // 添加方向光源
    for (i, light) in directional_lights.iter().enumerate() {
        if light.enabled {
            match light.to_light() {
                Ok(mut l) => {
                    // 如果全局启用阴影且配置了cast_shadow或是第一个光源，启用阴影
                    if use_shadows && (light.cast_shadow || (i == 0 && lights.is_empty())) {
                        l.set_cast_shadow(true);
                    }
                    lights.push(l);
                }
                Err(e) => eprintln!("方向光 #{} 配置错误: {}", i + 1, e),
            }
        }
    }

    // 添加点光源
    for (i, light) in point_lights.iter().enumerate() {
        if light.enabled {
            match light.to_light() {
                Ok(l) => lights.push(l),
                Err(e) => eprintln!("点光源 #{} 配置错误: {}", i + 1, e),
            }
        }
    }

    lights
}
