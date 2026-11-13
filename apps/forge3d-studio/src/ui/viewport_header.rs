use egui::{Color32, Margin, Stroke, Ui};
use forge_geometry::primitives::{
    generate_circle, generate_cone, generate_cube, generate_cylinder, generate_icosphere,
    generate_monkey, generate_plane, generate_torus, generate_uv_sphere,
};
use forge_math::{Transform, Vec3};
use forge_render::ViewPreset;
use forge_scene::{
    Action, EditMode, LightData, LightKind, PivotPoint, PrimitiveType, SceneObject,
    SelectionElement, TransformMode, TransformOrientation, ViewportMatcap,
};

use crate::app::{Forge3dStudioApp, InteractiveEditToolType};

fn draw_shading_sphere(
    ui: &mut Ui,
    active: bool,
    sphere_idx: usize,
    tooltip: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
    let painter = ui.painter();
    let center = rect.center();
    let radius = 6.5;

    // Hover or active indicator
    if active {
        painter.circle_filled(center, radius + 2.5, Color32::from_rgb(71, 114, 179));
    } else if resp.hovered() {
        painter.circle_filled(center, radius + 2.0, Color32::from_rgb(60, 60, 60));
    }

    match sphere_idx {
        0 => {
            // Wireframe: hollow circle outline
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(
                    1.2,
                    if active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    },
                ),
            );
        }
        1 => {
            // Solid: solid neutral filled circle
            painter.circle_filled(
                center,
                radius,
                if active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(170, 170, 170)
                },
            );
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(1.0, Color32::from_rgb(45, 45, 45)),
            );
        }
        2 => {
            // Material Preview: half light, half dark
            painter.circle_filled(center, radius, Color32::from_rgb(65, 65, 65));
            let points = [
                center + egui::vec2(0.0, -radius),
                center + egui::vec2(-radius * 0.7, -radius * 0.7),
                center + egui::vec2(-radius, 0.0),
                center + egui::vec2(-radius * 0.7, radius * 0.7),
                center + egui::vec2(0.0, radius),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                Color32::from_rgb(220, 220, 220),
                Stroke::NONE,
            ));
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(1.0, Color32::from_rgb(40, 40, 40)),
            );
        }
        3 => {
            // Rendered: illuminated warm center
            painter.circle_filled(center, radius, Color32::from_rgb(225, 155, 35));
            painter.circle_filled(center, radius * 0.45, Color32::from_rgb(255, 235, 160));
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(1.0, Color32::from_rgb(150, 95, 15)),
            );
        }
        _ => {}
    }

    resp.on_hover_text(tooltip)
}

fn add_scene_mesh(
    app: &mut Forge3dStudioApp,
    name: &str,
    prim_type: PrimitiveType,
    mesh: forge_geometry::Mesh,
    color: [f32; 4],
    offset: Vec3,
) {
    let cursor = app.editor_state.cursor_3d;
    let id = app.scene.add_object(
        SceneObject::new(name, prim_type, mesh)
            .with_color(color)
            .with_transform(Transform::from_translation(cursor + offset)),
    );
    if let Some(obj) = app.scene.get_object(id) {
        app.history.push(Action::AddObject {
            object: obj.clone(),
        });
    }
    app.selection.select(id);
    app.status_message = format!("Added {name}");
}

