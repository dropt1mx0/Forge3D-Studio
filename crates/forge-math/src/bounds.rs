use glam::Vec3;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ray::Ray;
use crate::transform::Transform;

/// Axis-Aligned Bounding Box (AABB) in 3D space.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb {
    /// Inverted empty bounding box for accumulation.
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    /// Constructs an AABB from explicit min and max corners.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Constructs an AABB containing a slice of 3D points.
    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = Self::EMPTY;
        for &p in points {
            aabb.encapsulate(p);
        }
        aabb
    }

    /// Constructs a centered AABB with given extents (half-sizes).
    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self {
            min: center - extents,
            max: center + extents,
        }
    }

    /// Extends the bounding box to enclose the given point.
    pub fn encapsulate(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Returns true if the bounding box has inverted bounds (empty).
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// Returns the center of the bounding box.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Returns the size (width, height, depth) of the bounding box.
    pub fn size(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    /// Returns the half-extents of the bounding box.
    pub fn extents(&self) -> Vec3 {
        self.size() * 0.5
    }

    /// Combines this bounding box with another.
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Transforms this AABB by a 3D transform, returning the new bounding box enclosing all 8 transformed corners.
    pub fn transform(&self, transform: &Transform) -> Aabb {
        if self.min.x > self.max.x {
            return *self;
        }

        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut transformed_aabb = Aabb::EMPTY;
        for corner in corners {
            transformed_aabb.encapsulate(transform.transform_point(corner));
        }
        transformed_aabb
    }

    /// Tests if a point is contained inside this AABB.
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Tests ray intersection using slab method. Returns distance along ray if intersecting.
    /// Robust against zero-thickness bounding boxes (such as planar meshes) and parallel rays.
    pub fn intersect_ray(&self, ray: &Ray) -> Option<f32> {
        let mut tmin = 0.0_f32;
        let mut tmax = f32::INFINITY;

        // Ensure flat bounding boxes (planes, thin sheets) have a small epsilon slab thickness
        // to prevent false rejections due to numerical precision.
        const EPSILON: f32 = 0.005;
        let min = self.min.min(self.max);
        let max = self.max.max(self.min);
        let center = (min + max) * 0.5;
        let half_size = ((max - min) * 0.5).max(Vec3::splat(EPSILON));
        let b_min = center - half_size;
        let b_max = center + half_size;

        for i in 0..3 {
            let origin = ray.origin[i];
            let dir = ray.direction[i];
            let min_val = b_min[i];
            let max_val = b_max[i];

            if dir.abs() < 1e-8 {
                // Ray is parallel to this axis slab. Check if origin is inside.
                if origin < min_val || origin > max_val {
                    return None;
                }
            } else {
                let inv_d = 1.0 / dir;
                let mut t1 = (min_val - origin) * inv_d;
                let mut t2 = (max_val - origin) * inv_d;
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                tmin = tmin.max(t1);
                tmax = tmax.min(t2);
                if tmin > tmax {
                    return None;
                }
            }
        }

        if tmax < 0.0 {
            return None;
        }

        Some(if tmin < 0.0 { tmax } else { tmin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_encapsulate() {
        let mut aabb = Aabb::EMPTY;
        aabb.encapsulate(Vec3::new(-1.0, 0.0, 2.0));
        aabb.encapsulate(Vec3::new(3.0, 5.0, -2.0));
        assert_eq!(aabb.min, Vec3::new(-1.0, 0.0, -2.0));
        assert_eq!(aabb.max, Vec3::new(3.0, 5.0, 2.0));
        assert_eq!(aabb.center(), Vec3::new(1.0, 2.5, 0.0));
    }

    #[test]
    fn test_aabb_ray_intersection_box() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = aabb.intersect_ray(&ray);
        assert!(hit.is_some());
        let dist = hit.unwrap();
        assert!((dist - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_aabb_ray_intersection_flat_plane() {
        // Flat horizontal plane: zero extent along Y axis
        let plane_aabb = Aabb::new(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 0.0, 5.0));
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = plane_aabb.intersect_ray(&ray);
        assert!(
            hit.is_some(),
            "Ray directly targeting flat plane must intersect"
        );
        let dist = hit.unwrap();
        assert!((dist - 10.0).abs() < 0.02);

        // Angled ray
        let angled_ray = Ray::new(
            Vec3::new(0.0, 5.0, 5.0),
            Vec3::new(0.0, -1.0, -1.0).normalize(),
        );
        assert!(plane_aabb.intersect_ray(&angled_ray).is_some());

        // Ray missing the plane
        let miss_ray = Ray::new(Vec3::new(15.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        assert!(plane_aabb.intersect_ray(&miss_ray).is_none());
    }

    #[test]
    fn test_aabb_ray_parallel_miss_and_hit() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        // Ray perfectly parallel to X and Y, shooting along Z from inside X/Y range
        let hit_ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(aabb.intersect_ray(&hit_ray).is_some());

        // Ray perfectly parallel to Z but outside X range
        let miss_ray = Ray::new(Vec3::new(2.5, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(aabb.intersect_ray(&miss_ray).is_none());
    }
}
