use egui::{DragValue, Ui};
use forge_modifiers::{
    ArrayModifier, MirrorModifier, ModifierType, SolidifyModifier, SubdivisionModifier,
};
use forge_scene::{PivotPoint, TransformOrientation};

use crate::app::Forge3dStudioApp;

pub fn render_properties(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.heading("Properties");
    ui.separator();

    if let Some(selected_id) = app.selection.active_id {
        let mut apply_tf = false;
        let mut reset_tf = false;
        let mut add_modifier_type = None;
        let mut remove_mod_idx = None;
        let mut apply_mod_idx = None;
        let mut move_up_idx = None;
        let mut move_down_idx = None;

        if let Some(obj) = app.scene.get_object_mut(selected_id) {
            // Object Name
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut obj.name);
            });

            ui.add_space(6.0);
            ui.separator();

            // Transform Section
            ui.label(egui::RichText::new("Transform").strong());

            // Orientation & Pivot options
            ui.horizontal(|ui| {
                ui.label("Orientation:");
                egui::ComboBox::from_id_salt("transform_orientation")
                    .selected_text(format!("{:?}", app.editor_state.transform_orientation))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.editor_state.transform_orientation,
                            TransformOrientation::Global,
                            "Global",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.transform_orientation,
                            TransformOrientation::Local,
                            "Local",
                        );
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Pivot Point:");
                egui::ComboBox::from_id_salt("pivot_point")
                    .selected_text(format!("{:?}", app.editor_state.pivot_point))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::MedianPoint,
                            "Median Point",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::ActiveElement,
                            "Active Element",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::Cursor3D,
                            "3D Cursor",
                        );
                    });
            });

            ui.add_space(4.0);

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

            ui.horizontal(|ui| {
                if ui.button("✔ Apply Transform").clicked() {
                    apply_tf = true;
                }
                if ui.button("↺ Reset").clicked() {
                    reset_tf = true;
                }
            });

            ui.add_space(8.0);
            ui.separator();

            // Modifiers Section
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Modifiers").strong());
                egui::ComboBox::from_id_salt("add_modifier_combo")
                    .selected_text("+ Add Modifier")
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(false, "Mirror").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Mirror(MirrorModifier::default()));
                        }
                        if ui.selectable_label(false, "Subdivision Surface").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Subdivide(SubdivisionModifier::default()));
                        }
                        if ui.selectable_label(false, "Solidify").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Solidify(SolidifyModifier::default()));
                        }
                        if ui.selectable_label(false, "Array").clicked() {
                            add_modifier_type = Some(ModifierType::Array(ArrayModifier::default()));
                        }
                    });
            });

            let num_mods = obj.modifiers.len();
            for i in 0..num_mods {
                let entry = &mut obj.modifiers.modifiers[i];
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut entry.enabled, "");
                        ui.label(egui::RichText::new(&entry.name).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("❌").on_hover_text("Delete Modifier").clicked() {
                                remove_mod_idx = Some(i);
                            }
                            if ui.button("✔").on_hover_text("Apply to Mesh").clicked() {
                                apply_mod_idx = Some(i);
                            }
                            if i > 0 && ui.button("▲").clicked() {
                                move_up_idx = Some(i);
                            }
                            if i + 1 < num_mods && ui.button("▼").clicked() {
                                move_down_idx = Some(i);
                            }
                        });
                    });

                    // Modifier-specific parameters
                    match &mut entry.modifier {
                        ModifierType::Mirror(m) => {
                            ui.horizontal(|ui| {
                                ui.label("Axis:");
                                ui.checkbox(&mut m.axis_x, "X");
                                ui.checkbox(&mut m.axis_y, "Y");
                                ui.checkbox(&mut m.axis_z, "Z");
                            });
                        }
                        ModifierType::Subdivide(s) => {
                            ui.horizontal(|ui| {
                                ui.label("Levels:");
                                ui.add(egui::Slider::new(&mut s.levels, 1..=4));
                            });
                        }
                        ModifierType::Solidify(s) => {
                            ui.horizontal(|ui| {
                                ui.label("Thickness:");
                                ui.add(
                                    DragValue::new(&mut s.thickness)
                                        .speed(0.01)
                                        .range(0.0..=10.0),
                                );
                            });
                        }
                        ModifierType::Array(a) => {
                            ui.horizontal(|ui| {
                                ui.label("Count:");
                                ui.add(egui::Slider::new(&mut a.count, 1..=20));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Offset:");
                                ui.add(DragValue::new(&mut a.offset.x).speed(0.1));
                                ui.add(DragValue::new(&mut a.offset.y).speed(0.1));
                                ui.add(DragValue::new(&mut a.offset.z).speed(0.1));
                            });
                        }
                    }
                });
            }

            ui.add_space(8.0);
            ui.separator();

            // Material Section
            ui.label(egui::RichText::new("Material").strong());
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_rgba_unmultiplied(&mut obj.color);
            });

            ui.add_space(8.0);
            ui.separator();

            // Geometry Statistics
            ui.label(egui::RichText::new("Geometry Info").strong());
            let eval = obj.evaluated_mesh();
            ui.label(format!("Base Vertices: {}", obj.mesh.vertices.len()));
            ui.label(format!("Evaluated Vertices: {}", eval.vertices.len()));
            ui.label(format!("Triangles: {}", eval.indices.len() / 3));
        }

        if apply_tf {
            app.apply_transform_selected();
        }
        if reset_tf {
            app.scene.reset_transform_for_object(selected_id);
        }
        if let Some(mod_type) = add_modifier_type
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.add(mod_type);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
        if let Some(idx) = remove_mod_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.remove(idx);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
        if let Some(idx) = apply_mod_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.apply_to_mesh(idx, &mut obj.mesh);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
            app.status_message = "Applied modifier to base mesh".to_string();
        }
        if let Some(idx) = move_up_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.move_up(idx);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
        if let Some(idx) = move_down_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.move_down(idx);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
    } else {
        ui.label("No object selected.");
        ui.add_space(8.0);
        ui.label(
            "Click an object in the 3D viewport or outliner to view and modify its properties.",
        );
    }
}
