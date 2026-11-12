use std::time::Instant;

use eframe::egui;
use forge_core::id::ObjectId;
use forge_geometry::{EditSelectionMode, EditableMesh};
use forge_gpu::GpuAdapterInfo;
use forge_math::{Ray, Transform, Vec2, Vec3};
use forge_modifiers::ModifierType;
use forge_render::{Camera, ViewportRenderer};
use forge_scene::{
    Action, EditMode, EditorState, HistoryStack, PrimitiveType, Scene, SceneObject,
    SelectionElement, SelectionState,
};

use crate::ui::gizmo::GizmoState;
use crate::ui::menu_bar::render_menu_bar;
use crate::ui::outliner::{render_outliner, OutlinerFilterSettings, OutlinerTypeFilter};
use crate::ui::properties::render_properties;
use crate::ui::status_bar::render_status_bar;
use crate::ui::toolbar::render_toolbar;
use crate::ui::viewport::render_viewport;

const FORGE3D_LOGO_BYTES: &[u8] = include_bytes!("../assets/forge3d-logo.jpg");
const MASCOT_BYTES: &[u8] = include_bytes!("../assets/mascot.png");

fn load_forge3d_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let decoded = image::load_from_memory(FORGE3D_LOGO_BYTES).ok()?;
    let thumbnail = decoded
        .resize_exact(96, 96, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let size = [thumbnail.width() as usize, thumbnail.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, thumbnail.as_raw());

    Some(ctx.load_texture("forge3d-logo", color_image, egui::TextureOptions::LINEAR))
}

fn load_mascot_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let decoded = image::load_from_memory(MASCOT_BYTES).ok()?;
    let rgba = decoded.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());

    Some(ctx.load_texture("forge3d-mascot", color_image, egui::TextureOptions::LINEAR))
}

fn create_radial_bloom_image() -> egui::ColorImage {
    let size = 128;
    let mut pixels = Vec::with_capacity(size * size * 4);
    let center = size as f32 / 2.0;
    let radius = center;
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5) - center;
            let dy = (y as f32 + 0.5) - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let norm = (dist / radius).clamp(0.0, 1.0);
            let alpha = if norm >= 1.0 {
                0.0
            } else {
                let t = 1.0 - norm;
                let smooth = t * t * (3.0 - 2.0 * t); // smoothstep with zero derivative at both center and edge
                (smooth * smooth) * 82.0
            };
            let a = alpha.clamp(0.0, 255.0) as u8;
            pixels.push(255);
            pixels.push(140);
            pixels.push(20);
            pixels.push(a);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([size, size], &pixels)
}

fn load_glow_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let color_image = create_radial_bloom_image();
    Some(ctx.load_texture("mascot-glow", color_image, egui::TextureOptions::LINEAR))
}

/// Active state for free cursor object dragging in world space.
#[derive(Debug, Clone)]
pub struct ObjectDragState {
    /// Initial 3D intersection point on the object's surface where dragging started.
    pub initial_hit_point: Vec3,
    /// Working plane point through which the camera ray will intersect during drag.
    pub plane_point: Vec3,
    /// Working plane normal facing the camera.
    pub plane_normal: Vec3,
    /// Initial local transforms of all selected objects before drag began.
    pub initial_transforms: std::collections::HashMap<ObjectId, Transform>,
    /// Initial offsets from initial hit point to each selected object's translation.
    pub initial_offsets: std::collections::HashMap<ObjectId, Vec3>,
    /// Screen-space pointer position when pressed.
    pub initial_pointer_pos: Vec2,
    /// Whether the drag threshold has been exceeded and active moving is happening.
    pub is_active_drag: bool,
    /// The object that was clicked on.
    pub pressed_object_id: Option<ObjectId>,
    /// Whether this click started on an already selected object.
    pub was_already_selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieMenuKind {
    Mode,
    Shading,
    Pivot,
    SnapCursor,
    UvMapping,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PieMenuState {
    pub kind: PieMenuKind,
    pub center: egui::Pos2,
}

pub struct Forge3dStudioApp {
    pub scene: Scene,
    pub selection: SelectionState,
    pub editor_state: EditorState,
    pub camera: Camera,
    pub renderer: Option<ViewportRenderer>,
    pub gpu_info: Option<GpuAdapterInfo>,
    pub status_message: String,
    pub show_about_dialog: bool,
    pub show_gpu_dialog: bool,
    pub show_material_editor: bool,
    pub show_shader_editor: bool,
    pub active_shader_graph: forge_scene::ShaderGraph,
    pub shader_editor_state: crate::ui::shader_editor::ShaderEditorState,
    pub show_command_palette: bool,
    pub show_quick_add_menu: Option<egui::Pos2>,
    pub active_pie_menu: Option<PieMenuState>,
    pub show_plugin_manager: bool,
    pub is_in_startup_hub: bool,
    pub extension_manager: forge_scene::ExtensionManager,
    pub command_palette_query: String,
    pub command_palette_selected_index: usize,
    pub sculpt_settings: forge_geometry::SculptSettings,
    pub sculpt_stroke_initial_mesh: Option<std::sync::Arc<forge_geometry::Mesh>>,

    // Undo / Redo Command History
    pub history: HistoryStack,

    // Interactive 3D Viewport Gizmo
    pub gizmo_state: GizmoState,

    // Active mesh in Edit Mode: (ObjectId, EditableMesh)
    pub active_edit_mesh: Option<(ObjectId, EditableMesh)>,
    pub edit_overlay_cache: crate::ui::viewport::EditOverlayCache,

    // Stored WGPU render state from CreationContext
    pub wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// Embedded Forge3D brand mark shown in the top menu bar.
    pub logo_texture: Option<egui::TextureHandle>,
    /// Embedded Forge3D orange mascot texture for the startup hub.
    pub mascot_texture: Option<egui::TextureHandle>,
    /// Smooth radial bloom texture rendered behind the mascot.
    pub glow_texture: Option<egui::TextureHandle>,

    // Offscreen 3D viewport render target
    pub viewport_texture: Option<wgpu::Texture>,
    pub viewport_texture_view: Option<wgpu::TextureView>,
    pub viewport_egui_texture_id: Option<egui::TextureId>,
    pub viewport_pixel_size: (u32, u32),

    // Performance metrics
    last_frame_instant: Instant,
    pub fps: f32,
    pub frame_time_ms: f32,

    /// Elapsed time for the in-viewport showcase construction visualization.
    /// `None` means the playback is complete or not running.
    pub showcase_demo_elapsed: Option<f32>,

    // Multi-object transformation initial transforms cache
    pub gizmo_initial_transforms: std::collections::HashMap<ObjectId, Transform>,

    // Free cursor object dragging state
    pub object_drag_state: Option<ObjectDragState>,

    // Pending asynchronous GPU picking request for hover with full context validation
    pub pending_hover_pick: Option<PendingHoverPick>,

    /// Blender-style Local View / Isolate Selected state: saved visibility map when active.
    pub local_view_saved_visibility: Option<std::collections::HashMap<ObjectId, bool>>,

    /// Viewport Overlays popover dropdown menu open state.
    pub show_overlays_dropdown: bool,

    /// Viewport navigation widget expanded/collapsed state.
    pub nav_widget_expanded: bool,

    /// Blender-style Outliner search, restriction toggles, and filter settings.
    pub outliner_filter: OutlinerFilterSettings,

    /// Active interactive modeling tool state in Edit Mode (Extrude, Inset, Loop Cut).
    pub interactive_edit_tool: Option<InteractiveEditToolState>,
}

/// Interactive Edit Mode tool types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveEditToolType {
    Extrude,
    Inset,
    LoopCut,
}

/// Active interactive modeling tool state (modal execution with mouse sliding).
#[derive(Debug, Clone)]
pub struct InteractiveEditToolState {
    pub tool_type: InteractiveEditToolType,
    pub start_pointer_pos: Vec2,
    pub initial_mesh: std::sync::Arc<forge_geometry::Mesh>,
    pub initial_edit_mesh: EditableMesh,
    pub target_edge_idx: usize,
    pub current_value: f32,
}

/// Tracks a pending asynchronous hover picking request with full context to validate click pre-polling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingHoverPick {
    pub request_id: u64,
    pub pixel_coords: (u32, u32),
    pub edit_mode: EditMode,
    pub selection_element: SelectionElement,
    pub active_object_id: Option<ObjectId>,
    pub invalidation_epoch: u64,
}

fn set_showcase_camera_view(camera: &mut Camera) {
    camera.target = Vec3::new(0.0, 1.15, 0.0);
    camera.eye = Vec3::new(7.2, 5.25, 8.2);
    camera.up = Vec3::Y;
    camera.sync_zoom_targets();
}

fn showcase_object_stage(name: &str) -> usize {
    if name == "Grassy Ground" || name == "Front Path" {
        0
    } else if name == "House Foundation" || name == "House Walls" {
        1
    } else if name.starts_with("Roof ") || name.starts_with("Chimney") {
        2
    } else if name.starts_with("Porch ")
        || name == "Front Door"
        || name == "Door Handle"
        || name.contains("Window")
    {
        3
    } else if name.starts_with("Tree ") {
        4
    } else {
        5
    }
}

fn reveal_showcase_through_stage(scene: &mut Scene, stage: usize) -> bool {
    let mut changed = false;
    for object in scene.objects.values_mut() {
        let should_be_visible = showcase_object_stage(&object.name) <= stage;
        if object.visible != should_be_visible {
            object.visible = should_be_visible;
            changed = true;
        }
    }
    if changed {
        scene.dirty = true;
    }
    changed
}

fn default_extension_manager() -> forge_scene::ExtensionManager {
    let mut mgr = forge_scene::ExtensionManager::new();
    let core_manifest = forge_scene::PluginManifest::new(
        "org.forge3d.core_tools",
        "Core Modeling Tools",
        "0.1.0",
        "Forge3D Team",
    )
    .with_permission(forge_scene::PluginPermission::SceneMutate);
    let _ = mgr.register_plugin(core_manifest);

    let batch_manifest = forge_scene::PluginManifest::new(
        "org.forge3d.batch_converter",
        "Batch Format Converter",
        "0.1.0",
        "Forge3D Team",
    )
    .with_permission(forge_scene::PluginPermission::FileRead)
    .with_permission(forge_scene::PluginPermission::FileWrite)
    .with_permission(forge_scene::PluginPermission::SceneMutate);
    let _ = mgr.register_plugin(batch_manifest);

    mgr
}

pub fn dark_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(19, 20, 25);
    visuals.window_fill = egui::Color32::from_rgb(24, 25, 32);
    visuals.faint_bg_color = egui::Color32::from_rgb(16, 17, 21);
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 13, 16);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 28, 36);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(36, 39, 49));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(32, 35, 44);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 46, 58));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(44, 48, 62);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 66, 84));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(37, 99, 235);
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(96, 165, 250));
    visuals.selection.bg_fill = egui::Color32::from_rgb(37, 99, 235); // Studio Electric Blue
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(96, 165, 250));
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals
}

pub fn light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::from_rgb(245, 246, 250);
    visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
    visuals.faint_bg_color = egui::Color32::from_rgb(238, 240, 246);
    visuals.extreme_bg_color = egui::Color32::from_rgb(228, 232, 240);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(240, 242, 248);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(215, 220, 232));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(235, 238, 246);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(205, 212, 224));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(224, 230, 242);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 192, 212));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(37, 99, 235);
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(29, 78, 216));
    visuals.selection.bg_fill = egui::Color32::from_rgb(37, 99, 235);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(29, 78, 216));
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals
}

pub fn apply_theme(is_light: bool, ctx: &egui::Context) {
    if is_light {
        ctx.set_visuals(light_visuals());
    } else {
        ctx.set_visuals(dark_visuals());
    }
}

