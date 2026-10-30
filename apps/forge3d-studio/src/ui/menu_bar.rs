use egui::Ui;
use forge_geometry::primitives::{generate_cube, generate_plane, generate_uv_sphere};
use forge_math::{Transform, Vec3};
use forge_scene::{ForgeProject, PrimitiveType, Scene, SceneObject};

use crate::app::Forge3dStudioApp;

pub fn render_menu_bar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        // File Menu
        ui.menu_button("File", |ui| {
            if ui.button("📄 New Scene").clicked() {
                app.scene = Scene::create_default_scene();
                app.selection.deselect();
                app.status_message = "Created new scene".to_string();
                ui.close();
            }

            ui.separator();

            if ui.button("📂 Open (.forge)...").clicked() {
                let default_path = "scene.forge";
                match std::fs::read_to_string(default_path) {
                    Ok(json) => match ForgeProject::from_json(&json) {
                        Ok(project) => {
                            app.scene = project.scene;
                            app.selection.deselect();
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

            if ui.button("💾 Save Project (.forge)").clicked() {
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

            if ui.button("❌ Exit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        // Add Menu
        ui.menu_button("Add", |ui| {
            if ui.button("📦 Cube").clicked() {
                let mesh = generate_cube(1.0);
                let id = app.scene.add_object(
                    SceneObject::new("Cube", PrimitiveType::Cube, mesh)
                        .with_color([0.85, 0.45, 0.25, 1.0])
                        .with_transform(Transform::from_translation(Vec3::new(0.0, 0.5, 0.0))),
                );
                app.selection.select(id);
                app.status_message = "Added Cube".to_string();
                ui.close();
            }

            if ui.button("🌐 UV Sphere").clicked() {
                let mesh = generate_uv_sphere(0.65, 32, 16);
                let id = app.scene.add_object(
                    SceneObject::new("Sphere", PrimitiveType::Sphere, mesh)
                        .with_color([0.25, 0.65, 0.85, 1.0])
                        .with_transform(Transform::from_translation(Vec3::new(0.0, 0.65, 0.0))),
                );
                app.selection.select(id);
                app.status_message = "Added UV Sphere".to_string();
                ui.close();
            }

            if ui.button("⬛ Plane").clicked() {
                let mesh = generate_plane(10.0, 10);
                let id = app.scene.add_object(
                    SceneObject::new("Plane", PrimitiveType::Plane, mesh)
                        .with_color([0.28, 0.30, 0.34, 1.0])
                        .with_transform(Transform::IDENTITY),
                );
                app.selection.select(id);
                app.status_message = "Added Plane".to_string();
                ui.close();
            }
        });

        // View Menu
        ui.menu_button("View", |ui| {
            if ui.button("🔄 Reset Camera").clicked() {
                app.camera = forge_render::Camera::default();
                app.status_message = "Camera reset".to_string();
                ui.close();
            }

            if ui.button("🎯 Frame Selected").clicked() {
                if let Some(id) = app.selection.active_id {
                    let world_tf = app.scene.world_transform(id);
                    app.camera.target = world_tf.translation;
                    let offset = (app.camera.eye - app.camera.target).normalize_or_zero() * 4.0;
                    app.camera.eye = app.camera.target + offset;
                    app.status_message = "Framed selected object".to_string();
                }
                ui.close();
            }

            ui.separator();

            ui.checkbox(&mut app.editor_state.show_grid, "Show Ground Grid");
            ui.checkbox(&mut app.editor_state.show_axes, "Show Coordinate Axes");
        });

        // Help Menu
        ui.menu_button("Help", |ui| {
            if ui.button("ℹ About Forge3D").clicked() {
                app.show_about_dialog = true;
                ui.close();
            }

            if ui.button("🖥 GPU Diagnostics").clicked() {
                app.show_gpu_dialog = true;
                ui.close();
            }
        });
    });
}
