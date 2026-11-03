use forge_math::{Vec2, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::handles::HalfEdgeHandle;

/// A topological vertex in a `HalfEdgeMesh`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfEdgeVertex {
    /// 3D position of the vertex in object space.
    pub position: Vec3,
    /// One outgoing half-edge originating from this vertex.
    /// If this vertex is on a mesh boundary, this should ideally point to an outgoing boundary half-edge.
    pub half_edge: Option<HalfEdgeHandle>,
    /// Optional smooth vertex normal.
    pub normal: Option<Vec3>,
    /// Optional primary vertex UV coordinate.
    pub uv: Option<Vec2>,
    /// Optional RGBA vertex color.
    pub color: Option<[f32; 4]>,
    /// Generation counter for handle validation.
    pub(crate) generation: u32,
    /// Tombstone flag for deleted elements in slot-reuse.
    pub(crate) deleted: bool,
}

/// Type alias for the topological half-edge vertex.
pub type Vertex = HalfEdgeVertex;

impl HalfEdgeVertex {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            half_edge: None,
            normal: None,
            uv: None,
            color: None,
            generation: 0,
            deleted: false,
        }
    }

    #[inline]
    pub fn with_half_edge(mut self, half_edge: HalfEdgeHandle) -> Self {
        self.half_edge = Some(half_edge);
        self
    }

    #[inline]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    #[inline]
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    #[inline]
    pub fn half_edge(&self) -> Option<HalfEdgeHandle> {
        self.half_edge
    }

    #[inline]
    pub fn set_half_edge(&mut self, half_edge: Option<HalfEdgeHandle>) {
        self.half_edge = half_edge;
    }

    #[inline]
    pub fn has_half_edge(&self) -> bool {
        self.half_edge.is_some()
    }

    #[inline]
    pub fn normal(&self) -> Option<Vec3> {
        self.normal
    }

    #[inline]
    pub fn set_normal(&mut self, normal: Option<Vec3>) {
        self.normal = normal;
    }

    #[inline]
    pub fn uv(&self) -> Option<Vec2> {
        self.uv
    }

    #[inline]
    pub fn set_uv(&mut self, uv: Option<Vec2>) {
        self.uv = uv;
    }

    #[inline]
    pub fn color(&self) -> Option<[f32; 4]> {
        self.color
    }

    #[inline]
    pub fn set_color(&mut self, color: Option<[f32; 4]>) {
        self.color = color;
    }

    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    #[inline]
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}
