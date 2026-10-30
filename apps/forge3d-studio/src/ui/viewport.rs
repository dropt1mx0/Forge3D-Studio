use egui::{Color32, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2 as EguiVec2};
use forge_math::{Vec2, Vec3};

use crate::app::Forge3dStudioApp;

pub fn render_viewport(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let available_size = ui.available_size();
    if available_size.x < 10.0 || available_size.y < 10.0 {
        return;
    }

    // Allocate exact viewport area for pointer events
    let (rect, response) = ui.allocate_exact_size(available_size, Sense::click_and_drag());

    // Update camera aspect ratio
    app.camera.set_aspect_ratio(rect.width(), rect.height());

    // Process user input for camera navigation and object picking
    let input = ui.input(|i| i.clone());

    // 1. Mouse Drag: Orbit or Pan
    if response.dragged_by(PointerButton::Middle)
        || (response.dragged_by(PointerButton::Primary) && input.modifiers.alt)
    {
        let drag_delta = input.pointer.delta();
        if input.modifiers.shift {
            // Pan
            app.camera.pan(drag_delta.x, drag_delta.y);
        } else {
            // Orbit
            app.camera.orbit(drag_delta.x * 0.008, drag_delta.y * 0.008);
        }
    }

    // 2. Mouse Scroll: Zoom
    if response.hovered() && input.smooth_scroll_delta.y.abs() > 0.0 {
        let zoom_factor = input.smooth_scroll_delta.y * 0.02;
        app.camera.zoom(zoom_factor);
    }

    // 3. Left Mouse Click: Object Selection Raycast
    if response.clicked_by(PointerButton::Primary)
        && !input.modifiers.alt
        && let Some(pos) = input.pointer.interact_pos()
    {
        let local_pos = pos - rect.min;
        let screen_pos = Vec2::new(local_pos.x, local_pos.y);
        let viewport_size = Vec2::new(rect.width(), rect.height());

        let ray = app.camera.screen_to_ray(screen_pos, viewport_size);

        if let Some((hit_id, _dist)) = app.scene.raycast(&ray) {
            app.selection.select(hit_id);
            if let Some(obj) = app.scene.get_object(hit_id) {
                app.status_message = format!("Selected: {}", obj.name);
            }
        } else {
            app.selection.deselect();
            app.status_message = "Selection cleared".to_string();
        }
    }

    // 4. Paint 3D Viewport Texture
    let painter = ui.painter_at(rect);
    if let Some(texture_id) = app.viewport_egui_texture_id {
        painter.image(
            texture_id,
            rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        // Fallback dark placeholder before first frame renders
        painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 32, 36));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Initializing 3D Viewport...",
            egui::FontId::proportional(18.0),
            Color32::from_gray(160),
        );
    }

    // 5. Draw Viewport Overlays
    // Mode badge in top-left
    let mode_str = format!("{:?}", app.editor_state.transform_mode);
    painter.text(
        rect.min + EguiVec2::new(12.0, 12.0),
        egui::Align2::LEFT_TOP,
        format!("Mode: {mode_str} | Perspective"),
        egui::FontId::proportional(13.0),
        Color32::from_gray(200),
    );

    // Orientation Gizmo indicator in top-right
    let gizmo_center = rect.right_top() + EguiVec2::new(-40.0, 40.0);
    let view_mat = app.camera.view_matrix();
    let draw_axis = |axis: Vec3, color: Color32| {
        let transformed = view_mat.transform_vector3(axis);
        let end = gizmo_center + EguiVec2::new(transformed.x * 24.0, -transformed.y * 24.0);
        painter.line_segment([gizmo_center, end], Stroke::new(2.5, color));
    };

    draw_axis(Vec3::X, Color32::from_rgb(230, 60, 60)); // X - Red
    draw_axis(Vec3::Y, Color32::from_rgb(60, 220, 60)); // Y - Green
    draw_axis(Vec3::Z, Color32::from_rgb(60, 120, 240)); // Z - Blue
}
