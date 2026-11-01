use std::time::Instant;

use eframe::egui;
use forge_core::id::ObjectId;
use forge_geometry::{EditSelectionMode, EditableMesh};
use forge_gpu::GpuAdapterInfo;
use forge_math::{Ray, Transform, Vec2, Vec3};
use forge_render::{Camera, ViewportRenderer};
use forge_scene::{Action, EditMode, EditorState, HistoryStack, Scene, SelectionState};

use crate::ui::gizmo::GizmoState;
use crate::ui::menu_bar::render_menu_bar;
use crate::ui::outliner::render_outliner;
use crate::ui::properties::render_properties;
use crate::ui::status_bar::render_status_bar;
use crate::ui::toolbar::render_toolbar;
use crate::ui::viewport::render_viewport;

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

    // Undo / Redo Command History
    pub history: HistoryStack,

    // Interactive 3D Viewport Gizmo
    pub gizmo_state: GizmoState,

    // Active mesh in Edit Mode: (ObjectId, EditableMesh)
    pub active_edit_mesh: Option<(ObjectId, EditableMesh)>,

    // Stored WGPU render state from CreationContext
    pub wgpu_render_state: Option<egui_wgpu::RenderState>,

    // Offscreen 3D viewport render target
    pub viewport_texture: Option<wgpu::Texture>,
    pub viewport_texture_view: Option<wgpu::TextureView>,
    pub viewport_egui_texture_id: Option<egui::TextureId>,
    pub viewport_pixel_size: (u32, u32),

    // Performance metrics
    last_frame_instant: Instant,
    pub fps: f32,
    pub frame_time_ms: f32,

    // Multi-object transformation initial transforms cache
    pub gizmo_initial_transforms: std::collections::HashMap<ObjectId, Transform>,

    // Free cursor object dragging state
    pub object_drag_state: Option<ObjectDragState>,
}

