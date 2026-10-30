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

/// Settings controlling navigation sensitivity and speed for the 3D viewport camera.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CameraSettings {
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub fly_speed: f32,
    pub walk_height: f32,
    pub invert_y: bool,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            orbit_sensitivity: 0.007,
            pan_sensitivity: 0.0015,
            zoom_sensitivity: 0.02,
            fly_speed: 5.0,
            walk_height: 1.6,
            invert_y: false,
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
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: Vec3::new(4.5, 3.5, 5.5),
            target: Vec3::new(0.0, 0.5, 0.0),
            up: Vec3::Y,
            fov_y_deg: 45.0,
            z_near: 0.1,
            z_far: 1000.0,
            aspect_ratio: 16.0 / 9.0,
            projection: CameraProjection::Perspective,
            ortho_size: 4.0,
            settings: CameraSettings::default(),
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
    }

    /// Zoom camera closer or further from target.
    /// Positive `factor` zooms in, negative zooms out.
    pub fn zoom(&mut self, factor: f32) {
        match self.projection {
            CameraProjection::Perspective => {
                let offset = self.eye - self.target;
                let dist = offset.length();
                let zoom_mult =
                    (1.0 - factor * self.settings.zoom_sensitivity * 5.0).clamp(0.05, 5.0);
                let new_dist = (dist * zoom_mult).clamp(0.1, 2000.0);
                self.eye = self.target + offset.normalize_or_zero() * new_dist;
            }
            CameraProjection::Orthographic => {
                let zoom_mult =
                    (1.0 - factor * self.settings.zoom_sensitivity * 5.0).clamp(0.05, 5.0);
                self.ortho_size = (self.ortho_size * zoom_mult).clamp(0.05, 5000.0);
            }
        }
    }

    /// Dolly camera forward or backward along its look vector.
    pub fn dolly(&mut self, delta: f32) {
        let forward = self.forward();
        self.eye += forward * delta;
        self.target += forward * delta;
    }

    /// Sets the orbit pivot point without changing the camera's eye position.
    pub fn set_orbit_pivot(&mut self, new_pivot: Vec3) {
        self.target = new_pivot;
    }

    /// Resets camera to standard default overview angle and distance.
    pub fn reset_view(&mut self) {
        self.eye = Vec3::new(4.5, 3.5, 5.5);
        self.target = Vec3::new(0.0, 0.5, 0.0);
        self.up = Vec3::Y;
        self.fov_y_deg = 45.0;
        self.projection = CameraProjection::Perspective;
        self.ortho_size = 4.0;
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
                let near_ndc = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
                let far_ndc = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

                let near_world_h = inv_view_proj * near_ndc;
                let far_world_h = inv_view_proj * far_ndc;

                let near_world = near_world_h.truncate() / near_world_h.w;
                let far_world = far_world_h.truncate() / far_world_h.w;

                let direction = (far_world - near_world).normalize_or_zero();
                Ray::new(near_world, direction)
            }
            CameraProjection::Orthographic => {
                let near_ndc = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
                let near_world_h = inv_view_proj * near_ndc;
                let ray_origin = near_world_h.truncate();
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
}
