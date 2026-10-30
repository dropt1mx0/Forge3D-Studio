use glam::{EulerRot, Mat4, Quat, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::conventions::{vec3_deg_to_rad, vec3_rad_to_deg};

/// 3D affine transform represented by translation, rotation (quaternion), and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    /// Identity transform (at origin, no rotation, uniform unit scale).
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Creates a transform with translation only.
    pub const fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform with rotation only.
    pub const fn from_rotation(rotation: Quat) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform with uniform or non-uniform scale only.
    pub const fn from_scale(scale: Vec3) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
        }
    }

    /// Creates a transform from translation, rotation, and scale components.
    pub const fn from_translation_rotation_scale(
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Converts this transform into a 4x4 affine transformation matrix.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Creates a transform from a 4x4 matrix (decomposing into scale, rotation, translation).
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Combines this parent transform with a child transform (`parent * child`).
    pub fn mul_transform(&self, child: &Transform) -> Transform {
        let parent_mat = self.to_matrix();
        let child_mat = child.to_matrix();
        Self::from_matrix(parent_mat * child_mat)
    }

    /// Computes the inverse transform.
    pub fn inverse(&self) -> Self {
        Self::from_matrix(self.to_matrix().inverse())
    }

    /// Transforms a 3D position point (applies scale, rotation, and translation).
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.to_matrix().transform_point3(point)
    }

    /// Transforms a 3D direction vector (applies scale and rotation only, no translation).
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        self.to_matrix().transform_vector3(vector)
    }

    /// Returns Euler angles in degrees (YXZ order: Yaw, Pitch, Roll).
    pub fn euler_angles_deg(&self) -> Vec3 {
        let (y, x, z) = self.rotation.to_euler(EulerRot::YXZ);
        vec3_rad_to_deg(Vec3::new(x, y, z))
    }

    /// Sets the rotation from Euler angles in degrees (Pitch X, Yaw Y, Roll Z).
    pub fn set_euler_angles_deg(&mut self, angles_deg: Vec3) {
        let rad = vec3_deg_to_rad(angles_deg);
        self.rotation = Quat::from_euler(EulerRot::YXZ, rad.y, rad.x, rad.z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_identity() {
        let t = Transform::IDENTITY;
        let pt = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(t.transform_point(pt), pt);
    }

    #[test]
    fn test_transform_translation_and_scale() {
        let t = Transform::from_translation_rotation_scale(
            Vec3::new(10.0, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::splat(2.0),
        );
        let pt = Vec3::new(1.0, 1.0, 1.0);
        let res = t.transform_point(pt);
        assert_eq!(res, Vec3::new(12.0, 2.0, 2.0));
    }

    #[test]
    fn test_transform_inverse() {
        let t = Transform::from_translation_rotation_scale(
            Vec3::new(5.0, -3.0, 2.0),
            Quat::from_rotation_y(0.5),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let inv = t.inverse();
        let pt = Vec3::new(1.0, 2.0, 3.0);
        let roundtrip = inv.transform_point(t.transform_point(pt));
        assert!((roundtrip - pt).length() < 1e-4);
    }
}
