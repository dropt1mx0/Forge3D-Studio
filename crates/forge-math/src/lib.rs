//! # Forge3D Math
//!
//! 3D linear algebra and spatial math primitives, coordinate conventions,
//! transforms, bounding boxes, and ray intersections.
//!
//! Re-exports foundational types from `glam` alongside engine-specific spatial structures.

pub mod bounds;
pub mod conventions;
pub mod ray;
pub mod transform;

pub use bounds::Aabb;
pub use conventions::*;
pub use ray::Ray;
pub use transform::Transform;

// Re-export common glam types
pub use glam::{EulerRot, Mat2, Mat3, Mat4, Quat, Vec2, Vec3, Vec3A, Vec4};
