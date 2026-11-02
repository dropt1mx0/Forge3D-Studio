use forge_math::{Aabb, Mat4, Ray, Vec2, Vec3};

/// Projection mode for the 3D viewport camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

/// Standard preset viewpoints for 3D navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPreset {
    Front,
    Back,
    Right,
    Left,
    Top,
    Bottom,
}

pub const MIN_CAMERA_DISTANCE: f32 = 0.05;
pub const MAX_CAMERA_DISTANCE: f32 = 5000.0;
pub const BASE_ZOOM_SENSITIVITY: f32 = 0.0025;

fn default_zoom_to_cursor() -> bool {
    true
}

fn default_smooth_zoom() -> bool {
    true
}

/// Settings controlling navigation sensitivity and speed for the 3D viewport camera.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CameraSettings {
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub fly_speed: f32,
    pub walk_height: f32,
    pub invert_y: bool,
    #[serde(default = "default_zoom_to_cursor")]
    pub zoom_to_cursor: bool,
    #[serde(default = "default_smooth_zoom")]
    pub smooth_zoom: bool,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            orbit_sensitivity: 0.007,
            pan_sensitivity: 0.0015,
            zoom_sensitivity: 1.0,
            fly_speed: 5.0,
            walk_height: 1.6,
            invert_y: false,
            zoom_to_cursor: true,
            smooth_zoom: true,
        }
    }
}

/// A comprehensive 3D camera supporting Orbit, Pan, Zoom, Dolly, Presets, Fly/Walk, and Orthographic/Perspective projection.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_deg: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub aspect_ratio: f32,
    pub projection: CameraProjection,
    pub ortho_size: f32,
    pub settings: CameraSettings,
    pub zoom_target_eye: Vec3,
    pub zoom_target_target: Vec3,
    pub zoom_target_ortho_size: f32,
    pub is_smooth_zooming: bool,
}

impl Default for Camera {
    fn default() -> Self {
        let eye = Vec3::new(4.5, 3.5, 5.5);
        let target = Vec3::new(0.0, 0.5, 0.0);
        Self {
            eye,
            target,
            up: Vec3::Y,
            fov_y_deg: 45.0,
            z_near: 0.1,
            z_far: 1000.0,
            aspect_ratio: 16.0 / 9.0,
            projection: CameraProjection::Perspective,
            ortho_size: 4.0,
            settings: CameraSettings::default(),
            zoom_target_eye: eye,
            zoom_target_target: target,
            zoom_target_ortho_size: 4.0,
            is_smooth_zooming: false,
        }
    }
}

impl Camera {
    /// Creates a camera positioned at `eye`, looking at `target`.
    pub fn new(eye: Vec3, target: Vec3, aspect_ratio: f32) -> Self {
        Self {
            eye,
            target,
            up: Vec3::Y,
            fov_y_deg: 45.0,
            z_near: 0.1,
            z_far: 1000.0,
            aspect_ratio,
            projection: CameraProjection::Perspective,
            ortho_size: 4.0,
            settings: CameraSettings::default(),
            zoom_target_eye: eye,
            zoom_target_target: target,
            zoom_target_ortho_size: 4.0,
            is_smooth_zooming: false,
        }
    }

    /// Sets the viewport aspect ratio (width / height).
    pub fn set_aspect_ratio(&mut self, width: f32, height: f32) {
        if height > 0.0 && width > 0.0 {
            self.aspect_ratio = width / height;
        }
    }

