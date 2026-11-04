//! # Half-Edge Topology System
//!
//! A high-performance, 2-manifold B-Rep mesh representation implementing
//! full incidence and adjacency for vertices, directed half-edges, undirected edges,
//! and polygon faces ($N \ge 3$).
//!
//! ## Key Invariants & Design
//! - **Boundary representation**: Every edge has exactly two twin half-edges. Boundary half-edges
//!   have `face: None` and are linked into closed cycles, forming explicit boundary loops without
//!   fictitious faces.
//! - **Generational Handles**: Compact `VertexHandle`, `HalfEdgeHandle`, `EdgeHandle`, and `FaceHandle`
//!   incorporate generation counters for safe element reuse and detection of dangling references.
//! - **Corner Attributes**: Wedge/corner UVs and normals are tracked per-half-edge, preserving UV seams
//!   and hard normal edges across topological mutations.

pub mod builder;
pub mod conversion;
pub mod edge;
pub mod error;
pub mod face;
pub mod half_edge;
pub mod handles;
pub mod mesh;
pub mod operations;
pub mod selection;
pub mod traversal;
pub mod validation;
pub mod vertex;

pub use builder::TopologyBuilder;
pub use edge::{Edge, HalfEdgeEdge};
pub use error::TopologyError;
pub use face::{Face, HalfEdgeFace};
pub use half_edge::HalfEdge;
pub use handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};
pub use mesh::HalfEdgeMesh;
pub use operations::{
    CollapseEdgeResult, ExtrudeFaceResult, FlipEdgeResult, InsetFaceResult, MergeFacesResult,
    PokeFaceResult, SplitEdgeResult, SplitFaceResult,
};
pub use selection::{EdgeSelectionPath, TerminationReason};
pub use validation::{
    IssueCategory, IssueSeverity, TopologyReport, TopologyValidator, ValidationIssue,
    ValidationReport,
};
pub use vertex::{HalfEdgeVertex, Vertex};

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Vec3;
    use std::collections::HashSet;

    #[test]
    fn test_handle_creation() {
        let vh = VertexHandle::new(10, 2);
        assert_eq!(vh.index(), 10);
        assert_eq!(vh.raw_index(), 10);
        assert_eq!(vh.generation(), 2);
        assert!(vh.is_valid());
        assert!(!vh.is_null());

        let heh = HalfEdgeHandle::new(5, 1);
        assert_eq!(heh.index(), 5);
        assert_eq!(heh.raw_index(), 5);
        assert_eq!(heh.generation(), 1);
        assert!(heh.is_valid());
        assert!(!heh.is_null());

        let eh = EdgeHandle::new(3, 0);
        assert_eq!(eh.index(), 3);
        assert_eq!(eh.raw_index(), 3);
        assert_eq!(eh.generation(), 0);
        assert!(eh.is_valid());
        assert!(!eh.is_null());

        let fh = FaceHandle::new(7, 4);
        assert_eq!(fh.index(), 7);
        assert_eq!(fh.raw_index(), 7);
        assert_eq!(fh.generation(), 4);
        assert!(fh.is_valid());
        assert!(!fh.is_null());

        assert!(VertexHandle::NULL.is_null());
        assert!(!VertexHandle::NULL.is_valid());
        assert!(HalfEdgeHandle::NULL.is_null());
        assert!(!HalfEdgeHandle::NULL.is_valid());
        assert!(EdgeHandle::NULL.is_null());
        assert!(!EdgeHandle::NULL.is_valid());
        assert!(FaceHandle::NULL.is_null());
        assert!(!FaceHandle::NULL.is_valid());

        assert_eq!(VertexHandle::default(), VertexHandle::NULL);
        assert_eq!(HalfEdgeHandle::default(), HalfEdgeHandle::NULL);
        assert_eq!(EdgeHandle::default(), EdgeHandle::NULL);
        assert_eq!(FaceHandle::default(), FaceHandle::NULL);
    }

    #[test]
    fn test_handle_comparison() {
        let v1 = VertexHandle::new(1, 0);
        let v2 = VertexHandle::new(1, 0);
        let v3 = VertexHandle::new(2, 0);
        let v4 = VertexHandle::new(1, 1);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
        assert_ne!(v1, v4);
        assert!(v1 < v3);
    }

    #[test]
    fn test_handle_hash() {
        let mut set = HashSet::new();
        let v1 = VertexHandle::new(1, 0);
        let v2 = VertexHandle::new(1, 0);
        let v3 = VertexHandle::new(2, 0);

        set.insert(v1);
        assert!(set.contains(&v2));
        assert!(!set.contains(&v3));
        set.insert(v3);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_handle_type_safety_no_mixing() {
        fn accept_vertex(_h: VertexHandle) -> bool {
            true
        }
        fn accept_half_edge(_h: HalfEdgeHandle) -> bool {
            true
        }
        fn accept_edge(_h: EdgeHandle) -> bool {
            true
        }
        fn accept_face(_h: FaceHandle) -> bool {
            true
        }

        let vh = VertexHandle::new(0, 0);
        let heh = HalfEdgeHandle::new(0, 0);
        let eh = EdgeHandle::new(0, 0);
        let fh = FaceHandle::new(0, 0);

        assert!(accept_vertex(vh));
        assert!(accept_half_edge(heh));
        assert!(accept_edge(eh));
        assert!(accept_face(fh));
        // Type system prevents assigning or passing vh into accept_half_edge, etc.
    }

    #[test]
    fn test_basic_structures_creation() {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let mut vert = Vertex::new(pos);
        assert_eq!(vert.position(), pos);
        assert!(!vert.has_half_edge());

        let he_h = HalfEdgeHandle::new(0, 0);
        vert.set_half_edge(Some(he_h));
        assert_eq!(vert.half_edge(), Some(he_h));
        assert!(vert.has_half_edge());

        let v_h = VertexHandle::new(0, 0);
        let e_h = EdgeHandle::new(0, 0);
        let f_h = FaceHandle::new(0, 0);

        let he = HalfEdge::new(v_h, e_h, Some(f_h));
        assert_eq!(he.origin(), v_h);
        assert_eq!(he.vertex(), v_h);
        assert_eq!(he.edge(), e_h);
        assert_eq!(he.face(), Some(f_h));
        assert!(!he.is_boundary());

        let edge = Edge::new(he_h);
        assert_eq!(edge.half_edge(), he_h);
        assert_eq!(edge.crease_weight(), 0.0);

        let face = Face::new(he_h);
        assert_eq!(face.half_edge(), he_h);
        assert!(face.normal().is_none());
        assert!(face.material_id().is_none());
    }

    #[test]
    fn test_initial_option_connections() {
        let v = Vertex::new(Vec3::ZERO);
        assert_eq!(v.half_edge(), None);
        assert!(!v.has_half_edge());
        assert_eq!(v.normal(), None);
        assert_eq!(v.uv(), None);
        assert_eq!(v.color(), None);

        let v_h = VertexHandle::new(1, 0);
        let e_h = EdgeHandle::new(2, 0);
        let boundary_he = HalfEdge::new(v_h, e_h, None);
        assert_eq!(boundary_he.twin(), None);
        assert_eq!(boundary_he.next(), None);
        assert_eq!(boundary_he.prev(), None);
        assert_eq!(boundary_he.face(), None);
        assert!(!boundary_he.has_twin());
        assert!(!boundary_he.has_next());
        assert!(!boundary_he.has_prev());
        assert!(!boundary_he.has_face());
        assert!(boundary_he.is_boundary());

        let edge = Edge::new(HalfEdgeHandle::new(0, 0));
        assert_eq!(edge.crease_weight(), 0.0);

        let face = Face::new(HalfEdgeHandle::new(0, 0));
        assert_eq!(face.normal(), None);
        assert_eq!(face.material_id(), None);
        assert!(!face.has_normal());
        assert!(!face.has_material_id());
    }
}
