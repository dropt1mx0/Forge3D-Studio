use egui::Ui;
use forge_render::CameraProjection;
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

    let is_dark = ui.visuals().dark_mode;
    let inactive_fg = if is_dark {
        egui::Color32::from_rgb(200, 200, 200)
    } else {
        egui::Color32::from_rgb(50, 50, 50)
    };
    let inactive_bg = if is_dark {
        egui::Color32::from_rgb(50, 50, 50)
    } else {
        egui::Color32::from_rgb(228, 228, 228)
    };

    match display_mode {
        ToolbarDisplayMode::IconOnly => {
            let btn = egui::Button::new(egui::RichText::new(icon).size(11.0).color(if active {
                egui::Color32::WHITE
            } else {
                inactive_fg
            }))
            .selected(active)
            .fill(if active {
                egui::Color32::from_rgb(74, 137, 220)
            } else {
                inactive_bg
            })
            .corner_radius(egui::CornerRadius::same(2));
            ui.add_sized([ui.available_width().max(32.0), 22.0], btn)
                .on_hover_text(tooltip)
        }
        ToolbarDisplayMode::IconAndLabel => {
            let text = if hotkey.is_empty() {
                label.to_string()
            } else {
                format!("{label} ({hotkey})")
            };
            let btn = egui::Button::new(egui::RichText::new(text).size(11.0).color(if active {
                egui::Color32::WHITE
            } else {
                inactive_fg
            }))
            .selected(active)
            .fill(if active {
                egui::Color32::from_rgb(74, 137, 220)
            } else {
                inactive_bg
            })
            .corner_radius(egui::CornerRadius::same(2));
            ui.add_sized([ui.available_width(), 22.0], btn)
                .on_hover_text(tooltip)
        }
    }
}

