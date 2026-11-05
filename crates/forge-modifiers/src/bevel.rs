use forge_geometry::{EditableMesh, Mesh};
use serde::{Deserialize, Serialize};

/// Modifier that chamfers/bevels edges non-destructively.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BevelModifier {
    /// Width of the chamfer/bevel in world units.
    pub width: f32,
    /// Number of segments (currently 1 for chamfer).
    pub segments: u32,
    /// Only bevel edges sharper than this angle in radians (e.g. 0.5 rad ≈ 30°).
    pub angle_threshold: f32,
}

impl Default for BevelModifier {
    fn default() -> Self {
        Self {
            width: 0.1,
            segments: 1,
            angle_threshold: 0.5,
        }
    }
}

impl BevelModifier {
    pub fn new(width: f32) -> Self {
        Self {
            width,
            ..Default::default()
        }
    }

    pub fn apply(&self, mesh: &Mesh) -> Mesh {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() || self.width <= 1e-5 {
            return mesh.clone();
        }

        let mut edit_mesh = EditableMesh::from_mesh(mesh);
        edit_mesh.weld_colocated_vertices(1e-4);
        edit_mesh.tris_to_quads();

        // Mark sharp edges for beveling
        for edge in &mut edit_mesh.edges {
            edge.selected = true;
        }

        let _ = edit_mesh.bevel_selected_edges(self.width);
        edit_mesh.to_render_mesh()
    }
}
