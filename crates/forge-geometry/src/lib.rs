//! # Forge3D Geometry
//!
//! Pure-CPU mesh topology, validation, editable mesh (B-Rep), and procedural primitives.
//!
//! Strictly contains NO GPU handles or UI dependencies.

pub mod edit_mesh;
pub mod mesh;
pub mod primitives;
pub mod sculpt;
pub mod topology;
pub mod uv;
pub mod validation;
pub mod vertex;

pub use edit_mesh::{
    EditEdge, EditFace, EditSelectionMode, EditVertex, EditableMesh, ProportionalFalloff,
};
pub use mesh::Mesh;
pub use primitives::{
    generate_circle, generate_cone, generate_cube, generate_cylinder, generate_grid_lines,
    generate_icosphere, generate_plane, generate_torus, generate_uv_sphere,
};
pub use sculpt::{SculptBrush, SculptSettings, apply_sculpt_stroke, sculpt_falloff};
pub use topology::{
    BevelEdgeResult, CollapseEdgeResult, EdgeHandle, EdgeSelectionPath, ExtrudeFaceResult,
    FaceHandle, FlipEdgeResult, HalfEdge, HalfEdgeEdge, HalfEdgeFace, HalfEdgeHandle, HalfEdgeMesh,
    HalfEdgeVertex, InsetFaceResult, IssueCategory, IssueSeverity, MergeFacesResult,
    PokeFaceResult, SplitEdgeResult, SplitFaceResult, TerminationReason, TopologyBuilder,
    TopologyError, TopologyReport, TopologyValidator, ValidationIssue, ValidationReport,
    VertexHandle,
};
pub use uv::{
    SmartProjectOptions, box_project_uvs, cylinder_project_uvs, pack_uv_islands, smart_project_uvs,
    sphere_project_uvs,
};
pub use validation::validate_mesh_data;
pub use vertex::{LineVertex, Vertex};
