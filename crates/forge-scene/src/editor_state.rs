use serde::{Deserialize, Serialize};

/// Active manipulation mode for the selected object in the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransformMode {
    #[default]
    Select,
    Translate,
    Rotate,
    Scale,
}

/// Transient editor preferences and viewport toggles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorState {
    pub transform_mode: TransformMode,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_wireframe: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            transform_mode: TransformMode::Select,
            show_grid: true,
            show_axes: true,
            show_wireframe: false,
        }
    }
}
