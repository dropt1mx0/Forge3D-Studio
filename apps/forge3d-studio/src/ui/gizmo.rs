use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2 as EguiVec2};
use forge_math::{Mat4, Quat, Ray, Transform, Vec2, Vec3};
use forge_scene::{TransformMode, TransformOrientation};

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

/// Active drag state for the gizmo.
#[derive(Debug, Clone)]
pub struct GizmoDragState {
    pub active_part: GizmoPart,
    pub start_mouse: Vec2,
    pub start_transform: Transform,
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
/// Returns `true` if the gizmo was interacted with or dragged (consuming the pointer).
#[allow(clippy::too_many_arguments)]
pub fn render_and_interact_gizmo(
    gizmo_state: &mut GizmoState,
    painter: &Painter,
    camera_view_proj: Mat4,
    camera_eye: Vec3,
    camera_screen_to_ray: impl Fn(Vec2) -> Ray,
    viewport_rect: Rect,
    pivot_world: Vec3,
    active_transform: &mut Transform,
    mode: TransformMode,
    orientation: TransformOrientation,
    mouse_pos: Option<Pos2>,
    lmb_pressed: bool,
    lmb_down: bool,
    lmb_released: bool,
    snap_enabled: bool,
    grid_increment: f32,
) -> bool {
    if mode == TransformMode::Select {
        gizmo_state.hovered_part = None;
        gizmo_state.drag_state = None;
        return false;
    }

    let Some(center_screen) = world_to_screen_pos(pivot_world, camera_view_proj, viewport_rect)
    else {
        gizmo_state.hovered_part = None;
        return false;
    };

    // Calculate dynamic 3D scale so gizmo maintains a comfortable screen size (~90 pixels)
    let dist_to_cam = (camera_eye - pivot_world).length().max(0.5);
    let gizmo_3d_len = (dist_to_cam * 0.16).clamp(0.5, 50.0);

    // Orientation basis vectors
    let (axis_x, axis_y, axis_z) = match orientation {
        TransformOrientation::Global => (Vec3::X, Vec3::Y, Vec3::Z),
        TransformOrientation::Local => {
            let rot = active_transform.rotation;
            (
                rot.mul_vec3(Vec3::X),
                rot.mul_vec3(Vec3::Y),
                rot.mul_vec3(Vec3::Z),
            )
        }
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
        // A. Translation Gizmo
        if mode == TransformMode::Translate {
            // Plane handles
            let p_xy = pivot_world + (axis_x + axis_y) * (gizmo_3d_len * 0.35);
            let p_xz = pivot_world + (axis_x + axis_z) * (gizmo_3d_len * 0.35);
            let p_yz = pivot_world + (axis_y + axis_z) * (gizmo_3d_len * 0.35);

            if let Some(s_xz) = world_to_screen_pos(p_xz, camera_view_proj, viewport_rect)
                && mouse.distance(s_xz) < hit_dist_threshold
            {
                hovered = Some(GizmoPart::PlaneXZ);
            } else if let Some(s_xy) = world_to_screen_pos(p_xy, camera_view_proj, viewport_rect)
                && mouse.distance(s_xy) < hit_dist_threshold
            {
                hovered = Some(GizmoPart::PlaneXY);
            } else if let Some(s_yz) = world_to_screen_pos(p_yz, camera_view_proj, viewport_rect)
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
        } else if mode == TransformMode::Scale {
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
        } else if mode == TransformMode::Rotate {
            // Check rotation rings
            if let Some(sx) = screen_x
                && mouse.distance(sx) < hit_dist_threshold * 1.5
            {
                hovered = Some(GizmoPart::RotateX);
            } else if let Some(sy) = screen_y
                && mouse.distance(sy) < hit_dist_threshold * 1.5
            {
                hovered = Some(GizmoPart::RotateY);
            } else if let Some(sz) = screen_z
                && mouse.distance(sz) < hit_dist_threshold * 1.5
            {
                hovered = Some(GizmoPart::RotateZ);
            }
        }
    }

    if !gizmo_state.is_dragging() {
        gizmo_state.hovered_part = hovered;
    }

    // 2. Drag Start
    if lmb_pressed
        && let Some(part) = gizmo_state.hovered_part
        && let Some(mouse) = mouse_pos
    {
        let mouse_vec = Vec2::new(mouse.x - viewport_rect.min.x, mouse.y - viewport_rect.min.y);
        let ray = camera_screen_to_ray(mouse_vec);

        let initial_hit_coord = match part {
            GizmoPart::AxisX => ray.closest_point_on_line(pivot_world, axis_x),
            GizmoPart::AxisY => ray.closest_point_on_line(pivot_world, axis_y),
            GizmoPart::AxisZ => ray.closest_point_on_line(pivot_world, axis_z),
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
            start_transform: *active_transform,
            initial_hit_coord,
            initial_hit_point,
        });
    }

