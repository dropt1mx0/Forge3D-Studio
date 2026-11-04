use std::sync::OnceLock;

use forge_core::id::ObjectId;
use forge_core::traits::{Identifiable, Named};
use forge_geometry::Mesh;
use forge_math::{Quat, Transform, Vec3};
use serde::{Deserialize, Serialize};

use crate::material::LightData;

/// Type of 3D primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    Cube,
    Sphere,
    Icosphere,
    Plane,
    Circle,
    Cylinder,
    Cone,
    Torus,
    Grid,
    Capsule,
    Monkey,
    Camera,
    Light,
    Custom,
}

/// Camera projection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CameraProjectionType {
    #[default]
    Perspective,
    Orthographic,
}

/// Optical and rendering properties of a physical Camera object in the scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraData {
    pub projection: CameraProjectionType,
    pub fov_y_deg: f32,
    pub focal_length_mm: f32,
    pub sensor_width_mm: f32,
    pub sensor_height_mm: f32,
    pub clip_start: f32,
    pub clip_end: f32,
    pub ortho_scale: f32,
    pub dof_enabled: bool,
    pub dof_distance: f32,
    pub dof_fstop: f32,
    pub resolution: [u32; 2],
}

impl Default for CameraData {
    fn default() -> Self {
        Self {
            projection: CameraProjectionType::Perspective,
            fov_y_deg: 50.0,
            focal_length_mm: 50.0,
            sensor_width_mm: 36.0,
            sensor_height_mm: 24.0,
            clip_start: 0.1,
            clip_end: 1000.0,
            ortho_scale: 6.0,
            dof_enabled: false,
            dof_distance: 5.0,
            dof_fstop: 2.8,
            resolution: [1920, 1080],
        }
    }
}

/// Calculated view and projection parameters for looking through a scene camera.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraViewParameters {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_deg: f32,
    pub clip_start: f32,
    pub clip_end: f32,
    pub ortho_scale: f32,
    pub is_orthographic: bool,
}

/// Locks for individual transform components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TransformLocks {
    pub lock_pos: [bool; 3],
    pub lock_rot: [bool; 3],
    pub lock_scale: [bool; 3],
}

/// A 3D object in the scene hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub local_transform: Transform,
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub locks: TransformLocks,
    pub primitive_type: PrimitiveType,
    pub mesh: Mesh,
    pub color: [f32; 4],
    /// Optional renderer material identifier; absent IDs use the default viewport material.
    #[serde(default)]
    pub material_id: Option<String>,
    pub parent: Option<ObjectId>,
    pub children: Vec<ObjectId>,
    pub modifiers: forge_modifiers::ModifierStack,
    pub camera_data: Option<CameraData>,
    #[serde(default)]
    pub light_data: Option<LightData>,
    #[serde(skip, default)]
    pub generation: u64,
    #[serde(skip, default)]
    evaluated_mesh_cache: OnceLock<Mesh>,
}

impl SceneObject {
    /// Constructs a new scene object with the given name, primitive type, and mesh.
    pub fn new(name: impl Into<String>, primitive_type: PrimitiveType, mesh: Mesh) -> Self {
        Self {
            id: ObjectId::new(),
            name: name.into(),
            local_transform: Transform::IDENTITY,
            visible: true,
            locked: false,
            locks: TransformLocks::default(),
            primitive_type,
            mesh,
            color: [0.8, 0.8, 0.85, 1.0], // Neutral clay/matte color
            material_id: None,
            parent: None,
            children: Vec::new(),
            modifiers: forge_modifiers::ModifierStack::new(),
            camera_data: None,
            light_data: None,
            generation: 0,
            evaluated_mesh_cache: OnceLock::new(),
        }
    }

    /// Constructs a new Camera object with physical marker mesh and default optics.
    pub fn new_camera(name: impl Into<String>) -> Self {
        let mesh = forge_geometry::primitives::generate_camera_marker();
        let mut obj = Self::new(name, PrimitiveType::Camera, mesh);
        obj.color = [0.4, 0.45, 0.55, 1.0];
        obj.camera_data = Some(CameraData::default());
        obj
    }

