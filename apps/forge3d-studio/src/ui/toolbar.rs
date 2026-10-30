use egui::Ui;
use forge_geometry::primitives::{generate_cube, generate_plane, generate_uv_sphere};
use forge_math::{Transform, Vec3};
use forge_scene::{PrimitiveType, SceneObject, TransformMode};

use crate::app::Forge3dStudioApp;

pub fn render_toolbar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.heading("Tools");
        ui.separator();

        // Transform mode buttons
        let modes = [
            (TransformMode::Select, "🔍", "Select (Q)"),
            (TransformMode::Translate, "↔", "Move (W)"),
            (TransformMode::Rotate, "🔄", "Rotate (E)"),
            (TransformMode::Scale, "📐", "Scale (R)"),
        ];

        for (mode, icon, tooltip) in modes {
            let is_active = app.editor_state.transform_mode == mode;
            let btn = ui.selectable_label(
                is_active,
                format!(
                    "{icon}\n{}",
                    match mode {
                        TransformMode::Select => "Select",
                        TransformMode::Translate => "Move",
                        TransformMode::Rotate => "Rotate",
                        TransformMode::Scale => "Scale",
                    }
                ),
            );

            if btn.on_hover_text(tooltip).clicked() {
                app.editor_state.transform_mode = mode;
                app.status_message = format!("Active tool: {:?}", mode);
            }
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label("Add");

        if ui.button("📦\nCube").on_hover_text("Add Cube").clicked() {
            let mesh = generate_cube(1.0);
            let id = app.scene.add_object(
                SceneObject::new("Cube", PrimitiveType::Cube, mesh)
                    .with_color([0.85, 0.45, 0.25, 1.0])
                    .with_transform(Transform::from_translation(Vec3::new(0.0, 0.5, 0.0))),
            );
            app.selection.select(id);
            app.status_message = "Added Cube".to_string();
        }

        if ui
            .button("🌐\nSphere")
            .on_hover_text("Add Sphere")
            .clicked()
        {
            let mesh = generate_uv_sphere(0.65, 32, 16);
            let id = app.scene.add_object(
                SceneObject::new("Sphere", PrimitiveType::Sphere, mesh)
                    .with_color([0.25, 0.65, 0.85, 1.0])
                    .with_transform(Transform::from_translation(Vec3::new(0.0, 0.65, 0.0))),
            );
            app.selection.select(id);
            app.status_message = "Added Sphere".to_string();
        }

        if ui.button("⬛\nPlane").on_hover_text("Add Plane").clicked() {
            let mesh = generate_plane(10.0, 10);
            let id = app.scene.add_object(
                SceneObject::new("Plane", PrimitiveType::Plane, mesh)
                    .with_color([0.28, 0.30, 0.34, 1.0])
                    .with_transform(Transform::IDENTITY),
            );
            app.selection.select(id);
            app.status_message = "Added Plane".to_string();
        }

        ui.add_space(12.0);
        ui.separator();

        if let Some(selected_id) = app.selection.active_id
            && ui
                .button("🗑\nDelete")
                .on_hover_text("Delete selected object (Del)")
                .clicked()
        {
            app.scene.remove_object(selected_id);
            app.selection.deselect();
            app.status_message = "Deleted object".to_string();
        }
    });
}
