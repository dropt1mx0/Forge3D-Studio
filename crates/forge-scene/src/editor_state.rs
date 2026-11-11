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
    Sculpt,
}

/// Workspaces layout preset configuring panel and viewport layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkspaceMode {
    #[default]
    Modeling,
    Sculpting,
    UvEditing,
    Shading,
    Animation,
    Rendering,
    GeometryNodes,
    Scripting,
}

/// Active manipulation mode for the selected object or geometry in the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransformMode {
    #[default]
    Select,
    BoxSelect,
    Translate,
    Rotate,
    Scale,
}

/// Active transformation gizmo visualization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

/// Pointer interaction state machine for viewport input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PointerInteractionState {
    #[default]
    Idle,
    ObjectPressed,
    ObjectDragging,
    GizmoDragging,
    BoxSelecting,
    ViewportOrbiting,
    UIInteraction,
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

/// Display mode for the left toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToolbarDisplayMode {
    #[default]
    IconOnly,
    IconAndLabel,
}

/// Viewport MatCap shading mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewportMatcap {
    #[default]
    Studio = 0,
    Clay = 1,
    Chrome = 2,
    Normal = 3,
    Pearl = 4,
}

fn default_zoom_sensitivity() -> f32 {
    1.0
}

fn default_zoom_to_cursor() -> bool {
    true
}

fn default_smooth_zoom() -> bool {
    true
}

fn default_menu_bar_right_width() -> f32 {
    200.0
}

/// Recent project record stored in user configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentProjectItem {
    pub path: String,
    pub name: String,
    pub timestamp: u64,
}

/// Persistent workspace panel configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSettings {
    pub toolbar_width: f32,
    pub toolbar_visible: bool,
    pub toolbar_mode: ToolbarDisplayMode,
    pub sidebar_width: f32,
    pub sidebar_visible: bool,
    pub outliner_height_ratio: f32,
    pub timeline_height: f32,
    pub timeline_visible: bool,
    #[serde(default = "default_zoom_sensitivity")]
    pub zoom_sensitivity: f32,
    #[serde(default = "default_zoom_to_cursor")]
    pub zoom_to_cursor: bool,
    #[serde(default = "default_smooth_zoom")]
    pub smooth_zoom: bool,
    #[serde(default = "default_menu_bar_right_width")]
    pub menu_bar_right_width: f32,
    #[serde(default)]
    pub is_light_theme: bool,
    #[serde(default)]
    pub recent_projects: Vec<RecentProjectItem>,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            toolbar_width: 58.0,
            toolbar_visible: true,
            toolbar_mode: ToolbarDisplayMode::IconOnly,
            sidebar_width: 350.0,
            sidebar_visible: true,
            outliner_height_ratio: 0.45,
            timeline_height: 100.0,
            timeline_visible: false,
            zoom_sensitivity: 1.0,
            zoom_to_cursor: true,
            smooth_zoom: true,
            menu_bar_right_width: 200.0,
            is_light_theme: false,
            recent_projects: Vec::new(),
        }
    }
}

impl PanelSettings {
    pub const SETTINGS_FILE: &'static str = "forge3d_settings.json";

    /// Returns clamped panel settings within safe minimum and maximum bounds.
    pub fn clamped(&self) -> Self {
        Self {
            toolbar_width: self.toolbar_width.clamp(48.0, 280.0),
            toolbar_visible: self.toolbar_visible,
            toolbar_mode: self.toolbar_mode,
            sidebar_width: self.sidebar_width.clamp(220.0, 600.0),
            sidebar_visible: self.sidebar_visible,
            outliner_height_ratio: self.outliner_height_ratio.clamp(0.15, 0.85),
            timeline_height: self.timeline_height.clamp(60.0, 350.0),
            timeline_visible: self.timeline_visible,
            zoom_sensitivity: if self.zoom_sensitivity.is_finite() {
                self.zoom_sensitivity.clamp(0.1, 5.0)
            } else {
                1.0
            },
            zoom_to_cursor: self.zoom_to_cursor,
            smooth_zoom: self.smooth_zoom,
            menu_bar_right_width: self.menu_bar_right_width.clamp(80.0, 500.0),
            is_light_theme: self.is_light_theme,
            recent_projects: self.recent_projects.clone(),
        }
    }

