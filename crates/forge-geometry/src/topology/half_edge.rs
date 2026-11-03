use forge_math::{Vec2, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};

/// A directed half-edge in a `HalfEdgeMesh`.
///
/// Each edge is composed of two opposite half-edges (`twin` relationship).
/// If `face` is `None`, this half-edge represents a virtual boundary half-edge lying along
/// an open boundary / hole cycle.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfEdge {
    /// Vertex from which this directed half-edge originates.
    pub origin: VertexHandle,
    /// The opposite, oppositely directed half-edge.
    pub twin: HalfEdgeHandle,
    /// Next half-edge in counter-clockwise order around the incident face or boundary loop.
    pub next: HalfEdgeHandle,
    /// Previous half-edge in the incident face or boundary loop.
    pub prev: HalfEdgeHandle,
    /// Incident polygon face (`Some`) or virtual boundary loop (`None`).
    pub face: Option<FaceHandle>,
    /// The undirected edge corresponding to this half-edge and its twin.
    pub edge: EdgeHandle,
    /// Optional wedge/corner UV coordinate specific to this face corner.
    pub uv: Option<Vec2>,
    /// Optional wedge/corner normal specific to this face corner (for hard edges).
    pub normal: Option<Vec3>,
    /// Generation counter for handle validation.
    pub(crate) generation: u32,
    /// Tombstone flag.
    pub(crate) deleted: bool,
}

impl HalfEdge {
    pub fn new(origin: VertexHandle, edge: EdgeHandle, face: Option<FaceHandle>) -> Self {
        Self {
            origin,
            twin: HalfEdgeHandle::new(u32::MAX, 0),
            next: HalfEdgeHandle::new(u32::MAX, 0),
            prev: HalfEdgeHandle::new(u32::MAX, 0),
            face,
            edge,
            uv: None,
            normal: None,
            generation: 0,
            deleted: false,
        }
    }

    #[inline]
    pub fn origin(&self) -> VertexHandle {
        self.origin
    }

    #[inline]
    pub fn vertex(&self) -> VertexHandle {
        self.origin
    }

    #[inline]
    pub fn set_origin(&mut self, origin: VertexHandle) {
        self.origin = origin;
    }

    #[inline]
    pub fn edge(&self) -> EdgeHandle {
        self.edge
    }

    #[inline]
    pub fn set_edge(&mut self, edge: EdgeHandle) {
        self.edge = edge;
    }

    #[inline]
    pub fn face(&self) -> Option<FaceHandle> {
        self.face
    }

    #[inline]
    pub fn set_face(&mut self, face: Option<FaceHandle>) {
        self.face = face;
    }

    #[inline]
    pub fn has_face(&self) -> bool {
        self.face.is_some()
    }

    #[inline]
    pub fn twin(&self) -> Option<HalfEdgeHandle> {
        if self.twin.is_null() {
            None
        } else {
            Some(self.twin)
        }
    }

    #[inline]
    pub fn set_twin(&mut self, twin: Option<HalfEdgeHandle>) {
        self.twin = twin.unwrap_or(HalfEdgeHandle::NULL);
    }

    #[inline]
    pub fn has_twin(&self) -> bool {
        !self.twin.is_null()
    }

    #[inline]
    pub fn next(&self) -> Option<HalfEdgeHandle> {
        if self.next.is_null() {
            None
        } else {
            Some(self.next)
        }
    }

    #[inline]
    pub fn set_next(&mut self, next: Option<HalfEdgeHandle>) {
        self.next = next.unwrap_or(HalfEdgeHandle::NULL);
    }

    #[inline]
    pub fn has_next(&self) -> bool {
        !self.next.is_null()
    }

    #[inline]
    pub fn prev(&self) -> Option<HalfEdgeHandle> {
        if self.prev.is_null() {
            None
        } else {
            Some(self.prev)
        }
    }

    #[inline]
    pub fn set_prev(&mut self, prev: Option<HalfEdgeHandle>) {
        self.prev = prev.unwrap_or(HalfEdgeHandle::NULL);
    }

    #[inline]
    pub fn has_prev(&self) -> bool {
        !self.prev.is_null()
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
    pub fn normal(&self) -> Option<Vec3> {
        self.normal
    }

    #[inline]
    pub fn set_normal(&mut self, normal: Option<Vec3>) {
        self.normal = normal;
    }

    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    #[inline]
    pub fn is_boundary(&self) -> bool {
        self.face.is_none()
    }

    #[inline]
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}
