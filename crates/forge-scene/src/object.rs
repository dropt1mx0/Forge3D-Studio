use forge_core::id::ObjectId;
use forge_core::traits::{Identifiable, Named};
use forge_geometry::Mesh;
use forge_math::{Quat, Transform, Vec3};
use serde::{Deserialize, Serialize};

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
    Camera,
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
    pub parent: Option<ObjectId>,
    pub children: Vec<ObjectId>,
    pub modifiers: forge_modifiers::ModifierStack,
    pub camera_data: Option<CameraData>,
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
            parent: None,
            children: Vec::new(),
            modifiers: forge_modifiers::ModifierStack::new(),
            camera_data: None,
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

    /// Evaluates base mesh through all active modifiers in stack.
    pub fn evaluated_mesh(&self) -> Mesh {
        if self.modifiers.is_empty() {
            self.mesh.clone()
        } else {
            self.modifiers.evaluate(&self.mesh)
        }
    }

    /// Applies the current local transform permanently to the mesh geometry and resets transform to identity.
    pub fn apply_transform(&mut self) {
        self.mesh.transform(&self.local_transform);
        self.local_transform = Transform::IDENTITY;
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
