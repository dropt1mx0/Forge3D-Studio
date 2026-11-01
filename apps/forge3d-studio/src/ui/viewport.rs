use egui::{Color32, Key, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2 as EguiVec2};
use forge_geometry::EditSelectionMode;
use forge_math::{Quat, Vec2, Vec3};
use forge_render::{CameraProjection, ViewPreset};
use forge_scene::{Action, AxisConstraint, EditMode, PointerInteractionState, TransformMode};

use crate::app::Forge3dStudioApp;
use crate::ui::gizmo::{render_and_interact_gizmo, world_to_screen_pos};

pub fn render_viewport(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let available_size = ui.available_size();
    if available_size.x < 10.0 || available_size.y < 10.0 {
        return;
    }

    // Allocate exact viewport area for pointer and keyboard interaction
    let (rect, response) = ui.allocate_exact_size(available_size, Sense::click_and_drag());

    // Update camera aspect ratio
    app.camera.set_aspect_ratio(rect.width(), rect.height());

    let input = ui.input(|i| i.clone());
    let mouse_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()));
    let is_pointer_in_viewport = response.hovered()
        || response.contains_pointer()
        || mouse_pos.map(|p| rect.contains(p)).unwrap_or(false);
    let dt = (app.frame_time_ms / 1000.0).clamp(0.001, 0.1);

    let wants_keyboard = ui.ctx().egui_wants_keyboard_input();
    let mut modal_consumed_click = false;

    // =========================================================================
    // 1. FLY / WALK NAVIGATION MODE
    // =========================================================================
    // Toggle Fly Nav: Shift + F (or View -> Fly Navigation menu)
    if !wants_keyboard
        && input.modifiers.shift
        && input.key_pressed(Key::F)
        && !app.editor_state.modal_transform.active
    {
        app.toggle_fly_navigation();
        ui.ctx().request_repaint();
    }

    if app.editor_state.fly_nav_active {
        // Exit fly navigation on Escape, Enter, or Space
        if input.key_pressed(Key::Escape)
            || input.key_pressed(Key::Enter)
            || input.key_pressed(Key::Space)
        {
            app.editor_state.fly_nav_active = false;
            app.status_message = "Exited Fly Navigation".to_string();
            ui.ctx().request_repaint();
        } else {
            // Speed modifiers: Shift (boost), Ctrl (slow)
            let mut speed_mult = 1.0;
            if input.modifiers.shift {
                speed_mult = 2.5;
            }
            if input.modifiers.ctrl {
                speed_mult = 0.3;
            }

            // Directional movement in camera space: Z = forward/back, X = right/left, Y = up/down
            let mut move_dir = Vec3::ZERO;
            if input.key_down(Key::W) {
                move_dir.z += 1.0;
            }
            if input.key_down(Key::S) {
                move_dir.z -= 1.0;
            }
            if input.key_down(Key::D) {
                move_dir.x += 1.0;
            }
            if input.key_down(Key::A) {
                move_dir.x -= 1.0;
            }
            if input.key_down(Key::E) {
                move_dir.y += 1.0;
            }
            if input.key_down(Key::Q) {
                move_dir.y -= 1.0;
            }

            if move_dir.length_squared() > 0.0 {
                app.camera.fly_move(
                    move_dir.normalize(),
                    dt,
                    speed_mult,
                    app.editor_state.walk_mode,
                );
                ui.ctx().request_repaint();
            }

            // Mouse look: delta
            let delta = input.pointer.delta();
            if delta.length_sq() > 0.0 {
                app.camera.fly_look(delta.x * 0.003, delta.y * 0.003);
                ui.ctx().request_repaint();
            }

            // Mouse wheel adjusts fly speed
            if input.smooth_scroll_delta.y.abs() > 0.0 {
                app.camera.settings.fly_speed = (app.camera.settings.fly_speed
                    + input.smooth_scroll_delta.y * 0.05)
                    .clamp(0.5, 50.0);
                ui.ctx().request_repaint();
            }

            // Tab toggles walk constraint
            if input.key_pressed(Key::Tab) {
                app.editor_state.walk_mode = !app.editor_state.walk_mode;
                ui.ctx().request_repaint();
            }
        }
    }

    // =========================================================================
    // 2. MODAL TRANSFORMATION (G, R, S, Axes, Numeric Input, Confirm/Cancel)
    // =========================================================================
    if app.editor_state.modal_transform.active {
        // A. Axis constraints
        if input.key_pressed(Key::X) {
            if input.modifiers.shift {
                app.editor_state
                    .modal_transform
                    .set_or_toggle_axis(AxisConstraint::PlaneYZ);
            } else {
                app.editor_state
                    .modal_transform
                    .set_or_toggle_axis(AxisConstraint::X);
            }
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Y) {
            if input.modifiers.shift {
                app.editor_state
                    .modal_transform
                    .set_or_toggle_axis(AxisConstraint::PlaneXZ);
            } else {
                app.editor_state
                    .modal_transform
                    .set_or_toggle_axis(AxisConstraint::Y);
            }
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Z) {
            if input.modifiers.shift {
                app.editor_state
                    .modal_transform
                    .set_or_toggle_axis(AxisConstraint::PlaneXY);
            } else {
                app.editor_state
                    .modal_transform
                    .set_or_toggle_axis(AxisConstraint::Z);
            }
            ui.ctx().request_repaint();
        }

        // B. Numeric input parsing (0..9, minus, period, backspace)
        let numpad_keys = [
            (Key::Num0, '0'),
            (Key::Num1, '1'),
            (Key::Num2, '2'),
            (Key::Num3, '3'),
            (Key::Num4, '4'),
            (Key::Num5, '5'),
            (Key::Num6, '6'),
            (Key::Num7, '7'),
            (Key::Num8, '8'),
            (Key::Num9, '9'),
        ];
        for (k, c) in numpad_keys {
            if input.key_pressed(k) && !input.modifiers.any() {
                app.editor_state.modal_transform.push_char(c);
                ui.ctx().request_repaint();
            }
        }
        if input.key_pressed(Key::Minus) {
            app.editor_state.modal_transform.push_char('-');
            ui.ctx().request_repaint();
        }
        if input.key_pressed(Key::Period) {
            app.editor_state.modal_transform.push_char('.');
            ui.ctx().request_repaint();
        }
        if input.key_pressed(Key::Backspace) {
            app.editor_state.modal_transform.pop_char();
            ui.ctx().request_repaint();
        }

        // Handle text events for modal numeric input
        for event in &input.events {
            if let egui::Event::Text(text) = event {
                for c in text.chars() {
                    if c.is_ascii_digit() || c == '.' || c == '-' {
                        app.editor_state.modal_transform.push_char(c);
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

        // C. Confirm (Enter or LMB) / Cancel (Escape or RMB)
        let lmb_clicked = input.pointer.button_clicked(PointerButton::Primary);
        let rmb_clicked = input.pointer.button_clicked(PointerButton::Secondary);

        if input.key_pressed(Key::Enter) || lmb_clicked {
            app.apply_modal_transform();
            modal_consumed_click = true;
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Escape) || rmb_clicked {
            app.cancel_modal_transform();
            modal_consumed_click = true;
            ui.ctx().request_repaint();
        } else {
            // D. Real-time calculation of modal transform delta
            let numeric_val = app.editor_state.modal_transform.parsed_number();
            let mouse = mouse_pos.unwrap_or_else(|| {
                Pos2::new(
                    rect.min.x + app.editor_state.modal_transform.start_mouse_pos.x,
                    rect.min.y + app.editor_state.modal_transform.start_mouse_pos.y,
                )
            });
            let local_mouse = Vec2::new(mouse.x - rect.min.x, mouse.y - rect.min.y);
            let mouse_delta = local_mouse - app.editor_state.modal_transform.start_mouse_pos;
            let pivot = app.editor_state.modal_transform.pivot_world;

            match app.editor_state.modal_transform.mode {
                TransformMode::Translate => {
                    let mut translate_delta = Vec3::ZERO;

                    if let Some(num) = numeric_val {
                        // Precise numeric displacement
                        match app.editor_state.modal_transform.constraint {
                            Some(AxisConstraint::X) => translate_delta.x = num,
                            Some(AxisConstraint::Y) => translate_delta.y = num,
                            Some(AxisConstraint::Z) => translate_delta.z = num,
                            Some(AxisConstraint::PlaneXY) => {
                                translate_delta.x = num;
                                translate_delta.y = num;
                            }
                            Some(AxisConstraint::PlaneXZ) => {
                                translate_delta.x = num;
                                translate_delta.z = num;
                            }
                            Some(AxisConstraint::PlaneYZ) => {
                                translate_delta.y = num;
                                translate_delta.z = num;
                            }
                            None => translate_delta = app.camera.right() * num,
                        }
                    } else {
                        // Mouse movement displacement
                        let dist = (app.camera.eye - pivot).length().max(1.0);
                        let factor = dist * 0.002;

                        let cam_right = app.camera.right();
                        let cam_up = app.camera.camera_up();
                        let view_plane_delta =
                            cam_right * mouse_delta.x * factor + cam_up * (-mouse_delta.y) * factor;

                        match app.editor_state.modal_transform.constraint {
                            Some(AxisConstraint::X) => {
                                translate_delta.x = view_plane_delta.dot(Vec3::X);
                            }
                            Some(AxisConstraint::Y) => {
                                translate_delta.y = view_plane_delta.dot(Vec3::Y);
                            }
                            Some(AxisConstraint::Z) => {
                                translate_delta.z = view_plane_delta.dot(Vec3::Z);
                            }
                            Some(AxisConstraint::PlaneXY) => {
                                translate_delta.x = view_plane_delta.x;
                                translate_delta.y = view_plane_delta.y;
                            }
                            Some(AxisConstraint::PlaneXZ) => {
                                translate_delta.x = view_plane_delta.x;
                                translate_delta.z = view_plane_delta.z;
                            }
                            Some(AxisConstraint::PlaneYZ) => {
                                translate_delta.y = view_plane_delta.y;
                                translate_delta.z = view_plane_delta.z;
                            }
                            None => translate_delta = view_plane_delta,
                        }

                        if app.editor_state.snap_settings.enabled
                            && app.editor_state.snap_settings.grid_increment > 0.0
                        {
                            let step = app.editor_state.snap_settings.grid_increment;
                            translate_delta = (translate_delta / step).round() * step;
                        }
                    }

                    // Apply to initial transforms
                    for (&id, &initial_tf) in &app.editor_state.modal_transform.initial_transforms {
                        if let Some(obj) = app.scene.get_object_mut(id) {
                            obj.local_transform.translation =
                                initial_tf.translation + translate_delta;
                        }
                    }
                    app.scene.dirty = true;
                    ui.ctx().request_repaint();
                }
                TransformMode::Rotate => {
                    let angle_rad = if let Some(num) = numeric_val {
                        num.to_radians()
                    } else {
                        let center_screen =
                            world_to_screen_pos(pivot, app.camera.view_proj_matrix(), rect)
                                .unwrap_or(rect.center());
                        let cur_angle =
                            (mouse.y - center_screen.y).atan2(mouse.x - center_screen.x);
                        let mut diff = cur_angle - app.editor_state.modal_transform.start_angle_rad;
                        if app.editor_state.snap_settings.enabled {
                            let step = 15.0_f32.to_radians();
                            diff = (diff / step).round() * step;
                        }
                        diff
                    };

                    let rot_axis = match app.editor_state.modal_transform.constraint {
                        Some(AxisConstraint::X) => Vec3::X,
                        Some(AxisConstraint::Y) => Vec3::Y,
                        Some(AxisConstraint::Z) => Vec3::Z,
                        _ => -app.camera.forward(), // View-aligned rotation
                    };

                    let rot_quat = Quat::from_axis_angle(rot_axis, angle_rad);

                    for (&id, &initial_tf) in &app.editor_state.modal_transform.initial_transforms {
                        if let Some(obj) = app.scene.get_object_mut(id) {
                            obj.local_transform.rotation = rot_quat * initial_tf.rotation;
                            if app.editor_state.pivot_point
                                != forge_scene::PivotPoint::IndividualOrigins
                            {
                                let offset = initial_tf.translation - pivot;
                                obj.local_transform.translation = pivot + rot_quat * offset;
                            }
                        }
                    }
                    app.scene.dirty = true;
                    ui.ctx().request_repaint();
                }
                TransformMode::Scale => {
                    let scale_factor = if let Some(num) = numeric_val {
                        num.max(0.001)
                    } else {
                        let center_screen =
                            world_to_screen_pos(pivot, app.camera.view_proj_matrix(), rect)
                                .unwrap_or(rect.center());
                        let cur_dist = mouse.distance(center_screen);
                        let base_dist = app
                            .editor_state
                            .modal_transform
                            .start_distance_from_pivot
                            .max(20.0);
                        (cur_dist / base_dist).clamp(0.01, 100.0)
                    };

                    let scale_vec = match app.editor_state.modal_transform.constraint {
                        Some(AxisConstraint::X) => Vec3::new(scale_factor, 1.0, 1.0),
                        Some(AxisConstraint::Y) => Vec3::new(1.0, scale_factor, 1.0),
                        Some(AxisConstraint::Z) => Vec3::new(1.0, 1.0, scale_factor),
                        Some(AxisConstraint::PlaneXY) => Vec3::new(scale_factor, scale_factor, 1.0),
                        Some(AxisConstraint::PlaneXZ) => Vec3::new(scale_factor, 1.0, scale_factor),
                        Some(AxisConstraint::PlaneYZ) => Vec3::new(1.0, scale_factor, scale_factor),
                        None => Vec3::splat(scale_factor),
                    };

                    for (&id, &initial_tf) in &app.editor_state.modal_transform.initial_transforms {
                        if let Some(obj) = app.scene.get_object_mut(id) {
                            obj.local_transform.scale = initial_tf.scale * scale_vec;
                            if app.editor_state.pivot_point
                                != forge_scene::PivotPoint::IndividualOrigins
                            {
                                let offset = initial_tf.translation - pivot;
                                obj.local_transform.translation = pivot + offset * scale_vec;
                            }
                        }
                    }
                    app.scene.dirty = true;
                    ui.ctx().request_repaint();
                }
                _ => {}
            }
        }
    }

    // =========================================================================
    // 3. GLOBAL KEYBOARD SHORTCUTS (Only when NOT in modal transform or fly nav)
    // =========================================================================
    if !wants_keyboard
        && !app.editor_state.modal_transform.active
        && !app.editor_state.fly_nav_active
    {
        // Mode Switch: Tab
        if input.key_pressed(Key::Tab) {
            app.toggle_edit_mode();
            ui.ctx().request_repaint();
        }

        // Camera View (Numpad 0)
        let num0 = input.key_pressed(Key::Num0);
        if num0 {
            if input.modifiers.ctrl && input.modifiers.alt {
                // Align Active Camera to View
                app.align_active_camera_to_view();
            } else {
                app.toggle_camera_view();
            }
            ui.ctx().request_repaint();
        }

        // Standard Views (Numpad 1, 3, 7, 5, 2, 4, 6, 8, Period)
        let emulate =
            app.editor_state.emulate_numpad && app.editor_state.edit_mode == EditMode::Object;

        if input.key_pressed(Key::Num1) || (emulate && input.key_pressed(Key::Num1)) {
            if input.modifiers.ctrl {
                app.camera.set_view_preset(ViewPreset::Back);
                app.status_message = "View: Back".to_string();
            } else {
                app.camera.set_view_preset(ViewPreset::Front);
                app.status_message = "View: Front".to_string();
            }
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num3) || (emulate && input.key_pressed(Key::Num3)) {
            if input.modifiers.ctrl {
                app.camera.set_view_preset(ViewPreset::Left);
                app.status_message = "View: Left".to_string();
            } else {
                app.camera.set_view_preset(ViewPreset::Right);
                app.status_message = "View: Right".to_string();
            }
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num7) || (emulate && input.key_pressed(Key::Num7)) {
            if input.modifiers.ctrl {
                app.camera.set_view_preset(ViewPreset::Bottom);
                app.status_message = "View: Bottom".to_string();
            } else {
                app.camera.set_view_preset(ViewPreset::Top);
                app.status_message = "View: Top".to_string();
            }
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num5) || (emulate && input.key_pressed(Key::Num5)) {
            app.camera.toggle_projection();
            app.status_message = format!("Camera: {:?}", app.camera.projection);
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num2) || (emulate && input.key_pressed(Key::Num2)) {
            app.camera.step_rotate(0, 1);
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num8) || (emulate && input.key_pressed(Key::Num8)) {
            app.camera.step_rotate(0, -1);
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num4) || (emulate && input.key_pressed(Key::Num4)) {
            app.camera.step_rotate(-1, 0);
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Num6) || (emulate && input.key_pressed(Key::Num6)) {
            app.camera.step_rotate(1, 0);
            ui.ctx().request_repaint();
        }

        // Framing: 'F' or Home
        if input.key_pressed(Key::F) {
            app.frame_selected();
            ui.ctx().request_repaint();
        } else if input.key_pressed(Key::Home) {
            app.frame_all();
            ui.ctx().request_repaint();
        }

        // Start Modal Transformations (G = Move, R = Rotate, S = Scale)
        let mouse_vec2 = mouse_pos
            .map(|p| Vec2::new(p.x - rect.min.x, p.y - rect.min.y))
            .unwrap_or(Vec2::new(rect.width() * 0.5, rect.height() * 0.5));

        if !input.modifiers.command
            && !input.modifiers.ctrl
            && !input.modifiers.alt
            && !input.modifiers.shift
        {
            if input.key_pressed(Key::G) {
                app.editor_state.active_gizmo_mode = forge_scene::GizmoMode::Translate;
                app.editor_state.transform_mode = TransformMode::Translate;
                app.begin_modal_transform(TransformMode::Translate, mouse_vec2);
                ui.ctx().request_repaint();
            } else if input.key_pressed(Key::R) && app.editor_state.edit_mode == EditMode::Object {
                app.editor_state.active_gizmo_mode = forge_scene::GizmoMode::Rotate;
                app.editor_state.transform_mode = TransformMode::Rotate;
                app.begin_modal_transform(TransformMode::Rotate, mouse_vec2);
                ui.ctx().request_repaint();
            } else if input.key_pressed(Key::S) {
                app.editor_state.active_gizmo_mode = forge_scene::GizmoMode::Scale;
                app.editor_state.transform_mode = TransformMode::Scale;
                app.begin_modal_transform(TransformMode::Scale, mouse_vec2);
                ui.ctx().request_repaint();
            }
        }

        // Selection Shortcuts (A = Select All, Alt+A = Deselect All, Ctrl+I = Invert Selection, B = Box Select)
        if !input.modifiers.command
            && !input.modifiers.ctrl
            && !input.modifiers.alt
            && !input.modifiers.shift
        {
            if input.key_pressed(Key::A) {
                app.select_all();
                ui.ctx().request_repaint();
            } else if input.key_pressed(Key::B) {
                app.editor_state.transform_mode = TransformMode::BoxSelect;
                app.status_message = "Box Select: Drag over objects to select".to_string();
                ui.ctx().request_repaint();
            }
        } else if input.modifiers.alt && input.key_pressed(Key::A) {
            app.deselect_all();
            ui.ctx().request_repaint();
        } else if (input.modifiers.ctrl || input.modifiers.command) && input.key_pressed(Key::I) {
            app.invert_selection();
            ui.ctx().request_repaint();
        }

        // Tool Selector (Q, W, E, R, Shift+R)
        if !input.modifiers.command && !input.modifiers.ctrl && !input.modifiers.alt {
            if input.key_pressed(Key::Q) {
                app.editor_state.transform_mode = TransformMode::Select;
                app.status_message = "Tool: Select (Q)".to_string();
            } else if input.key_pressed(Key::W) {
                app.editor_state.transform_mode = TransformMode::Translate;
                app.status_message = "Tool: Move (W)".to_string();
            } else if input.key_pressed(Key::E) {
                if app.editor_state.edit_mode == EditMode::Edit {
                    app.extrude_selected_geometry(0.5);
                } else {
                    app.editor_state.transform_mode = TransformMode::Rotate;
                    app.status_message = "Tool: Rotate (E)".to_string();
                }
            } else if input.key_pressed(Key::R) && app.editor_state.edit_mode == EditMode::Edit {
                app.editor_state.transform_mode = TransformMode::Scale;
                app.status_message = "Tool: Scale (R)".to_string();
            }
        }
        if input.modifiers.shift && input.key_pressed(Key::R) {
            app.editor_state.transform_mode = TransformMode::Scale;
            app.status_message = "Tool: Scale (Shift+R)".to_string();
            ui.ctx().request_repaint();
        }

        // Edit Mode Selection Element Shortcuts (1 = Vertex, 2 = Edge, 3 = Face)
        if app.editor_state.edit_mode == EditMode::Edit && !emulate {
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
                    app.status_message = "Reset Position (Alt+G)".to_string();
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
                    app.status_message = "Reset Rotation (Alt+R)".to_string();
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
                app.status_message = "Reset Scale (Alt+S)".to_string();
                ui.ctx().request_repaint();
            } else if input.key_pressed(Key::H) {
                // Unhide all: Alt + H
                app.unhide_all();
                ui.ctx().request_repaint();
            }
        }

        // Hide Selected: H
        if !input.modifiers.any() && input.key_pressed(Key::H) {
            app.hide_selected();
            ui.ctx().request_repaint();
        }

        // Parent / Unparent: Ctrl + P / Alt + P
        if (input.modifiers.ctrl || input.modifiers.command) && input.key_pressed(Key::P) {
            app.parent_selected_to_active();
            ui.ctx().request_repaint();
        } else if input.modifiers.alt && input.key_pressed(Key::P) {
            app.unparent_selected();
            ui.ctx().request_repaint();
        }

        // Join: Ctrl + J
        if (input.modifiers.ctrl || input.modifiers.command) && input.key_pressed(Key::J) {
            app.join_selected();
            ui.ctx().request_repaint();
        }

        // Duplicate: Shift + D (Duplicates and enters Grab mode)
        if input.modifiers.shift && input.key_pressed(Key::D) {
            app.duplicate_selected();
            let mouse_v = mouse_pos
                .map(|p| Vec2::new(p.x - rect.min.x, p.y - rect.min.y))
                .unwrap_or(Vec2::ZERO);
            app.begin_modal_transform(TransformMode::Translate, mouse_v);
            ui.ctx().request_repaint();
        }

        // Delete: Delete or X
        if input.key_pressed(Key::Delete) || (!input.modifiers.any() && input.key_pressed(Key::X)) {
            app.delete_selected();
            ui.ctx().request_repaint();
        }
    }

    // =========================================================================
    // 4. MOUSE CAMERA NAVIGATION: ORBIT, PAN, ZOOM, DOLLY
    // =========================================================================
    if !app.editor_state.fly_nav_active && !app.editor_state.modal_transform.active {
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
                    app.camera.orbit(
                        delta.x * app.camera.settings.orbit_sensitivity,
                        delta.y * app.camera.settings.orbit_sensitivity,
                    );
                }
                ui.ctx().request_repaint();
            }
        }

        // Mouse Wheel: Zoom
        if is_pointer_in_viewport && input.smooth_scroll_delta.y.abs() > 0.0 {
            let zoom_factor = input.smooth_scroll_delta.y;
            app.camera.zoom(zoom_factor);
            ui.ctx().request_repaint();
        }
    }

    // =========================================================================
    // 5. PAINT 3D VIEWPORT TEXTURE & CAMERA VIEW LETTERBOX
    // =========================================================================
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

    // Camera View Frame / Safe Area Overlay
    if app.editor_state.is_camera_view {
        let active_cam_res = app
            .scene
            .active_camera()
            .map(|(_, d)| (d.resolution[0] as f32, d.resolution[1] as f32))
            .unwrap_or((1920.0, 1080.0));

        let cam_aspect = active_cam_res.0 / active_cam_res.1;
        let view_aspect = rect.width() / rect.height();

        let (frame_w, frame_h) = if view_aspect > cam_aspect {
            let h = rect.height() * 0.90;
            (h * cam_aspect, h)
        } else {
            let w = rect.width() * 0.90;
            (w, w / cam_aspect)
        };

        let frame_rect = Rect::from_center_size(rect.center(), egui::vec2(frame_w, frame_h));

        // Darken areas outside camera frame
        let mask_col = Color32::from_rgba_unmultiplied(10, 12, 16, 180);
        // Top
        painter.rect_filled(
            Rect::from_min_max(rect.min, Pos2::new(rect.max.x, frame_rect.min.y)),
            0.0,
            mask_col,
        );
        // Bottom
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(rect.min.x, frame_rect.max.y), rect.max),
            0.0,
            mask_col,
        );
        // Left
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(rect.min.x, frame_rect.min.y),
                Pos2::new(frame_rect.min.x, frame_rect.max.y),
            ),
            0.0,
            mask_col,
        );
        // Right
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(frame_rect.max.x, frame_rect.min.y),
                Pos2::new(rect.max.x, frame_rect.max.y),
            ),
            0.0,
            mask_col,
        );

        // Frame border
        painter.rect_stroke(
            frame_rect,
            0.0,
            Stroke::new(1.5, Color32::from_rgb(220, 130, 40)),
            egui::StrokeKind::Outside,
        );

        painter.text(
            frame_rect.left_top() + EguiVec2::new(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            format!(
                "Camera View [{}x{}]",
                active_cam_res.0 as u32, active_cam_res.1 as u32
            ),
            egui::FontId::proportional(12.0),
            Color32::from_rgb(240, 160, 60),
        );
    }

    // =========================================================================
    // 6. POINTER INTERACTION & 3D GIZMO
    // =========================================================================
    let lmb_pressed =
        input.pointer.button_pressed(PointerButton::Primary) && is_pointer_in_viewport;
    let lmb_down = input.pointer.button_down(PointerButton::Primary);
    let lmb_released = input.pointer.button_released(PointerButton::Primary);

    // Safety & Focus Loss handling: if primary mouse button is released, reset interaction state
    if !lmb_down && app.editor_state.pointer_state != PointerInteractionState::Idle {
        if let Some(drag) = app.object_drag_state.take()
            && drag.is_active_drag
        {
            app.commit_object_drag(&drag);
        }
        app.editor_state.pointer_state = PointerInteractionState::Idle;
        app.editor_state.pointer_press_pos = None;
        app.editor_state.box_select_start = None;
        app.gizmo_state.drag_state = None;
    }

    // Cancel active free object drag on Escape
    if input.key_pressed(Key::Escape) && app.object_drag_state.is_some() {
        app.cancel_object_drag();
        ui.ctx().request_repaint();
    }

    let ui_captures_pointer = !is_pointer_in_viewport
        || ui.ctx().egui_wants_pointer_input()
        || app.editor_state.pointer_state == PointerInteractionState::UIInteraction;

    // UI Interaction tracking: if pointer was pressed outside viewport (or egui captured it)
    if input.pointer.button_pressed(PointerButton::Primary) && ui_captures_pointer {
        app.editor_state.pointer_state = PointerInteractionState::UIInteraction;
    }

    let view_proj = app.camera.view_proj_matrix();
    let eye = app.camera.eye;
    let screen_to_ray = |sp: Vec2| {
        app.camera
            .screen_to_ray(sp, Vec2::new(rect.width(), rect.height()))
    };

    let mut gizmo_consumed = false;

    if !app.editor_state.fly_nav_active && !app.editor_state.modal_transform.active {
        if app.editor_state.edit_mode == EditMode::Object {
            if !app.selection.is_empty()
                && app.editor_state.transform_mode != TransformMode::BoxSelect
            {
                let pivot_world = match app.editor_state.pivot_point {
                    forge_scene::PivotPoint::MedianPoint => {
                        let mut sum = Vec3::ZERO;
                        let mut count = 0;
                        for &id in &app.selection.selected_ids {
                            if let Some(obj) = app.scene.get_object(id) {
                                sum += obj.local_transform.translation;
                                count += 1;
                            }
                        }
                        if count > 0 {
                            sum / (count as f32)
                        } else {
                            Vec3::ZERO
                        }
                    }
                    forge_scene::PivotPoint::ActiveElement => app
                        .selection
                        .active_id
                        .and_then(|id| app.scene.get_object(id))
                        .map(|obj| obj.local_transform.translation)
                        .unwrap_or(Vec3::ZERO),
                    forge_scene::PivotPoint::Cursor3D => app.editor_state.cursor_3d,
                    forge_scene::PivotPoint::IndividualOrigins => app
                        .selection
                        .active_id
                        .and_then(|id| app.scene.get_object(id))
                        .map(|obj| obj.local_transform.translation)
                        .unwrap_or(Vec3::ZERO),
                };

                let orientation_quat = app
                    .selection
                    .active_id
                    .and_then(|id| app.scene.get_object(id))
                    .map(|obj| obj.local_transform.rotation)
                    .unwrap_or(Quat::IDENTITY);

                let effective_gizmo_mode = match app.editor_state.transform_mode {
                    TransformMode::Translate => forge_scene::GizmoMode::Translate,
                    TransformMode::Rotate => forge_scene::GizmoMode::Rotate,
                    TransformMode::Scale => forge_scene::GizmoMode::Scale,
                    _ => app.editor_state.active_gizmo_mode,
                };

                let gizmo_res = render_and_interact_gizmo(
                    &mut app.gizmo_state,
                    &painter,
                    view_proj,
                    eye,
                    screen_to_ray,
                    rect,
                    pivot_world,
                    effective_gizmo_mode,
                    app.editor_state.transform_orientation,
                    orientation_quat,
                    mouse_pos,
                    lmb_pressed && !input.modifiers.alt && !ui_captures_pointer,
                    lmb_down && !input.modifiers.alt && !ui_captures_pointer,
                    lmb_released,
                    app.editor_state.snap_settings.enabled,
                    app.editor_state.snap_settings.grid_increment,
                );

                if gizmo_res.drag_started {
                    app.gizmo_initial_transforms.clear();
                    for &id in &app.selection.selected_ids {
                        if let Some(obj) = app.scene.get_object(id) {
                            app.gizmo_initial_transforms.insert(id, obj.local_transform);
                        }
                    }
                }

                if gizmo_res.is_dragging {
                    gizmo_consumed = true;
                    app.editor_state.pointer_state = PointerInteractionState::GizmoDragging;

                    if let Some(delta) = gizmo_res.delta {
                        match delta {
                            crate::ui::gizmo::GizmoDelta::Translate(d) => {
                                for (&id, &init_tf) in &app.gizmo_initial_transforms {
                                    if let Some(obj) = app.scene.get_object_mut(id) {
                                        obj.local_transform.translation = init_tf.translation + d;
                                    }
                                }
                            }
                            crate::ui::gizmo::GizmoDelta::Rotate(rot_delta) => {
                                for (&id, &init_tf) in &app.gizmo_initial_transforms {
                                    if let Some(obj) = app.scene.get_object_mut(id) {
                                        obj.local_transform.rotation = rot_delta * init_tf.rotation;
                                        if app.editor_state.pivot_point
                                            != forge_scene::PivotPoint::IndividualOrigins
                                        {
                                            let offset = init_tf.translation - pivot_world;
                                            obj.local_transform.translation =
                                                pivot_world + rot_delta.mul_vec3(offset);
                                        }
                                    }
                                }
                            }
                            crate::ui::gizmo::GizmoDelta::Scale(scale_vec) => {
                                for (&id, &init_tf) in &app.gizmo_initial_transforms {
                                    if let Some(obj) = app.scene.get_object_mut(id) {
                                        obj.local_transform.scale = init_tf.scale * scale_vec;
                                        if app.editor_state.pivot_point
                                            != forge_scene::PivotPoint::IndividualOrigins
                                        {
                                            let offset = init_tf.translation - pivot_world;
                                            obj.local_transform.translation =
                                                pivot_world + offset * scale_vec;
                                        }
                                    }
                                }
                            }
                        }
                        app.scene.dirty = true;
                        ui.ctx().request_repaint();
                    }
                }

                if gizmo_res.drag_ended {
                    let mut changed = Vec::new();
                    for (&id, &init_tf) in &app.gizmo_initial_transforms {
                        if let Some(obj) = app.scene.get_object(id)
                            && obj.local_transform != init_tf
                        {
                            changed.push((id, init_tf, obj.local_transform));
                        }
                    }
                    if changed.len() == 1 {
                        let (id, old_transform, new_transform) = changed[0];
                        app.history.push(Action::Transform {
                            id,
                            old_transform,
                            new_transform,
                        });
                    } else if changed.len() > 1 {
                        app.history.push(Action::MultiTransform {
                            transforms: changed,
                        });
                    }
                    app.gizmo_initial_transforms.clear();
                    app.scene.dirty = true;
                }

                if gizmo_res.consumed {
                    gizmo_consumed = true;
                }
            }
        } else if app.editor_state.edit_mode == EditMode::Edit
            && let Some((_obj_id, edit_mesh)) = &mut app.active_edit_mesh
        {
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

            // Draw vertex dots
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

            let edit_mode_sel = match app.editor_state.selection_element {
                forge_scene::SelectionElement::Vertex => EditSelectionMode::Vertex,
                forge_scene::SelectionElement::Edge => EditSelectionMode::Edge,
                forge_scene::SelectionElement::Face => EditSelectionMode::Face,
            };

            if let Some(pivot) = edit_mesh.selection_center(edit_mode_sel) {
                let gizmo_res = render_and_interact_gizmo(
                    &mut app.gizmo_state,
                    &painter,
                    view_proj,
                    eye,
                    screen_to_ray,
                    rect,
                    pivot,
                    app.editor_state.active_gizmo_mode,
                    app.editor_state.transform_orientation,
                    Quat::IDENTITY,
                    mouse_pos,
                    lmb_pressed && !input.modifiers.alt && !ui_captures_pointer,
                    lmb_down && !input.modifiers.alt && !ui_captures_pointer,
                    lmb_released,
                    app.editor_state.snap_settings.enabled,
                    app.editor_state.snap_settings.grid_increment,
                );
                gizmo_consumed = gizmo_res.consumed;
                if gizmo_consumed {
                    app.editor_state.pointer_state = PointerInteractionState::GizmoDragging;
                }

                if let Some(crate::ui::gizmo::GizmoDelta::Translate(delta)) = gizmo_res.delta {
                    edit_mesh.translate_selection(delta, edit_mode_sel);
                    app.sync_edit_mesh_to_scene();
                    ui.ctx().request_repaint();
                }
            }
        }
    }

    // Update hover target under cursor
    if !ui_captures_pointer
        && is_pointer_in_viewport
        && !lmb_down
        && !app.editor_state.fly_nav_active
        && !app.editor_state.modal_transform.active
        && let Some(pos) = mouse_pos
    {
        let local_pos = pos - rect.min;
        let screen_pos = Vec2::new(local_pos.x, local_pos.y);
        let viewport_size = Vec2::new(rect.width(), rect.height());
        let ray = app.camera.screen_to_ray(screen_pos, viewport_size);
        let hover_hit = app.scene.raycast(&ray).map(|(id, _)| id);
        app.selection.set_hovered(hover_hit);
    } else if ui_captures_pointer {
        app.selection.set_hovered(None);
    }

    // =========================================================================
    // 7. POINTER INTERACTION STATE MACHINE & EMPTY SPACE ORBIT
    // =========================================================================
    if !app.editor_state.fly_nav_active
        && !app.editor_state.modal_transform.active
        && !modal_consumed_click
        && !input.modifiers.alt
    {
        // A. Pointer Press Transition (Determine interaction state)
        if lmb_pressed
            && is_pointer_in_viewport
            && !ui_captures_pointer
            && let Some(pos) = mouse_pos
        {
            if gizmo_consumed {
                app.editor_state.pointer_state = PointerInteractionState::GizmoDragging;
            } else if app.editor_state.transform_mode == TransformMode::BoxSelect {
                app.editor_state.pointer_state = PointerInteractionState::BoxSelecting;
                app.editor_state.box_select_start = Some(Vec2::new(pos.x, pos.y));
            } else {
                let local_pos = pos - rect.min;
                let screen_pos = Vec2::new(local_pos.x, local_pos.y);
                let viewport_size = Vec2::new(rect.width(), rect.height());
                let ray = app.camera.screen_to_ray(screen_pos, viewport_size);

                if app.editor_state.edit_mode == EditMode::Object {
                    if let Some((hit_id, dist)) = app.scene.raycast(&ray) {
                        let hit_point = ray.point_at(dist);
                        let was_already_selected = app.selection.is_selected(hit_id);

                        if !was_already_selected {
                            if input.modifiers.shift {
                                app.selection.toggle_selection(hit_id);
                            } else {
                                app.selection.select(hit_id);
                            }
                        }

                        app.start_object_drag(
                            hit_point,
                            hit_id,
                            Vec2::new(pos.x, pos.y),
                            was_already_selected,
                        );

                        if let Some(obj) = app.scene.get_object(hit_id) {
                            app.status_message = format!("Selected: {}", obj.name);
                        }
                        ui.ctx().request_repaint();
                    } else {
                        // Empty space under cursor -> initiate Viewport Orbiting
                        app.editor_state.pointer_state = PointerInteractionState::ViewportOrbiting;
                        app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                    }
                } else if app.editor_state.edit_mode == EditMode::Edit
                    && let Some((_, edit_mesh)) = &mut app.active_edit_mesh
                {
                    let mut closest_vert = None;
                    let mut min_dist_px = 15.0;

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
                        app.editor_state.pointer_state = PointerInteractionState::ObjectPressed;
                        app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                        if input.modifiers.shift {
                            edit_mesh.vertices[v_idx].selected =
                                !edit_mesh.vertices[v_idx].selected;
                        } else {
                            edit_mesh.deselect_all(EditSelectionMode::Vertex);
                            edit_mesh.vertices[v_idx].selected = true;
                        }
                        app.status_message = format!("Selected Vertex #{}", v_idx);
                        app.sync_edit_mesh_to_scene();
                        ui.ctx().request_repaint();
                    } else {
                        // Empty space in edit mode -> initiate Viewport Orbiting
                        app.editor_state.pointer_state = PointerInteractionState::ViewportOrbiting;
                        app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                    }
                }
            }
        }

        // B. Active Dragging by State
        if lmb_down {
            match app.editor_state.pointer_state {
                PointerInteractionState::ObjectPressed
                | PointerInteractionState::ObjectDragging => {
                    if let Some(pos) = mouse_pos
                        && let Some(drag) = &mut app.object_drag_state
                    {
                        let curr_pos = Vec2::new(pos.x, pos.y);
                        let moved_px = (curr_pos - drag.initial_pointer_pos).length();

                        if !drag.is_active_drag && moved_px >= 3.0 {
                            drag.is_active_drag = true;
                            app.editor_state.pointer_state =
                                PointerInteractionState::ObjectDragging;
                        }

                        if drag.is_active_drag {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            let local_pos = pos - rect.min;
                            let screen_pos = Vec2::new(local_pos.x, local_pos.y);
                            let viewport_size = Vec2::new(rect.width(), rect.height());
                            let curr_ray = app.camera.screen_to_ray(screen_pos, viewport_size);

                            if let Some(delta) = app.update_object_drag(&curr_ray) {
                                ui.ctx().request_repaint();
                                app.status_message = format!(
                                    "Dragging: Δ({:.2}, {:.2}, {:.2})",
                                    delta.x, delta.y, delta.z
                                );
                            }
                        }
                    }
                }
                PointerInteractionState::ViewportOrbiting => {
                    if is_pointer_in_viewport {
                        let delta = input.pointer.delta();
                        if delta.length_sq() > 0.0 {
                            app.camera.orbit(
                                delta.x * app.camera.settings.orbit_sensitivity,
                                delta.y * app.camera.settings.orbit_sensitivity,
                            );
                            ui.ctx().request_repaint();
                        }
                    }
                }
                PointerInteractionState::BoxSelecting => {
                    if let Some(start) = app.editor_state.box_select_start
                        && let Some(curr) = mouse_pos
                    {
                        let start_pos = Pos2::new(start.x, start.y);
                        let box_rect = Rect::from_two_pos(start_pos, curr);
                        if box_rect.width() > 3.0 || box_rect.height() > 3.0 {
                            painter.rect_filled(
                                box_rect,
                                2.0,
                                Color32::from_rgba_unmultiplied(255, 170, 40, 35),
                            );
                            painter.rect_stroke(
                                box_rect,
                                2.0,
                                Stroke::new(1.5, Color32::from_rgb(255, 170, 40)),
                                egui::StrokeKind::Outside,
                            );
                            ui.ctx().request_repaint();
                        }
                    }
                }
                _ => {}
            }
        }

        // C. Pointer Release Handling
        if lmb_released {
            match app.editor_state.pointer_state {
                PointerInteractionState::ObjectDragging => {
                    if let Some(drag) = app.object_drag_state.take()
                        && drag.is_active_drag
                    {
                        app.commit_object_drag(&drag);
                    }
                    app.editor_state.pointer_state = PointerInteractionState::Idle;
                    app.editor_state.pointer_press_pos = None;
                    ui.ctx().request_repaint();
                }
                PointerInteractionState::ObjectPressed => {
                    if let Some(drag) = app.object_drag_state.take()
                        && drag.was_already_selected
                        && !input.modifiers.shift
                        && let Some(pressed_id) = drag.pressed_object_id
                    {
                        app.selection.select(pressed_id);
                        if let Some(obj) = app.scene.get_object(pressed_id) {
                            app.status_message = format!("Selected: {}", obj.name);
                        }
                    }
                    app.editor_state.pointer_state = PointerInteractionState::Idle;
                    app.editor_state.pointer_press_pos = None;
                    ui.ctx().request_repaint();
                }
                PointerInteractionState::ViewportOrbiting => {
                    let moved_dist = if let (Some(press), Some(curr)) =
                        (app.editor_state.pointer_press_pos, mouse_pos)
                    {
                        (Vec2::new(curr.x, curr.y) - press).length()
                    } else {
                        0.0
                    };
                    // Normal static click on empty space clears selection!
                    if moved_dist < 4.0 && !input.modifiers.shift {
                        if app.editor_state.edit_mode == EditMode::Object {
                            app.selection.deselect();
                            app.status_message = "Selection cleared".to_string();
                        } else if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
                            edit_mesh.deselect_all(EditSelectionMode::Vertex);
                            app.sync_edit_mesh_to_scene();
                        }
                        ui.ctx().request_repaint();
                    }
                    app.editor_state.pointer_press_pos = None;
                    app.editor_state.pointer_state = PointerInteractionState::Idle;
                }
                PointerInteractionState::BoxSelecting => {
                    if let Some(start) = app.editor_state.box_select_start.take()
                        && let Some(curr) = mouse_pos
                    {
                        let start_pos = Pos2::new(start.x, start.y);
                        let box_rect = Rect::from_two_pos(start_pos, curr);
                        if box_rect.width() > 5.0 || box_rect.height() > 5.0 {
                            app.box_select(box_rect, view_proj, rect, input.modifiers.shift);
                            ui.ctx().request_repaint();
                        }
                    }
                    app.editor_state.pointer_state = PointerInteractionState::Idle;
                }
                _ => {
                    app.object_drag_state = None;
                    app.editor_state.pointer_state = PointerInteractionState::Idle;
                    app.editor_state.pointer_press_pos = None;
                }
            }
        }
    }

    // =========================================================================
    // 8. VIEWPORT HUD & STATUS OVERLAYS
    // =========================================================================
    // Top-left Mode & Tool badge
    let mode_str = match app.editor_state.edit_mode {
        EditMode::Object => "Object Mode".to_string(),
        EditMode::Edit => format!("Edit Mode ({:?})", app.editor_state.selection_element),
    };
    let tool_str = format!("{:?}", app.editor_state.transform_mode);
    let proj_str = match app.camera.projection {
        CameraProjection::Perspective => "Perspective",
        CameraProjection::Orthographic => "Orthographic",
    };

    let ptr_state_str = if app.editor_state.pointer_state != PointerInteractionState::Idle {
        format!(" | State: {:?}", app.editor_state.pointer_state)
    } else {
        String::new()
    };

    painter.text(
        rect.min + EguiVec2::new(14.0, 14.0),
        egui::Align2::LEFT_TOP,
        format!("{mode_str} | Tool: {tool_str} | {proj_str}{ptr_state_str}"),
        egui::FontId::proportional(14.0),
        Color32::from_gray(210),
    );

    // Active Fly Navigation HUD
    if app.editor_state.fly_nav_active {
        let walk_str = if app.editor_state.walk_mode {
            "Walk"
        } else {
            "Fly"
        };
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + EguiVec2::new(14.0, 36.0),
                egui::vec2(520.0, 26.0),
            ),
            4.0,
            Color32::from_rgba_unmultiplied(200, 60, 40, 220),
        );
        painter.text(
            rect.min + EguiVec2::new(20.0, 40.0),
            egui::Align2::LEFT_TOP,
            format!(
                "[{} NAVIGATION] Speed: {:.1}m/s | WASD: Move | QE: Up/Down | Scroll: Speed | Esc: Exit",
                walk_str.to_uppercase(),
                app.camera.settings.fly_speed,
            ),
            egui::FontId::proportional(13.0),
            Color32::WHITE,
        );
    }

    // Active Modal Transform HUD (G, R, S)
    if app.editor_state.modal_transform.active {
        let mode_name = match app.editor_state.modal_transform.mode {
            TransformMode::Translate => "MOVE",
            TransformMode::Rotate => "ROTATE",
            TransformMode::Scale => "SCALE",
            _ => "TRANSFORM",
        };

        let constraint_str = match app.editor_state.modal_transform.constraint {
            Some(AxisConstraint::X) => {
                if app.editor_state.modal_transform.is_local_constraint {
                    "Local X"
                } else {
                    "Global X"
                }
            }
            Some(AxisConstraint::Y) => {
                if app.editor_state.modal_transform.is_local_constraint {
                    "Local Y"
                } else {
                    "Global Y"
                }
            }
            Some(AxisConstraint::Z) => {
                if app.editor_state.modal_transform.is_local_constraint {
                    "Local Z"
                } else {
                    "Global Z"
                }
            }
            Some(AxisConstraint::PlaneXY) => "Plane XY",
            Some(AxisConstraint::PlaneXZ) => "Plane XZ",
            Some(AxisConstraint::PlaneYZ) => "Plane YZ",
            None => "Free (View Plane)",
        };

        let num_str = if app.editor_state.modal_transform.numeric_input.is_empty() {
            "Mouse".to_string()
        } else {
            app.editor_state.modal_transform.numeric_input.clone()
        };

        painter.rect_filled(
            Rect::from_min_size(
                rect.min + EguiVec2::new(14.0, 36.0),
                egui::vec2(540.0, 26.0),
            ),
            4.0,
            Color32::from_rgba_unmultiplied(220, 120, 20, 230),
        );
        painter.text(
            rect.min + EguiVec2::new(20.0, 40.0),
            egui::Align2::LEFT_TOP,
            format!(
                "[{mode_name}] Axis: {constraint_str} | Value: {num_str} | Enter/LMB: Apply | Esc/RMB: Cancel"
            ),
            egui::FontId::proportional(13.0),
            Color32::WHITE,
        );
    }

    // Coordinate orientation indicator in top-right
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

    // Floating Debug Telemetry Panel
    if app.editor_state.show_debug_info {
        let debug_rect = Rect::from_min_size(
            rect.left_bottom() + EguiVec2::new(14.0, -140.0),
            egui::vec2(320.0, 126.0),
        );
        painter.rect_filled(
            debug_rect,
            4.0,
            Color32::from_rgba_unmultiplied(20, 24, 30, 225),
        );
        painter.rect_stroke(
            debug_rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(80, 95, 120)),
            egui::StrokeKind::Outside,
        );

        let pointer_pos_str = if let Some(pos) = mouse_pos {
            format!("({:.1}, {:.1})", pos.x, pos.y)
        } else {
            "None".to_string()
        };

        let debug_text = format!(
            "POINTER & VIEWPORT TELEMETRY:\n• Pointer State: {:?}\n• Pointer Pos: {}\n• Viewport Rect: ({:.0}, {:.0}) - ({:.0}, {:.0}) [{:.0}x{:.0}]\n• Camera Mode: {}\n• Orbit Pivot: ({:.2}, {:.2}, {:.2})\n• Camera Dist: {:.2}",
            app.editor_state.pointer_state,
            pointer_pos_str,
            rect.min.x,
            rect.min.y,
            rect.max.x,
            rect.max.y,
            rect.width(),
            rect.height(),
            proj_str,
            app.camera.target.x,
            app.camera.target.y,
            app.camera.target.z,
            app.camera.distance(),
        );

        painter.text(
            debug_rect.min + EguiVec2::new(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            debug_text,
            egui::FontId::monospace(11.0),
            Color32::from_rgb(200, 225, 255),
        );
    }
}
