//! # Forge3D Geometry
//!
//! Pure-CPU mesh topology, validation, editable mesh (B-Rep), and procedural primitives.
//!
//! Strictly contains NO GPU handles or UI dependencies.

pub mod edit_mesh;
pub mod mesh;
pub mod primitives;
pub mod topology;
pub mod validation;
pub mod vertex;

pub use edit_mesh::{EditEdge, EditFace, EditSelectionMode, EditVertex, EditableMesh};
pub use mesh::Mesh;
pub use primitives::{
    generate_circle, generate_cone, generate_cube, generate_cylinder, generate_grid_lines,
    generate_icosphere, generate_plane, generate_torus, generate_uv_sphere,
};
pub use topology::{
    BevelEdgeResult, CollapseEdgeResult, EdgeHandle, EdgeSelectionPath, ExtrudeFaceResult,
    FaceHandle, FlipEdgeResult, HalfEdge, HalfEdgeEdge, HalfEdgeFace, HalfEdgeHandle, HalfEdgeMesh,
    HalfEdgeVertex, InsetFaceResult, IssueCategory, IssueSeverity, MergeFacesResult,
    PokeFaceResult, SplitEdgeResult, SplitFaceResult, TerminationReason, TopologyBuilder,
    TopologyError, TopologyReport, TopologyValidator, ValidationIssue, ValidationReport,
    VertexHandle,
};
pub use validation::validate_mesh_data;
pub use vertex::{LineVertex, Vertex};
