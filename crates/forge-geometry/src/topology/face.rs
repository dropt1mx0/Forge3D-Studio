use forge_math::Vec3;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::handles::HalfEdgeHandle;

/// A polygon face in a `HalfEdgeMesh` bounded by a directed cycle of half-edges.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfEdgeFace {
    /// One representative half-edge belonging to the boundary cycle of this face.
    pub half_edge: HalfEdgeHandle,
    /// Cached face normal vector.
    pub normal: Option<Vec3>,
    /// Optional material index for multi-material meshes.
    pub material_id: Option<u32>,
    /// Generation counter for handle validation.
    pub(crate) generation: u32,
    /// Tombstone flag.
    pub(crate) deleted: bool,
}

/// Type alias for the topological half-edge face.
pub type Face = HalfEdgeFace;

impl HalfEdgeFace {
    pub fn new(half_edge: HalfEdgeHandle) -> Self {
        Self {
            half_edge,
            normal: None,
            material_id: None,
            generation: 0,
            deleted: false,
        }
    }

    #[inline]
    pub fn half_edge(&self) -> HalfEdgeHandle {
        self.half_edge
    }

    #[inline]
    pub fn set_half_edge(&mut self, half_edge: HalfEdgeHandle) {
        self.half_edge = half_edge;
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
    pub fn has_normal(&self) -> bool {
        self.normal.is_some()
    }

    #[inline]
    pub fn material_id(&self) -> Option<u32> {
        self.material_id
    }

    #[inline]
    pub fn set_material_id(&mut self, material_id: Option<u32>) {
        self.material_id = material_id;
    }

    #[inline]
    pub fn has_material_id(&self) -> bool {
        self.material_id.is_some()
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
