use egui::{DragValue, Ui};
use forge_math::Transform;

use crate::app::Forge3dStudioApp;

pub fn render_properties(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.heading("Properties");
    ui.separator();

    if let Some(selected_id) = app.selection.active_id {
        if let Some(obj) = app.scene.get_object_mut(selected_id) {
            // Object Name
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut obj.name);
            });

            ui.add_space(8.0);
            ui.separator();

            // Transform Section
            ui.label(egui::RichText::new("Transform").strong());

            // 1. Position (Translation)
            ui.label("Position (m):");
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(DragValue::new(&mut obj.local_transform.translation.x).speed(0.05));
                ui.label("Y:");
                ui.add(DragValue::new(&mut obj.local_transform.translation.y).speed(0.05));
                ui.label("Z:");
                ui.add(DragValue::new(&mut obj.local_transform.translation.z).speed(0.05));
            });

            // 2. Rotation (Degrees)
            let mut rot_deg = obj.local_transform.euler_angles_deg();
            ui.label("Rotation (deg):");
            let mut rot_changed = false;
            ui.horizontal(|ui| {
                ui.label("X:");
                if ui.add(DragValue::new(&mut rot_deg.x).speed(1.0)).changed() {
                    rot_changed = true;
                }
                ui.label("Y:");
                if ui.add(DragValue::new(&mut rot_deg.y).speed(1.0)).changed() {
                    rot_changed = true;
                }
                ui.label("Z:");
                if ui.add(DragValue::new(&mut rot_deg.z).speed(1.0)).changed() {
                    rot_changed = true;
                }
            });
            if rot_changed {
                obj.local_transform.set_euler_angles_deg(rot_deg);
            }

            // 3. Scale
            ui.label("Scale:");
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(
                    DragValue::new(&mut obj.local_transform.scale.x)
                        .speed(0.05)
                        .range(0.001..=100.0),
                );
                ui.label("Y:");
                ui.add(
                    DragValue::new(&mut obj.local_transform.scale.y)
                        .speed(0.05)
                        .range(0.001..=100.0),
                );
                ui.label("Z:");
                ui.add(
                    DragValue::new(&mut obj.local_transform.scale.z)
                        .speed(0.05)
                        .range(0.001..=100.0),
                );
            });

            if ui.button("↺ Reset Transform").clicked() {
                obj.local_transform = Transform::IDENTITY;
            }

            ui.add_space(8.0);
            ui.separator();

            // Material / Color Section
            ui.label(egui::RichText::new("Material").strong());
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_rgba_unmultiplied(&mut obj.color);
            });

            ui.add_space(8.0);
            ui.separator();

            // Geometry Statistics
            ui.label(egui::RichText::new("Geometry Info").strong());
            ui.label(format!("Vertices: {}", obj.mesh.vertices.len()));
            ui.label(format!("Triangles: {}", obj.mesh.indices.len() / 3));
            let size = obj.mesh.aabb.size();
            ui.label(format!(
                "Bounds: {:.2} x {:.2} x {:.2} m",
                size.x, size.y, size.z
            ));
        } else {
            // Stale selection ID
            app.selection.deselect();
        }
    } else {
        ui.label("No object selected.");
        ui.add_space(8.0);
        ui.label(
            "Click an object in the 3D viewport or outliner to view and modify its properties.",
        );
        ui.add_space(16.0);
        ui.separator();

        ui.label(egui::RichText::new("Scene Overview").strong());
        ui.label(format!("Total Objects: {}", app.scene.objects.len()));
        let total_triangles: usize = app
            .scene
            .objects
            .values()
            .map(|o| o.mesh.indices.len() / 3)
            .sum();
        ui.label(format!("Total Triangles: {total_triangles}"));
        ui.label(format!(
            "Camera Pos: ({:.1}, {:.1}, {:.1})",
            app.camera.eye.x, app.camera.eye.y, app.camera.eye.z
        ));
    }
}
