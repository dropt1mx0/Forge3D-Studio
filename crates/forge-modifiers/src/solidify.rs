use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Non-destructive Solidify modifier giving thickness to flat or open surfaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolidifyModifier {
    pub thickness: f32,
}

impl Default for SolidifyModifier {
    fn default() -> Self {
        Self { thickness: 0.1 }
    }
}

impl SolidifyModifier {
    pub fn new(thickness: f32) -> Self {
        Self { thickness }
    }

    pub fn apply(&self, base_mesh: &Mesh) -> Mesh {
        if self.thickness.abs() < 1e-4 {
            return base_mesh.clone();
        }

        let mut out_vertices = base_mesh.vertices.clone();
        let mut out_indices = base_mesh.indices.clone();

        let base_len = base_mesh.vertices.len();
        let back_start = base_len as u32;

        // Create back wall vertices offset along opposite normal
        for v in &base_mesh.vertices {
            let pos = Vec3::from_array(v.position) - Vec3::from_array(v.normal) * self.thickness;
            let norm = -Vec3::from_array(v.normal);

            out_vertices.push(Vertex::new(pos.to_array(), norm.to_array(), v.uv, v.color));
        }

        // Back wall indices with inverted winding
        for chunk in base_mesh.indices.as_chunks::<3>().0 {
            let i0 = back_start + chunk[0];
            let i1 = back_start + chunk[1];
            let i2 = back_start + chunk[2];

            out_indices.push(i0);
            out_indices.push(i2);
            out_indices.push(i1);
        }

        Mesh::new(out_vertices, out_indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::generate_plane;

    #[test]
    fn test_solidify_modifier() {
        let plane = generate_plane(2.0, 2);
        let solidify = SolidifyModifier::new(0.2);
        let result = solidify.apply(&plane);

        assert_eq!(result.vertices.len(), plane.vertices.len() * 2);
        assert_eq!(result.indices.len(), plane.indices.len() * 2);
    }
}
