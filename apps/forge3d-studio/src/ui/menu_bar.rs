use egui::Ui;
use forge_geometry::primitives::{
    generate_capsule, generate_circle, generate_cone, generate_cube, generate_cylinder,
    generate_icosphere, generate_monkey, generate_plane, generate_torus, generate_uv_sphere,
};
use forge_math::{Transform, Vec3};
use forge_render::{CameraProjection, ViewPreset};
use forge_scene::{Action, EditMode, ForgeProject, PrimitiveType, Scene, SceneObject};

use crate::app::Forge3dStudioApp;

pub fn render_menu_bar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.label(
            egui::RichText::new("Forge3D")
                .strong()
                .size(13.0)
                .color(egui::Color32::from_rgb(59, 130, 246)),
        );
        ui.add_space(2.0);
        // ---------------- File Menu ----------------
        ui.menu_button("File", |ui| {
            if ui.button("New Scene").clicked() {
                app.scene = Scene::create_default_scene();
                app.selection.deselect();
                app.history.clear();
                app.status_message = "Created new scene".to_string();
                ui.close();
            }

            ui.separator();

            if ui.button("Open (.forge)...").clicked() {
                let default_path = "scene.forge";
                match std::fs::read_to_string(default_path) {
                    Ok(json) => match ForgeProject::from_json(&json) {
                        Ok(project) => {
                            app.scene = project.scene;
                            app.selection.deselect();
                            app.history.clear();
                            if let Some(r) = &mut app.renderer {
                                r.invalidate_all_meshes();
                            }
                            app.status_message = format!("Loaded project from {default_path}");
                        }
                        Err(e) => {
                            app.status_message = format!("Error loading project: {e}");
                        }
                    },
                    Err(e) => {
                        app.status_message = format!("Could not open {default_path}: {e}");
                    }
                }
                ui.close();
            }

            if ui.button("Save Project (.forge)").clicked() {
                let default_path = "scene.forge";
                let project = ForgeProject::new(&app.scene.name, app.scene.clone());
                match project.to_json() {
                    Ok(json) => match std::fs::write(default_path, json) {
                        Ok(_) => {
                            app.status_message = format!("Saved project to {default_path}");
                        }
                        Err(e) => {
                            app.status_message = format!("Write error: {e}");
                        }
                    },
                    Err(e) => {
                        app.status_message = format!("Serialization error: {e}");
                    }
                }
                ui.close();
            }

            ui.separator();

            ui.menu_button("Import", |ui| {
                if ui.button("Wavefront (.obj)...").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Wavefront OBJ", &["obj"])
                        .set_title("Import Wavefront OBJ")
                        .pick_file()
                        && let Err(e) = app.import_obj_from_path(&path)
                    {
                        app.status_message = format!("Import error: {e}");
                    }
                }
            });

            ui.menu_button("Export", |ui| {
                if ui.button("Wavefront (.obj)...").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Wavefront OBJ", &["obj"])
                        .set_file_name("scene.obj")
                        .set_title("Export Wavefront OBJ")
                        .save_file()
                        && let Err(e) = app.export_obj_to_path(&path, false)
                    {
                        app.status_message = format!("Export error: {e}");
                    }
                }
                if ui.button("Wavefront (.obj) [Selected]...").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Wavefront OBJ", &["obj"])
                        .set_file_name("selected.obj")
                        .set_title("Export Selected to Wavefront OBJ")
                        .save_file()
                        && let Err(e) = app.export_obj_to_path(&path, true)
                    {
                        app.status_message = format!("Export error: {e}");
                    }
                }
            });

            ui.separator();

            if ui.button("Exit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        // ---------------- Edit Menu ----------------
        ui.menu_button("Edit", |ui| {
            let undo_label = if let Some(desc) = app.history.last_undo_description() {
                format!("Undo: {desc} (Ctrl+Z)")
            } else {
                "Undo (Ctrl+Z)".to_string()
            };
            if ui
                .add_enabled(app.history.can_undo(), egui::Button::new(undo_label))
                .clicked()
            {
                app.undo();
                ui.close();
            }

            let redo_label = if let Some(desc) = app.history.last_redo_description() {
                format!("Redo: {desc} (Ctrl+Shift+Z)")
            } else {
                "Redo (Ctrl+Shift+Z)".to_string()
            };
            if ui
                .add_enabled(app.history.can_redo(), egui::Button::new(redo_label))
                .clicked()
            {
                app.redo();
                ui.close();
            }

            ui.separator();

            let has_sel = !app.selection.is_empty();
            if ui
                .add_enabled(has_sel, egui::Button::new("Duplicate (Shift+D)"))
                .clicked()
            {
                app.duplicate_selected();
                ui.close();
            }

            if ui
                .add_enabled(has_sel, egui::Button::new("Delete (Del)"))
                .clicked()
            {
                app.delete_selected();
                ui.close();
            }

            ui.separator();

            let mode_text = match app.editor_state.edit_mode {
                EditMode::Object => "Switch to Edit Mode (Tab)",
                EditMode::Edit => "Switch to Object Mode (Tab)",
            };
            if ui.button(mode_text).clicked() {
                app.toggle_edit_mode();
                ui.close();
            }
        });

        // ---------------- Create Menu ----------------
        ui.menu_button("Create", |ui| {
            let cursor = app.editor_state.cursor_3d;

            // Search filter field
            let search_id = egui::Id::NULL.with("add_menu_search_query");
            let mut search_buf =
                ui.data_mut(|d| d.get_temp_mut_or_default::<String>(search_id).clone());

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Search:").size(11.0).weak());
                ui.add(
                    egui::TextEdit::singleline(&mut search_buf)
                        .hint_text("Search primitives...")
                        .desired_width(140.0),
                );
                if !search_buf.is_empty() && ui.small_button("Clear").clicked() {
                    search_buf.clear();
                }
            });
            ui.data_mut(|d| {
                d.insert_temp(search_id, search_buf.clone());
            });

            let query = search_buf.trim().to_lowercase();
            let matches =
                |name: &str| -> bool { query.is_empty() || name.to_lowercase().contains(&query) };

            ui.separator();

            let show_mesh_header = matches("Cube")
                || matches("UV Sphere")
                || matches("Icosphere")
                || matches("Cylinder")
                || matches("Cone")
                || matches("Torus")
                || matches("Capsule")
                || matches("Plane")
                || matches("Grid")
                || matches("Circle")
                || matches("Suzanne")
                || matches("Monkey");

            if show_mesh_header {
                ui.label(egui::RichText::new("Mesh Primitives").weak().size(11.0));

                if matches("Cube") && ui.button("Cube").clicked() {
                    let mesh = generate_cube(1.0);
                    let id = app.scene.add_object(
                        SceneObject::new("Cube", PrimitiveType::Cube, mesh)
                            .with_color([0.85, 0.45, 0.25, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.5, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Cube".to_string();
                    ui.close();
                }

                if matches("UV Sphere") && ui.button("UV Sphere").clicked() {
                    let mesh = generate_uv_sphere(0.75, 32, 16);
                    let id = app.scene.add_object(
                        SceneObject::new("UV Sphere", PrimitiveType::Sphere, mesh)
                            .with_color([0.25, 0.65, 0.85, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.75, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added UV Sphere".to_string();
                    ui.close();
                }

                if matches("Icosphere") && ui.button("Icosphere").clicked() {
                    let mesh = generate_icosphere(0.75, 2);
                    let id = app.scene.add_object(
                        SceneObject::new("Icosphere", PrimitiveType::Icosphere, mesh)
                            .with_color([0.45, 0.75, 0.45, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.75, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Icosphere".to_string();
                    ui.close();
                }

                if matches("Cylinder") && ui.button("Cylinder").clicked() {
                    let mesh = generate_cylinder(0.6, 1.5, 24);
                    let id = app.scene.add_object(
                        SceneObject::new("Cylinder", PrimitiveType::Cylinder, mesh)
                            .with_color([0.75, 0.75, 0.35, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.75, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Cylinder".to_string();
                    ui.close();
                }

                if matches("Cone") && ui.button("Cone").clicked() {
                    let mesh = generate_cone(0.8, 1.5, 24);
                    let id = app.scene.add_object(
                        SceneObject::new("Cone", PrimitiveType::Cone, mesh)
                            .with_color([0.85, 0.55, 0.35, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.75, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Cone".to_string();
                    ui.close();
                }

                if matches("Torus") && ui.button("Torus").clicked() {
                    let mesh = generate_torus(1.0, 0.3, 24, 16);
                    let id = app.scene.add_object(
                        SceneObject::new("Torus", PrimitiveType::Torus, mesh)
                            .with_color([0.85, 0.35, 0.65, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.5, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Torus".to_string();
                    ui.close();
                }

                if matches("Capsule") && ui.button("Capsule").clicked() {
                    let mesh = generate_capsule(0.5, 1.0, 24, 8);
                    let id = app.scene.add_object(
                        SceneObject::new("Capsule", PrimitiveType::Capsule, mesh)
                            .with_color([0.85, 0.65, 0.25, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 1.0, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Capsule".to_string();
                    ui.close();
                }

                if (matches("Plane") || matches("Grid")) && ui.button("Plane / Grid").clicked() {
                    let mesh = generate_plane(8.0, 8);
                    let id = app.scene.add_object(
                        SceneObject::new("Plane", PrimitiveType::Plane, mesh)
                            .with_color([0.28, 0.30, 0.34, 1.0])
                            .with_transform(Transform::from_translation(cursor)),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Plane Grid".to_string();
                    ui.close();
                }

                if matches("Circle") && ui.button("Circle").clicked() {
                    let mesh = generate_circle(1.0, 32);
                    let id = app.scene.add_object(
                        SceneObject::new("Circle", PrimitiveType::Circle, mesh)
                            .with_color([0.5, 0.7, 0.8, 1.0])
                            .with_transform(Transform::from_translation(cursor)),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Circle".to_string();
                    ui.close();
                }

                if (matches("Suzanne") || matches("Monkey"))
                    && ui.button("Suzanne (Monkey)").clicked()
                {
                    let mesh = generate_monkey(1.0);
                    let id = app.scene.add_object(
                        SceneObject::new("Suzanne", PrimitiveType::Monkey, mesh)
                            .with_color([0.78, 0.52, 0.32, 1.0])
                            .with_transform(Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.75, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Suzanne (Monkey Mesh)".to_string();
                    ui.close();
                }
            }

            if matches("Camera") {
                ui.separator();
                ui.label(egui::RichText::new("Scene Objects").weak().size(11.0));
                if ui.button("Camera").clicked() {
                    let mut camera_obj = SceneObject::new_camera("Camera");
                    camera_obj.local_transform.translation = cursor + Vec3::new(0.0, 1.5, 3.0);
                    let id = app.scene.add_object(camera_obj);
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject {
                            object: obj.clone(),
                        });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Camera".to_string();
                    ui.close();
                }
            }
        });

        ui.menu_button("Examples", |ui| {
            ui.label(egui::RichText::new("Ready-made scenes").weak().size(11.0));
            if ui.button("House, Tree & Rock").clicked() {
                app.load_showcase_scene();
                ui.close();
            }
        });

        // ---------------- Tools Menu ----------------
        ui.menu_button("Tools", |ui| {
            let has_sel = !app.selection.is_empty();

            if ui
                .add_enabled(has_sel, egui::Button::new("Apply Transform"))
                .clicked()
            {
                app.apply_transform_selected();
                ui.close();
            }

            ui.separator();

            ui.menu_button("Parent", |ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Make Parent to Active (Ctrl+P)"))
                    .clicked()
                {
                    app.parent_selected_to_active();
                    ui.close();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Clear Parent (Alt+P)"))
                    .clicked()
                {
                    app.unparent_selected();
                    ui.close();
                }
            });

            if ui
                .add_enabled(
                    app.selection.selected_ids.len() > 1,
                    egui::Button::new("Join Objects (Ctrl+J)"),
                )
                .clicked()
            {
                app.join_selected();
                ui.close();
            }

            ui.menu_button("Set Origin", |ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Origin to Geometry"))
                    .clicked()
                {
                    app.set_origin_selected_geometry();
                    ui.close();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Origin to 3D Cursor"))
                    .clicked()
                {
                    app.set_origin_selected_cursor();
                    ui.close();
                }
            });

            ui.separator();

            ui.menu_button("Show / Hide", |ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Hide Selected (H)"))
                    .clicked()
                {
                    app.hide_selected();
                    ui.close();
                }
                if ui.button("Unhide All (Alt+H)").clicked() {
                    app.unhide_all();
                    ui.close();
                }
            });

            ui.separator();

            if ui
                .add_enabled(has_sel, egui::Button::new("Reset Position (Alt+G)"))
                .clicked()
            {
                if let Some(id) = app.selection.active_id
                    && let Some(obj) = app.scene.get_object_mut(id)
                {
                    let old = obj.local_transform;
                    obj.reset_translation();
                    app.history.push(Action::Transform {
                        id,
                        old_transform: old,
                        new_transform: obj.local_transform,
                    });
                }
                ui.close();
            }

            if ui
                .add_enabled(has_sel, egui::Button::new("Reset Rotation (Alt+R)"))
                .clicked()
            {
                if let Some(id) = app.selection.active_id
                    && let Some(obj) = app.scene.get_object_mut(id)
                {
                    let old = obj.local_transform;
                    obj.reset_rotation();
                    app.history.push(Action::Transform {
                        id,
                        old_transform: old,
                        new_transform: obj.local_transform,
                    });
                }
                ui.close();
            }

            if ui
                .add_enabled(has_sel, egui::Button::new("Reset Scale (Alt+S)"))
                .clicked()
            {
                if let Some(id) = app.selection.active_id
                    && let Some(obj) = app.scene.get_object_mut(id)
                {
                    let old = obj.local_transform;
                    obj.reset_scale();
                    app.history.push(Action::Transform {
                        id,
                        old_transform: old,
                        new_transform: obj.local_transform,
                    });
                }
                ui.close();
            }
        });

        // ---------------- Mesh Menu ----------------
        ui.menu_button("Mesh", |ui| {
            if ui.button("Extrude (E)").clicked() {
                if app.editor_state.edit_mode != EditMode::Edit {
                    app.toggle_edit_mode();
                }
                app.extrude_selected_geometry(0.5);
                ui.close();
            }

            if ui.button("Inset (I)").clicked() {
                if app.editor_state.edit_mode != EditMode::Edit {
                    app.toggle_edit_mode();
                }
                app.inset_selected_geometry(0.15);
                ui.close();
            }

            if ui.button("Subdivide").clicked() {
                if app.editor_state.edit_mode != EditMode::Edit {
                    app.toggle_edit_mode();
                }
                app.subdivide_selected_geometry();
                ui.close();
            }

            if ui.button("Merge at Center (M)").clicked() {
                if app.editor_state.edit_mode != EditMode::Edit {
                    app.toggle_edit_mode();
                }
                app.merge_selected_geometry();
                ui.close();
            }

            if ui.button("Fill Face/Edge (F)").clicked() {
                if app.editor_state.edit_mode != EditMode::Edit {
                    app.toggle_edit_mode();
                }
                app.fill_selected_geometry();
                ui.close();
            }

            ui.separator();

            if ui.button("Delete Selected").clicked() {
                app.delete_selected();
                ui.close();
            }
        });

        // ---------------- Render Menu ----------------
        ui.menu_button("Render", |ui| {
            if ui.button("Live Viewer (WGPU Viewport)").clicked() {
                app.status_message = "Live Viewer Active".to_string();
                ui.close();
            }
            if ui.button("Render Image (F12)").clicked() {
                app.status_message = "Render Frame Complete".to_string();
                ui.close();
            }
            ui.separator();
            ui.label(egui::RichText::new("Octane / PBR Engine").weak().size(11.0));
            if ui.button("Viewport Shading: Lit PBR").clicked() {
                app.status_message = "PBR Forward Shading Active".to_string();
                ui.close();
            }
        });

        // ---------------- Plugins Menu ----------------
        ui.menu_button("Plugins", |ui| {
            if ui.button("Plugin Manager...").clicked() {
                app.status_message = "Plugins: 0 loaded".to_string();
                ui.close();
            }
            if ui.button("Cinema 4D / Octane Bridge").clicked() {
                app.status_message = "Bridge Ready".to_string();
                ui.close();
            }
        });

        // ---------------- Windows Menu ----------------
        ui.menu_button("Windows", |ui| {
            let cam_view_text = if app.editor_state.is_camera_view {
                "Exit Camera View (Numpad 0)"
            } else {
                "Camera View (Numpad 0)"
            };
            if ui.button(cam_view_text).clicked() {
                app.toggle_camera_view();
                ui.close();
            }

            if ui
                .button("Align Active Camera to View (Ctrl+Alt+Numpad 0)")
                .clicked()
            {
                app.align_active_camera_to_view();
                ui.close();
            }

            ui.separator();

            let fly_text = if app.editor_state.fly_nav_active {
                "Exit Fly Navigation (Esc)"
            } else {
                "Fly Navigation (Shift+~ / Shift+F)"
            };
            if ui.button(fly_text).clicked() {
                app.toggle_fly_navigation();
                ui.close();
            }

            ui.separator();

            ui.menu_button("Standard Viewpoints", |ui| {
                if ui.button("Front (Numpad 1)").clicked() {
                    app.camera.set_view_preset(ViewPreset::Front);
                    ui.close();
                }
                if ui.button("Back (Ctrl+Numpad 1)").clicked() {
                    app.camera.set_view_preset(ViewPreset::Back);
                    ui.close();
                }
                if ui.button("Right (Numpad 3)").clicked() {
                    app.camera.set_view_preset(ViewPreset::Right);
                    ui.close();
                }
                if ui.button("Left (Ctrl+Numpad 3)").clicked() {
                    app.camera.set_view_preset(ViewPreset::Left);
                    ui.close();
                }
                if ui.button("Top (Numpad 7)").clicked() {
                    app.camera.set_view_preset(ViewPreset::Top);
                    ui.close();
                }
                if ui.button("Bottom (Ctrl+Numpad 7)").clicked() {
                    app.camera.set_view_preset(ViewPreset::Bottom);
                    ui.close();
                }
            });

            let proj_label = match app.camera.projection {
                CameraProjection::Perspective => {
                    "Projection: Perspective → Switch to Orthographic (Numpad 5)"
                }
                CameraProjection::Orthographic => {
                    "Projection: Orthographic → Switch to Perspective (Numpad 5)"
                }
            };
            if ui.button(proj_label).clicked() {
                app.camera.toggle_projection();
                ui.close();
            }

            ui.separator();

            if ui.button("Frame Selected (F / Numpad .)").clicked() {
                app.frame_selected();
                ui.close();
            }

            if ui.button("Frame All (Home)").clicked() {
                app.frame_all();
                ui.close();
            }

            if ui.button("Reset View").clicked() {
                app.camera.reset_view();
                app.editor_state.is_camera_view = false;
                ui.close();
            }

            ui.separator();

            ui.checkbox(
                &mut app.editor_state.emulate_numpad,
                "Emulate Numpad (Top Row 1-0)",
            );

            ui.checkbox(
                &mut app.editor_state.show_debug_info,
                "Show Viewport Debug Telemetry",
            );
            ui.checkbox(&mut app.editor_state.walk_mode, "Walk Mode (Lock Ground Y)");
            ui.checkbox(&mut app.editor_state.show_grid, "Show Ground Grid");
            ui.checkbox(&mut app.editor_state.show_axes, "Show Coordinate Axes");
            ui.checkbox(&mut app.editor_state.show_wireframe, "Show Wireframe");
            ui.checkbox(
                &mut app.editor_state.use_gpu_wireframe,
                "GPU Edit Wireframe",
            )
            .on_hover_text(
                "Render Edit Mode mesh edges directly via GPU pipeline instead of CPU overlay",
            );
            ui.checkbox(
                &mut app.editor_state.use_gpu_vertex_handles,
                "GPU Edit Vertex Handles",
            )
            .on_hover_text("Render Edit Mode vertex markers directly via GPU instanced pipeline");
            ui.checkbox(
                &mut app.editor_state.use_gpu_picking,
                "GPU Picking (ID Buffer)",
            )
            .on_hover_text(
                "Use hardware ID buffer for sub-millisecond picking instead of CPU raycasting",
            );

            ui.separator();
            ui.label(egui::RichText::new("Viewport Zoom").weak().size(11.0));
            if ui
                .checkbox(
                    &mut app.editor_state.panel_settings.zoom_to_cursor,
                    "Zoom to Cursor",
                )
                .on_hover_text("Zoom towards the 3D surface or focus plane under the mouse cursor")
                .changed()
            {
                app.camera.settings.zoom_to_cursor = app.editor_state.panel_settings.zoom_to_cursor;
                let _ = app.editor_state.panel_settings.save();
            }
            if ui
                .checkbox(
                    &mut app.editor_state.panel_settings.smooth_zoom,
                    "Smooth Zoom",
                )
                .on_hover_text(
                    "Interpolate camera distance smoothly using frame-rate independent easing",
                )
                .changed()
            {
                app.camera.settings.smooth_zoom = app.editor_state.panel_settings.smooth_zoom;
                if !app.camera.settings.smooth_zoom {
                    app.camera.sync_zoom_targets();
                }
                let _ = app.editor_state.panel_settings.save();
            }
            ui.horizontal(|ui| {
                ui.label("Zoom Sensitivity:");
                let mut sens = app.editor_state.panel_settings.zoom_sensitivity;
                if ui
                    .add(
                        egui::Slider::new(&mut sens, 0.1..=3.0)
                            .step_by(0.1)
                            .text("x"),
                    )
                    .on_hover_text("Adjust mouse wheel zoom sensitivity multiplier (default 1.0x)")
                    .changed()
                {
                    app.editor_state.panel_settings.zoom_sensitivity = sens;
                    app.camera.settings.zoom_sensitivity = sens;
                    let _ = app.editor_state.panel_settings.save();
                }
            });

            ui.separator();
            ui.label(egui::RichText::new("Workspace Panels").weak().size(11.0));
            if ui
                .checkbox(
                    &mut app.editor_state.panel_settings.toolbar_visible,
                    "Show Left Toolbar",
                )
                .changed()
            {
                let _ = app.editor_state.panel_settings.save();
            }
            if ui
                .checkbox(
                    &mut app.editor_state.panel_settings.sidebar_visible,
                    "Show Right Sidebar",
                )
                .changed()
            {
                let _ = app.editor_state.panel_settings.save();
            }
            if ui
                .checkbox(
                    &mut app.editor_state.panel_settings.timeline_visible,
                    "Show Timeline",
                )
                .changed()
            {
                let _ = app.editor_state.panel_settings.save();
            }
            ui.checkbox(
                &mut app.show_material_editor,
                "Show Material Inspector (Octane Style)",
            );
            if ui.button("Reset Workspace Layout").clicked() {
                app.editor_state.panel_settings = forge_scene::PanelSettings::default();
                let _ = app.editor_state.panel_settings.save();
                app.status_message = "Reset workspace layout to defaults".to_string();
                ui.close();
            }
        });

        // ---------------- Help Menu ----------------
        ui.menu_button("Help", |ui| {
            if ui.button("About Forge3D").clicked() {
                app.show_about_dialog = true;
                ui.close();
            }

            if ui.button("GPU Diagnostics").clicked() {
                app.show_gpu_dialog = true;
                ui.close();
            }
        });

        // ---------------- Right-aligned Window Controls & Layout Selector ----------------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);

            // Close button with hover highlight
            let close_resp = ui.add(
                egui::Button::new(
                    egui::RichText::new("✕")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(180, 190, 210)),
                )
                .fill(egui::Color32::TRANSPARENT)
                .corner_radius(egui::CornerRadius::same(3))
                .min_size(egui::vec2(24.0, 20.0)),
            );
            if close_resp.clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Maximize / Restore button
            let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            let max_symbol = if is_maximized { "⧉" } else { "▢" };
            let max_resp = ui.add(
                egui::Button::new(
                    egui::RichText::new(max_symbol)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(180, 190, 210)),
                )
                .fill(egui::Color32::TRANSPARENT)
                .corner_radius(egui::CornerRadius::same(3))
                .min_size(egui::vec2(24.0, 20.0)),
            );
            if max_resp.clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            }

            // Minimize button
            let min_resp = ui.add(
                egui::Button::new(
                    egui::RichText::new("—")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(180, 190, 210)),
                )
                .fill(egui::Color32::TRANSPARENT)
                .corner_radius(egui::CornerRadius::same(3))
                .min_size(egui::vec2(24.0, 20.0)),
            );
            if min_resp.clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Layout Mode Selector
            egui::ComboBox::from_id_salt("layout_mode_selector")
                .selected_text("Layout: Model ▾")
                .show_ui(ui, |ui| {
                    let _ = ui.selectable_label(true, "Model");
                    let _ = ui.selectable_label(false, "Animate");
                    let _ = ui.selectable_label(false, "Sculpt");
                    let _ = ui.selectable_label(false, "Render / UV");
                });

            // Draggable window title space in the center
            let remaining_w = ui.available_width();
            if remaining_w > 10.0 {
                let (drag_rect, drag_resp) = ui.allocate_exact_size(
                    egui::vec2(remaining_w, 20.0),
                    egui::Sense::click_and_drag(),
                );
                if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_resp.double_clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }
                ui.painter().text(
                    drag_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Forge3D Studio",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(110, 118, 135),
                );
            }
        });
    });

    // ---------------- Secondary Quick Tool Shelf ----------------
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.0, 2.0);

        // Undo / Redo
        if ui
            .add_enabled(app.history.can_undo(), egui::Button::new("Undo"))
            .on_hover_text("Undo (Ctrl+Z)")
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(app.history.can_redo(), egui::Button::new("Redo"))
            .on_hover_text("Redo (Ctrl+Shift+Z)")
            .clicked()
        {
            app.redo();
        }
        ui.separator();

        // Selection modes
        let is_pointer = app.editor_state.transform_mode != forge_scene::TransformMode::BoxSelect;
        if ui
            .selectable_label(is_pointer, "Select")
            .on_hover_text("Pointer Selection (LMB Click/Drag)")
            .clicked()
        {
            app.editor_state.transform_mode = forge_scene::TransformMode::Translate;
        }
        let is_box = app.editor_state.transform_mode == forge_scene::TransformMode::BoxSelect;
        if ui
            .selectable_label(is_box, "Box")
            .on_hover_text("Box Selection (B)")
            .clicked()
        {
            app.editor_state.transform_mode = forge_scene::TransformMode::BoxSelect;
        }
        ui.separator();

        // Transform Gizmo modes
        let is_trans = app.editor_state.active_gizmo_mode == forge_scene::GizmoMode::Translate;
        if ui
            .selectable_label(is_trans, "Move")
            .on_hover_text("Translate Tool (G / Move Gizmo)")
            .clicked()
        {
            app.editor_state.active_gizmo_mode = forge_scene::GizmoMode::Translate;
            app.editor_state.transform_mode = forge_scene::TransformMode::Translate;
        }
        let is_rot = app.editor_state.active_gizmo_mode == forge_scene::GizmoMode::Rotate;
        if ui
            .selectable_label(is_rot, "Rotate")
            .on_hover_text("Rotate Tool (R / Rotate Gizmo)")
            .clicked()
        {
            app.editor_state.active_gizmo_mode = forge_scene::GizmoMode::Rotate;
            app.editor_state.transform_mode = forge_scene::TransformMode::Rotate;
        }
        let is_scale = app.editor_state.active_gizmo_mode == forge_scene::GizmoMode::Scale;
        if ui
            .selectable_label(is_scale, "Scale")
            .on_hover_text("Scale Tool (S / Scale Gizmo)")
            .clicked()
        {
            app.editor_state.active_gizmo_mode = forge_scene::GizmoMode::Scale;
            app.editor_state.transform_mode = forge_scene::TransformMode::Scale;
        }
        ui.separator();

        // Primitives Quick Add
        let cursor = app.editor_state.cursor_3d;
        if ui
            .button("Cube")
            .on_hover_text("Add Cube at 3D Cursor")
            .clicked()
        {
            let mesh = generate_cube(1.0);
            let id = app.scene.add_object(
                SceneObject::new("Cube", PrimitiveType::Cube, mesh)
                    .with_color([0.35, 0.65, 0.95, 1.0])
                    .with_transform(Transform::from_translation(
                        cursor + Vec3::new(0.0, 0.5, 0.0),
                    )),
            );
            if let Some(obj) = app.scene.get_object(id) {
                app.history.push(Action::AddObject {
                    object: obj.clone(),
                });
            }
            app.selection.select(id);
            app.status_message = "Added Cube".to_string();
        }
        if ui
            .button("Sphere")
            .on_hover_text("Add UV Sphere at 3D Cursor")
            .clicked()
        {
            let mesh = generate_uv_sphere(0.75, 24, 16);
            let id = app.scene.add_object(
                SceneObject::new("Sphere", PrimitiveType::Sphere, mesh)
                    .with_color([0.90, 0.45, 0.35, 1.0])
                    .with_transform(Transform::from_translation(
                        cursor + Vec3::new(0.0, 0.75, 0.0),
                    )),
            );
            if let Some(obj) = app.scene.get_object(id) {
                app.history.push(Action::AddObject {
                    object: obj.clone(),
                });
            }
            app.selection.select(id);
            app.status_message = "Added UV Sphere".to_string();
        }
        if ui
            .button("Light")
            .on_hover_text("Add Light Object")
            .clicked()
        {
            let mesh = generate_uv_sphere(0.25, 12, 8);
            let id = app.scene.add_object(
                SceneObject::new("Light", PrimitiveType::Sphere, mesh)
                    .with_color([1.0, 0.95, 0.70, 1.0])
                    .with_transform(Transform::from_translation(
                        cursor + Vec3::new(2.0, 4.0, 2.0),
                    )),
            );
            if let Some(obj) = app.scene.get_object(id) {
                app.history.push(Action::AddObject {
                    object: obj.clone(),
                });
            }
            app.selection.select(id);
            app.status_message = "Added Light".to_string();
        }
        if ui
            .button("Camera")
            .on_hover_text("Add Camera Object")
            .clicked()
        {
            let mesh = generate_cone(0.4, 0.6, 12);
            let mut cam_obj = SceneObject::new("Camera", PrimitiveType::Camera, mesh)
                .with_color([0.2, 0.8, 0.4, 1.0])
                .with_transform(Transform::from_translation(
                    cursor + Vec3::new(0.0, 2.0, 4.0),
                ));
            cam_obj.camera_data = Some(forge_scene::CameraData::default());
            let id = app.scene.add_object(cam_obj);
            if let Some(obj) = app.scene.get_object(id) {
                app.history.push(Action::AddObject {
                    object: obj.clone(),
                });
            }
            app.selection.select(id);
            app.status_message = "Added Camera".to_string();
        }
        ui.separator();

        // Snapping & Grid Toggles
        let snap_on = app.editor_state.snap_settings.enabled;
        if ui
            .selectable_label(snap_on, "Snap")
            .on_hover_text("Toggle Snapping to Grid")
            .clicked()
        {
            app.editor_state.snap_settings.enabled = !snap_on;
        }
        let grid_on = app.editor_state.show_grid;
        if ui
            .selectable_label(grid_on, "Grid")
            .on_hover_text("Toggle Viewport Ground Grid")
            .clicked()
        {
            app.editor_state.show_grid = !grid_on;
        }
        ui.separator();

        // Right-aligned Live Viewer status pill
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Live Viewer")
                            .color(egui::Color32::from_rgb(180, 220, 255)),
                    )
                    .fill(egui::Color32::from_rgb(28, 38, 56)),
                )
                .on_hover_text("Cinema 4D / Octane Live Viewport Renderer")
                .clicked()
            {
                app.status_message = "Live Viewer Active (WGPU Viewport)".to_string();
            }
        });
    });
}
