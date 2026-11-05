use std::collections::HashMap;

use forge_geometry::{Mesh, Vertex};
use serde::{Deserialize, Serialize};

/// Modifier that simplifies meshes to reduce polygon count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecimateModifier {
    /// Target triangle ratio (0.05 to 1.0). E.g. 0.5 targets 50% of triangles.
    pub ratio: f32,
}

impl Default for DecimateModifier {
    fn default() -> Self {
        Self { ratio: 0.5 }
    }
}

impl DecimateModifier {
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.05, 1.0),
        }
    }

    pub fn apply(&self, mesh: &Mesh) -> Mesh {
        if mesh.vertices.is_empty() || mesh.indices.len() < 6 || self.ratio >= 0.999 {
            return mesh.clone();
        }

        // Simplify using spatial clustering / grid collapse
        let aabb = mesh.aabb;
        let max_dim = aabb.size().max_element().max(1e-4);

        // Adjust cluster cell size dynamically based on ratio
        let divisions = ((1.0 / (1.0 - self.ratio + 0.05)) * 10.0).clamp(4.0, 100.0);
        let cell_size = max_dim / divisions;
        let inv_cell = 1.0 / cell_size;

        let mut cluster_map: HashMap<(i64, i64, i64), usize> = HashMap::new();
        let mut vertex_remap: Vec<usize> = Vec::with_capacity(mesh.vertices.len());
        let mut new_vertices: Vec<Vertex> = Vec::new();

        for v in &mesh.vertices {
            let key = (
                (v.position[0] * inv_cell).round() as i64,
                (v.position[1] * inv_cell).round() as i64,
                (v.position[2] * inv_cell).round() as i64,
            );

            match cluster_map.get(&key) {
                Some(&existing) => {
                    vertex_remap.push(existing);
                }
                None => {
                    let idx = new_vertices.len();
                    new_vertices.push(*v);
                    cluster_map.insert(key, idx);
                    vertex_remap.push(idx);
                }
            }
        }

        let mut new_indices: Vec<u32> = Vec::new();
        for chunk in mesh.indices.as_chunks::<3>().0 {
            let i0 = vertex_remap[chunk[0] as usize] as u32;
            let i1 = vertex_remap[chunk[1] as usize] as u32;
            let i2 = vertex_remap[chunk[2] as usize] as u32;

            // Reject collapsed/degenerate triangles
            if i0 != i1 && i1 != i2 && i0 != i2 {
                new_indices.push(i0);
                new_indices.push(i1);
                new_indices.push(i2);
            }
        }

        if new_indices.len() < 3 || new_vertices.is_empty() {
            return mesh.clone();
        }

        let mut result = Mesh::new(new_vertices, new_indices);
        result.recalculate_normals();
        result
    }
}
