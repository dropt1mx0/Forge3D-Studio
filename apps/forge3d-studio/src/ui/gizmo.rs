use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2 as EguiVec2};
use forge_math::{Mat4, Quat, Ray, Vec2, Vec3};
use forge_scene::{GizmoMode, TransformOrientation};

/// Identifies an interactive component of the 3D transform gizmo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoPart {
    AxisX,
    AxisY,
    AxisZ,
    PlaneXY,
    PlaneXZ,
    PlaneYZ,
    RotateX,
    RotateY,
    RotateZ,
    ScaleX,
    ScaleY,
    ScaleZ,
    ScaleUniform,
}

/// Calculated transformation delta during an active gizmo drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoDelta {
    Translate(Vec3),
    Rotate(Quat),
    Scale(Vec3),
}

/// Active drag state for the gizmo.
#[derive(Debug, Clone)]
pub struct GizmoDragState {
    pub active_part: GizmoPart,
    pub start_mouse: Vec2,
    pub start_pivot: Vec3,
    pub initial_hit_coord: f32,
    pub initial_hit_point: Vec3,
}

/// State tracking hovered and dragging gizmo parts.
#[derive(Debug, Clone, Default)]
pub struct GizmoState {
    pub hovered_part: Option<GizmoPart>,
    pub drag_state: Option<GizmoDragState>,
}

impl GizmoState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_state.is_some()
    }
}

/// Interaction result returned after rendering and hit-testing the gizmo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoInteractionResult {
    /// True if the gizmo was hovered, clicked, or actively dragged, consuming pointer input.
    pub consumed: bool,
    /// True if the user is currently dragging a gizmo part.
    pub is_dragging: bool,
    /// Active transformation delta relative to drag start.
    pub delta: Option<GizmoDelta>,
    /// True on the first frame when drag begins.
    pub drag_started: bool,
    /// True on the frame when drag is released.
    pub drag_ended: bool,
}

/// Helper function to project a 3D world position to 2D screen pixels within `rect`.
pub fn world_to_screen_pos(world_pos: Vec3, view_proj: Mat4, rect: Rect) -> Option<Pos2> {
    let clip = view_proj * glam::Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
    if clip.w <= 0.001 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if ndc.z < 0.0 || ndc.z > 1.0 {
        return None;
    }
    let screen_x = rect.min.x + (ndc.x + 1.0) * 0.5 * rect.width();
    let screen_y = rect.min.y + (1.0 - ndc.y) * 0.5 * rect.height();
    Some(Pos2::new(screen_x, screen_y))
}

