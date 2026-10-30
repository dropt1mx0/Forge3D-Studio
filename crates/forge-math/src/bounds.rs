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
    pub fn intersect_ray(&self, ray: &Ray) -> Option<f32> {
        let inv_dir = Vec3::new(
            1.0 / ray.direction.x,
            1.0 / ray.direction.y,
            1.0 / ray.direction.z,
        );

        let t1 = (self.min.x - ray.origin.x) * inv_dir.x;
        let t2 = (self.max.x - ray.origin.x) * inv_dir.x;
        let t3 = (self.min.y - ray.origin.y) * inv_dir.y;
        let t4 = (self.max.y - ray.origin.y) * inv_dir.y;
        let t5 = (self.min.z - ray.origin.z) * inv_dir.z;
        let t6 = (self.max.z - ray.origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
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
}
