use std::collections::HashMap;

use forge_core::id::ObjectId;
use forge_math::{Transform, Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Mode of operation in the 3D editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EditMode {
    #[default]
    Object,
    Edit,
}

/// Active manipulation mode for the selected object or geometry in the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransformMode {
    #[default]
    Select,
    Translate,
    Rotate,
    Scale,
}

/// Mesh component selection type in Edit Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectionElement {
    #[default]
    Vertex,
    Edge,
    Face,
}

/// Transform orientation frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransformOrientation {
    #[default]
    Global,
    Local,
}

/// Pivot point used as origin for rotations and scalings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PivotPoint {
    #[default]
    MedianPoint,
    ActiveElement,
    Cursor3D,
    IndividualOrigins,
}

/// Axis or plane constraint during transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxisConstraint {
    X,
    Y,
    Z,
    PlaneXY,
    PlaneXZ,
    PlaneYZ,
}

/// Snapping options for precision transformation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SnapSettings {
    pub enabled: bool,
    pub snap_to_grid: bool,
    pub snap_to_vertex: bool,
    pub grid_increment: f32,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            snap_to_grid: true,
            snap_to_vertex: false,
            grid_increment: 1.0,
        }
    }
}

/// Transient modal state when actively transforming objects or geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModalTransformState {
    pub active: bool,
    pub mode: TransformMode,
    pub constraint: Option<AxisConstraint>,
    pub numeric_input: String,
    pub initial_transforms: HashMap<ObjectId, Transform>,
    pub start_mouse_pos: Vec2,
    pub pivot_world: Vec3,
}

impl ModalTransformState {
    /// Resets the modal transform state.
    pub fn cancel(&mut self) {
        self.active = false;
        self.constraint = None;
        self.numeric_input.clear();
        self.initial_transforms.clear();
    }

    /// Appends a character to the numeric input buffer (e.g., '2', '.', '-').
    pub fn push_char(&mut self, c: char) {
        if c.is_ascii_digit() || c == '.' || c == '-' {
            if c == '-' && !self.numeric_input.is_empty() {
                return;
            }
            if c == '.' && self.numeric_input.contains('.') {
                return;
            }
            self.numeric_input.push(c);
        }
    }

    /// Backspaces the numeric buffer.
    pub fn pop_char(&mut self) {
        self.numeric_input.pop();
    }

    /// Parses the numeric input buffer as f32 if valid.
    pub fn parsed_number(&self) -> Option<f32> {
        self.numeric_input.parse::<f32>().ok()
    }
}

/// Transient editor preferences, modes, and viewport toggles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorState {
    pub edit_mode: EditMode,
    pub transform_mode: TransformMode,
    pub selection_element: SelectionElement,
    pub transform_orientation: TransformOrientation,
    pub pivot_point: PivotPoint,
    pub snap_settings: SnapSettings,
    pub cursor_3d: Vec3,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_wireframe: bool,
    pub proportional_editing: bool,
    pub proportional_radius: f32,

    #[serde(skip)]
    pub modal_transform: ModalTransformState,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            edit_mode: EditMode::Object,
            transform_mode: TransformMode::Select,
            selection_element: SelectionElement::Vertex,
            transform_orientation: TransformOrientation::Global,
            pivot_point: PivotPoint::MedianPoint,
            snap_settings: SnapSettings::default(),
            cursor_3d: Vec3::ZERO,
            show_grid: true,
            show_axes: true,
            show_wireframe: false,
            proportional_editing: false,
            proportional_radius: 1.0,
            modal_transform: ModalTransformState::default(),
        }
    }
}
