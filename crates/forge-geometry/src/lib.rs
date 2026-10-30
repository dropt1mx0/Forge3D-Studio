//! # Forge3D Geometry
//!
//! Pure-CPU mesh topology, validation, and procedural primitives (Cube, UV Sphere, Plane, Grid).
//!
//! Strictly contains NO GPU handles or UI dependencies.

pub mod mesh;
pub mod primitives;
pub mod validation;
pub mod vertex;

pub use mesh::Mesh;
pub use primitives::{generate_cube, generate_grid_lines, generate_plane, generate_uv_sphere};
pub use validation::validate_mesh_data;
pub use vertex::{LineVertex, Vertex};
