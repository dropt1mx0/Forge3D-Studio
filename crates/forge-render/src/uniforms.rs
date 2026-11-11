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
    pub normal_model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub is_selected: u32,
    pub matcap_mode: u32,
    pub _padding: [u32; 2], // Padding to maintain 16-byte struct size
}

impl ObjectUniform {
    pub fn new(model: Mat4, color: [f32; 4], is_selected: bool, matcap_mode: u32) -> Self {
        Self {
            model: model.to_cols_array_2d(),
            normal_model: model.inverse().transpose().to_cols_array_2d(),
            color,
            is_selected: if is_selected { 1 } else { 0 },
            matcap_mode,
            _padding: [0, 0],
        }
    }
}

/// Edit Mode configuration uniform buffer matching WGSL bind group 2 layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct EditModeUniform {
    pub viewport_size: [f32; 2], // Viewport width, height in physical pixels (8 bytes)
    pub vertex_point_size: f32,  // Base marker diameter in screen pixels (4 bytes)
    pub is_vertex_mode: u32,     // 1 if Vertex selection mode, 0 otherwise (4 bytes)
}

impl EditModeUniform {
    pub fn new(viewport_size: [f32; 2], vertex_point_size: f32, is_vertex_mode: bool) -> Self {
        Self {
            viewport_size,
            vertex_point_size,
            is_vertex_mode: if is_vertex_mode { 1 } else { 0 },
        }
    }
}

/// Picking configuration uniform buffer data matching WGSL picking bind group 1 layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct PickingUniform {
    pub id_code: u32,
    pub _pad0: u32,
    pub viewport_size: [f32; 2],
    pub point_size: f32,
    pub _pad1: [u32; 3],
}

impl PickingUniform {
    pub fn new(id_code: u32, viewport_size: [f32; 2], point_size: f32) -> Self {
        Self {
            id_code,
            _pad0: 0,
            viewport_size,
            point_size,
            _pad1: [0, 0, 0],
        }
    }
}
