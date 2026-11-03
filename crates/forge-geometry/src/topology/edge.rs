#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::handles::HalfEdgeHandle;

/// An undirected edge in a `HalfEdgeMesh`, linking two twin half-edges.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfEdgeEdge {
    /// One of the two directed half-edges constituting this undirected edge.
    pub half_edge: HalfEdgeHandle,
    /// Edge sharpness / crease weight (0.0 = smooth, 1.0 = sharp crease).
    pub crease_weight: f32,
    /// Generation counter for handle validation.
    pub(crate) generation: u32,
    /// Tombstone flag.
    pub(crate) deleted: bool,
}

/// Type alias for the topological half-edge edge.
pub type Edge = HalfEdgeEdge;

impl HalfEdgeEdge {
    pub fn new(half_edge: HalfEdgeHandle) -> Self {
        Self {
            half_edge,
            crease_weight: 0.0,
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
    pub fn crease_weight(&self) -> f32 {
        self.crease_weight
    }

    #[inline]
    pub fn set_crease_weight(&mut self, crease_weight: f32) {
        self.crease_weight = crease_weight;
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
