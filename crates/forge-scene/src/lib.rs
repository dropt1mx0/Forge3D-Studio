//! # Forge3D Scene
//!
//! Scene graph, object hierarchy, selection state, editor preferences,
//! and versioned project serialization (`.forge`).
//!
//! Strictly contains NO GPU handles or UI dependencies.

pub mod editor_state;
pub mod history;
pub mod object;
pub mod scene;
pub mod selection;
pub mod serialization;

pub use editor_state::{
    AxisConstraint, EditMode, EditorState, ModalTransformState, PivotPoint, SelectionElement,
    SnapSettings, TransformMode, TransformOrientation,
};
pub use history::{Action, HistoryStack};
pub use object::{
    CameraData, CameraProjectionType, CameraViewParameters, PrimitiveType, SceneObject,
    TransformLocks,
};
pub use scene::Scene;
pub use selection::SelectionState;
pub use serialization::ForgeProject;
