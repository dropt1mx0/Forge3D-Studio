use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Non-destructive Array modifier repeating geometry `count` times with `offset`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayModifier {
    pub count: usize,
    pub offset: Vec3,
}

impl Default for ArrayModifier {
    fn default() -> Self {
        Self {
            count: 3,
            offset: Vec3::new(2.0, 0.0, 0.0),
        }
    }
}

impl ArrayModifier {
    pub fn new(count: usize, offset: Vec3) -> Self {
        Self {
            count: count.max(1),
            offset,
        }
    }

    pub fn apply(&self, base_mesh: &Mesh) -> Mesh {
        if self.count <= 1 {
            return base_mesh.clone();
        }

        let mut out_vertices = Vec::with_capacity(base_mesh.vertices.len() * self.count);
        let mut out_indices = Vec::with_capacity(base_mesh.indices.len() * self.count);

        for step in 0..self.count {
            let offset_step = self.offset * step as f32;
            let start_v = out_vertices.len() as u32;

            for v in &base_mesh.vertices {
                let pos = Vec3::from_array(v.position) + offset_step;
                out_vertices.push(Vertex::new(pos.to_array(), v.normal, v.uv, v.color));
            }

            for &idx in &base_mesh.indices {
                out_indices.push(start_v + idx);
            }
        }

        Mesh::new(out_vertices, out_indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::generate_cube;

    #[test]
    fn test_array_modifier() {
        let cube = generate_cube(1.0);
        let array = ArrayModifier::new(3, Vec3::new(2.0, 0.0, 0.0));
        let result = array.apply(&cube);

        assert_eq!(result.vertices.len(), cube.vertices.len() * 3);
        assert_eq!(result.indices.len(), cube.indices.len() * 3);
    }
}