pub fn render_viewport_header(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let is_dark = ui.visuals().dark_mode;
    let header_bg = if is_dark {
        Color32::from_rgb(38, 38, 38) // #262626 Blender 4.x header background
    } else {
        Color32::from_rgb(230, 230, 230)
    };
    let header_border = if is_dark {
        Color32::from_rgb(26, 26, 26)
    } else {
        Color32::from_rgb(205, 205, 205)
    };

    let mut overlays_button_rect = egui::Rect::NOTHING;

    egui::Frame::new()
        .fill(header_bg)
        .stroke(Stroke::new(1.0, header_border))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            ui.horizontal(|ui| {
                // 1. Viewport Icon
                ui.label(
                    egui::RichText::new("⛶")
                        .color(Color32::from_rgb(110, 160, 230))
                        .size(13.0)
                        .strong(),
                )
                .on_hover_text("3D Viewport");

                // 2. Mode Selector Pill Button
                let mode_label = match app.editor_state.edit_mode {
                    EditMode::Object => "Object Mode ▾",
                    EditMode::Edit => "Edit Mode ▾",
                    EditMode::Sculpt => "Sculpt Mode ▾",
                };
                let mode_text_col = match app.editor_state.edit_mode {
                    EditMode::Object => Color32::WHITE,
                    EditMode::Edit => Color32::from_rgb(240, 175, 70),
                    EditMode::Sculpt => Color32::from_rgb(120, 180, 245),
                };

                ui.menu_button(
                    egui::RichText::new(mode_label)
                        .size(11.5)
                        .strong()
                        .color(mode_text_col),
                    |ui| {
                        if ui.button("Object Mode (Tab)").clicked() {
                            if app.editor_state.edit_mode != EditMode::Object {
                                app.toggle_edit_mode();
                            }
                            ui.close();
                        }
                        if ui.button("Edit Mode (Tab)").clicked() {
                            if app.editor_state.edit_mode != EditMode::Edit {
                                app.toggle_edit_mode();
                            }
                            ui.close();
                        }
                        if ui.button("Sculpt Mode").clicked() {
                            app.editor_state.edit_mode = EditMode::Sculpt;
                            app.status_message = "Switched to Sculpt Mode".to_string();
                            ui.close();
                        }
                    },
                );

                // 3. Selection Elements Toggles (When in Edit Mode: Vertex, Edge, Face)
                if app.editor_state.edit_mode == EditMode::Edit {
                    let sel_elem = app.editor_state.selection_element;
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);

                    let v_active = sel_elem == SelectionElement::Vertex;
                    let v_btn =
                        egui::Button::new(egui::RichText::new("•").size(12.0).color(if v_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(180, 180, 180)
                        }))
                        .fill(if v_active {
                            Color32::from_rgb(71, 114, 179)
                        } else {
                            Color32::from_rgb(50, 50, 50)
                        })
                        .corner_radius(egui::CornerRadius::same(2));
                    if ui
                        .add(v_btn)
                        .on_hover_text("Vertex Selection (1)")
                        .clicked()
                    {
                        app.editor_state.selection_element = SelectionElement::Vertex;
                    }

                    let e_active = sel_elem == SelectionElement::Edge;
                    let e_btn =
                        egui::Button::new(egui::RichText::new("/").size(12.0).color(if e_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(180, 180, 180)
                        }))
                        .fill(if e_active {
                            Color32::from_rgb(71, 114, 179)
                        } else {
                            Color32::from_rgb(50, 50, 50)
                        })
                        .corner_radius(egui::CornerRadius::same(2));
                    if ui.add(e_btn).on_hover_text("Edge Selection (2)").clicked() {
                        app.editor_state.selection_element = SelectionElement::Edge;
                    }

                    let f_active = sel_elem == SelectionElement::Face;
                    let f_btn =
                        egui::Button::new(egui::RichText::new("▱").size(11.0).color(if f_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(180, 180, 180)
                        }))
                        .fill(if f_active {
                            Color32::from_rgb(71, 114, 179)
                        } else {
                            Color32::from_rgb(50, 50, 50)
                        })
                        .corner_radius(egui::CornerRadius::same(2));
                    if ui.add(f_btn).on_hover_text("Face Selection (3)").clicked() {
                        app.editor_state.selection_element = SelectionElement::Face;
                    }
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                }

                ui.separator();

                // 4. Viewport Sub-menus: View, Select, Add, Object / Mesh
                ui.menu_button("View", |ui| {
                    if ui.button("Perspective / Orthographic (Numpad 5)").clicked() {
                        app.camera.toggle_projection();
                        ui.close();
                    }
                    let cam_txt = if app.editor_state.is_camera_view {
                        "Exit Camera View (Numpad 0)"
                    } else {
                        "View Camera (Numpad 0)"
                    };
                    if ui.button(cam_txt).clicked() {
                        app.toggle_camera_view();
                        ui.close();
                    }
                    if ui
                        .button("Align Camera to View (Ctrl+Alt+Numpad 0)")
                        .clicked()
                    {
                        app.align_active_camera_to_view();
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("Viewpoint", |ui| {
                        if ui.button("Top (Numpad 7)").clicked() {
                            app.camera.set_view_preset(ViewPreset::Top);
                            ui.close();
                        }
                        if ui.button("Bottom (Ctrl+Numpad 7)").clicked() {
                            app.camera.set_view_preset(ViewPreset::Bottom);
                            ui.close();
                        }
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
                    });
                    ui.separator();
                    if ui.button("Frame Selected (F / Numpad .)").clicked() {
                        app.frame_selected();
                        ui.close();
                    }
                    if ui.button("Frame All (Home)").clicked() {
                        app.frame_all();
                        ui.close();
                    }
                    let local_txt = if app.is_in_local_view() {
                        "Exit Local View (Numpad /)"
                    } else {
                        "Local View / Isolate Selected (Numpad /)"
                    };
                    if ui.button(local_txt).clicked() {
                        app.toggle_local_view();
                        ui.close();
                    }
                    if ui.button("Walk / Fly Navigation (Shift+F)").clicked() {
                        app.toggle_fly_navigation();
                        ui.close();
                    }
                });

                ui.menu_button("Select", |ui| {
                    if ui.button("All (A)").clicked() {
                        app.select_all();
                        ui.close();
                    }
                    if ui.button("None (Alt+A)").clicked() {
                        app.deselect_all();
                        ui.close();
                    }
                    if ui.button("Invert (Ctrl+I)").clicked() {
                        app.invert_selection();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Box Select (B)").clicked() {
                        app.editor_state.transform_mode = TransformMode::BoxSelect;
                        ui.close();
                    }
                    if ui.button("Circle Select (C)").clicked() {
                        app.editor_state.transform_mode = TransformMode::CircleSelect;
                        app.editor_state.circle_select_active = true;
                        ui.close();
                    }
                    if ui.button("Lasso Select").clicked() {
                        app.editor_state.transform_mode = TransformMode::LassoSelect;
                        ui.close();
                    }
                    if app.editor_state.edit_mode == EditMode::Edit {
                        ui.separator();
                        if ui.button("Select Linked (L)").clicked() {
                            app.select_linked(false);
                            ui.close();
                        }
                    }
                });

                ui.menu_button("Add", |ui| {
                    ui.menu_button("Mesh", |ui| {
                        if ui.button("Plane").clicked() {
                            add_scene_mesh(
                                app,
                                "Plane",
                                PrimitiveType::Plane,
                                generate_plane(2.0, 1),
                                [0.65, 0.65, 0.65, 1.0],
                                Vec3::ZERO,
                            );
                            ui.close();
                        }
                        if ui.button("Cube").clicked() {
                            add_scene_mesh(
                                app,
                                "Cube",
                                PrimitiveType::Cube,
                                generate_cube(1.0),
                                [0.72, 0.72, 0.72, 1.0],
                                Vec3::new(0.0, 0.5, 0.0),
                            );
                            ui.close();
                        }
                        if ui.button("Circle").clicked() {
                            add_scene_mesh(
                                app,
                                "Circle",
                                PrimitiveType::Circle,
                                generate_circle(1.0, 32),
                                [0.70, 0.70, 0.70, 1.0],
                                Vec3::ZERO,
                            );
                            ui.close();
                        }
                        if ui.button("UV Sphere").clicked() {
                            add_scene_mesh(
                                app,
                                "Sphere",
                                PrimitiveType::Sphere,
                                generate_uv_sphere(0.75, 24, 16),
                                [0.85, 0.55, 0.40, 1.0],
                                Vec3::new(0.0, 0.75, 0.0),
                            );
                            ui.close();
                        }
                        if ui.button("Icosphere").clicked() {
                            add_scene_mesh(
                                app,
                                "Icosphere",
                                PrimitiveType::Icosphere,
                                generate_icosphere(0.75, 2),
                                [0.45, 0.75, 0.45, 1.0],
                                Vec3::new(0.0, 0.75, 0.0),
                            );
                            ui.close();
                        }
                        if ui.button("Cylinder").clicked() {
                            add_scene_mesh(
                                app,
                                "Cylinder",
                                PrimitiveType::Cylinder,
                                generate_cylinder(0.6, 1.5, 24),
                                [0.75, 0.75, 0.35, 1.0],
                                Vec3::new(0.0, 0.75, 0.0),
                            );
                            ui.close();
                        }
                        if ui.button("Cone").clicked() {
                            add_scene_mesh(
                                app,
                                "Cone",
                                PrimitiveType::Cone,
                                generate_cone(0.8, 1.5, 24),
                                [0.85, 0.55, 0.35, 1.0],
                                Vec3::new(0.0, 0.75, 0.0),
                            );
                            ui.close();
                        }
                        if ui.button("Torus").clicked() {
                            add_scene_mesh(
                                app,
                                "Torus",
                                PrimitiveType::Torus,
                                generate_torus(1.0, 0.3, 24, 16),
                                [0.85, 0.35, 0.65, 1.0],
                                Vec3::new(0.0, 0.5, 0.0),
                            );
                            ui.close();
                        }
                        if ui.button("Suzanne (Monkey)").clicked() {
                            add_scene_mesh(
                                app,
                                "Suzanne",
                                PrimitiveType::Monkey,
                                generate_monkey(1.0),
                                [0.78, 0.52, 0.32, 1.0],
                                Vec3::new(0.0, 0.75, 0.0),
                            );
                            ui.close();
                        }
                    });

                    ui.separator();
                    ui.menu_button("Light", |ui| {
                        if ui.button("Point Light").clicked() {
                            let mesh = generate_uv_sphere(0.2, 12, 8);
                            let cursor = app.editor_state.cursor_3d;
                            let mut light = SceneObject::new_light(
                                "Point Light",
                                LightData {
                                    kind: LightKind::Point,
                                    color: [1.0, 0.95, 0.8],
                                    intensity: 40.0,
                                    range: 12.0,
                                    ..LightData::default()
                                },
                                mesh,
                            )
                            .with_transform(
                                Transform::from_translation(cursor + Vec3::new(2.0, 4.0, 2.0)),
                            );
                            light.visible = false;
                            let id = app.scene.add_object(light);
                            app.selection.select(id);
                            app.status_message = "Added Point Light".to_string();
                            ui.close();
                        }
                    });

                    if ui.button("Camera").clicked() {
                        let cursor = app.editor_state.cursor_3d;
                        let mut cam = SceneObject::new_camera("Camera");
                        cam.local_transform.translation = cursor + Vec3::new(0.0, 2.0, 4.0);
                        let id = app.scene.add_object(cam);
                        app.selection.select(id);
                        app.status_message = "Added Camera".to_string();
                        ui.close();
                    }
                });

                if app.editor_state.edit_mode == EditMode::Object {
                    ui.menu_button("Object", |ui| {
                        let has_sel = !app.selection.is_empty();
                        if ui
                            .add_enabled(has_sel, egui::Button::new("Shade Smooth"))
                            .clicked()
                        {
                            if let Some(aid) = app.selection.active_id
                                && let Some(obj) = app.scene.get_object_mut(aid)
                            {
                                obj.mesh.recalculate_normals();
                                if let Some(r) = &mut app.renderer {
                                    r.invalidate_mesh(&aid);
                                }
                                app.status_message = "Shade Smooth applied".to_string();
                            }
                            ui.close();
                        }
                        if ui
                            .add_enabled(has_sel, egui::Button::new("Shade Flat"))
                            .clicked()
                        {
                            app.status_message = "Shade Flat applied".to_string();
                            ui.close();
                        }
                        ui.separator();
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
                        ui.separator();
                        if ui
                            .add_enabled(has_sel, egui::Button::new("Duplicate (Shift+D)"))
                            .clicked()
                        {
                            app.duplicate_selected();
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                app.selection.selected_ids.len() > 1,
                                egui::Button::new("Join (Ctrl+J)"),
                            )
                            .clicked()
                        {
                            app.join_selected();
                            ui.close();
                        }
                        if ui
                            .add_enabled(has_sel, egui::Button::new("Delete (X / Del)"))
                            .clicked()
                        {
                            app.delete_selected();
                            ui.close();
                        }
                    });
                } else if app.editor_state.edit_mode == EditMode::Edit {
                    ui.menu_button("Mesh", |ui| {
                        if ui.button("Extrude (E)").clicked() {
                            app.begin_interactive_edit_tool(
                                InteractiveEditToolType::Extrude,
                                forge_math::Vec2::new(400.0, 300.0),
                            );
                            ui.close();
                        }
                        if ui.button("Inset Faces (I)").clicked() {
                            app.begin_interactive_edit_tool(
                                InteractiveEditToolType::Inset,
                                forge_math::Vec2::new(400.0, 300.0),
                            );
                            ui.close();
                        }
                        if ui.button("Bevel (Ctrl+B)").clicked() {
                            app.begin_interactive_edit_tool(
                                InteractiveEditToolType::Bevel,
                                forge_math::Vec2::new(400.0, 300.0),
                            );
                            ui.close();
                        }
                        if ui.button("Loop Cut (Ctrl+R)").clicked() {
                            app.begin_interactive_edit_tool(
                                InteractiveEditToolType::LoopCut,
                                forge_math::Vec2::new(400.0, 300.0),
                            );
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Subdivide").clicked() {
                            app.subdivide_selected_geometry();
                            ui.close();
                        }
                        if ui.button("Merge by Distance (Alt+M)").clicked() {
                            app.merge_by_distance_geometry(0.001);
                            ui.close();
                        }
                        if ui.button("Merge at Center (M)").clicked() {
                            app.merge_selected_geometry();
                            ui.close();
                        }
                        if ui.button("Separate (P)").clicked() {
                            app.separate_selected_geometry();
                            ui.close();
                        }
                    });
                }

                ui.separator();

                // 5. Center Section: Orientation, Pivot, Snap, Proportional
                let orient_label = match app.editor_state.transform_orientation {
                    TransformOrientation::Global => "🌐 Global ▾",
                    TransformOrientation::Local => "📐 Local ▾",
                };
                egui::ComboBox::from_id_salt("vp_header_orientation")
                    .selected_text(orient_label)
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

                let pivot_label = match app.editor_state.pivot_point {
                    PivotPoint::MedianPoint => "☍ Median Point ▾",
                    PivotPoint::ActiveElement => "☉ Active Element ▾",
                    PivotPoint::Cursor3D => "◎ 3D Cursor ▾",
                    PivotPoint::IndividualOrigins => "⚬ Individual Origins ▾",
                };
                egui::ComboBox::from_id_salt("vp_header_pivot")
                    .selected_text(pivot_label)
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
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::IndividualOrigins,
                            "Individual Origins",
                        );
                    });

                ui.separator();

                // Snapping: Magnet Toggle + Snap Element
                let snap_on = app.editor_state.snap_settings.enabled;
                let snap_btn =
                    egui::Button::new(egui::RichText::new("🧲").size(12.0).color(if snap_on {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    }))
                    .fill(if snap_on {
                        Color32::from_rgb(71, 114, 179)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .corner_radius(egui::CornerRadius::same(3));
                if ui
                    .add(snap_btn)
                    .on_hover_text("Toggle Snapping (Shift+Tab)")
                    .clicked()
                {
                    app.editor_state.snap_settings.enabled = !snap_on;
                }

                let snap_label = if app.editor_state.snap_settings.snap_to_vertex {
                    "Vertex ▾"
                } else {
                    "Increment ▾"
                };
                egui::ComboBox::from_id_salt("vp_header_snap_elem")
                    .selected_text(snap_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                app.editor_state.snap_settings.snap_to_grid
                                    && !app.editor_state.snap_settings.snap_to_vertex,
                                "Increment (Grid)",
                            )
                            .clicked()
                        {
                            app.editor_state.snap_settings.snap_to_grid = true;
                            app.editor_state.snap_settings.snap_to_vertex = false;
                        }
                        if ui
                            .selectable_label(
                                app.editor_state.snap_settings.snap_to_vertex,
                                "Vertex",
                            )
                            .clicked()
                        {
                            app.editor_state.snap_settings.snap_to_grid = false;
                            app.editor_state.snap_settings.snap_to_vertex = true;
                        }
                    });

                ui.separator();

                // Proportional Editing: Toggle + Radius
                let prop_on = app.editor_state.proportional_editing;
                let prop_btn =
                    egui::Button::new(egui::RichText::new("◎").size(13.0).color(if prop_on {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    }))
                    .fill(if prop_on {
                        Color32::from_rgb(71, 114, 179)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .corner_radius(egui::CornerRadius::same(3));
                if ui
                    .add(prop_btn)
                    .on_hover_text("Toggle Proportional Editing (O)")
                    .clicked()
                {
                    app.editor_state.proportional_editing = !prop_on;
                }

                ui.add(
                    egui::DragValue::new(&mut app.editor_state.proportional_radius)
                        .speed(0.1)
                        .range(0.1..=50.0)
                        .prefix("R: "),
                )
                .on_hover_text("Proportional Influence Radius (Scroll wheel during G/R/S)");

                // 6. Right Section: Options, Overlays, X-Ray, 4 Shading Spheres
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);

                    // 4 Shading Spheres (Rendered, Material Preview, Solid, Wireframe)
                    let is_wireframe = app.editor_state.show_wireframe;
                    let is_matpreview = app.show_material_editor && !is_wireframe;
                    let is_rendered = app.editor_state.viewport_matcap == ViewportMatcap::Studio
                        && !is_wireframe
                        && !is_matpreview;
                    let is_solid = !is_wireframe && !is_matpreview && !is_rendered;

                    if draw_shading_sphere(ui, is_rendered, 3, "Viewport Shading: Rendered (Z)")
                        .clicked()
                    {
                        app.editor_state.show_wireframe = false;
                        app.show_material_editor = false;
                        app.editor_state.viewport_matcap = ViewportMatcap::Studio;
                    }

                    if draw_shading_sphere(
                        ui,
                        is_matpreview,
                        2,
                        "Viewport Shading: Material Preview",
                    )
                    .clicked()
                    {
                        app.editor_state.show_wireframe = false;
                        app.show_material_editor = true;
                    }

                    if draw_shading_sphere(ui, is_solid, 1, "Viewport Shading: Solid").clicked() {
                        app.editor_state.show_wireframe = false;
                        app.show_material_editor = false;
                        app.editor_state.viewport_matcap = ViewportMatcap::Clay;
                    }

                    if draw_shading_sphere(
                        ui,
                        is_wireframe,
                        0,
                        "Viewport Shading: Wireframe (Shift+Z)",
                    )
                    .clicked()
                    {
                        app.editor_state.show_wireframe = !is_wireframe;
                    }

                    ui.separator();

                    // X-Ray Toggle Button
                    let xray_on = app.editor_state.show_xray;
                    let xray_btn =
                        egui::Button::new(egui::RichText::new("⧉").size(13.0).color(if xray_on {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(170, 170, 170)
                        }))
                        .fill(if xray_on {
                            Color32::from_rgb(71, 114, 179)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .corner_radius(egui::CornerRadius::same(3));
                    if ui
                        .add(xray_btn)
                        .on_hover_text("Toggle X-Ray Mode (Alt+Z)")
                        .clicked()
                    {
                        app.editor_state.show_xray = !xray_on;
                    }

                    // Viewport Overlays Dropdown Button
                    let overlays_on = app.editor_state.show_overlays;
                    let overlays_btn =
                        egui::Button::new(egui::RichText::new("◫ Overlays ▾").size(11.0).color(
                            if overlays_on {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(150, 150, 150)
                            },
                        ))
                        .fill(if app.show_overlays_dropdown {
                            Color32::from_rgb(50, 60, 75)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .corner_radius(egui::CornerRadius::same(3));

                    let overlays_resp = ui
                        .add(overlays_btn)
                        .on_hover_text("Viewport Overlays Popover (Shift+Alt+Z)");
                    overlays_button_rect = overlays_resp.rect;
                    if overlays_resp.clicked() {
                        app.show_overlays_dropdown = !app.show_overlays_dropdown;
                    }

                    ui.menu_button("Options ▾", |ui| {
                        ui.label(
                            egui::RichText::new("Transform & Navigation")
                                .weak()
                                .size(11.0),
                        );
                        ui.checkbox(
                            &mut app.editor_state.panel_settings.zoom_to_cursor,
                            "Zoom to Mouse Position",
                        );
                        ui.checkbox(
                            &mut app.editor_state.panel_settings.smooth_zoom,
                            "Smooth Zoom Inertia",
                        );
                        ui.separator();
                        ui.label(egui::RichText::new("Display").weak().size(11.0));
                        ui.checkbox(
                            &mut app.editor_state.show_debug_info,
                            "Show Diagnostics Telemetry",
                        );
                    });
                });
            });
        });

    // Viewport Overlays Popover Dropdown
    if app.show_overlays_dropdown && overlays_button_rect != egui::Rect::NOTHING {
        let popover_pos = overlays_button_rect.left_bottom() + egui::vec2(-80.0, 4.0);
        egui::Area::new(ui.make_persistent_id("vp_header_overlays_popover"))
            .fixed_pos(popover_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(220.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Viewport Overlays").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                app.show_overlays_dropdown = false;
                            }
                        });
                    });
                    ui.separator();
                    ui.checkbox(
                        &mut app.editor_state.show_overlays,
                        "Show Overlays (Shift+Alt+Z)",
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Guides")
                            .color(Color32::from_rgb(180, 190, 210))
                            .size(11.0),
                    );
                    ui.checkbox(&mut app.editor_state.show_grid, "Floor Grid");
                    ui.checkbox(&mut app.editor_state.show_axes, "Coordinate Axes");
                    ui.checkbox(&mut app.editor_state.show_cursor_3d, "3D Cursor");
                    ui.checkbox(&mut app.editor_state.show_origins, "Object Origins");
                    ui.checkbox(
                        &mut app.editor_state.show_nav_widget,
                        "Navigation Compass & Buttons",
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Geometry & Shading")
                            .color(Color32::from_rgb(180, 190, 210))
                            .size(11.0),
                    );
                    ui.checkbox(&mut app.editor_state.show_wireframe, "Wireframe");
                    ui.checkbox(&mut app.editor_state.show_xray, "X-Ray Mode (Alt+Z)");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Scene Statistics")
                            .color(Color32::from_rgb(180, 190, 210))
                            .size(11.0),
                    );
                    ui.checkbox(&mut app.editor_state.show_statistics, "Scene Statistics");
                    ui.checkbox(&mut app.editor_state.show_debug_info, "Debug Telemetry");
                });
            });
    }
}