    // 3. Dragging Execution
    let is_interacting = gizmo_state.is_dragging();
    if let Some(drag) = &gizmo_state.drag_state
        && lmb_down
        && let Some(mouse) = mouse_pos
    {
        let mouse_vec = Vec2::new(mouse.x - viewport_rect.min.x, mouse.y - viewport_rect.min.y);
        let ray = camera_screen_to_ray(mouse_vec);

        match drag.active_part {
            GizmoPart::AxisX => {
                let current_coord = ray.closest_point_on_line(pivot_world, axis_x);
                let mut delta = current_coord - drag.initial_hit_coord;
                if snap_enabled && grid_increment > 0.0 {
                    delta = (delta / grid_increment).round() * grid_increment;
                }
                active_transform.translation = drag.start_transform.translation + axis_x * delta;
            }
            GizmoPart::AxisY => {
                let current_coord = ray.closest_point_on_line(pivot_world, axis_y);
                let mut delta = current_coord - drag.initial_hit_coord;
                if snap_enabled && grid_increment > 0.0 {
                    delta = (delta / grid_increment).round() * grid_increment;
                }
                active_transform.translation = drag.start_transform.translation + axis_y * delta;
            }
            GizmoPart::AxisZ => {
                let current_coord = ray.closest_point_on_line(pivot_world, axis_z);
                let mut delta = current_coord - drag.initial_hit_coord;
                if snap_enabled && grid_increment > 0.0 {
                    delta = (delta / grid_increment).round() * grid_increment;
                }
                active_transform.translation = drag.start_transform.translation + axis_z * delta;
            }
            GizmoPart::PlaneXZ => {
                if let Some(t) = ray.intersect_plane(pivot_world, axis_y) {
                    let mut delta = ray.point_at(t) - drag.initial_hit_point;
                    if snap_enabled && grid_increment > 0.0 {
                        delta.x = (delta.x / grid_increment).round() * grid_increment;
                        delta.z = (delta.z / grid_increment).round() * grid_increment;
                    }
                    active_transform.translation = drag.start_transform.translation + delta;
                }
            }
            GizmoPart::PlaneXY => {
                if let Some(t) = ray.intersect_plane(pivot_world, axis_z) {
                    let mut delta = ray.point_at(t) - drag.initial_hit_point;
                    if snap_enabled && grid_increment > 0.0 {
                        delta.x = (delta.x / grid_increment).round() * grid_increment;
                        delta.y = (delta.y / grid_increment).round() * grid_increment;
                    }
                    active_transform.translation = drag.start_transform.translation + delta;
                }
            }
            GizmoPart::PlaneYZ => {
                if let Some(t) = ray.intersect_plane(pivot_world, axis_x) {
                    let mut delta = ray.point_at(t) - drag.initial_hit_point;
                    if snap_enabled && grid_increment > 0.0 {
                        delta.y = (delta.y / grid_increment).round() * grid_increment;
                        delta.z = (delta.z / grid_increment).round() * grid_increment;
                    }
                    active_transform.translation = drag.start_transform.translation + delta;
                }
            }
            GizmoPart::RotateX => {
                if let Some(t) = ray.intersect_plane(pivot_world, axis_x) {
                    let v0 = (drag.initial_hit_point - pivot_world).normalize_or_zero();
                    let v1 = (ray.point_at(t) - pivot_world).normalize_or_zero();
                    let angle = v0.cross(v1).dot(axis_x).atan2(v0.dot(v1));
                    let rot_delta = Quat::from_axis_angle(axis_x, angle);
                    active_transform.rotation = rot_delta * drag.start_transform.rotation;
                }
            }
            GizmoPart::RotateY => {
                if let Some(t) = ray.intersect_plane(pivot_world, axis_y) {
                    let v0 = (drag.initial_hit_point - pivot_world).normalize_or_zero();
                    let v1 = (ray.point_at(t) - pivot_world).normalize_or_zero();
                    let angle = v0.cross(v1).dot(axis_y).atan2(v0.dot(v1));
                    let rot_delta = Quat::from_axis_angle(axis_y, angle);
                    active_transform.rotation = rot_delta * drag.start_transform.rotation;
                }
            }
            GizmoPart::RotateZ => {
                if let Some(t) = ray.intersect_plane(pivot_world, axis_z) {
                    let v0 = (drag.initial_hit_point - pivot_world).normalize_or_zero();
                    let v1 = (ray.point_at(t) - pivot_world).normalize_or_zero();
                    let angle = v0.cross(v1).dot(axis_z).atan2(v0.dot(v1));
                    let rot_delta = Quat::from_axis_angle(axis_z, angle);
                    active_transform.rotation = rot_delta * drag.start_transform.rotation;
                }
            }
            GizmoPart::ScaleUniform => {
                let start_d = (drag.start_mouse - Vec2::new(center_screen.x, center_screen.y))
                    .length()
                    .max(10.0);
                let curr_d = (Vec2::new(mouse.x, mouse.y)
                    - Vec2::new(center_screen.x, center_screen.y))
                .length();
                let factor = (curr_d / start_d).clamp(0.01, 100.0);
                active_transform.scale = drag.start_transform.scale * factor;
            }
            GizmoPart::ScaleX => {
                let current_coord = ray.closest_point_on_line(pivot_world, axis_x);
                let initial_coord = drag.initial_hit_coord.abs().max(0.1);
                let factor = (current_coord / initial_coord).clamp(0.01, 100.0);
                active_transform.scale.x = drag.start_transform.scale.x * factor;
            }
            GizmoPart::ScaleY => {
                let current_coord = ray.closest_point_on_line(pivot_world, axis_y);
                let initial_coord = drag.initial_hit_coord.abs().max(0.1);
                let factor = (current_coord / initial_coord).clamp(0.01, 100.0);
                active_transform.scale.y = drag.start_transform.scale.y * factor;
            }
            GizmoPart::ScaleZ => {
                let current_coord = ray.closest_point_on_line(pivot_world, axis_z);
                let initial_coord = drag.initial_hit_coord.abs().max(0.1);
                let factor = (current_coord / initial_coord).clamp(0.01, 100.0);
                active_transform.scale.z = drag.start_transform.scale.z * factor;
            }
        }
    }