    /// Constructs a light object with a small viewport marker mesh.
    pub fn new_light(name: impl Into<String>, light_data: LightData, mesh: Mesh) -> Self {
        let mut obj = Self::new(name, PrimitiveType::Light, mesh);
        obj.light_data = Some(light_data);
        obj.color = [
            light_data.color[0],
            light_data.color[1],
            light_data.color[2],
            1.0,
        ];
        obj
    }

    /// Builder method to set initial transform.
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Builder method to set initial color.
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Builder method to select a renderer material by asset ID.
    pub fn with_material_id(mut self, material_id: impl Into<String>) -> Self {
        self.material_id = Some(material_id.into());
        self
    }

    /// Invalidates the cached evaluated mesh and increments geometry generation.
    pub fn invalidate_evaluated_mesh(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.evaluated_mesh_cache.take();
    }

    /// Returns whether the evaluated mesh is currently cached in memory.
    pub fn is_evaluated_mesh_cached(&self) -> bool {
        if self.modifiers.is_empty() {
            true
        } else {
            self.evaluated_mesh_cache.get().is_some()
        }
    }

    /// Sets a new base mesh, invalidating any cached evaluated mesh and incrementing generation.
    pub fn set_mesh(&mut self, mesh: Mesh) {
        self.mesh = mesh;
        self.invalidate_evaluated_mesh();
    }

    /// Provides mutable access to the base mesh, invalidating the evaluated cache.
    pub fn mesh_mut(&mut self) -> &mut Mesh {
        self.invalidate_evaluated_mesh();
        &mut self.mesh
    }

    /// Provides mutable access to the modifier stack, invalidating the evaluated cache.
    pub fn modifiers_mut(&mut self) -> &mut forge_modifiers::ModifierStack {
        self.invalidate_evaluated_mesh();
        &mut self.modifiers
    }

    /// Evaluates base mesh through all active modifiers in stack with zero-copy caching.
    ///
    /// If no modifiers are present, returns a direct reference to `self.mesh` without cloning.
    /// If modifiers are present, computes the result once and caches it until invalidated.
    pub fn evaluated_mesh(&self) -> &Mesh {
        if self.modifiers.is_empty() {
            &self.mesh
        } else {
            self.evaluated_mesh_cache
                .get_or_init(|| self.modifiers.evaluate(&self.mesh))
        }
    }

    /// Returns an owned clone of the evaluated mesh.
    pub fn evaluated_mesh_cloned(&self) -> Mesh {
        self.evaluated_mesh().clone()
    }

    /// Applies the current local transform permanently to the mesh geometry and resets transform to identity.
    pub fn apply_transform(&mut self) {
        self.mesh.transform(&self.local_transform);
        self.local_transform = Transform::IDENTITY;
        self.invalidate_evaluated_mesh();
    }

    /// Resets translation, rotation, and scale to identity.
    pub fn reset_transform(&mut self) {
        self.local_transform = Transform::IDENTITY;
    }

    /// Resets translation to (0, 0, 0).
    pub fn reset_translation(&mut self) {
        self.local_transform.translation = Vec3::ZERO;
    }

    /// Resets rotation to identity quaternion (0, 0, 0, 1).
    pub fn reset_rotation(&mut self) {
        self.local_transform.rotation = Quat::IDENTITY;
    }

    /// Resets scale to uniform (1, 1, 1).
    pub fn reset_scale(&mut self) {
        self.local_transform.scale = Vec3::ONE;
    }
}

impl Identifiable for SceneObject {
    fn id(&self) -> ObjectId {
        self.id
    }
}

impl Named for SceneObject {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::generate_cube;
    use forge_modifiers::{ModifierEntry, ModifierType, SubdivisionModifier};