    /// Computes the view matrix: `look_at_rh(eye, target, up)`.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    /// Computes the projection matrix for wgpu (clip space Z in [0, 1]).
    pub fn proj_matrix(&self) -> Mat4 {
        match self.projection {
            CameraProjection::Perspective => Mat4::perspective_rh(
                self.fov_y_deg.to_radians(),
                self.aspect_ratio,
                self.z_near,
                self.z_far,
            ),
            CameraProjection::Orthographic => {
                let half_h = self.ortho_size.max(0.01);
                let half_w = half_h * self.aspect_ratio;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, self.z_near, self.z_far)
            }
        }
    }

    /// Computes combined `proj * view` matrix.
    pub fn view_proj_matrix(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }

    /// Normalized forward vector towards target.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.eye).normalize_or_zero()
    }

    /// Normalized horizontal right vector.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize_or_zero()
    }

    /// Normalized true up vector perpendicular to forward and right.
    pub fn camera_up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize_or_zero()
    }

    /// Distance from the camera eye to its target pivot point.
    pub fn distance(&self) -> f32 {
        (self.eye - self.target).length()
    }

    /// Synchronizes smooth zoom target values to current camera state and cancels ongoing smooth zoom transitions.
    pub fn sync_zoom_targets(&mut self) {
        self.zoom_target_eye = self.eye;
        self.zoom_target_target = self.target;
        self.zoom_target_ortho_size = self.ortho_size;
        self.is_smooth_zooming = false;
    }

    /// Orbit rotation around the target point.
    /// `delta_yaw`: horizontal rotation in radians (around world Y axis).
    /// `delta_pitch`: vertical rotation in radians (around camera right axis).
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let offset = self.eye - self.target;
        let radius = offset.length();
        if radius < 1e-4 {
            return;
        }

        let pitch_sign = if self.settings.invert_y { -1.0 } else { 1.0 };

        // Spherical coordinate angles
        let mut yaw = offset.z.atan2(offset.x);
        let mut pitch = (offset.y / radius).clamp(-0.999, 0.999).asin();

        yaw -= delta_yaw;
        pitch = (pitch - delta_pitch * pitch_sign).clamp(
            -std::f32::consts::FRAC_PI_2 + 0.02,
            std::f32::consts::FRAC_PI_2 - 0.02,
        );

        let new_offset = Vec3::new(
            radius * pitch.cos() * yaw.cos(),
            radius * pitch.sin(),
            radius * pitch.cos() * yaw.sin(),
        );

        self.eye = self.target + new_offset;
        self.up = Vec3::Y;
        self.sync_zoom_targets();
    }

    /// Step rotation around focus point by 15-degree increments (for Numpad 2/4/6/8).
    pub fn step_rotate(&mut self, yaw_steps: i32, pitch_steps: i32) {
        let step_rad = 15.0_f32.to_radians();
        self.orbit(yaw_steps as f32 * step_rad, pitch_steps as f32 * step_rad);
    }

    /// Pan camera along viewport right and up axes.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = self.forward();
        let right = forward.cross(self.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let dist = (self.eye - self.target).length();
        let factor = match self.projection {
            CameraProjection::Perspective => (dist * self.settings.pan_sensitivity).max(0.001),
            CameraProjection::Orthographic => (self.ortho_size * 0.002).max(0.001),
        };

        let translation = -right * delta_x * factor + up * delta_y * factor;
        self.eye += translation;
        self.target += translation;
        self.sync_zoom_targets();
    }

    /// Backward-compatible immediate zoom step. Positive `factor` zooms in, negative zooms out.
    pub fn zoom(&mut self, factor: f32) {
        if !factor.is_finite() || factor.abs() < 1e-5 {
            return;
        }
        let sensitivity = BASE_ZOOM_SENSITIVITY * self.settings.zoom_sensitivity.clamp(0.1, 5.0);
        let raw_exponent = -factor * 15.0 * sensitivity;
        let clamped_exponent = raw_exponent.clamp(-1.2, 1.2);
        let zoom_multiplier = clamped_exponent.exp();
        self.apply_zoom_step(zoom_multiplier, None, false);
        self.sync_zoom_targets();
    }

    /// Adaptive viewport zoom with cursor anchoring and smooth transition options.
    ///
    /// Mathematical model:
    /// `distance_new = distance_old * exp(-scroll_delta * sensitivity)`
    ///
    /// In egui, `scroll_delta > 0` on forward wheel scroll (Zoom In), decreasing distance.
    /// `scroll_delta < 0` on backward wheel scroll (Zoom Out), increasing distance.
    pub fn zoom_adaptive(&mut self, scroll_delta: f32, cursor_world_hit: Option<Vec3>) {
        if !scroll_delta.is_finite() || scroll_delta.abs() < 1e-5 {
            return;
        }

        let sensitivity = BASE_ZOOM_SENSITIVITY * self.settings.zoom_sensitivity.clamp(0.1, 5.0);
        let raw_exponent = -scroll_delta * sensitivity;
        let clamped_exponent = raw_exponent.clamp(-1.2, 1.2);
        let zoom_multiplier = clamped_exponent.exp();

        if self.settings.smooth_zoom {
            if !self.is_smooth_zooming {
                self.zoom_target_eye = self.eye;
                self.zoom_target_target = self.target;
                self.zoom_target_ortho_size = self.ortho_size;
                self.is_smooth_zooming = true;
            }

            self.apply_zoom_step(zoom_multiplier, cursor_world_hit, true);
        } else {
            self.apply_zoom_step(zoom_multiplier, cursor_world_hit, false);
            self.sync_zoom_targets();
        }
    }

    /// Applies a multiplicative zoom step to either the active or smooth-target camera state.
    fn apply_zoom_step(
        &mut self,
        zoom_multiplier: f32,
        cursor_world_hit: Option<Vec3>,
        to_target: bool,
    ) {
        let (ref_eye, ref_target, ref_ortho) = if to_target {
            (
                self.zoom_target_eye,
                self.zoom_target_target,
                self.zoom_target_ortho_size,
            )
        } else {
            (self.eye, self.target, self.ortho_size)
        };

        match self.projection {
            CameraProjection::Perspective => {
                let offset = ref_eye - ref_target;
                let current_dist = offset.length();
                if current_dist < 1e-4 || !current_dist.is_finite() {
                    return;
                }

                // If updating smooth target, prevent unbounded accumulation beyond [0.15 * current, 6.0 * current]
                let (min_d, max_d) = if to_target {
                    let actual_dist = (self.eye - self.target).length().max(MIN_CAMERA_DISTANCE);
                    (
                        (actual_dist * 0.15).max(MIN_CAMERA_DISTANCE),
                        (actual_dist * 6.0).min(MAX_CAMERA_DISTANCE),
                    )
                } else {
                    (MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE)
                };

                let target_dist = (current_dist * zoom_multiplier).clamp(min_d, max_d);
                let factor = target_dist / current_dist;

                let (new_eye, new_target) = if self.settings.zoom_to_cursor
                    && let Some(hit) = cursor_world_hit
                    && hit.is_finite()
                {
                    // Scale camera eye and target around hit point:
                    // new_eye = hit + (ref_eye - hit) * factor
                    // new_target = hit + (ref_target - hit) * factor
                    let cand_eye = hit + (ref_eye - hit) * factor;
                    let cand_target = hit + (ref_target - hit) * factor;

                    if cand_eye.is_finite() && cand_target.is_finite() {
                        (cand_eye, cand_target)
                    } else {
                        (
                            ref_target + offset.normalize_or_zero() * target_dist,
                            ref_target,
                        )
                    }
                } else {
                    (
                        ref_target + offset.normalize_or_zero() * target_dist,
                        ref_target,
                    )
                };

                if to_target {
                    self.zoom_target_eye = new_eye;
                    self.zoom_target_target = new_target;
                } else {
                    self.eye = new_eye;
                    self.target = new_target;
                }
            }
            CameraProjection::Orthographic => {
                let current_size = ref_ortho.max(0.001);
                let (min_s, max_s) = if to_target {
                    let actual_size = self.ortho_size.max(MIN_CAMERA_DISTANCE);
                    (
                        (actual_size * 0.15).max(MIN_CAMERA_DISTANCE),
                        (actual_size * 6.0).min(MAX_CAMERA_DISTANCE),
                    )
                } else {
                    (MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE)
                };

                let target_size = (current_size * zoom_multiplier).clamp(min_s, max_s);
                let factor = target_size / current_size;

                let (new_eye, new_target) = if self.settings.zoom_to_cursor
                    && let Some(hit) = cursor_world_hit
                    && hit.is_finite()
                {
                    let cand_target = hit + (ref_target - hit) * factor;
                    let delta_target = cand_target - ref_target;
                    let cand_eye = ref_eye + delta_target;
                    if cand_eye.is_finite() && cand_target.is_finite() {
                        (cand_eye, cand_target)
                    } else {
                        (ref_eye, ref_target)
                    }
                } else {
                    (ref_eye, ref_target)
                };

                if to_target {
                    self.zoom_target_eye = new_eye;
                    self.zoom_target_target = new_target;
                    self.zoom_target_ortho_size = target_size;
                } else {
                    self.eye = new_eye;
                    self.target = new_target;
                    self.ortho_size = target_size;
                }
            }
        }
    }

    /// Advances smooth zoom interpolation based on frame time `dt` (in seconds).
    ///
    /// Uses exponential time-based interpolation: `alpha = 1.0 - exp(-lambda * dt)`.
    /// Returns `true` if smooth zoom is still active and moving, or `false` if converged/inactive.
    pub fn update_smooth_zoom(&mut self, dt: f32) -> bool {
        if !self.is_smooth_zooming || !self.settings.smooth_zoom {
            self.is_smooth_zooming = false;
            return false;
        }

        let dt_clamped = dt.clamp(0.0005, 0.1);
        let lambda = 18.0;
        let alpha = 1.0 - (-lambda * dt_clamped).exp();

        self.eye = self.eye.lerp(self.zoom_target_eye, alpha);
        self.target = self.target.lerp(self.zoom_target_target, alpha);
        self.ortho_size += (self.zoom_target_ortho_size - self.ortho_size) * alpha;

        let eye_diff = (self.eye - self.zoom_target_eye).length();
        let target_diff = (self.target - self.zoom_target_target).length();
        let ortho_diff = (self.ortho_size - self.zoom_target_ortho_size).abs();

        if eye_diff < 0.001 && target_diff < 0.001 && ortho_diff < 0.001 {
            self.eye = self.zoom_target_eye;
            self.target = self.zoom_target_target;
            self.ortho_size = self.zoom_target_ortho_size;
            self.is_smooth_zooming = false;
            false
        } else {
            true
        }
    }

    /// Dolly camera forward or backward along its look vector.
    pub fn dolly(&mut self, delta: f32) {
        let forward = self.forward();
        self.eye += forward * delta;
        self.target += forward * delta;
        self.sync_zoom_targets();
    }

    /// Sets the orbit pivot point without changing the camera's eye position.
    pub fn set_orbit_pivot(&mut self, new_pivot: Vec3) {
        self.target = new_pivot;
        self.sync_zoom_targets();
    }

    /// Resets camera to standard default overview angle and distance.
    pub fn reset_view(&mut self) {
        self.eye = Vec3::new(4.5, 3.5, 5.5);
        self.target = Vec3::new(0.0, 0.5, 0.0);
        self.up = Vec3::Y;
        self.fov_y_deg = 45.0;
        self.projection = CameraProjection::Perspective;
        self.ortho_size = 4.0;
        self.sync_zoom_targets();
    }

    /// Fly / Walk translation: moves both `eye` and `target` so orientation is preserved.
    /// `dir`: (x = right/left, y = up/down, z = forward/back) in camera space.
    pub fn fly_move(&mut self, dir: Vec3, dt: f32, speed_mult: f32, is_walk: bool) {
        let fwd = if is_walk {
            Vec3::new(self.forward().x, 0.0, self.forward().z).normalize_or_zero()
        } else {
            self.forward()
        };
        let right = if is_walk {
            Vec3::new(self.right().x, 0.0, self.right().z).normalize_or_zero()
        } else {
            self.right()
        };
        let up = if is_walk { Vec3::Y } else { self.camera_up() };

        let speed = self.settings.fly_speed * speed_mult * dt;
        let mut move_delta = right * dir.x * speed + up * dir.y * speed + fwd * dir.z * speed;

        if is_walk {
            // Lock camera eye Y to walk height
            move_delta.y = 0.0;
            self.eye.y = self.settings.walk_height;
        }

        self.eye += move_delta;
        self.target += move_delta;
        self.sync_zoom_targets();
    }

    /// Fly / Walk orientation look: rotates the look direction from the fixed `eye` position.
    pub fn fly_look(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let forward = self.forward();
        let dist = (self.target - self.eye).length().max(1.0);

        let pitch_sign = if self.settings.invert_y { -1.0 } else { 1.0 };

        let mut yaw = forward.z.atan2(forward.x);
        let mut pitch = forward.y.clamp(-0.999, 0.999).asin();

        yaw -= delta_yaw;
        pitch = (pitch - delta_pitch * pitch_sign).clamp(
            -std::f32::consts::FRAC_PI_2 + 0.02,
            std::f32::consts::FRAC_PI_2 - 0.02,
        );

        let new_forward = Vec3::new(
            pitch.cos() * yaw.cos(),
            pitch.sin(),
            pitch.cos() * yaw.sin(),
        )
        .normalize_or_zero();

        self.target = self.eye + new_forward * dist;
        self.up = Vec3::Y;
        self.sync_zoom_targets();
    }

    /// Toggles between Perspective and Orthographic projection.
    pub fn toggle_projection(&mut self) {
        match self.projection {
            CameraProjection::Perspective => {
                let dist = (self.eye - self.target).length();
                self.ortho_size = (dist * (self.fov_y_deg * 0.5).to_radians().tan()).max(0.5);
                self.projection = CameraProjection::Orthographic;
            }
            CameraProjection::Orthographic => {
                self.projection = CameraProjection::Perspective;
            }
        }
        self.sync_zoom_targets();
    }

    /// Aligns camera with a standard preset viewpoint.
    pub fn set_view_preset(&mut self, preset: ViewPreset) {
        let dist = (self.eye - self.target).length().max(1.0);
        match preset {
            ViewPreset::Front => {
                self.eye = self.target + Vec3::new(0.0, 0.0, dist);
                self.up = Vec3::Y;
            }
            ViewPreset::Back => {
                self.eye = self.target + Vec3::new(0.0, 0.0, -dist);
                self.up = Vec3::Y;
            }
            ViewPreset::Right => {
                self.eye = self.target + Vec3::new(dist, 0.0, 0.0);
                self.up = Vec3::Y;
            }
            ViewPreset::Left => {
                self.eye = self.target + Vec3::new(-dist, 0.0, 0.0);
                self.up = Vec3::Y;
            }
            ViewPreset::Top => {
                self.eye = self.target + Vec3::new(0.0, dist, 0.0);
                self.up = -Vec3::Z;
            }
            ViewPreset::Bottom => {
                self.eye = self.target + Vec3::new(0.0, -dist, 0.0);
                self.up = Vec3::Z;
            }
        }
        self.sync_zoom_targets();
    }

    /// Centers camera on a given bounding box (`Aabb`) and adjusts distance to fit.
    pub fn frame_box(&mut self, aabb: &Aabb) {
        if aabb.is_empty() {
            return;
        }

        let center = aabb.center();
        let extents = aabb.extents();
        let radius = extents.length().max(0.5);

        let current_dir = (self.eye - self.target).normalize_or_zero();
        let dir = if current_dir.length_squared() < 0.1 {
            Vec3::new(0.5, 0.4, 0.7).normalize()
        } else {
            current_dir
        };

        let half_fov = (self.fov_y_deg * 0.5).to_radians().sin().max(0.1);
        let dist = (radius / half_fov) * 1.5;

        self.target = center;
        self.eye = center + dir * dist;
        self.ortho_size = radius * 1.5;
        self.up = Vec3::Y;
        self.sync_zoom_targets();
    }

    /// Converts a 2D screen/viewport position into a 3D picking `Ray` in world space.
    pub fn screen_to_ray(&self, screen_pos: Vec2, viewport_size: Vec2) -> Ray {
        if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
            return Ray::new(self.eye, (self.target - self.eye).normalize_or_zero());
        }

        // Convert to Normalized Device Coordinates (NDC) [-1, 1]
        let ndc_x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0;

        let inv_view_proj = self.view_proj_matrix().inverse();

        match self.projection {
            CameraProjection::Perspective => {
                let far_ndc = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
                let far_world_h = inv_view_proj * far_ndc;
                let far_world = far_world_h.truncate() / far_world_h.w;

                let direction = (far_world - self.eye).normalize_or_zero();
                Ray::new(self.eye, direction)
            }
            CameraProjection::Orthographic => {
                let near_ndc = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
                let near_world_h = inv_view_proj * near_ndc;
                let ray_origin = near_world_h.truncate() / near_world_h.w;
                let ray_dir = (self.target - self.eye).normalize_or_zero();
                Ray::new(ray_origin, ray_dir)
            }
        }
    }

    /// Projects a 3D world coordinate to 2D screen space (within `viewport_size`).
    /// Returns `Some(Vec2(screen_x, screen_y))` if the point is in front of the camera, or `None` if clipped.
    pub fn world_to_screen(&self, world_pos: Vec3, viewport_size: Vec2) -> Option<Vec2> {
        if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
            return None;
        }

        let clip =
            self.view_proj_matrix() * glam::Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        if clip.w <= 0.0001 && self.projection == CameraProjection::Perspective {
            return None;
        }

        let ndc = clip.truncate() / clip.w;
        if ndc.z < 0.0 || ndc.z > 1.0 {
            return None;
        }

        let screen_x = (ndc.x + 1.0) * 0.5 * viewport_size.x;
        let screen_y = (1.0 - ndc.y) * 0.5 * viewport_size.y;
        Some(Vec2::new(screen_x, screen_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_matrices_finite() {
        let mut cam = Camera::default();
        let vp = cam.view_proj_matrix();
        assert!(vp.is_finite());

        cam.toggle_projection();
        assert_eq!(cam.projection, CameraProjection::Orthographic);
        let vp_ortho = cam.view_proj_matrix();
        assert!(vp_ortho.is_finite());
    }

    #[test]
    fn test_camera_presets() {
        let mut cam = Camera::default();
        cam.set_view_preset(ViewPreset::Front);
        assert!((cam.eye.z - cam.target.z) > 0.0);
        assert!(cam.eye.x.abs() < 1e-4);

        cam.set_view_preset(ViewPreset::Top);
        assert!((cam.eye.y - cam.target.y) > 0.0);
        assert!(cam.eye.z.abs() < 1e-4);
    }

    #[test]
    fn test_camera_frame_box() {
        let mut cam = Camera::default();
        let aabb = Aabb::new(Vec3::new(-2.0, -1.0, -2.0), Vec3::new(2.0, 1.0, 2.0));
        cam.frame_box(&aabb);
        assert!((cam.target - aabb.center()).length() < 1e-4);
        assert!(cam.eye.is_finite());
    }

    #[test]
    fn test_screen_to_ray() {
        let cam = Camera::default();
        let ray = cam.screen_to_ray(Vec2::new(400.0, 300.0), Vec2::new(800.0, 600.0));
        assert!(ray.origin.is_finite());
        assert!(ray.direction.is_finite());
        assert!((ray.direction.length() - 1.0).abs() < 1e-4);
        let expected_dir = (cam.target - cam.eye).normalize();
        assert!(
            (ray.direction - expected_dir).length() < 1e-3,
            "Ray direction {:?} did not match expected {:?}",
            ray.direction,
            expected_dir
        );
    }

    #[test]
    fn test_world_to_screen() {
        let cam = Camera::default();
        let screen_pos = cam.world_to_screen(cam.target, Vec2::new(800.0, 600.0));
        assert!(screen_pos.is_some());
        let pos = screen_pos.unwrap();
        // Target should project near center of screen
        assert!((pos.x - 400.0).abs() < 5.0);
        assert!((pos.y - 300.0).abs() < 5.0);
    }

    #[test]
    fn test_step_rotate() {
        let mut cam = Camera::default();
        let initial_eye = cam.eye;
        cam.step_rotate(1, 0); // Rotate 15 deg yaw
        assert_ne!(cam.eye, initial_eye);
        assert!((cam.eye.y - initial_eye.y).abs() < 1e-4); // Y should stay constant for pure yaw
    }

    #[test]
    fn test_fly_movement_and_look() {
        let mut cam = Camera::default();
        let initial_eye = cam.eye;
        // Move forward by 1 unit
        cam.fly_move(Vec3::new(0.0, 0.0, 1.0), 1.0, 1.0, false);
        assert_ne!(cam.eye, initial_eye);
        assert!((cam.forward().length() - 1.0).abs() < 1e-4);

        // Fly look changes target
        let initial_target = cam.target;
        cam.fly_look(0.1, 0.0);
        assert_ne!(cam.target, initial_target);
    }

    #[test]
    fn test_camera_reset_and_pivot() {
        let mut cam = Camera::default();
        cam.set_orbit_pivot(Vec3::new(10.0, 5.0, -2.0));
        assert_eq!(cam.target, Vec3::new(10.0, 5.0, -2.0));

        cam.reset_view();
        assert_eq!(cam.target, Vec3::new(0.0, 0.5, 0.0));
    }

    #[test]
    fn test_adaptive_zoom_multiplicative_formula() {
        let mut cam = Camera {
            settings: CameraSettings {
                smooth_zoom: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let d0 = cam.distance();

        // Positive scroll delta in egui = wheel scrolled forward => zoom in => distance decreases
        cam.zoom_adaptive(30.0, None);
        let d1 = cam.distance();
        assert!(d1 < d0, "Zoom in must decrease distance: d1={d1}, d0={d0}");

        // Expected multiplier: exp(-30.0 * 0.0025 * 1.0) = exp(-0.075) = ~0.9277
        let expected_ratio = (-30.0 * BASE_ZOOM_SENSITIVITY).exp();
        assert!((d1 / d0 - expected_ratio).abs() < 1e-4);

        // Negative scroll delta in egui = wheel scrolled backward => zoom out => distance increases
        cam.zoom_adaptive(-30.0, None);
        let d2 = cam.distance();
        assert!(d2 > d1, "Zoom out must increase distance: d2={d2}, d1={d1}");
        assert!((d2 - d0).abs() < 1e-3);
    }

    #[test]
    fn test_adaptive_zoom_bounds_clamping() {
        let mut cam = Camera {
            settings: CameraSettings {
                smooth_zoom: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // Extreme zoom in repeatedly
        for _ in 0..100 {
            cam.zoom_adaptive(500.0, None);
        }
        assert!(
            cam.distance() >= MIN_CAMERA_DISTANCE - 1e-4,
            "Distance must be clamped at MIN_CAMERA_DISTANCE: {}",
            cam.distance()
        );

        // Extreme zoom out repeatedly
        for _ in 0..100 {
            cam.zoom_adaptive(-500.0, None);
        }
        assert!(
            cam.distance() <= MAX_CAMERA_DISTANCE + 1e-4,
            "Distance must be clamped at MAX_CAMERA_DISTANCE: {}",
            cam.distance()
        );
    }

    #[test]
    fn test_zoom_to_cursor_perspective_geometry() {
        let mut cam = Camera {
            settings: CameraSettings {
                smooth_zoom: false,
                zoom_to_cursor: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let hit_point = Vec3::new(1.0, 0.5, -0.5);
        let old_dir_to_hit = (hit_point - cam.eye).normalize();
        let old_offset_to_hit = hit_point - cam.eye;

        cam.zoom_adaptive(40.0, Some(hit_point));

        let new_offset_to_hit = hit_point - cam.eye;
        let new_dir_to_hit = new_offset_to_hit.normalize();

        // Direction to cursor hit point in camera space must remain strictly collinear
        assert!(
            (new_dir_to_hit - old_dir_to_hit).length() < 1e-4,
            "Cursor ray direction must remain invariant under Zoom to Cursor"
        );

        // The distance from camera eye to hit point must shrink proportionally
        assert!(new_offset_to_hit.length() < old_offset_to_hit.length());

        // Target must shift logically towards hit point without jump
        assert!(cam.target.is_finite());
        assert!(cam.eye.is_finite());
    }

    #[test]
    fn test_zoom_to_cursor_orthographic() {
        let mut cam = Camera {
            projection: CameraProjection::Orthographic,
            settings: CameraSettings {
                smooth_zoom: false,
                zoom_to_cursor: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let old_ortho = cam.ortho_size;

        let hit = Vec3::new(2.0, 1.0, 0.0);
        cam.zoom_adaptive(35.0, Some(hit));

        assert!(
            cam.ortho_size < old_ortho,
            "Ortho size must decrease on zoom in"
        );
        assert!(cam.eye.is_finite());
        assert!(cam.target.is_finite());
    }

    #[test]
    fn test_smooth_zoom_convergence_and_cancellation() {
        let mut cam = Camera::default();
        cam.settings.smooth_zoom = true;
        let initial_eye = cam.eye;

        cam.zoom_adaptive(50.0, None);
        assert!(cam.is_smooth_zooming);
        assert_eq!(
            cam.eye, initial_eye,
            "Smooth zoom target queues without jumping instantly"
        );

        // Advance several 60fps frames (dt = 0.016)
        let mut frames = 0;
        while cam.update_smooth_zoom(0.016) {
            frames += 1;
            assert!(
                frames < 200,
                "Smooth zoom must converge within reasonable time"
            );
        }

        assert!(!cam.is_smooth_zooming);
        assert!(cam.distance() < (initial_eye - cam.target).length());

        // Test navigation cancellation
        cam.zoom_adaptive(50.0, None);
        assert!(cam.is_smooth_zooming);
        cam.orbit(0.1, 0.05);
        assert!(
            !cam.is_smooth_zooming,
            "Orbit must synchronize and cancel smooth zoom"
        );
    }

    #[test]
    fn test_adaptive_zoom_nan_infinity_resilience() {
        let mut cam = Camera::default();
        let eye_before = cam.eye;

        // NaN scroll delta
        cam.zoom_adaptive(f32::NAN, None);
        assert_eq!(cam.eye, eye_before);

        // Infinity scroll delta
        cam.zoom_adaptive(f32::INFINITY, None);
        assert_eq!(cam.eye, eye_before);

        // NaN hit point
        cam.zoom_adaptive(20.0, Some(Vec3::new(f32::NAN, 0.0, 0.0)));
        assert!(cam.eye.is_finite());
        assert!(cam.target.is_finite());
    }
}
