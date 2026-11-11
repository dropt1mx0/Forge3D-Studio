use std::sync::Arc;

use egui::{Color32, Key, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2 as EguiVec2};
use forge_geometry::EditSelectionMode;
use forge_math::{Quat, Vec2, Vec3};
use forge_render::{CameraProjection, PickResult, ViewPreset, screen_to_texture_coords};
use forge_scene::{
    Action, AxisConstraint, EditMode, LightData, LightKind, PivotPoint, PointerInteractionState,
    PrimitiveType, SceneObject, TransformMode,
};

use crate::app::{Forge3dStudioApp, PieMenuKind, PieMenuState};
use crate::ui::gizmo::{render_and_interact_gizmo, world_to_screen_pos};

pub fn render_viewport(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let available_size = ui.available_size();
    if available_size.x < 10.0 || available_size.y < 10.0 {
        return;
    }

    // Allocate exact viewport area for pointer and keyboard interaction
    let (rect, response) = ui.allocate_exact_size(available_size, Sense::click_and_drag());
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
        // Mode Switch: Tab (fast toggle) or Ctrl+Tab (Mode Pie Menu)
        if input.key_pressed(Key::Tab) {
            if input.modifiers.ctrl {
                app.active_pie_menu = Some(PieMenuState {
                    kind: PieMenuKind::Mode,
                    center: mouse_pos.unwrap_or_else(|| rect.center()),
                });
            } else {
                app.toggle_edit_mode();
            }
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

        // Selection Shortcuts (A = Select All, Alt+A / Escape = Deselect All, Ctrl+I = Invert Selection, B = Box Select)
        if !input.modifiers.command
            && !input.modifiers.ctrl
            && !input.modifiers.alt
            && !input.modifiers.shift
        {
            if input.key_pressed(Key::Escape) {
                if !app.selection.is_empty() {
                    app.deselect_all();
                    ui.ctx().request_repaint();
                } else if let Some((_, edit_mesh)) = &mut app.active_edit_mesh {
                    edit_mesh.deselect_all(EditSelectionMode::Vertex);
                    app.editor_state.active_vertex = None;
                    app.sync_edit_mesh_to_scene();
                    ui.ctx().request_repaint();
                }
            } else if input.key_pressed(Key::A) {
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
            } else if (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(Key::B)
            {
                app.bevel_selected_geometry(0.1);
            } else if (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(Key::R)
            {
                app.loop_cut_selected_geometry(0.5);
            } else if (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(Key::X)
            {
                app.dissolve_selected_geometry();
            } else if !input.modifiers.any() && input.key_pressed(Key::P) {
                app.separate_selected_geometry();
            } else if !input.modifiers.any() && input.key_pressed(Key::K) {
                app.knife_connect_selected_geometry();
            } else if input.modifiers.alt && input.key_pressed(Key::B) {
                app.bisect_selected_geometry(
                    forge_math::Vec3::ZERO,
                    forge_math::Vec3::Y,
                    true,
                    true,
                );
            } else if input.modifiers.alt && input.key_pressed(Key::S) {
                app.spin_selected_geometry(12, std::f32::consts::TAU);
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

        // Command Palette Shortcut: F3, Ctrl+K, or Space (when not dragging)
        if input.key_pressed(Key::F3)
            || ((input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(Key::K))
            || (!input.modifiers.any()
                && input.key_pressed(Key::Space)
                && !app.editor_state.modal_transform.active
                && app.object_drag_state.is_none())
        {
            app.show_command_palette = true;
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

        // Toggle Left Toolbar: T
        if !input.modifiers.any() && input.key_pressed(Key::T) {
            app.editor_state.panel_settings.toolbar_visible =
                !app.editor_state.panel_settings.toolbar_visible;
            let _ = app.editor_state.panel_settings.save();
            app.status_message = format!(
                "Toolbar: {}",
                if app.editor_state.panel_settings.toolbar_visible {
                    "Shown"
                } else {
                    "Hidden"
                }
            );
            ui.ctx().request_repaint();
        }

        // Toggle Right Sidebar (Outliner & Properties): N
        if !input.modifiers.any() && input.key_pressed(Key::N) {
            let new_vis = !app.editor_state.panel_settings.sidebar_visible;
            app.editor_state.panel_settings.sidebar_visible = new_vis;
            let _ = app.editor_state.panel_settings.save();
            app.status_message = format!(
                "Sidebar: {}",
                if new_vis { "Shown" } else { "Hidden" }
            );
            ui.ctx().request_repaint();
        }

        // Quick Add Menu: Shift + A (Blender-compatible)
        if input.modifiers.shift && input.key_pressed(Key::A) {
            app.show_quick_add_menu = mouse_pos.or_else(|| Some(rect.center()));
            ui.ctx().request_repaint();
        }

        // Shading Pie Menu: Z (Blender-compatible)
        if !input.modifiers.any() && input.key_pressed(Key::Z) {
            app.active_pie_menu = Some(PieMenuState {
                kind: PieMenuKind::Shading,
                center: mouse_pos.unwrap_or_else(|| rect.center()),
            });
            ui.ctx().request_repaint();
        }

        // Pivot Point Pie Menu: Period (.)
        if !input.modifiers.any() && input.key_pressed(Key::Period) {
            app.active_pie_menu = Some(PieMenuState {
                kind: PieMenuKind::Pivot,
                center: mouse_pos.unwrap_or_else(|| rect.center()),
            });
            ui.ctx().request_repaint();
        }
    }

    // =========================================================================
    // 4. MOUSE CAMERA NAVIGATION: ORBIT, PAN, ZOOM, DOLLY
    // =========================================================================
    if !app.editor_state.fly_nav_active && !app.editor_state.modal_transform.active && app.active_pie_menu.is_none() {
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
                app.pending_hover_pick = None;
                if let Some(r) = &mut app.renderer {
                    r.invalidate_picking();
                }
                ui.ctx().request_repaint();
            }
        }

        // Mouse Wheel: Adaptive Viewport Zoom (with Zoom to Cursor and Smooth Zoom)
        if is_pointer_in_viewport && input.smooth_scroll_delta.y.abs() > 0.0 {
            let scroll_delta = input.smooth_scroll_delta.y;

            let cursor_world_hit = if app.camera.settings.zoom_to_cursor {
                mouse_pos.and_then(|pos| {
                    if rect.contains(pos) && rect.width() > 0.0 && rect.height() > 0.0 {
                        let local_pos = Vec2::new(pos.x - rect.min.x, pos.y - rect.min.y);
                        let vp_size = Vec2::new(rect.width(), rect.height());
                        let ray = app.camera.screen_to_ray(local_pos, vp_size);

                        // 1. Scene object geometry raycast hit
                        if let Some((_id, dist)) = app.scene.raycast(&ray) {
                            let hit = ray.point_at(dist);
                            if hit.is_finite() {
                                return Some(hit);
                            }
                        }

                        // 2. Safe fallback: focus plane through camera target
                        let plane_point = app.camera.target;
                        let plane_normal = -app.camera.forward();
                        crate::app::intersect_ray_plane(&ray, plane_point, plane_normal)
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            app.camera.zoom_adaptive(scroll_delta, cursor_world_hit);
            app.pending_hover_pick = None;
            if let Some(r) = &mut app.renderer {
                r.invalidate_picking();
            }
            ui.ctx().request_repaint();
        }

        // Smooth Zoom frame interpolation
        let dt = (app.frame_time_ms * 0.001).clamp(0.0005, 0.1);
        if app.camera.update_smooth_zoom(dt) {
            app.pending_hover_pick = None;
            if let Some(r) = &mut app.renderer {
                r.invalidate_picking();
            }
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

    // UI Interaction tracking: if pointer was pressed outside viewport (or obscured by higher UI window / popup menu)
    if input.pointer.button_pressed(PointerButton::Primary)
        && (!is_pointer_in_viewport || !response.hovered())
    {
        app.editor_state.pointer_state = PointerInteractionState::UIInteraction;
    }

    let is_ui_interacting =
        app.editor_state.pointer_state == PointerInteractionState::UIInteraction;

    let view_proj = app.camera.view_proj_matrix();
    let eye = app.camera.eye;
    let screen_to_ray = |sp: Vec2| {
        app.camera
            .screen_to_ray(sp, Vec2::new(rect.width(), rect.height()))
    };

    let is_object_dragging_active = matches!(
        app.editor_state.pointer_state,
        PointerInteractionState::ObjectDragging | PointerInteractionState::ObjectPressed
    );

    let mut gizmo_consumed = false;

    if !app.editor_state.fly_nav_active && !app.editor_state.modal_transform.active {
        let gizmo_mouse = if is_object_dragging_active {
            None
        } else {
            mouse_pos
        };
        let gizmo_lmb_pressed =
            lmb_pressed && !input.modifiers.alt && !is_ui_interacting && !is_object_dragging_active;
        let gizmo_lmb_down =
            lmb_down && !input.modifiers.alt && !is_ui_interacting && !is_object_dragging_active;

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

                let mut base_scale = 1.0_f32;
                if let Some(active_id) = app.selection.active_id {
                    if let Some(obj) = app.scene.get_object(active_id) {
                        let s = obj.local_transform.scale;
                        base_scale = s.x.abs().max(s.y.abs()).max(s.z.abs());
                    }
                }

                let gizmo_res =
                    if app.editor_state.pointer_state == PointerInteractionState::ObjectDragging {
                        // Cleanly suppress gizmo during active free cursor drag to prevent jitter/conflict
                        crate::ui::gizmo::GizmoInteractionResult {
                            consumed: false,
                            is_dragging: false,
                            delta: None,
                            drag_started: false,
                            drag_ended: false,
                        }
                    } else {
                        render_and_interact_gizmo(
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
                            gizmo_mouse,
                            gizmo_lmb_pressed,
                            gizmo_lmb_down,
                            lmb_released,
                            app.editor_state.snap_settings.enabled,
                            app.editor_state.snap_settings.grid_increment,
                            base_scale,
                        )
                    };

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

                if gizmo_res.consumed && !is_object_dragging_active {
                    gizmo_consumed = true;
                }
            }
        } else if app.editor_state.edit_mode == EditMode::Edit
            && let Some((_obj_id, edit_mesh)) = &mut app.active_edit_mesh
        {
            // Draw batched wireframe edges and vertex handles (CPU overlay used for fallback if GPU disabled)
            let draw_cpu_edges = !app.editor_state.use_gpu_wireframe;
            let draw_cpu_vertices = !app.editor_state.use_gpu_vertex_handles;
            render_edit_mode_overlay(
                &mut app.edit_overlay_cache,
                &painter,
                edit_mesh,
                app.editor_state.selection_element,
                view_proj,
                rect,
                app.scene.dirty,
                draw_cpu_edges,
                draw_cpu_vertices,
            );

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
                    gizmo_mouse,
                    gizmo_lmb_pressed,
                    gizmo_lmb_down,
                    lmb_released,
                    app.editor_state.snap_settings.enabled,
                    app.editor_state.snap_settings.grid_increment,
                    1.0,
                );
                gizmo_consumed = gizmo_res.consumed && !is_object_dragging_active;
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

    // Update hover target under cursor (suppressed during active dragging or when pointer is over UI / popups)
    let is_dragging_active = app.editor_state.pointer_state != PointerInteractionState::Idle;
    if !is_ui_interacting
        && is_pointer_in_viewport
        && !lmb_down
        && !is_dragging_active
        && !app.editor_state.fly_nav_active
        && !app.editor_state.modal_transform.active
        && let Some(pos) = mouse_pos
    {
        if app.editor_state.use_gpu_picking
            && let (tex_w, tex_h) = app.viewport_pixel_size
            && tex_w > 0
            && tex_h > 0
            && let Some((px, py)) = screen_to_texture_coords(
                (pos.x, pos.y),
                (rect.min.x, rect.min.y),
                (rect.width(), rect.height()),
                tex_w,
                tex_h,
            )
            && let (Some(renderer), Some(wgpu_state)) =
                (&mut app.renderer, app.wgpu_render_state.clone())
        {
            // 1. Poll previous hover pick request if available
            if let Some(pending) = app.pending_hover_pick
                && let Some(pick_result) = renderer.poll_pick_result(pending.request_id)
            {
                app.pending_hover_pick = None;
                match pick_result {
                    PickResult::Object(id) => app.selection.set_hovered(Some(id)),
                    _ => app.selection.set_hovered(None),
                }
            }

            // 2. Schedule non-blocking async pick request for current cursor position
            let active_mesh_ref = app.active_edit_mesh.as_ref().map(|(id, m)| (id, m));
            if let Some(new_req) = renderer.schedule_pick_request(
                &wgpu_state.device,
                &wgpu_state.queue,
                tex_w,
                tex_h,
                &app.scene,
                &app.camera,
                app.editor_state.edit_mode,
                app.editor_state.selection_element,
                active_mesh_ref,
                px,
                py,
            ) {
                app.pending_hover_pick = Some(crate::app::PendingHoverPick {
                    request_id: new_req,
                    pixel_coords: (px, py),
                    edit_mode: app.editor_state.edit_mode,
                    selection_element: app.editor_state.selection_element,
                    active_object_id: app.active_edit_mesh.as_ref().map(|(id, _)| *id),
                    invalidation_epoch: renderer.picking_ctx.invalidation_epoch,
                });
            }
        } else {
            // CPU fallback raycast
            let local_pos = pos - rect.min;
            let screen_pos = Vec2::new(local_pos.x, local_pos.y);
            let viewport_size = Vec2::new(rect.width(), rect.height());
            let ray = app.camera.screen_to_ray(screen_pos, viewport_size);
            let hover_hit = app.scene.raycast(&ray).map(|(id, _)| id);
            app.selection.set_hovered(hover_hit);
        }
    } else if is_ui_interacting || is_dragging_active {
        app.selection.set_hovered(None);
        app.pending_hover_pick = None;
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
            && !is_ui_interacting
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

                let gpu_pick_result = if app.editor_state.use_gpu_picking
                    && let (tex_w, tex_h) = app.viewport_pixel_size
                    && tex_w > 0
                    && tex_h > 0
                    && let Some((px, py)) = screen_to_texture_coords(
                        (pos.x, pos.y),
                        (rect.min.x, rect.min.y),
                        (rect.width(), rect.height()),
                        tex_w,
                        tex_h,
                    )
                    && let (Some(renderer), Some(wgpu_state)) =
                        (&mut app.renderer, app.wgpu_render_state.clone())
                {
                    // Audit Click Picking Pre-Poll:
                    // Validate pixel coordinates, mode, element, active object, and epoch!
                    let pre_polled_result = if let Some(pending) = app.pending_hover_pick.take() {
                        let same_pixel = pending.pixel_coords == (px, py);
                        let same_mode = pending.edit_mode == app.editor_state.edit_mode;
                        let same_elem =
                            pending.selection_element == app.editor_state.selection_element;
                        let same_obj = pending.active_object_id
                            == app.active_edit_mesh.as_ref().map(|(id, _)| *id);
                        let same_epoch =
                            pending.invalidation_epoch == renderer.picking_ctx.invalidation_epoch;

                        if same_pixel && same_mode && same_elem && same_obj && same_epoch {
                            renderer.poll_pick_result(pending.request_id)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(res) = pre_polled_result {
                        Some(res)
                    } else {
                        let active_mesh_ref = app.active_edit_mesh.as_ref().map(|(id, m)| (id, m));
                        Some(renderer.pick_sync(
                            &wgpu_state.device,
                            &wgpu_state.queue,
                            tex_w,
                            tex_h,
                            &app.scene,
                            &app.camera,
                            app.editor_state.edit_mode,
                            app.editor_state.selection_element,
                            active_mesh_ref,
                            px,
                            py,
                        ))
                    }
                } else {
                    None
                };

                if let Some(pick_res) = gpu_pick_result {
                    match pick_res {
                        PickResult::Object(hit_id) => {
                            let was_already_selected = app.selection.is_selected(hit_id);

                            if !was_already_selected {
                                if input.modifiers.shift {
                                    app.selection.toggle_selection(hit_id);
                                } else {
                                    app.selection.select(hit_id);
                                }
                            }

                            let hit_point = if let Some(obj) = app.scene.get_object(hit_id) {
                                obj.local_transform.translation
                            } else {
                                ray.point_at(5.0)
                            };

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
                        }
                        PickResult::Vertex(v_idx) => {
                            if let Some((_, edit_mesh)) = &mut app.active_edit_mesh
                                && v_idx < edit_mesh.vertices.len()
                            {
                                app.editor_state.pointer_state =
                                    PointerInteractionState::ObjectPressed;
                                app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                                if input.modifiers.shift {
                                    edit_mesh.vertices[v_idx].selected =
                                        !edit_mesh.vertices[v_idx].selected;
                                    if edit_mesh.vertices[v_idx].selected {
                                        app.editor_state.active_vertex = Some(v_idx);
                                    } else if app.editor_state.active_vertex == Some(v_idx) {
                                        app.editor_state.active_vertex = None;
                                    }
                                } else {
                                    edit_mesh.deselect_all(EditSelectionMode::Vertex);
                                    edit_mesh.vertices[v_idx].selected = true;
                                    app.editor_state.active_vertex = Some(v_idx);
                                }
                                app.status_message = format!("Selected Vertex #{}", v_idx);
                                app.sync_edit_mesh_to_scene();
                                ui.ctx().request_repaint();
                            }
                        }
                        PickResult::Edge(e_idx) => {
                            if let Some((_, edit_mesh)) = &mut app.active_edit_mesh
                                && e_idx < edit_mesh.edges.len()
                            {
                                app.editor_state.pointer_state =
                                    PointerInteractionState::ObjectPressed;
                                app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                                if input.modifiers.shift {
                                    edit_mesh.edges[e_idx].selected =
                                        !edit_mesh.edges[e_idx].selected;
                                } else {
                                    edit_mesh.deselect_all(EditSelectionMode::Edge);
                                    edit_mesh.edges[e_idx].selected = true;
                                }
                                app.status_message = format!("Selected Edge #{}", e_idx);
                                app.sync_edit_mesh_to_scene();
                                ui.ctx().request_repaint();
                            }
                        }
                        PickResult::Face(f_idx) => {
                            if let Some((_, edit_mesh)) = &mut app.active_edit_mesh
                                && f_idx < edit_mesh.faces.len()
                            {
                                app.editor_state.pointer_state =
                                    PointerInteractionState::ObjectPressed;
                                app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                                if input.modifiers.shift {
                                    edit_mesh.faces[f_idx].selected =
                                        !edit_mesh.faces[f_idx].selected;
                                } else {
                                    edit_mesh.deselect_all(EditSelectionMode::Face);
                                    edit_mesh.faces[f_idx].selected = true;
                                }
                                app.status_message = format!("Selected Face #{}", f_idx);
                                app.sync_edit_mesh_to_scene();
                                ui.ctx().request_repaint();
                            }
                        }
                        PickResult::None => {
                            app.editor_state.pointer_state =
                                PointerInteractionState::ViewportOrbiting;
                            app.editor_state.pointer_press_pos = Some(Vec2::new(pos.x, pos.y));
                        }
                    }
                } else if app.editor_state.edit_mode == EditMode::Object {
                    // CPU fallback Object Mode
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
                    // CPU fallback Edit Mode
                    let mut closest_vert = None;
                    let mut min_dist_px = 15.0;

                    for (idx, v) in edit_mesh.vertices.iter().enumerate() {
                        let sp_opt = app
                            .edit_overlay_cache
                            .projected_vertices
                            .get(idx)
                            .copied()
                            .flatten()
                            .or_else(|| world_to_screen_pos(v.position, view_proj, rect));

                        if let Some(sp) = sp_opt {
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
                            if edit_mesh.vertices[v_idx].selected {
                                app.editor_state.active_vertex = Some(v_idx);
                            } else if app.editor_state.active_vertex == Some(v_idx) {
                                app.editor_state.active_vertex = None;
                            }
                        } else {
                            edit_mesh.deselect_all(EditSelectionMode::Vertex);
                            edit_mesh.vertices[v_idx].selected = true;
                            app.editor_state.active_vertex = Some(v_idx);
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
                            app.pending_hover_pick = None;
                            if let Some(r) = &mut app.renderer {
                                r.invalidate_picking();
                            }
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

        // Live Sculpting Interaction
        if app.editor_state.edit_mode == EditMode::Sculpt
            && lmb_down
            && is_pointer_in_viewport
            && let Some(pos) = mouse_pos
        {
            let local_pos = pos - rect.min;
            let screen_pos = Vec2::new(local_pos.x, local_pos.y);
            let viewport_size = Vec2::new(rect.width(), rect.height());
            let ray = app.camera.screen_to_ray(screen_pos, viewport_size);

            if let Some((active_id, edit_mesh)) = &mut app.active_edit_mesh {
                let temp_mesh = edit_mesh.to_render_mesh();
                let tf = app.scene.world_transform(*active_id);
                if let Some(hit_dist) = temp_mesh.raycast(&ray, &tf) {
                    let hit_world_pt = ray.origin + ray.direction * hit_dist;
                    let hit_local_pt = tf.inverse().transform_point(hit_world_pt);
                    let hit_local_norm = -tf
                        .inverse()
                        .transform_vector(ray.direction)
                        .normalize_or_zero();
                    let ptr_delta = input.pointer.delta();
                    let stroke_delta = Vec3::new(ptr_delta.x * 0.01, -ptr_delta.y * 0.01, 0.0);
                    let affected = forge_geometry::apply_sculpt_stroke(
                        edit_mesh,
                        hit_local_pt,
                        hit_local_norm,
                        stroke_delta,
                        &app.sculpt_settings,
                    );
                    if affected > 0 {
                        app.sync_edit_mesh_to_scene();
                        app.status_message = format!(
                            "Sculpting ({}): {} vertices affected",
                            app.sculpt_settings.brush.name(),
                            affected
                        );
                        ui.ctx().request_repaint();
                    }
                }
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
                    // Clicking or dragging in empty space preserves the active selection and gizmo vectors
                    app.editor_state.pointer_press_pos = None;
                    app.editor_state.pointer_state = PointerInteractionState::Idle;
                    ui.ctx().request_repaint();
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
        EditMode::Sculpt => "Sculpt Mode".to_string(),
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

    // Sleek Cinema 4D / Octane Viewport Header Strip
    let header_w = (rect.width() - 28.0).clamp(100.0, 520.0);
    let header_rect = Rect::from_min_size(
        rect.min + EguiVec2::new(14.0, 14.0),
        egui::vec2(header_w, 26.0),
    );
    painter.rect_filled(
        header_rect,
        egui::CornerRadius::same(5),
        Color32::from_rgba_unmultiplied(16, 18, 24, 220),
    );
    painter.rect_stroke(
        header_rect,
        egui::CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(38, 42, 54)),
        egui::StrokeKind::Outside,
    );

    let hud_text = format!(
        "Live Viewer  |  View   Cameras   Display   Options   |   {mode_str} | Tool: {tool_str} ({proj_str}){ptr_state_str}"
    );
    painter.text(
        header_rect.min + EguiVec2::new(12.0, 6.0),
        egui::Align2::LEFT_TOP,
        hud_text,
        egui::FontId::proportional(12.0),
        Color32::from_rgb(215, 220, 235),
    );

    // Active Fly Navigation HUD
    if app.editor_state.fly_nav_active {
        let walk_str = if app.editor_state.walk_mode {
            "Walk"
        } else {
            "Fly"
        };
        let fly_w = (rect.width() - 28.0).clamp(100.0, 520.0);
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + EguiVec2::new(14.0, 44.0),
                egui::vec2(fly_w, 26.0),
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

    // Coordinate orientation indicator in bottom-left matching reference screenshot
    let gizmo_center = rect.left_bottom() + EguiVec2::new(45.0, -45.0);
    let view_mat = app.camera.view_matrix();
    let draw_axis = |axis: Vec3, color: Color32, label: &str| {
        let transformed = view_mat.transform_vector3(axis);
        let end = gizmo_center + EguiVec2::new(transformed.x * 24.0, -transformed.y * 24.0);
        painter.line_segment([gizmo_center, end], Stroke::new(2.5, color));
        painter.text(
            end + EguiVec2::new(transformed.x * 6.0, -transformed.y * 6.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.0),
            color,
        );
    };

    draw_axis(Vec3::X, Color32::from_rgb(235, 65, 65), "X"); // X - Red
    draw_axis(Vec3::Y, Color32::from_rgb(65, 225, 75), "Y"); // Y - Green
    draw_axis(Vec3::Z, Color32::from_rgb(65, 130, 245), "Z"); // Z - Blue

    // =========================================================================
    // 3D VIEWPORT NAVIGATION COMPASS & GIZMO (Blender-compatible)
    // =========================================================================
    let compass_center = rect.right_top() + EguiVec2::new(-50.0, 52.0);
    let compass_radius = 38.0;
    let compass_rect = Rect::from_center_size(
        compass_center,
        egui::vec2(compass_radius * 2.0, compass_radius * 2.0),
    );

    let compass_resp = ui.allocate_rect(compass_rect, Sense::click_and_drag());
    let is_light = app.editor_state.panel_settings.is_light_theme;
    let bg_color = if is_light {
        if compass_resp.hovered() {
            Color32::from_rgba_unmultiplied(235, 240, 250, 200)
        } else {
            Color32::from_rgba_unmultiplied(240, 243, 250, 140)
        }
    } else if compass_resp.hovered() {
        Color32::from_rgba_unmultiplied(35, 38, 48, 220)
    } else {
        Color32::from_rgba_unmultiplied(22, 24, 30, 160)
    };
    painter.circle_filled(compass_center, compass_radius, bg_color);
    painter.circle_stroke(
        compass_center,
        compass_radius,
        Stroke::new(
            1.0,
            if is_light {
                Color32::from_rgb(205, 212, 225)
            } else {
                Color32::from_rgb(50, 55, 70)
            },
        ),
    );

    struct AxisTip {
        label: &'static str,
        world_dir: Vec3,
        screen_pos: Pos2,
        depth_z: f32,
        color: Color32,
        preset: ViewPreset,
        is_positive: bool,
    }

    let mut axes = [
        AxisTip {
            label: "X",
            world_dir: Vec3::X,
            screen_pos: Pos2::ZERO,
            depth_z: 0.0,
            color: Color32::from_rgb(235, 65, 65),
            preset: ViewPreset::Right,
            is_positive: true,
        },
        AxisTip {
            label: "-X",
            world_dir: -Vec3::X,
            screen_pos: Pos2::ZERO,
            depth_z: 0.0,
            color: Color32::from_rgb(140, 60, 60),
            preset: ViewPreset::Left,
            is_positive: false,
        },
        AxisTip {
            label: "Y",
            world_dir: Vec3::Y,
            screen_pos: Pos2::ZERO,
            depth_z: 0.0,
            color: Color32::from_rgb(65, 215, 75),
            preset: ViewPreset::Top,
            is_positive: true,
        },
        AxisTip {
            label: "-Y",
            world_dir: -Vec3::Y,
            screen_pos: Pos2::ZERO,
            depth_z: 0.0,
            color: Color32::from_rgb(50, 130, 60),
            preset: ViewPreset::Bottom,
            is_positive: false,
        },
        AxisTip {
            label: "Z",
            world_dir: Vec3::Z,
            screen_pos: Pos2::ZERO,
            depth_z: 0.0,
            color: Color32::from_rgb(65, 130, 245),
            preset: ViewPreset::Front,
            is_positive: true,
        },
        AxisTip {
            label: "-Z",
            world_dir: -Vec3::Z,
            screen_pos: Pos2::ZERO,
            depth_z: 0.0,
            color: Color32::from_rgb(50, 85, 150),
            preset: ViewPreset::Back,
            is_positive: false,
        },
    ];

    let arm_len = 25.0;
    for axis in &mut axes {
        let v = view_mat.transform_vector3(axis.world_dir);
        axis.screen_pos = compass_center + EguiVec2::new(v.x * arm_len, -v.y * arm_len);
        axis.depth_z = v.z;
    }

    axes.sort_by(|a, b| {
        a.depth_z
            .partial_cmp(&b.depth_z)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let pointer_pos = input.pointer.hover_pos();
    let mut clicked_preset = None;

    if compass_resp.dragged() {
        let delta = input.pointer.delta();
        if delta.length_sq() > 0.0 {
            app.camera.orbit(delta.x * 0.008, delta.y * 0.008);
            ui.ctx().request_repaint();
        }
    }

    if compass_resp.clicked() {
        if let Some(pos) = pointer_pos {
            for axis in axes.iter().rev() {
                if pos.distance(axis.screen_pos) <= 10.0 {
                    clicked_preset = Some(axis.preset);
                    break;
                }
            }
        }
    }

    if let Some(preset) = clicked_preset {
        app.camera.set_view_preset(preset);
        app.status_message = format!("View: {:?}", preset);
        ui.ctx().request_repaint();
    }

    for axis in &axes {
        if axis.is_positive {
            painter.line_segment(
                [compass_center, axis.screen_pos],
                Stroke::new(2.0, axis.color.linear_multiply(0.85)),
            );
        }

        let radius = if axis.is_positive { 8.0 } else { 4.5 };
        let is_hovered = pointer_pos
            .map(|p| p.distance(axis.screen_pos) <= radius + 2.0)
            .unwrap_or(false);
        let bubble_color = if is_hovered {
            Color32::WHITE
        } else {
            axis.color
        };

        painter.circle_filled(axis.screen_pos, radius, bubble_color);
        if axis.is_positive {
            let text_color = if is_hovered {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            painter.text(
                axis.screen_pos,
                egui::Align2::CENTER_CENTER,
                axis.label,
                egui::FontId::proportional(10.0),
                text_color,
            );
        }
    }

    // Quick Navigation Buttons (Zoom, Pan, Camera, Ortho/Persp)
    let btn_start_y = compass_center.y + compass_radius + 10.0;
    let btn_size = egui::vec2(24.0, 24.0);
    let nav_btn_x = compass_center.x - btn_size.x / 2.0;

    let nav_buttons = [
        ("🔍", "Zoom View (Drag up/down)"),
        ("✋", "Pan View (Drag)"),
        ("🎥", "Toggle Camera View (Numpad 0)"),
        (
            match app.camera.projection {
                CameraProjection::Perspective => "◬",
                CameraProjection::Orthographic => "⊞",
            },
            "Toggle Perspective / Orthographic (Numpad 5)",
        ),
    ];

    for (idx, (icon, tooltip)) in nav_buttons.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            egui::pos2(nav_btn_x, btn_start_y + idx as f32 * 27.0),
            btn_size,
        );
        let resp = ui.allocate_rect(btn_rect, Sense::click_and_drag());
        let is_hover = resp.hovered();
        let bg = if is_hover {
            if is_light {
                Color32::from_rgb(220, 226, 240)
            } else {
                Color32::from_rgb(45, 50, 65)
            }
        } else if is_light {
            Color32::from_rgb(240, 243, 250)
        } else {
            Color32::from_rgb(26, 28, 36)
        };

        painter.rect(
            btn_rect,
            egui::CornerRadius::same(5),
            bg,
            Stroke::new(
                1.0,
                if is_light {
                    Color32::from_rgb(205, 212, 225)
                } else {
                    Color32::from_rgb(45, 50, 62)
                },
            ),
            egui::StrokeKind::Inside,
        );
        painter.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            *icon,
            egui::FontId::proportional(11.5),
            if is_hover {
                if is_light {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                }
            } else if is_light {
                Color32::from_rgb(60, 68, 85)
            } else {
                Color32::from_rgb(180, 185, 200)
            },
        );

        let resp = resp.on_hover_text(*tooltip);

        if idx == 0 && resp.dragged() {
            let dy = input.pointer.delta().y;
            app.camera.zoom(1.0 - dy * 0.02);
            ui.ctx().request_repaint();
        } else if idx == 1 && resp.dragged() {
            let delta = input.pointer.delta();
            app.camera.pan(delta.x, delta.y);
            ui.ctx().request_repaint();
        } else if idx == 2 && resp.clicked() {
            app.toggle_camera_view();
            ui.ctx().request_repaint();
        } else if idx == 3 && resp.clicked() {
            app.camera.projection = match app.camera.projection {
                CameraProjection::Perspective => CameraProjection::Orthographic,
                CameraProjection::Orthographic => CameraProjection::Perspective,
            };
            app.status_message = format!("Projection: {:?}", app.camera.projection);
            ui.ctx().request_repaint();
        }
    }

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

    // =========================================================================
    // QUICK ADD POPUP (Shift+A)
    // =========================================================================
    if let Some(pos) = app.show_quick_add_menu {
        let is_light = app.editor_state.panel_settings.is_light_theme;
        let mut close_menu = false;
        let mut item_to_add: Option<&'static str> = None;

        let clamped_pos = egui::pos2(
            pos.x.clamp(rect.min.x + 10.0, (rect.max.x - 200.0).max(rect.min.x + 10.0)),
            pos.y.clamp(rect.min.y + 10.0, (rect.max.y - 360.0).max(rect.min.y + 10.0)),
        );

        egui::Window::new("quick_add_popup")
            .fixed_pos(clamped_pos)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::menu(ui.style())
                    .fill(if is_light {
                        egui::Color32::from_rgb(250, 252, 255)
                    } else {
                        egui::Color32::from_rgb(22, 24, 30)
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if is_light {
                            egui::Color32::from_rgb(210, 215, 228)
                        } else {
                            egui::Color32::from_rgb(45, 48, 62)
                        },
                    ))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(8.0),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(170.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Add").strong().size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Shift+A")
                                .size(10.5)
                                .color(egui::Color32::from_gray(140)),
                        );
                    });
                });
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                ui.label(egui::RichText::new("MESH").weak().size(10.0));
                for name in [
                    "Cube",
                    "UV Sphere",
                    "Cylinder",
                    "Cone",
                    "Torus",
                    "Capsule",
                    "Plane",
                    "Monkey (Suzanne)",
                ] {
                    if ui.button(name).clicked() {
                        item_to_add = Some(name);
                        close_menu = true;
                    }
                }

                ui.add_space(3.0);
                ui.separator();
                ui.add_space(2.0);

                ui.label(egui::RichText::new("LIGHT & CAMERA").weak().size(10.0));
                if ui.button("Point Light").clicked() {
                    item_to_add = Some("Point Light");
                    close_menu = true;
                }
                if ui.button("Camera").clicked() {
                    item_to_add = Some("Camera");
                    close_menu = true;
                }
            });

        if ui.input(|i| i.key_pressed(Key::Escape)) {
            close_menu = true;
        }

        if close_menu {
            app.show_quick_add_menu = None;
        }

        if let Some(item) = item_to_add {
            let cursor = app.editor_state.cursor_3d;
            match item {
                "Cube" => {
                    let mesh = forge_geometry::primitives::generate_cube(1.0);
                    let id = app.scene.add_object(
                        SceneObject::new("Cube", PrimitiveType::Cube, mesh)
                            .with_color([0.85, 0.45, 0.25, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.5, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Cube".to_string();
                }
                "UV Sphere" => {
                    let mesh = forge_geometry::primitives::generate_uv_sphere(1.0, 32, 16);
                    let id = app.scene.add_object(
                        SceneObject::new("Sphere", PrimitiveType::Sphere, mesh)
                            .with_color([0.25, 0.65, 0.85, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 1.0, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added UV Sphere".to_string();
                }
                "Cylinder" => {
                    let mesh = forge_geometry::primitives::generate_cylinder(1.0, 2.0, 32);
                    let id = app.scene.add_object(
                        SceneObject::new("Cylinder", PrimitiveType::Cylinder, mesh)
                            .with_color([0.45, 0.75, 0.35, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 1.0, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Cylinder".to_string();
                }
                "Cone" => {
                    let mesh = forge_geometry::primitives::generate_cone(0.8, 1.5, 24);
                    let id = app.scene.add_object(
                        SceneObject::new("Cone", PrimitiveType::Cone, mesh)
                            .with_color([0.9, 0.6, 0.2, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.75, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Cone".to_string();
                }
                "Torus" => {
                    let mesh = forge_geometry::primitives::generate_torus(1.0, 0.3, 24, 16);
                    let id = app.scene.add_object(
                        SceneObject::new("Torus", PrimitiveType::Torus, mesh)
                            .with_color([0.75, 0.35, 0.85, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 0.3, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Torus".to_string();
                }
                "Capsule" => {
                    let mesh = forge_geometry::primitives::generate_capsule(0.5, 1.0, 24, 8);
                    let id = app.scene.add_object(
                        SceneObject::new("Capsule", PrimitiveType::Capsule, mesh)
                            .with_color([0.85, 0.65, 0.25, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 1.0, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Capsule".to_string();
                }
                "Plane" => {
                    let mesh = forge_geometry::primitives::generate_plane(8.0, 8);
                    let id = app.scene.add_object(
                        SceneObject::new("Plane", PrimitiveType::Plane, mesh)
                            .with_color([0.55, 0.65, 0.75, 1.0])
                            .with_transform(forge_math::Transform::from_translation(cursor)),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Plane".to_string();
                }
                "Monkey (Suzanne)" => {
                    let mesh = forge_geometry::primitives::generate_monkey(1.0);
                    let id = app.scene.add_object(
                        SceneObject::new("Suzanne", PrimitiveType::Monkey, mesh)
                            .with_color([0.65, 0.45, 0.25, 1.0])
                            .with_transform(forge_math::Transform::from_translation(
                                cursor + Vec3::new(0.0, 1.0, 0.0),
                            )),
                    );
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Suzanne".to_string();
                }
                "Point Light" => {
                    let mut light = SceneObject::new_light(
                        "Point Light",
                        LightData {
                            kind: LightKind::Point,
                            color: [1.0, 0.92, 0.72],
                            intensity: 30.0,
                            ..LightData::default()
                        },
                        forge_geometry::primitives::generate_uv_sphere(0.25, 12, 8),
                    );
                    light.local_transform.translation = cursor + Vec3::new(0.0, 3.0, 0.0);
                    let id = app.scene.add_object(light);
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Point Light".to_string();
                }
                "Camera" => {
                    let mut camera_obj = SceneObject::new_camera("Camera");
                    camera_obj.local_transform.translation = cursor + Vec3::new(0.0, 1.5, 3.0);
                    let id = app.scene.add_object(camera_obj);
                    if let Some(obj) = app.scene.get_object(id) {
                        app.history.push(Action::AddObject { object: obj.clone() });
                    }
                    app.selection.select(id);
                    app.status_message = "Added Camera".to_string();
                }
                _ => {}
            }
            if let Some(r) = &mut app.renderer {
                r.invalidate_all_meshes();
            }
            app.scene.dirty = true;
            ui.ctx().request_repaint();
        }
    }

    // =========================================================================
    // PIE MENU OVERLAY (Ctrl+Tab, Z, Period)
    // =========================================================================
    if let Some(pie_state) = app.active_pie_menu {
        let keep_open = render_pie_menu_overlay(ui, app, rect, pie_state);
        if !keep_open {
            app.active_pie_menu = None;
            ui.ctx().request_repaint();
        }
    }

    if let Some(elapsed) = app.showcase_demo_elapsed {
        render_showcase_demo_overlay(&painter, rect, elapsed);
    }
}

enum PieMenuAction {
    SetEditMode(EditMode),
    SetShadingWireframe(bool),
    OpenMaterialEditor,
    SetPivotPoint(PivotPoint),
}

struct PieSlice {
    angle: f32,
    icon: &'static str,
    label: &'static str,
    subtext: &'static str,
    action: PieMenuAction,
    is_active: bool,
}

fn execute_pie_action(app: &mut Forge3dStudioApp, action: &PieMenuAction) {
    match action {
        PieMenuAction::SetEditMode(mode) => {
            app.set_edit_mode(*mode);
        }
        PieMenuAction::SetShadingWireframe(wireframe) => {
            app.editor_state.show_wireframe = *wireframe;
            app.status_message = format!("Wireframe: {}", wireframe);
        }
        PieMenuAction::OpenMaterialEditor => {
            app.show_material_editor = true;
            app.status_message = "Material Editor Opened".to_string();
        }
        PieMenuAction::SetPivotPoint(pivot) => {
            app.editor_state.pivot_point = *pivot;
            app.status_message = format!("Pivot Point: {:?}", pivot);
        }
    }
}

fn render_pie_menu_overlay(
    ui: &mut Ui,
    app: &mut Forge3dStudioApp,
    viewport: Rect,
    pie_state: PieMenuState,
) -> bool {
    let painter = ui.painter_at(viewport);
    let center = egui::pos2(
        pie_state.center.x.clamp(viewport.min.x + 150.0, viewport.max.x - 150.0),
        pie_state.center.y.clamp(viewport.min.y + 110.0, viewport.max.y - 110.0),
    );
    let mouse_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos())).unwrap_or(center);
    let delta = mouse_pos - center;
    let dist = delta.length();
    let deadzone_radius = 26.0;
    let in_deadzone = dist < deadzone_radius;
    let mouse_angle = delta.y.atan2(delta.x);
    let is_light = app.editor_state.panel_settings.is_light_theme;

    let (title, slices) = match pie_state.kind {
        PieMenuKind::Mode => (
            "INTERACTION MODE",
            vec![
                PieSlice {
                    angle: -std::f32::consts::FRAC_PI_2,
                    icon: "✏",
                    label: "Edit Mode",
                    subtext: "Tab",
                    action: PieMenuAction::SetEditMode(EditMode::Edit),
                    is_active: app.editor_state.edit_mode == EditMode::Edit,
                },
                PieSlice {
                    angle: std::f32::consts::FRAC_PI_2,
                    icon: "🧊",
                    label: "Object Mode",
                    subtext: "4",
                    action: PieMenuAction::SetEditMode(EditMode::Object),
                    is_active: app.editor_state.edit_mode == EditMode::Object,
                },
                PieSlice {
                    angle: std::f32::consts::PI,
                    icon: "🏺",
                    label: "Sculpt Mode",
                    subtext: "6",
                    action: PieMenuAction::SetEditMode(EditMode::Sculpt),
                    is_active: app.editor_state.edit_mode == EditMode::Sculpt,
                },
            ],
        ),
        PieMenuKind::Shading => (
            "VIEWPORT SHADING",
            vec![
                PieSlice {
                    angle: -std::f32::consts::FRAC_PI_2,
                    icon: "⚪",
                    label: "Solid",
                    subtext: "8",
                    action: PieMenuAction::SetShadingWireframe(false),
                    is_active: !app.editor_state.show_wireframe,
                },
                PieSlice {
                    angle: std::f32::consts::FRAC_PI_2,
                    icon: "🕸",
                    label: "Wireframe",
                    subtext: "4",
                    action: PieMenuAction::SetShadingWireframe(true),
                    is_active: app.editor_state.show_wireframe,
                },
                PieSlice {
                    angle: 0.0,
                    icon: "🎨",
                    label: "Material Preview",
                    subtext: "6",
                    action: PieMenuAction::OpenMaterialEditor,
                    is_active: app.show_material_editor,
                },
                PieSlice {
                    angle: std::f32::consts::PI,
                    icon: "✨",
                    label: "Rendered",
                    subtext: "2",
                    action: PieMenuAction::SetShadingWireframe(false),
                    is_active: false,
                },
            ],
        ),
        PieMenuKind::Pivot => (
            "PIVOT POINT",
            vec![
                PieSlice {
                    angle: -std::f32::consts::FRAC_PI_2,
                    icon: "⌖",
                    label: "Median Point",
                    subtext: "8",
                    action: PieMenuAction::SetPivotPoint(PivotPoint::MedianPoint),
                    is_active: app.editor_state.pivot_point == PivotPoint::MedianPoint,
                },
                PieSlice {
                    angle: 0.0,
                    icon: "🔘",
                    label: "Active Element",
                    subtext: "6",
                    action: PieMenuAction::SetPivotPoint(PivotPoint::ActiveElement),
                    is_active: app.editor_state.pivot_point == PivotPoint::ActiveElement,
                },
                PieSlice {
                    angle: std::f32::consts::FRAC_PI_2,
                    icon: "🎯",
                    label: "3D Cursor",
                    subtext: "2",
                    action: PieMenuAction::SetPivotPoint(PivotPoint::Cursor3D),
                    is_active: app.editor_state.pivot_point == PivotPoint::Cursor3D,
                },
                PieSlice {
                    angle: std::f32::consts::PI,
                    icon: "⚬",
                    label: "Individual Origins",
                    subtext: "4",
                    action: PieMenuAction::SetPivotPoint(PivotPoint::IndividualOrigins),
                    is_active: app.editor_state.pivot_point == PivotPoint::IndividualOrigins,
                },
            ],
        ),
    };

    let radius = 95.0;
    let mut hovered_idx = None;

    if !in_deadzone {
        let mut best_diff = f32::MAX;
        let mut best_idx = None;
        for (idx, slice) in slices.iter().enumerate() {
            let mut diff = (mouse_angle - slice.angle).abs();
            if diff > std::f32::consts::PI {
                diff = std::f32::consts::TAU - diff;
            }
            if diff < best_diff {
                best_diff = diff;
                best_idx = Some(idx);
            }
        }
        hovered_idx = best_idx;
    }

    for (idx, slice) in slices.iter().enumerate() {
        let card_pos = center + egui::vec2(slice.angle.cos(), slice.angle.sin()) * radius;
        let card_rect = Rect::from_center_size(card_pos, egui::vec2(134.0, 32.0));
        if card_rect.contains(mouse_pos) {
            hovered_idx = Some(idx);
            break;
        }
    }

    for slice in &slices {
        let trigger = match slice.subtext {
            "1" => ui.input(|i| i.key_pressed(Key::Num1)),
            "2" => ui.input(|i| i.key_pressed(Key::Num2)),
            "3" => ui.input(|i| i.key_pressed(Key::Num3)),
            "4" => ui.input(|i| i.key_pressed(Key::Num4)),
            "6" => ui.input(|i| i.key_pressed(Key::Num6)),
            "8" => ui.input(|i| i.key_pressed(Key::Num8)),
            "9" => ui.input(|i| i.key_pressed(Key::Num9)),
            "Tab" => ui.input(|i| i.key_pressed(Key::Tab)),
            _ => false,
        };
        if trigger {
            execute_pie_action(app, &slice.action);
            return false;
        }
    }

    let clicked = ui.input(|i| i.pointer.button_clicked(PointerButton::Primary));
    if clicked {
        if let Some(idx) = hovered_idx {
            execute_pie_action(app, &slices[idx].action);
            return false;
        } else if in_deadzone || dist > radius + 100.0 {
            return false;
        }
    }

    if ui.input(|i| i.key_pressed(Key::Escape) || i.pointer.button_clicked(PointerButton::Secondary)) {
        return false;
    }

    // 1. Subtle radial outer guide circle
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.0,
            if is_light {
                Color32::from_black_alpha(20)
            } else {
                Color32::from_white_alpha(15)
            },
        ),
    );

    // 2. Center deadzone disc
    painter.circle_filled(
        center,
        deadzone_radius,
        if is_light {
            Color32::from_rgba_unmultiplied(235, 238, 245, 230)
        } else {
            Color32::from_rgba_unmultiplied(22, 24, 30, 235)
        },
    );
    painter.circle_stroke(
        center,
        deadzone_radius,
        Stroke::new(
            1.5,
            if is_light {
                Color32::from_rgb(180, 190, 210)
            } else {
                Color32::from_rgb(50, 58, 72)
            },
        ),
    );
    painter.circle_filled(center, 3.5, Color32::from_rgb(45, 125, 245));

    // Title centered above the deadzone
    painter.text(
        center + egui::vec2(0.0, -10.0),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(8.0),
        if is_light {
            Color32::from_rgb(80, 95, 120)
        } else {
            Color32::from_rgb(130, 145, 175)
        },
    );

    // 3. Direction line from deadzone to cursor
    if !in_deadzone {
        let dir = delta / dist;
        let line_end = center + dir * dist.min(radius);
        painter.line_segment(
            [center + dir * deadzone_radius, line_end],
            Stroke::new(2.5, Color32::from_rgb(50, 135, 250)),
        );
    }

    // 4. Render slices
    for (idx, slice) in slices.iter().enumerate() {
        let card_pos = center + egui::vec2(slice.angle.cos(), slice.angle.sin()) * radius;
        let card_rect = Rect::from_center_size(card_pos, egui::vec2(134.0, 32.0));
        let is_hovered = hovered_idx == Some(idx);

        let (fill, stroke, text_color, sub_color) = if is_hovered {
            (
                Color32::from_rgb(32, 105, 225),
                Stroke::new(1.8, Color32::from_rgb(130, 195, 255)),
                Color32::WHITE,
                Color32::from_rgb(205, 230, 255),
            )
        } else if slice.is_active {
            if is_light {
                (
                    Color32::from_rgb(220, 235, 255),
                    Stroke::new(1.5, Color32::from_rgb(60, 130, 220)),
                    Color32::from_rgb(15, 35, 80),
                    Color32::from_rgb(70, 105, 160),
                )
            } else {
                (
                    Color32::from_rgba_unmultiplied(35, 52, 78, 235),
                    Stroke::new(1.5, Color32::from_rgb(75, 145, 235)),
                    Color32::from_rgb(150, 205, 255),
                    Color32::from_rgb(120, 170, 235),
                )
            }
        } else if is_light {
            (
                Color32::from_rgba_unmultiplied(248, 250, 254, 230),
                Stroke::new(1.0, Color32::from_black_alpha(25)),
                Color32::from_rgb(35, 40, 50),
                Color32::from_black_alpha(110),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(24, 27, 34, 230),
                Stroke::new(1.0, Color32::from_white_alpha(32)),
                Color32::from_rgb(220, 225, 235),
                Color32::from_white_alpha(100),
            )
        };

        if is_hovered {
            painter.rect_filled(
                card_rect.expand(2.0),
                egui::CornerRadius::same(8),
                Color32::from_rgba_unmultiplied(32, 105, 225, 45),
            );
        }

        painter.rect_filled(card_rect, egui::CornerRadius::same(6), fill);
        painter.rect_stroke(card_rect, egui::CornerRadius::same(6), stroke, egui::StrokeKind::Inside);

        painter.text(
            card_rect.left_center() + egui::vec2(10.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", slice.icon, slice.label),
            egui::FontId::proportional(12.0),
            text_color,
        );

        painter.text(
            card_rect.right_center() + egui::vec2(-10.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            slice.subtext,
            egui::FontId::proportional(10.0),
            sub_color,
        );
    }

    true
}

fn render_showcase_demo_overlay(painter: &egui::Painter, viewport: Rect, elapsed: f32) {
    let stage_duration = 1.35;
    let stage = ((elapsed / stage_duration).floor() as usize).min(5);
    let progress = (elapsed / stage_duration).fract();
    let stage_names = [
        "Yard and path",
        "House foundation and walls",
        "Roof and chimney",
        "Porch and facade",
        "Tree and foliage",
        "Rock and garden details",
    ];
    let active_keys: &[&str] = match stage {
        0 => &["LMB"],
        1 => &["G", "X", "ENTER"],
        2 => &["R", "Z", "ENTER"],
        3 => &["S", "Y", "ENTER"],
        4 => &["G", "Y", "ENTER"],
        _ => &["LMB", "CTRL"],
    };

    let panel_width = 284.0_f32.min(viewport.width() - 24.0);
    let panel_height = 184.0_f32.min(viewport.height() - 24.0);
    if panel_width < 200.0 || panel_height < 150.0 {
        return;
    }
    let panel = Rect::from_min_size(
        Pos2::new(
            viewport.right() - panel_width - 12.0,
            viewport.bottom() - panel_height - 12.0,
        ),
        egui::vec2(panel_width, panel_height),
    );
    painter.rect_filled(panel, 8.0, Color32::from_rgba_unmultiplied(13, 16, 22, 235));
    painter.rect_stroke(
        panel,
        8.0,
        Stroke::new(1.0, Color32::from_rgb(67, 83, 107)),
        egui::StrokeKind::Outside,
    );

    let text_left = panel.left() + 11.0;
    painter.text(
        Pos2::new(text_left, panel.top() + 8.0),
        egui::Align2::LEFT_TOP,
        "BUILD DEMO  /  SIMULATED INPUT",
        egui::FontId::monospace(10.0),
        Color32::from_rgb(118, 183, 255),
    );
    painter.text(
        Pos2::new(text_left, panel.top() + 24.0),
        egui::Align2::LEFT_TOP,
        format!("Step {} of 6  ·  {}", stage + 1, stage_names[stage]),
        egui::FontId::proportional(11.5),
        Color32::from_rgb(232, 236, 245),
    );

    let key_height = 17.0;
    let key_gap = 3.0;
    let draw_key = |label: &str, x: f32, y: f32, width: f32| {
        let active = active_keys.contains(&label);
        let key_rect = Rect::from_min_size(Pos2::new(x, y), egui::vec2(width, key_height));
        painter.rect_filled(
            key_rect,
            3.0,
            if active {
                Color32::from_rgb(33, 101, 190)
            } else {
                Color32::from_rgb(37, 42, 52)
            },
        );
        painter.rect_stroke(
            key_rect,
            3.0,
            Stroke::new(
                0.8,
                if active {
                    Color32::from_rgb(127, 190, 255)
                } else {
                    Color32::from_rgb(73, 81, 96)
                },
            ),
            egui::StrokeKind::Outside,
        );
        painter.text(
            key_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::monospace(if label.len() > 3 { 7.5 } else { 9.0 }),
            if active {
                Color32::WHITE
            } else {
                Color32::from_rgb(188, 196, 210)
            },
        );
    };

    let row_y = panel.top() + 52.0;
    let first_row = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"];
    let second_row = ["A", "S", "D", "F", "G", "H", "J", "K", "L"];
    let third_row = ["Z", "X", "C", "V", "B", "N", "M"];
    let key_width = ((panel.width() - 24.0 - key_gap * 9.0) / 10.0).min(23.0);
    let draw_letter_row = |keys: &[&str], y: f32| {
        let row_width = keys.len() as f32 * key_width + (keys.len() - 1) as f32 * key_gap;
        let mut x = panel.center().x - row_width * 0.5;
        for key in keys {
            draw_key(key, x, y, key_width);
            x += key_width + key_gap;
        }
    };
    draw_letter_row(&first_row, row_y);
    draw_letter_row(&second_row, row_y + 20.0);
    draw_letter_row(&third_row, row_y + 40.0);

    let control_keys = [
        ("SHIFT", 34.0),
        ("CTRL", 31.0),
        ("SPACE", 50.0),
        ("ENTER", 40.0),
        ("LMB", 30.0),
    ];
    let control_width = control_keys.iter().map(|(_, width)| width).sum::<f32>()
        + key_gap * (control_keys.len() - 1) as f32;
    let mut control_x = panel.center().x - control_width * 0.5;
    for (label, width) in control_keys {
        draw_key(label, control_x, row_y + 60.0, width);
        control_x += width + key_gap;
    }

    painter.text(
        Pos2::new(text_left, panel.bottom() - 30.0),
        egui::Align2::LEFT_TOP,
        "Screen illustration only · sends no OS input",
        egui::FontId::proportional(9.0),
        Color32::from_rgb(166, 176, 192),
    );
    let progress_rect = Rect::from_min_size(
        Pos2::new(text_left, panel.bottom() - 13.0),
        egui::vec2(panel.width() - 22.0, 4.0),
    );
    painter.rect_filled(progress_rect, 2.0, Color32::from_rgb(43, 49, 61));
    painter.rect_filled(
        Rect::from_min_size(
            progress_rect.min,
            egui::vec2(progress_rect.width() * progress, progress_rect.height()),
        ),
        2.0,
        Color32::from_rgb(64, 146, 245),
    );

    // This animated screen-space pointer illustrates the current focus of the walkthrough.
    let cursor_targets = [
        (0.48, 0.79),
        (0.49, 0.55),
        (0.51, 0.24),
        (0.51, 0.64),
        (0.73, 0.44),
        (0.29, 0.72),
    ];
    let from = cursor_targets[stage];
    let to = cursor_targets[(stage + 1).min(cursor_targets.len() - 1)];
    let t = progress * progress * (3.0 - 2.0 * progress);
    let cursor_pos = Pos2::new(
        viewport.left() + viewport.width() * (from.0 + (to.0 - from.0) * t),
        viewport.top() + viewport.height() * (from.1 + (to.1 - from.1) * t),
    );
    let pulse = 7.0 + (elapsed * 5.0).sin().abs() * 4.0;
    painter.circle_stroke(
        cursor_pos,
        pulse,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(106, 186, 255, 210)),
    );
    let cursor_points = vec![
        cursor_pos,
        cursor_pos + EguiVec2::new(2.0, 21.0),
        cursor_pos + EguiVec2::new(7.0, 15.0),
        cursor_pos + EguiVec2::new(13.0, 24.0),
        cursor_pos + EguiVec2::new(17.0, 22.0),
        cursor_pos + EguiVec2::new(12.0, 13.0),
        cursor_pos + EguiVec2::new(21.0, 12.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        cursor_points,
        Color32::WHITE,
        Stroke::new(1.5, Color32::from_rgb(20, 26, 35)),
    ));
}

/// Pre-allocated cache for Edit Mode viewport overlay rendering.
/// Reuses projection buffers, culls offscreen elements, and batches wireframe/vertices into pre-tessellated `egui::Mesh`.
#[derive(Default, Clone)]
pub struct EditOverlayCache {
    pub projected_vertices: Vec<Option<Pos2>>,
    pub mesh: egui::Mesh,
    pub cached_shape: Option<Arc<egui::Mesh>>,
    pub last_view_proj: forge_math::Mat4,
    pub last_rect_min: Pos2,
    pub last_rect_max: Pos2,
    pub last_selection_element: forge_scene::SelectionElement,
    pub last_selected_verts: usize,
    pub last_selected_edges: usize,
    pub last_vert_count: usize,
    pub last_edge_count: usize,
    pub last_draw_edges: bool,
    pub last_draw_vertices: bool,
    pub build_count: usize,
}

impl EditOverlayCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets cached shape to force rebuild on next frame (e.g. after camera movement or geometry edits).
    pub fn reset(&mut self) {
        self.cached_shape = None;
        self.last_view_proj = forge_math::Mat4::ZERO;
        self.last_rect_min = Pos2::ZERO;
        self.last_rect_max = Pos2::ZERO;
    }

    /// Clears and shrinks all internal buffers back to zero capacity to free RAM when exiting Edit Mode.
    pub fn clear(&mut self) {
        self.reset();
        self.projected_vertices.clear();
        self.projected_vertices.shrink_to_fit();
        self.mesh.vertices.clear();
        self.mesh.vertices.shrink_to_fit();
        self.mesh.indices.clear();
        self.mesh.indices.shrink_to_fit();
    }
}

fn add_line_quad(mesh: &mut egui::Mesh, p0: Pos2, p1: Pos2, half_width: f32, color: Color32) {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-4 {
        return;
    }
    let inv_len = half_width / len_sq.sqrt();
    let nx = -dy * inv_len;
    let ny = dx * inv_len;

    let v0 = Pos2::new(p0.x + nx, p0.y + ny);
    let v1 = Pos2::new(p1.x + nx, p1.y + ny);
    let v2 = Pos2::new(p1.x - nx, p1.y - ny);
    let v3 = Pos2::new(p0.x - nx, p0.y - ny);

    let base_idx = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v0,
        uv: egui::epaint::WHITE_UV,
        color,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v1,
        uv: egui::epaint::WHITE_UV,
        color,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v2,
        uv: egui::epaint::WHITE_UV,
        color,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v3,
        uv: egui::epaint::WHITE_UV,
        color,
    });

    mesh.indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);
}

fn add_box_quad(mesh: &mut egui::Mesh, center: Pos2, half_size: f32, color: Color32) {
    let min_x = center.x - half_size;
    let max_x = center.x + half_size;
    let min_y = center.y - half_size;
    let max_y = center.y + half_size;

    let v0 = Pos2::new(min_x, min_y);
    let v1 = Pos2::new(max_x, min_y);
    let v2 = Pos2::new(max_x, max_y);
    let v3 = Pos2::new(min_x, max_y);

    let base_idx = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v0,
        uv: egui::epaint::WHITE_UV,
        color,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v1,
        uv: egui::epaint::WHITE_UV,
        color,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v2,
        uv: egui::epaint::WHITE_UV,
        color,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: v3,
        uv: egui::epaint::WHITE_UV,
        color,
    });

    mesh.indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);
}

#[allow(clippy::too_many_arguments)]
pub fn render_edit_mode_overlay(
    cache: &mut EditOverlayCache,
    painter: &egui::Painter,
    edit_mesh: &forge_geometry::EditableMesh,
    selection_element: forge_scene::SelectionElement,
    view_proj: forge_math::Mat4,
    rect: Rect,
    scene_dirty: bool,
    draw_edges: bool,
    draw_vertices: bool,
) {
    let selected_verts = edit_mesh.vertices.iter().filter(|v| v.selected).count();
    let selected_edges = edit_mesh.edges.iter().filter(|e| e.selected).count();

    let cache_valid = cache.cached_shape.is_some()
        && !scene_dirty
        && cache.last_view_proj == view_proj
        && cache.last_rect_min == rect.min
        && cache.last_rect_max == rect.max
        && cache.last_selection_element == selection_element
        && cache.last_vert_count == edit_mesh.vertices.len()
        && cache.last_edge_count == edit_mesh.edges.len()
        && cache.last_selected_verts == selected_verts
        && cache.last_selected_edges == selected_edges
        && cache.last_draw_edges == draw_edges
        && cache.last_draw_vertices == draw_vertices;

    if cache_valid && let Some(shape) = &cache.cached_shape {
        painter.add(egui::Shape::Mesh(Arc::clone(shape)));
        return;
    }

    // 1. Precompute vertex screen projections once into scratch buffer
    cache.projected_vertices.clear();
    cache.projected_vertices.reserve(edit_mesh.vertices.len());
    for v in &edit_mesh.vertices {
        cache
            .projected_vertices
            .push(world_to_screen_pos(v.position, view_proj, rect));
    }

    // 2. Prepare pre-tessellated egui::Mesh buffer (reusing existing capacity)
    cache.mesh.vertices.clear();
    cache.mesh.indices.clear();

    let cull_rect_edges = rect.expand(20.0);
    let cull_rect_verts = rect.expand(10.0);

    let est_edges = if draw_edges { edit_mesh.edges.len() } else { 0 };
    let est_verts = if draw_vertices {
        edit_mesh.vertices.len()
    } else {
        0
    };
    cache.mesh.vertices.reserve(est_edges * 4 + est_verts * 4);
    cache.mesh.indices.reserve(est_edges * 6 + est_verts * 6);

    // 3. Batch wireframe edges (only if CPU edges are requested)
    if draw_edges {
        for edge in &edit_mesh.edges {
            let (Some(s0), Some(s1)) = (
                cache.projected_vertices[edge.v0],
                cache.projected_vertices[edge.v1],
            ) else {
                continue;
            };

            // Frustum culling: skip if both endpoints are outside the viewport bounds
            if !cull_rect_edges.contains(s0) && !cull_rect_edges.contains(s1) {
                continue;
            }

            let col = if edge.selected {
                Color32::from_rgb(255, 170, 40)
            } else {
                Color32::from_rgba_unmultiplied(160, 170, 190, 120)
            };
            let half_width = if edge.selected { 1.25 } else { 0.6 };
            add_line_quad(&mut cache.mesh, s0, s1, half_width, col);
        }
    }

    // 4. Batch vertex handles (only if CPU vertices are requested)
    if draw_vertices {
        let is_vertex_mode = selection_element == forge_scene::SelectionElement::Vertex;
        for (idx, vertex) in edit_mesh.vertices.iter().enumerate() {
            let Some(sp) = cache.projected_vertices[idx] else {
                continue;
            };
            if !cull_rect_verts.contains(sp) {
                continue;
            }

            if vertex.selected {
                // Highlighted selected vertex handle
                add_box_quad(&mut cache.mesh, sp, 4.0, Color32::from_rgb(255, 180, 40));
                add_box_quad(&mut cache.mesh, sp, 2.0, Color32::from_rgb(255, 230, 100));
            } else if is_vertex_mode {
                // Unselected vertex handle (only in Vertex selection mode)
                add_box_quad(&mut cache.mesh, sp, 2.0, Color32::from_rgb(45, 50, 60));
            }
        }
    }

    // 5. Store in Arc and commit single Shape::Mesh to painter
    let arc_mesh = Arc::new(cache.mesh.clone());
    cache.cached_shape = Some(Arc::clone(&arc_mesh));
    cache.last_view_proj = view_proj;
    cache.last_rect_min = rect.min;
    cache.last_rect_max = rect.max;
    cache.last_selection_element = selection_element;
    cache.last_vert_count = edit_mesh.vertices.len();
    cache.last_edge_count = edit_mesh.edges.len();
    cache.last_selected_verts = selected_verts;
    cache.last_selected_edges = selected_edges;
    cache.last_draw_edges = draw_edges;
    cache.last_draw_vertices = draw_vertices;
    cache.build_count += 1;

    painter.add(egui::Shape::Mesh(arc_mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::EditableMesh;
    use forge_geometry::primitives::{generate_cube, generate_plane, generate_uv_sphere};
    use forge_math::{Mat4, Vec3};

    fn make_test_camera_matrix(eye: Vec3, target: Vec3) -> Mat4 {
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);
        let proj = Mat4::perspective_rh(50.0_f32.to_radians(), 1.0, 0.1, 100.0);
        proj * view
    }

    #[test]
    fn test_edit_mode_small_mesh_overlay_cache() {
        let cube = generate_cube(1.0);
        let edit_mesh = EditableMesh::from_mesh(&cube);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(800.0, 600.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        // Frame 1: Initial build
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(cache.build_count, 1);
        assert!(cache.cached_shape.is_some());
        let initial_verts = cache.cached_shape.as_ref().unwrap().vertices.len();
        assert!(initial_verts > 0);

        // Frame 2: Static camera & mesh -> must reuse cached shape without rebuild
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(
            cache.build_count, 1,
            "Static frame must reuse cache without incrementing build_count"
        );
    }

    #[test]
    fn test_edit_mode_10k_vertices_overlay_batching() {
        // Plane with 100x100 resolution = 10,201 vertices, ~20,000 triangles, ~30,000 edges
        let plane = generate_plane(10.0, 100);
        let edit_mesh = EditableMesh::from_mesh(&plane);
        assert!(edit_mesh.vertices.len() >= 10_000);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 15.0, 15.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(1024.0, 768.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        let t0 = std::time::Instant::now();
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        let elapsed = t0.elapsed();

        assert_eq!(cache.build_count, 1);
        let max_allowed_ms = if cfg!(debug_assertions) { 250 } else { 50 };
        assert!(
            elapsed.as_millis() < max_allowed_ms,
            "10k vertices batched overlay build should take < {max_allowed_ms}ms (took {}ms)",
            elapsed.as_millis()
        );

        // Second frame: zero work
        let t1 = std::time::Instant::now();
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        let cached_elapsed = t1.elapsed();
        assert_eq!(cache.build_count, 1);
        let max_cached_ms = if cfg!(debug_assertions) { 25 } else { 5 };
        assert!(
            cached_elapsed.as_millis() < max_cached_ms,
            "Cached frame must take < {max_cached_ms}ms (took {}µs)",
            cached_elapsed.as_micros()
        );
    }

    #[test]
    fn test_edit_mode_50k_vertices_overlay_batching() {
        // High density UV Sphere: 250 x 200 = ~50,000 vertices
        let sphere = generate_uv_sphere(2.0, 250, 200);
        let edit_mesh = EditableMesh::from_mesh(&sphere);
        assert!(edit_mesh.vertices.len() > 40_000);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(1920.0, 1080.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(cache.build_count, 1);

        // Reusing capacity during camera orbit (view_proj changed)
        let new_view_proj = make_test_camera_matrix(Vec3::new(2.0, 1.0, 6.0), Vec3::ZERO);
        let vert_cap_before = cache.mesh.vertices.capacity();
        let ind_cap_before = cache.mesh.indices.capacity();

        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            new_view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(cache.build_count, 2);
        // Buffers should reuse existing capacity without growing repeatedly
        assert_eq!(cache.mesh.vertices.capacity(), vert_cap_before);
        assert_eq!(cache.mesh.indices.capacity(), ind_cap_before);
    }

    #[test]
    fn test_edit_mode_100k_vertices_overlay_batching() {
        // High density UV Sphere: 360 x 280 = ~100,000 vertices
        let sphere = generate_uv_sphere(2.0, 360, 280);
        let edit_mesh = EditableMesh::from_mesh(&sphere);
        assert!(edit_mesh.vertices.len() > 90_000);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(1920.0, 1080.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(cache.build_count, 1);
        assert!(cache.cached_shape.is_some());
    }

    #[test]
    fn test_edit_mode_geometry_modification_invalidation() {
        let cube = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(800.0, 600.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        // Frame 1
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(cache.build_count, 1);

        // Modify vertex position and flag scene dirty
        edit_mesh.vertices[0].position.x += 10.0;
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            true, // scene_dirty = true
            true,
            true,
        );
        assert_eq!(
            cache.build_count, 2,
            "Geometry modification must invalidate cache and rebuild"
        );
    }

    #[test]
    fn test_edit_mode_selection_change_invalidation() {
        let cube = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(800.0, 600.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        // Frame 1
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(cache.build_count, 1);

        // Select a vertex
        edit_mesh.vertices[0].selected = true;
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert_eq!(
            cache.build_count, 2,
            "Selection count change must invalidate cache and rebuild"
        );
    }

    #[test]
    fn test_edit_mode_switch_memory_cleared() {
        let sphere = generate_uv_sphere(1.0, 64, 32);
        let edit_mesh = EditableMesh::from_mesh(&sphere);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(800.0, 600.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        // Render in edit mode
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            true,
            true,
        );
        assert!(cache.projected_vertices.capacity() > 0);
        assert!(cache.mesh.vertices.capacity() > 0);
        assert!(cache.cached_shape.is_some());

        // Exit edit mode -> clear cache
        cache.clear();
        assert_eq!(cache.projected_vertices.capacity(), 0);
        assert_eq!(cache.mesh.vertices.capacity(), 0);
        assert_eq!(cache.mesh.indices.capacity(), 0);
        assert!(cache.cached_shape.is_none());
    }

    #[test]
    fn test_edit_mode_gpu_vertex_toggle_skips_cpu_vertex_generation() {
        let cube = generate_cube(1.0);
        let edit_mesh = EditableMesh::from_mesh(&cube);

        let mut cache = EditOverlayCache::new();
        let view_proj = make_test_camera_matrix(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(800.0, 600.0));

        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), rect);

        // When both GPU wireframe and GPU vertex handles are active:
        // draw_edges = false, draw_vertices = false -> CPU overlay generates 0 vertices!
        render_edit_mode_overlay(
            &mut cache,
            &painter,
            &edit_mesh,
            forge_scene::SelectionElement::Vertex,
            view_proj,
            rect,
            false,
            false,
            false,
        );

        assert_eq!(cache.build_count, 1);
        let shape = cache.cached_shape.as_ref().unwrap();
        assert_eq!(
            shape.vertices.len(),
            0,
            "When GPU handles and edges are active, CPU overlay mesh should have 0 vertices"
        );
        assert_eq!(shape.indices.len(), 0);
    }
}
