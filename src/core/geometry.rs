use crate::geometry::camera::Camera;
use crate::geometry::transform::vertex_pipeline_parallel;
use crate::material_system::materials::ModelData;
use crate::scene::scene_object::SceneObject;
use nalgebra::{Point2, Point3, Vector3};

// ===== 几何结果结构 =====

pub struct GeometryResult {
    pub screen_coords: Vec<Point2<f32>>,
    pub view_coords: Vec<Point3<f32>>,
    pub view_normals: Vec<Vector3<f32>>,
    pub mesh_offsets: Vec<usize>,
}

// ===== 几何处理功能 =====

/// 执行几何变换
pub fn transform_geometry(
    scene_object: &SceneObject,
    camera: &mut Camera,
    frame_width: usize,
    frame_height: usize,
) -> GeometryResult {
    camera.update_matrices();

    let (vertices, normals, mesh_offsets) = collect_geometry(&scene_object.model_data);

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

/// 收集几何数据
fn collect_geometry(model_data: &ModelData) -> (Vec<Point3<f32>>, Vec<Vector3<f32>>, Vec<usize>) {
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
