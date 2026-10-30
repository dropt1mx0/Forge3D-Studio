use std::collections::HashMap;

use forge_core::id::ObjectId;
use forge_geometry::primitives::{generate_cube, generate_plane, generate_uv_sphere};
use forge_math::{Aabb, Ray, Transform, Vec3};
use serde::{Deserialize, Serialize};

use crate::object::{PrimitiveType, SceneObject};

/// The core scene containing all objects, their hierarchy, and dirty state tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub objects: HashMap<ObjectId, SceneObject>,
    pub root_objects: Vec<ObjectId>,
    #[serde(skip)]
    pub dirty: bool,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new("Untitled Scene")
    }
}

impl Scene {
    /// Creates a new empty scene with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            objects: HashMap::new(),
            root_objects: Vec::new(),
            dirty: true,
        }
    }

    /// Adds a new object to the scene and returns its `ObjectId`.
    pub fn add_object(&mut self, object: SceneObject) -> ObjectId {
        let id = object.id;
        if object.parent.is_none() && !self.root_objects.contains(&id) {
            self.root_objects.push(id);
        }
        self.objects.insert(id, object);
        self.dirty = true;
        id
    }

    /// Removes an object from the scene by ID.
    pub fn remove_object(&mut self, id: ObjectId) -> Option<SceneObject> {
        self.root_objects.retain(|&root_id| root_id != id);

        // Also detach from parent if any
        if let Some(obj) = self.objects.get(&id)
            && let Some(parent_id) = obj.parent
            && let Some(parent) = self.objects.get_mut(&parent_id)
        {
            parent.children.retain(|&c| c != id);
        }

        self.dirty = true;
        self.objects.remove(&id)
    }

    /// Returns a reference to an object by ID.
    pub fn get_object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.get(&id)
    }

    /// Returns a mutable reference to an object by ID, marking the scene dirty.
    pub fn get_object_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.dirty = true;
        self.objects.get_mut(&id)
    }

    /// Computes the world-space transform for an object by traversing up to the root.
    pub fn world_transform(&self, id: ObjectId) -> Transform {
        let mut current_id = Some(id);
        let mut transforms = Vec::new();

        // Collect transforms upwards to root
        while let Some(obj_id) = current_id {
            if let Some(obj) = self.objects.get(&obj_id) {
                transforms.push(obj.local_transform);
                current_id = obj.parent;
            } else {
                break;
            }
        }

        // Multiply downwards from root to child: (root * ... * child)
        let mut world = Transform::IDENTITY;
        for t in transforms.into_iter().rev() {
            world = world.mul_transform(&t);
        }
        world
    }

    /// Computes the world AABB for an object.
    pub fn world_aabb(&self, id: ObjectId) -> Option<Aabb> {
        let obj = self.objects.get(&id)?;
        let world_tf = self.world_transform(id);
        Some(obj.mesh.aabb.transform(&world_tf))
    }

    /// Performs precise raycasting against all visible objects in the scene.
    /// Returns the closest hit `(ObjectId, hit_distance)`.
    pub fn raycast(&self, ray: &Ray) -> Option<(ObjectId, f32)> {
        let mut closest_hit: Option<(ObjectId, f32)> = None;

        for (&id, obj) in &self.objects {
            if !obj.visible {
                continue;
            }

            let world_tf = self.world_transform(id);
            if let Some(dist) = obj.mesh.raycast(ray, &world_tf)
                && dist > 0.0
            {
                match closest_hit {
                    Some((_, closest_dist)) if dist < closest_dist => {
                        closest_hit = Some((id, dist));
                    }
                    None => {
                        closest_hit = Some((id, dist));
                    }
                    _ => {}
                }
            }
        }

        closest_hit
    }

    /// Duplicates an object in the scene, returning its new `ObjectId`.
    pub fn duplicate_object(&mut self, id: ObjectId) -> Option<ObjectId> {
        let original = self.objects.get(&id)?.clone();
        let mut duplicate = original;
        duplicate.id = ObjectId::new();
        duplicate.name = format!("{} Copy", duplicate.name);
        duplicate.local_transform.translation += Vec3::new(1.0, 0.0, 0.0);
        duplicate.parent = None;
        duplicate.children.clear();

        let new_id = self.add_object(duplicate);
        Some(new_id)
    }

    /// Computes the collective world-space bounding box of all visible objects in the scene.
    pub fn scene_aabb(&self) -> Aabb {
        let mut combined = Aabb::EMPTY;
        for (&id, obj) in &self.objects {
            if obj.visible && !obj.mesh.aabb.is_empty() {
                let world_tf = self.world_transform(id);
                combined = combined.union(&obj.mesh.aabb.transform(&world_tf));
            }
        }
        if combined.is_empty() {
            Aabb::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::new(2.0, 2.0, 2.0))
        } else {
            combined
        }
    }

    /// Computes the collective world-space bounding box of currently selected objects.
    pub fn selected_aabb(&self, selection: &crate::selection::SelectionState) -> Aabb {
        let mut combined = Aabb::EMPTY;
        for &id in &selection.selected_ids {
            if let Some(obj) = self.objects.get(&id)
                && !obj.mesh.aabb.is_empty()
            {
                let world_tf = self.world_transform(id);
                combined = combined.union(&obj.mesh.aabb.transform(&world_tf));
            }
        }
        if combined.is_empty() {
            self.scene_aabb()
        } else {
            combined
        }
    }

    /// Applies the local transform of an object permanently into its mesh.
    pub fn apply_transform_to_object(&mut self, id: ObjectId) {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.apply_transform();
            self.dirty = true;
        }
    }

    /// Resets transform for an object.
    pub fn reset_transform_for_object(&mut self, id: ObjectId) {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.reset_transform();
            self.dirty = true;
        }
    }

    /// Clears the scene.
    pub fn clear(&mut self) {
        self.objects.clear();
        self.root_objects.clear();
        self.dirty = true;
    }

    /// Creates a default scene containing a Cube, Sphere, and Ground Plane as required by Alpha 0.1.
    pub fn create_default_scene() -> Self {
        let mut scene = Self::new("Default Scene");

        // 1. Ground Plane (10x10)
        let plane_mesh = generate_plane(10.0, 10);
        let plane = SceneObject::new("Ground Plane", PrimitiveType::Plane, plane_mesh)
            .with_color([0.28, 0.30, 0.34, 1.0])
            .with_transform(Transform::from_translation(Vec3::ZERO));
        scene.add_object(plane);

        // 2. Center Cube (1x1x1, resting at y=0.5)
        let cube_mesh = generate_cube(1.0);
        let cube = SceneObject::new("Cube", PrimitiveType::Cube, cube_mesh)
            .with_color([0.85, 0.45, 0.25, 1.0]) // Warm terracotta / orange
            .with_transform(Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)));
        scene.add_object(cube);

        // 3. Offset UV Sphere (radius 0.65, resting at x=-2.0, y=0.65)
        let sphere_mesh = generate_uv_sphere(0.65, 32, 16);
        let sphere = SceneObject::new("Sphere", PrimitiveType::Sphere, sphere_mesh)
            .with_color([0.25, 0.65, 0.85, 1.0]) // Cyan / azure
            .with_transform(Transform::from_translation(Vec3::new(-2.0, 0.65, 0.0)));
        scene.add_object(sphere);

        scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scene_objects() {
        let scene = Scene::create_default_scene();
        assert_eq!(scene.objects.len(), 3);
        assert_eq!(scene.root_objects.len(), 3);
    }

    #[test]
    fn test_scene_raycast() {
        let scene = Scene::create_default_scene();
        // Cast ray towards the cube at (0, 0.5, 0)
        let ray = Ray::new(Vec3::new(0.0, 0.5, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = scene.raycast(&ray);
        assert!(hit.is_some());
    }
}
