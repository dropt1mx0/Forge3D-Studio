use std::f32::consts::PI;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a triangulated 2D circle in the XZ plane centered at the origin with normal +Y.
pub fn generate_circle(radius: f32, segments: u32) -> Mesh {
    let segments = segments.max(3);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let center_idx = 0;
    vertices.push(Vertex::new(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
        [1.0, 1.0, 1.0, 1.0],
    ));

    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * 2.0 * PI;
        let cos = theta.cos();
        let sin = theta.sin();

        vertices.push(Vertex::new(
            [radius * cos, 0.0, radius * sin],
            [0.0, 1.0, 0.0],
            [0.5 + 0.5 * cos, 0.5 + 0.5 * sin],
            [1.0, 1.0, 1.0, 1.0],
        ));
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(center_idx);
        indices.push(1 + i);
        indices.push(1 + next);
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_circle_generation_validity() {
        let mesh = generate_circle(1.0, 16);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());
    }
}