impl Forge3dStudioApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let editor_state = EditorState {
            panel_settings: forge_scene::PanelSettings::load_or_default(),
            ..Default::default()
        };

        apply_theme(editor_state.panel_settings.is_light_theme, &cc.egui_ctx);

        let mut renderer = None;
        let mut gpu_info = None;

        if let Some(wgpu_state) = &cc.wgpu_render_state {
            let info = wgpu_state.adapter.get_info();
            let parsed_info = GpuAdapterInfo::from_wgpu(&info);
            tracing::info!("Initialized 3D Viewport on GPU: {}", parsed_info);
            gpu_info = Some(parsed_info);

            // Viewport targets RGBA8UnormSrgb texture
            renderer = Some(ViewportRenderer::new(
                &wgpu_state.device,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ));
        }

        let logo_texture = load_forge3d_logo(&cc.egui_ctx);
        if logo_texture.is_none() {
            tracing::warn!("Could not decode the embedded Forge3D logo");
        }

        let mascot_texture = load_mascot_texture(&cc.egui_ctx);
        if mascot_texture.is_none() {
            tracing::warn!("Could not decode the embedded Forge3D mascot");
        }

        let glow_texture = load_glow_texture(&cc.egui_ctx);

        let editor_state = EditorState {
            panel_settings: forge_scene::PanelSettings::load_or_default(),
            ..Default::default()
        };

        let mut camera = Camera::default();
        camera.settings.zoom_sensitivity = editor_state.panel_settings.zoom_sensitivity;
        camera.settings.zoom_to_cursor = editor_state.panel_settings.zoom_to_cursor;
        camera.settings.smooth_zoom = editor_state.panel_settings.smooth_zoom;

        Self {
            scene: Scene::new("Untitled"),
            selection: SelectionState::new(),
            editor_state,
            camera,
            renderer,
            gpu_info,
            status_message: "Ready".to_string(),
            show_about_dialog: false,
            show_gpu_dialog: false,
            show_material_editor: false,
            show_shader_editor: false,
            active_shader_graph: forge_scene::ShaderGraph::default(),
            shader_editor_state: crate::ui::shader_editor::ShaderEditorState::default(),
            show_command_palette: false,
            show_quick_add_menu: None,
            active_pie_menu: None,
            show_plugin_manager: false,
            is_in_startup_hub: true,
            extension_manager: default_extension_manager(),
            command_palette_query: String::new(),
            command_palette_selected_index: 0,
            sculpt_settings: forge_geometry::SculptSettings::default(),
            sculpt_stroke_initial_mesh: None,
            history: HistoryStack::new(),
            gizmo_state: GizmoState::new(),
            active_edit_mesh: None,
            edit_overlay_cache: crate::ui::viewport::EditOverlayCache::default(),
            wgpu_render_state: cc.wgpu_render_state.clone(),
            logo_texture,
            mascot_texture,
            glow_texture,
            viewport_texture: None,
            viewport_texture_view: None,
            viewport_egui_texture_id: None,
            viewport_pixel_size: (0, 0),
            last_frame_instant: Instant::now(),
            fps: 60.0,
            frame_time_ms: 16.6,
            showcase_demo_elapsed: None,
            gizmo_initial_transforms: std::collections::HashMap::new(),
            object_drag_state: None,
            pending_hover_pick: None,
            local_view_saved_visibility: None,
            show_overlays_dropdown: false,
            nav_widget_expanded: false,
            outliner_filter: OutlinerFilterSettings::default(),
            interactive_edit_tool: None,
        }
    }

    /// Creates a headless instance of the app (no GPU/eframe required) for automated integration testing.
    pub fn new_headless() -> Self {
        let editor_state = EditorState::default();
        let mut camera = Camera::default();
        camera.settings.zoom_sensitivity = editor_state.panel_settings.zoom_sensitivity;
        camera.settings.zoom_to_cursor = editor_state.panel_settings.zoom_to_cursor;
        camera.settings.smooth_zoom = editor_state.panel_settings.smooth_zoom;

        Self {
            scene: Scene::create_default_scene(),
            selection: SelectionState::new(),
            editor_state,
            camera,
            renderer: None,
            gpu_info: None,
            status_message: "Ready (Headless)".to_string(),
            show_about_dialog: false,
            show_gpu_dialog: false,
            show_material_editor: false,
            show_shader_editor: false,
            active_shader_graph: forge_scene::ShaderGraph::default(),
            shader_editor_state: crate::ui::shader_editor::ShaderEditorState::default(),
            show_command_palette: false,
            show_quick_add_menu: None,
            active_pie_menu: None,
            show_plugin_manager: false,
            is_in_startup_hub: false,
            extension_manager: default_extension_manager(),
            command_palette_query: String::new(),
            command_palette_selected_index: 0,
            sculpt_settings: forge_geometry::SculptSettings::default(),
            sculpt_stroke_initial_mesh: None,
            history: HistoryStack::new(),
            gizmo_state: GizmoState::new(),
            active_edit_mesh: None,
            edit_overlay_cache: crate::ui::viewport::EditOverlayCache::default(),
            wgpu_render_state: None,
            logo_texture: None,
            mascot_texture: None,
            glow_texture: None,
            viewport_texture: None,
            viewport_texture_view: None,
            viewport_egui_texture_id: None,
            viewport_pixel_size: (1280, 800),
            last_frame_instant: Instant::now(),
            fps: 60.0,
            frame_time_ms: 16.6,
            showcase_demo_elapsed: None,
            gizmo_initial_transforms: std::collections::HashMap::new(),
            object_drag_state: None,
            pending_hover_pick: None,
            local_view_saved_visibility: None,
            show_overlays_dropdown: false,
            nav_widget_expanded: false,
            outliner_filter: OutlinerFilterSettings::default(),
            interactive_edit_tool: None,
        }
    }

    /// Sets the active interaction mode (Object, Edit, Sculpt).
    pub fn set_edit_mode(&mut self, target_mode: EditMode) {
        if self.editor_state.edit_mode == target_mode {
            return;
        }
        self.pending_hover_pick = None;
        if let Some(r) = &mut self.renderer {
            r.invalidate_picking();
        }

        match target_mode {
            EditMode::Object => {
                if self.editor_state.edit_mode == EditMode::Edit || self.editor_state.edit_mode == EditMode::Sculpt {
                    self.commit_edit_mesh_to_scene();
                    self.active_edit_mesh = None;
                    self.edit_overlay_cache.clear();
                }
                self.editor_state.edit_mode = EditMode::Object;
                self.status_message = "Object Mode".to_string();
            }
            EditMode::Edit => {
                if self.editor_state.edit_mode == EditMode::Sculpt {
                    self.editor_state.edit_mode = EditMode::Edit;
                    self.status_message = "Edit Mode".to_string();
                } else if let Some(active_id) = self.selection.active_id {
                    if let Some(obj) = self.scene.get_object(active_id) {
                        let mut edit_mesh = EditableMesh::from_mesh(&obj.mesh);
                        edit_mesh.weld_colocated_vertices(1e-4);
                        edit_mesh.tris_to_quads();
                        self.active_edit_mesh = Some((active_id, edit_mesh));
                        self.edit_overlay_cache.reset();
                        self.editor_state.edit_mode = EditMode::Edit;
                        self.status_message = format!("Edit Mode: {}", obj.name);
                    }
                } else {
                    self.status_message = "Select an object to enter Edit Mode.".to_string();
                }
            }
            EditMode::Sculpt => {
                if self.editor_state.edit_mode == EditMode::Edit {
                    self.editor_state.edit_mode = EditMode::Sculpt;
                    self.status_message = "Sculpt Mode".to_string();
                } else if let Some(active_id) = self.selection.active_id {
                    if let Some(obj) = self.scene.get_object(active_id) {
                        let edit_mesh = EditableMesh::from_mesh(&obj.mesh);
                        self.active_edit_mesh = Some((active_id, edit_mesh));
                        self.edit_overlay_cache.reset();
                        self.editor_state.edit_mode = EditMode::Sculpt;
                        self.status_message = format!("Sculpt Mode: {}", obj.name);
                    }
                } else {
                    self.status_message = "Select an object to enter Sculpt Mode.".to_string();
                }
            }
        }
    }

    /// Toggles between Object Mode and Edit Mode.
    pub fn toggle_edit_mode(&mut self) {
        if self.editor_state.edit_mode == EditMode::Object {
            self.set_edit_mode(EditMode::Edit);
        } else {
            self.set_edit_mode(EditMode::Object);
        }
    }

    pub fn set_workspace(&mut self, workspace: forge_scene::WorkspaceMode) {
        self.editor_state.workspace = workspace;
        match workspace {
            forge_scene::WorkspaceMode::Modeling => {
                self.editor_state.panel_settings.toolbar_visible = true;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = false;
                self.show_material_editor = false;
                if self.editor_state.edit_mode == EditMode::Sculpt {
                    self.editor_state.edit_mode = EditMode::Object;
                }
            }
            forge_scene::WorkspaceMode::Sculpting => {
                self.editor_state.panel_settings.toolbar_visible = true;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = false;
                self.show_material_editor = false;
                if self.editor_state.edit_mode != EditMode::Sculpt {
                    if let Some(active_id) = self.selection.active_id
                        && let Some(obj) = self.scene.get_object(active_id)
                    {
                        let edit_mesh = EditableMesh::from_mesh(&obj.mesh);
                        self.active_edit_mesh = Some((active_id, edit_mesh));
                    }
                    self.editor_state.edit_mode = EditMode::Sculpt;
                }
            }
            forge_scene::WorkspaceMode::UvEditing => {
                self.editor_state.panel_settings.toolbar_visible = true;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = false;
                self.show_material_editor = false;
            }
            forge_scene::WorkspaceMode::Shading => {
                self.editor_state.panel_settings.toolbar_visible = true;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = false;
                self.show_material_editor = true;
                self.open_shader_editor_for_selected();
            }
            forge_scene::WorkspaceMode::Animation => {
                self.editor_state.panel_settings.toolbar_visible = true;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = true;
                self.show_material_editor = false;
            }
            forge_scene::WorkspaceMode::Rendering => {
                self.editor_state.panel_settings.toolbar_visible = false;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = false;
                self.show_material_editor = true;
            }
            forge_scene::WorkspaceMode::GeometryNodes => {
                self.editor_state.panel_settings.toolbar_visible = true;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = false;
                self.show_material_editor = false;
            }
            forge_scene::WorkspaceMode::Scripting => {
                self.editor_state.panel_settings.toolbar_visible = false;
                self.editor_state.panel_settings.sidebar_visible = true;
                self.editor_state.panel_settings.timeline_visible = true;
                self.show_material_editor = false;
            }
        }
        self.status_message = format!("Switched to {:?} Workspace", workspace);
    }

    /// Opens the Shader Node Editor for the currently selected object's material.
    pub fn open_shader_editor_for_selected(&mut self) {
        if let Some(active_id) = self.selection.active_id {
            if let Some(obj) = self.scene.get_object(active_id) {
                if let Some(mat_id) = &obj.material_id {
                    if let Some(mat) = self.scene.materials.get(mat_id) {
                        if let Some(graph) = &mat.shader_graph {
                            self.active_shader_graph = graph.clone();
                        } else {
                            let mut graph = forge_scene::ShaderGraph::standard_pbr_graph(&mat.name);
                            if let Some(bsdf) = graph
                                .nodes
                                .iter_mut()
                                .find(|n| matches!(n.kind, forge_scene::NodeKind::PrincipledBsdf))
                            {
                                if let Some(inp) = bsdf.inputs.iter_mut().find(|i| i.id == "base_color") {
                                    inp.default_value = forge_scene::SocketDefaultValue::Color(mat.base_color_factor);
                                }
                                if let Some(inp) = bsdf.inputs.iter_mut().find(|i| i.id == "metallic") {
                                    inp.default_value = forge_scene::SocketDefaultValue::Float(mat.metallic_factor);
                                }
                                if let Some(inp) = bsdf.inputs.iter_mut().find(|i| i.id == "roughness") {
                                    inp.default_value = forge_scene::SocketDefaultValue::Float(mat.roughness_factor);
                                }
                            }
                            self.active_shader_graph = graph;
                        }
                    }
                }
            }
        }
        self.show_shader_editor = true;
        self.shader_editor_state.wgsl_dirty = true;
    }

    /// Evaluates the active shader graph and updates the active object's material
    /// (and active material in scene) so changes reflect immediately in the viewport.
    pub fn sync_active_shader_graph_to_material(&mut self) {
        let pbr = self.active_shader_graph.evaluate_to_pbr_material();
        if let Some(active_id) = self.selection.active_id {
            let existing_mat_id = self
                .scene
                .get_object(active_id)
                .and_then(|obj| obj.material_id.clone());

            let mat_id = match existing_mat_id {
                Some(id) => id,
                None => {
                    let new_id = format!("material_{}", active_id);
                    if let Some(obj) = self.scene.get_object_mut(active_id) {
                        obj.material_id = Some(new_id.clone());
                    }
                    new_id
                }
            };

            let mut updated_pbr = pbr.clone();
            updated_pbr.shader_graph = Some(self.active_shader_graph.clone());
            if let Some(existing) = self.scene.materials.get(&mat_id) {
                updated_pbr.textures = existing.textures.clone();
            }
            self.scene.materials.insert(mat_id, updated_pbr);
            if let Some(obj) = self.scene.get_object_mut(active_id) {
                obj.color = pbr.base_color_factor;
            }
            self.scene.dirty = true;
        }
    }

    /// Commits changes in `active_edit_mesh` back to the scene object and marks scene dirty.
    pub fn commit_edit_mesh_to_scene(&mut self) {
        if let Some((id, edit_mesh)) = &self.active_edit_mesh
            && let Some(obj) = self.scene.get_object_mut(*id)
        {
            let old_mesh = obj.mesh.clone();
            let new_mesh = edit_mesh.to_render_mesh();
            obj.set_mesh(new_mesh.clone());
            self.scene.dirty = true;

            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(id);
            }

            self.history.push(Action::modify_mesh(
                *id,
                old_mesh,
                new_mesh,
                "Edit Mesh Geometry",
            ));
            self.edit_overlay_cache.reset();
        }
    }

    /// Blender-style "Select Linked" (L / Shift+L): selects or deselects all connected geometry
    /// in the active edit mesh belonging to the same island as the hovered or selected element.
    pub fn select_linked(&mut self, deselect: bool) {
        if self.editor_state.edit_mode != EditMode::Edit {
            return;
        }

        let seed = self.editor_state.active_vertex;
        let mode = match self.editor_state.selection_element {
            forge_scene::SelectionElement::Vertex => EditSelectionMode::Vertex,
            forge_scene::SelectionElement::Edge => EditSelectionMode::Edge,
            forge_scene::SelectionElement::Face => EditSelectionMode::Face,
        };

        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            let count = edit_mesh.select_linked(mode, seed, deselect);
            self.sync_edit_mesh_to_scene();
            let action = if deselect { "Deselected" } else { "Selected" };
            self.status_message = format!("{} {} linked element(s) (L)", action, count);
        }
    }

    /// Performs uniform Voxel Remeshing on the active object's geometry.
    /// Pushes an undo step (`Action::ModifyMesh`) for full non-destructive rollback.
    pub fn voxel_remesh_active(&mut self, voxel_size: f32) {
        let Some(active_id) = self.selection.active_id else {
            self.status_message = "Voxel Remesh: No active object selected".into();
            return;
        };

        // If in Edit Mode, commit first
        if self.editor_state.edit_mode == EditMode::Edit {
            self.commit_edit_mesh_to_scene();
        }

        let (old_mesh, new_mesh, obj_name, vert_count, tri_count) = {
            let Some(obj) = self.scene.get_object_mut(active_id) else {
                return;
            };

            let old_mesh = std::sync::Arc::new(obj.mesh.clone());
            let remeshed = forge_geometry::voxel_remesh(&obj.mesh, voxel_size);
            let new_mesh = std::sync::Arc::new(remeshed.clone());

            obj.set_mesh(remeshed);
            (
                old_mesh,
                new_mesh.clone(),
                obj.name.clone(),
                obj.mesh.vertices.len(),
                obj.mesh.indices.len() / 3,
            )
        };

        self.scene.dirty = true;

        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&active_id);
        }

        // If in Edit or Sculpt Mode, update active_edit_mesh
        if self.editor_state.edit_mode == EditMode::Edit || self.editor_state.edit_mode == EditMode::Sculpt {
            let edit_mesh = forge_geometry::EditableMesh::from_mesh(&new_mesh);
            self.active_edit_mesh = Some((active_id, edit_mesh));
            self.edit_overlay_cache.reset();
        }

        self.history.push(Action::ModifyMesh {
            id: active_id,
            old_mesh,
            new_mesh,
            name: format!("Voxel Remesh ({:.2})", voxel_size),
        });

        self.status_message = format!(
            "Voxel Remeshed '{}' with size {:.2} ({} vertices, {} triangles)",
            obj_name,
            voxel_size,
            vert_count,
            tri_count
        );
    }

    /// Appends a new modifier to the given object's modifier stack.
    pub fn add_modifier(&mut self, id: ObjectId, modifier: ModifierType) {
        let mod_name = modifier.name().to_string();
        let obj_name = {
            let Some(obj) = self.scene.get_object_mut(id) else {
                return;
            };
            obj.modifiers.add(modifier);
            obj.invalidate_evaluated_mesh();
            obj.name.clone()
        };
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }
        self.status_message = format!("Added modifier '{}' to {}", mod_name, obj_name);
    }

    /// Removes a modifier from the given object's modifier stack at `index`.
    pub fn remove_modifier(&mut self, id: ObjectId, index: usize) {
        let removed_name = {
            let Some(obj) = self.scene.get_object_mut(id) else {
                return;
            };
            let Some(removed) = obj.modifiers.remove(index) else {
                return;
            };
            obj.invalidate_evaluated_mesh();
            removed.name
        };
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }
        self.status_message = format!("Removed modifier '{}'", removed_name);
    }

    /// Moves the modifier at `index` up in the stack.
    pub fn move_modifier_up(&mut self, id: ObjectId, index: usize) {
        if let Some(obj) = self.scene.get_object_mut(id) {
            obj.modifiers.move_up(index);
            obj.invalidate_evaluated_mesh();
        }
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }
    }

    /// Moves the modifier at `index` down in the stack.
    pub fn move_modifier_down(&mut self, id: ObjectId, index: usize) {
        if let Some(obj) = self.scene.get_object_mut(id) {
            obj.modifiers.move_down(index);
            obj.invalidate_evaluated_mesh();
        }
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }
    }

    /// Duplicates the modifier at `index` on the given object.
    pub fn duplicate_modifier(&mut self, id: ObjectId, index: usize) {
        let obj_name = {
            let Some(obj) = self.scene.get_object_mut(id) else {
                return;
            };
            if !obj.modifiers.duplicate(index) {
                return;
            }
            obj.invalidate_evaluated_mesh();
            obj.name.clone()
        };
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }
        self.status_message = format!("Duplicated modifier on {}", obj_name);
    }

    /// Bakes the modifier at `index` into the base mesh and removes it from the stack.
    /// Pushes an undo step (`Action::ModifyMesh`) for full rollback.
    pub fn apply_modifier(&mut self, id: ObjectId, index: usize) -> bool {
        if self.editor_state.edit_mode == EditMode::Edit {
            self.status_message = "Cannot apply modifier in Edit Mode. Switch to Object Mode first.".into();
            return false;
        }

        let Some(obj) = self.scene.get_object_mut(id) else {
            return false;
        };

        if index >= obj.modifiers.len() {
            return false;
        }

        let mod_name = obj.modifiers.modifiers[index].name.clone();
        let old_mesh = std::sync::Arc::new(obj.mesh.clone());
        obj.modifiers.apply_to_mesh(index, &mut obj.mesh);
        let new_mesh = std::sync::Arc::new(obj.mesh.clone());
        let obj_name = obj.name.clone();

        obj.invalidate_evaluated_mesh();
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }

        self.history.push(Action::ModifyMesh {
            id,
            old_mesh,
            new_mesh,
            name: format!("Apply Modifier: {}", mod_name),
        });

        self.status_message = format!("Applied modifier '{}' on {}", mod_name, obj_name);
        true
    }

    /// Evaluates and bakes all modifiers into base mesh, then clears the stack.
    /// Pushes an undo step (`Action::ModifyMesh`).
    pub fn apply_all_modifiers(&mut self, id: ObjectId) -> bool {
        if self.editor_state.edit_mode == EditMode::Edit {
            self.status_message = "Cannot apply modifiers in Edit Mode. Switch to Object Mode first.".into();
            return false;
        }

        let Some(obj) = self.scene.get_object_mut(id) else {
            return false;
        };

        if obj.modifiers.is_empty() {
            return false;
        }

        let old_mesh = std::sync::Arc::new(obj.mesh.clone());
        obj.modifiers.apply_all(&mut obj.mesh);
        let new_mesh = std::sync::Arc::new(obj.mesh.clone());
        let obj_name = obj.name.clone();

        obj.invalidate_evaluated_mesh();
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }

        self.history.push(Action::ModifyMesh {
            id,
            old_mesh,
            new_mesh,
            name: "Apply All Modifiers".to_string(),
        });

        self.status_message = format!("Applied all modifiers on {}", obj_name);
        true
    }

    /// Clears all modifiers from the given object's stack.
    pub fn clear_all_modifiers(&mut self, id: ObjectId) {
        let obj_name = {
            let Some(obj) = self.scene.get_object_mut(id) else {
                return;
            };
            obj.modifiers.clear();
            obj.invalidate_evaluated_mesh();
            obj.name.clone()
        };
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_mesh(&id);
        }
        self.status_message = format!("Cleared all modifiers on {}", obj_name);
    }

    /// Fast sync of edit mesh to scene object during interactive operations without pushing undo step.
    pub fn sync_edit_mesh_to_scene(&mut self) {
        if let Some((id, edit_mesh)) = &self.active_edit_mesh {
            let obj_id = *id;
            if let Some(obj) = self.scene.get_object_mut(obj_id) {
                obj.set_mesh(edit_mesh.to_render_mesh());
                self.scene.dirty = true;
            }
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&obj_id);
            }
            self.edit_overlay_cache.reset();
        }
    }

    pub fn extrude_selected_geometry(&mut self, distance: f32) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            let res = edit_mesh.extrude_faces(distance);
            if !res.is_empty() {
                let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
                self.sync_edit_mesh_to_scene();
                self.history.push(Action::modify_mesh(
                    obj_id,
                    old_mesh,
                    new_mesh,
                    format!("Extrude {} face(s)", res.len()),
                ));
                self.status_message = format!("Extruded {} face(s)", res.len());
            }
        }
    }

    pub fn inset_selected_geometry(&mut self, amount: f32) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            let res = edit_mesh.inset_faces(amount);
            if !res.is_empty() {
                let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
                self.sync_edit_mesh_to_scene();
                self.history.push(Action::modify_mesh(
                    obj_id,
                    old_mesh,
                    new_mesh,
                    format!("Inset {} face(s)", res.len()),
                ));
                self.status_message = format!("Inset {} face(s)", res.len());
            }
        }
    }

    pub fn subdivide_selected_geometry(&mut self) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            edit_mesh.subdivide();
            let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            self.sync_edit_mesh_to_scene();
            self.history.push(Action::modify_mesh(
                obj_id,
                old_mesh,
                new_mesh,
                "Subdivide Faces".to_string(),
            ));
            self.status_message = "Subdivided selected faces".to_string();
        }
    }

    pub fn merge_selected_geometry(&mut self) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            edit_mesh.merge_selected_at_center();
            let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            self.sync_edit_mesh_to_scene();
            self.history.push(Action::modify_mesh(
                obj_id,
                old_mesh,
                new_mesh,
                "Merge Selected Vertices".to_string(),
            ));
            self.status_message = "Merged selected vertices at center".to_string();
        }
    }

    pub fn fill_selected_geometry(&mut self) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            if let Some(idx) = edit_mesh.fill_face_from_selection() {
                let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
                self.sync_edit_mesh_to_scene();
                self.history.push(Action::modify_mesh(
                    obj_id,
                    old_mesh,
                    new_mesh,
                    format!("Create Face #{}", idx),
                ));
                self.status_message = format!("Created Face #{}", idx);
            }
        }
    }

    pub fn bevel_selected_geometry(&mut self, width: f32) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            match edit_mesh.bevel_selected_edges(width) {
                Ok(faces) if !faces.is_empty() => {
                    let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
                    self.sync_edit_mesh_to_scene();
                    self.history.push(Action::modify_mesh(
                        obj_id,
                        old_mesh,
                        new_mesh,
                        format!("Bevel {} edge(s)", faces.len()),
                    ));
                    self.status_message = format!("Beveled {} edge(s)", faces.len());
                }
                Ok(_) => {
                    self.status_message = "No eligible edges for bevel (select edges)".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Bevel: {e}");
                }
            }
        }
    }

    pub fn loop_cut_selected_geometry(&mut self, factor: f32) {
        if let Some((obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            let obj_id = *obj_id;
            let target_edge_idx = edit_mesh.edges.iter().position(|e| e.selected).unwrap_or(0);
            let old_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            match edit_mesh.loop_cut(target_edge_idx, factor) {
                Ok(faces) if !faces.is_empty() => {
                    let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
                    self.sync_edit_mesh_to_scene();
                    self.history.push(Action::modify_mesh(
                        obj_id,
                        old_mesh,
                        new_mesh,
                        format!("Loop Cut {} face(s)", faces.len()),
                    ));
                    self.status_message = format!("Loop Cut: created {} new face(s)", faces.len());
                }
                Ok(_) => {
                    self.status_message = "Loop Cut completed".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Loop Cut: {e}");
                }
            }
        }
    }

    /// Begins interactive modal execution for an Edit Mode modeling tool (Extrude, Inset, Loop Cut).
    pub fn begin_interactive_edit_tool(
        &mut self,
        tool: InteractiveEditToolType,
        mouse_pos: Vec2,
    ) -> bool {
        let Some((_obj_id, edit_mesh)) = &self.active_edit_mesh else {
            self.status_message = "No active mesh in Edit Mode".to_string();
            return false;
        };

        match tool {
            InteractiveEditToolType::Extrude => {
                if edit_mesh.selected_faces().is_empty() {
                    self.status_message =
                        "Select at least one face to extrude (Press 3 for Face selection)"
                            .to_string();
                    return false;
                }
            }
            InteractiveEditToolType::Inset => {
                if edit_mesh.selected_faces().is_empty() {
                    self.status_message =
                        "Select at least one face to inset (Press 3 for Face selection)".to_string();
                    return false;
                }
            }
            InteractiveEditToolType::LoopCut => {
                if edit_mesh.edges.is_empty() {
                    self.status_message = "No edges available for Loop Cut".to_string();
                    return false;
                }
            }
        }

        let initial_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
        let initial_edit_mesh = edit_mesh.clone();
        let target_edge_idx = edit_mesh.edges.iter().position(|e| e.selected).unwrap_or(0);

        let initial_val = match tool {
            InteractiveEditToolType::Extrude => 0.0,
            InteractiveEditToolType::Inset => 0.05,
            InteractiveEditToolType::LoopCut => 0.5,
        };

        self.interactive_edit_tool = Some(InteractiveEditToolState {
            tool_type: tool,
            start_pointer_pos: mouse_pos,
            initial_mesh,
            initial_edit_mesh,
            target_edge_idx,
            current_value: initial_val,
        });

        self.status_message = match tool {
            InteractiveEditToolType::Extrude => {
                "Interactive Extrude: Move mouse to extrude | LMB: Confirm | Esc: Cancel".to_string()
            }
            InteractiveEditToolType::Inset => {
                "Interactive Inset: Move mouse to inset | LMB: Confirm | Esc: Cancel".to_string()
            }
            InteractiveEditToolType::LoopCut => {
                "Interactive Loop Cut: Slide mouse to position | LMB: Confirm | Esc: Cancel"
                    .to_string()
            }
        };

        true
    }

    /// Updates the active interactive modeling tool with current mouse coordinates and modifier keys.
    pub fn update_interactive_edit_tool(
        &mut self,
        curr_pos: Vec2,
        shift_held: bool,
        ctrl_held: bool,
    ) {
        let Some(tool_state) = self.interactive_edit_tool.as_mut() else {
            return;
        };

        let start_pos = tool_state.start_pointer_pos;
        let delta_y = start_pos.y - curr_pos.y;
        let delta_x = curr_pos.x - start_pos.x;

        match tool_state.tool_type {
            InteractiveEditToolType::Extrude => {
                let rate = if shift_held { 0.002 } else { 0.01 };
                let mut val = delta_y * rate;
                if ctrl_held {
                    val = (val * 10.0).round() / 10.0;
                }
                tool_state.current_value = val;
            }
            InteractiveEditToolType::Inset => {
                let rate = if shift_held { 0.001 } else { 0.005 };
                let mut val = (delta_y * rate).clamp(0.001, 0.95);
                if ctrl_held {
                    val = (val * 20.0).round() / 20.0;
                }
                tool_state.current_value = val;
            }
            InteractiveEditToolType::LoopCut => {
                let rate = if shift_held { 0.0006 } else { 0.003 };
                let mut val = (0.5 + delta_x * rate).clamp(0.02, 0.98);
                if ctrl_held {
                    val = (val * 10.0).round() / 10.0;
                }
                tool_state.current_value = val;
            }
        }

        let val = tool_state.current_value;
        let tool_type = tool_state.tool_type;
        let target_edge = tool_state.target_edge_idx;
        let cloned_init = tool_state.initial_edit_mesh.clone();

        if let Some((_obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            *edit_mesh = cloned_init;
            match tool_type {
                InteractiveEditToolType::Extrude => {
                    edit_mesh.extrude_faces(val);
                }
                InteractiveEditToolType::Inset => {
                    edit_mesh.inset_faces(val);
                }
                InteractiveEditToolType::LoopCut => {
                    let _ = edit_mesh.loop_cut(target_edge, val);
                }
            }
            self.sync_edit_mesh_to_scene();
        }
    }

    /// Confirms the active interactive tool, applying the transformation and pushing it to the undo stack.
    pub fn confirm_interactive_edit_tool(&mut self) {
        let Some(tool_state) = self.interactive_edit_tool.take() else {
            return;
        };

        if let Some((obj_id, edit_mesh)) = &self.active_edit_mesh {
            let obj_id = *obj_id;
            let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
            let name = match tool_state.tool_type {
                InteractiveEditToolType::Extrude => {
                    format!("Extrude {:+.2} m", tool_state.current_value)
                }
                InteractiveEditToolType::Inset => {
                    format!("Inset {:.2} m", tool_state.current_value)
                }
                InteractiveEditToolType::LoopCut => {
                    format!("Loop Cut {:.0}%", tool_state.current_value * 100.0)
                }
            };

            self.history.push(Action::modify_mesh(
                obj_id,
                tool_state.initial_mesh,
                new_mesh,
                name.clone(),
            ));
            self.status_message = format!("Confirmed {name}");
        }
    }

    /// Cancels the active interactive tool, reverting mesh geometry to its state prior to starting the tool.
    pub fn cancel_interactive_edit_tool(&mut self) {
        let Some(tool_state) = self.interactive_edit_tool.take() else {
            return;
        };

        if let Some((_obj_id, edit_mesh)) = &mut self.active_edit_mesh {
            *edit_mesh = tool_state.initial_edit_mesh;
            self.sync_edit_mesh_to_scene();
            self.status_message = format!("{:?} cancelled", tool_state.tool_type);
        }
    }

    pub fn dissolve_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            match edit_mesh.dissolve_selected_edges() {
                Ok(count) if count > 0 => {
                    self.sync_edit_mesh_to_scene();
                    self.status_message = format!("Dissolved {} edge(s)", count);
                }
                Ok(_) => {
                    self.status_message =
                        "No internal manifold edges selected to dissolve".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Dissolve: {e}");
                }
            }
        }
    }

    pub fn bridge_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            match edit_mesh.bridge_edge_loops() {
                Ok(count) if count > 0 => {
                    self.sync_edit_mesh_to_scene();
                    self.status_message = format!("Bridged edge loops: created {} face(s)", count);
                }
                Ok(_) => {
                    self.status_message = "No edge loops bridged".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Bridge: {e}");
                }
            }
        }
    }

    pub fn separate_selected_geometry(&mut self) {
        if let Some((active_id, edit_mesh)) = &mut self.active_edit_mesh {
            if let Some(mut separated_mesh) = edit_mesh.separate_selected() {
                separated_mesh.recalculate_normals();
                let render_mesh = separated_mesh.to_render_mesh();
                let parent_tf = self
                    .scene
                    .get_object(*active_id)
                    .map(|o| o.local_transform)
                    .unwrap_or(Transform::IDENTITY);
                let mut new_obj =
                    SceneObject::new("Separated_Part", PrimitiveType::Custom, render_mesh);
                new_obj.local_transform = parent_tf;
                let new_id = self.scene.add_object(new_obj.clone());
                self.history.push(Action::AddObject { object: new_obj });
                self.sync_edit_mesh_to_scene();
                self.status_message = format!("Separated selection into new object {:?}", new_id);
            } else {
                self.status_message = "No faces selected to separate".to_string();
            }
        }
    }

    pub fn add_primitive(&mut self, prim: PrimitiveType) {
        let cursor = self.editor_state.cursor_3d;
        let (name, color, mesh, tf) = match prim {
            PrimitiveType::Cube => (
                "Cube",
                [0.85, 0.45, 0.25, 1.0],
                forge_geometry::primitives::generate_cube(1.0),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.5, 0.0)),
            ),
            PrimitiveType::Sphere => (
                "UV Sphere",
                [0.25, 0.65, 0.85, 1.0],
                forge_geometry::primitives::generate_uv_sphere(0.75, 32, 16),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.75, 0.0)),
            ),
            PrimitiveType::Icosphere => (
                "Icosphere",
                [0.45, 0.75, 0.45, 1.0],
                forge_geometry::primitives::generate_icosphere(0.75, 2),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.75, 0.0)),
            ),
            PrimitiveType::Cylinder => (
                "Cylinder",
                [0.85, 0.75, 0.25, 1.0],
                forge_geometry::primitives::generate_cylinder(0.5, 1.5, 32),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.75, 0.0)),
            ),
            PrimitiveType::Cone => (
                "Cone",
                [0.75, 0.35, 0.65, 1.0],
                forge_geometry::primitives::generate_cone(0.75, 1.5, 32),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.75, 0.0)),
            ),
            PrimitiveType::Torus => (
                "Torus",
                [0.85, 0.55, 0.45, 1.0],
                forge_geometry::primitives::generate_torus(0.75, 0.25, 32, 16),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.25, 0.0)),
            ),
            PrimitiveType::Plane => (
                "Plane",
                [0.72, 0.74, 0.78, 1.0],
                forge_geometry::primitives::generate_plane(8.0, 8),
                Transform::from_translation(cursor),
            ),
            PrimitiveType::Monkey => (
                "Suzanne",
                [0.75, 0.55, 0.25, 1.0],
                forge_geometry::primitives::generate_monkey(1.0),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.75, 0.0)),
            ),
            PrimitiveType::Capsule => (
                "Capsule",
                [0.45, 0.65, 0.65, 1.0],
                forge_geometry::primitives::generate_capsule(0.4, 1.0, 16, 8),
                Transform::from_translation(cursor + Vec3::new(0.0, 0.9, 0.0)),
            ),
            PrimitiveType::Circle => (
                "Circle",
                [0.5, 0.7, 0.8, 1.0],
                forge_geometry::primitives::generate_circle(1.0, 32),
                Transform::from_translation(cursor),
            ),
            PrimitiveType::Grid => (
                "Grid",
                [0.3, 0.35, 0.4, 1.0],
                forge_geometry::primitives::generate_plane(10.0, 10),
                Transform::from_translation(cursor),
            ),
            PrimitiveType::Camera => (
                "Camera",
                [0.3, 0.7, 1.0, 1.0],
                forge_geometry::primitives::generate_camera_marker(),
                Transform::from_translation(cursor + Vec3::new(0.0, 1.5, 4.0)),
            ),
            PrimitiveType::Light => (
                "Light",
                [1.0, 0.95, 0.8, 1.0],
                forge_geometry::primitives::generate_icosphere(0.2, 1),
                Transform::from_translation(cursor + Vec3::new(0.0, 2.0, 0.0)),
            ),
            PrimitiveType::Custom => (
                "Custom",
                [0.7, 0.7, 0.7, 1.0],
                forge_geometry::primitives::generate_cube(1.0),
                Transform::from_translation(cursor),
            ),
        };

        let id = self.scene.add_object(
            SceneObject::new(name, prim, mesh)
                .with_color(color)
                .with_transform(tf),
        );
        if let Some(obj) = self.scene.get_object(id) {
            self.history.push(Action::AddObject {
                object: obj.clone(),
            });
        }
        self.selection.select(id);
        self.status_message = format!("Added {}", name);
    }

    pub fn add_point_light(&mut self) {
        let cursor = self.editor_state.cursor_3d;
        let mut obj = SceneObject::new_light(
            "Point Light",
            forge_scene::LightData {
                kind: forge_scene::LightKind::Point,
                enabled: true,
                color: [1.0, 0.95, 0.8],
                intensity: 80.0,
                range: 15.0,
                inner_cone_angle_deg: 25.0,
                outer_cone_angle_deg: 35.0,
            },
            forge_geometry::primitives::generate_icosphere(0.2, 1),
        );
        obj.color = [1.0, 0.95, 0.8, 1.0];
        obj.local_transform = Transform::from_translation(cursor + Vec3::new(0.0, 2.0, 0.0));
        let id = self.scene.add_object(obj.clone());
        self.history.push(Action::AddObject { object: obj });
        self.selection.select(id);
        self.status_message = "Added Point Light".to_string();
    }

    pub fn add_directional_light(&mut self) {
        let cursor = self.editor_state.cursor_3d;
        let mut obj = SceneObject::new_light(
            "Directional Light",
            forge_scene::LightData {
                kind: forge_scene::LightKind::Directional,
                enabled: true,
                color: [1.0, 1.0, 0.95],
                intensity: 3.0,
                range: 50.0,
                inner_cone_angle_deg: 25.0,
                outer_cone_angle_deg: 35.0,
            },
            forge_geometry::primitives::generate_cone(0.2, 0.5, 8),
        );
        obj.color = [1.0, 1.0, 0.9, 1.0];
        obj.local_transform = Transform::from_translation(cursor + Vec3::new(2.0, 4.0, 2.0));
        let id = self.scene.add_object(obj.clone());
        self.history.push(Action::AddObject { object: obj });
        self.selection.select(id);
        self.status_message = "Added Directional Light (Sun)".to_string();
    }

    pub fn add_camera(&mut self) {
        let cursor = self.editor_state.cursor_3d;
        let mut obj = SceneObject::new_camera("Camera");
        obj.local_transform = Transform::from_translation(cursor + Vec3::new(0.0, 1.5, 4.0));
        let id = self.scene.add_object(obj.clone());
        self.history.push(Action::AddObject { object: obj });
        self.selection.select(id);
        self.status_message = "Added Camera".to_string();
    }

    pub fn tris_to_quads_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            edit_mesh.tris_to_quads();
            self.sync_edit_mesh_to_scene();
            self.status_message = "Converted coplanar triangles to quads".to_string();
        }
    }

    pub fn weld_colocated_selected_geometry(&mut self, tolerance: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            edit_mesh.weld_colocated_vertices(tolerance);
            self.sync_edit_mesh_to_scene();
            self.status_message = "Welded co-located vertices".to_string();
        }
    }

    pub fn bisect_selected_geometry(
        &mut self,
        plane_point: Vec3,
        plane_normal: Vec3,
        clear_outer: bool,
        fill_cut: bool,
    ) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            match edit_mesh.bisect(plane_point, plane_normal, clear_outer, fill_cut) {
                Ok(face_count) => {
                    self.sync_edit_mesh_to_scene();
                    self.status_message = format!("Bisect complete: {} face(s)", face_count);
                }
                Err(e) => {
                    self.status_message = format!("Bisect error: {e}");
                }
            }
        }
    }

    pub fn knife_connect_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            let selected_verts = edit_mesh.selected_vertices();
            if selected_verts.len() >= 2 {
                match edit_mesh.knife_connect_vertices(selected_verts[0], selected_verts[1]) {
                    Ok(face_idx) => {
                        self.sync_edit_mesh_to_scene();
                        self.status_message = format!("Knife cut face {}", face_idx);
                    }
                    Err(e) => {
                        self.status_message = format!("Knife connect error: {e}");
                    }
                }
            } else {
                self.status_message = "Select 2 vertices across a face to knife cut".to_string();
            }
        }
    }

    pub fn spin_selected_geometry(&mut self, steps: usize, angle_radians: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            match edit_mesh.spin_selected(Vec3::Y, Vec3::ZERO, angle_radians, steps, 0.0) {
                Ok(faces) if !faces.is_empty() => {
                    self.sync_edit_mesh_to_scene();
                    self.status_message = format!("Spin complete: created {} faces", faces.len());
                }
                Ok(_) => {
                    self.status_message = "No faces created by spin".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Spin error: {e}");
                }
            }
        }
    }

    pub fn smart_project_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            forge_geometry::smart_project_uvs(
                edit_mesh,
                &forge_geometry::SmartProjectOptions::default(),
            );
            self.sync_edit_mesh_to_scene();
            self.status_message = "Smart UV Projection complete (packed into [0, 1])".to_string();
        } else if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object_mut(active_id)
        {
            let mut edit_mesh = forge_geometry::EditableMesh::from_mesh(&obj.mesh);
            forge_geometry::smart_project_uvs(
                &mut edit_mesh,
                &forge_geometry::SmartProjectOptions::default(),
            );
            obj.mesh = edit_mesh.to_render_mesh();
            obj.invalidate_evaluated_mesh();
            self.status_message = format!("Smart UV Projection applied to {}", obj.name);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&active_id);
            }
        }
    }

    pub fn box_project_selected_geometry(&mut self, scale: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            forge_geometry::box_project_uvs(edit_mesh, scale);
            self.sync_edit_mesh_to_scene();
            self.status_message = "Box UV Projection complete".to_string();
        } else if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object_mut(active_id)
        {
            let mut edit_mesh = forge_geometry::EditableMesh::from_mesh(&obj.mesh);
            forge_geometry::box_project_uvs(&mut edit_mesh, scale);
            obj.mesh = edit_mesh.to_render_mesh();
            obj.invalidate_evaluated_mesh();
            self.status_message = format!("Box UV Projection applied to {}", obj.name);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&active_id);
            }
        }
    }

    pub fn sphere_project_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            forge_geometry::sphere_project_uvs(edit_mesh, Vec3::ZERO);
            self.sync_edit_mesh_to_scene();
            self.status_message = "Sphere UV Projection complete".to_string();
        } else if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object_mut(active_id)
        {
            let mut edit_mesh = forge_geometry::EditableMesh::from_mesh(&obj.mesh);
            forge_geometry::sphere_project_uvs(&mut edit_mesh, Vec3::ZERO);
            obj.mesh = edit_mesh.to_render_mesh();
            obj.invalidate_evaluated_mesh();
            self.status_message = format!("Sphere UV Projection applied to {}", obj.name);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&active_id);
            }
        }
    }

    pub fn cylinder_project_selected_geometry(&mut self, height: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            forge_geometry::cylinder_project_uvs(edit_mesh, Vec3::ZERO, height);
            self.sync_edit_mesh_to_scene();
            self.status_message = "Cylinder UV Projection complete".to_string();
        } else if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object_mut(active_id)
        {
            let mut edit_mesh = forge_geometry::EditableMesh::from_mesh(&obj.mesh);
            forge_geometry::cylinder_project_uvs(&mut edit_mesh, Vec3::ZERO, height);
            obj.mesh = edit_mesh.to_render_mesh();
            obj.invalidate_evaluated_mesh();
            self.status_message = format!("Cylinder UV Projection applied to {}", obj.name);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&active_id);
            }
        }
    }

    pub fn pack_uv_islands_selected_geometry(&mut self, margin: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            forge_geometry::pack_uv_islands(edit_mesh, margin);
            self.sync_edit_mesh_to_scene();
            self.status_message = "Packed UV Islands".to_string();
        } else if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object_mut(active_id)
        {
            let mut edit_mesh = forge_geometry::EditableMesh::from_mesh(&obj.mesh);
            forge_geometry::pack_uv_islands(&mut edit_mesh, margin);
            obj.mesh = edit_mesh.to_render_mesh();
            obj.invalidate_evaluated_mesh();
            self.status_message = format!("Packed UV Islands on {}", obj.name);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&active_id);
            }
        }
    }

    pub fn delete_selected(&mut self) {
        match self.editor_state.edit_mode {
            EditMode::Object => {
                let to_delete: Vec<ObjectId> =
                    self.selection.selected_ids.iter().copied().collect();
                for id in to_delete {
                    if let Some(removed) = self.scene.remove_object(id) {
                        if let Some(r) = &mut self.renderer {
                            r.invalidate_mesh(&id);
                        }
                        self.history.push(Action::DeleteObject { object: removed });
                    }
                }
                self.selection.deselect();
                self.pending_hover_pick = None;
                if let Some(r) = &mut self.renderer {
                    r.invalidate_picking();
                }
                self.status_message = "Deleted selected object(s)".to_string();
            }
            EditMode::Edit | EditMode::Sculpt => {
                if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
                    let sel_mode = match self.editor_state.selection_element {
                        forge_scene::SelectionElement::Vertex => EditSelectionMode::Vertex,
                        forge_scene::SelectionElement::Edge => EditSelectionMode::Edge,
                        forge_scene::SelectionElement::Face => EditSelectionMode::Face,
                    };
                    edit_mesh.delete_selected(sel_mode);
                    self.sync_edit_mesh_to_scene();
                    self.pending_hover_pick = None;
                    if let Some(r) = &mut self.renderer {
                        r.invalidate_picking();
                    }
                    self.status_message = "Deleted selected elements".to_string();
                }
            }
        }
    }

    pub fn save_to_string(&self) -> forge_core::error::Result<String> {
        let project = forge_scene::ForgeProject::new("Forge Project", self.scene.clone());
        project.to_json()
    }

    pub fn load_from_string(&mut self, s: &str) -> forge_core::error::Result<()> {
        let project = forge_scene::ForgeProject::from_json(s)?;
        self.scene = project.scene;
        self.selection.deselect();
        self.scene.dirty = true;
        self.status_message = format!("Loaded project: {}", project.name);
        Ok(())
    }

    pub fn import_obj_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();
        let options = forge_io::ImportOptions::default();
        let asset = match forge_io::import_obj(path_ref, &options) {
            Ok(a) => a,
            Err(e) => {
                let msg = format!("Import failed: {e}");
                self.status_message = msg.clone();
                return Err(msg);
            }
        };

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.obj");

        let total_objs = asset.objects.len();
        let total_tris = asset.total_triangles();

        let scene_objects = asset.into_scene_objects();
        if scene_objects.is_empty() {
            let msg = "Import failed: No objects imported".to_string();
            self.status_message = msg.clone();
            return Err(msg);
        }

        let mut imported_ids = std::collections::HashSet::new();
        let mut first_id = None;

        for obj in &scene_objects {
            imported_ids.insert(obj.id);
            if first_id.is_none() {
                first_id = Some(obj.id);
            }
        }

        let history_objects = scene_objects.clone();
        for obj in scene_objects {
            self.scene.add_object(obj);
        }

        self.history.push(Action::ImportObjects {
            objects: history_objects,
        });

        self.selection.selected_ids = imported_ids;
        self.selection.active_id = first_id;
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_all_meshes();
        }

        self.status_message = format!(
            "Imported {} ({} object{}, {} tris)",
            file_name,
            total_objs,
            if total_objs == 1 { "" } else { "s" },
            total_tris
        );

        self.editor_state.panel_settings.add_recent_project(
            path_ref.to_string_lossy().to_string(),
            file_name.to_string(),
        );

        Ok(())
    }

    pub fn export_obj_to_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
        selected_only: bool,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();

        let export_scene = if selected_only {
            if self.selection.selected_ids.is_empty() {
                let msg = "Export error: No objects selected to export".to_string();
                self.status_message = msg.clone();
                return Err(msg);
            }
            let mut sub_scene = Scene::new("SelectedExport");
            for &id in &self.selection.selected_ids {
                if let Some(obj) = self.scene.get_object(id) {
                    sub_scene.add_object(obj.clone());
                }
            }
            sub_scene
        } else {
            self.scene.clone()
        };

        let options = forge_io::ExportOptions {
            world_space: true,
            ..Default::default()
        };

        let mut file = match std::fs::File::create(path_ref) {
            Ok(f) => f,
            Err(e) => {
                let msg = format!("Export error: Failed to create file: {e}");
                self.status_message = msg.clone();
                return Err(msg);
            }
        };

        if let Err(e) = forge_io::export_scene_obj(&export_scene, &options, &mut file) {
            let msg = format!("Export error: {e}");
            self.status_message = msg.clone();
            return Err(msg);
        }

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("export.obj");

        self.status_message = format!("Exported Wavefront OBJ to {file_name}");
        Ok(())
    }

    pub fn import_stl_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();
        let options = forge_io::StlImportOptions::default();
        let asset = match forge_io::import_stl(path_ref, &options) {
            Ok(a) => a,
            Err(e) => {
                let msg = format!("STL Import failed: {e}");
                self.status_message = msg.clone();
                return Err(msg);
            }
        };

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.stl");

        let total_objs = asset.objects.len();
        let total_tris = asset.total_triangles();

        let scene_objects = asset.into_scene_objects();
        if scene_objects.is_empty() {
            let msg = "STL Import failed: No objects imported".to_string();
            self.status_message = msg.clone();
            return Err(msg);
        }

        let mut imported_ids = std::collections::HashSet::new();
        let mut first_id = None;

        for obj in &scene_objects {
            imported_ids.insert(obj.id);
            if first_id.is_none() {
                first_id = Some(obj.id);
            }
        }

        let history_objects = scene_objects.clone();
        for obj in scene_objects {
            self.scene.add_object(obj);
        }

        self.history.push(Action::ImportObjects {
            objects: history_objects,
        });

        self.selection.selected_ids = imported_ids;
        self.selection.active_id = first_id;
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_all_meshes();
        }

        self.status_message = format!(
            "Imported STL {} ({} object{}, {} tris)",
            file_name,
            total_objs,
            if total_objs == 1 { "" } else { "s" },
            total_tris
        );

        Ok(())
    }

    pub fn export_stl_to_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
        selected_only: bool,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();

        let export_scene = if selected_only {
            if self.selection.selected_ids.is_empty() {
                let msg = "Export error: No objects selected to export".to_string();
                self.status_message = msg.clone();
                return Err(msg);
            }
            let mut sub_scene = Scene::new("SelectedExport");
            for &id in &self.selection.selected_ids {
                if let Some(obj) = self.scene.get_object(id) {
                    sub_scene.add_object(obj.clone());
                }
            }
            sub_scene
        } else {
            self.scene.clone()
        };

        let solid_name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("forge3d_export")
            .to_string();

        let options = forge_io::StlExportOptions {
            format: forge_io::StlFormat::Binary,
            apply_world_transform: true,
            solid_name,
            visible_only: true,
        };

        if let Err(e) = forge_io::export_scene_stl_to_file(&export_scene, path_ref, &options) {
            let msg = format!("STL Export error: {e}");
            self.status_message = msg.clone();
            return Err(msg);
        }

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("export.stl");

        self.status_message = format!("Exported STL to {file_name}");
        Ok(())
    }

    pub fn import_ply_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();
        let options = forge_io::PlyImportOptions::default();
        let asset = match forge_io::import_ply(path_ref, &options) {
            Ok(a) => a,
            Err(e) => {
                let msg = format!("PLY Import failed: {e}");
                self.status_message = msg.clone();
                return Err(msg);
            }
        };

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.ply");

        let total_objs = asset.objects.len();
        let total_tris = asset.total_triangles();

        let scene_objects = asset.into_scene_objects();
        if scene_objects.is_empty() {
            let msg = "PLY Import failed: No objects imported".to_string();
            self.status_message = msg.clone();
            return Err(msg);
        }

        let mut imported_ids = std::collections::HashSet::new();
        let mut first_id = None;

        for obj in &scene_objects {
            imported_ids.insert(obj.id);
            if first_id.is_none() {
                first_id = Some(obj.id);
            }
        }

        let history_objects = scene_objects.clone();
        for obj in scene_objects {
            self.scene.add_object(obj);
        }

        self.history.push(Action::ImportObjects {
            objects: history_objects,
        });

        self.selection.selected_ids = imported_ids;
        self.selection.active_id = first_id;
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_all_meshes();
        }

        self.status_message = format!(
            "Imported PLY {} ({} object{}, {} tris)",
            file_name,
            total_objs,
            if total_objs == 1 { "" } else { "s" },
            total_tris
        );

        Ok(())
    }

    pub fn export_ply_to_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
        selected_only: bool,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();

        let export_scene = if selected_only {
            if self.selection.selected_ids.is_empty() {
                let msg = "Export error: No objects selected to export".to_string();
                self.status_message = msg.clone();
                return Err(msg);
            }
            let mut sub_scene = Scene::new("SelectedExport");
            for &id in &self.selection.selected_ids {
                if let Some(obj) = self.scene.get_object(id) {
                    sub_scene.add_object(obj.clone());
                }
            }
            sub_scene
        } else {
            self.scene.clone()
        };

        let options = forge_io::PlyExportOptions {
            format: forge_io::PlyFormat::BinaryLittleEndian,
            apply_world_transform: true,
            export_normals: true,
            export_colors: true,
            export_uvs: true,
            visible_only: true,
            comments: vec!["Exported from Forge3D Rust Edition".to_string()],
        };

        if let Err(e) = forge_io::export_scene_ply_to_file(&export_scene, path_ref, &options) {
            let msg = format!("PLY Export error: {e}");
            self.status_message = msg.clone();
            return Err(msg);
        }

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("export.ply");

        self.status_message = format!("Exported PLY to {file_name}");
        Ok(())
    }

    pub fn import_gltf_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();
        let options = forge_io::GltfImportOptions::default();
        let result = match forge_io::import_gltf(path_ref, &options) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("glTF/GLB Import failed: {e}");
                self.status_message = msg.clone();
                return Err(msg);
            }
        };

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.glb");

        let total_objs = result.objects.len();
        let total_mats = result.materials.len();

        if result.objects.is_empty() {
            let msg = "glTF/GLB Import failed: No objects imported".to_string();
            self.status_message = msg.clone();
            return Err(msg);
        }

        // Add materials to scene
        for mat in result.materials {
            self.scene.materials.insert(mat.name.clone(), mat);
        }

        let mut imported_ids = std::collections::HashSet::new();
        let mut first_id = None;

        for obj in &result.objects {
            imported_ids.insert(obj.id);
            if first_id.is_none() {
                first_id = Some(obj.id);
            }
        }

        let history_objects = result.objects.clone();
        for obj in result.objects {
            self.scene.add_object(obj);
        }

        self.history.push(Action::ImportObjects {
            objects: history_objects,
        });

        self.selection.selected_ids = imported_ids;
        self.selection.active_id = first_id;
        self.scene.dirty = true;
        if let Some(r) = &mut self.renderer {
            r.invalidate_all_meshes();
        }

        self.status_message = format!(
            "Imported glTF/GLB {} ({} object{}, {} material{})",
            file_name,
            total_objs,
            if total_objs == 1 { "" } else { "s" },
            total_mats,
            if total_mats == 1 { "" } else { "s" },
        );

        self.editor_state.panel_settings.add_recent_project(
            path_ref.to_string_lossy().to_string(),
            file_name.to_string(),
        );

        Ok(())
    }

    pub fn export_gltf_to_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
        selected_only: bool,
    ) -> Result<(), String> {
        let path_ref = path.as_ref();

        let ext = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let format = if ext == "gltf" {
            forge_io::GltfExportFormat::GltfEmbedded
        } else {
            forge_io::GltfExportFormat::Glb
        };

        let options = forge_io::GltfExportOptions {
            format,
            export_selected_only: selected_only,
            selected_ids: if selected_only {
                Some(self.selection.selected_ids.clone())
            } else {
                None
            },
            include_normals: true,
            include_uvs: true,
            include_colors: true,
            include_materials: true,
        };

        if let Err(e) = forge_io::export_scene_gltf_to_file(&self.scene, path_ref, &options) {
            let msg = format!("glTF/GLB Export error: {e}");
            self.status_message = msg.clone();
            return Err(msg);
        }

        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("export.glb");

        self.status_message = format!("Exported glTF/GLB to {file_name}");
        Ok(())
    }

    pub fn duplicate_selected(&mut self) {
        if self.editor_state.edit_mode == EditMode::Edit {
            if let Some((_active_id, edit_mesh)) = &mut self.active_edit_mesh {
                let edit_sel_mode = match self.editor_state.selection_element {
                    forge_scene::SelectionElement::Vertex => forge_geometry::EditSelectionMode::Vertex,
                    forge_scene::SelectionElement::Edge => forge_geometry::EditSelectionMode::Edge,
                    forge_scene::SelectionElement::Face => forge_geometry::EditSelectionMode::Face,
                };
                let new_indices = edit_mesh.duplicate_selected(edit_sel_mode);
                if !new_indices.is_empty() {
                    self.sync_edit_mesh_to_scene();
                    self.status_message = format!("Duplicated {} elements in Edit Mode", new_indices.len());
                    return;
                }
            }
        }

        let mut target_ids: Vec<ObjectId> = self.selection.selected_ids.iter().copied().collect();
        if target_ids.is_empty() {
            if let Some(aid) = self.selection.active_id {
                target_ids.push(aid);
            } else {
                self.status_message = "No object selected to duplicate".to_string();
                return;
            }
        }

        let mut new_ids = Vec::new();
        for id in target_ids {
            if let Some(new_id) = self.scene.duplicate_object(id) {
                if let Some(dup) = self.scene.get_object(new_id) {
                    self.history.push(Action::AddObject {
                        object: dup.clone(),
                    });
                }
                new_ids.push(new_id);
            }
        }

        if !new_ids.is_empty() {
            let active = new_ids.last().copied();
            self.selection.selected_ids = new_ids.into_iter().collect();
            self.selection.active_id = active;
            self.scene.dirty = true;
            if let Some(r) = &mut self.renderer {
                r.invalidate_picking();
            }
            self.pending_hover_pick = None;
            self.status_message = format!(
                "Duplicated {} object{} (Shift+D)",
                self.selection.selected_ids.len(),
                if self.selection.selected_ids.len() == 1 { "" } else { "s" }
            );
        }
    }

    pub fn apply_transform_selected(&mut self) {
        if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object(active_id)
        {
            let old_transform = obj.local_transform;
            let old_mesh = obj.mesh.clone();
            self.scene.apply_transform_to_object(active_id);
            self.history
                .push(Action::apply_transform(active_id, old_transform, old_mesh));
            self.status_message = "Applied Transform".to_string();
        }
    }

    pub fn select_all(&mut self) {
        let mut ids = std::collections::HashSet::new();
        for (&id, obj) in &self.scene.objects {
            if obj.visible {
                ids.insert(id);
            }
        }
        self.selection.selected_ids = ids;
        self.selection.active_id = self.selection.selected_ids.iter().next().copied();
        self.status_message = format!(
            "Selected All ({} objects)",
            self.selection.selected_ids.len()
        );
    }

    pub fn deselect_all(&mut self) {
        self.selection.deselect();
        self.status_message = "Selection cleared".to_string();
    }

    pub fn invert_selection(&mut self) {
        let all_ids: Vec<ObjectId> = self.scene.objects.keys().copied().collect();
        let mut new_selection = std::collections::HashSet::new();
        for id in all_ids {
            if let Some(obj) = self.scene.get_object(id)
                && obj.visible
                && !self.selection.is_selected(id)
            {
                new_selection.insert(id);
            }
        }
        self.selection.selected_ids = new_selection;
        self.selection.active_id = self.selection.selected_ids.iter().next().copied();
        self.status_message = format!(
            "Inverted Selection ({} selected)",
            self.selection.selected_ids.len()
        );
    }
}