    // 4. Drag End
    if lmb_released {
        gizmo_state.drag_state = None;
    }

    // 5. Drawing Gizmo Graphics with Anti-Aliased Lines & Colors
    let active_part = gizmo_state
        .drag_state
        .as_ref()
        .map(|d| d.active_part)
        .or(gizmo_state.hovered_part);

    let col_x = if active_part == Some(GizmoPart::AxisX) || active_part == Some(GizmoPart::ScaleX) {
        Color32::from_rgb(255, 230, 80) // Bright yellow highlight
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

    let thickness = 3.5;

    // Draw Translation
    if mode == TransformMode::Translate {
        // Axis lines & arrows
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

        // Draw planar square handles
        let p_xz = pivot_world + (axis_x + axis_z) * (gizmo_3d_len * 0.35);
        if let Some(sxz) = world_to_screen_pos(p_xz, camera_view_proj, viewport_rect) {
            let col = if active_part == Some(GizmoPart::PlaneXZ) {
                Color32::from_rgb(255, 230, 80)
            } else {
                Color32::from_rgba_unmultiplied(65, 225, 75, 160)
            };
            painter.rect_filled(Rect::from_center_size(sxz, EguiVec2::splat(10.0)), 2.0, col);
        }
    } else if mode == TransformMode::Scale {
        // Draw scale sticks & cubes
        if let Some(sx) = screen_x {
            painter.line_segment([center_screen, sx], Stroke::new(thickness, col_x));
            painter.rect_filled(Rect::from_center_size(sx, EguiVec2::splat(8.0)), 1.5, col_x);
        }
        if let Some(sy) = screen_y {
            painter.line_segment([center_screen, sy], Stroke::new(thickness, col_y));
            painter.rect_filled(Rect::from_center_size(sy, EguiVec2::splat(8.0)), 1.5, col_y);
        }
        if let Some(sz) = screen_z {
            painter.line_segment([center_screen, sz], Stroke::new(thickness, col_z));
            painter.rect_filled(Rect::from_center_size(sz, EguiVec2::splat(8.0)), 1.5, col_z);
        }

        // Central uniform scale box
        let center_col = if active_part == Some(GizmoPart::ScaleUniform) {
            Color32::from_rgb(255, 230, 80)
        } else {
            Color32::from_gray(220)
        };
        painter.rect_filled(
            Rect::from_center_size(center_screen, EguiVec2::splat(12.0)),
            2.0,
            center_col,
        );
    } else if mode == TransformMode::Rotate {
        // Draw rotation circle arcs
        draw_projected_ring(
            painter,
            camera_view_proj,
            viewport_rect,
            pivot_world,
            axis_x,
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
            axis_y,
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
            axis_z,
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

    is_interacting || hovered.is_some()
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

fn draw_arrowhead(painter: &Painter, start: Pos2, end: Pos2, color: Color32) {
    let dir = (end - start).normalized();
    let perp = EguiVec2::new(-dir.y, dir.x);
    let arrow_len = 10.0;
    let arrow_width = 5.0;

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
    _axis_normal: Vec3,
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
