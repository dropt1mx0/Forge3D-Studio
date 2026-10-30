use egui::Ui;
use forge_geometry::primitives::{
    generate_cube, generate_cylinder, generate_plane, generate_uv_sphere,
};
use forge_math::{Transform, Vec3};
use forge_scene::{Action, EditMode, PrimitiveType, SceneObject, SelectionElement, TransformMode};

use crate::app::Forge3dStudioApp;

pub fn render_toolbar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        // Mode switch button (Tab)
        let mode_is_edit = app.editor_state.edit_mode == EditMode::Edit;
        let mode_btn_label = if mode_is_edit { "✏ Edit" } else { "📦 Obj" };
        if ui
            .selectable_label(mode_is_edit, mode_btn_label)
            .on_hover_text("Toggle Object Mode / Edit Mode (Tab)")
            .clicked()
        {
            app.toggle_edit_mode();
        }

        ui.add_space(4.0);
        ui.separator();

        if app.editor_state.edit_mode == EditMode::Object {
            // Transform tools
            let tools = [
                (TransformMode::Select, "🔍", "Select (Q)"),
                (TransformMode::Translate, "↔", "Move (W)"),
                (TransformMode::Rotate, "🔄", "Rotate (E)"),
                (TransformMode::Scale, "📐", "Scale (R)"),
            ];

            for (mode, icon, tooltip) in tools {
                let is_active = app.editor_state.transform_mode == mode;
                let btn = ui.selectable_label(is_active, icon);
                if btn.on_hover_text(tooltip).clicked() {
                    app.editor_state.transform_mode = mode;
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label("Add");

            if ui.button("📦").on_hover_text("Add Cube").clicked() {
                let mesh = generate_cube(1.0);
                let id = app.scene.add_object(
                    SceneObject::new("Cube", PrimitiveType::Cube, mesh)
                        .with_color([0.85, 0.45, 0.25, 1.0])
                        .with_transform(Transform::from_translation(Vec3::new(0.0, 0.5, 0.0))),
                );
                if let Some(obj) = app.scene.get_object(id) {
                    app.history.push(Action::AddObject {
                        object: obj.clone(),
                    });
                }
                app.selection.select(id);
                app.status_message = "Added Cube".to_string();
            }

            if ui.button("🌐").on_hover_text("Add Sphere").clicked() {
                let mesh = generate_uv_sphere(0.75, 32, 16);
                let id = app.scene.add_object(
                    SceneObject::new("Sphere", PrimitiveType::Sphere, mesh)
                        .with_color([0.25, 0.65, 0.85, 1.0])
                        .with_transform(Transform::from_translation(Vec3::new(0.0, 0.75, 0.0))),
                );
                if let Some(obj) = app.scene.get_object(id) {
                    app.history.push(Action::AddObject {
                        object: obj.clone(),
                    });
                }
                app.selection.select(id);
                app.status_message = "Added UV Sphere".to_string();
            }

            if ui.button("🛢").on_hover_text("Add Cylinder").clicked() {
                let mesh = generate_cylinder(0.6, 1.5, 24);
                let id = app.scene.add_object(
                    SceneObject::new("Cylinder", PrimitiveType::Cylinder, mesh)
                        .with_color([0.75, 0.75, 0.35, 1.0])
                        .with_transform(Transform::from_translation(Vec3::new(0.0, 0.75, 0.0))),
                );
                if let Some(obj) = app.scene.get_object(id) {
                    app.history.push(Action::AddObject {
                        object: obj.clone(),
                    });
                }
                app.selection.select(id);
                app.status_message = "Added Cylinder".to_string();
            }

            if ui.button("⬛").on_hover_text("Add Plane").clicked() {
                let mesh = generate_plane(8.0, 8);
                let id = app.scene.add_object(
                    SceneObject::new("Plane", PrimitiveType::Plane, mesh)
                        .with_color([0.28, 0.30, 0.34, 1.0])
                        .with_transform(Transform::IDENTITY),
                );
                if let Some(obj) = app.scene.get_object(id) {
                    app.history.push(Action::AddObject {
                        object: obj.clone(),
                    });
                }
                app.selection.select(id);
                app.status_message = "Added Plane".to_string();
            }
        } else {
            // Edit Mode Tools
            ui.label("Select");
            let vert_sel = app.editor_state.selection_element == SelectionElement::Vertex;
            if ui
                .selectable_label(vert_sel, "•")
                .on_hover_text("Vertex Select (1)")
                .clicked()
            {
                app.editor_state.selection_element = SelectionElement::Vertex;
            }
            let edge_sel = app.editor_state.selection_element == SelectionElement::Edge;
            if ui
                .selectable_label(edge_sel, "╱")
                .on_hover_text("Edge Select (2)")
                .clicked()
            {
                app.editor_state.selection_element = SelectionElement::Edge;
            }
            let face_sel = app.editor_state.selection_element == SelectionElement::Face;
            if ui
                .selectable_label(face_sel, "■")
                .on_hover_text("Face Select (3)")
                .clicked()
            {
                app.editor_state.selection_element = SelectionElement::Face;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label("Mesh");

            if ui.button("⏏").on_hover_text("Extrude (E)").clicked() {
                app.extrude_selected_geometry(0.5);
            }
            if ui.button("⇲").on_hover_text("Inset (I)").clicked() {
                app.inset_selected_geometry(0.15);
            }
            if ui.button("⊞").on_hover_text("Subdivide").clicked() {
                app.subdivide_selected_geometry();
            }
            if ui.button("⨁").on_hover_text("Merge (M)").clicked() {
                app.merge_selected_geometry();
            }
            if ui.button("▱").on_hover_text("Fill (F)").clicked() {
                app.fill_selected_geometry();
            }
        }

        ui.add_space(8.0);
        ui.separator();
        // Snapping toggle
        let snap = app.editor_state.snap_settings.enabled;
        if ui
            .selectable_label(snap, "🧲")
            .on_hover_text("Toggle Snapping to Grid")
            .clicked()
        {
            app.editor_state.snap_settings.enabled = !snap;
        }
    });
}