pub fn point_in_polygon(point: egui::Pos2, polygon: &[egui::Pos2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

impl Forge3dStudioApp {
    pub fn box_select(
        &mut self,
        box_rect: egui::Rect,
        view_proj: forge_math::Mat4,
        viewport_rect: egui::Rect,
        toggle: bool,
    ) {
        if self.editor_state.edit_mode == EditMode::Edit {
            if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
                let aid = self.selection.active_id;
                let model_mat = aid
                    .and_then(|id| self.scene.get_object(id))
                    .map(|obj| obj.local_transform.to_matrix())
                    .unwrap_or(forge_math::Mat4::IDENTITY);

                let mut modified_count = 0;
                match self.editor_state.selection_element {
                    forge_scene::SelectionElement::Vertex => {
                        for v in &mut edit_mesh.vertices {
                            let world_p = model_mat.transform_point3(v.position);
                            if let Some(screen_p) =
                                crate::ui::gizmo::world_to_screen_pos(world_p, view_proj, viewport_rect)
                            {
                                if box_rect.contains(screen_p) {
                                    if toggle {
                                        v.selected = !v.selected;
                                    } else {
                                        v.selected = true;
                                    }
                                    modified_count += 1;
                                } else if !toggle {
                                    v.selected = false;
                                }
                            }
                        }
                    }
                    forge_scene::SelectionElement::Edge => {
                        for e in &mut edit_mesh.edges {
                            if e.v0 < edit_mesh.vertices.len() && e.v1 < edit_mesh.vertices.len() {
                                let p0 = model_mat.transform_point3(edit_mesh.vertices[e.v0].position);
                                let p1 = model_mat.transform_point3(edit_mesh.vertices[e.v1].position);
                                let mid = (p0 + p1) * 0.5;
                                if let Some(screen_p) =
                                    crate::ui::gizmo::world_to_screen_pos(mid, view_proj, viewport_rect)
                                {
                                    if box_rect.contains(screen_p) {
                                        if toggle {
                                            e.selected = !e.selected;
                                        } else {
                                            e.selected = true;
                                        }
                                        modified_count += 1;
                                    } else if !toggle {
                                        e.selected = false;
                                    }
                                }
                            }
                        }
                    }
                    forge_scene::SelectionElement::Face => {
                        for f in &mut edit_mesh.faces {
                            let mut face_center = forge_math::Vec3::ZERO;
                            let mut v_count = 0.0;
                            for &v_idx in &f.vertices {
                                if v_idx < edit_mesh.vertices.len() {
                                    face_center += model_mat.transform_point3(edit_mesh.vertices[v_idx].position);
                                    v_count += 1.0;
                                }
                            }
                            if v_count > 0.0 {
                                face_center /= v_count;
                                if let Some(screen_p) =
                                    crate::ui::gizmo::world_to_screen_pos(face_center, view_proj, viewport_rect)
                                {
                                    if box_rect.contains(screen_p) {
                                        if toggle {
                                            f.selected = !f.selected;
                                        } else {
                                            f.selected = true;
                                        }
                                        modified_count += 1;
                                    } else if !toggle {
                                        f.selected = false;
                                    }
                                }
                            }
                        }
                    }
                }
                self.sync_edit_mesh_to_scene();
                self.status_message = format!("Box Selected {} element(s)", modified_count);
                return;
            }
        }

        let mut hit_ids = std::collections::HashSet::new();
        for (&id, obj) in &self.scene.objects {
            if !obj.visible || !obj.selectable || obj.locked {
                continue;
            }
            let world_tf = self.scene.world_transform(id);
            let model_mat = world_tf.to_matrix();
            let mut screen_points = Vec::new();

            // 1. Pivot / Center
            if let Some(screen_p) =
                crate::ui::gizmo::world_to_screen_pos(world_tf.translation, view_proj, viewport_rect)
            {
                screen_points.push(screen_p);
            }

            // 2. 8 corners of object bounding box
            let aabb = obj.mesh.aabb;
            if !aabb.is_empty() {
                let corners = [
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z)),
                ];
                for c in &corners {
                    if let Some(sp) = crate::ui::gizmo::world_to_screen_pos(*c, view_proj, viewport_rect) {
                        screen_points.push(sp);
                    }
                }
            }

            // Check if any point is inside box_rect
            let mut is_hit = screen_points.iter().any(|p| box_rect.contains(*p));

            // If not directly containing points, check if projected 2D AABB intersects box_rect
            if !is_hit && screen_points.len() >= 2 {
                let mut min_x = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for p in &screen_points {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                let screen_aabb = egui::Rect::from_min_max(
                    egui::pos2(min_x, min_y),
                    egui::pos2(max_x, max_y),
                );
                if screen_aabb.intersects(box_rect) {
                    is_hit = true;
                }
            }

            if is_hit {
                hit_ids.insert(id);
            }
        }

        if toggle {
            for id in hit_ids {
                self.selection.toggle_selection(id);
            }
        } else if !hit_ids.is_empty() {
            self.selection.selected_ids = hit_ids;
            self.selection.active_id = self.selection.selected_ids.iter().next().copied();
        } else {
            self.selection.deselect();
        }
        self.status_message = format!(
            "Box Selected ({} objects)",
            self.selection.selected_ids.len()
        );
    }

    pub fn lasso_select(
        &mut self,
        polygon: &[egui::Pos2],
        view_proj: forge_math::Mat4,
        viewport_rect: egui::Rect,
        toggle: bool,
    ) {
        if polygon.len() < 3 {
            return;
        }

        let mut poly_bbox = egui::Rect::NOTHING;
        for p in polygon {
            poly_bbox.extend_with(*p);
        }

        if self.editor_state.edit_mode == EditMode::Edit {
            if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
                let aid = self.selection.active_id;
                let model_mat = aid
                    .and_then(|id| self.scene.get_object(id))
                    .map(|obj| obj.local_transform.to_matrix())
                    .unwrap_or(forge_math::Mat4::IDENTITY);

                let mut modified_count = 0;
                match self.editor_state.selection_element {
                    forge_scene::SelectionElement::Vertex => {
                        for v in &mut edit_mesh.vertices {
                            let world_p = model_mat.transform_point3(v.position);
                            if let Some(screen_p) =
                                crate::ui::gizmo::world_to_screen_pos(world_p, view_proj, viewport_rect)
                            {
                                if poly_bbox.contains(screen_p) && point_in_polygon(screen_p, polygon) {
                                    if toggle {
                                        v.selected = !v.selected;
                                    } else {
                                        v.selected = true;
                                    }
                                    modified_count += 1;
                                } else if !toggle {
                                    v.selected = false;
                                }
                            }
                        }
                    }
                    forge_scene::SelectionElement::Edge => {
                        for e in &mut edit_mesh.edges {
                            if e.v0 < edit_mesh.vertices.len() && e.v1 < edit_mesh.vertices.len() {
                                let p0 = model_mat.transform_point3(edit_mesh.vertices[e.v0].position);
                                let p1 = model_mat.transform_point3(edit_mesh.vertices[e.v1].position);
                                let mid = (p0 + p1) * 0.5;
                                if let Some(screen_p) =
                                    crate::ui::gizmo::world_to_screen_pos(mid, view_proj, viewport_rect)
                                {
                                    if poly_bbox.contains(screen_p) && point_in_polygon(screen_p, polygon) {
                                        if toggle {
                                            e.selected = !e.selected;
                                        } else {
                                            e.selected = true;
                                        }
                                        modified_count += 1;
                                    } else if !toggle {
                                        e.selected = false;
                                    }
                                }
                            }
                        }
                    }
                    forge_scene::SelectionElement::Face => {
                        for f in &mut edit_mesh.faces {
                            let mut face_center = forge_math::Vec3::ZERO;
                            let mut v_count = 0.0;
                            for &v_idx in &f.vertices {
                                if v_idx < edit_mesh.vertices.len() {
                                    face_center += model_mat.transform_point3(edit_mesh.vertices[v_idx].position);
                                    v_count += 1.0;
                                }
                            }
                            if v_count > 0.0 {
                                face_center /= v_count;
                                if let Some(screen_p) =
                                    crate::ui::gizmo::world_to_screen_pos(face_center, view_proj, viewport_rect)
                                {
                                    if poly_bbox.contains(screen_p) && point_in_polygon(screen_p, polygon) {
                                        if toggle {
                                            f.selected = !f.selected;
                                        } else {
                                            f.selected = true;
                                        }
                                        modified_count += 1;
                                    } else if !toggle {
                                        f.selected = false;
                                    }
                                }
                            }
                        }
                    }
                }
                self.sync_edit_mesh_to_scene();
                self.status_message = format!("Lasso Selected {} element(s)", modified_count);
                return;
            }
        }

        let mut hit_ids = std::collections::HashSet::new();
        for (&id, obj) in &self.scene.objects {
            if !obj.visible || !obj.selectable || obj.locked {
                continue;
            }
            let world_tf = self.scene.world_transform(id);
            let model_mat = world_tf.to_matrix();
            let mut screen_points = Vec::new();

            if let Some(screen_p) =
                crate::ui::gizmo::world_to_screen_pos(world_tf.translation, view_proj, viewport_rect)
            {
                screen_points.push(screen_p);
            }

            let aabb = obj.mesh.aabb;
            if !aabb.is_empty() {
                let corners = [
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z)),
                ];
                for c in &corners {
                    if let Some(sp) = crate::ui::gizmo::world_to_screen_pos(*c, view_proj, viewport_rect) {
                        screen_points.push(sp);
                    }
                }
            }

            let is_hit = screen_points.iter().any(|p| poly_bbox.contains(*p) && point_in_polygon(*p, polygon));
            if is_hit {
                hit_ids.insert(id);
            }
        }

        if toggle {
            for id in hit_ids {
                self.selection.toggle_selection(id);
            }
        } else if !hit_ids.is_empty() {
            self.selection.selected_ids = hit_ids;
            self.selection.active_id = self.selection.selected_ids.iter().next().copied();
        } else {
            self.selection.deselect();
        }
        self.status_message = format!(
            "Lasso Selected ({} objects)",
            self.selection.selected_ids.len()
        );
    }

    pub fn circle_select(
        &mut self,
        center: egui::Pos2,
        radius: f32,
        view_proj: forge_math::Mat4,
        viewport_rect: egui::Rect,
        deselect: bool,
    ) {
        if self.editor_state.edit_mode == EditMode::Edit {
            if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
                let aid = self.selection.active_id;
                let model_mat = aid
                    .and_then(|id| self.scene.get_object(id))
                    .map(|obj| obj.local_transform.to_matrix())
                    .unwrap_or(forge_math::Mat4::IDENTITY);

                let mut modified_count = 0;
                match self.editor_state.selection_element {
                    forge_scene::SelectionElement::Vertex => {
                        for v in &mut edit_mesh.vertices {
                            let world_p = model_mat.transform_point3(v.position);
                            if let Some(screen_p) =
                                crate::ui::gizmo::world_to_screen_pos(world_p, view_proj, viewport_rect)
                            {
                                if screen_p.distance(center) <= radius {
                                    v.selected = !deselect;
                                    modified_count += 1;
                                }
                            }
                        }
                    }
                    forge_scene::SelectionElement::Edge => {
                        for e in &mut edit_mesh.edges {
                            if e.v0 < edit_mesh.vertices.len() && e.v1 < edit_mesh.vertices.len() {
                                let p0 = model_mat.transform_point3(edit_mesh.vertices[e.v0].position);
                                let p1 = model_mat.transform_point3(edit_mesh.vertices[e.v1].position);
                                let mid = (p0 + p1) * 0.5;
                                if let Some(screen_p) =
                                    crate::ui::gizmo::world_to_screen_pos(mid, view_proj, viewport_rect)
                                {
                                    if screen_p.distance(center) <= radius {
                                        e.selected = !deselect;
                                        modified_count += 1;
                                    }
                                }
                            }
                        }
                    }
                    forge_scene::SelectionElement::Face => {
                        for f in &mut edit_mesh.faces {
                            let mut face_center = forge_math::Vec3::ZERO;
                            let mut v_count = 0.0;
                            for &v_idx in &f.vertices {
                                if v_idx < edit_mesh.vertices.len() {
                                    face_center += model_mat.transform_point3(edit_mesh.vertices[v_idx].position);
                                    v_count += 1.0;
                                }
                            }
                            if v_count > 0.0 {
                                face_center /= v_count;
                                if let Some(screen_p) =
                                    crate::ui::gizmo::world_to_screen_pos(face_center, view_proj, viewport_rect)
                                {
                                    if screen_p.distance(center) <= radius {
                                        f.selected = !deselect;
                                        modified_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                self.sync_edit_mesh_to_scene();
                self.status_message = format!("Circle Selected {} element(s)", modified_count);
                return;
            }
        }

        let mut hit_ids = Vec::new();
        for (&id, obj) in &self.scene.objects {
            if !obj.visible || !obj.selectable || obj.locked {
                continue;
            }
            let world_tf = self.scene.world_transform(id);
            let model_mat = world_tf.to_matrix();
            let mut screen_points = Vec::new();

            if let Some(screen_p) =
                crate::ui::gizmo::world_to_screen_pos(world_tf.translation, view_proj, viewport_rect)
            {
                screen_points.push(screen_p);
            }

            let aabb = obj.mesh.aabb;
            if !aabb.is_empty() {
                let corners = [
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z)),
                    model_mat.transform_point3(forge_math::Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z)),
                ];
                for c in &corners {
                    if let Some(sp) = crate::ui::gizmo::world_to_screen_pos(*c, view_proj, viewport_rect) {
                        screen_points.push(sp);
                    }
                }
            }

            let is_hit = screen_points.iter().any(|p| p.distance(center) <= radius);
            if is_hit {
                hit_ids.push(id);
            }
        }

        for id in hit_ids {
            if deselect {
                self.selection.remove_from_selection(id);
            } else {
                self.selection.add_to_selection(id);
            }
        }
        self.status_message = format!(
            "Circle Selected ({} objects)",
            self.selection.selected_ids.len()
        );
    }

    pub fn begin_modal_transform(
        &mut self,
        mode: forge_scene::TransformMode,
        mouse_pos: forge_math::Vec2,
    ) {
        let mut ids = self.selection.selected_ids.clone();
        if ids.is_empty() {
            if let Some(aid) = self.selection.active_id {
                ids.insert(aid);
            } else {
                return;
            }
        }

        let mut initial_transforms = std::collections::HashMap::new();
        let mut center_sum = forge_math::Vec3::ZERO;
        let mut count = 0.0;

        for &id in &ids {
            if let Some(obj) = self.scene.get_object(id) {
                initial_transforms.insert(id, obj.local_transform);
                center_sum += obj.local_transform.translation;
                count += 1.0;
            }
        }

        if count == 0.0 {
            return;
        }

        let pivot = match self.editor_state.pivot_point {
            forge_scene::PivotPoint::MedianPoint => center_sum / count,
            forge_scene::PivotPoint::ActiveElement => {
                if let Some(aid) = self.selection.active_id
                    && let Some(obj) = self.scene.get_object(aid)
                {
                    obj.local_transform.translation
                } else {
                    center_sum / count
                }
            }
            forge_scene::PivotPoint::Cursor3D => self.editor_state.cursor_3d,
            forge_scene::PivotPoint::IndividualOrigins => center_sum / count,
        };

        self.editor_state
            .modal_transform
            .begin(mode, mouse_pos, pivot, initial_transforms);
        self.status_message = format!("Modal Transform: {:?}", mode);
    }

    pub fn apply_modal_transform(&mut self) {
        self.editor_state.active_snap_target = None;
        if !self.editor_state.modal_transform.active {
            return;
        }

        let mut transforms = Vec::new();
        for (&id, &old_tf) in &self.editor_state.modal_transform.initial_transforms {
            if let Some(obj) = self.scene.get_object(id)
                && obj.local_transform != old_tf
            {
                transforms.push((id, old_tf, obj.local_transform));
            }
        }

        if !transforms.is_empty() {
            self.history.push(Action::MultiTransform { transforms });
            self.scene.dirty = true;
        }

        self.editor_state.modal_transform.cancel();
        self.status_message = "Transform Applied".to_string();
    }

    pub fn cancel_modal_transform(&mut self) {
        self.editor_state.active_snap_target = None;
        if !self.editor_state.modal_transform.active {
            return;
        }

        for (&id, &old_tf) in &self.editor_state.modal_transform.initial_transforms {
            if let Some(obj) = self.scene.get_object_mut(id) {
                obj.local_transform = old_tf;
            }
        }
        self.scene.dirty = true;
        self.editor_state.modal_transform.cancel();
        self.status_message = "Transform Cancelled".to_string();
    }

    /// Initiates a free cursor object drag on the current selection.
    pub fn start_object_drag(
        &mut self,
        hit_point: Vec3,
        hit_id: ObjectId,
        pointer_pos: Vec2,
        was_already_selected: bool,
    ) {
        let mut plane_normal = -self.camera.forward();
        if plane_normal.length_squared() < 1e-4 {
            plane_normal = (self.camera.eye - hit_point).normalize_or_zero();
        } else {
            plane_normal = plane_normal.normalize();
        }

        let mut initial_transforms = std::collections::HashMap::new();
        let mut initial_offsets = std::collections::HashMap::new();
        for &id in &self.selection.selected_ids {
            if let Some(obj) = self.scene.get_object(id) {
                initial_transforms.insert(id, obj.local_transform);
                initial_offsets.insert(id, obj.local_transform.translation - hit_point);
            }
        }
        if let Some(aid) = self.selection.active_id {
            if let Some(obj) = self.scene.get_object(aid) {
                initial_transforms.insert(aid, obj.local_transform);
                initial_offsets.insert(aid, obj.local_transform.translation - hit_point);
            }
        }

        self.object_drag_state = Some(ObjectDragState {
            initial_hit_point: hit_point,
            plane_point: hit_point,
            plane_normal,
            initial_transforms,
            initial_offsets,
            initial_pointer_pos: pointer_pos,
            is_active_drag: false,
            pressed_object_id: Some(hit_id),
            was_already_selected,
        });
        self.editor_state.pointer_state = forge_scene::PointerInteractionState::ObjectPressed;
        self.editor_state.pointer_press_pos = Some(pointer_pos);
    }

    /// Updates active free cursor object dragging given the current camera ray.
    /// Returns the world-space delta if successfully computed and applied.
    pub fn update_object_drag(&mut self, curr_ray: &Ray) -> Option<Vec3> {
        if let Some(drag) = &mut self.object_drag_state
            && let Some(mut delta) = calculate_drag_delta(
                curr_ray,
                drag.plane_point,
                drag.plane_normal,
                drag.initial_hit_point,
            )
        {
            if self.editor_state.snap_settings.enabled
                && self.editor_state.snap_settings.grid_increment > 0.0
            {
                let inc = self.editor_state.snap_settings.grid_increment;
                delta = Vec3::new(
                    (delta.x / inc).round() * inc,
                    (delta.y / inc).round() * inc,
                    (delta.z / inc).round() * inc,
                );
            }

            for (&id, &init_tf) in &drag.initial_transforms {
                if let Some(obj) = self.scene.get_object_mut(id) {
                    obj.local_transform.translation = init_tf.translation + delta;
                }
            }
            self.scene.dirty = true;
            return Some(delta);
        }
        None
    }

    /// Commits the active object drag operation into the Undo history stack.
    pub fn commit_object_drag(&mut self, drag: &ObjectDragState) {
        let mut changed = Vec::new();
        for (&id, &init_tf) in &drag.initial_transforms {
            if let Some(obj) = self.scene.get_object(id)
                && obj.local_transform.translation != init_tf.translation
            {
                changed.push((id, init_tf, obj.local_transform));
            }
        }
        if changed.len() == 1 {
            let (id, old_transform, new_transform) = changed[0];
            self.history.push(Action::Transform {
                id,
                old_transform,
                new_transform,
            });
            self.status_message = "Object moved".to_string();
        } else if changed.len() > 1 {
            self.history.push(Action::MultiTransform {
                transforms: changed,
            });
            self.status_message = "Multiple objects moved".to_string();
        }
        self.scene.dirty = true;
    }

    /// Cancels the active object drag operation, reverting all transformed objects to their initial positions.
    pub fn cancel_object_drag(&mut self) {
        if let Some(drag) = self.object_drag_state.take() {
            for (&id, &init_tf) in &drag.initial_transforms {
                if let Some(obj) = self.scene.get_object_mut(id) {
                    obj.local_transform = init_tf;
                }
            }
            self.editor_state.pointer_state = forge_scene::PointerInteractionState::Idle;
            self.editor_state.pointer_press_pos = None;
            self.scene.dirty = true;
            self.status_message = "Object drag cancelled".to_string();
        }
    }

    pub fn toggle_fly_navigation(&mut self) {
        self.editor_state.fly_nav_active = !self.editor_state.fly_nav_active;
        if self.editor_state.fly_nav_active {
            self.status_message =
                "Fly Navigation Active (WASD move, Mouse look, Esc exit)".to_string();
        } else {
            self.status_message = "Exited Fly Navigation".to_string();
        }
    }

    pub fn toggle_camera_view(&mut self) {
        if self.editor_state.is_camera_view {
            self.editor_state.is_camera_view = false;
            self.status_message = "User View".to_string();
        } else if let Some(cam_id) = self.scene.active_camera_id {
            if let Some(params) = self.scene.camera_view_parameters(cam_id) {
                self.camera.eye = params.eye;
                self.camera.target = params.target;
                self.camera.up = params.up;
                self.camera.fov_y_deg = params.fov_y_deg;
                self.camera.z_near = params.clip_start;
                self.camera.z_far = params.clip_end;
                if params.is_orthographic {
                    self.camera.projection = forge_render::CameraProjection::Orthographic;
                    self.camera.ortho_size = params.ortho_scale;
                } else {
                    self.camera.projection = forge_render::CameraProjection::Perspective;
                }
                self.editor_state.is_camera_view = true;
                self.status_message = "Camera View (Numpad 0 to exit)".to_string();
            }
        } else {
            self.status_message =
                "No active camera in scene. Add a Camera via Add menu.".to_string();
        }
    }

    pub fn toggle_xray(&mut self) {
        self.editor_state.show_xray = !self.editor_state.show_xray;
        self.status_message = format!(
            "X-Ray: {}",
            if self.editor_state.show_xray { "ON (Alt+Z)" } else { "OFF" }
        );
    }

    pub fn is_in_local_view(&self) -> bool {
        self.local_view_saved_visibility.is_some()
    }

    pub fn toggle_local_view(&mut self) {
        if let Some(saved) = self.local_view_saved_visibility.take() {
            // Exit Local View: restore previous visibility of each object
            for (&id, obj) in &mut self.scene.objects {
                if let Some(&was_visible) = saved.get(&id) {
                    obj.visible = was_visible;
                }
            }
            self.scene.dirty = true;
            if let Some(r) = &mut self.renderer {
                r.invalidate_picking();
            }
            self.pending_hover_pick = None;
            self.frame_selected();
            self.status_message = "Exited Local View (Global View)".to_string();
        } else {
            // Enter Local View: isolate selected objects
            let target_ids: Vec<ObjectId> = if self.editor_state.edit_mode == EditMode::Edit {
                if let Some((active_id, _)) = self.active_edit_mesh {
                    vec![active_id]
                } else {
                    self.selection.selected_ids.iter().copied().collect()
                }
            } else {
                self.selection.selected_ids.iter().copied().collect()
            };

            if target_ids.is_empty() {
                self.status_message =
                    "Local View: Select at least one object to isolate (Numpad /)".to_string();
                return;
            }

            let saved: std::collections::HashMap<ObjectId, bool> = self
                .scene
                .objects
                .iter()
                .map(|(&id, obj)| (id, obj.visible))
                .collect();

            for (&id, obj) in &mut self.scene.objects {
                obj.visible = target_ids.contains(&id);
            }

            self.scene.dirty = true;
            if let Some(r) = &mut self.renderer {
                r.invalidate_picking();
            }
            self.pending_hover_pick = None;
            self.local_view_saved_visibility = Some(saved);
            self.frame_selected();
            self.status_message = format!(
                "Local View: Isolated {} object{} (Press '/' to exit)",
                target_ids.len(),
                if target_ids.len() == 1 { "" } else { "s" }
            );
        }
    }

    pub fn toggle_overlays(&mut self) {
        self.editor_state.show_overlays = !self.editor_state.show_overlays;
        self.status_message = format!(
            "Viewport Overlays: {}",
            if self.editor_state.show_overlays { "ON" } else { "OFF" }
        );
    }

    pub fn toggle_nav_widget(&mut self) {
        self.editor_state.show_nav_widget = !self.editor_state.show_nav_widget;
        self.status_message = format!(
            "Navigation Widget: {}",
            if self.editor_state.show_nav_widget { "ON" } else { "OFF" }
        );
    }

    /// Returns true if an object matches the active Outliner filters (search text, type, selection, visibility).
    pub fn object_matches_outliner_filter(&self, id: ObjectId) -> bool {
        let Some(obj) = self.scene.objects.get(&id) else {
            return false;
        };

        // 1. Text search query (case-insensitive substring match)
        let query = self.outliner_filter.search_query.trim().to_lowercase();
        if !query.is_empty() && !obj.name.to_lowercase().contains(&query) {
            return false;
        }

        // 2. Type filter
        match self.outliner_filter.type_filter {
            OutlinerTypeFilter::All => {}
            OutlinerTypeFilter::Meshes => {
                if obj.primitive_type == PrimitiveType::Camera
                    || obj.primitive_type == PrimitiveType::Light
                {
                    return false;
                }
            }
            OutlinerTypeFilter::Lights => {
                if obj.primitive_type != PrimitiveType::Light {
                    return false;
                }
            }
            OutlinerTypeFilter::Cameras => {
                if obj.primitive_type != PrimitiveType::Camera {
                    return false;
                }
            }
        }

        // 3. Only selected
        if self.outliner_filter.only_selected && !self.selection.is_selected(id) {
            return false;
        }

        // 4. Only visible
        if self.outliner_filter.only_visible && !obj.visible {
            return false;
        }

        true
    }

    /// Returns true if an object or any of its descendants match the Outliner filter.
    pub fn outliner_subtree_has_match(&self, id: ObjectId) -> bool {
        if self.object_matches_outliner_filter(id) {
            return true;
        }
        if let Some(obj) = self.scene.objects.get(&id) {
            for child_id in &obj.children {
                if self.outliner_subtree_has_match(*child_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Sets selectability flag for all objects in the scene.
    pub fn make_all_objects_selectable(&mut self, selectable: bool) {
        for obj in self.scene.objects.values_mut() {
            obj.selectable = selectable;
        }
        self.status_message = if selectable {
            "All objects marked selectable".to_string()
        } else {
            "All objects marked unselectable".to_string()
        };
    }

    /// Sets visibility flag for all objects in the scene.
    pub fn make_all_objects_visible(&mut self, visible: bool) {
        for obj in self.scene.objects.values_mut() {
            obj.visible = visible;
        }
        self.status_message = if visible {
            "All objects visible".to_string()
        } else {
            "All objects hidden".to_string()
        };
    }

    /// Toggles the selectability column visibility in the Outliner.
    pub fn toggle_outliner_selectability_column(&mut self) {
        self.outliner_filter.show_selectability_column =
            !self.outliner_filter.show_selectability_column;
        self.status_message = format!(
            "Outliner Selectability Column: {}",
            if self.outliner_filter.show_selectability_column {
                "ON"
            } else {
                "OFF"
            }
        );
    }

    pub fn align_active_camera_to_view(&mut self) {
        if let Some(cam_id) = self.scene.active_camera_id {
            self.scene.align_active_camera_to_view(
                self.camera.eye,
                self.camera.target,
                self.camera.up,
            );
            if let Some(cam_obj) = self.scene.get_object_mut(cam_id)
                && let Some(data) = &mut cam_obj.camera_data
            {
                data.fov_y_deg = self.camera.fov_y_deg;
                data.clip_start = self.camera.z_near;
                data.clip_end = self.camera.z_far;
                data.ortho_scale = self.camera.ortho_size;
                data.projection = match self.camera.projection {
                    forge_render::CameraProjection::Perspective => {
                        forge_scene::CameraProjectionType::Perspective
                    }
                    forge_render::CameraProjection::Orthographic => {
                        forge_scene::CameraProjectionType::Orthographic
                    }
                };
            }
            self.status_message = "Aligned Active Camera to View".to_string();
        } else {
            self.status_message = "No active camera to align".to_string();
        }
    }

    pub fn parent_selected_to_active(&mut self) {
        let active = self.selection.active_id;
        let selected = self.selection.selected_ids.clone();
        if let Some(active_id) = active {
            for id in selected {
                if id != active_id
                    && let Some(action) = self.scene.parent_object(id, Some(active_id))
                {
                    self.history.push(action);
                }
            }
            self.status_message = "Parented selected objects to active".to_string();
        }
    }

    pub fn unparent_selected(&mut self) {
        let selected = self.selection.selected_ids.clone();
        for id in selected {
            if let Some(action) = self.scene.parent_object(id, None) {
                self.history.push(action);
            }
        }
        self.status_message = "Unparented selected objects".to_string();
    }

    pub fn join_selected(&mut self) {
        let active = self.selection.active_id;
        let selected: Vec<ObjectId> = self.selection.selected_ids.iter().copied().collect();
        if let Some(active_id) = active
            && let Some(action) = self.scene.join_objects(active_id, &selected)
        {
            self.history.push(action);
            self.selection.select(active_id);
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
            self.status_message = "Joined selected objects".to_string();
        }
    }

    pub fn hide_selected(&mut self) {
        for &id in &self.selection.selected_ids {
            self.scene.set_object_visibility(id, false);
        }
        self.selection.deselect();
        self.status_message = "Hidden selected objects (Alt+H to unhide)".to_string();
    }

    pub fn unhide_all(&mut self) {
        self.scene.unhide_all();
        self.status_message = "Unhid all objects".to_string();
    }

    pub fn set_origin_selected_geometry(&mut self) {
        if let Some(aid) = self.selection.active_id
            && let Some(action) = self.scene.set_origin_to_geometry(aid)
        {
            self.history.push(action);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&aid);
            }
            self.status_message = "Set Origin to Geometry".to_string();
        }
    }

    pub fn set_origin_selected_cursor(&mut self) {
        let cursor = self.editor_state.cursor_3d;
        if let Some(aid) = self.selection.active_id
            && let Some(action) = self.scene.set_origin_to_cursor(aid, cursor)
        {
            self.history.push(action);
            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(&aid);
            }
            self.status_message = "Set Origin to 3D Cursor".to_string();
        }
    }

    pub fn cursor_to_selected(&mut self) {
        if self.editor_state.edit_mode == EditMode::Edit {
            if let Some((_, edit_mesh)) = &self.active_edit_mesh {
                if let Some(center_local) = edit_mesh.selection_center(forge_geometry::EditSelectionMode::Vertex) {
                    let center_world = if let Some(aid) = self.selection.active_id
                        && let Some(obj) = self.scene.get_object(aid)
                    {
                        obj.local_transform.to_matrix().transform_point3(center_local)
                    } else {
                        center_local
                    };
                    self.editor_state.cursor_3d = center_world;
                    self.status_message = format!(
                        "Cursor to Selected: ({:.2}, {:.2}, {:.2})",
                        center_world.x, center_world.y, center_world.z
                    );
                    return;
                }
            }
        }

        let bbox = self.scene.selected_aabb(&self.selection);
        if !bbox.is_empty() {
            self.editor_state.cursor_3d = bbox.center();
            self.status_message = format!(
                "Cursor to Selected: ({:.2}, {:.2}, {:.2})",
                bbox.center().x, bbox.center().y, bbox.center().z
            );
        } else if let Some(aid) = self.selection.active_id
            && let Some(obj) = self.scene.get_object(aid)
        {
            self.editor_state.cursor_3d = obj.local_transform.translation;
            self.status_message = format!(
                "Cursor to Active: ({:.2}, {:.2}, {:.2})",
                obj.local_transform.translation.x,
                obj.local_transform.translation.y,
                obj.local_transform.translation.z
            );
        } else {
            self.status_message = "No object selected".to_string();
        }
    }

    pub fn cursor_to_world_origin(&mut self) {
        self.editor_state.cursor_3d = Vec3::ZERO;
        self.status_message = "Cursor to World Origin (0, 0, 0)".to_string();
    }

    pub fn cursor_to_grid(&mut self) {
        let step = self.editor_state.snap_settings.grid_increment.max(0.01);
        self.editor_state.cursor_3d = (self.editor_state.cursor_3d / step).round() * step;
        self.status_message = format!(
            "Cursor to Grid: ({:.2}, {:.2}, {:.2})",
            self.editor_state.cursor_3d.x,
            self.editor_state.cursor_3d.y,
            self.editor_state.cursor_3d.z
        );
    }

    pub fn selection_to_cursor(&mut self) {
        let target_cursor = self.editor_state.cursor_3d;
        let mut transforms = Vec::new();

        if self.selection.selected_ids.is_empty() {
            if let Some(aid) = self.selection.active_id {
                if let Some(obj) = self.scene.get_object_mut(aid) {
                    let old_tf = obj.local_transform;
                    obj.local_transform.translation = target_cursor;
                    transforms.push((aid, old_tf, obj.local_transform));
                }
            }
        } else {
            let bbox = self.scene.selected_aabb(&self.selection);
            let center = if !bbox.is_empty() {
                bbox.center()
            } else if let Some(aid) = self.selection.active_id && let Some(obj) = self.scene.get_object(aid) {
                obj.local_transform.translation
            } else {
                target_cursor
            };
            let delta = target_cursor - center;

            for &id in &self.selection.selected_ids {
                if let Some(obj) = self.scene.get_object_mut(id) {
                    let old_tf = obj.local_transform;
                    obj.local_transform.translation += delta;
                    transforms.push((id, old_tf, obj.local_transform));
                }
            }
        }

        if !transforms.is_empty() {
            self.history.push(Action::MultiTransform { transforms });
            self.scene.dirty = true;
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
            self.status_message = "Selection to Cursor".to_string();
        } else {
            self.status_message = "No object selected to move".to_string();
        }
    }

    pub fn selection_to_grid(&mut self) {
        let step = self.editor_state.snap_settings.grid_increment.max(0.01);
        let mut transforms = Vec::new();

        for &id in &self.selection.selected_ids {
            if let Some(obj) = self.scene.get_object_mut(id) {
                let old_tf = obj.local_transform;
                let snapped = (obj.local_transform.translation / step).round() * step;
                if snapped != obj.local_transform.translation {
                    obj.local_transform.translation = snapped;
                    transforms.push((id, old_tf, obj.local_transform));
                }
            }
        }

        if !transforms.is_empty() {
            self.history.push(Action::MultiTransform { transforms });
            self.scene.dirty = true;
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
            self.status_message = "Selection to Grid".to_string();
        } else {
            self.status_message = "Selection already on Grid".to_string();
        }
    }

    pub fn undo(&mut self) {
        if let Some(desc) = self.history.undo(&mut self.scene) {
            self.status_message = format!("Undo: {desc}");
            self.pending_hover_pick = None;
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
            if let Some((active_id, _)) = &self.active_edit_mesh {
                let aid = *active_id;
                if let Some(obj) = self.scene.get_object(aid) {
                    let mut new_em = EditableMesh::from_mesh(&obj.mesh);
                    new_em.weld_colocated_vertices(1e-4);
                    new_em.tris_to_quads();
                    self.active_edit_mesh = Some((aid, new_em));
                    self.edit_overlay_cache.reset();
                }
            }
        } else {
            self.status_message = "Nothing to undo".to_string();
        }
    }

    pub fn redo(&mut self) {
        if let Some(desc) = self.history.redo(&mut self.scene) {
            self.status_message = format!("Redo: {desc}");
            self.pending_hover_pick = None;
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
            if let Some((active_id, _)) = &self.active_edit_mesh {
                let aid = *active_id;
                if let Some(obj) = self.scene.get_object(aid) {
                    let mut new_em = EditableMesh::from_mesh(&obj.mesh);
                    new_em.weld_colocated_vertices(1e-4);
                    new_em.tris_to_quads();
                    self.active_edit_mesh = Some((aid, new_em));
                    self.edit_overlay_cache.reset();
                }
            }
        } else {
            self.status_message = "Nothing to redo".to_string();
        }
    }

    pub fn frame_selected(&mut self) {
        if self.editor_state.edit_mode == EditMode::Edit {
            if let Some((active_id, edit_mesh)) = &self.active_edit_mesh {
                let aid = *active_id;
                let tf = self.scene.world_transform(aid);
                let model_mat = tf.to_matrix();
                let sel_pts: Vec<forge_math::Vec3> = edit_mesh
                    .vertices
                    .iter()
                    .filter(|v| v.selected)
                    .map(|v| model_mat.transform_point3(v.position))
                    .collect();
                if !sel_pts.is_empty() {
                    let aabb = forge_math::Aabb::from_points(&sel_pts);
                    self.camera.frame_box(&aabb);
                    self.status_message = format!("Framed {} Selected Vertex/Vertices", sel_pts.len());
                    self.pending_hover_pick = None;
                    if let Some(r) = &mut self.renderer {
                        r.invalidate_picking();
                    }
                    return;
                }
            }
        }

        let mut bbox = self.scene.selected_aabb(&self.selection);
        if bbox.is_empty() {
            if let Some(aid) = self.selection.active_id {
                let tf = self.scene.world_transform(aid);
                if let Some(obj) = self.scene.get_object(aid) {
                    if !obj.mesh.aabb.is_empty() {
                        let corners = [
                            tf.transform_point(Vec3::new(obj.mesh.aabb.min.x, obj.mesh.aabb.min.y, obj.mesh.aabb.min.z)),
                            tf.transform_point(Vec3::new(obj.mesh.aabb.max.x, obj.mesh.aabb.max.y, obj.mesh.aabb.max.z)),
                        ];
                        bbox = forge_math::Aabb::from_points(&corners);
                    } else {
                        bbox = forge_math::Aabb::from_center_extents(tf.translation, Vec3::splat(1.0));
                    }
                }
            } else {
                bbox = self.scene.scene_aabb();
            }
        }

        self.camera.frame_box(&bbox);
        self.pending_hover_pick = None;
        if let Some(r) = &mut self.renderer {
            r.invalidate_picking();
        }
        self.status_message = "Framed Selected (Focus)".to_string();
    }

    pub fn frame_all(&mut self) {
        let bbox = self.scene.scene_aabb();
        self.camera.frame_box(&bbox);
        self.pending_hover_pick = None;
        if let Some(r) = &mut self.renderer {
            r.invalidate_picking();
        }
        self.status_message = "Framed All".to_string();
    }

    /// Replaces the current scene with the built-in house, tree, and rock showcase.
    pub fn load_showcase_scene(&mut self) {
        if self.editor_state.modal_transform.active {
            self.cancel_modal_transform();
        }
        if self.object_drag_state.is_some() {
            self.cancel_object_drag();
        }

        self.scene = Scene::create_showcase_scene();
        self.selection.deselect();
        self.history.clear();
        self.active_edit_mesh = None;
        self.edit_overlay_cache.clear();
        self.editor_state.edit_mode = EditMode::Object;
        self.editor_state.fly_nav_active = false;
        self.editor_state.pointer_state = forge_scene::PointerInteractionState::Idle;
        self.gizmo_state.drag_state = None;
        self.gizmo_initial_transforms.clear();
        self.pending_hover_pick = None;
        set_showcase_camera_view(&mut self.camera);

        if let Some(renderer) = &mut self.renderer {
            renderer.invalidate_all_meshes();
        }

        self.start_showcase_demo();
    }

    /// Starts the timed, clearly labeled visual walkthrough of the showcase scene.
    pub fn start_showcase_demo(&mut self) {
        self.showcase_demo_elapsed = Some(0.0);
        reveal_showcase_through_stage(&mut self.scene, 0);
        self.pending_hover_pick = None;
        if let Some(renderer) = &mut self.renderer {
            renderer.invalidate_picking();
        }
        self.status_message = "Showcase demo: visually previewing the build steps".to_string();
    }

    fn update_showcase_demo(&mut self, dt: f32) {
        let Some(elapsed) = self.showcase_demo_elapsed else {
            return;
        };

        let elapsed = elapsed + dt.max(0.0);
        let stage_duration = 1.35;
        let stage = ((elapsed / stage_duration).floor() as usize).min(5);
        if reveal_showcase_through_stage(&mut self.scene, stage) {
            self.pending_hover_pick = None;
            if let Some(renderer) = &mut self.renderer {
                renderer.invalidate_picking();
            }
        }

        if elapsed >= stage_duration * 6.0 + 0.35 {
            self.showcase_demo_elapsed = None;
            self.status_message = "Showcase build demo complete".to_string();
        } else {
            self.showcase_demo_elapsed = Some(elapsed);
        }
    }

    /// Trims free heap memory back to the Linux OS via glibc malloc_trim.
    #[inline]
    pub fn trim_process_memory(&self) {
        #[cfg(target_os = "linux")]
        unsafe {
            extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            malloc_trim(0);
        }
    }

    /// Switches from Startup Hub to active editor with a clean default 3D scene (Cube, Camera, Light).
    pub fn start_new_scene(&mut self) {
        self.pending_hover_pick = None;
        self.local_view_saved_visibility = None;
        self.scene = Scene::create_default_scene();
        self.selection.deselect();
        self.history.clear();
        self.editor_state.workspace = forge_scene::WorkspaceMode::Modeling;
        self.editor_state.edit_mode = EditMode::Object;
        if let Some(r) = &mut self.renderer {
            r.invalidate_all_meshes();
        }
        self.trim_process_memory();
        self.status_message = "New scene created".to_string();
        self.is_in_startup_hub = false;
    }

    /// Loads a `.forge` project file directly from disk and enters active workspace.
    pub fn open_project_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.pending_hover_pick = None;
        let path_ref = path.as_ref();
        let json = std::fs::read_to_string(path_ref)?;
        let project = forge_scene::ForgeProject::from_json(&json)?;
        self.scene = project.scene;
        self.selection.deselect();
        self.history.clear();
        if let Some(r) = &mut self.renderer {
            r.invalidate_all_meshes();
        }
        self.trim_process_memory();
        let file_name = path_ref
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project.forge");
        self.status_message = format!("Loaded project from {}", file_name);

        self.editor_state.panel_settings.add_recent_project(
            path_ref.to_string_lossy().to_string(),
            self.scene.name.clone(),
        );
        self.is_in_startup_hub = false;
        Ok(())
    }

    /// Opens or imports a file based on its extension (.forge, .obj, .gltf, .glb, .stl, .ply).
    pub fn open_or_import_file(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        self.pending_hover_pick = None;
        let path_ref = path.as_ref();
        let ext = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let res = match ext.as_str() {
            "forge" => self.open_project_from_path(path_ref).map_err(|e| e.to_string()),
            "obj" => self.import_obj_from_path(path_ref),
            "gltf" | "glb" => self.import_gltf_from_path(path_ref),
            "stl" => self.import_stl_from_path(path_ref).map_err(|e| e.to_string()),
            "ply" => self.import_ply_from_path(path_ref).map_err(|e| e.to_string()),
            _ => Err(format!("Unsupported file extension: .{ext}")),
        };

        if res.is_ok() {
            self.is_in_startup_hub = false;
        }
        res
    }

    /// Loads the architectural showcase villa and enters active workspace.
    pub fn load_showcase_and_start(&mut self) {
        self.pending_hover_pick = None;
        self.load_showcase_scene();
        self.start_showcase_demo();
        self.status_message = "Loaded Architectural Showcase".to_string();
        self.is_in_startup_hub = false;
    }

    /// Toggles between light and dark visual interface theme and persists preference.
    pub fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.editor_state.panel_settings.is_light_theme =
            !self.editor_state.panel_settings.is_light_theme;
        apply_theme(self.editor_state.panel_settings.is_light_theme, ctx);
        let _ = self.editor_state.panel_settings.save();
        ctx.request_repaint();
    }

    pub fn render_app_ui(&mut self, ui: &mut egui::Ui) {
        if self.is_in_startup_hub {
            crate::ui::welcome::render_welcome_hub(self, ui);
            return;
        }

        let ctx = ui.ctx().clone();
        let is_light = self.editor_state.panel_settings.is_light_theme;
        let panel_bg = if is_light {
            egui::Color32::from_rgb(245, 246, 250)
        } else {
            egui::Color32::from_rgb(19, 20, 25)
        };
        let panel_border = if is_light {
            egui::Color32::from_rgb(215, 220, 232)
        } else {
            egui::Color32::from_rgb(32, 35, 42)
        };
        let side_border = if is_light {
            egui::Color32::from_rgb(215, 220, 232)
        } else {
            egui::Color32::from_rgb(38, 42, 54)
        };
        let central_bg = if is_light {
            egui::Color32::from_rgb(230, 234, 242)
        } else {
            egui::Color32::from_rgb(20, 22, 26)
        };

        // 1. Measure frame delta and FPS
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;
        if dt > 0.0 {
            let current_fps = 1.0 / dt;
            self.fps = self.fps * 0.9 + current_fps * 0.1;
            self.frame_time_ms = dt * 1000.0;
        }
        if self.showcase_demo_elapsed.is_some() {
            self.update_showcase_demo(dt);
            ctx.request_repaint();
        }

        // Safety: Window focus loss cancels modal operations safely
        let window_focused = ui.input(|i| i.focused);
        if !window_focused {
            if self.editor_state.modal_transform.active {
                self.cancel_modal_transform();
            }
            if self.object_drag_state.is_some() {
                self.cancel_object_drag();
            }
            self.editor_state.pointer_state = forge_scene::PointerInteractionState::Idle;
            self.gizmo_state.drag_state = None;
        }

        // 2. Top Menu Bar
        egui::Panel::top("menu_bar")
            .frame(
                egui::Frame::new()
                    .fill(panel_bg)
                    .stroke(egui::Stroke::new(1.0, panel_border))
                    .inner_margin(4.0),
            )
            .show(ui, |ui| {
                render_menu_bar(self, ui);
            });

        // 3. Bottom Status Bar
        egui::Panel::bottom("status_bar")
            .frame(
                egui::Frame::new()
                    .fill(panel_bg)
                    .stroke(egui::Stroke::new(1.0, panel_border))
                    .inner_margin(4.0),
            )
            .show(ui, |ui| {
                render_status_bar(self, ui);
            });

        // 4. Left Toolbar (Natively resizable with clean border separator and auto mode switch)
        if self.editor_state.panel_settings.toolbar_visible {
            let toolbar_w = self.editor_state.panel_settings.toolbar_width;
            let panel_resp = egui::Panel::left("tools_panel")
                .resizable(true)
                .default_size(toolbar_w.clamp(48.0, 260.0))
                .size_range(48.0..=280.0)
                .show_separator_line(true)
                .frame(
                    egui::Frame::new()
                        .fill(panel_bg)
                        .stroke(egui::Stroke::new(1.0, side_border))
                        .inner_margin(4.0),
                )
                .show(ui, |ui| {
                    render_toolbar(self, ui);
                });

            let new_w = panel_resp.response.rect.width();
            if (new_w - self.editor_state.panel_settings.toolbar_width).abs() > 0.5 {
                self.editor_state.panel_settings.toolbar_width = new_w;
                if new_w >= 90.0 {
                    self.editor_state.panel_settings.toolbar_mode =
                        forge_scene::ToolbarDisplayMode::IconAndLabel;
                } else {
                    self.editor_state.panel_settings.toolbar_mode =
                        forge_scene::ToolbarDisplayMode::IconOnly;
                }
                let _ = self.editor_state.panel_settings.save();
            }
        }

        // 5. Bottom Timeline Panel (Optional / Resizable)
        if self.editor_state.panel_settings.timeline_visible {
            let timeline_h = self.editor_state.panel_settings.timeline_height;
            egui::Panel::bottom("timeline_panel")
                .resizable(false)
                .exact_size(timeline_h)
                .show(ui, |ui| {
                    let total_w = ui.available_width();
                    // Top horizontal drag splitter
                    let (splitter_rect, response) = ui.allocate_exact_size(
                        egui::vec2(total_w, 6.0),
                        egui::Sense::click_and_drag(),
                    );
                    if response.hovered() || response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if response.dragged() {
                        let delta_y = response.drag_delta().y;
                        self.editor_state.panel_settings.timeline_height =
                            (self.editor_state.panel_settings.timeline_height - delta_y)
                                .clamp(60.0, 350.0);
                        let _ = self.editor_state.panel_settings.save();
                    }
                    let col = if response.dragged() {
                        egui::Color32::from_rgb(240, 144, 32)
                    } else if response.hovered() {
                        egui::Color32::from_rgb(255, 175, 50)
                    } else {
                        egui::Color32::from_rgb(38, 42, 50)
                    };
                    ui.painter().rect_filled(splitter_rect, 1.0, col);

                    let frame_id = ui.make_persistent_id("studio_timeline_frame");
                    let mut current_frame: i32 =
                        ui.data_mut(|d| *d.get_temp_mut_or_default(frame_id));
                    if current_frame == 0 {
                        current_frame = 20; // Default matching reference screenshot
                    }

                    // 1. Frame Ruler Row (0 ... 30 F with ticks)
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 2.0);
                        let frames = [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30];
                        for f in frames {
                            if f == current_frame {
                                let _ = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(format!("[{f}]"))
                                            .strong()
                                            .size(10.5)
                                            .color(egui::Color32::from_rgb(34, 197, 94)),
                                    )
                                    .fill(egui::Color32::from_rgb(26, 42, 32))
                                    .corner_radius(egui::CornerRadius::same(3)),
                                );
                            } else if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(format!("{f}"))
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(140, 150, 170)),
                                    )
                                    .fill(egui::Color32::TRANSPARENT),
                                )
                                .clicked()
                            {
                                current_frame = f;
                            }
                        }
                        ui.label(
                            egui::RichText::new("30 F")
                                .strong()
                                .size(10.5)
                                .color(egui::Color32::from_rgb(170, 180, 200)),
                        );
                    });

                    ui.add_space(2.0);

                    // 2. Transport Scrubber Row with Red Bar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 2.0);
                        ui.label(
                            egui::RichText::new("0 F :")
                                .monospace()
                                .size(11.0)
                                .color(egui::Color32::from_rgb(160, 170, 190)),
                        );

                        // Red Scrub Progress Bar
                        let bar_w = (ui.available_width() - 240.0).max(120.0);
                        let (bar_rect, bar_resp) = ui.allocate_exact_size(
                            egui::vec2(bar_w, 14.0),
                            egui::Sense::click_and_drag(),
                        );
                        let progress = (current_frame as f32 / 30.0).clamp(0.0, 1.0);

                        // Background track
                        ui.painter().rect_filled(
                            bar_rect,
                            egui::CornerRadius::same(3),
                            egui::Color32::from_rgb(28, 30, 38),
                        );
                        // Red filled active progress
                        let filled_w = bar_rect.width() * progress;
                        let filled_rect = egui::Rect::from_min_size(
                            bar_rect.min,
                            egui::vec2(filled_w, bar_rect.height()),
                        );
                        ui.painter().rect_filled(
                            filled_rect,
                            egui::CornerRadius::same(3),
                            egui::Color32::from_rgb(239, 68, 68), // Octane Red
                        );
                        // Playhead handle
                        let handle_x = bar_rect.min.x + filled_w;
                        let handle_rect = egui::Rect::from_center_size(
                            egui::pos2(handle_x, bar_rect.center().y),
                            egui::vec2(6.0, 18.0),
                        );
                        ui.painter().rect_filled(
                            handle_rect,
                            egui::CornerRadius::same(2),
                            egui::Color32::WHITE,
                        );

                        if (bar_resp.clicked() || bar_resp.dragged())
                            && let Some(mouse) = bar_resp.interact_pointer_pos()
                        {
                            let norm =
                                ((mouse.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            current_frame = (norm * 30.0).round() as i32;
                        }

                        ui.label(
                            egui::RichText::new(format!("{current_frame} F :"))
                                .monospace()
                                .size(11.0)
                                .color(egui::Color32::from_rgb(160, 170, 190)),
                        );

                        // Transport playback buttons
                        if ui
                            .button("|<")
                            .on_hover_text("Jump to First Frame (Shift+Left)")
                            .clicked()
                        {
                            current_frame = 0;
                            self.status_message = "Timeline: Jump to Frame 0".to_string();
                        }
                        if ui
                            .button("<")
                            .on_hover_text("Step Back 1 Frame (Left)")
                            .clicked()
                        {
                            current_frame = (current_frame - 1).max(0);
                        }
                        if ui
                            .button("Play")
                            .on_hover_text("Play / Pause (Space)")
                            .clicked()
                        {
                            self.status_message = "Timeline: Play / Pause".to_string();
                        }
                        if ui
                            .button(">|")
                            .on_hover_text("Jump to End Frame (Shift+Right)")
                            .clicked()
                        {
                            current_frame = 30;
                            self.status_message = "Timeline: Jump to End".to_string();
                        }
                        if ui
                            .button("Loop")
                            .on_hover_text("Toggle Loop Playback")
                            .clicked()
                        {
                            self.status_message = "Timeline: Loop Active".to_string();
                        }
                    });

                    ui.data_mut(|d| *d.get_temp_mut_or_default(frame_id) = current_frame);

                    ui.add_space(4.0);
                    ui.separator();

                    // 3. Bottom Material Shelf preview
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                        let link_col = egui::Color32::from_rgb(150, 160, 180);
                        if ui
                            .button(egui::RichText::new("Create").size(11.0).color(link_col))
                            .clicked()
                        {
                            self.show_material_editor = true;
                        }
                        if ui
                            .button(egui::RichText::new("Edit").size(11.0).color(link_col))
                            .clicked()
                        {
                            self.show_material_editor = !self.show_material_editor;
                        }
                        if ui
                            .button(egui::RichText::new("Function").size(11.0).color(link_col))
                            .clicked()
                        {
                            self.show_material_editor = true;
                        }
                        if ui
                            .button(egui::RichText::new("Texture").size(11.0).color(link_col))
                            .clicked()
                        {
                            self.show_material_editor = true;
                        }
                        ui.separator();

                        // Material spheres preview pills
                        if ui
                            .button(
                                egui::RichText::new("Gold")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 215, 80)),
                            )
                            .on_hover_text("Assign Gold Material to Selected Object")
                            .clicked()
                            && let Some(id) = self.selection.active_id
                        {
                            let mut updated = None;
                            if let Some(obj) = self.scene.get_object_mut(id) {
                                obj.color = [1.0, 0.85, 0.25, 1.0];
                                updated = Some(obj.name.clone());
                            }
                            if let Some(name) = updated {
                                self.scene.dirty = true;
                                self.status_message = format!("Assigned Gold material to {}", name);
                            }
                        }

                        if ui
                            .button(
                                egui::RichText::new("Diffuse")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(230, 235, 245)),
                            )
                            .on_hover_text("Assign Diffuse Material to Selected Object")
                            .clicked()
                            && let Some(id) = self.selection.active_id
                        {
                            self.scene
                                .materials
                                .entry("builtin/diffuse".into())
                                .or_insert_with(|| forge_scene::PbrMaterial {
                                    name: "Diffuse".into(),
                                    base_color_factor: [0.8, 0.8, 0.85, 1.0],
                                    metallic_factor: 0.0,
                                    roughness_factor: 0.8,
                                    ..Default::default()
                                });
                            let mut updated = None;
                            if let Some(obj) = self.scene.get_object_mut(id) {
                                obj.color = [1.0; 4];
                                obj.material_id = Some("builtin/diffuse".into());
                                updated = Some(obj.name.clone());
                            }
                            if let Some(name) = updated {
                                self.scene.dirty = true;
                                self.status_message =
                                    format!("Assigned Diffuse material to {}", name);
                            }
                        }

                        if ui
                            .button(
                                egui::RichText::new("Metal")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(190, 205, 225)),
                            )
                            .on_hover_text("Assign Brushed Metal Material to Selected Object")
                            .clicked()
                            && let Some(id) = self.selection.active_id
                        {
                            self.scene
                                .materials
                                .entry("builtin/brushed_metal".into())
                                .or_insert_with(|| forge_scene::PbrMaterial {
                                    name: "Brushed Metal".into(),
                                    base_color_factor: [0.65, 0.70, 0.75, 1.0],
                                    metallic_factor: 1.0,
                                    roughness_factor: 0.32,
                                    ..Default::default()
                                });
                            let mut updated = None;
                            if let Some(obj) = self.scene.get_object_mut(id) {
                                obj.color = [1.0; 4];
                                obj.material_id = Some("builtin/brushed_metal".into());
                                updated = Some(obj.name.clone());
                            }
                            if let Some(name) = updated {
                                self.scene.dirty = true;
                                self.status_message =
                                    format!("Assigned Brushed Metal material to {}", name);
                            }
                        }
                    });
                });
        }

        // 6. Right Sidebar: Outliner (Top) and Properties (Bottom)
        if self.editor_state.panel_settings.sidebar_visible {
            let sidebar_w = self.editor_state.panel_settings.sidebar_width;
            let panel_resp = egui::Panel::right("outliner_properties_panel")
                .resizable(true)
                .default_size(sidebar_w.clamp(240.0, 600.0))
                .size_range(240.0..=600.0)
                .show_separator_line(true)
                .frame(
                    egui::Frame::new()
                        .fill(panel_bg)
                        .stroke(egui::Stroke::new(1.0, side_border))
                        .inner_margin(4.0),
                )
                .show(ui, |ui| {
                    let total_h = ui.available_height();
                    let outliner_h = (total_h
                        * self.editor_state.panel_settings.outliner_height_ratio)
                        .clamp(60.0, (total_h - 80.0).max(60.0));
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), outliner_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            render_outliner(self, ui);
                        },
                    );

                    // Horizontal drag splitter between Outliner and Properties
                    let (h_rect, h_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 6.0),
                        egui::Sense::click_and_drag(),
                    );
                    if h_resp.hovered() || h_resp.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if h_resp.dragged() {
                        let delta_y = h_resp.drag_delta().y;
                        let new_ratio = (outliner_h + delta_y) / total_h.max(1.0);
                        self.editor_state.panel_settings.outliner_height_ratio =
                            new_ratio.clamp(0.15, 0.85);
                        let _ = self.editor_state.panel_settings.save();
                    }
                    let h_col = if h_resp.dragged() {
                        egui::Color32::from_rgb(240, 144, 32)
                    } else if h_resp.hovered() {
                        egui::Color32::from_rgb(255, 175, 50)
                    } else if is_light {
                        egui::Color32::from_rgb(215, 220, 230)
                    } else {
                        egui::Color32::from_rgb(45, 48, 55)
                    };
                    ui.painter().rect_filled(h_rect, 1.0, h_col);

                    egui::ScrollArea::vertical()
                        .id_salt("sidebar_properties_scroll")
                        .show(ui, |ui| {
                            render_properties(self, ui);
                        });
                });

            let new_width = panel_resp.response.rect.width();
            if (new_width - self.editor_state.panel_settings.sidebar_width).abs() > 0.5 {
                self.editor_state.panel_settings.sidebar_width = new_width;
                let _ = self.editor_state.panel_settings.save();
            }
        }

        // 6. Central 3D Viewport Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(central_bg))
            .show(ui, |ui| {
                let available_size = ui.available_size();
                let ppp = ctx.pixels_per_point();
                let width = (available_size.x * ppp).round() as u32;
                let height = (available_size.y * ppp).round() as u32;

                // A. Recreate offscreen WGPU texture if size changed
                if width >= 10
                    && height >= 10
                    && let Some(wgpu_state) = self.wgpu_render_state.clone()
                    && (self.viewport_pixel_size != (width, height)
                        || self.viewport_texture.is_none())
                {
                    let texture = wgpu_state.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Viewport Offscreen Color Texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });

                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                    let _texture_id = if let Some(existing_id) = self.viewport_egui_texture_id {
                        wgpu_state
                            .renderer
                            .write()
                            .update_egui_texture_from_wgpu_texture(
                                &wgpu_state.device,
                                &view,
                                wgpu::FilterMode::Linear,
                                existing_id,
                            );
                        existing_id
                    } else {
                        let id = wgpu_state.renderer.write().register_native_texture(
                            &wgpu_state.device,
                            &view,
                            wgpu::FilterMode::Linear,
                        );
                        self.viewport_egui_texture_id = Some(id);
                        id
                    };

                    self.viewport_texture = Some(texture);
                    self.viewport_texture_view = Some(view);
                    self.viewport_pixel_size = (width, height);
                }

                // B. Render viewport interaction and paint canvas
                render_viewport(self, ui);

                // C. Render 3D GPU Scene into offscreen texture with the freshly updated camera and scene!
                if width >= 10
                    && height >= 10
                    && let Some(wgpu_state) = self.wgpu_render_state.clone()
                    && let (Some(renderer), Some(target_view)) =
                        (&mut self.renderer, &self.viewport_texture_view)
                {
                    let edit_obj = if self.editor_state.use_gpu_wireframe
                        || self.editor_state.use_gpu_vertex_handles
                    {
                        self.active_edit_mesh.as_ref().map(|(id, m)| (id, m))
                    } else {
                        None
                    };

                    let edit_options = forge_render::EditRenderOptions {
                        draw_edges: self.editor_state.show_overlays && self.editor_state.use_gpu_wireframe,
                        draw_vertices: self.editor_state.show_overlays && self.editor_state.use_gpu_vertex_handles,
                        is_vertex_mode: self.editor_state.selection_element
                            == forge_scene::SelectionElement::Vertex,
                        active_vertex: self.editor_state.active_vertex,
                        matcap_mode: self.editor_state.viewport_matcap as u32,
                        xray: self.editor_state.show_xray,
                        xray_alpha: self.editor_state.xray_alpha,
                    };

                    renderer.render_with_edit_mesh(
                        &wgpu_state.device,
                        &wgpu_state.queue,
                        target_view,
                        width,
                        height,
                        &self.scene,
                        &self.selection,
                        &self.camera,
                        self.editor_state.show_overlays && self.editor_state.show_grid,
                        edit_obj,
                        edit_options,
                    );
                }
            });

        // 7. About Modal Dialog
        if self.show_about_dialog {
            egui::Window::new("About Forge3D Studio")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.heading("Forge3D Studio");
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Independent, open-source 3D creation suite built in Rust.");
                    ui.add_space(8.0);
                    ui.label("Key Features:");
                    ui.label("• Interactive 3D Viewport with Orbit, Pan, Zoom, Dolly");
                    ui.label("• Camera Presets (Front, Back, Right, Left, Top, Bottom)");
                    ui.label("• Orthographic & Perspective projection");
                    ui.label("• Interactive 3D Gizmos (Move, Rotate, Scale)");
                    ui.label("• Object Mode & Edit Mode (Tab)");
                    ui.label(
                        "• Vertex, Edge, Face Modeling (Extrude, Inset, Subdivide, Merge, Fill)",
                    );
                    ui.label("• Non-destructive Modifiers (Mirror, Subdivision, Solidify, Array)");
                    ui.label("• Full Undo / Redo Command History (Ctrl+Z, Ctrl+Shift+Z)");
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        self.show_about_dialog = false;
                    }
                });
        }

        // 8. GPU Diagnostics Modal Dialog
        if self.show_gpu_dialog {
            egui::Window::new("GPU Hardware Diagnostics")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.heading("Graphics Adapter Info");
                    if let Some(gpu) = &self.gpu_info {
                        ui.label(format!("Adapter Name: {}", gpu.name));
                        ui.label(format!("Active Backend: {}", gpu.backend));
                        ui.label(format!("Device Type: {}", gpu.device_type));
                        if !gpu.driver.is_empty() {
                            ui.label(format!("Driver: {}", gpu.driver));
                        }
                        if !gpu.driver_info.is_empty() {
                            ui.label(format!("Driver Details: {}", gpu.driver_info));
                        }
                    } else {
                        ui.label("No GPU adapter information available.");
                    }
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        self.show_gpu_dialog = false;
                    }
                });
        }

        // 9. Scene-backed PBR material and lighting inspector.
        if self.show_material_editor {
            let mut open = self.show_material_editor;
            let selected_id = self.selection.active_id;
            egui::Window::new("PBR Material & Lighting")
                .open(&mut open)
                .resizable(true)
                .default_size(egui::vec2(420.0, 560.0))
                .default_pos(egui::pos2(280.0, 140.0))
                .show(&ctx, |ui| {
                    let mut scene_changed = false;
                    if let Some(id) = selected_id {
                        let is_light = self
                            .scene
                            .objects
                            .get(&id)
                            .is_some_and(|object| object.light_data.is_some());
                        if is_light {
                            ui.heading("Selected Light");
                            if let Some(light) = self
                                .scene
                                .objects
                                .get_mut(&id)
                                .and_then(|object| object.light_data.as_mut())
                            {
                                egui::ComboBox::from_id_salt(("light-kind", id))
                                    .selected_text(format!("{:?}", light.kind))
                                    .show_ui(ui, |ui| {
                                        scene_changed |= ui
                                            .selectable_value(
                                                &mut light.kind,
                                                forge_scene::LightKind::Directional,
                                                "Directional",
                                            )
                                            .changed();
                                        scene_changed |= ui
                                            .selectable_value(
                                                &mut light.kind,
                                                forge_scene::LightKind::Point,
                                                "Point",
                                            )
                                            .changed();
                                        scene_changed |= ui
                                            .selectable_value(
                                                &mut light.kind,
                                                forge_scene::LightKind::Spot,
                                                "Spot",
                                            )
                                            .changed();
                                    });
                                scene_changed |= ui.checkbox(&mut light.enabled, "Enabled").changed();
                                ui.horizontal(|ui| {
                                    ui.label("Color");
                                    scene_changed |= ui.color_edit_button_rgb(&mut light.color).changed();
                                });
                                scene_changed |= ui
                                    .add(egui::Slider::new(&mut light.intensity, 0.0..=100.0).text("Intensity"))
                                    .changed();
                                if light.kind != forge_scene::LightKind::Directional {
                                    scene_changed |= ui
                                        .add(egui::Slider::new(&mut light.range, 0.1..=100.0).text("Range"))
                                        .changed();
                                }
                                if light.kind == forge_scene::LightKind::Spot {
                                    scene_changed |= ui
                                        .add(egui::Slider::new(&mut light.inner_cone_angle_deg, 0.0..=88.0).text("Inner cone"))
                                        .changed();
                                    scene_changed |= ui
                                        .add(egui::Slider::new(&mut light.outer_cone_angle_deg, 1.0..=89.0).text("Outer cone"))
                                        .changed();
                                }
                                ui.label("Point and spot falloff use inverse-square distance attenuation.");
                            }
                        } else {
                            ui.heading("Material Assignment");
                            let current_material_id = self
                                .scene
                                .objects
                                .get(&id)
                                .and_then(|object| object.material_id.clone());
                            let options: Vec<(String, String)> = self
                                .scene
                                .materials
                                .iter()
                                .map(|(material_id, material)| (material_id.clone(), material.name.clone()))
                                .collect();
                            let selected_name = current_material_id
                                .as_ref()
                                .and_then(|material_id| self.scene.materials.get(material_id))
                                .map(|material| material.name.clone())
                                .unwrap_or_else(|| "Object color only".into());
                            let mut chosen_material: Option<Option<String>> = None;
                            egui::ComboBox::from_id_salt(("object-material", id))
                                .selected_text(selected_name)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(current_material_id.is_none(), "Object color only")
                                        .clicked()
                                    {
                                        chosen_material = Some(None);
                                    }
                                    for (material_id, name) in &options {
                                        if ui
                                            .selectable_label(
                                                current_material_id.as_ref() == Some(material_id),
                                                name,
                                            )
                                            .on_hover_text(material_id)
                                            .clicked()
                                        {
                                            chosen_material = Some(Some(material_id.clone()));
                                        }
                                    }
                                });
                            if let Some(material_id) = chosen_material
                                && let Some(object) = self.scene.objects.get_mut(&id)
                            {
                                object.material_id = material_id;
                                scene_changed = true;
                            }
                            if ui.button("Create PBR material for selected object").clicked() {
                                let material_id = format!("user/material-{}", forge_core::id::ObjectId::new().raw());
                                let material = forge_scene::PbrMaterial {
                                    name: "New PBR Material".into(),
                                    ..Default::default()
                                };
                                self.scene.materials.insert(material_id.clone(), material);
                                if let Some(object) = self.scene.objects.get_mut(&id) {
                                    object.material_id = Some(material_id);
                                }
                                scene_changed = true;
                            }

                            let active_material_id = self
                                .scene
                                .objects
                                .get(&id)
                                .and_then(|object| object.material_id.clone());
                            if let Some(material_id) = active_material_id
                                && let Some(material) = self.scene.materials.get_mut(&material_id)
                            {
                                ui.separator();
                                ui.heading(&material.name);
                                scene_changed |= ui.text_edit_singleline(&mut material.name).changed();
                                ui.horizontal(|ui| {
                                    ui.label("Base color");
                                    scene_changed |= ui
                                        .color_edit_button_rgba_unmultiplied(&mut material.base_color_factor)
                                        .changed();
                                });
                                scene_changed |= ui
                                    .add(egui::Slider::new(&mut material.metallic_factor, 0.0..=1.0).text("Metallic"))
                                    .changed();
                                scene_changed |= ui
                                    .add(egui::Slider::new(&mut material.roughness_factor, 0.045..=1.0).text("Roughness"))
                                    .changed();
                                scene_changed |= ui
                                    .add(egui::Slider::new(&mut material.normal_scale, 0.0..=2.0).text("Normal strength"))
                                    .changed();
                                scene_changed |= ui
                                    .add(egui::Slider::new(&mut material.occlusion_strength, 0.0..=1.0).text("AO strength"))
                                    .changed();
                                ui.horizontal(|ui| {
                                    ui.label("Emissive");
                                    scene_changed |= ui
                                        .color_edit_button_rgb(&mut material.emissive_factor)
                                        .changed();
                                });
                                ui.horizontal(|ui| {
                                    ui.label("UV scale");
                                    scene_changed |= ui
                                        .add(egui::DragValue::new(&mut material.uv_scale[0]).speed(0.05).range(0.01..=100.0))
                                        .changed();
                                    scene_changed |= ui
                                        .add(egui::DragValue::new(&mut material.uv_scale[1]).speed(0.05).range(0.01..=100.0))
                                        .changed();
                                });
                                ui.collapsing("Texture maps", |ui| {
                                    for (label, slot) in [
                                        ("Base color", &mut material.textures.base_color),
                                        ("Normal", &mut material.textures.normal),
                                        ("Metallic", &mut material.textures.metallic),
                                        ("Roughness", &mut material.textures.roughness),
                                        ("Occlusion", &mut material.textures.ambient_occlusion),
                                        ("Emissive", &mut material.textures.emissive),
                                    ] {
                                        ui.horizontal(|ui| {
                                            ui.label(label);
                                            let mut path_value = slot.clone().unwrap_or_default();
                                            if ui.text_edit_singleline(&mut path_value).changed() {
                                                *slot = (!path_value.trim().is_empty()).then_some(path_value.clone());
                                                scene_changed = true;
                                            }
                                            if ui.button("Browse…").clicked()
                                                && let Some(path) = rfd::FileDialog::new()
                                                    .add_filter("Image", &["png", "jpg", "jpeg", "tga", "bmp"])
                                                    .pick_file()
                                            {
                                                *slot = Some(path.to_string_lossy().into_owned());
                                                scene_changed = true;
                                            }
                                        });
                                    }
                                });
                                ui.label("Color maps are read as sRGB. Normal, metallic, roughness, and AO maps are linear data.");
                            }
                        }
                    } else {
                        ui.label("Select an object to edit its material or light.");
                    }

                    ui.separator();
                    ui.collapsing("World lighting", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sky");
                            scene_changed |= ui.color_edit_button_rgb(&mut self.scene.world_lighting.sky_color).changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Ground");
                            scene_changed |= ui.color_edit_button_rgb(&mut self.scene.world_lighting.ground_color).changed();
                        });
                        scene_changed |= ui
                            .add(egui::Slider::new(&mut self.scene.world_lighting.intensity, 0.0..=3.0).text("Environment strength"))
                            .changed();
                        ui.label("Hemispheric ambient fill; direct lights are controlled by selecting a light object.");
                    });
                    if scene_changed {
                        self.scene.dirty = true;
                    }
                });
            self.show_material_editor = open;
        }

        // 9b. Interactive Shader Node Editor (WGSL)
        if self.show_shader_editor {
            let mut open = self.show_shader_editor;
            crate::ui::shader_editor::render_shader_editor(self, &ctx, &mut open);
            self.show_shader_editor = open;
        }

        // 9. Command Palette Modal Dialog (F3 / Space / Ctrl+K)
        crate::ui::command_palette::render_command_palette(self, &ctx);

        // 10. Plugin Manager Modal Dialog
        if self.show_plugin_manager {
            let mut open = self.show_plugin_manager;
            egui::Window::new("Extension & Plugin Manager")
                .open(&mut open)
                .resizable(true)
                .default_size(egui::vec2(560.0, 420.0))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.heading("Installed Extensions");
                    ui.label(
                        "Manage native and scripting extensions with runtime sandbox permissions.",
                    );
                    ui.add_space(8.0);

                    let plugin_ids: Vec<String> = self
                        .extension_manager
                        .plugins()
                        .map(|p| p.manifest.id.clone())
                        .collect();
                    for id in plugin_ids {
                        let plugin = self
                            .extension_manager
                            .plugins()
                            .find(|p| p.manifest.id == id)
                            .map(|p| (p.manifest.clone(), p.state));
                        if let Some((manifest, state)) = plugin {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let is_enabled = state == forge_scene::PluginState::Enabled;
                                    let status_text = if is_enabled {
                                        "● Enabled"
                                    } else {
                                        "○ Disabled"
                                    };
                                    let status_color = if is_enabled {
                                        egui::Color32::from_rgb(34, 197, 94)
                                    } else {
                                        egui::Color32::GRAY
                                    };
                                    ui.colored_label(status_color, status_text);
                                    ui.strong(&manifest.name);
                                    ui.label(format!("v{}", manifest.version));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let toggle_btn =
                                                if is_enabled { "Disable" } else { "Enable" };
                                            if ui.button(toggle_btn).clicked() {
                                                if is_enabled {
                                                    let _ =
                                                        self.extension_manager.disable_plugin(&id);
                                                    self.status_message = format!(
                                                        "Disabled plugin: {}",
                                                        manifest.name
                                                    );
                                                } else {
                                                    let _ =
                                                        self.extension_manager.enable_plugin(&id);
                                                    self.status_message = format!(
                                                        "Enabled plugin: {}",
                                                        manifest.name
                                                    );
                                                }
                                            }
                                        },
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&manifest.id).weak().size(11.0));
                                    ui.label("•");
                                    ui.label(
                                        egui::RichText::new(format!("By {}", manifest.author))
                                            .weak()
                                            .size(11.0),
                                    );
                                });

                                if !manifest.permissions.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Permissions:").size(11.0));
                                        for perm in &manifest.permissions {
                                            ui.label(
                                                egui::RichText::new(format!("[{}]", perm.name()))
                                                    .weak()
                                                    .size(11.0),
                                            );
                                        }
                                    });
                                }
                            });
                            ui.add_space(4.0);
                        }
                    }

                    ui.add_space(10.0);
                    ui.collapsing("Event Log & Sandbox Telemetry", |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("telemetry_event_log_scroll")
                            .max_height(100.0)
                            .show(ui, |ui| {
                                if self.extension_manager.event_logs().is_empty() {
                                    ui.label(egui::RichText::new("No event logs recorded").weak());
                                } else {
                                    for log in self.extension_manager.event_logs().iter().rev() {
                                        ui.label(egui::RichText::new(log).monospace().size(11.0));
                                    }
                                }
                            });
                    });
                });
            self.show_plugin_manager = open;
        }
    }
}

