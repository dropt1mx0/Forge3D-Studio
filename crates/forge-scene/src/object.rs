use forge_core::id::ObjectId;
use forge_core::traits::{Identifiable, Named};
use forge_geometry::Mesh;
use forge_math::Transform;
use serde::{Deserialize, Serialize};

/// Type of 3D primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    Cube,
    Sphere,
    Plane,
    Custom,
}

/// A 3D object in the scene hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub local_transform: Transform,
    pub visible: bool,
    pub primitive_type: PrimitiveType,
    pub mesh: Mesh,
    pub color: [f32; 4],
    pub parent: Option<ObjectId>,
    pub children: Vec<ObjectId>,
}

impl SceneObject {
    /// Constructs a new scene object with the given name, primitive type, and mesh.
    pub fn new(name: impl Into<String>, primitive_type: PrimitiveType, mesh: Mesh) -> Self {
        Self {
            id: ObjectId::new(),
            name: name.into(),
            local_transform: Transform::IDENTITY,
            visible: true,
            primitive_type,
            mesh,
            color: [0.8, 0.8, 0.85, 1.0], // Neutral clay/matte color
            parent: None,
            children: Vec::new(),
        }
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
