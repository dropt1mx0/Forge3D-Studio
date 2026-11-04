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
    pub active_camera_id: Option<ObjectId>,
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
            active_camera_id: None,
            dirty: true,
        }
    }

    /// Adds a new object to the scene and returns its `ObjectId`.
    pub fn add_object(&mut self, object: SceneObject) -> ObjectId {
        let id = object.id;
        let is_camera = object.primitive_type == PrimitiveType::Camera;
        if object.parent.is_none() && !self.root_objects.contains(&id) {
            self.root_objects.push(id);
        }
        self.objects.insert(id, object);
        if is_camera && self.active_camera_id.is_none() {
            self.active_camera_id = Some(id);
        }
        self.dirty = true;
        id
    }

    /// Sets the active scene camera.
    pub fn set_active_camera(&mut self, id: ObjectId) {
        if let Some(obj) = self.objects.get(&id)
            && obj.primitive_type == PrimitiveType::Camera
        {
            self.active_camera_id = Some(id);
            self.dirty = true;
        }
    }

    /// Returns the active camera object and its camera data if present.
    pub fn active_camera(&self) -> Option<(&SceneObject, &crate::object::CameraData)> {
        let cam_id = self.active_camera_id?;
        let obj = self.objects.get(&cam_id)?;
        let data = obj.camera_data.as_ref()?;
        Some((obj, data))
    }

    /// Returns list of all camera object IDs in the scene.
    pub fn camera_ids(&self) -> Vec<ObjectId> {
        self.objects
            .iter()
            .filter(|(_, obj)| obj.primitive_type == PrimitiveType::Camera)
            .map(|(&id, _)| id)
            .collect()
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

        if self.active_camera_id == Some(id) {
            self.active_camera_id = self
                .objects
                .iter()
                .find(|(oid, obj)| **oid != id && obj.primitive_type == PrimitiveType::Camera)
                .map(|(oid, _)| *oid);
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

    /// Rebuilds root_objects and parent/child lists to ensure strict hierarchy consistency.
    pub fn rebuild_hierarchy(&mut self) {
        self.root_objects.clear();
        for obj in self.objects.values_mut() {
            obj.children.clear();
        }

        let ids: Vec<ObjectId> = self.objects.keys().copied().collect();
        for id in ids {
            let parent_id = self.objects.get(&id).and_then(|o| o.parent);
            if let Some(pid) = parent_id {
                if let Some(parent) = self.objects.get_mut(&pid) {
                    if !parent.children.contains(&id) {
                        parent.children.push(id);
                    }
                } else if let Some(child) = self.objects.get_mut(&id) {
                    child.parent = None;
                    self.root_objects.push(id);
                }
            } else {
                self.root_objects.push(id);
            }
        }
        self.dirty = true;
    }

    /// Sets or clears the parent of an object while preserving its world-space transform.
    pub fn parent_object(
        &mut self,
        child_id: ObjectId,
        new_parent_id: Option<ObjectId>,
    ) -> Option<crate::history::Action> {
        if Some(child_id) == new_parent_id {
            return None;
        }

        // Cycle check: verify new_parent_id is not a descendant of child_id
        if let Some(mut ancestor) = new_parent_id {
            while let Some(parent_obj) = self.objects.get(&ancestor) {
                if ancestor == child_id {
                    return None; // Cycle detected!
                }
                if let Some(next_p) = parent_obj.parent {
                    ancestor = next_p;
                } else {
                    break;
                }
            }
        }

        let child_world = self.world_transform(child_id);
        let child = self.objects.get(&child_id)?;
        let old_parent = child.parent;
        let old_local_tf = child.local_transform;

        let new_local_tf = if let Some(pid) = new_parent_id {
            let parent_world = self.world_transform(pid);
            parent_world.inverse().mul_transform(&child_world)
        } else {
            child_world
        };

        if let Some(c) = self.objects.get_mut(&child_id) {
            c.parent = new_parent_id;
            c.local_transform = new_local_tf;
        }

        self.rebuild_hierarchy();

        Some(crate::history::Action::ParentChange {
            child_id,
            old_parent,
            new_parent: new_parent_id,
            old_local_tf,
            new_local_tf,
        })
    }

    /// Merges multiple secondary mesh objects into the active object and deletes the secondary objects.
    pub fn join_objects(
        &mut self,
        active_id: ObjectId,
        secondary_ids: &[ObjectId],
    ) -> Option<crate::history::Action> {
        let active_world = self.world_transform(active_id);
        let inv_active_world = active_world.inverse();
        let old_mesh = self.objects.get(&active_id)?.mesh.clone();

        let mut removed_objects = Vec::new();
        let mut appended_vertices = Vec::new();
        let mut appended_indices = Vec::new();

        let mut current_vertex_base = old_mesh.vertices.len() as u32;

        for &sec_id in secondary_ids {
            if sec_id == active_id {
                continue;
            }

            let sec_world = self.world_transform(sec_id);
            if let Some(sec_obj) = self.remove_object(sec_id) {
                let rel_tf = inv_active_world.mul_transform(&sec_world);
                for v in &sec_obj.mesh.vertices {
                    let world_pos = rel_tf.transform_point(Vec3::from(v.position));
                    let world_norm = rel_tf
                        .transform_vector(Vec3::from(v.normal))
                        .normalize_or_zero();
                    let mut new_v = *v;
                    new_v.position = world_pos.to_array();
                    new_v.normal = world_norm.to_array();
                    appended_vertices.push(new_v);
                }

                for &idx in &sec_obj.mesh.indices {
                    appended_indices.push(current_vertex_base + idx);
                }

                current_vertex_base += sec_obj.mesh.vertices.len() as u32;
                removed_objects.push(sec_obj);
            }
        }

        if removed_objects.is_empty() {
            return None;
        }

        if let Some(active_obj) = self.objects.get_mut(&active_id) {
            active_obj.mesh.vertices.extend(appended_vertices);
            active_obj.mesh.indices.extend(appended_indices);
            active_obj.mesh.recalculate_aabb();
            active_obj.invalidate_evaluated_mesh();
        }

        self.dirty = true;

        Some(crate::history::Action::join(
            active_id,
            old_mesh,
            removed_objects,
        ))
    }

    /// Centers the mesh geometry around the object's origin (0, 0, 0) and moves the object position to the center.
    pub fn set_origin_to_geometry(&mut self, id: ObjectId) -> Option<crate::history::Action> {
        let obj = self.objects.get_mut(&id)?;
        let center = obj.mesh.aabb.center();
        if center.length_squared() < 1e-6 {
            return None;
        }

        let old_transform = obj.local_transform;
        let old_mesh = obj.mesh.clone();

        for v in &mut obj.mesh.vertices {
            v.position[0] -= center.x;
            v.position[1] -= center.y;
            v.position[2] -= center.z;
        }
        obj.mesh.recalculate_aabb();
        obj.invalidate_evaluated_mesh();

        // Offset position by rotated and scaled center
        let world_offset = obj.local_transform.rotation * (center * obj.local_transform.scale);
        obj.local_transform.translation += world_offset;
        self.dirty = true;

        Some(crate::history::Action::set_origin(
            id,
            old_transform,
            old_mesh,
            obj.local_transform,
            obj.mesh.clone(),
        ))
    }

    /// Sets the object's origin to a specific point in world space.
    pub fn set_origin_to_cursor(
        &mut self,
        id: ObjectId,
        cursor_world: Vec3,
    ) -> Option<crate::history::Action> {
        let world_tf = self.world_transform(id);
        let local_cursor = world_tf.inverse().transform_point(cursor_world);
        if local_cursor.length_squared() < 1e-6 {
            return None;
        }

        let obj = self.objects.get_mut(&id)?;
        let old_transform = obj.local_transform;
        let old_mesh = obj.mesh.clone();

        for v in &mut obj.mesh.vertices {
            v.position[0] -= local_cursor.x;
            v.position[1] -= local_cursor.y;
            v.position[2] -= local_cursor.z;
        }
        obj.mesh.recalculate_aabb();
        obj.invalidate_evaluated_mesh();

        let world_offset =
            obj.local_transform.rotation * (local_cursor * obj.local_transform.scale);
        obj.local_transform.translation += world_offset;
        self.dirty = true;

        Some(crate::history::Action::set_origin(
            id,
            old_transform,
            old_mesh,
            obj.local_transform,
            obj.mesh.clone(),
        ))
    }

    /// Sets visibility for an object.
    pub fn set_object_visibility(&mut self, id: ObjectId, visible: bool) {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.visible = visible;
            self.dirty = true;
        }
    }

    /// Unhides all objects in the scene.
    pub fn unhide_all(&mut self) {
        for obj in self.objects.values_mut() {
            obj.visible = true;
        }
        self.dirty = true;
    }

    /// Aligns the active scene camera's position and orientation to match the given viewport camera.
    pub fn align_active_camera_to_view(&mut self, eye: Vec3, target: Vec3, up: Vec3) {
        if let Some(cam_id) = self.active_camera_id
            && let Some(obj) = self.objects.get_mut(&cam_id)
        {
            let view_mat = forge_math::Mat4::look_at_rh(eye, target, up);
            let inv_view = view_mat.inverse();
            let (_, rot, pos) = inv_view.to_scale_rotation_translation();
            obj.local_transform.translation = pos;
            obj.local_transform.rotation = rot;
            self.dirty = true;
        }
    }

    /// Computes the view and projection parameters for looking through a scene camera.
    pub fn camera_view_parameters(
        &self,
        cam_id: ObjectId,
    ) -> Option<crate::object::CameraViewParameters> {
        let obj = self.objects.get(&cam_id)?;
        if obj.primitive_type != PrimitiveType::Camera {
            return None;
        }

        let world_tf = self.world_transform(cam_id);
        let world_mat = world_tf.to_matrix();

        let eye = world_tf.translation;
        // Camera looks along its local -Z axis
        let forward = -world_mat.transform_vector3(Vec3::Z).normalize_or_zero();
        let up = world_mat.transform_vector3(Vec3::Y).normalize_or_zero();
        let dist = 10.0;
        let target = eye + forward * dist;

        let data = obj.camera_data.as_ref();
        let fov_y_deg = data.map(|d| d.fov_y_deg).unwrap_or(50.0);
        let clip_start = data.map(|d| d.clip_start).unwrap_or(0.1);
        let clip_end = data.map(|d| d.clip_end).unwrap_or(1000.0);
        let ortho_scale = data.map(|d| d.ortho_scale).unwrap_or(6.0);
        let is_orthographic = data
            .map(|d| d.projection == crate::object::CameraProjectionType::Orthographic)
            .unwrap_or(false);

        Some(crate::object::CameraViewParameters {
            eye,
            target,
            up,
            fov_y_deg,
            clip_start,
            clip_end,
            ortho_scale,
            is_orthographic,
        })
    }

    /// Clears the scene.
    pub fn clear(&mut self) {
        self.objects.clear();
        self.root_objects.clear();
        self.active_camera_id = None;
        self.dirty = true;
    }

    /// Creates a default scene containing a Cube, Sphere, Ground Plane, and Scene Camera.
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

        // 4. Default Scene Camera (positioned at 5.5, 4.0, 6.0 looking at 0.0, 0.5, 0.0)
        let mut camera_obj = SceneObject::new_camera("Camera");
        let view_mat = forge_math::Mat4::look_at_rh(
            Vec3::new(5.5, 4.0, 6.0),
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::Y,
        );
        let inv_view = view_mat.inverse();
        let (_, rot, pos) = inv_view.to_scale_rotation_translation();
        camera_obj.local_transform.translation = pos;
        camera_obj.local_transform.rotation = rot;
        let cam_id = scene.add_object(camera_obj);
        scene.active_camera_id = Some(cam_id);

        scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scene_objects() {
        let scene = Scene::create_default_scene();
        assert_eq!(scene.objects.len(), 4);
        assert_eq!(scene.root_objects.len(), 4);
        assert!(scene.active_camera_id.is_some());
        assert_eq!(scene.camera_ids().len(), 1);
    }

    #[test]
    fn test_scene_raycast() {
        let scene = Scene::create_default_scene();
        // Cast ray towards the cube at (0, 0.5, 0)
        let ray = Ray::new(Vec3::new(0.0, 0.5, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = scene.raycast(&ray);
        assert!(hit.is_some());
    }

    #[test]
    fn test_scene_camera_management() {
        let mut scene = Scene::create_default_scene();
        let cam_id = scene.active_camera_id.expect("Default scene has a camera");
        assert!(scene.active_camera().is_some());

        // Align camera to view
        scene.align_active_camera_to_view(Vec3::new(10.0, 5.0, 10.0), Vec3::ZERO, Vec3::Y);
        let cam_obj = scene.get_object(cam_id).unwrap();
        assert!((cam_obj.local_transform.translation - Vec3::new(10.0, 5.0, 10.0)).length() < 1e-4);

        // Remove camera
        scene.remove_object(cam_id);
        assert!(scene.active_camera_id.is_none());
        assert_eq!(scene.camera_ids().len(), 0);
    }

    #[test]
    fn test_parent_and_unparent_objects() {
        let mut scene = Scene::create_default_scene();
        let cube_id = scene
            .objects
            .values()
            .find(|o| o.name == "Cube")
            .unwrap()
            .id;
        let sphere_id = scene
            .objects
            .values()
            .find(|o| o.name == "Sphere")
            .unwrap()
            .id;

        let sphere_world_before = scene.world_transform(sphere_id);

        // Parent sphere to cube
        let action = scene.parent_object(sphere_id, Some(cube_id));
        assert!(action.is_some());
        assert_eq!(scene.get_object(sphere_id).unwrap().parent, Some(cube_id));

        // World transform must be preserved after parenting
        let sphere_world_after = scene.world_transform(sphere_id);
        assert!((sphere_world_before.translation - sphere_world_after.translation).length() < 1e-4);

        // Unparent sphere
        let action_unparent = scene.parent_object(sphere_id, None);
        assert!(action_unparent.is_some());
        assert_eq!(scene.get_object(sphere_id).unwrap().parent, None);
    }

    #[test]
    fn test_join_objects() {
        let mut scene = Scene::create_default_scene();
        let cube_id = scene
            .objects
            .values()
            .find(|o| o.name == "Cube")
            .unwrap()
            .id;
        let sphere_id = scene
            .objects
            .values()
            .find(|o| o.name == "Sphere")
            .unwrap()
            .id;

        let cube_verts_before = scene.get_object(cube_id).unwrap().mesh.vertices.len();
        let sphere_verts = scene.get_object(sphere_id).unwrap().mesh.vertices.len();

        let action = scene.join_objects(cube_id, &[sphere_id]);
        assert!(action.is_some());

        // Cube mesh must contain both vertex sets
        let cube_verts_after = scene.get_object(cube_id).unwrap().mesh.vertices.len();
        assert_eq!(cube_verts_after, cube_verts_before + sphere_verts);
        // Sphere must be removed
        assert!(scene.get_object(sphere_id).is_none());
    }

    #[test]
    fn test_visibility_and_unhide_all() {
        let mut scene = Scene::create_default_scene();
        let cube_id = scene
            .objects
            .values()
            .find(|o| o.name == "Cube")
            .unwrap()
            .id;

        scene.set_object_visibility(cube_id, false);
        assert!(!scene.get_object(cube_id).unwrap().visible);

        scene.unhide_all();
        assert!(scene.get_object(cube_id).unwrap().visible);
    }
}