    #[test]
    fn test_zero_copy_without_modifiers() {
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);

        // When no modifiers are active, evaluated_mesh points directly to obj.mesh without clone
        let eval_ref = obj.evaluated_mesh();
        assert!(std::ptr::eq(eval_ref, &obj.mesh));
        assert!(obj.is_evaluated_mesh_cached());
    }

    #[test]
    fn test_evaluated_mesh_caching_with_modifiers() {
        let cube = generate_cube(1.0);
        let mut obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        obj.modifiers
            .add(ModifierType::Subdivide(SubdivisionModifier::new(1)));

        // Initially not yet evaluated
        assert!(!obj.is_evaluated_mesh_cached());

        // First evaluation computes and caches the result
        let eval1_ptr = obj.evaluated_mesh() as *const Mesh;
        assert!(obj.is_evaluated_mesh_cached());

        // Second evaluation reuses the exact same cached instance in memory
        let eval2_ptr = obj.evaluated_mesh() as *const Mesh;
        assert_eq!(
            eval1_ptr, eval2_ptr,
            "Subsequent calls must return identical cached mesh address"
        );

        // Evaluated mesh has more triangles than base cube
        assert!(obj.evaluated_mesh().indices.len() > obj.mesh.indices.len());
    }

    #[test]
    fn test_cache_invalidation_on_mesh_change() {
        let cube = generate_cube(1.0);
        let mut obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        obj.modifiers
            .add(ModifierType::Subdivide(SubdivisionModifier::new(1)));

        let _ = obj.evaluated_mesh();
        assert!(obj.is_evaluated_mesh_cached());
        let gen_before = obj.generation;

        // Invalidate via set_mesh
        let new_cube = generate_cube(2.0);
        obj.set_mesh(new_cube);

        assert!(
            !obj.is_evaluated_mesh_cached(),
            "Cache must be cleared after set_mesh"
        );
        assert_eq!(
            obj.generation,
            gen_before + 1,
            "Generation must increment on mesh change"
        );

        // Next evaluation re-caches
        let _ = obj.evaluated_mesh();
        assert!(obj.is_evaluated_mesh_cached());
    }

    #[test]
    fn test_cache_invalidation_on_modifier_change() {
        let cube = generate_cube(1.0);
        let mut obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        obj.modifiers
            .add(ModifierType::Subdivide(SubdivisionModifier::new(1)));

        let eval1_tris = obj.evaluated_mesh().indices.len() / 3;
        assert!(obj.is_evaluated_mesh_cached());
        let gen_before = obj.generation;

        // Mutate modifiers via modifiers_mut()
        obj.modifiers_mut().modifiers[0] =
            ModifierEntry::new(ModifierType::Subdivide(SubdivisionModifier::new(2)));
        assert!(
            !obj.is_evaluated_mesh_cached(),
            "Cache must be cleared on modifiers_mut"
        );
        assert_eq!(obj.generation, gen_before + 1);

        // Next evaluation computes updated geometry with level 2 subdivision
        let eval2_tris = obj.evaluated_mesh().indices.len() / 3;
        assert!(
            eval2_tris > eval1_tris,
            "Level 2 subdivision must produce more triangles than level 1"
        );
    }

    #[test]
    fn test_cache_invalidation_on_apply_transform() {
        let cube = generate_cube(1.0);
        let mut obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        obj.local_transform.translation = Vec3::new(5.0, 0.0, 0.0);
        obj.modifiers
            .add(ModifierType::Subdivide(SubdivisionModifier::new(1)));

        let _ = obj.evaluated_mesh();
        assert!(obj.is_evaluated_mesh_cached());
        let gen_before = obj.generation;

        obj.apply_transform();
        assert!(
            !obj.is_evaluated_mesh_cached(),
            "apply_transform must invalidate evaluated cache"
        );
        assert_eq!(obj.generation, gen_before + 1);
        assert_eq!(obj.local_transform, Transform::IDENTITY);
    }
}
