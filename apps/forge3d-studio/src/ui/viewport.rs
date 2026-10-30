use egui::{Color32, Key, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2 as EguiVec2};
use forge_geometry::EditSelectionMode;
use forge_math::{Transform, Vec2, Vec3};
use forge_render::{CameraProjection, ViewPreset};
use forge_scene::{Action, EditMode, TransformMode};

use crate::app::Forge3dStudioApp;
use crate::ui::gizmo::{render_and_interact_gizmo, world_to_screen_pos};

pub fn render_viewport(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let available_size = ui.available_size();
    if available_size.x < 10.0 || available_size.y < 10.0 {
        return;
    }

    // Allocate exact viewport area for pointer events
    let (rect, _response) = ui.allocate_exact_size(available_size, Sense::click_and_drag());

    // Update camera aspect ratio
    app.camera.set_aspect_ratio(rect.width(), rect.height());

    let input = ui.input(|i| i.clone());
    let mouse_pos = input.pointer.interact_pos();

    // -------------------------------------------------------------
    // 1. Keyboard Shortcuts (Camera Navigation, Modes, Transforms)
    // -------------------------------------------------------------
    // Mode Switch: Tab
    if input.key_pressed(Key::Tab) {
        app.toggle_edit_mode();
        ui.ctx().request_repaint();
    }

    // Numpad Camera Presets & Views
    if input.key_pressed(Key::Num1) {
        if input.modifiers.ctrl {
            app.camera.set_view_preset(ViewPreset::Back);
            app.status_message = "View: Back".to_string();
        } else {
            app.camera.set_view_preset(ViewPreset::Front);
            app.status_message = "View: Front".to_string();
        }
        ui.ctx().request_repaint();
    } else if input.key_pressed(Key::Num3) {
        if input.modifiers.ctrl {
            app.camera.set_view_preset(ViewPreset::Left);
            app.status_message = "View: Left".to_string();
        } else {
            app.camera.set_view_preset(ViewPreset::Right);
            app.status_message = "View: Right".to_string();
        }
        ui.ctx().request_repaint();
    } else if input.key_pressed(Key::Num7) {
        if input.modifiers.ctrl {
            app.camera.set_view_preset(ViewPreset::Bottom);
            app.status_message = "View: Bottom".to_string();
        } else {
            app.camera.set_view_preset(ViewPreset::Top);
            app.status_message = "View: Top".to_string();
        }
        ui.ctx().request_repaint();
    } else if input.key_pressed(Key::Num5) {
        app.camera.toggle_projection();
        app.status_message = format!("Camera: {:?}", app.camera.projection);
        ui.ctx().request_repaint();
    }

    // Framing: 'F' or Numpad Period
    if input.key_pressed(Key::F) {
        let bbox = app.scene.selected_aabb(&app.selection);
        app.camera.frame_box(&bbox);
        app.status_message = "Framed Selected".to_string();
        ui.ctx().request_repaint();
    } else if input.key_pressed(Key::Home) {
        let bbox = app.scene.scene_aabb();
        app.camera.frame_box(&bbox);
        app.status_message = "Framed All".to_string();
        ui.ctx().request_repaint();
    }

    // Transform Mode Shortcuts (Q, W, E, R)
    if !app.editor_state.modal_transform.active && !input.modifiers.command && !input.modifiers.ctrl
    {
        if input.key_pressed(Key::Q) {
            app.editor_state.transform_mode = TransformMode::Select;
            app.status_message = "Tool: Select (Q)".to_string();
        } else if input.key_pressed(Key::W) {
            app.editor_state.transform_mode = TransformMode::Translate;
            app.status_message = "Tool: Move (W)".to_string();
        } else if input.key_pressed(Key::E) {
            if app.editor_state.edit_mode == EditMode::Edit {
                // Extrude in edit mode
                app.extrude_selected_geometry(0.5);
            } else {
                app.editor_state.transform_mode = TransformMode::Rotate;
                app.status_message = "Tool: Rotate (E)".to_string();
            }
        } else if input.key_pressed(Key::R) {
            app.editor_state.transform_mode = TransformMode::Scale;
            app.status_message = "Tool: Scale (R)".to_string();
        }
    }

    // Edit Mode Selection Element Shortcuts (1 = Vertex, 2 = Edge, 3 = Face)
    if app.editor_state.edit_mode == EditMode::Edit {
        if input.key_pressed(Key::Num1) && !input.modifiers.any() {
            app.editor_state.selection_element = forge_scene::SelectionElement::Vertex;
            app.status_message = "Selection: Vertex (1)".to_string();
        } else if input.key_pressed(Key::Num2) && !input.modifiers.any() {
            app.editor_state.selection_element = forge_scene::SelectionElement::Edge;
            app.status_message = "Selection: Edge (2)".to_string();
        } else if input.key_pressed(Key::Num3) && !input.modifiers.any() {
            app.editor_state.selection_element = forge_scene::SelectionElement::Face;
            app.status_message = "Selection: Face (3)".to_string();
        } else if input.key_pressed(Key::I) {
            app.inset_selected_geometry(0.15);
        } else if input.key_pressed(Key::M) {
            app.merge_selected_geometry();
        } else if input.key_pressed(Key::F) {
            app.fill_selected_geometry();
        }
    }

    // Undo / Redo Shortcuts (Ctrl+Z, Ctrl+Shift+Z / Ctrl+Y)
    if (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(Key::Z) {
        if input.modifiers.shift {
            app.redo();
        } else {
            app.undo();
        }
        ui.ctx().request_repaint();
    } else if (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(Key::Y) {
        app.redo();
        ui.ctx().request_repaint();
    }

    // Reset Transform Shortcuts: Alt+G, Alt+R, Alt+S
    if input.modifiers.alt
        && let Some(active_id) = app.selection.active_id
    {
        if input.key_pressed(Key::G) {
            if let Some(obj) = app.scene.get_object_mut(active_id) {
                let old_tf = obj.local_transform;
                obj.reset_translation();
                let new_tf = obj.local_transform;
                app.history.push(Action::Transform {
                    id: active_id,
                    old_transform: old_tf,
                    new_transform: new_tf,
                });
                app.status_message = "Reset Position".to_string();
                ui.ctx().request_repaint();
            }
        } else if input.key_pressed(Key::R) {
            if let Some(obj) = app.scene.get_object_mut(active_id) {
                let old_tf = obj.local_transform;
                obj.reset_rotation();
                let new_tf = obj.local_transform;
                app.history.push(Action::Transform {
                    id: active_id,
                    old_transform: old_tf,
                    new_transform: new_tf,
                });
                app.status_message = "Reset Rotation".to_string();
                ui.ctx().request_repaint();
            }
        } else if input.key_pressed(Key::S)
            && let Some(obj) = app.scene.get_object_mut(active_id)
        {
            let old_tf = obj.local_transform;
            obj.reset_scale();
            let new_tf = obj.local_transform;
            app.history.push(Action::Transform {
                id: active_id,
                old_transform: old_tf,
                new_transform: new_tf,
            });
            app.status_message = "Reset Scale".to_string();
            ui.ctx().request_repaint();
        }
    }

    // Duplicate Shortcut: Shift+D
    if input.modifiers.shift && input.key_pressed(Key::D) {
        app.duplicate_selected();
        ui.ctx().request_repaint();
    }

    // Delete Shortcut: Delete or X
    if input.key_pressed(Key::Delete)
        || (!app.editor_state.modal_transform.active && input.key_pressed(Key::X))
    {
        app.delete_selected();
        ui.ctx().request_repaint();
    }

    // -------------------------------------------------------------
    // 2. Camera Navigation: Orbit, Pan, Zoom, Dolly
    // -------------------------------------------------------------
    let is_pointer_in_viewport = mouse_pos.map(|p| rect.contains(p)).unwrap_or(false);

    // MMB Drag or Alt+LMB Drag -> Orbit / Pan / Dolly
    let mmb_down = input.pointer.button_down(PointerButton::Middle);
    let alt_lmb_down = input.modifiers.alt && input.pointer.button_down(PointerButton::Primary);

    if (mmb_down || alt_lmb_down) && is_pointer_in_viewport {
        let delta = input.pointer.delta();
        if delta.length_sq() > 0.0 {
            if input.modifiers.shift {
                // Pan
                app.camera.pan(delta.x, delta.y);
            } else if input.modifiers.ctrl {
                // Smooth Dolly Zoom
                app.camera.dolly(delta.y * 0.04);
            } else {
                // Orbit
                app.camera.orbit(delta.x * 0.007, delta.y * 0.007);
            }
            ui.ctx().request_repaint();
        }
    }

    // Mouse Wheel: Zoom
    if is_pointer_in_viewport && input.smooth_scroll_delta.y.abs() > 0.0 {
        let zoom_factor = input.smooth_scroll_delta.y * 0.02;
        app.camera.zoom(zoom_factor);
        ui.ctx().request_repaint();
    }

    // -------------------------------------------------------------
    // 3. Paint 3D Viewport Texture
    // -------------------------------------------------------------
    let painter = ui.painter_at(rect);
    if let Some(texture_id) = app.viewport_egui_texture_id {
        painter.image(
            texture_id,
            rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(rect, 0.0, Color32::from_rgb(26, 28, 33));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Initializing 3D Viewport...",
            egui::FontId::proportional(18.0),
            Color32::from_gray(160),
        );
    }

    // -------------------------------------------------------------
    // 4. Interactive 3D Gizmo (Move, Rotate, Scale)
    // -------------------------------------------------------------
    let lmb_pressed =
        input.pointer.button_pressed(PointerButton::Primary) && is_pointer_in_viewport;
    let lmb_down = input.pointer.button_down(PointerButton::Primary);
    let lmb_released = input.pointer.button_released(PointerButton::Primary);

    let view_proj = app.camera.view_proj_matrix();
    let eye = app.camera.eye;
    let screen_to_ray = |sp: Vec2| {
        app.camera
            .screen_to_ray(sp, Vec2::new(rect.width(), rect.height()))
    };

    let mut gizmo_consumed = false;

    if app.editor_state.edit_mode == EditMode::Object {
        if let Some(active_id) = app.selection.active_id
            && let Some(obj) = app.scene.get_object_mut(active_id)
        {
            let pivot_world = obj.local_transform.translation;
            let old_transform = obj.local_transform;

            gizmo_consumed = render_and_interact_gizmo(
                &mut app.gizmo_state,
                &painter,
                view_proj,
                eye,
                screen_to_ray,
                rect,
                pivot_world,
                &mut obj.local_transform,
                app.editor_state.transform_mode,
                app.editor_state.transform_orientation,
                mouse_pos,
                lmb_pressed && !input.modifiers.alt,
                lmb_down && !input.modifiers.alt,
                lmb_released,
                app.editor_state.snap_settings.enabled,
                app.editor_state.snap_settings.grid_increment,
            );

            if gizmo_consumed {
                ui.ctx().request_repaint();
            }

            // Push to undo stack when gizmo drag finishes
            if lmb_released && old_transform != obj.local_transform {
                app.history.push(Action::Transform {
                    id: active_id,
                    old_transform,
                    new_transform: obj.local_transform,
                });
                app.scene.dirty = true;
            }
        }
    } else if app.editor_state.edit_mode == EditMode::Edit {
        // Edit Mode: Overlay vertices, edges, and interact with geometry
        if let Some((_obj_id, edit_mesh)) = &mut app.active_edit_mesh {
            // Draw wireframe edges
            for edge in &edit_mesh.edges {
                let p0 = edit_mesh.vertices[edge.v0].position;
                let p1 = edit_mesh.vertices[edge.v1].position;
                if let (Some(s0), Some(s1)) = (
                    world_to_screen_pos(p0, view_proj, rect),
                    world_to_screen_pos(p1, view_proj, rect),
                ) {
                    let col = if edge.selected {
                        Color32::from_rgb(255, 170, 40)
                    } else {
                        Color32::from_rgba_unmultiplied(160, 170, 190, 120)
                    };
                    painter.line_segment(
                        [s0, s1],
                        Stroke::new(if edge.selected { 2.5 } else { 1.2 }, col),
                    );
                }
            }

            // Draw vertex points
            for vertex in &edit_mesh.vertices {
                if let Some(sp) = world_to_screen_pos(vertex.position, view_proj, rect) {
                    let col = if vertex.selected {
                        Color32::from_rgb(255, 180, 40)
                    } else {
                        Color32::from_rgb(40, 45, 55)
                    };
                    let radius = if vertex.selected { 5.0 } else { 3.5 };
                    painter.circle_filled(sp, radius, col);
                    painter.circle_stroke(sp, radius, Stroke::new(1.0, Color32::from_gray(220)));
                }
            }

            // Selection center gizmo in edit mode
            let edit_mode_sel = match app.editor_state.selection_element {
                forge_scene::SelectionElement::Vertex => EditSelectionMode::Vertex,
                forge_scene::SelectionElement::Edge => EditSelectionMode::Edge,
                forge_scene::SelectionElement::Face => EditSelectionMode::Face,
            };

            if let Some(pivot) = edit_mesh.selection_center(edit_mode_sel) {
                let mut dummy_tf = Transform::from_translation(pivot);
                let old_pos = dummy_tf.translation;

                gizmo_consumed = render_and_interact_gizmo(
                    &mut app.gizmo_state,
                    &painter,
                    view_proj,
                    eye,
                    screen_to_ray,
                    rect,
                    pivot,
                    &mut dummy_tf,
                    app.editor_state.transform_mode,
                    app.editor_state.transform_orientation,
                    mouse_pos,
                    lmb_pressed && !input.modifiers.alt,
                    lmb_down && !input.modifiers.alt,
                    lmb_released,
                    app.editor_state.snap_settings.enabled,
                    app.editor_state.snap_settings.grid_increment,
                );

                let delta = dummy_tf.translation - old_pos;
                if delta.length_squared() > 1e-6 {
                    edit_mesh.translate_selection(delta, edit_mode_sel);
                    app.sync_edit_mesh_to_scene();
                    ui.ctx().request_repaint();
                }
            }
        }
    }

    // -------------------------------------------------------------
    // 5. Left Click: Object Selection Raycast (if gizmo not clicked)
    // -------------------------------------------------------------
    if lmb_pressed
        && !gizmo_consumed
        && !input.modifiers.alt
        && is_pointer_in_viewport
        && let Some(pos) = mouse_pos
    {
        let local_pos = pos - rect.min;
        let screen_pos = Vec2::new(local_pos.x, local_pos.y);
        let viewport_size = Vec2::new(rect.width(), rect.height());

        let ray = app.camera.screen_to_ray(screen_pos, viewport_size);

        if app.editor_state.edit_mode == EditMode::Object {
            if let Some((hit_id, _dist)) = app.scene.raycast(&ray) {
                if input.modifiers.shift {
                    app.selection.toggle_selection(hit_id);
                } else {
                    app.selection.select(hit_id);
                }
                if let Some(obj) = app.scene.get_object(hit_id) {
                    app.status_message = format!("Selected: {}", obj.name);
                }
            } else if !input.modifiers.shift {
                app.selection.deselect();
                app.status_message = "Selection cleared".to_string();
            }
            ui.ctx().request_repaint();
        } else if app.editor_state.edit_mode == EditMode::Edit {
            // Raycast closest vertex or element in Edit Mode
            if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
                let mut closest_vert = None;
                let mut min_dist_px = 15.0; // 15px picking radius

                for (idx, v) in edit_mesh.vertices.iter().enumerate() {
                    if let Some(sp) = world_to_screen_pos(v.position, view_proj, rect) {
                        let d = pos.distance(sp);
                        if d < min_dist_px {
                            min_dist_px = d;
                            closest_vert = Some(idx);
                        }
                    }
                }

                if let Some(v_idx) = closest_vert {
                    if input.modifiers.shift {
                        edit_mesh.vertices[v_idx].selected = !edit_mesh.vertices[v_idx].selected;
                    } else {
                        edit_mesh.deselect_all(EditSelectionMode::Vertex);
                        edit_mesh.vertices[v_idx].selected = true;
                    }
                    app.status_message = format!("Selected Vertex #{}", v_idx);
                } else if !input.modifiers.shift {
                    edit_mesh.deselect_all(EditSelectionMode::Vertex);
                }
                app.sync_edit_mesh_to_scene();
                ui.ctx().request_repaint();
            }
        }
    }

    // -------------------------------------------------------------
    // 6. Viewport HUD & Overlays
    // -------------------------------------------------------------
    // Top-left Mode badge
    let mode_str = match app.editor_state.edit_mode {
        EditMode::Object => "Object Mode".to_string(),
        EditMode::Edit => format!("Edit Mode ({:?})", app.editor_state.selection_element),
    };
    let tool_str = format!("{:?}", app.editor_state.transform_mode);
    let proj_str = match app.camera.projection {
        CameraProjection::Perspective => "Perspective",
        CameraProjection::Orthographic => "Orthographic",
    };

    painter.text(
        rect.min + EguiVec2::new(14.0, 14.0),
        egui::Align2::LEFT_TOP,
        format!("{mode_str} | Tool: {tool_str} | {proj_str}"),
        egui::FontId::proportional(14.0),
        Color32::from_gray(210),
    );

    // Orientation Gizmo indicator in top-right
    let gizmo_center = rect.right_top() + EguiVec2::new(-45.0, 45.0);
    let view_mat = app.camera.view_matrix();
    let draw_axis = |axis: Vec3, color: Color32| {
        let transformed = view_mat.transform_vector3(axis);
        let end = gizmo_center + EguiVec2::new(transformed.x * 26.0, -transformed.y * 26.0);
        painter.line_segment([gizmo_center, end], Stroke::new(2.5, color));
    };

    draw_axis(Vec3::X, Color32::from_rgb(235, 65, 65)); // X - Red
    draw_axis(Vec3::Y, Color32::from_rgb(65, 225, 75)); // Y - Green
    draw_axis(Vec3::Z, Color32::from_rgb(65, 130, 245)); // Z - Blue
}
