use bytemuck::{Pod, Zeroable};
use forge_math::{Mat4, Vec3};

/// Camera uniform buffer data matching WGSL bind group 0 layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4], // xyz = position, w = 1.0 (16-byte alignment)
}

impl CameraUniform {
    pub fn new(view_proj: Mat4, camera_pos: Vec3) -> Self {
        Self {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 1.0],
        }
    }
}

/// Object uniform buffer data matching WGSL bind group 1 layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ObjectUniform {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub is_selected: u32,
    pub _padding: [u32; 3], // Padding to maintain 16-byte struct size
}

impl ObjectUniform {
    pub fn new(model: Mat4, color: [f32; 4], is_selected: bool) -> Self {
        Self {
            model: model.to_cols_array_2d(),
            color,
            is_selected: if is_selected { 1 } else { 0 },
            _padding: [0, 0, 0],
        }
    }
}