pub fn render_toolbar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let display_mode = app.editor_state.panel_settings.toolbar_mode;
    let is_dark = ui.visuals().dark_mode;

    egui::ScrollArea::vertical()
        .id_salt("left_toolbar_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 3.0);
            ui.vertical(|ui| {
                // --- 0. Top Controls (Mode Toggle & Display Mode) ---
                let mode_is_edit = app.editor_state.edit_mode == EditMode::Edit;
                let mode_is_sculpt = app.editor_state.edit_mode == EditMode::Sculpt;
                let mode_btn_label = match app.editor_state.edit_mode {
                    EditMode::Object => "Object Mode",
                    EditMode::Edit => "Edit Mode",
                    EditMode::Sculpt => "Sculpt Mode",
                };
                let mode_btn_icon = match app.editor_state.edit_mode {
                    EditMode::Object => "Obj",
                    EditMode::Edit => "Edit",
                    EditMode::Sculpt => "Sclp",
                };

                let mode_btn = egui::Button::new(
                    egui::RichText::new(match display_mode {
                        ToolbarDisplayMode::IconOnly => mode_btn_icon,
                        ToolbarDisplayMode::IconAndLabel => mode_btn_label,
                    })
                    .strong()
                    .size(11.5)
                    .color(if mode_is_sculpt {
                        if is_dark {
                            egui::Color32::from_rgb(130, 185, 240)
                        } else {
                            egui::Color32::from_rgb(40, 90, 200)
                        }
                    } else if mode_is_edit {
                        if is_dark {
                            egui::Color32::from_rgb(230, 170, 60)
                        } else {
                            egui::Color32::from_rgb(160, 90, 20)
                        }
                    } else {
                        if is_dark {
                            egui::Color32::from_rgb(200, 200, 200)
                        } else {
                            egui::Color32::from_rgb(50, 50, 50)
                        }
                    }),
                )
                .fill(if mode_is_sculpt {
                    if is_dark {
                        egui::Color32::from_rgb(45, 55, 72)
                    } else {
                        egui::Color32::from_rgb(210, 225, 245)
                    }
                } else if mode_is_edit {
                    if is_dark {
                        egui::Color32::from_rgb(62, 52, 38)
                    } else {
                        egui::Color32::from_rgb(248, 238, 210)
                    }
                } else {
                    if is_dark {
                        egui::Color32::from_rgb(50, 50, 50)
                    } else {
                        egui::Color32::from_rgb(228, 228, 228)
                    }
                })
                .stroke(egui::Stroke::new(
                    1.0,
                    if mode_is_sculpt {
                        egui::Color32::from_rgb(74, 137, 220)
                    } else if mode_is_edit {
                        egui::Color32::from_rgb(210, 140, 40)
                    } else {
                        if is_dark {
                            egui::Color32::from_rgb(65, 65, 65)
                        } else {
                            egui::Color32::from_rgb(195, 195, 195)
                        }
                    },
                ))
                .corner_radius(egui::CornerRadius::same(2));

                if ui
                    .add_sized([ui.available_width().max(32.0), 24.0], mode_btn)
                    .on_hover_text("Toggle Object / Edit / Sculpt Mode")
                    .clicked()
                {
                    match app.editor_state.edit_mode {
                        EditMode::Object => app.toggle_edit_mode(),
                        EditMode::Edit => {
                            app.editor_state.edit_mode = EditMode::Sculpt;
                            app.status_message = "Sculpt Mode".to_string();
                        }
                        EditMode::Sculpt => app.toggle_edit_mode(),
                    }
                }

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    let is_label = display_mode == ToolbarDisplayMode::IconAndLabel;
                    let toggle_label = if is_label { "Compact" } else { "Wide" };
                    let toggle_tip = if is_label {
                        "Switch to Icon Only mode"
                    } else {
                        "Switch to Icon + Label mode"
                    };
                    if ui
                        .small_button(toggle_label)
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
                        .small_button("<")
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
                if tool_button(ui, select_active, "Sel", "Select", "Q", display_mode).clicked() {
                    app.editor_state.transform_mode = TransformMode::Select;
                    app.status_message = "Tool: Select (Q)".to_string();
                }

                let box_active = app.editor_state.transform_mode == TransformMode::BoxSelect;
                if tool_button(ui, box_active, "Box", "Box Select", "B", display_mode).clicked() {
                    app.editor_state.transform_mode = TransformMode::BoxSelect;
                    app.status_message = "Tool: Box Select (B)".to_string();
                }

                let lasso_active = app.editor_state.transform_mode == TransformMode::LassoSelect;
                if tool_button(ui, lasso_active, "Las", "Lasso Select", "Ctrl+RMB", display_mode).clicked() {
                    app.editor_state.transform_mode = TransformMode::LassoSelect;
                    app.status_message = "Tool: Lasso Select (Ctrl+RMB)".to_string();
                }

                let circle_active = app.editor_state.transform_mode == TransformMode::CircleSelect;
                if tool_button(ui, circle_active, "Cir", "Circle Select", "C", display_mode).clicked() {
                    app.editor_state.transform_mode = TransformMode::CircleSelect;
                    app.editor_state.circle_select_active = true;
                    app.status_message = "Tool: Circle Select (C)".to_string();
                }

                ui.add_space(4.0);
                ui.separator();

                // --- 2. Transform Tools ---
                if display_mode == ToolbarDisplayMode::IconAndLabel {
                    ui.label(egui::RichText::new("Transform").weak().size(11.0));
                }

                let is_move = app.editor_state.active_gizmo_mode == GizmoMode::Translate;
                if tool_button(ui, is_move, "Move", "Move", "G / W", display_mode).clicked() {
                    app.editor_state.active_gizmo_mode = GizmoMode::Translate;
                    app.editor_state.transform_mode = TransformMode::Translate;
                    app.status_message = "Gizmo: Move (G / W)".to_string();
                }

                let is_rot = app.editor_state.active_gizmo_mode == GizmoMode::Rotate;
                if tool_button(ui, is_rot, "Rot", "Rotate", "R / E", display_mode).clicked() {
                    app.editor_state.active_gizmo_mode = GizmoMode::Rotate;
                    app.editor_state.transform_mode = TransformMode::Rotate;
                    app.status_message = "Gizmo: Rotate (R / E)".to_string();
                }

                let is_scale = app.editor_state.active_gizmo_mode == GizmoMode::Scale;
                if tool_button(ui, is_scale, "Scale", "Scale", "S", display_mode).clicked() {
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
                    if tool_button(ui, vert_sel, "Vert", "Vertex Selection", "1", display_mode)
                        .clicked()
                    {
                        app.editor_state.selection_element = SelectionElement::Vertex;
                    }

                    let edge_sel = app.editor_state.selection_element == SelectionElement::Edge;
                    if tool_button(ui, edge_sel, "Edge", "Edge Selection", "2", display_mode)
                        .clicked()
                    {
                        app.editor_state.selection_element = SelectionElement::Edge;
                    }

                    let face_sel = app.editor_state.selection_element == SelectionElement::Face;
                    if tool_button(ui, face_sel, "Face", "Face Selection", "3", display_mode)
                        .clicked()
                    {
                        app.editor_state.selection_element = SelectionElement::Face;
                    }

                    if tool_button(ui, false, "Ext", "Extrude", "E", display_mode).clicked() {
                        let center = forge_math::Vec2::new(400.0, 300.0);
                        app.begin_interactive_edit_tool(
                            crate::app::InteractiveEditToolType::Extrude,
                            center,
                        );
                    }

                    if tool_button(ui, false, "Ins", "Inset", "I", display_mode).clicked() {
                        let center = forge_math::Vec2::new(400.0, 300.0);
                        app.begin_interactive_edit_tool(
                            crate::app::InteractiveEditToolType::Inset,
                            center,
                        );
                    }

                    if tool_button(ui, false, "Sub", "Subdivide", "", display_mode).clicked() {
                        app.subdivide_selected_geometry();
                    }

                    if tool_button(ui, false, "Mrg", "Merge", "M", display_mode).clicked() {
                        app.merge_selected_geometry();
                    }

                    if tool_button(ui, false, "Fill", "Fill", "F", display_mode).clicked() {
                        app.fill_selected_geometry();
                    }

                    if tool_button(ui, false, "Bev", "Bevel", "Ctrl+B", display_mode).clicked() {
                        app.bevel_selected_geometry(0.1);
                    }

                    if tool_button(ui, false, "Loop", "Loop Cut", "Ctrl+R", display_mode).clicked()
                    {
                        let center = forge_math::Vec2::new(400.0, 300.0);
                        app.begin_interactive_edit_tool(
                            crate::app::InteractiveEditToolType::LoopCut,
                            center,
                        );
                    }

                    if tool_button(ui, false, "Dis", "Dissolve", "Ctrl+X", display_mode).clicked() {
                        app.dissolve_selected_geometry();
                    }

                    if tool_button(ui, false, "Brdg", "Bridge", "", display_mode).clicked() {
                        app.bridge_selected_geometry();
                    }

                    if tool_button(ui, false, "Sep", "Separate", "P", display_mode).clicked() {
                        app.separate_selected_geometry();
                    }

                    if tool_button(ui, false, "Knf", "Knife", "K", display_mode).clicked() {
                        app.knife_connect_selected_geometry();
                    }

                    if tool_button(ui, false, "Bsc", "Bisect", "Alt+B", display_mode).clicked() {
                        app.bisect_selected_geometry(
                            forge_math::Vec3::ZERO,
                            forge_math::Vec3::Y,
                            true,
                            true,
                        );
                    }

                    if tool_button(ui, false, "Spn", "Spin", "Alt+S", display_mode).clicked() {
                        app.spin_selected_geometry(12, std::f32::consts::TAU);
                    }

                    ui.add_space(4.0);
                    ui.separator();
                }

                // --- 3b. Sculpting Mode Tools & Brushes ---
                if app.editor_state.edit_mode == EditMode::Sculpt {
                    if display_mode == ToolbarDisplayMode::IconAndLabel {
                        ui.label(egui::RichText::new("Sculpt Brushes").weak().size(11.0));
                    }

                    for brush in [
                        forge_geometry::SculptBrush::Draw,
                        forge_geometry::SculptBrush::Clay,
                        forge_geometry::SculptBrush::Smooth,
                        forge_geometry::SculptBrush::Flatten,
                        forge_geometry::SculptBrush::Inflate,
                        forge_geometry::SculptBrush::Deflate,
                        forge_geometry::SculptBrush::Grab,
                        forge_geometry::SculptBrush::SnakeHook,
                        forge_geometry::SculptBrush::ElasticDeform,
                        forge_geometry::SculptBrush::Thumb,
                        forge_geometry::SculptBrush::Pinch,
                        forge_geometry::SculptBrush::Crease,
                    ] {
                        let is_active = app.sculpt_settings.brush == brush;
                        let icon = match brush {
                            forge_geometry::SculptBrush::Draw => "Drw",
                            forge_geometry::SculptBrush::Clay => "Cly",
                            forge_geometry::SculptBrush::Smooth => "Smth",
                            forge_geometry::SculptBrush::Flatten => "Flt",
                            forge_geometry::SculptBrush::Inflate => "Inf",
                            forge_geometry::SculptBrush::Deflate => "Def",
                            forge_geometry::SculptBrush::Grab => "Grb",
                            forge_geometry::SculptBrush::SnakeHook => "Snk",
                            forge_geometry::SculptBrush::ElasticDeform => "Elst",
                            forge_geometry::SculptBrush::Thumb => "Thmb",
                            forge_geometry::SculptBrush::Pinch => "Pnch",
                            forge_geometry::SculptBrush::Crease => "Crs",
                        };
                        if tool_button(ui, is_active, icon, brush.name(), "", display_mode)
                            .clicked()
                        {
                            app.sculpt_settings.brush = brush;
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();

                    if display_mode == ToolbarDisplayMode::IconAndLabel {
                        ui.horizontal(|ui| {
                            ui.label("Radius:");
                            ui.add(egui::Slider::new(
                                &mut app.sculpt_settings.radius,
                                0.05..=2.0,
                            ));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Str:");
                            ui.add(egui::Slider::new(
                                &mut app.sculpt_settings.strength,
                                0.01..=1.0,
                            ));
                        });
                        ui.checkbox(&mut app.sculpt_settings.symmetry_x, "Symm X");
                        ui.checkbox(&mut app.sculpt_settings.symmetry_y, "Symm Y");
                        ui.checkbox(&mut app.sculpt_settings.symmetry_z, "Symm Z");

                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("Remesh").weak().size(11.0));
                        ui.horizontal(|ui| {
                            ui.label("Vox:");
                            ui.add(egui::Slider::new(
                                &mut app.sculpt_settings.voxel_size,
                                0.02..=0.5,
                            ));
                        });
                        if ui.button("⬚ Remesh (Ctrl+R)").clicked() {
                            app.voxel_remesh_active(app.sculpt_settings.voxel_size);
                        }
                    } else {
                        if tool_button(ui, false, "Rmsh", "Voxel Remesh", "Ctrl+R", display_mode).clicked() {
                            app.voxel_remesh_active(app.sculpt_settings.voxel_size);
                        }
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

                    if tool_button(ui, false, "Cube", "Cube", "", display_mode).clicked() {
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

                    if tool_button(ui, false, "Sphere", "UV Sphere", "", display_mode).clicked() {
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

                    if tool_button(ui, false, "Cyl", "Cylinder", "", display_mode).clicked() {
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

                    if tool_button(ui, false, "Caps", "Capsule", "", display_mode).clicked() {
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

                    if tool_button(ui, false, "Suzanne", "Suzanne", "", display_mode).clicked() {
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

                    if tool_button(ui, false, "Plane", "Plane", "", display_mode).clicked() {
                        let mesh = generate_plane(8.0, 8);
                        let id = app.scene.add_object(
                            SceneObject::new("Plane", PrimitiveType::Plane, mesh)
                                .with_color([0.72, 0.74, 0.78, 1.0])
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

                // Snapping toggle (LMB: toggle on/off, RMB: switch between Grid and Vertex snap)
                let snap = app.editor_state.snap_settings.enabled;
                let snap_kind = if app.editor_state.snap_settings.snap_to_vertex { "Vertex" } else { "Grid" };
                let snap_title = format!("Snap ({})", snap_kind);
                let snap_btn = tool_button(ui, snap, "Snap", &snap_title, "Shift+Tab", display_mode);
                if snap_btn.clicked() {
                    app.editor_state.snap_settings.enabled = !snap;
                    app.status_message = format!(
                        "Snapping: {} ({})",
                        if !snap { "ON" } else { "OFF" },
                        snap_kind
                    );
                }
                if snap_btn.secondary_clicked() {
                    app.editor_state.snap_settings.snap_to_vertex = !app.editor_state.snap_settings.snap_to_vertex;
                    app.editor_state.snap_settings.snap_to_grid = !app.editor_state.snap_settings.snap_to_vertex;
                    app.status_message = format!(
                        "Snap Element: {}",
                        if app.editor_state.snap_settings.snap_to_vertex { "Vertex" } else { "Grid Increment" }
                    );
                }

                // Orientation toggle (Global / Local)
                let is_local =
                    app.editor_state.transform_orientation == TransformOrientation::Local;
                let (ori_icon, ori_label) = if is_local {
                    ("Local", "Local Space")
                } else {
                    ("Global", "Global Space")
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
                    PivotPoint::MedianPoint => ("Median", "Median Point"),
                    PivotPoint::ActiveElement => ("Active", "Active Element"),
                    PivotPoint::Cursor3D => ("Cursor", "3D Cursor"),
                    PivotPoint::IndividualOrigins => ("Indiv", "Individual Origins"),
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

                ui.add_space(4.0);
                ui.separator();

                // --- 6. Navigation & View ---
                if display_mode == ToolbarDisplayMode::IconAndLabel {
                    ui.label(egui::RichText::new("Navigation").weak().size(11.0));
                }

                // Zoom button (drag up/down to zoom view)
                let zoom_resp = tool_button(ui, false, "Zoom", "Zoom View", "Drag", display_mode);
                if zoom_resp.dragged() {
                    let dy = ui.input(|i| i.pointer.delta().y);
                    app.camera.zoom(1.0 - dy * 0.02);
                    ui.ctx().request_repaint();
                }

                // Pan button (drag to pan view)
                let pan_resp = tool_button(ui, false, "Pan", "Pan View", "Drag", display_mode);
                if pan_resp.dragged() {
                    let delta = ui.input(|i| i.pointer.delta());
                    app.camera.pan(delta.x, delta.y);
                    ui.ctx().request_repaint();
                }

                // Camera view toggle
                if tool_button(ui, false, "Cam", "Camera View", "Num 0", display_mode).clicked() {
                    app.toggle_camera_view();
                }

                // Focus / Frame Selected
                if tool_button(ui, false, "Focus", "Frame Selected", "F", display_mode).clicked() {
                    app.frame_selected();
                }

                // Perspective / Orthographic toggle
                let (proj_icon, proj_label) = match app.camera.projection {
                    CameraProjection::Perspective => ("Ortho", "Orthographic View"),
                    CameraProjection::Orthographic => ("Persp", "Perspective View"),
                };
                if tool_button(ui, false, proj_icon, proj_label, "Num 5", display_mode).clicked() {
                    app.camera.projection = match app.camera.projection {
                        CameraProjection::Perspective => CameraProjection::Orthographic,
                        CameraProjection::Orthographic => CameraProjection::Perspective,
                    };
                    app.status_message = format!("Projection: {:?}", app.camera.projection);
                }

                // X-Ray Mode
                let is_xray = app.editor_state.show_xray;
                if tool_button(ui, is_xray, "X-Ray", "X-Ray Mode", "Alt+Z", display_mode).clicked() {
                    app.toggle_xray();
                }

                // Local View / Isolate Selected
                let is_local = app.is_in_local_view();
                if tool_button(
                    ui,
                    is_local,
                    "Local",
                    "Local View (Isolate)",
                    "Num /",
                    display_mode,
                )
                .clicked()
                {
                    app.toggle_local_view();
                }
            });
        });
}
