use std::collections::HashMap;

use forge_core::id::ObjectId;
use forge_geometry::primitives::{
    generate_cube, generate_cylinder, generate_icosphere, generate_plane, generate_uv_sphere,
};
use forge_geometry::{Mesh, Vertex};
use forge_math::{Aabb, Quat, Ray, Transform, Vec3};
use serde::{Deserialize, Serialize};

use crate::material::{LightData, LightKind, PbrMaterial, WorldLighting};
use crate::object::{PrimitiveType, SceneObject};

/// Material asset ID for Poly Haven's CC0 brown floor tile set.
pub const BROWN_FLOOR_TILES_MATERIAL_ID: &str = "polyhaven/brown_floor_tiles";

/// The core scene containing all objects, their hierarchy, and dirty state tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub objects: HashMap<ObjectId, SceneObject>,
    #[serde(default)]
    pub materials: HashMap<String, PbrMaterial>,
    #[serde(default)]
    pub world_lighting: WorldLighting,
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
            materials: HashMap::new(),
            world_lighting: WorldLighting::default(),
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

    /// Creates a colorful, camera-ready sample scene with a house, tree, rock, and yard.
    ///
    /// This remains separate from `create_default_scene()` so the standard primitive
    /// workspace and its editing workflows keep their existing behavior.
    pub fn create_showcase_scene() -> Self {
        let mut scene = Self::new("House, Tree & Rock Showcase");

        scene.materials.insert(
            BROWN_FLOOR_TILES_MATERIAL_ID.into(),
            PbrMaterial::brown_floor_tiles(),
        );
        scene.materials.insert(
            "polyhaven/bark_brown_01".into(),
            PbrMaterial::bark_brown_01(),
        );
        scene.materials.insert(
            "polyhaven/wooden_planks".into(),
            PbrMaterial::wooden_planks(),
        );
        scene
            .materials
            .insert("polyhaven/mossy_rock".into(), PbrMaterial::mossy_rock());

        // The sun and fill are scene objects, so their transforms and values are editable.
        let mut sun = SceneObject::new_light(
            "Sun Light",
            LightData {
                kind: LightKind::Directional,
                color: [1.0, 0.91, 0.78],
                intensity: 2.7,
                range: 100.0,
                ..LightData::default()
            },
            generate_uv_sphere(0.12, 10, 6),
        );
        sun.visible = false;
        sun.local_transform = Transform::from_translation_rotation_scale(
            Vec3::new(-4.0, 7.0, 5.0),
            Quat::from_rotation_x(0.72) * Quat::from_rotation_y(-0.55),
            Vec3::ONE,
        );
        scene.add_object(sun);

        let mut fill = SceneObject::new_light(
            "Porch Fill Light",
            LightData {
                kind: LightKind::Point,
                color: [0.68, 0.79, 1.0],
                intensity: 18.0,
                range: 9.0,
                ..LightData::default()
            },
            generate_uv_sphere(0.1, 8, 6),
        );
        fill.visible = false;
        fill.local_transform.translation = Vec3::new(0.0, 2.2, 2.1);
        scene.add_object(fill);

        add_showcase_object(
            &mut scene,
            "Grassy Ground",
            PrimitiveType::Plane,
            generate_plane(10.0, 1),
            Transform::IDENTITY,
            [0.25, 0.36, 0.23, 1.0],
        );

        // A short stone path makes the front entrance read clearly in the viewport.
        add_showcase_object_with_material(
            &mut scene,
            "Front Path",
            PrimitiveType::Cube,
            generate_material_cube(Vec3::new(0.72, 0.07, 1.35)),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.0, 0.035, 1.72),
                Quat::IDENTITY,
                Vec3::new(0.72, 0.07, 1.35),
            ),
            [1.0, 1.0, 1.0, 1.0],
            Some(BROWN_FLOOR_TILES_MATERIAL_ID),
        );

        // House body, plinth, and pitched two-piece roof.
        add_showcase_object(
            &mut scene,
            "House Foundation",
            PrimitiveType::Cube,
            generate_cube(1.0),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.0, 0.09, 0.0),
                Quat::IDENTITY,
                Vec3::new(2.42, 0.18, 2.02),
            ),
            [0.34, 0.35, 0.36, 1.0],
        );
        add_showcase_object(
            &mut scene,
            "House Walls",
            PrimitiveType::Cube,
            generate_cube(1.0),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.0, 0.86, 0.0),
                Quat::IDENTITY,
                Vec3::new(2.2, 1.62, 1.8),
            ),
            [0.76, 0.53, 0.33, 1.0],
        );
        let roof_angle = 35.0_f32.to_radians();
        for (name, x, angle) in [
            ("Roof Left", -0.63, roof_angle),
            ("Roof Right", 0.63, -roof_angle),
        ] {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Cube,
                generate_cube(1.0),
                Transform::from_translation_rotation_scale(
                    Vec3::new(x, 2.08, 0.0),
                    Quat::from_rotation_z(angle),
                    Vec3::new(1.55, 0.18, 2.12),
                ),
                [0.43, 0.16, 0.13, 1.0],
            );
        }
        add_showcase_object(
            &mut scene,
            "Chimney",
            PrimitiveType::Cube,
            generate_cube(1.0),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.58, 2.49, -0.34),
                Quat::IDENTITY,
                Vec3::new(0.38, 0.72, 0.42),
            ),
            [0.50, 0.28, 0.22, 1.0],
        );
        add_showcase_object(
            &mut scene,
            "Chimney Cap",
            PrimitiveType::Cube,
            generate_cube(1.0),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.58, 2.88, -0.34),
                Quat::IDENTITY,
                Vec3::new(0.48, 0.12, 0.52),
            ),
            [0.38, 0.19, 0.16, 1.0],
        );

        // Dark ridge and front/back fascia make the roof silhouette read at a distance.
        add_showcase_object(
            &mut scene,
            "Roof Ridge Cap",
            PrimitiveType::Cube,
            generate_cube(1.0),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.0, 2.54, 0.0),
                Quat::IDENTITY,
                Vec3::new(0.16, 0.13, 2.28),
            ),
            [0.31, 0.10, 0.09, 1.0],
        );
        for (name, z) in [("Roof Front Fascia", 1.02), ("Roof Rear Fascia", -1.02)] {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Cube,
                generate_cube(1.0),
                Transform::from_translation_rotation_scale(
                    Vec3::new(0.0, 1.61, z),
                    Quat::IDENTITY,
                    Vec3::new(2.30, 0.12, 0.10),
                ),
                [0.34, 0.12, 0.10, 1.0],
            );
        }

        add_showcase_object(
            &mut scene,
            "Front Door",
            PrimitiveType::Cube,
            generate_cube(1.0),
            Transform::from_translation_rotation_scale(
                Vec3::new(0.0, 0.69, 0.95),
                Quat::IDENTITY,
                Vec3::new(0.58, 1.14, 0.10),
            ),
            [0.27, 0.15, 0.09, 1.0],
        );
        add_showcase_object(
            &mut scene,
            "Door Handle",
            PrimitiveType::Sphere,
            generate_uv_sphere(0.065, 16, 8),
            Transform::from_translation(Vec3::new(0.18, 0.72, 1.015)),
            [0.88, 0.67, 0.25, 1.0],
        );

        // Two blue window panes with simple cream-colored frames on the front wall.
        for (side, x) in [("Left", -0.69), ("Right", 0.69)] {
            add_showcase_object(
                &mut scene,
                format!("{side} Window Glass"),
                PrimitiveType::Cube,
                generate_cube(1.0),
                Transform::from_translation_rotation_scale(
                    Vec3::new(x, 1.12, 0.95),
                    Quat::IDENTITY,
                    Vec3::new(0.48, 0.40, 0.06),
                ),
                [0.30, 0.69, 0.79, 1.0],
            );
            for (frame, frame_x, frame_y, frame_scale) in [
                ("Top", x, 1.37, Vec3::new(0.58, 0.07, 0.10)),
                ("Bottom", x, 0.87, Vec3::new(0.58, 0.07, 0.10)),
                ("Left", x - 0.275, 1.12, Vec3::new(0.07, 0.50, 0.10)),
                ("Right", x + 0.275, 1.12, Vec3::new(0.07, 0.50, 0.10)),
            ] {
                add_showcase_object(
                    &mut scene,
                    format!("{side} Window {frame} Frame"),
                    PrimitiveType::Cube,
                    generate_cube(1.0),
                    Transform::from_translation_rotation_scale(
                        Vec3::new(frame_x, frame_y, 0.99),
                        Quat::IDENTITY,
                        frame_scale,
                    ),
                    [0.88, 0.76, 0.55, 1.0],
                );
            }

            // A cross-shaped muntin and projecting sill give each window more depth.
            for (detail, position, scale) in [
                (
                    "Vertical Muntin",
                    Vec3::new(x, 1.12, 1.045),
                    Vec3::new(0.045, 0.40, 0.035),
                ),
                (
                    "Horizontal Muntin",
                    Vec3::new(x, 1.12, 1.048),
                    Vec3::new(0.48, 0.045, 0.04),
                ),
                (
                    "Window Sill",
                    Vec3::new(x, 0.81, 1.025),
                    Vec3::new(0.68, 0.10, 0.20),
                ),
                (
                    "Window Box",
                    Vec3::new(x, 0.68, 1.08),
                    Vec3::new(0.48, 0.16, 0.22),
                ),
            ] {
                add_showcase_object(
                    &mut scene,
                    format!("{side} {detail}"),
                    PrimitiveType::Cube,
                    generate_cube(1.0),
                    Transform::from_translation_rotation_scale(position, Quat::IDENTITY, scale),
                    if detail == "Window Box" {
                        [0.35, 0.19, 0.12, 1.0]
                    } else {
                        [0.88, 0.76, 0.55, 1.0]
                    },
                );
            }
        }

        // A small covered porch frames the entrance and visually anchors the door.
        for (name, position, scale, color, material_id) in [
            (
                "Porch Deck",
                Vec3::new(0.0, 0.14, 1.27),
                Vec3::new(1.42, 0.14, 0.78),
                [1.0, 1.0, 1.0, 1.0],
                Some(BROWN_FLOOR_TILES_MATERIAL_ID),
            ),
            (
                "Porch Awning",
                Vec3::new(0.0, 1.52, 1.34),
                Vec3::new(1.34, 0.13, 0.78),
                [0.42, 0.15, 0.12, 1.0],
                None,
            ),
            (
                "Porch Front Beam",
                Vec3::new(0.0, 1.40, 1.70),
                Vec3::new(1.38, 0.12, 0.12),
                [0.34, 0.18, 0.12, 1.0],
                None,
            ),
        ] {
            let mesh = if material_id.is_some() {
                generate_material_cube(scale)
            } else {
                generate_cube(1.0)
            };
            add_showcase_object_with_material(
                &mut scene,
                name,
                PrimitiveType::Cube,
                mesh,
                Transform::from_translation_rotation_scale(position, Quat::IDENTITY, scale),
                color,
                material_id,
            );
        }
        for (side, x) in [("Left", -0.55), ("Right", 0.55)] {
            add_showcase_object(
                &mut scene,
                format!("Porch {side} Post"),
                PrimitiveType::Cube,
                generate_cube(1.0),
                Transform::from_translation_rotation_scale(
                    Vec3::new(x, 0.82, 1.63),
                    Quat::IDENTITY,
                    Vec3::new(0.13, 1.36, 0.13),
                ),
                [0.88, 0.76, 0.55, 1.0],
            );
        }

        // A broadleaf tree with a visible trunk and overlapping low-poly foliage.
        add_showcase_object(
            &mut scene,
            "Tree Trunk",
            PrimitiveType::Cylinder,
            generate_bark_cylinder(0.23, 1.85, 48, 32, 2.0),
            Transform::from_translation(Vec3::new(3.18, 0.925, -0.18)),
            [1.0, 1.0, 1.0, 1.0],
        );
        for (name, position, radius, color) in [
            (
                "Tree Canopy Center",
                Vec3::new(3.18, 2.13, -0.18),
                0.87,
                [0.19, 0.43, 0.22, 1.0],
            ),
            (
                "Tree Canopy Left",
                Vec3::new(2.62, 2.42, -0.18),
                0.65,
                [0.24, 0.51, 0.25, 1.0],
            ),
            (
                "Tree Canopy Right",
                Vec3::new(3.73, 2.43, -0.18),
                0.66,
                [0.27, 0.55, 0.27, 1.0],
            ),
        ] {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Icosphere,
                generate_icosphere(radius, 2),
                Transform::from_translation(position),
                color,
            );
        }

        // Fine foliage highlights and two branching limbs break up the round canopy.
        for (name, position, radius, color) in [
            (
                "Tree Canopy Highlight Left",
                Vec3::new(2.78, 2.75, 0.08),
                0.31,
                [0.34, 0.61, 0.28, 1.0],
            ),
            (
                "Tree Canopy Highlight Right",
                Vec3::new(3.59, 2.82, -0.33),
                0.28,
                [0.37, 0.64, 0.30, 1.0],
            ),
        ] {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Icosphere,
                generate_icosphere(radius, 1),
                Transform::from_translation(position),
                color,
            );
        }
        for (name, position, rotation) in [
            (
                "Tree Branch Left",
                Vec3::new(2.93, 1.62, -0.18),
                Quat::from_rotation_z(-0.78),
            ),
            (
                "Tree Branch Right",
                Vec3::new(3.42, 1.66, -0.18),
                Quat::from_rotation_z(0.76),
            ),
        ] {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Cylinder,
                generate_bark_cylinder(0.09, 0.83, 24, 12, position.x),
                Transform::from_translation_rotation_scale(
                    position,
                    rotation,
                    Vec3::new(1.0, 1.0, 1.0),
                ),
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // Faceted stone and a smaller pebble complete the yard composition.
        let mut rock_mesh = generate_icosphere(0.82, 3);
        apply_stone_mottling(&mut rock_mesh, 11.0);
        add_showcase_object(
            &mut scene,
            "Rock",
            PrimitiveType::Icosphere,
            rock_mesh,
            Transform::from_translation_rotation_scale(
                Vec3::new(-3.15, 0.42, 0.18),
                Quat::from_rotation_y(0.24),
                Vec3::new(1.0, 0.56, 0.82),
            ),
            [1.0, 1.0, 1.0, 1.0],
        );
        let mut pebble_mesh = generate_icosphere(0.36, 2);
        apply_stone_mottling(&mut pebble_mesh, 23.0);
        add_showcase_object(
            &mut scene,
            "Pebble",
            PrimitiveType::Icosphere,
            pebble_mesh,
            Transform::from_translation_rotation_scale(
                Vec3::new(-2.47, 0.18, 0.92),
                Quat::from_rotation_y(-0.31),
                Vec3::new(1.0, 0.55, 0.76),
            ),
            [1.0, 1.0, 1.0, 1.0],
        );

        // Individual stepping stones extend the path into the front yard.
        for (index, (x, z)) in [(-0.18, 2.63), (0.14, 3.12), (-0.12, 3.61), (0.20, 4.08)]
            .into_iter()
            .enumerate()
        {
            add_showcase_object(
                &mut scene,
                format!("Stepping Stone {}", index + 1),
                PrimitiveType::Icosphere,
                {
                    let mut mesh = generate_icosphere(0.34, 2);
                    apply_stone_mottling(&mut mesh, index as f32 * 4.0 + 31.0);
                    mesh
                },
                Transform::from_translation_rotation_scale(
                    Vec3::new(x, 0.055, z),
                    Quat::from_rotation_y(index as f32 * 0.28),
                    Vec3::new(1.0, 0.20, 0.72),
                ),
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // A low garden fence sits behind the tree, with evenly spaced pickets.
        for (index, (name, y)) in [
            ("Garden Fence Lower Rail", 0.39),
            ("Garden Fence Upper Rail", 0.79),
        ]
        .into_iter()
        .enumerate()
        {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Cube,
                generate_wood_grain_cube(0, index as u32),
                Transform::from_translation_rotation_scale(
                    Vec3::new(3.25, y, -1.28),
                    Quat::IDENTITY,
                    Vec3::new(2.75, 0.11, 0.12),
                ),
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        for index in 0..8 {
            let x = 1.98 + index as f32 * 0.36;
            add_showcase_object(
                &mut scene,
                format!("Garden Fence Picket {}", index + 1),
                PrimitiveType::Cube,
                generate_wood_grain_cube(1, index),
                Transform::from_translation_rotation_scale(
                    Vec3::new(x, 0.59, -1.28),
                    Quat::IDENTITY,
                    Vec3::new(0.11, 0.85, 0.14),
                ),
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // Shrubs and small flower clusters add scale cues without external assets.
        for (name, position, radius, color) in [
            (
                "Garden Shrub Left",
                Vec3::new(-1.63, 0.27, 0.34),
                0.45,
                [0.16, 0.36, 0.18, 1.0],
            ),
            (
                "Garden Shrub Center",
                Vec3::new(-1.30, 0.24, 0.51),
                0.36,
                [0.20, 0.43, 0.20, 1.0],
            ),
            (
                "Garden Shrub Right",
                Vec3::new(1.53, 0.25, 0.26),
                0.39,
                [0.17, 0.39, 0.18, 1.0],
            ),
        ] {
            add_showcase_object(
                &mut scene,
                name,
                PrimitiveType::Icosphere,
                generate_icosphere(radius, 1),
                Transform::from_translation(position),
                color,
            );
        }
        for (index, (x, z, color)) in [
            (-1.92, 0.94, [0.93, 0.69, 0.25, 1.0]),
            (-1.68, 1.12, [0.78, 0.30, 0.28, 1.0]),
            (-1.43, 0.98, [0.93, 0.69, 0.25, 1.0]),
            (1.52, 0.95, [0.78, 0.30, 0.28, 1.0]),
            (1.78, 0.80, [0.93, 0.69, 0.25, 1.0]),
        ]
        .into_iter()
        .enumerate()
        {
            add_showcase_object(
                &mut scene,
                format!("Flower Stem {}", index + 1),
                PrimitiveType::Cylinder,
                generate_cylinder(0.025, 0.38, 8),
                Transform::from_translation(Vec3::new(x, 0.23, z)),
                [0.18, 0.34, 0.15, 1.0],
            );
            add_showcase_object(
                &mut scene,
                format!("Flower Head {}", index + 1),
                PrimitiveType::Icosphere,
                generate_icosphere(0.13, 1),
                Transform::from_translation(Vec3::new(x, 0.48, z)),
                color,
            );
        }

        // Exercise full image-based PBR on the tree, garden timber, and natural stone.
        for object in scene.objects.values_mut() {
            let material_id =
                if object.name == "Tree Trunk" || object.name.starts_with("Tree Branch ") {
                    Some("polyhaven/bark_brown_01")
                } else if object.name.starts_with("Garden Fence ")
                    || (object.name.starts_with("Porch ") && object.name != "Porch Deck")
                {
                    Some("polyhaven/wooden_planks")
                } else if object.name == "Rock"
                    || object.name == "Pebble"
                    || object.name.starts_with("Stepping Stone ")
                {
                    Some("polyhaven/mossy_rock")
                } else {
                    None
                };
            if let Some(material_id) = material_id {
                object.material_id = Some(material_id.into());
                for vertex in &mut object.mesh.vertices {
                    vertex.color = [1.0; 4];
                }
            }
        }

        scene
    }
}

/// Builds a segmented cylinder whose vertex colors carry longitudinal bark grain.
fn generate_bark_cylinder(
    radius: f32,
    height: f32,
    sectors: u32,
    height_segments: u32,
    seed: f32,
) -> Mesh {
    let sectors = sectors.max(8);
    let height_segments = height_segments.max(2);
    let half_height = height * 0.5;
    let stride = sectors as usize + 1;
    let mut vertices = Vec::with_capacity((height_segments as usize + 1) * stride);
    let mut indices = Vec::with_capacity((height_segments * sectors * 6) as usize);

    for row in 0..=height_segments {
        let v = row as f32 / height_segments as f32;
        let y = -half_height + v * height;
        for sector in 0..=sectors {
            let u = sector as f32 / sectors as f32;
            let angle = u * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            vertices.push(Vertex::new(
                [radius * cos, y, radius * sin],
                [cos, 0.0, sin],
                [u, v],
                bark_color(angle, v, seed),
            ));
        }
    }

    for row in 0..height_segments {
        for sector in 0..sectors {
            let lower = (row as usize * stride + sector as usize) as u32;
            let upper = lower + stride as u32;
            indices.extend_from_slice(&[upper, lower, upper + 1, upper + 1, lower, lower + 1]);
        }
    }

    // Separate cap vertices keep the end grain and top/bottom normals clean.
    let top_center = vertices.len() as u32;
    vertices.push(Vertex::new(
        [0.0, half_height, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
        [0.36, 0.20, 0.11, 1.0],
    ));
    let top_rim = vertices.len() as u32;
    for sector in 0..sectors {
        let angle = sector as f32 / sectors as f32 * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        vertices.push(Vertex::new(
            [radius * cos, half_height, radius * sin],
            [0.0, 1.0, 0.0],
            [0.5 + cos * 0.5, 0.5 + sin * 0.5],
            [0.39, 0.22, 0.12, 1.0],
        ));
    }
    for sector in 0..sectors {
        indices.extend_from_slice(&[
            top_center,
            top_rim + sector,
            top_rim + (sector + 1) % sectors,
        ]);
    }

    let bottom_center = vertices.len() as u32;
    vertices.push(Vertex::new(
        [0.0, -half_height, 0.0],
        [0.0, -1.0, 0.0],
        [0.5, 0.5],
        [0.31, 0.17, 0.09, 1.0],
    ));
    let bottom_rim = vertices.len() as u32;
    for sector in 0..sectors {
        let angle = sector as f32 / sectors as f32 * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        vertices.push(Vertex::new(
            [radius * cos, -half_height, radius * sin],
            [0.0, -1.0, 0.0],
            [0.5 + cos * 0.5, 0.5 + sin * 0.5],
            [0.31, 0.17, 0.09, 1.0],
        ));
    }
    for sector in 0..sectors {
        indices.extend_from_slice(&[
            bottom_center,
            bottom_rim + (sector + 1) % sectors,
            bottom_rim + sector,
        ]);
    }

    Mesh::new(vertices, indices)
}

fn bark_color(angle: f32, height_ratio: f32, seed: f32) -> [f32; 4] {
    let broad_ridge = (angle * 7.0 + (height_ratio * 5.5 + seed).sin() * 0.65).sin();
    let fine_ridge = (angle * 19.0 + height_ratio * 8.0 + seed * 1.7).sin();
    let broken_furrow = (angle * 11.0 + height_ratio * 15.0 + seed * 0.4).sin();
    let furrow = if broken_furrow > 0.965 { 0.62 } else { 1.0 };
    let brightness = (0.84 + 0.14 * broad_ridge + 0.07 * fine_ridge) * furrow;
    [
        (0.36 * brightness).clamp(0.0, 1.0),
        (0.19 * brightness).clamp(0.0, 1.0),
        (0.095 * brightness).clamp(0.0, 1.0),
        1.0,
    ]
}

/// Builds a subdivided box with wood fibers running along the selected local axis.
/// `grain_axis` uses 0=X, 1=Y, and 2=Z.
fn generate_wood_grain_cube(grain_axis: usize, seed: u32) -> Mesh {
    let faces = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, Vec3::Y, Vec3::X),
    ];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (normal, u_axis, v_axis) in faces {
        let u_steps = if axis_component(u_axis, grain_axis) != 0.0 {
            28
        } else {
            5
        };
        let v_steps = if axis_component(v_axis, grain_axis) != 0.0 {
            28
        } else {
            5
        };
        let base = vertices.len() as u32;
        for v in 0..=v_steps {
            let fv = v as f32 / v_steps as f32;
            for u in 0..=u_steps {
                let fu = u as f32 / u_steps as f32;
                let local = normal * 0.5 + u_axis * (fu - 0.5) + v_axis * (fv - 0.5);
                vertices.push(Vertex::new(
                    local.to_array(),
                    normal.to_array(),
                    [fu, fv],
                    wood_grain_color(local, grain_axis, seed),
                ));
            }
        }
        let row = u_steps + 1;
        for v in 0..v_steps {
            for u in 0..u_steps {
                let lower_left = base + v * row + u;
                let lower_right = lower_left + 1;
                let upper_left = lower_left + row;
                let upper_right = upper_left + 1;
                indices.extend_from_slice(&[
                    lower_left,
                    lower_right,
                    upper_right,
                    lower_left,
                    upper_right,
                    upper_left,
                ]);
            }
        }
    }

    Mesh::new(vertices, indices)
}

fn axis_component(vector: Vec3, axis: usize) -> f32 {
    match axis {
        0 => vector.x,
        1 => vector.y,
        _ => vector.z,
    }
}

fn wood_grain_color(position: Vec3, grain_axis: usize, seed: u32) -> [f32; 4] {
    let (along, across, depth) = match grain_axis {
        0 => (position.x, position.y, position.z),
        1 => (position.y, position.x, position.z),
        _ => (position.z, position.x, position.y),
    };
    let phase = seed as f32 * 1.31;
    let broad_fiber = (along * 39.0 + (across * 5.7 + phase).sin() * 0.72).sin();
    let fine_fiber = (along * 93.0 + across * 11.0 + phase * 0.37).sin();
    let knot_center = ((seed as f32 * 0.173).sin()) * 0.22;
    let knot_radius = ((along - knot_center) * 4.2).powi(2) + (across * 3.2).powi(2);
    let knot = (-knot_radius * 1.8).exp();
    let ring = (knot_radius.sqrt() * 31.0 + depth * 2.0).sin();
    let brightness = (0.83 + 0.13 * broad_fiber + 0.045 * fine_fiber - knot * (0.12 + 0.08 * ring))
        .clamp(0.55, 1.08);
    [
        0.43 * brightness,
        0.255 * brightness,
        0.145 * brightness,
        1.0,
    ]
}

/// Bakes a mottled, subtly veined stone pattern into per-triangle vertex colors.
fn apply_stone_mottling(mesh: &mut Mesh, seed: f32) {
    let source_vertices = &mesh.vertices;
    let mut vertices = Vec::with_capacity(mesh.indices.len());
    let mut indices = Vec::with_capacity(mesh.indices.len());

    for (face_index, triangle) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let p0 = Vec3::from_array(source_vertices[triangle[0] as usize].position);
        let p1 = Vec3::from_array(source_vertices[triangle[1] as usize].position);
        let p2 = Vec3::from_array(source_vertices[triangle[2] as usize].position);
        let color = stone_surface_color((p0 + p1 + p2) / 3.0, seed, face_index as f32);
        for &source_index in triangle {
            let mut vertex = source_vertices[source_index as usize];
            vertex.color = color;
            indices.push(vertices.len() as u32);
            vertices.push(vertex);
        }
    }

    mesh.vertices = vertices;
    mesh.indices = indices;
    mesh.recalculate_aabb();
}

fn stone_surface_color(position: Vec3, seed: f32, face_index: f32) -> [f32; 4] {
    let broad_mottle = (position.x * 17.0 + position.y * 11.0 + position.z * 13.0 + seed).sin();
    let fine_grain = (position.x * 41.0 - position.y * 29.0 + position.z * 37.0 + seed * 0.7).sin();
    let fleck = (face_index * 2.399 + seed * 1.73).sin();
    let vein = (position.x * 23.0 + position.y * 19.0 - position.z * 21.0 + seed * 0.3).sin();
    let crack = if vein.abs() < 0.075 { 0.70 } else { 1.0 };
    let brightness = (0.86 + 0.10 * broad_mottle + 0.055 * fine_grain + 0.035 * fleck) * crack;
    let warm_mineral =
        (0.5 + 0.5 * (position.x * 8.0 + position.z * 12.0 + seed).sin()).powi(8) * 0.045;
    [
        (0.39 * brightness + warm_mineral).clamp(0.0, 1.0),
        (0.41 * brightness + warm_mineral * 0.68).clamp(0.0, 1.0),
        (0.43 * brightness + warm_mineral * 0.30).clamp(0.0, 1.0),
        1.0,
    ]
}

fn add_showcase_object(
    scene: &mut Scene,
    name: impl Into<String>,
    primitive_type: PrimitiveType,
    mesh: forge_geometry::Mesh,
    transform: Transform,
    color: [f32; 4],
) {
    add_showcase_object_with_material(scene, name, primitive_type, mesh, transform, color, None);
}

/// Builds a cube with UV density matching the tile asset's 1.7 m physical width.
fn generate_material_cube(world_scale: Vec3) -> forge_geometry::Mesh {
    const MATERIAL_WIDTH_METERS: f32 = 1.7;
    let mut mesh = generate_cube(1.0);
    for vertex in &mut mesh.vertices {
        let normal = vertex.normal;
        let (u_scale, v_scale) = if normal[1].abs() > 0.5 {
            (
                world_scale.x / MATERIAL_WIDTH_METERS,
                world_scale.z / MATERIAL_WIDTH_METERS,
            )
        } else if normal[2].abs() > 0.5 {
            (
                world_scale.x / MATERIAL_WIDTH_METERS,
                world_scale.y / MATERIAL_WIDTH_METERS,
            )
        } else {
            (
                world_scale.z / MATERIAL_WIDTH_METERS,
                world_scale.y / MATERIAL_WIDTH_METERS,
            )
        };
        vertex.uv[0] *= u_scale;
        vertex.uv[1] *= v_scale;
    }
    mesh
}

fn add_showcase_object_with_material(
    scene: &mut Scene,
    name: impl Into<String>,
    primitive_type: PrimitiveType,
    mesh: forge_geometry::Mesh,
    transform: Transform,
    color: [f32; 4],
    material_id: Option<&str>,
) {
    let mut object = SceneObject::new(name, primitive_type, mesh)
        .with_transform(transform)
        .with_color(color);
    if let Some(material_id) = material_id {
        object = object.with_material_id(material_id);
    }
    scene.add_object(object);
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
    fn test_showcase_scene_contains_facade_and_garden_detail() {
        let scene = Scene::create_showcase_scene();
        let names: std::collections::HashSet<&str> = scene
            .objects
            .values()
            .map(|object| object.name.as_str())
            .collect();

        for expected in [
            "Porch Deck",
            "Roof Ridge Cap",
            "Left Vertical Muntin",
            "Tree Branch Left",
            "Garden Fence Upper Rail",
            "Flower Head 1",
            "Stepping Stone 4",
        ] {
            assert!(
                names.contains(expected),
                "missing showcase detail: {expected}"
            );
        }
        assert!(scene.objects.len() >= 60);

        for material_object_name in ["Front Path", "Porch Deck"] {
            let material_object = scene
                .objects
                .values()
                .find(|object| object.name == material_object_name)
                .expect("showcase surface exists");
            assert_eq!(
                material_object.material_id.as_deref(),
                Some(BROWN_FLOOR_TILES_MATERIAL_ID)
            );
        }

        for (object_name, material_id) in [
            ("Tree Trunk", "polyhaven/bark_brown_01"),
            ("Garden Fence Picket 1", "polyhaven/wooden_planks"),
            ("Rock", "polyhaven/mossy_rock"),
        ] {
            let object = scene
                .objects
                .values()
                .find(|object| object.name == object_name)
                .expect("showcase surface exists");
            assert_eq!(object.material_id.as_deref(), Some(material_id));
            let material = scene.materials.get(material_id).expect("PBR asset exists");
            assert!(material.textures.base_color.is_some());
            assert!(material.textures.normal.is_some());
            assert!(material.textures.roughness.is_some());
            assert!(material.textures.ambient_occlusion.is_some());
        }

        for textured_name in ["Tree Trunk", "Garden Fence Picket 1", "Rock"] {
            let mesh = &scene
                .objects
                .values()
                .find(|object| object.name == textured_name)
                .expect("textured showcase object exists")
                .mesh;
            assert!(mesh.validate().is_ok(), "invalid mesh for {textured_name}");
            assert!(
                mesh.vertices.iter().all(|vertex| vertex.color == [1.0; 4]),
                "PBR maps should not be multiplied by old vertex-color patterns on {textured_name}"
            );
        }
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
