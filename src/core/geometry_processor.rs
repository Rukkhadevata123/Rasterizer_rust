use crate::geometry::camera::Camera;
use crate::geometry::transform::vertex_pipeline_parallel;
use crate::scene::scene_object::SceneObject;
use nalgebra::{Point2, Point3, Vector3};

/// 收集的几何数据类型
type GeometryCollection = (Vec<Point3<f32>>, Vec<Vector3<f32>>, Vec<usize>);

/// 几何变换结果
pub struct GeometryResult {
    pub screen_coords: Vec<Point2<f32>>,
    pub view_coords: Vec<Point3<f32>>,
    pub view_normals: Vec<Vector3<f32>>,
    pub mesh_offsets: Vec<usize>,
}

/// 几何处理器
pub struct GeometryProcessor;

impl GeometryProcessor {
    /// 执行几何变换 - 简化接口
    pub fn transform_geometry(
        scene_object: &SceneObject,
        camera: &mut Camera,
        frame_width: usize,
        frame_height: usize,
    ) -> GeometryResult {
        camera.update_matrices();

        let (vertices, normals, mesh_offsets) = Self::collect_geometry(&scene_object.model_data);

        let (screen_coords, view_coords, view_normals) = vertex_pipeline_parallel(
            &vertices,
            &normals,
            &scene_object.transform,
            &camera.view_matrix(),
            &camera.projection_matrix(),
            frame_width,
            frame_height,
        );

        GeometryResult {
            screen_coords,
            view_coords,
            view_normals,

            mesh_offsets,
        }
    }

    /// 收集几何数据 - 提取为独立方法
    fn collect_geometry(
        model_data: &crate::material_system::materials::ModelData,
    ) -> GeometryCollection {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut mesh_offsets = vec![0];

        for mesh in &model_data.meshes {
            vertices.extend(mesh.vertices.iter().map(|v| v.position));
            normals.extend(mesh.vertices.iter().map(|v| v.normal));
            mesh_offsets.push(vertices.len());
        }

        (vertices, normals, mesh_offsets)
    }
}
