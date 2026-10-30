use bytemuck::{Pod, Zeroable};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Standard vertex structure for 3D meshes.
///
/// Guaranteed to match WGSL shader vertex buffer layout with 16-byte alignment considerations:
/// - `position`: [f32; 3] (12 bytes)
/// - `normal`: [f32; 3]   (12 bytes)
/// - `uv`: [f32; 2]       (8 bytes)
/// - `color`: [f32; 4]    (16 bytes)
///
/// Total size: 48 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    /// Constructs a full vertex.
    pub const fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            position,
            normal,
            uv,
            color,
        }
    }

    /// Constructs a vertex with default white color.
    pub const fn with_pos_norm_uv(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Simple vertex for line drawing (grid and coordinate axes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl LineVertex {
    pub const fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self { position, color }
    }
}
