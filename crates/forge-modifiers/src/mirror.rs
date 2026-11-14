use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Non-destructive Mirror modifier reflecting geometry across X, Y, or Z axes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MirrorModifier {
    pub axis_x: bool,
    pub axis_y: bool,
    pub axis_z: bool,
    pub merge: bool,
    pub merge_threshold: f32,
}

impl Default for MirrorModifier {
    fn default() -> Self {
        Self {
            axis_x: true,
            axis_y: false,
            axis_z: false,
            merge: true,
            merge_threshold: 0.001,
        }
    }
}

impl MirrorModifier {
    pub fn new_x() -> Self {
        Self::default()
    }

    /// Evaluates the mirror modifier on a given base mesh.
    pub fn apply(&self, base_mesh: &Mesh) -> Mesh {
        if !self.axis_x && !self.axis_y && !self.axis_z {
            return base_mesh.clone();
        }

        let mut out_vertices = base_mesh.vertices.clone();
        let mut out_indices = base_mesh.indices.clone();

        let scale = Vec3::new(
            if self.axis_x { -1.0 } else { 1.0 },
            if self.axis_y { -1.0 } else { 1.0 },
            if self.axis_z { -1.0 } else { 1.0 },
        );

        // Mirror vertices
        let mirrored_start = out_vertices.len() as u32;
        for v in &base_mesh.vertices {
            let pos = Vec3::from_array(v.position) * scale;
            let norm = Vec3::from_array(v.normal) * scale;

            out_vertices.push(Vertex::new(pos.to_array(), norm.to_array(), v.uv, v.color));
        }

        // Mirror indices with reversed winding order (i0, i2, i1) to keep outward face normals
        for chunk in base_mesh.indices.as_chunks::<3>().0 {
            let i0 = mirrored_start + chunk[0];
            let i1 = mirrored_start + chunk[1];
            let i2 = mirrored_start + chunk[2];

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
    use forge_geometry::primitives::generate_cube;

    #[test]
    fn test_mirror_modifier_doubles_cube() {
        let cube = generate_cube(1.0);
        let initial_verts = cube.vertices.len();
        let initial_indices = cube.indices.len();

        let mirror = MirrorModifier::new_x();
        let mirrored = mirror.apply(&cube);

        assert_eq!(mirrored.vertices.len(), initial_verts * 2);
        assert_eq!(mirrored.indices.len(), initial_indices * 2);
    }
}
