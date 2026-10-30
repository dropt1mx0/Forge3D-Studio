use forge_geometry::{EditSelectionMode, EditableMesh, Mesh};
use serde::{Deserialize, Serialize};

/// Non-destructive Subdivision Surface modifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubdivisionModifier {
    pub levels: u32,
}

impl Default for SubdivisionModifier {
    fn default() -> Self {
        Self { levels: 1 }
    }
}

impl SubdivisionModifier {
    pub fn new(levels: u32) -> Self {
        Self {
            levels: levels.clamp(1, 4),
        }
    }

    /// Evaluates subdivision by converting to `EditableMesh`, running `subdivide` for each level, and converting back.
    pub fn apply(&self, base_mesh: &Mesh) -> Mesh {
        if self.levels == 0 {
            return base_mesh.clone();
        }

        let mut edit_mesh = EditableMesh::from_mesh(base_mesh);
        for _ in 0..self.levels.min(4) {
            edit_mesh.select_all(EditSelectionMode::Face);
            edit_mesh.subdivide();
        }

        edit_mesh.to_render_mesh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::generate_cube;

    #[test]
    fn test_subdivision_modifier() {
        let cube = generate_cube(1.0);
        let initial_triangles = cube.indices.len() / 3;

        let subdiv = SubdivisionModifier::new(1);
        let result = subdiv.apply(&cube);

        let new_triangles = result.indices.len() / 3;
        assert!(new_triangles > initial_triangles);
    }
}
