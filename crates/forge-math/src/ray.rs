use glam::Vec3;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::bounds::Aabb;

/// A 3D Ray with origin and normalized direction.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    /// Creates a new ray with normalized direction.
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    /// Computes the point along the ray at distance `t`: `origin + direction * t`.
    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Intersects ray with a plane defined by a point and a normal.
    /// Returns the distance `t` along the ray if hit.
    pub fn intersect_plane(&self, plane_point: Vec3, plane_normal: Vec3) -> Option<f32> {
        let denom = plane_normal.dot(self.direction);
        if denom.abs() > 1e-6 {
            let t = (plane_point - self.origin).dot(plane_normal) / denom;
            if t >= 0.0 {
                return Some(t);
            }
        }
        None
    }

    /// Intersects ray with a sphere. Returns the closest positive distance `t`.
    pub fn intersect_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let b = oc.dot(self.direction);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_disc = discriminant.sqrt();
        let t1 = -b - sqrt_disc;
        let t2 = -b + sqrt_disc;

        if t1 >= 0.0 {
            Some(t1)
        } else if t2 >= 0.0 {
            Some(t2)
        } else {
            None
        }
    }

    /// Intersects ray with an AABB.
    pub fn intersect_aabb(&self, aabb: &Aabb) -> Option<f32> {
        aabb.intersect_ray(self)
    }

    /// Intersects ray with a triangle using the Möller–Trumbore algorithm.
    /// Returns `Some((t, barycentric_coords))` where `t >= 0.0`.
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<(f32, Vec3)> {
        const EPSILON: f32 = 1e-7;
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < EPSILON {
            return None; // Ray is parallel to the triangle
        }

        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t > EPSILON {
            let w = 1.0 - u - v;
            Some((t, Vec3::new(u, v, w)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_plane_intersection() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = ray.intersect_plane(Vec3::ZERO, Vec3::Y);
        assert_eq!(hit, Some(5.0));
        assert_eq!(ray.point_at(5.0), Vec3::ZERO);
    }

    #[test]
    fn test_ray_sphere_intersection() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let hit = ray.intersect_sphere(Vec3::ZERO, 1.0);
        assert_eq!(hit, Some(4.0));
    }

    #[test]
    fn test_ray_triangle_intersection() {
        let ray = Ray::new(Vec3::new(0.2, 0.2, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        let hit = ray.intersect_triangle(v0, v1, v2);
        assert!(hit.is_some());
        let (t, _) = hit.unwrap();
        assert!((t - 5.0).abs() < 1e-4);
    }
}
