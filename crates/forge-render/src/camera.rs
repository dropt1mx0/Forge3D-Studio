use forge_math::{Mat4, Ray, Vec2, Vec3};

/// A perspective 3D camera supporting Orbit, Pan, Zoom, and screen-to-ray conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_deg: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub aspect_ratio: f32,
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
        }
    }

    /// Sets the viewport aspect ratio (width / height).
    pub fn set_aspect_ratio(&mut self, width: f32, height: f32) {
        if height > 0.0 {
            self.aspect_ratio = width / height;
        }
    }

    /// Computes the view matrix: `look_at_rh(eye, target, up)`.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    /// Computes the perspective projection matrix for wgpu (clip space Z in [0, 1]).
    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y_deg.to_radians(),
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        )
    }

    /// Computes combined `proj * view` matrix.
    pub fn view_proj_matrix(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
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

        // Spherical coordinate angles
        let mut yaw = offset.z.atan2(offset.x);
        let mut pitch = (offset.y / radius).clamp(-0.999, 0.999).asin();

        yaw -= delta_yaw;
        pitch = (pitch - delta_pitch).clamp(
            -std::f32::consts::FRAC_PI_2 + 0.05,
            std::f32::consts::FRAC_PI_2 - 0.05,
        );

        let new_offset = Vec3::new(
            radius * pitch.cos() * yaw.cos(),
            radius * pitch.sin(),
            radius * pitch.cos() * yaw.sin(),
        );

        self.eye = self.target + new_offset;
    }

    /// Pan camera along viewport right and up axes.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let dist = (self.eye - self.target).length();
        let factor = (dist * 0.0015).max(0.001);

        let translation = -right * delta_x * factor + up * delta_y * factor;
        self.eye += translation;
        self.target += translation;
    }

    /// Zoom camera closer or further from target.
    /// Positive `factor` zooms in, negative zooms out.
    pub fn zoom(&mut self, factor: f32) {
        let offset = self.eye - self.target;
        let dist = offset.length();
        let zoom_mult = (1.0 - factor * 0.1).clamp(0.1, 10.0);
        let new_dist = (dist * zoom_mult).clamp(0.5, 500.0);

        self.eye = self.target + offset.normalize_or_zero() * new_dist;
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

        // Unproject near point (NDC z = 0.0 in wgpu) and far point (NDC z = 1.0)
        let near_ndc = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_ndc = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_world_h = inv_view_proj * near_ndc;
        let far_world_h = inv_view_proj * far_ndc;

        let near_world = near_world_h.truncate() / near_world_h.w;
        let far_world = far_world_h.truncate() / far_world_h.w;

        let direction = (far_world - near_world).normalize_or_zero();
        Ray::new(near_world, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_matrices_finite() {
        let cam = Camera::default();
        let vp = cam.view_proj_matrix();
        assert!(vp.is_finite());
    }

    #[test]
    fn test_screen_to_ray() {
        let cam = Camera::default();
        let ray = cam.screen_to_ray(Vec2::new(400.0, 300.0), Vec2::new(800.0, 600.0));
        assert!(ray.origin.is_finite());
        assert!(ray.direction.is_finite());
        assert!((ray.direction.length() - 1.0).abs() < 1e-4);
    }
}
