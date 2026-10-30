//! # Forge3D Scene
//!
//! Scene graph, object hierarchy, selection state, editor preferences,
//! and versioned project serialization (`.forge`).
//!
//! Strictly contains NO GPU handles or UI dependencies.

pub mod editor_state;
pub mod object;
pub mod scene;
pub mod selection;
pub mod serialization;

pub use editor_state::{EditorState, TransformMode};
pub use object::{PrimitiveType, SceneObject};
pub use scene::Scene;
pub use selection::SelectionState;
pub use serialization::ForgeProject;