    pub fn add_recent_project(&mut self, path: String, name: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.recent_projects.retain(|p| p.path != path);
        self.recent_projects.insert(
            0,
            RecentProjectItem {
                path,
                name,
                timestamp: now,
            },
        );
        if self.recent_projects.len() > 10 {
            self.recent_projects.truncate(10);
        }
        let _ = self.save();
    }

    pub fn remove_recent_project(&mut self, path: &str) {
        self.recent_projects.retain(|p| p.path != path);
        let _ = self.save();
    }

    pub fn clear_recent_projects(&mut self) {
        self.recent_projects.clear();
        let _ = self.save();
    }

    /// Resolves the file path for persistent settings.
    /// Prefers a local file if it exists (for portable runs and tests),
    /// otherwise defaults to the platform-standard config directory.
    pub fn settings_path() -> std::path::PathBuf {
        let local = std::path::PathBuf::from(Self::SETTINGS_FILE);
        if local.exists() {
            return local;
        }

        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            let dir = std::path::PathBuf::from(appdata).join("Forge3D");
            return dir.join(Self::SETTINGS_FILE);
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
                if !xdg.trim().is_empty() {
                    return std::path::PathBuf::from(xdg).join("forge3d").join(Self::SETTINGS_FILE);
                }
            }
            if let Ok(home) = std::env::var("HOME") {
                return std::path::PathBuf::from(home)
                    .join(".config")
                    .join("forge3d")
                    .join(Self::SETTINGS_FILE);
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        if let Ok(home) = std::env::var("HOME") {
            let dir = std::path::PathBuf::from(home).join(".config").join("forge3d");
            return dir.join(Self::SETTINGS_FILE);
        }

        local
    }

    /// Loads settings from disk if available, otherwise returns default settings.
    pub fn load_or_default() -> Self {
        let path = Self::settings_path();
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str::<Self>(&contents)
            .map(|s| s.clamped())
            .unwrap_or_default()
    }

    /// Saves settings to disk as formatted JSON.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self.clamped()).map_err(std::io::Error::other)?;
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, json)
    }
}

/// Transient modal state when actively transforming objects or geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModalTransformState {
    pub active: bool,
    pub mode: TransformMode,
    pub constraint: Option<AxisConstraint>,
    pub is_local_constraint: bool,
    pub numeric_input: String,
    pub initial_transforms: HashMap<ObjectId, Transform>,
    pub start_mouse_pos: Vec2,
    pub pivot_world: Vec3,
    pub start_distance_from_pivot: f32,
    pub start_angle_rad: f32,
}

impl ModalTransformState {
    /// Begins a modal transformation session.
    pub fn begin(
        &mut self,
        mode: TransformMode,
        start_mouse: Vec2,
        pivot: Vec3,
        initial_transforms: HashMap<ObjectId, Transform>,
    ) {
        self.active = true;
        self.mode = mode;
        self.constraint = None;
        self.is_local_constraint = false;
        self.numeric_input.clear();
        self.start_mouse_pos = start_mouse;
        self.pivot_world = pivot;
        self.initial_transforms = initial_transforms;
        self.start_distance_from_pivot = (start_mouse - Vec2::new(pivot.x, pivot.y))
            .length()
            .max(1.0);
        self.start_angle_rad = (start_mouse.y - pivot.y).atan2(start_mouse.x - pivot.x);
    }

    /// Toggles axis constraint: if same axis pressed again, toggles between Global and Local space!
    pub fn set_or_toggle_axis(&mut self, axis: AxisConstraint) {
        if self.constraint == Some(axis) {
            if self.is_local_constraint {
                // Third press clears constraint
                self.constraint = None;
                self.is_local_constraint = false;
            } else {
                // Second press switches to local
                self.is_local_constraint = true;
            }
        } else {
            self.constraint = Some(axis);
            self.is_local_constraint = false;
        }
    }

