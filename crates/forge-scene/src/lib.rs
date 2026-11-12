//! # Forge3D Scene
//!
//! Scene graph, object hierarchy, selection state, editor preferences,
//! and versioned project serialization (`.forge`).
//!
//! Strictly contains NO GPU handles or UI dependencies.

pub mod editor_state;
pub mod extension;
pub mod history;
pub mod material;
pub mod object;
pub mod scene;
pub mod selection;
pub mod serialization;
pub mod shader_graph;

pub use editor_state::{
    AxisConstraint, EditMode, EditorState, GizmoMode, ModalTransformState, PanelSettings,
    PivotPoint, PointerInteractionState, RecentProjectItem, SelectionElement, SnapSettings,
    ToolbarDisplayMode, TransformMode, TransformOrientation, ViewportMatcap, WorkspaceMode,
};
pub use extension::{
    CustomOperator, EventHook, ExtensionManager, PluginManifest, PluginPermission, PluginState,
    RegisteredPlugin,
};
pub use history::{Action, HistoryStack};
pub use material::{LightData, LightKind, PbrMaterial, PbrTextureSet, WorldLighting};
pub use object::{
    CameraData, CameraProjectionType, CameraViewParameters, PrimitiveType, SceneObject,
    TransformLocks,
};
pub use scene::{BROWN_FLOOR_TILES_MATERIAL_ID, Scene};
pub use selection::SelectionState;
pub use serialization::ForgeProject;
pub use shader_graph::{
    Connection, InputSocket, MathOp, NodeKind, NodeSocketRef, OutputSocket, ShaderGraph,
    ShaderNode, SocketDefaultValue, SocketType, VectorMathOp,
};
