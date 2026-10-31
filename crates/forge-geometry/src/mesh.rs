use forge_core::error::Result;
use forge_math::{Aabb, Ray, Transform, Vec3};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::validation::validate_mesh_data;
use crate::vertex::Vertex;

/// Pure-CPU mesh data representation containing vertex and index buffers.
///
/// Contains NO GPU handles or UI dependencies.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub aabb: Aabb,
}

impl Mesh {
    /// Creates a new mesh, automatically calculating its bounding box.
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        let mut mesh = Self {
            vertices,
            indices,
            aabb: Aabb::EMPTY,
        };
        mesh.recalculate_aabb();
        mesh
    }

    /// Recalculates the axis-aligned bounding box from vertex positions.
    pub fn recalculate_aabb(&mut self) {
        let mut aabb = Aabb::EMPTY;
        for v in &self.vertices {
            aabb.encapsulate(Vec3::from_array(v.position));
        }
        self.aabb = aabb;
    }

    /// Recalculates smooth vertex normals by accumulating triangle face normals.
    pub fn recalculate_normals(&mut self) {
        if self.indices.is_empty() {
            return;
        }

        // Zero out normals
        for v in &mut self.vertices {
            v.normal = [0.0, 0.0, 0.0];
        }

        // Accumulate face normals
        for chunk in self.indices.as_chunks::<3>().0 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 >= self.vertices.len() || i1 >= self.vertices.len() || i2 >= self.vertices.len() {
                continue;
            }

            let p0 = Vec3::from_array(self.vertices[i0].position);
            let p1 = Vec3::from_array(self.vertices[i1].position);
            let p2 = Vec3::from_array(self.vertices[i2].position);

            let face_normal = (p1 - p0).cross(p2 - p0);

            let n0 = Vec3::from_array(self.vertices[i0].normal) + face_normal;
            let n1 = Vec3::from_array(self.vertices[i1].normal) + face_normal;
            let n2 = Vec3::from_array(self.vertices[i2].normal) + face_normal;

            self.vertices[i0].normal = *n0.as_ref();
            self.vertices[i1].normal = *n1.as_ref();
            self.vertices[i2].normal = *n2.as_ref();
        }

        // Normalize
        for v in &mut self.vertices {
            let norm = Vec3::from_array(v.normal).normalize_or_zero();
            v.normal = norm.to_array();
        }
    }

    /// Validates mesh topology and numerical integrity.
    pub fn validate(&self) -> Result<()> {
        validate_mesh_data(&self.vertices, &self.indices)
    }

    /// Applies a 3D transform to all vertices, updating positions, normals, and bounding box.
    pub fn transform(&mut self, transform: &Transform) {
        let mat = transform.to_matrix();
        let norm_mat = mat.inverse().transpose();

        for v in &mut self.vertices {
            let p = Vec3::from_array(v.position);
            let n = Vec3::from_array(v.normal);

            v.position = mat.transform_point3(p).to_array();
            v.normal = norm_mat.transform_vector3(n).normalize_or_zero().to_array();
        }

        self.recalculate_aabb();
    }

    /// Tests ray intersection in world space given the object's world transform.
    /// Returns the closest hit distance `t` along the ray in world units.
    pub fn raycast(&self, ray: &Ray, world_transform: &Transform) -> Option<f32> {
        // Quick broad-phase test with world AABB
        let world_aabb = self.aabb.transform(world_transform);
        world_aabb.intersect_ray(ray)?;

        // Narrow-phase triangle testing: transform ray into local model space
        let inv_transform = world_transform.inverse();
        let local_origin = inv_transform.transform_point(ray.origin);
        let local_dir = inv_transform
            .transform_vector(ray.direction)
            .normalize_or_zero();
        let local_ray = Ray::new(local_origin, local_dir);

        let mut closest_local_t = f32::INFINITY;

        for chunk in self.indices.as_chunks::<3>().0 {
            let (i0, i1, i2) = (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);
            if i0 >= self.vertices.len() || i1 >= self.vertices.len() || i2 >= self.vertices.len() {
                continue;
            }

            let p0 = Vec3::from_array(self.vertices[i0].position);
            let p1 = Vec3::from_array(self.vertices[i1].position);
            let p2 = Vec3::from_array(self.vertices[i2].position);

            if let Some((t, _bary)) = local_ray.intersect_triangle(p0, p1, p2)
                && t > 0.0
                && t < closest_local_t
            {
                closest_local_t = t;
            }
        }

        if closest_local_t.is_finite() {
            // Compute hit point in world space and find distance along world ray
            let local_hit = local_ray.point_at(closest_local_t);
            let world_hit = world_transform.transform_point(local_hit);
            let world_dist = (world_hit - ray.origin).dot(ray.direction);
            if world_dist > 0.0 {
                Some(world_dist)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{generate_cube, generate_plane};

    #[test]
    fn test_mesh_raycast_cube() {
        let cube = generate_cube(1.0);
        let tf = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = cube.raycast(&ray, &tf);
        assert!(hit.is_some());
        let dist = hit.unwrap();
        // Cube extends from -0.5 to 0.5 along Z, so ray starting at 5.0 hits at distance 4.5
        assert!((dist - 4.5).abs() < 1e-4);

        // Ray behind the cube looking away
        let miss_ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(cube.raycast(&miss_ray, &tf).is_none());
    }

    #[test]
    fn test_mesh_raycast_flat_plane() {
        let plane = generate_plane(10.0, 1);
        let tf = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = plane.raycast(&ray, &tf);
        assert!(hit.is_some(), "Raycast against flat plane must hit");
        let dist = hit.unwrap();
        assert!((dist - 5.0).abs() < 0.01);
    }
}