/// Renders the transform gizmo and handles interactive drag manipulation.
#[allow(clippy::too_many_arguments)]
pub fn render_and_interact_gizmo(
    gizmo_state: &mut GizmoState,
    painter: &Painter,
    camera_view_proj: Mat4,
    camera_eye: Vec3,
    camera_screen_to_ray: impl Fn(Vec2) -> Ray,
    viewport_rect: Rect,
    pivot_world: Vec3,
    mode: GizmoMode,
    orientation: TransformOrientation,
    orientation_quat: Quat,
    mouse_pos: Option<Pos2>,
    lmb_pressed: bool,
    lmb_down: bool,
    lmb_released: bool,
    snap_enabled: bool,
    grid_increment: f32,
    base_scale: f32,
) -> GizmoInteractionResult {
    let Some(center_screen) = world_to_screen_pos(pivot_world, camera_view_proj, viewport_rect)
    else {
        gizmo_state.hovered_part = None;
        return GizmoInteractionResult {
            consumed: false,
            is_dragging: false,
            delta: None,
            drag_started: false,
            drag_ended: false,
        };
    };

    // Dynamic 3D world-space length that scales in perspective together with the object and camera zoom
    let base_world_len = (1.15 * base_scale.clamp(0.2, 50.0)).clamp(0.4, 50.0);

    let up_hint = if (camera_eye - pivot_world).normalize_or_zero().dot(Vec3::Y).abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let test_pos = pivot_world + up_hint * base_world_len;
    let gizmo_3d_len = if let Some(s_test) = world_to_screen_pos(test_pos, camera_view_proj, viewport_rect) {
        let current_screen_len = (s_test - center_screen).length();
        if current_screen_len < 26.0 {
            // Far distance fallback: maintain at least 26px on screen so user can still easily hit-test and click it
            base_world_len * (26.0 / current_screen_len.max(0.01))
        } else if current_screen_len > 340.0 {
            // Extreme close-up clamp: cap at 340px so handles stay within screen bounds
            base_world_len * (340.0 / current_screen_len)
        } else {
            // 1:1 true 3D world-space scaling with camera zoom
            base_world_len
        }
    } else {
        base_world_len
    };

    // Orientation basis vectors
    let (axis_x, axis_y, axis_z) = match orientation {
        TransformOrientation::Global => (Vec3::X, Vec3::Y, Vec3::Z),
        TransformOrientation::Local => (
            orientation_quat.mul_vec3(Vec3::X),
            orientation_quat.mul_vec3(Vec3::Y),
            orientation_quat.mul_vec3(Vec3::Z),
        ),
    };

    let p_x = pivot_world + axis_x * gizmo_3d_len;
    let p_y = pivot_world + axis_y * gizmo_3d_len;
    let p_z = pivot_world + axis_z * gizmo_3d_len;

    let screen_x = world_to_screen_pos(p_x, camera_view_proj, viewport_rect);
    let screen_y = world_to_screen_pos(p_y, camera_view_proj, viewport_rect);
    let screen_z = world_to_screen_pos(p_z, camera_view_proj, viewport_rect);

    // 1. Hit testing against gizmo parts
    let mut hovered = None;
    let hit_dist_threshold = 12.0;

    if let Some(mouse) = mouse_pos {
        match mode {
            GizmoMode::Translate => {
                // Planar handles
                let p_xy = pivot_world + (axis_x + axis_y) * (gizmo_3d_len * 0.35);
                let p_xz = pivot_world + (axis_x + axis_z) * (gizmo_3d_len * 0.35);
                let p_yz = pivot_world + (axis_y + axis_z) * (gizmo_3d_len * 0.35);

                if let Some(s_xz) = world_to_screen_pos(p_xz, camera_view_proj, viewport_rect)
                    && mouse.distance(s_xz) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::PlaneXZ);
                } else if let Some(s_xy) =
                    world_to_screen_pos(p_xy, camera_view_proj, viewport_rect)
                    && mouse.distance(s_xy) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::PlaneXY);
                } else if let Some(s_yz) =
                    world_to_screen_pos(p_yz, camera_view_proj, viewport_rect)
                    && mouse.distance(s_yz) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::PlaneYZ);
                } else if let Some(sx) = screen_x
                    && dist_to_segment_2d(mouse, center_screen, sx) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::AxisX);
                } else if let Some(sy) = screen_y
                    && dist_to_segment_2d(mouse, center_screen, sy) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::AxisY);
                } else if let Some(sz) = screen_z
                    && dist_to_segment_2d(mouse, center_screen, sz) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::AxisZ);
                }
            }
            GizmoMode::Scale => {
                // Center uniform scale box
                if mouse.distance(center_screen) < 14.0 {
                    hovered = Some(GizmoPart::ScaleUniform);
                } else if let Some(sx) = screen_x
                    && dist_to_segment_2d(mouse, center_screen, sx) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::ScaleX);
                } else if let Some(sy) = screen_y
                    && dist_to_segment_2d(mouse, center_screen, sy) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::ScaleY);
                } else if let Some(sz) = screen_z
                    && dist_to_segment_2d(mouse, center_screen, sz) < hit_dist_threshold
                {
                    hovered = Some(GizmoPart::ScaleZ);
                }
            }
            GizmoMode::Rotate => {
                // Accurate 3D ring hit-testing along circle circumferences
                let d_x = dist_to_projected_ring(
                    mouse,
                    camera_view_proj,
                    viewport_rect,
                    pivot_world,
                    axis_y,
                    axis_z,
                    gizmo_3d_len,
                );
                let d_y = dist_to_projected_ring(
                    mouse,
                    camera_view_proj,
                    viewport_rect,
                    pivot_world,
                    axis_z,
                    axis_x,
                    gizmo_3d_len,
                );
                let d_z = dist_to_projected_ring(
                    mouse,
                    camera_view_proj,
                    viewport_rect,
                    pivot_world,
                    axis_x,
                    axis_y,
                    gizmo_3d_len,
                );

                let min_d = d_x.min(d_y).min(d_z);
                if min_d < hit_dist_threshold * 1.25 {
                    if (min_d - d_x).abs() < 1e-3 {
                        hovered = Some(GizmoPart::RotateX);
                    } else if (min_d - d_y).abs() < 1e-3 {
                        hovered = Some(GizmoPart::RotateY);
                    } else {
                        hovered = Some(GizmoPart::RotateZ);
                    }
                }
            }
        }
    }

    if !gizmo_state.is_dragging() {
        gizmo_state.hovered_part = hovered;
    }

    // 2. Drag Start
    let mut drag_started = false;
    if lmb_pressed
        && let Some(part) = gizmo_state.hovered_part
        && let Some(mouse) = mouse_pos
    {
        let mouse_vec = Vec2::new(mouse.x - viewport_rect.min.x, mouse.y - viewport_rect.min.y);
        let ray = camera_screen_to_ray(mouse_vec);

        let initial_hit_coord = match part {
            GizmoPart::AxisX | GizmoPart::ScaleX => ray.closest_point_on_line(pivot_world, axis_x),
            GizmoPart::AxisY | GizmoPart::ScaleY => ray.closest_point_on_line(pivot_world, axis_y),
            GizmoPart::AxisZ | GizmoPart::ScaleZ => ray.closest_point_on_line(pivot_world, axis_z),
            _ => 0.0,
        };

        let initial_hit_point = match part {
            GizmoPart::PlaneXZ => ray
                .intersect_plane(pivot_world, axis_y)
                .map(|t| ray.point_at(t))
                .unwrap_or(pivot_world),
            GizmoPart::PlaneXY => ray
                .intersect_plane(pivot_world, axis_z)
                .map(|t| ray.point_at(t))
                .unwrap_or(pivot_world),
            GizmoPart::PlaneYZ => ray
                .intersect_plane(pivot_world, axis_x)
                .map(|t| ray.point_at(t))
                .unwrap_or(pivot_world),
            GizmoPart::RotateX => ray
                .intersect_plane(pivot_world, axis_x)
                .map(|t| ray.point_at(t))
                .unwrap_or(pivot_world),
            GizmoPart::RotateY => ray
                .intersect_plane(pivot_world, axis_y)
                .map(|t| ray.point_at(t))
                .unwrap_or(pivot_world),
            GizmoPart::RotateZ => ray
                .intersect_plane(pivot_world, axis_z)
                .map(|t| ray.point_at(t))
                .unwrap_or(pivot_world),
            _ => pivot_world,
        };

        gizmo_state.drag_state = Some(GizmoDragState {
            active_part: part,
            start_mouse: Vec2::new(mouse.x, mouse.y),
            start_pivot: pivot_world,
            initial_hit_coord,
            initial_hit_point,
        });
        drag_started = true;
    }

    // 3. Dragging Delta Calculation
    let mut delta = None;
    if let Some(drag) = &gizmo_state.drag_state
        && lmb_down
        && let Some(mouse) = mouse_pos
    {
        let mouse_vec = Vec2::new(mouse.x - viewport_rect.min.x, mouse.y - viewport_rect.min.y);
        let ray = camera_screen_to_ray(mouse_vec);

        match drag.active_part {
            GizmoPart::AxisX => {
                let current_coord = ray.closest_point_on_line(drag.start_pivot, axis_x);
                let mut d = current_coord - drag.initial_hit_coord;
                if snap_enabled && grid_increment > 0.0 {
                    d = (d / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Translate(axis_x * d));
            }
            GizmoPart::AxisY => {
                let current_coord = ray.closest_point_on_line(drag.start_pivot, axis_y);
                let mut d = current_coord - drag.initial_hit_coord;
                if snap_enabled && grid_increment > 0.0 {
                    d = (d / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Translate(axis_y * d));
            }
            GizmoPart::AxisZ => {
                let current_coord = ray.closest_point_on_line(drag.start_pivot, axis_z);
                let mut d = current_coord - drag.initial_hit_coord;
                if snap_enabled && grid_increment > 0.0 {
                    d = (d / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Translate(axis_z * d));
            }
            GizmoPart::PlaneXZ => {
                if let Some(t) = ray.intersect_plane(drag.start_pivot, axis_y) {
                    let mut d = ray.point_at(t) - drag.initial_hit_point;
                    if snap_enabled && grid_increment > 0.0 {
                        d.x = (d.x / grid_increment).round() * grid_increment;
                        d.z = (d.z / grid_increment).round() * grid_increment;
                    }
                    delta = Some(GizmoDelta::Translate(d));
                }
            }
            GizmoPart::PlaneXY => {
                if let Some(t) = ray.intersect_plane(drag.start_pivot, axis_z) {
                    let mut d = ray.point_at(t) - drag.initial_hit_point;
                    if snap_enabled && grid_increment > 0.0 {
                        d.x = (d.x / grid_increment).round() * grid_increment;
                        d.y = (d.y / grid_increment).round() * grid_increment;
                    }
                    delta = Some(GizmoDelta::Translate(d));
                }
            }
            GizmoPart::PlaneYZ => {
                if let Some(t) = ray.intersect_plane(drag.start_pivot, axis_x) {
                    let mut d = ray.point_at(t) - drag.initial_hit_point;
                    if snap_enabled && grid_increment > 0.0 {
                        d.y = (d.y / grid_increment).round() * grid_increment;
                        d.z = (d.z / grid_increment).round() * grid_increment;
                    }
                    delta = Some(GizmoDelta::Translate(d));
                }
            }
            GizmoPart::RotateX => {
                if let Some(t) = ray.intersect_plane(drag.start_pivot, axis_x) {
                    let v0 = (drag.initial_hit_point - drag.start_pivot).normalize_or_zero();
                    let v1 = (ray.point_at(t) - drag.start_pivot).normalize_or_zero();
                    if v0.length_squared() > 1e-4 && v1.length_squared() > 1e-4 {
                        let mut angle = v0.cross(v1).dot(axis_x).atan2(v0.dot(v1));
                        if snap_enabled {
                            let step = 15.0_f32.to_radians();
                            angle = (angle / step).round() * step;
                        }
                        delta = Some(GizmoDelta::Rotate(Quat::from_axis_angle(axis_x, angle)));
                    }
                }
            }
            GizmoPart::RotateY => {
                if let Some(t) = ray.intersect_plane(drag.start_pivot, axis_y) {
                    let v0 = (drag.initial_hit_point - drag.start_pivot).normalize_or_zero();
                    let v1 = (ray.point_at(t) - drag.start_pivot).normalize_or_zero();
                    if v0.length_squared() > 1e-4 && v1.length_squared() > 1e-4 {
                        let mut angle = v0.cross(v1).dot(axis_y).atan2(v0.dot(v1));
                        if snap_enabled {
                            let step = 15.0_f32.to_radians();
                            angle = (angle / step).round() * step;
                        }
                        delta = Some(GizmoDelta::Rotate(Quat::from_axis_angle(axis_y, angle)));
                    }
                }
            }
            GizmoPart::RotateZ => {
                if let Some(t) = ray.intersect_plane(drag.start_pivot, axis_z) {
                    let v0 = (drag.initial_hit_point - drag.start_pivot).normalize_or_zero();
                    let v1 = (ray.point_at(t) - drag.start_pivot).normalize_or_zero();
                    if v0.length_squared() > 1e-4 && v1.length_squared() > 1e-4 {
                        let mut angle = v0.cross(v1).dot(axis_z).atan2(v0.dot(v1));
                        if snap_enabled {
                            let step = 15.0_f32.to_radians();
                            angle = (angle / step).round() * step;
                        }
                        delta = Some(GizmoDelta::Rotate(Quat::from_axis_angle(axis_z, angle)));
                    }
                }
            }
            GizmoPart::ScaleUniform => {
                let start_d = (drag.start_mouse - Vec2::new(center_screen.x, center_screen.y))
                    .length()
                    .max(10.0);
                let curr_d = (Vec2::new(mouse.x, mouse.y)
                    - Vec2::new(center_screen.x, center_screen.y))
                .length();
                let mut factor = (curr_d / start_d).clamp(0.01, 100.0);
                if snap_enabled && grid_increment > 0.0 {
                    factor = (factor / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Scale(Vec3::splat(factor)));
            }
            GizmoPart::ScaleX => {
                let current_coord = ray.closest_point_on_line(drag.start_pivot, axis_x);
                let initial_coord = drag.initial_hit_coord.abs().max(0.1);
                let mut factor = (current_coord / initial_coord).clamp(0.01, 100.0);
                if snap_enabled && grid_increment > 0.0 {
                    factor = (factor / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Scale(Vec3::new(factor, 1.0, 1.0)));
            }
            GizmoPart::ScaleY => {
                let current_coord = ray.closest_point_on_line(drag.start_pivot, axis_y);
                let initial_coord = drag.initial_hit_coord.abs().max(0.1);
                let mut factor = (current_coord / initial_coord).clamp(0.01, 100.0);
                if snap_enabled && grid_increment > 0.0 {
                    factor = (factor / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Scale(Vec3::new(1.0, factor, 1.0)));
            }
            GizmoPart::ScaleZ => {
                let current_coord = ray.closest_point_on_line(drag.start_pivot, axis_z);
                let initial_coord = drag.initial_hit_coord.abs().max(0.1);
                let mut factor = (current_coord / initial_coord).clamp(0.01, 100.0);
                if snap_enabled && grid_increment > 0.0 {
                    factor = (factor / grid_increment).round() * grid_increment;
                }
                delta = Some(GizmoDelta::Scale(Vec3::new(1.0, 1.0, factor)));
            }
        }
    }

    // 4. Drag End
    let mut drag_ended = false;
    if lmb_released && gizmo_state.drag_state.is_some() {
        drag_ended = true;
        gizmo_state.drag_state = None;
    }

    // 5. Drawing Gizmo Graphics
    let active_part = gizmo_state
        .drag_state
        .as_ref()
        .map(|d| d.active_part)
        .or(gizmo_state.hovered_part);

    let col_x = if active_part == Some(GizmoPart::AxisX) || active_part == Some(GizmoPart::ScaleX) {
        Color32::from_rgb(255, 230, 80) // Yellow highlight
    } else {
        Color32::from_rgb(235, 65, 65) // Red
    };

    let col_y = if active_part == Some(GizmoPart::AxisY) || active_part == Some(GizmoPart::ScaleY) {
        Color32::from_rgb(255, 230, 80)
    } else {
        Color32::from_rgb(65, 225, 75) // Green
    };

    let col_z = if active_part == Some(GizmoPart::AxisZ) || active_part == Some(GizmoPart::ScaleZ) {
        Color32::from_rgb(255, 230, 80)
    } else {
        Color32::from_rgb(65, 130, 245) // Blue
    };

    let thickness = 2.5;

    match mode {
        GizmoMode::Translate => {
            // Draw axis lines and arrowheads
            if let Some(sx) = screen_x {
                painter.line_segment([center_screen, sx], Stroke::new(thickness, col_x));
                draw_arrowhead(painter, center_screen, sx, col_x);
            }
            if let Some(sy) = screen_y {
                painter.line_segment([center_screen, sy], Stroke::new(thickness, col_y));
                draw_arrowhead(painter, center_screen, sy, col_y);
            }
            if let Some(sz) = screen_z {
                painter.line_segment([center_screen, sz], Stroke::new(thickness, col_z));
                draw_arrowhead(painter, center_screen, sz, col_z);
            }

            // Draw all three planar handles (PlaneXY, PlaneXZ, PlaneYZ)
            let p_xy = pivot_world + (axis_x + axis_y) * (gizmo_3d_len * 0.35);
            let p_xz = pivot_world + (axis_x + axis_z) * (gizmo_3d_len * 0.35);
            let p_yz = pivot_world + (axis_y + axis_z) * (gizmo_3d_len * 0.35);

            if let Some(sxy) = world_to_screen_pos(p_xy, camera_view_proj, viewport_rect) {
                let col = if active_part == Some(GizmoPart::PlaneXY) {
                    Color32::from_rgb(255, 230, 80)
                } else {
                    Color32::from_rgba_unmultiplied(65, 130, 245, 190)
                };
                let r = Rect::from_center_size(sxy, EguiVec2::splat(9.0));
                painter.rect_filled(r, 2.0, col);
                painter.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }

            if let Some(sxz) = world_to_screen_pos(p_xz, camera_view_proj, viewport_rect) {
                let col = if active_part == Some(GizmoPart::PlaneXZ) {
                    Color32::from_rgb(255, 230, 80)
                } else {
                    Color32::from_rgba_unmultiplied(65, 225, 75, 190)
                };
                let r = Rect::from_center_size(sxz, EguiVec2::splat(9.0));
                painter.rect_filled(r, 2.0, col);
                painter.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }

            if let Some(syz) = world_to_screen_pos(p_yz, camera_view_proj, viewport_rect) {
                let col = if active_part == Some(GizmoPart::PlaneYZ) {
                    Color32::from_rgb(255, 230, 80)
                } else {
                    Color32::from_rgba_unmultiplied(235, 65, 65, 190)
                };
                let r = Rect::from_center_size(syz, EguiVec2::splat(9.0));
                painter.rect_filled(r, 2.0, col);
                painter.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
        }
        GizmoMode::Scale => {
            // Draw scale sticks and cube endpoints
            if let Some(sx) = screen_x {
                painter.line_segment([center_screen, sx], Stroke::new(thickness, col_x));
                painter.rect_filled(Rect::from_center_size(sx, EguiVec2::splat(9.0)), 1.5, col_x);
            }
            if let Some(sy) = screen_y {
                painter.line_segment([center_screen, sy], Stroke::new(thickness, col_y));
                painter.rect_filled(Rect::from_center_size(sy, EguiVec2::splat(9.0)), 1.5, col_y);
            }
            if let Some(sz) = screen_z {
                painter.line_segment([center_screen, sz], Stroke::new(thickness, col_z));
                painter.rect_filled(Rect::from_center_size(sz, EguiVec2::splat(9.0)), 1.5, col_z);
            }

            // Central uniform scale box
            let center_col = if active_part == Some(GizmoPart::ScaleUniform) {
                Color32::from_rgb(255, 230, 80)
            } else {
                Color32::from_gray(220)
            };
            painter.rect_filled(
                Rect::from_center_size(center_screen, EguiVec2::splat(10.0)),
                2.0,
                center_col,
            );
        }
        GizmoMode::Rotate => {
            // Draw projected rotation rings
            draw_projected_ring(
                painter,
                camera_view_proj,
                viewport_rect,
                pivot_world,
                axis_y,
                axis_z,
                gizmo_3d_len,
                if active_part == Some(GizmoPart::RotateX) {
                    Color32::from_rgb(255, 230, 80)
                } else {
                    col_x
                },
            );
            draw_projected_ring(
                painter,
                camera_view_proj,
                viewport_rect,
                pivot_world,
                axis_z,
                axis_x,
                gizmo_3d_len,
                if active_part == Some(GizmoPart::RotateY) {
                    Color32::from_rgb(255, 230, 80)
                } else {
                    col_y
                },
            );
            draw_projected_ring(
                painter,
                camera_view_proj,
                viewport_rect,
                pivot_world,
                axis_x,
                axis_y,
                gizmo_3d_len,
                if active_part == Some(GizmoPart::RotateZ) {
                    Color32::from_rgb(255, 230, 80)
                } else {
                    col_z
                },
            );
        }
    }

    let is_interacting = gizmo_state.is_dragging();
    let consumed = is_interacting || hovered.is_some();

    GizmoInteractionResult {
        consumed,
        is_dragging: is_interacting,
        delta,
        drag_started,
        drag_ended,
    }
}

fn dist_to_segment_2d(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-4 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    p.distance(proj)
}

fn dist_to_projected_ring(
    mouse: Pos2,
    view_proj: Mat4,
    viewport_rect: Rect,
    center: Vec3,
    u_axis: Vec3,
    v_axis: Vec3,
    radius: f32,
) -> f32 {
    let segments = 32;
    let mut min_dist = f32::MAX;
    let mut prev_pt = None;
    for i in 0..=segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let pos = center + (u_axis * theta.cos() + v_axis * theta.sin()) * radius;
        if let Some(screen_p) = world_to_screen_pos(pos, view_proj, viewport_rect) {
            if let Some(prev) = prev_pt {
                let d = dist_to_segment_2d(mouse, prev, screen_p);
                if d < min_dist {
                    min_dist = d;
                }
            }
            prev_pt = Some(screen_p);
        } else {
            prev_pt = None;
        }
    }
    min_dist
}

fn draw_arrowhead(painter: &Painter, start: Pos2, end: Pos2, color: Color32) {
    let dir = (end - start).normalized();
    let perp = EguiVec2::new(-dir.y, dir.x);
    let arrow_len = 8.5;
    let arrow_width = 4.2;

    let base = end - dir * arrow_len;
    let p1 = base + perp * arrow_width;
    let p2 = base - perp * arrow_width;

    painter.add(egui::Shape::convex_polygon(
        vec![end, p1, p2],
        color,
        Stroke::NONE,
    ));
}

#[allow(clippy::too_many_arguments)]
fn draw_projected_ring(
    painter: &Painter,
    view_proj: Mat4,
    viewport_rect: Rect,
    center: Vec3,
    u_axis: Vec3,
    v_axis: Vec3,
    radius: f32,
    color: Color32,
) {
    let segments = 36;
    let mut points = Vec::with_capacity(segments);
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let pos = center + (u_axis * theta.cos() + v_axis * theta.sin()) * radius;
        if let Some(screen_p) = world_to_screen_pos(pos, view_proj, viewport_rect) {
            points.push(screen_p);
        }
    }

    if points.len() > 2 {
        for i in 0..points.len() {
            let next = (i + 1) % points.len();
            painter.line_segment([points[i], points[next]], Stroke::new(2.5, color));
        }
    }
}
