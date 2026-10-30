use std::time::Instant;

use eframe::egui;
use forge_core::id::ObjectId;
use forge_geometry::{EditSelectionMode, EditableMesh};
use forge_gpu::GpuAdapterInfo;
use forge_render::{Camera, ViewportRenderer};
use forge_scene::{Action, EditMode, EditorState, HistoryStack, Scene, SelectionState};

use crate::ui::gizmo::GizmoState;
use crate::ui::menu_bar::render_menu_bar;
use crate::ui::outliner::render_outliner;
use crate::ui::properties::render_properties;
use crate::ui::status_bar::render_status_bar;
use crate::ui::toolbar::render_toolbar;
use crate::ui::viewport::render_viewport;

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

        Self {
            scene: Scene::create_default_scene(),
            selection: SelectionState::new(),
            editor_state: EditorState::default(),
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
}

impl eframe::App for Forge3dStudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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

        // 2. Top Menu Bar
        egui::Panel::top("menu_bar").show(ui, |ui| {
            render_menu_bar(self, ui);
        });

        // 3. Bottom Status Bar
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            render_status_bar(self, ui);
        });

        // 4. Left Toolbar
        egui::Panel::left("tools_panel")
            .resizable(false)
            .exact_size(68.0)
            .show(ui, |ui| {
                render_toolbar(self, ui);
            });

        // 5. Right Sidebar: Outliner (Top) and Properties (Bottom)
        egui::Panel::right("outliner_properties_panel")
            .default_size(300.0)
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height() * 0.40),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        render_outliner(self, ui);
                    },
                );

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_properties(self, ui);
                });
            });

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