impl Forge3dStudioApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // Set distinctive Dark Graphite Visual styling (#1a1c21 base)
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(26, 28, 33);
        visuals.window_fill = egui::Color32::from_rgb(30, 33, 38);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(36, 39, 46);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(42, 45, 54);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 56, 68);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(64, 70, 84);
        visuals.selection.bg_fill = egui::Color32::from_rgb(220, 120, 30); // Warm Forge amber
        cc.egui_ctx.set_visuals(visuals);

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

        let editor_state = EditorState {
            panel_settings: forge_scene::PanelSettings::load_or_default(),
            ..Default::default()
        };

        Self {
            scene: Scene::create_default_scene(),
            selection: SelectionState::new(),
            editor_state,
            camera: Camera::default(),
            renderer,
            gpu_info,
            status_message: "Ready. Welcome to Forge3D Rust Edition.".to_string(),
            show_about_dialog: false,
            show_gpu_dialog: false,
            history: HistoryStack::new(),
            gizmo_state: GizmoState::new(),
            active_edit_mesh: None,
            wgpu_render_state: cc.wgpu_render_state.clone(),
            viewport_texture: None,
            viewport_texture_view: None,
            viewport_egui_texture_id: None,
            viewport_pixel_size: (0, 0),
            last_frame_instant: Instant::now(),
            fps: 60.0,
            frame_time_ms: 16.6,
            gizmo_initial_transforms: std::collections::HashMap::new(),
            object_drag_state: None,
        }
    }

    /// Creates a headless instance of the app (no GPU/eframe required) for automated integration testing.
    pub fn new_headless() -> Self {
        Self {
            scene: Scene::create_default_scene(),
            selection: SelectionState::new(),
            editor_state: EditorState::default(),
            camera: Camera::default(),
            renderer: None,
            gpu_info: None,
            status_message: "Ready (Headless)".to_string(),
            show_about_dialog: false,
            show_gpu_dialog: false,
            history: HistoryStack::new(),
            gizmo_state: GizmoState::new(),
            active_edit_mesh: None,
            wgpu_render_state: None,
            viewport_texture: None,
            viewport_texture_view: None,
            viewport_egui_texture_id: None,
            viewport_pixel_size: (1280, 800),
            last_frame_instant: Instant::now(),
            fps: 60.0,
            frame_time_ms: 16.6,
            gizmo_initial_transforms: std::collections::HashMap::new(),
            object_drag_state: None,
        }
    }

    /// Toggles between Object Mode and Edit Mode.
    pub fn toggle_edit_mode(&mut self) {
        match self.editor_state.edit_mode {
            EditMode::Object => {
                if let Some(active_id) = self.selection.active_id {
                    if let Some(obj) = self.scene.get_object(active_id) {
                        let edit_mesh = EditableMesh::from_mesh(&obj.mesh);
                        self.active_edit_mesh = Some((active_id, edit_mesh));
                        self.editor_state.edit_mode = EditMode::Edit;
                        self.status_message = format!("Edit Mode: {}", obj.name);
                    }
                } else {
                    self.status_message = "Select an object to enter Edit Mode.".to_string();
                }
            }
            EditMode::Edit => {
                self.commit_edit_mesh_to_scene();
                self.active_edit_mesh = None;
                self.editor_state.edit_mode = EditMode::Object;
                self.status_message = "Object Mode".to_string();
            }
        }
    }

    /// Commits changes in `active_edit_mesh` back to the scene object and marks scene dirty.
    pub fn commit_edit_mesh_to_scene(&mut self) {
        if let Some((id, edit_mesh)) = &self.active_edit_mesh
            && let Some(obj) = self.scene.get_object_mut(*id)
        {
            let old_mesh = obj.mesh.clone();
            let new_mesh = edit_mesh.to_render_mesh();
            obj.mesh = new_mesh.clone();
            self.scene.dirty = true;

            if let Some(r) = &mut self.renderer {
                r.invalidate_mesh(id);
            }

            self.history.push(Action::ModifyMesh {
                id: *id,
                old_mesh,
                new_mesh,
                name: "Edit Mesh Geometry".to_string(),
            });
        }
    }

    /// Fast sync of edit mesh to scene object during interactive operations without pushing undo step.
    pub fn sync_edit_mesh_to_scene(&mut self) {
        if let Some((id, edit_mesh)) = &self.active_edit_mesh
            && let Some(obj) = self.scene.get_object_mut(*id)
        {
            obj.mesh = edit_mesh.to_render_mesh();
            self.scene.dirty = true;
        }
    }

    pub fn extrude_selected_geometry(&mut self, distance: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            let res = edit_mesh.extrude_faces(distance);
            if !res.is_empty() {
                self.sync_edit_mesh_to_scene();
                self.status_message = format!("Extruded {} face(s)", res.len());
            }
        }
    }

    pub fn inset_selected_geometry(&mut self, amount: f32) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            let res = edit_mesh.inset_faces(amount);
            if !res.is_empty() {
                self.sync_edit_mesh_to_scene();
                self.status_message = format!("Inset {} face(s)", res.len());
            }
        }
    }

    pub fn subdivide_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            edit_mesh.subdivide();
            self.sync_edit_mesh_to_scene();
            self.status_message = "Subdivided selected faces".to_string();
        }
    }

    pub fn merge_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
            edit_mesh.merge_selected_at_center();
            self.sync_edit_mesh_to_scene();
            self.status_message = "Merged selected vertices at center".to_string();
        }
    }

    pub fn fill_selected_geometry(&mut self) {
        if let Some((_, edit_mesh)) = &mut self.active_edit_mesh
            && let Some(idx) = edit_mesh.fill_face_from_selection()
        {
            self.sync_edit_mesh_to_scene();
            self.status_message = format!("Created Face #{}", idx);
        }
    }

    pub fn delete_selected(&mut self) {
        match self.editor_state.edit_mode {
            EditMode::Object => {
                let to_delete: Vec<ObjectId> =
                    self.selection.selected_ids.iter().copied().collect();
                for id in to_delete {
                    if let Some(removed) = self.scene.remove_object(id) {
                        self.history.push(Action::DeleteObject { object: removed });
                    }
                }
                self.selection.deselect();
                self.status_message = "Deleted selected object(s)".to_string();
            }
            EditMode::Edit => {
                if let Some((_, edit_mesh)) = &mut self.active_edit_mesh {
                    let sel_mode = match self.editor_state.selection_element {
                        forge_scene::SelectionElement::Vertex => EditSelectionMode::Vertex,
                        forge_scene::SelectionElement::Edge => EditSelectionMode::Edge,
                        forge_scene::SelectionElement::Face => EditSelectionMode::Face,
                    };
                    edit_mesh.delete_selected(sel_mode);
                    self.sync_edit_mesh_to_scene();
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

    pub fn duplicate_selected(&mut self) {
        if let Some(active_id) = self.selection.active_id
            && let Some(new_id) = self.scene.duplicate_object(active_id)
        {
            if let Some(dup) = self.scene.get_object(new_id) {
                self.history.push(Action::AddObject {
                    object: dup.clone(),
                });
            }
            self.selection.select(new_id);
            self.status_message = "Duplicated object".to_string();
        }
    }

    pub fn apply_transform_selected(&mut self) {
        if let Some(active_id) = self.selection.active_id
            && let Some(obj) = self.scene.get_object(active_id)
        {
            let old_transform = obj.local_transform;
            let old_mesh = obj.mesh.clone();
            self.scene.apply_transform_to_object(active_id);
            self.history.push(Action::ApplyTransform {
                id: active_id,
                old_transform,
                old_mesh,
            });
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

    pub fn box_select(
        &mut self,
        box_rect: egui::Rect,
        view_proj: forge_math::Mat4,
        viewport_rect: egui::Rect,
        toggle: bool,
    ) {
        let mut hit_ids = std::collections::HashSet::new();
        for (&id, obj) in &self.scene.objects {
            if !obj.visible {
                continue;
            }
            let world_tf = self.scene.world_transform(id);
            let world_pos = world_tf.translation;
            if let Some(screen_p) =
                crate::ui::gizmo::world_to_screen_pos(world_pos, view_proj, viewport_rect)
                && box_rect.contains(screen_p)
            {
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

    pub fn undo(&mut self) {
        if let Some(desc) = self.history.undo(&mut self.scene) {
            self.status_message = format!("Undo: {desc}");
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
        } else {
            self.status_message = "Nothing to undo".to_string();
        }
    }

    pub fn redo(&mut self) {
        if let Some(desc) = self.history.redo(&mut self.scene) {
            self.status_message = format!("Redo: {desc}");
            if let Some(r) = &mut self.renderer {
                r.invalidate_all_meshes();
            }
        } else {
            self.status_message = "Nothing to redo".to_string();
        }
    }

    pub fn frame_selected(&mut self) {
        let bbox = self.scene.selected_aabb(&self.selection);
        self.camera.frame_box(&bbox);
        self.status_message = "Framed Selected".to_string();
    }

    pub fn frame_all(&mut self) {
        let bbox = self.scene.scene_aabb();
        self.camera.frame_box(&bbox);
        self.status_message = "Framed All".to_string();
    }
    pub fn render_app_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        // 1. Measure frame delta and FPS
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;
        if dt > 0.0 {
            let current_fps = 1.0 / dt;
            self.fps = self.fps * 0.9 + current_fps * 0.1;
            self.frame_time_ms = dt * 1000.0;
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
        egui::Panel::top("menu_bar").show(ui, |ui| {
            render_menu_bar(self, ui);
        });

        // 3. Bottom Status Bar
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            render_status_bar(self, ui);
        });

        // 4. Left Toolbar (Resizable with drag handle and auto mode switch)
        if self.editor_state.panel_settings.toolbar_visible {
            let toolbar_w = self.editor_state.panel_settings.toolbar_width;
            egui::Panel::left("tools_panel")
                .resizable(false)
                .exact_size(toolbar_w)
                .show(ui, |ui| {
                    let total_h = ui.available_height();
                    let content_w = (toolbar_w - 6.0).max(10.0);
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(content_w, total_h),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                render_toolbar(self, ui);
                            },
                        );

                        // Vertical drag splitter on right edge
                        let (splitter_rect, response) = ui.allocate_exact_size(
                            egui::vec2(6.0, total_h),
                            egui::Sense::click_and_drag(),
                        );
                        if response.hovered() || response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                        if response.dragged() {
                            let delta_x = response.drag_delta().x;
                            let new_w = (self.editor_state.panel_settings.toolbar_width + delta_x)
                                .clamp(48.0, 240.0);
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
                        let col = if response.dragged() {
                            egui::Color32::from_rgb(240, 144, 32) // #f09020 accent
                        } else if response.hovered() {
                            egui::Color32::from_rgb(255, 175, 50)
                        } else {
                            egui::Color32::from_rgb(38, 42, 50)
                        };
                        ui.painter().rect_filled(splitter_rect, 1.0, col);
                    });
                });
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

                    ui.horizontal(|ui| {
                        ui.label("⏱ Timeline");
                        ui.separator();
                        if ui
                            .button("⏮")
                            .on_hover_text("First Frame (Shift+Left)")
                            .clicked()
                        {
                            self.status_message = "Timeline: Jump to Frame 1".to_string();
                        }
                        if ui
                            .button("▶")
                            .on_hover_text("Play / Pause (Space)")
                            .clicked()
                        {
                            self.status_message = "Timeline: Play / Pause".to_string();
                        }
                        if ui
                            .button("⏭")
                            .on_hover_text("Last Frame (Shift+Right)")
                            .clicked()
                        {
                            self.status_message = "Timeline: Jump to End".to_string();
                        }
                        ui.separator();
                        ui.label("Frame: 1 / 250");
                    });
                });
        }

        // 6. Right Sidebar: Outliner (Top) and Properties (Bottom)
        if self.editor_state.panel_settings.sidebar_visible {
            let sidebar_w = self.editor_state.panel_settings.sidebar_width;
            egui::Panel::right("outliner_properties_panel")
                .resizable(false)
                .exact_size(sidebar_w)
                .show(ui, |ui| {
                    let total_h = ui.available_height();
                    let content_w = (sidebar_w - 6.0).max(10.0);
                    ui.horizontal(|ui| {
                        // Vertical drag splitter on left edge of right panel
                        let (splitter_rect, response) = ui.allocate_exact_size(
                            egui::vec2(6.0, total_h),
                            egui::Sense::click_and_drag(),
                        );
                        if response.hovered() || response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                        if response.dragged() {
                            let delta_x = response.drag_delta().x;
                            // Dragging left (negative delta) increases sidebar width
                            self.editor_state.panel_settings.sidebar_width =
                                (self.editor_state.panel_settings.sidebar_width - delta_x)
                                    .clamp(220.0, 600.0);
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

                        ui.allocate_ui_with_layout(
                            egui::vec2(content_w, total_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
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
                                } else {
                                    egui::Color32::from_rgb(45, 48, 55)
                                };
                                ui.painter().rect_filled(h_rect, 1.0, h_col);

                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    render_properties(self, ui);
                                });
                            },
                        );
                    });
                });
        }

        // 6. Central 3D Viewport Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(20, 22, 26)))
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
                    renderer.render(
                        &wgpu_state.device,
                        &wgpu_state.queue,
                        target_view,
                        width,
                        height,
                        &self.scene,
                        &self.selection,
                        &self.camera,
                        self.editor_state.show_grid,
                    );
                }
            });

        // 7. About Modal Dialog
        if self.show_about_dialog {
            egui::Window::new("About Forge3D Rust Edition")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.heading("Forge3D Rust Edition");
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
