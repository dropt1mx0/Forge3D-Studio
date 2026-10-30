use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a 3D box/cube centered at origin with side length `size`.
///
/// Produces 24 vertices (4 per face) with explicit flat normals and (0..1) UV coordinates,
/// and 36 indices forming 12 triangles.
pub fn generate_cube(size: f32) -> Mesh {
    let half = size * 0.5;
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    // Helper macro to push a quad face (4 vertices, 6 indices)
    let mut add_face =
        |p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], normal: [f32; 3]| {
            let base_idx = vertices.len() as u32;

            vertices.push(Vertex::with_pos_norm_uv(p0, normal, [0.0, 0.0]));
            vertices.push(Vertex::with_pos_norm_uv(p1, normal, [1.0, 0.0]));
            vertices.push(Vertex::with_pos_norm_uv(p2, normal, [1.0, 1.0]));
            vertices.push(Vertex::with_pos_norm_uv(p3, normal, [0.0, 1.0]));

            // Triangle 1: 0, 1, 2
            indices.push(base_idx);
            indices.push(base_idx + 1);
            indices.push(base_idx + 2);

            // Triangle 2: 0, 2, 3
            indices.push(base_idx);
            indices.push(base_idx + 2);
            indices.push(base_idx + 3);
        };

    // Front (+Z)
    add_face(
        [-half, -half, half],
        [half, -half, half],
        [half, half, half],
        [-half, half, half],
        [0.0, 0.0, 1.0],
    );

    // Back (-Z)
    add_face(
        [half, -half, -half],
        [-half, -half, -half],
        [-half, half, -half],
        [half, half, -half],
        [0.0, 0.0, -1.0],
    );

    // Top (+Y)
    add_face(
        [-half, half, half],
        [half, half, half],
        [half, half, -half],
        [-half, half, -half],
        [0.0, 1.0, 0.0],
    );

    // Bottom (-Y)
    add_face(
        [-half, -half, -half],
        [half, -half, -half],
        [half, -half, half],
        [-half, -half, half],
        [0.0, -1.0, 0.0],
    );

    // Right (+X)
    add_face(
        [half, -half, half],
        [half, -half, -half],
        [half, half, -half],
        [half, half, half],
        [1.0, 0.0, 0.0],
    );

    // Left (-X)
    add_face(
        [-half, -half, -half],
        [-half, -half, half],
        [-half, half, half],
        [-half, half, -half],
        [-1.0, 0.0, 0.0],
    );

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_generation() {
        let cube = generate_cube(2.0);
        assert_eq!(cube.vertices.len(), 24);
        assert_eq!(cube.indices.len(), 36);
        assert!(cube.validate().is_ok());
        assert_eq!(cube.aabb.min, glam::Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(cube.aabb.max, glam::Vec3::new(1.0, 1.0, 1.0));
    }
}
