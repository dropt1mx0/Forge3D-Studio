use egui::Ui;
use forge_geometry::primitives::{
    generate_capsule, generate_cube, generate_cylinder, generate_monkey, generate_plane,
    generate_uv_sphere,
};
use forge_math::{Transform, Vec3};
use forge_scene::{
    Action, EditMode, GizmoMode, PivotPoint, PrimitiveType, SceneObject, SelectionElement,
    ToolbarDisplayMode, TransformMode, TransformOrientation,
};

use crate::app::Forge3dStudioApp;

fn tool_button(
    ui: &mut Ui,
    active: bool,
    icon: &str,
    label: &str,
    hotkey: &str,
    display_mode: ToolbarDisplayMode,
) -> egui::Response {
    let tooltip = if hotkey.is_empty() {
        label.to_string()
    } else {
        format!("{label} ({hotkey})")
    };

    match display_mode {
        ToolbarDisplayMode::IconOnly => ui.selectable_label(active, icon).on_hover_text(tooltip),
        ToolbarDisplayMode::IconAndLabel => {
            let text = if hotkey.is_empty() {
                format!("{icon}  {label}")
            } else {
                format!("{icon}  {label}  ({hotkey})")
            };
            ui.add_sized(
                [ui.available_width(), 22.0],
                egui::Button::new(text).selected(active),
            )
            .on_hover_text(tooltip)
        }
    }
}