    /// Resets the modal transform state.
    pub fn cancel(&mut self) {
        self.active = false;
        self.constraint = None;
        self.is_local_constraint = false;
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
    pub active_gizmo_mode: GizmoMode,
    pub selection_element: SelectionElement,
    pub transform_orientation: TransformOrientation,
    pub pivot_point: PivotPoint,
    pub snap_settings: SnapSettings,
    pub panel_settings: PanelSettings,
    pub workspace: WorkspaceMode,
    pub cursor_3d: Vec3,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_wireframe: bool,
    pub show_xray: bool,
    pub xray_alpha: f32,
    pub viewport_matcap: ViewportMatcap,
    pub use_gpu_wireframe: bool,
    pub use_gpu_vertex_handles: bool,
    pub use_gpu_picking: bool,
    pub proportional_editing: bool,
    pub proportional_radius: f32,
    pub emulate_numpad: bool,

    #[serde(skip)]
    pub fly_nav_active: bool,
    #[serde(skip)]
    pub walk_mode: bool,
    #[serde(skip)]
    pub is_camera_view: bool,
    #[serde(skip)]
    pub modal_transform: ModalTransformState,
    #[serde(skip)]
    pub transform_clipboard: Option<Transform>,
    #[serde(skip)]
    pub box_select_start: Option<Vec2>,
    #[serde(skip)]
    pub pointer_state: PointerInteractionState,
    #[serde(skip)]
    pub pointer_press_pos: Option<Vec2>,
    #[serde(skip)]
    pub active_vertex: Option<usize>,
    #[serde(skip)]
    pub show_debug_info: bool,
    #[serde(skip)]
    pub active_snap_target: Option<Vec3>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            edit_mode: EditMode::Object,
            transform_mode: TransformMode::Select,
            active_gizmo_mode: GizmoMode::Translate,
            selection_element: SelectionElement::Vertex,
            transform_orientation: TransformOrientation::Global,
            pivot_point: PivotPoint::MedianPoint,
            snap_settings: SnapSettings::default(),
            panel_settings: PanelSettings::default(),
            workspace: WorkspaceMode::Modeling,
            cursor_3d: Vec3::ZERO,
            show_grid: true,
            show_axes: true,
            show_wireframe: false,
            show_xray: false,
            xray_alpha: 0.5,
            viewport_matcap: ViewportMatcap::Studio,
            use_gpu_wireframe: true,
            use_gpu_vertex_handles: true,
            use_gpu_picking: true,
            proportional_editing: false,
            proportional_radius: 1.0,
            emulate_numpad: false,
            fly_nav_active: false,
            walk_mode: false,
            is_camera_view: false,
            modal_transform: ModalTransformState::default(),
            transform_clipboard: None,
            box_select_start: None,
            pointer_state: PointerInteractionState::Idle,
            pointer_press_pos: None,
            active_vertex: None,
            show_debug_info: false,
            active_snap_target: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_transform_axis_toggle() {
        let mut modal = ModalTransformState::default();
        modal.begin(
            TransformMode::Translate,
            Vec2::new(100.0, 100.0),
            Vec3::ZERO,
            HashMap::new(),
        );
        assert!(modal.active);
        assert_eq!(modal.constraint, None);

        // First press: X (Global)
        modal.set_or_toggle_axis(AxisConstraint::X);
        assert_eq!(modal.constraint, Some(AxisConstraint::X));
        assert!(!modal.is_local_constraint);

        // Second press: X (Local)
        modal.set_or_toggle_axis(AxisConstraint::X);
        assert_eq!(modal.constraint, Some(AxisConstraint::X));
        assert!(modal.is_local_constraint);

        // Third press: clears constraint
        modal.set_or_toggle_axis(AxisConstraint::X);
        assert_eq!(modal.constraint, None);
        assert!(!modal.is_local_constraint);
    }

    #[test]
    fn test_modal_numeric_input() {
        let mut modal = ModalTransformState::default();
        modal.push_char('-');
        modal.push_char('4');
        modal.push_char('.');
        modal.push_char('5');
        assert_eq!(modal.parsed_number(), Some(-4.5));

        modal.pop_char();
        assert_eq!(modal.parsed_number(), Some(-4.0));

        modal.cancel();
        assert!(!modal.active);
        assert!(modal.numeric_input.is_empty());
    }
}
