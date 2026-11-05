use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Modifier that displaces mesh vertices along their surface normals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplaceModifier {
    /// Displacement strength multiplier.
    pub strength: f32,
    /// Reference mid-level where no displacement occurs.
    pub midlevel: f32,
    /// Spatial frequency for procedural harmonic displacement.
    pub frequency: f32,
}

impl Default for DisplaceModifier {
    fn default() -> Self {
        Self {
            strength: 0.1,
            midlevel: 0.0,
            frequency: 2.0,
        }
    }
}

impl DisplaceModifier {
    pub fn new(strength: f32) -> Self {
        Self {
            strength,
            ..Default::default()
        }
    }

    pub fn apply(&self, mesh: &Mesh) -> Mesh {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() || self.strength.abs() < 1e-6 {
            return mesh.clone();
        }

        let new_vertices: Vec<Vertex> = mesh
            .vertices
            .iter()
            .map(|v| {
                let mut vert = *v;
                let pos = Vec3::from(v.position);
                let norm = Vec3::from(v.normal);

                // Procedural 3D harmonic displacement pattern
                let wave = (pos.x * self.frequency).sin()
                    * (pos.y * self.frequency).cos()
                    * (pos.z * self.frequency).sin();

                let factor = (wave - self.midlevel) * self.strength;
                let new_pos = pos + norm * factor;

                vert.position = new_pos.into();
                vert
            })
            .collect();

        let mut new_mesh = Mesh::new(new_vertices, mesh.indices.clone());
        new_mesh.recalculate_normals();
        new_mesh
    }
}