pub fn render_toolbar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let display_mode = app.editor_state.panel_settings.toolbar_mode;

    ui.vertical(|ui| {
        // --- 0. Top Controls (Mode Toggle & Display Mode) ---
        let mode_is_edit = app.editor_state.edit_mode == EditMode::Edit;
        let mode_btn_label = if mode_is_edit {
            "✏ Edit Mode"
        } else {
            "📦 Object Mode"
        };
        let mode_btn_icon = if mode_is_edit { "✏" } else { "📦" };

        let btn = match display_mode {
            ToolbarDisplayMode::IconOnly => ui.selectable_label(mode_is_edit, mode_btn_icon),
            ToolbarDisplayMode::IconAndLabel => ui.add_sized(
                [ui.available_width(), 24.0],
                egui::Button::new(format!("{mode_btn_label} (Tab)")).selected(mode_is_edit),
            ),
        };
        if btn
            .on_hover_text("Toggle Object Mode / Edit Mode (Tab)")
            .clicked()
        {
            app.toggle_edit_mode();
        }

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            // Mode toggle button between IconOnly and IconAndLabel
            let is_label = display_mode == ToolbarDisplayMode::IconAndLabel;
            let toggle_icon = if is_label { "▣" } else { "▤" };
            let toggle_tip = if is_label {
                "Switch to Icon Only mode"
            } else {
                "Switch to Icon + Label mode"
            };
            if ui
                .small_button(toggle_icon)
                .on_hover_text(toggle_tip)
                .clicked()
            {
                app.editor_state.panel_settings.toolbar_mode = if is_label {
                    ToolbarDisplayMode::IconOnly
                } else {
                    ToolbarDisplayMode::IconAndLabel
                };
                let _ = app.editor_state.panel_settings.save();
            }

            // Hide Toolbar button
            if ui
                .small_button("◀")
                .on_hover_text("Collapse Toolbar (Show in View menu)")
                .clicked()
            {
                app.editor_state.panel_settings.toolbar_visible = false;
                let _ = app.editor_state.panel_settings.save();
            }
        });

        ui.add_space(4.0);
        ui.separator();

        // --- 1. Selection Tools ---
        if display_mode == ToolbarDisplayMode::IconAndLabel {
            ui.label(egui::RichText::new("Selection").weak().size(11.0));
        }

        let select_active = app.editor_state.transform_mode == TransformMode::Select;
        if tool_button(ui, select_active, "🔍", "Select", "Q", display_mode).clicked() {
            app.editor_state.transform_mode = TransformMode::Select;
            app.status_message = "Tool: Select (Q)".to_string();
        }

        let box_active = app.editor_state.transform_mode == TransformMode::BoxSelect;
        if tool_button(ui, box_active, "⬚", "Box Select", "B", display_mode).clicked() {
            app.editor_state.transform_mode = TransformMode::BoxSelect;
            app.status_message = "Tool: Box Select (B)".to_string();
        }

        ui.add_space(4.0);
        ui.separator();

        // --- 2. Transform Tools ---
        if display_mode == ToolbarDisplayMode::IconAndLabel {
            ui.label(egui::RichText::new("Transform").weak().size(11.0));
        }

        let is_move = app.editor_state.active_gizmo_mode == GizmoMode::Translate;
        if tool_button(ui, is_move, "↔", "Move", "G / W", display_mode).clicked() {
            app.editor_state.active_gizmo_mode = GizmoMode::Translate;
            app.editor_state.transform_mode = TransformMode::Translate;
            app.status_message = "Gizmo: Move (G / W)".to_string();
        }

        let is_rot = app.editor_state.active_gizmo_mode == GizmoMode::Rotate;
        if tool_button(ui, is_rot, "🔄", "Rotate", "R / E", display_mode).clicked() {
            app.editor_state.active_gizmo_mode = GizmoMode::Rotate;
            app.editor_state.transform_mode = TransformMode::Rotate;
            app.status_message = "Gizmo: Rotate (R / E)".to_string();
        }

        let is_scale = app.editor_state.active_gizmo_mode == GizmoMode::Scale;
        if tool_button(ui, is_scale, "📐", "Scale", "S", display_mode).clicked() {
            app.editor_state.active_gizmo_mode = GizmoMode::Scale;
            app.editor_state.transform_mode = TransformMode::Scale;
            app.status_message = "Gizmo: Scale (S)".to_string();
        }

        ui.add_space(4.0);
        ui.separator();

        // --- 3. Modeling Tools (in Edit Mode) ---
        if app.editor_state.edit_mode == EditMode::Edit {
            if display_mode == ToolbarDisplayMode::IconAndLabel {
                ui.label(egui::RichText::new("Modeling").weak().size(11.0));
            }

            let vert_sel = app.editor_state.selection_element == SelectionElement::Vertex;
            if tool_button(ui, vert_sel, "•", "Vertex", "1", display_mode).clicked() {
                app.editor_state.selection_element = SelectionElement::Vertex;
            }

            let edge_sel = app.editor_state.selection_element == SelectionElement::Edge;
            if tool_button(ui, edge_sel, "╱", "Edge", "2", display_mode).clicked() {
                app.editor_state.selection_element = SelectionElement::Edge;
            }

            let face_sel = app.editor_state.selection_element == SelectionElement::Face;
            if tool_button(ui, face_sel, "■", "Face", "3", display_mode).clicked() {
                app.editor_state.selection_element = SelectionElement::Face;
            }

            if tool_button(ui, false, "⏏", "Extrude", "E", display_mode).clicked() {
                app.extrude_selected_geometry(0.5);
            }

            if tool_button(ui, false, "⇲", "Inset", "I", display_mode).clicked() {
                app.inset_selected_geometry(0.15);
            }

            if tool_button(ui, false, "⊞", "Subdivide", "", display_mode).clicked() {
                app.subdivide_selected_geometry();
            }

            if tool_button(ui, false, "⨁", "Merge", "M", display_mode).clicked() {
                app.merge_selected_geometry();
            }

            if tool_button(ui, false, "▱", "Fill", "F", display_mode).clicked() {
                app.fill_selected_geometry();
            }

            ui.add_space(4.0);
            ui.separator();
        }

        // --- 4. Mesh / Add Primitives ---
        if app.editor_state.edit_mode == EditMode::Object {
            if display_mode == ToolbarDisplayMode::IconAndLabel {
                ui.label(egui::RichText::new("Add Mesh").weak().size(11.0));
            }

            let cursor = app.editor_state.cursor_3d;

            if tool_button(ui, false, "📦", "Cube", "", display_mode).clicked() {
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
            }

            if tool_button(ui, false, "🌐", "Sphere", "", display_mode).clicked() {
                let mesh = generate_uv_sphere(0.75, 32, 16);
                let id = app.scene.add_object(
                    SceneObject::new("Sphere", PrimitiveType::Sphere, mesh)
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
            }

            if tool_button(ui, false, "🛢", "Cylinder", "", display_mode).clicked() {
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
            }

            if tool_button(ui, false, "💊", "Capsule", "", display_mode).clicked() {
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
            }

            if tool_button(ui, false, "🐵", "Suzanne", "", display_mode).clicked() {
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
            }

            if tool_button(ui, false, "⬛", "Plane", "", display_mode).clicked() {
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
            }

            ui.add_space(4.0);
            ui.separator();
        }

        // --- 5. Utilities ---
        if display_mode == ToolbarDisplayMode::IconAndLabel {
            ui.label(egui::RichText::new("Utilities").weak().size(11.0));
        }

        // Snapping toggle
        let snap = app.editor_state.snap_settings.enabled;
        if tool_button(ui, snap, "🧲", "Snapping", "Shift+Tab", display_mode).clicked() {
            app.editor_state.snap_settings.enabled = !snap;
            app.status_message = format!("Snapping: {}", if !snap { "ON" } else { "OFF" });
        }

        // Orientation toggle (Global / Local)
        let is_local = app.editor_state.transform_orientation == TransformOrientation::Local;
        let (ori_icon, ori_label) = if is_local {
            ("📐", "Local Space")
        } else {
            ("🌐", "Global Space")
        };
        if tool_button(ui, is_local, ori_icon, ori_label, "", display_mode).clicked() {
            app.editor_state.transform_orientation = if is_local {
                TransformOrientation::Global
            } else {
                TransformOrientation::Local
            };
            app.status_message =
                format!("Orientation: {:?}", app.editor_state.transform_orientation);
        }

        // Pivot Point cycle
        let (piv_icon, piv_label) = match app.editor_state.pivot_point {
            PivotPoint::MedianPoint => ("🎯", "Median Point"),
            PivotPoint::ActiveElement => ("📍", "Active Element"),
            PivotPoint::Cursor3D => ("➕", "3D Cursor"),
            PivotPoint::IndividualOrigins => ("🔘", "Individual Origins"),
        };
        if tool_button(ui, false, piv_icon, piv_label, "", display_mode).clicked() {
            app.editor_state.pivot_point = match app.editor_state.pivot_point {
                PivotPoint::MedianPoint => PivotPoint::ActiveElement,
                PivotPoint::ActiveElement => PivotPoint::Cursor3D,
                PivotPoint::Cursor3D => PivotPoint::IndividualOrigins,
                PivotPoint::IndividualOrigins => PivotPoint::MedianPoint,
            };
            app.status_message = format!("Pivot Point: {:?}", app.editor_state.pivot_point);
        }
    });
}
