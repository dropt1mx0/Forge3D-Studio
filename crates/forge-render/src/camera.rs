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

/// A comprehensive 3D camera supporting Orbit, Pan, Zoom, Dolly, Presets, and Orthographic/Perspective projection.
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

    /// Pan camera along viewport right and up axes.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let dist = (self.eye - self.target).length();
        let factor = match self.projection {
            CameraProjection::Perspective => (dist * 0.0015).max(0.001),
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
                let zoom_mult = (1.0 - factor * 0.1).clamp(0.05, 5.0);
                let new_dist = (dist * zoom_mult).clamp(0.1, 2000.0);
                self.eye = self.target + offset.normalize_or_zero() * new_dist;
            }
            CameraProjection::Orthographic => {
                let zoom_mult = (1.0 - factor * 0.1).clamp(0.05, 5.0);
                self.ortho_size = (self.ortho_size * zoom_mult).clamp(0.05, 5000.0);
            }
        }
    }

    /// Dolly camera forward or backward along its look vector.
    pub fn dolly(&mut self, delta: f32) {
        let forward = (self.target - self.eye).normalize_or_zero();
        self.eye += forward * delta;
        self.target += forward * delta;
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
}