impl eframe::App for Forge3dStudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_app_ui(ui);
    }
}

/// Intersects a 3D ray with a plane defined by a point and a normal vector.
///
/// Returns `Some(hit_point)` if the ray safely intersects the plane in front of the camera,
/// or `None` if the ray is parallel, pointing away, behind the ray origin, or degenerate.
pub fn intersect_ray_plane(ray: &Ray, plane_point: Vec3, plane_normal: Vec3) -> Option<Vec3> {
    if !ray.origin.is_finite()
        || !ray.direction.is_finite()
        || !plane_point.is_finite()
        || !plane_normal.is_finite()
    {
        return None;
    }
    let normal_len_sq = plane_normal.length_squared();
    if normal_len_sq < 1e-4 {
        return None;
    }
    let normal = plane_normal / normal_len_sq.sqrt();
    let denom = normal.dot(ray.direction);
    if denom.abs() < 1e-4 {
        return None;
    }
    let t = (plane_point - ray.origin).dot(normal) / denom;
    if t <= 0.0 || t > 10000.0 || !t.is_finite() {
        return None;
    }
    let point = ray.point_at(t);
    if point.is_finite() { Some(point) } else { None }
}

/// Calculates the world-space translation delta for a ray-plane intersection.
///
/// Returns `None` if the ray is nearly parallel to the plane, behind the camera, produces non-finite values,
/// or exceeds the maximum allowable delta threshold.
pub fn calculate_drag_delta(
    ray: &Ray,
    plane_point: Vec3,
    plane_normal: Vec3,
    initial_hit_point: Vec3,
) -> Option<Vec3> {
    intersect_ray_plane(ray, plane_point, plane_normal).and_then(|current_hit| {
        let delta = current_hit - initial_hit_point;
        if delta.is_finite() && delta.length() <= 5000.0 {
            Some(delta)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_demo_reveals_stages_and_finishes_with_everything_visible() {
        let mut app = Forge3dStudioApp::new_headless();
        app.load_showcase_scene();

        let object_named = |app: &Forge3dStudioApp, name: &str| {
            app.scene
                .objects
                .values()
                .find(|object| object.name == name)
                .unwrap_or_else(|| panic!("missing showcase object: {name}"))
                .visible
        };

        assert_eq!(app.showcase_demo_elapsed, Some(0.0));
        assert!(object_named(&app, "Grassy Ground"));
        assert!(!object_named(&app, "House Foundation"));
        assert!(!object_named(&app, "Tree Trunk"));

        app.update_showcase_demo(1.4);
        assert!(object_named(&app, "House Foundation"));
        assert!(!object_named(&app, "Roof Left"));

        app.update_showcase_demo(1.4);
        assert!(object_named(&app, "Roof Left"));
        assert!(!object_named(&app, "Porch Deck"));

        app.update_showcase_demo(1.4);
        assert!(object_named(&app, "Porch Deck"));
        assert!(!object_named(&app, "Tree Trunk"));

        app.update_showcase_demo(1.4);
        assert!(object_named(&app, "Tree Trunk"));
        assert!(!object_named(&app, "Rock"));

        app.update_showcase_demo(1.4);
        assert!(object_named(&app, "Rock"));
        app.update_showcase_demo(1.5);
        assert_eq!(app.showcase_demo_elapsed, None);
        assert!(app.scene.objects.values().all(|object| object.visible));
    }

    #[test]
    fn test_mascot_pixel_alpha() {
        let decoded = image::load_from_memory(MASCOT_BYTES).expect("decoded mascot");
        let rgba = decoded.to_rgba8();
        // Check transparent corner
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        // Check opaque core pixel
        let core = rgba.get_pixel(rgba.width() / 3, rgba.height() / 2);
        assert_eq!(core[3], 255);
        // Check vibrant orange core
        assert!(core[0] > 200 && core[1] > 100);
    }

    #[test]
    fn test_pie_menu_state_and_mode_switching() {
        let mut app = Forge3dStudioApp::new_headless();
        assert_eq!(app.editor_state.edit_mode, EditMode::Object);
        assert_eq!(app.active_pie_menu, None);

        // Set pie menu state
        app.active_pie_menu = Some(PieMenuState {
            kind: PieMenuKind::Mode,
            center: egui::pos2(400.0, 300.0),
        });
        assert!(app.active_pie_menu.is_some());
        assert_eq!(app.active_pie_menu.unwrap().kind, PieMenuKind::Mode);

        // Select an object and test mode switching via set_edit_mode
        let cube_id = app.scene.objects.keys().next().copied().unwrap();
        app.selection.select(cube_id);

        app.set_edit_mode(EditMode::Edit);
        assert_eq!(app.editor_state.edit_mode, EditMode::Edit);
        assert!(app.active_edit_mesh.is_some());

        app.set_edit_mode(EditMode::Sculpt);
        assert_eq!(app.editor_state.edit_mode, EditMode::Sculpt);

        app.set_edit_mode(EditMode::Object);
        assert_eq!(app.editor_state.edit_mode, EditMode::Object);
        assert!(app.active_edit_mesh.is_none());
    }

    #[test]
    fn test_snapping_system_and_target_lifecycle() {
        let mut app = Forge3dStudioApp::new_headless();
        assert!(!app.editor_state.snap_settings.enabled);
        assert!(!app.editor_state.snap_settings.snap_to_vertex);
        assert_eq!(app.editor_state.active_snap_target, None);

        // Toggle snapping on
        app.editor_state.snap_settings.enabled = true;
        assert!(app.editor_state.snap_settings.enabled);

        // Toggle vertex snapping
        app.editor_state.snap_settings.snap_to_vertex = true;
        assert!(app.editor_state.snap_settings.snap_to_vertex);

        // Set active snap target and verify cancel clears it
        app.editor_state.active_snap_target = Some(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(app.editor_state.active_snap_target, Some(Vec3::new(1.0, 2.0, 3.0)));

        app.cancel_modal_transform();
        assert_eq!(app.editor_state.active_snap_target, None);

        // Also verify apply_modal_transform clears it
        app.editor_state.active_snap_target = Some(Vec3::new(4.0, 5.0, 6.0));
        app.apply_modal_transform();
        assert_eq!(app.editor_state.active_snap_target, None);
    }

    #[test]
    fn test_snap_cursor_and_selection_operations() {
        let mut app = Forge3dStudioApp::new_headless();
        assert_eq!(app.editor_state.cursor_3d, Vec3::ZERO);

        // Select the Cube object explicitly
        let cube_id = app
            .scene
            .objects
            .values()
            .find(|o| o.name == "Cube")
            .unwrap()
            .id;
        app.selection.select(cube_id);
        let obj_pos = app.scene.get_object(cube_id).unwrap().local_transform.translation;

        // 1. Cursor to Selected
        app.cursor_to_selected();
        assert!((app.editor_state.cursor_3d - obj_pos).length() < 1e-4);

        // 2. Cursor to World Origin
        app.cursor_to_world_origin();
        assert_eq!(app.editor_state.cursor_3d, Vec3::ZERO);

        // 3. Move 3D Cursor and Selection to Cursor
        app.editor_state.cursor_3d = Vec3::new(5.0, 10.0, 15.0);
        app.selection_to_cursor();
        let new_pos = app.scene.get_object(cube_id).unwrap().local_transform.translation;
        assert!((new_pos - Vec3::new(5.0, 10.0, 15.0)).length() < 1e-4);

        // 4. Cursor to Grid
        app.editor_state.cursor_3d = Vec3::new(2.4, 3.7, -1.2);
        app.editor_state.snap_settings.grid_increment = 1.0;
        app.cursor_to_grid();
        assert_eq!(app.editor_state.cursor_3d, Vec3::new(2.0, 4.0, -1.0));

        // 5. Selection to Grid
        app.selection_to_grid();
        let grid_pos = app.scene.get_object(cube_id).unwrap().local_transform.translation;
        assert_eq!(grid_pos, Vec3::new(5.0, 10.0, 15.0));

        // 6. Pie Menu Kind SnapCursor state
        app.active_pie_menu = Some(PieMenuState {
            kind: PieMenuKind::SnapCursor,
            center: egui::pos2(200.0, 200.0),
        });
        assert_eq!(app.active_pie_menu.unwrap().kind, PieMenuKind::SnapCursor);
    }

    #[test]
    fn test_uv_mapping_pie_menu_and_projections() {
        let mut app = Forge3dStudioApp::new_headless();

        // 1. Verify PieMenuKind::UvMapping state
        app.active_pie_menu = Some(PieMenuState {
            kind: PieMenuKind::UvMapping,
            center: egui::pos2(300.0, 300.0),
        });
        assert_eq!(app.active_pie_menu.unwrap().kind, PieMenuKind::UvMapping);

        // 2. Select Cube and test Box UV Projection
        let cube_id = app
            .scene
            .objects
            .values()
            .find(|o| o.name == "Cube")
            .unwrap()
            .id;
        app.selection.select(cube_id);

        app.box_project_selected_geometry(1.0);
        assert!(app.status_message.contains("Box UV Projection applied"));

        // 3. Test Smart UV Projection on selected geometry
        app.smart_project_selected_geometry();
        assert!(app.status_message.contains("Smart UV Projection applied"));

        // 4. Test in Edit Mode
        app.set_edit_mode(EditMode::Edit);
        app.sphere_project_selected_geometry();
        assert!(app.status_message.contains("Sphere UV Projection complete"));

        app.cylinder_project_selected_geometry(2.0);
        assert!(app.status_message.contains("Cylinder UV Projection complete"));
    }

    #[test]
    fn test_viewport_matcap_shading_lifecycle() {
        use forge_scene::ViewportMatcap;
        let mut app = Forge3dStudioApp::new_headless();
        assert_eq!(app.editor_state.viewport_matcap, ViewportMatcap::Studio);

        // Switch to Clay matcap
        app.editor_state.viewport_matcap = ViewportMatcap::Clay;
        assert_eq!(app.editor_state.viewport_matcap as u32, 1);

        // Switch to Chrome matcap
        app.editor_state.viewport_matcap = ViewportMatcap::Chrome;
        assert_eq!(app.editor_state.viewport_matcap as u32, 2);

        // Switch to Normal matcap
        app.editor_state.viewport_matcap = ViewportMatcap::Normal;
        assert_eq!(app.editor_state.viewport_matcap as u32, 3);

        // Switch to Pearl matcap
        app.editor_state.viewport_matcap = ViewportMatcap::Pearl;
        assert_eq!(app.editor_state.viewport_matcap as u32, 4);

        // Reset to Studio
        app.editor_state.viewport_matcap = ViewportMatcap::Studio;
        assert_eq!(app.editor_state.viewport_matcap as u32, 0);
    }

    #[test]
    fn test_box_select_aabb_and_selectability() {
        let mut app = Forge3dStudioApp::new_headless();

        // Verify Ground Plane is not selectable by default in viewport
        let ground_id = app.scene.objects.values().find(|o| o.name == "Ground Plane").unwrap().id;
        assert!(!app.scene.get_object(ground_id).unwrap().selectable);

        // Verify Cube and Sphere are selectable
        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;
        assert!(app.scene.get_object(cube_id).unwrap().selectable);
        assert!(app.scene.get_object(sphere_id).unwrap().selectable);

        let view_proj = app.camera.view_proj_matrix();
        let viewport_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

        // Box that encloses the entire screen
        let large_box = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        app.box_select(large_box, view_proj, viewport_rect, false);

        // Should select Cube and Sphere, but NOT Ground Plane
        assert!(app.selection.selected_ids.contains(&cube_id));
        assert!(app.selection.selected_ids.contains(&sphere_id));
        assert!(!app.selection.selected_ids.contains(&ground_id));
        assert_eq!(app.selection.selected_ids.len(), 2);

        // Test in Edit Mode
        app.selection.select(cube_id);
        app.set_edit_mode(EditMode::Edit);
        assert!(app.active_edit_mesh.is_some());

        // Perform box select on cube vertices
        app.box_select(large_box, view_proj, viewport_rect, false);
        let (_, edit_mesh) = app.active_edit_mesh.as_ref().unwrap();
        let sel_count = edit_mesh.vertices.iter().filter(|v| v.selected).count();
        assert!(sel_count > 0);
    }

    #[test]
    fn test_sculpt_mode_lifecycle_and_undo_stack() {
        use forge_geometry::{SculptBrush, SculptSettings};

        let mut app = Forge3dStudioApp::new_headless();
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;
        app.selection.select(sphere_id);

        // Enter Sculpt Mode
        app.set_edit_mode(EditMode::Sculpt);
        assert_eq!(app.editor_state.edit_mode, EditMode::Sculpt);
        assert!(app.active_edit_mesh.is_some());

        // Test configuring brush and symmetry
        app.sculpt_settings = SculptSettings {
            brush: SculptBrush::Inflate,
            radius: 1.0,
            strength: 0.8,
            symmetry_x: true,
            ..Default::default()
        };
        assert_eq!(app.sculpt_settings.brush, SculptBrush::Inflate);
        assert!(app.sculpt_settings.symmetry_x);

        // Perform stroke
        let (active_id, edit_mesh) = app.active_edit_mesh.as_mut().unwrap();
        let orig_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
        let affected = forge_geometry::apply_sculpt_stroke(
            edit_mesh,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::Y,
            Vec3::ZERO,
            &app.sculpt_settings,
        );
        assert!(affected > 0);

        let new_mesh = std::sync::Arc::new(edit_mesh.to_render_mesh());
        assert_ne!(*orig_mesh, *new_mesh);

        // Record Undo action
        app.history.push(Action::ModifyMesh {
            id: *active_id,
            old_mesh: orig_mesh.clone(),
            new_mesh: new_mesh.clone(),
            name: "Sculpt Inflate".to_string(),
        });
        assert!(app.history.can_undo());

        // Test Undo restores original mesh
        app.undo();
        let restored_obj = app.scene.get_object(sphere_id).unwrap();
        assert_eq!(restored_obj.mesh.vertices.len(), orig_mesh.vertices.len());

        // Test SnakeHook pulling deformation
        app.sculpt_settings.brush = SculptBrush::SnakeHook;
        let (_, edit_mesh) = app.active_edit_mesh.as_mut().unwrap();
        let pull_delta = Vec3::new(1.0, 2.0, 0.0);
        let affected_hook = forge_geometry::apply_sculpt_stroke(
            edit_mesh,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::Y,
            pull_delta,
            &app.sculpt_settings,
        );
        assert!(affected_hook > 0);

        // Test ElasticDeform
        app.sculpt_settings.brush = SculptBrush::ElasticDeform;
        let (_, edit_mesh) = app.active_edit_mesh.as_mut().unwrap();
        let affected_elastic = forge_geometry::apply_sculpt_stroke(
            edit_mesh,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::Y,
            Vec3::new(0.0, 0.5, 0.0),
            &app.sculpt_settings,
        );
        assert!(affected_elastic > 0);
    }

    #[test]
    fn test_frame_selected_object_and_edit_mode() {
        let mut app = Forge3dStudioApp::new_headless();
        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        app.selection.select(cube_id);

        let initial_eye = app.camera.eye;
        app.frame_selected();
        assert!(app.status_message.contains("Framed Selected"));
        assert_ne!(app.camera.eye, initial_eye);
        // Target should be centered near cube position (0, 0.5, 0)
        assert!((app.camera.target - Vec3::new(0.0, 0.5, 0.0)).length() < 0.2);

        // Test in Edit Mode with selected vertices
        app.set_edit_mode(EditMode::Edit);
        if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
            edit_mesh.vertices[0].selected = true;
            edit_mesh.vertices[1].selected = true;
        }
        app.frame_selected();
        assert!(app.status_message.contains("Framed"));
    }

    #[test]
    fn test_xray_mode_toggle_and_rendering_options() {
        let mut app = Forge3dStudioApp::new_headless();
        assert!(!app.editor_state.show_xray);
        assert_eq!(app.editor_state.xray_alpha, 0.5);

        app.toggle_xray();
        assert!(app.editor_state.show_xray);
        assert!(app.status_message.contains("X-Ray: ON"));

        app.toggle_xray();
        assert!(!app.editor_state.show_xray);
        assert!(app.status_message.contains("X-Ray: OFF"));
    }

    #[test]
    fn test_local_view_isolation_and_restoration() {
        let mut app = Forge3dStudioApp::new_headless();
        assert!(!app.is_in_local_view());

        // Without selection, should warn
        app.toggle_local_view();
        assert!(!app.is_in_local_view());
        assert!(app.status_message.contains("Select at least one object"));

        // Select cube
        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;
        app.selection.select(cube_id);

        // Enter Local View
        app.toggle_local_view();
        assert!(app.is_in_local_view());
        assert!(app.scene.get_object(cube_id).unwrap().visible);
        assert!(!app.scene.get_object(sphere_id).unwrap().visible);
        assert!(app.status_message.contains("Local View"));

        // Exit Local View -> previous visibility restored
        app.toggle_local_view();
        assert!(!app.is_in_local_view());
        assert!(app.scene.get_object(cube_id).unwrap().visible);
        assert!(app.scene.get_object(sphere_id).unwrap().visible);
        assert!(app.status_message.contains("Exited Local View"));
    }

    #[test]
    fn test_duplicate_selected_multi_object_and_edit_mode() {
        let mut app = Forge3dStudioApp::new_headless();
        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;

        // Multi-select Cube + Sphere
        app.selection.select(cube_id);
        app.selection.add_to_selection(sphere_id);
        assert_eq!(app.selection.selected_ids.len(), 2);

        let initial_obj_count = app.scene.objects.len();
        app.duplicate_selected();
        assert_eq!(app.scene.objects.len(), initial_obj_count + 2);
        assert_eq!(app.selection.selected_ids.len(), 2);
        assert!(!app.selection.selected_ids.contains(&cube_id));
        assert!(!app.selection.selected_ids.contains(&sphere_id));

        // Test in Edit Mode
        let dup_id = app.selection.selected_ids.iter().next().copied().unwrap();
        app.selection.select(dup_id);
        app.set_edit_mode(EditMode::Edit);
        let orig_verts = if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
            edit_mesh.vertices[0].selected = true;
            edit_mesh.vertices[1].selected = true;
            edit_mesh.vertices.len()
        } else {
            0
        };
        app.duplicate_selected();
        if let Some((_, edit_mesh)) = &app.active_edit_mesh {
            assert_eq!(edit_mesh.vertices.len(), orig_verts + 2);
            assert!(!edit_mesh.vertices[0].selected);
            assert!(!edit_mesh.vertices[1].selected);
            assert!(edit_mesh.vertices[orig_verts].selected);
            assert!(edit_mesh.vertices[orig_verts + 1].selected);
        }
    }

    #[test]
    fn test_viewport_overlays_toggle_and_defaults() {
        let mut app = Forge3dStudioApp::new_headless();
        assert!(app.editor_state.show_overlays);
        assert!(app.editor_state.show_cursor_3d);
        assert!(app.editor_state.show_statistics);
        assert!(app.editor_state.show_origins);

        app.toggle_overlays();
        assert!(!app.editor_state.show_overlays);
        assert!(app.status_message.contains("Viewport Overlays: OFF"));

        app.toggle_overlays();
        assert!(app.editor_state.show_overlays);
        assert!(app.status_message.contains("Viewport Overlays: ON"));
    }

    #[test]
    fn test_outliner_filter_and_selectability_toggles() {
        let mut app = Forge3dStudioApp::new_headless();

        // 1. Verify default Outliner filter settings
        assert!(app.outliner_filter.search_query.is_empty());
        assert_eq!(app.outliner_filter.type_filter, OutlinerTypeFilter::All);
        assert!(app.outliner_filter.show_selectability_column);
        assert!(app.outliner_filter.show_visibility_column);
        assert!(app.outliner_filter.show_lock_column);
        assert!(!app.outliner_filter.only_selected);
        assert!(!app.outliner_filter.only_visible);

        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;
        let cam_id = app.scene.objects.values().find(|o| o.name == "Camera").unwrap().id;

        // Default state: all match
        assert!(app.object_matches_outliner_filter(cube_id));
        assert!(app.object_matches_outliner_filter(sphere_id));
        assert!(app.object_matches_outliner_filter(cam_id));

        // 2. Search query filtering
        app.outliner_filter.search_query = "cub".to_string();
        assert!(app.object_matches_outliner_filter(cube_id));
        assert!(!app.object_matches_outliner_filter(sphere_id));
        assert!(!app.object_matches_outliner_filter(cam_id));

        // Case-insensitivity check
        app.outliner_filter.search_query = "SPHERE".to_string();
        assert!(!app.object_matches_outliner_filter(cube_id));
        assert!(app.object_matches_outliner_filter(sphere_id));

        // 3. Type filtering
        app.outliner_filter.search_query.clear();
        app.outliner_filter.type_filter = OutlinerTypeFilter::Cameras;
        assert!(app.object_matches_outliner_filter(cam_id));
        assert!(!app.object_matches_outliner_filter(cube_id));
        assert!(!app.object_matches_outliner_filter(sphere_id));

        app.outliner_filter.type_filter = OutlinerTypeFilter::Meshes;
        assert!(!app.object_matches_outliner_filter(cam_id));
        assert!(app.object_matches_outliner_filter(cube_id));
        assert!(app.object_matches_outliner_filter(sphere_id));

        // 4. Only Selected filtering
        app.outliner_filter.type_filter = OutlinerTypeFilter::All;
        app.outliner_filter.only_selected = true;
        app.selection.select(cube_id);
        assert!(app.object_matches_outliner_filter(cube_id));
        assert!(!app.object_matches_outliner_filter(sphere_id));
        assert!(!app.object_matches_outliner_filter(cam_id));

        // 5. Bulk selectability actions
        app.outliner_filter.only_selected = false;
        app.make_all_objects_selectable(false);
        for obj in app.scene.objects.values() {
            assert!(!obj.selectable);
        }
        assert!(app.status_message.contains("unselectable"));

        app.make_all_objects_selectable(true);
        for obj in app.scene.objects.values() {
            assert!(obj.selectable);
        }
        assert!(app.status_message.contains("selectable"));

        // 6. Selectability column visibility toggle
        app.toggle_outliner_selectability_column();
        assert!(!app.outliner_filter.show_selectability_column);
        assert!(app.status_message.contains("OFF"));

        app.toggle_outliner_selectability_column();
        assert!(app.outliner_filter.show_selectability_column);
        assert!(app.status_message.contains("ON"));
    }

    #[test]
    fn test_interactive_edit_mode_extrude_inset_loop_cut_and_undo() {
        let mut app = Forge3dStudioApp::new_headless();
        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        app.selection.select(cube_id);

        // Enter Edit Mode
        app.set_edit_mode(EditMode::Edit);
        assert!(app.active_edit_mesh.is_some());

        let orig_indices_count = app.scene.get_object(cube_id).unwrap().mesh.indices.len();

        // Select Face #0
        let orig_faces_count = if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
            edit_mesh.faces[0].selected = true;
            edit_mesh.faces.len()
        } else {
            0
        };

        // 1. Interactive Extrude
        let started = app.begin_interactive_edit_tool(
            InteractiveEditToolType::Extrude,
            Vec2::new(100.0, 100.0),
        );
        assert!(started);
        assert!(app.interactive_edit_tool.is_some());

        // Simulate dragging upward (y = 50.0 -> delta_y = +50.0 -> distance = 0.50m)
        app.update_interactive_edit_tool(Vec2::new(100.0, 50.0), false, false);
        assert_eq!(
            app.interactive_edit_tool.as_ref().unwrap().current_value,
            0.5
        );
        if let Some((_, edit_mesh)) = &app.active_edit_mesh {
            assert!(edit_mesh.faces.len() > orig_faces_count);
        }

        // Confirm extrusion
        app.confirm_interactive_edit_tool();
        assert!(app.interactive_edit_tool.is_none());
        assert!(app.history.can_undo());
        let post_extrude_indices = app.scene.get_object(cube_id).unwrap().mesh.indices.len();
        assert!(post_extrude_indices > orig_indices_count);

        // Undo extrude
        app.undo();
        assert_eq!(
            app.scene.get_object(cube_id).unwrap().mesh.indices.len(),
            orig_indices_count
        );

        // Redo extrude
        app.redo();
        assert_eq!(
            app.scene.get_object(cube_id).unwrap().mesh.indices.len(),
            post_extrude_indices
        );

        // 2. Interactive Inset and Cancel
        if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
            edit_mesh.faces[0].selected = true;
        }
        let pre_inset_indices = app.scene.get_object(cube_id).unwrap().mesh.indices.len();
        let started_inset =
            app.begin_interactive_edit_tool(InteractiveEditToolType::Inset, Vec2::new(200.0, 200.0));
        assert!(started_inset);

        // Drag mouse
        app.update_interactive_edit_tool(Vec2::new(200.0, 160.0), false, false);
        // Cancel should revert exactly to pre-inset state
        app.cancel_interactive_edit_tool();
        assert!(app.interactive_edit_tool.is_none());
        assert_eq!(
            app.scene.get_object(cube_id).unwrap().mesh.indices.len(),
            pre_inset_indices
        );

        // 3. Interactive Loop Cut
        let pre_loop_indices = app.scene.get_object(cube_id).unwrap().mesh.indices.len();
        if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
            edit_mesh.edges[0].selected = true;
        }
        let started_loop = app.begin_interactive_edit_tool(
            InteractiveEditToolType::LoopCut,
            Vec2::new(300.0, 300.0),
        );
        assert!(started_loop);

        // Slide along X
        app.update_interactive_edit_tool(Vec2::new(350.0, 300.0), false, false);
        let factor = app.interactive_edit_tool.as_ref().unwrap().current_value;
        assert!(factor > 0.5);

        // Confirm
        app.confirm_interactive_edit_tool();
        assert!(app.interactive_edit_tool.is_none());
        assert!(app.scene.get_object(cube_id).unwrap().mesh.indices.len() > pre_loop_indices);

        // Undo loop cut
        app.undo();
        assert_eq!(
            app.scene.get_object(cube_id).unwrap().mesh.indices.len(),
            pre_loop_indices
        );
    }

    #[test]
    fn test_navigation_toolbar_and_viewport_collapsible_widget() {
        let mut app = Forge3dStudioApp::new_headless();

        // Defaults: navigation widget visible in viewport but collapsed into a compact [Nav ▸] header
        assert!(app.editor_state.show_nav_widget);
        assert!(!app.nav_widget_expanded);

        // Toggle expanded state
        app.nav_widget_expanded = true;
        assert!(app.nav_widget_expanded);

        // Toggle visibility via toggle_nav_widget
        app.toggle_nav_widget();
        assert!(!app.editor_state.show_nav_widget);
        assert_eq!(app.status_message, "Navigation Widget: OFF");

        app.toggle_nav_widget();
        assert!(app.editor_state.show_nav_widget);
        assert_eq!(app.status_message, "Navigation Widget: ON");
    }

    #[test]
    fn test_multi_object_box_select_and_drag_manipulation() {
        use forge_geometry::primitives::generate_capsule;
        use forge_math::Transform;
        use forge_scene::{PrimitiveType, SceneObject};

        let mut app = Forge3dStudioApp::new_headless();

        // 1. Add Capsule to scene
        let capsule_mesh = generate_capsule(0.5, 1.0, 16, 8);
        let capsule = SceneObject::new("Capsule", PrimitiveType::Capsule, capsule_mesh)
            .with_transform(Transform::from_translation(Vec3::new(2.0, 1.0, 0.0)));
        let capsule_id = app.scene.add_object(capsule);

        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;
        let ground_id = app.scene.objects.values().find(|o| o.name == "Ground Plane").unwrap().id;

        let view_proj = app.camera.view_proj_matrix();
        let viewport_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

        // 2. Box select covering foreground objects (Cube, Sphere, Capsule)
        let large_box = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        app.box_select(large_box, view_proj, viewport_rect, false);

        // Ground Plane is not selectable; Cube, Sphere, Capsule must all be selected!
        assert_eq!(app.selection.selected_ids.len(), 3);
        assert!(app.selection.is_selected(cube_id));
        assert!(app.selection.is_selected(sphere_id));
        assert!(app.selection.is_selected(capsule_id));
        assert!(!app.selection.is_selected(ground_id));

        // 3. Shift-select toggle: deselect sphere
        app.selection.toggle_selection(sphere_id);
        assert_eq!(app.selection.selected_ids.len(), 2);
        assert!(!app.selection.is_selected(sphere_id));
        assert!(app.selection.is_selected(cube_id));
        assert!(app.selection.is_selected(capsule_id));

        // Shift-select toggle: re-select sphere
        app.selection.toggle_selection(sphere_id);
        assert_eq!(app.selection.selected_ids.len(), 3);
        assert!(app.selection.is_selected(sphere_id));

        // 4. Test multi-object free drag manipulation
        let cube_initial_pos = app.scene.get_object(cube_id).unwrap().local_transform.translation;
        let sphere_initial_pos = app.scene.get_object(sphere_id).unwrap().local_transform.translation;
        let capsule_initial_pos = app.scene.get_object(capsule_id).unwrap().local_transform.translation;

        let hit_point = Vec3::new(0.0, 0.5, 0.0);
        app.start_object_drag(hit_point, cube_id, Vec2::new(500.0, 500.0), true);

        let delta = Vec3::new(3.0, 1.5, -2.0);
        // Apply delta to drag state
        if let Some(drag) = &mut app.object_drag_state {
            for (&id, &init_tf) in &drag.initial_transforms {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.local_transform.translation = init_tf.translation + delta;
                }
            }
        }
        let drag_clone = app.object_drag_state.take().unwrap();
        app.commit_object_drag(&drag_clone);

        // All 3 selected objects must have translated by delta
        assert_eq!(
            app.scene.get_object(cube_id).unwrap().local_transform.translation,
            cube_initial_pos + delta
        );
        assert_eq!(
            app.scene.get_object(sphere_id).unwrap().local_transform.translation,
            sphere_initial_pos + delta
        );
        assert_eq!(
            app.scene.get_object(capsule_id).unwrap().local_transform.translation,
            capsule_initial_pos + delta
        );

        // 5. Test Undo: reverts all 3 objects in a single history step
        app.undo();
        assert_eq!(
            app.scene.get_object(cube_id).unwrap().local_transform.translation,
            cube_initial_pos
        );
        assert_eq!(
            app.scene.get_object(sphere_id).unwrap().local_transform.translation,
            sphere_initial_pos
        );
        assert_eq!(
            app.scene.get_object(capsule_id).unwrap().local_transform.translation,
            capsule_initial_pos
        );
    }

    #[test]
    fn test_shader_editor_lifecycle_and_material_sync() {
        let mut app = Forge3dStudioApp::new_headless();
        assert!(!app.show_shader_editor);

        // 1. Select the default Cube
        let cube_id = app
            .scene
            .objects
            .values()
            .find(|o| o.name == "Cube")
            .map(|o| o.id)
            .expect("Cube should exist");
        app.selection.select(cube_id);
        assert_eq!(app.selection.active_id, Some(cube_id));

        // 2. Open Shader Node Editor
        app.open_shader_editor_for_selected();
        assert!(app.show_shader_editor);
        assert!(!app.active_shader_graph.nodes.is_empty());

        // 3. Modify Principled BSDF inputs in graph
        let bsdf_node = app
            .active_shader_graph
            .nodes
            .iter_mut()
            .find(|n| matches!(n.kind, forge_scene::NodeKind::PrincipledBsdf))
            .expect("Principled BSDF node should exist");

        if let Some(metal) = bsdf_node.inputs.iter_mut().find(|i| i.id == "metallic") {
            metal.default_value = forge_scene::SocketDefaultValue::Float(0.9);
        }
        if let Some(rough) = bsdf_node.inputs.iter_mut().find(|i| i.id == "roughness") {
            rough.default_value = forge_scene::SocketDefaultValue::Float(0.12);
        }

        // 4. Synchronize graph with viewport material
        app.sync_active_shader_graph_to_material();

        let obj = app.scene.get_object(cube_id).unwrap();
        let mat_id = obj.material_id.as_ref().expect("Material ID should be assigned");
        let mat = app.scene.materials.get(mat_id).expect("Material should be in scene");

        assert_eq!(mat.metallic_factor, 0.9);
        assert_eq!(mat.roughness_factor, 0.12);
        assert!(mat.shader_graph.is_some());
        assert!(app.scene.dirty);

        // 5. Verify WGSL compilation
        let wgsl = app.active_shader_graph.compile_to_wgsl();
        assert!(wgsl.contains("struct MaterialUniform"));
        assert!(wgsl.contains("distribution_ggx"));
        assert!(wgsl.contains("fresnel_schlick"));
        assert!(wgsl.contains("fn fs_main("));
    }

    #[test]
    fn test_lasso_select_and_circle_select_modes() {
        let mut app = Forge3dStudioApp::new_headless();

        // 1. Point in polygon basic test
        let triangle = [
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 0.0),
            egui::pos2(50.0, 100.0),
        ];
        assert!(point_in_polygon(egui::pos2(50.0, 30.0), &triangle));
        assert!(!point_in_polygon(egui::pos2(150.0, 30.0), &triangle));
        assert!(!point_in_polygon(egui::pos2(-10.0, 30.0), &triangle));

        // 2. Lasso select in Object Mode
        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;
        let ground_id = app.scene.objects.values().find(|o| o.name == "Ground Plane").unwrap().id;

        let view_proj = app.camera.view_proj_matrix();
        let viewport_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

        // Quad covering the whole viewport
        let whole_screen_polygon = [
            egui::pos2(0.0, 0.0),
            egui::pos2(1920.0, 0.0),
            egui::pos2(1920.0, 1080.0),
            egui::pos2(0.0, 1080.0),
        ];

        app.lasso_select(&whole_screen_polygon, view_proj, viewport_rect, false);
        assert!(app.selection.selected_ids.contains(&cube_id));
        assert!(app.selection.selected_ids.contains(&sphere_id));
        assert!(!app.selection.selected_ids.contains(&ground_id));

        // 3. Circle select in Object Mode
        app.deselect_all();
        assert_eq!(app.selection.selected_ids.len(), 0);

        let cube_tf = app.scene.world_transform(cube_id);
        let cube_screen_p = crate::ui::gizmo::world_to_screen_pos(cube_tf.translation, view_proj, viewport_rect)
            .expect("Cube should project to screen");

        app.circle_select(cube_screen_p, 100.0, view_proj, viewport_rect, false);
        assert!(app.selection.selected_ids.contains(&cube_id));

        // Deselect with circle select
        app.circle_select(cube_screen_p, 100.0, view_proj, viewport_rect, true);
        assert!(!app.selection.selected_ids.contains(&cube_id));

        // 4. Circle select in Edit Mode
        app.selection.select(cube_id);
        app.set_edit_mode(EditMode::Edit);
        assert!(app.active_edit_mesh.is_some());

        // Select all vertices with a large circle
        app.circle_select(cube_screen_p, 500.0, view_proj, viewport_rect, false);
        let (_, edit_mesh) = app.active_edit_mesh.as_ref().unwrap();
        assert!(edit_mesh.vertices.iter().all(|v| v.selected));

        // Deselect all vertices with a large circle
        app.circle_select(cube_screen_p, 500.0, view_proj, viewport_rect, true);
        let (_, edit_mesh) = app.active_edit_mesh.as_ref().unwrap();
        assert!(edit_mesh.vertices.iter().all(|v| !v.selected));
    }

    #[test]
    fn test_select_linked_edit_mode() {
        let mut app = Forge3dStudioApp::new_headless();

        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        let sphere_id = app.scene.objects.values().find(|o| o.name == "Sphere").unwrap().id;

        // Join Sphere into Cube to create a two-island mesh
        app.selection.select(cube_id);
        app.selection.add_to_selection(sphere_id);
        app.join_selected();

        let joined_id = app.selection.active_id.expect("Joined object should be active");

        // Enter Edit Mode
        app.set_edit_mode(EditMode::Edit);
        assert!(app.active_edit_mesh.is_some());
        assert_eq!(app.active_edit_mesh.as_ref().map(|(id, _)| *id), Some(joined_id));

        let total_verts = app.active_edit_mesh.as_ref().unwrap().1.vertices.len();

        // Deselect all vertices first
        if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
            for v in &mut edit_mesh.vertices {
                v.selected = false;
            }
            // Select only vertex 0 (which belongs to the cube island)
            edit_mesh.vertices[0].selected = true;
        }

        // Run select linked
        app.select_linked(false);

        let (_, edit_mesh) = app.active_edit_mesh.as_ref().unwrap();
        let selected_count = edit_mesh.vertices.iter().filter(|v| v.selected).count();

        // More than 1 vertex selected (the whole cube island), but not all vertices of both islands
        assert!(selected_count > 1);
        assert!(selected_count < total_verts);

        // Deselect linked
        app.select_linked(true);
        let (_, edit_mesh) = app.active_edit_mesh.as_ref().unwrap();
        let selected_count_after = edit_mesh.vertices.iter().filter(|v| v.selected).count();
        assert_eq!(selected_count_after, 0);
    }

    #[test]
    fn test_voxel_remesh_lifecycle_and_undo() {
        let mut app = Forge3dStudioApp::new_headless();

        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        app.selection.select(cube_id);

        let initial_vert_count = app.scene.get_object(cube_id).unwrap().mesh.vertices.len();

        // 1. Remesh in Object Mode
        app.voxel_remesh_active(0.35);

        let remeshed_vert_count = app.scene.get_object(cube_id).unwrap().mesh.vertices.len();
        assert!(remeshed_vert_count > 0);
        assert_ne!(remeshed_vert_count, initial_vert_count);

        // 2. Undo returns to initial mesh
        app.undo();
        let undone_vert_count = app.scene.get_object(cube_id).unwrap().mesh.vertices.len();
        assert_eq!(undone_vert_count, initial_vert_count);

        // 3. Redo returns to remeshed mesh
        app.redo();
        let redone_vert_count = app.scene.get_object(cube_id).unwrap().mesh.vertices.len();
        assert_eq!(redone_vert_count, remeshed_vert_count);

        // 4. Test in Sculpt Mode updates active_edit_mesh
        app.set_edit_mode(EditMode::Sculpt);
        assert!(app.active_edit_mesh.is_some());

        app.voxel_remesh_active(0.4);
        let (active_mesh_id, ref edit_mesh) = *app.active_edit_mesh.as_ref().unwrap();
        assert_eq!(active_mesh_id, cube_id);
        assert_eq!(edit_mesh.vertices.len(), app.scene.get_object(cube_id).unwrap().mesh.vertices.len());
    }

    #[test]
    fn test_modifier_stack_lifecycle_and_undo() {
        let mut app = Forge3dStudioApp::new_headless();

        let cube_id = app.scene.objects.values().find(|o| o.name == "Cube").unwrap().id;
        app.selection.select(cube_id);

        let initial_base_verts = app.scene.get_object(cube_id).unwrap().mesh.vertices.len();

        // 1. Add Subdivide modifier
        app.add_modifier(
            cube_id,
            ModifierType::Subdivide(forge_modifiers::SubdivisionModifier::new(1)),
        );
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.modifiers.len(), 1);
        assert_eq!(obj.mesh.vertices.len(), initial_base_verts); // base mesh untouched
        assert!(obj.evaluated_mesh().vertices.len() > initial_base_verts); // evaluated mesh subdivided

        // 2. Duplicate modifier
        app.duplicate_modifier(cube_id, 0);
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.modifiers.len(), 2);
        assert_eq!(obj.modifiers.modifiers[1].name, "Subdivision Surface (Copy)");

        // 3. Move modifiers up / down
        app.move_modifier_up(cube_id, 1);
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.modifiers.modifiers[0].name, "Subdivision Surface (Copy)");

        // 4. In Edit Mode, applying modifier is disallowed
        app.set_edit_mode(EditMode::Edit);
        assert!(!app.apply_modifier(cube_id, 0));
        assert_eq!(app.scene.get_object(cube_id).unwrap().modifiers.len(), 2);

        // 5. Back in Object Mode, apply first modifier with undo
        app.set_edit_mode(EditMode::Object);
        assert!(app.apply_modifier(cube_id, 0));
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.modifiers.len(), 1);
        let baked_verts = obj.mesh.vertices.len();
        assert!(baked_verts > initial_base_verts);

        // 6. Undo restores base mesh and modifier can be redone
        app.undo();
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.mesh.vertices.len(), initial_base_verts);

        app.redo();
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.mesh.vertices.len(), baked_verts);

        // 7. Apply All Modifiers
        app.add_modifier(cube_id, ModifierType::Solidify(forge_modifiers::SolidifyModifier::default()));
        assert!(app.apply_all_modifiers(cube_id));
        let obj = app.scene.get_object(cube_id).unwrap();
        assert_eq!(obj.modifiers.len(), 0);

        // 8. Clear all
        app.add_modifier(cube_id, ModifierType::Mirror(forge_modifiers::MirrorModifier::default()));
        assert_eq!(app.scene.get_object(cube_id).unwrap().modifiers.len(), 1);
        app.clear_all_modifiers(cube_id);
        assert_eq!(app.scene.get_object(cube_id).unwrap().modifiers.len(), 0);
    }
}
