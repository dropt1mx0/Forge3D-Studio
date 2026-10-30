use std::f32::consts::PI;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a triangulated 3D cylinder centered at the origin, aligned along the Y axis.
/// `radius`: radius of base and top caps.
/// `height`: total height of the cylinder along Y.
/// `segments`: number of radial subdivisions (minimum 3).
pub fn generate_cylinder(radius: f32, height: f32, segments: u32) -> Mesh {
    let segments = segments.max(3);
    let half_h = height * 0.5;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // 1. Side vertices (two rings: top and bottom with outward normals)
    let side_start_idx = vertices.len() as u32;
    for i in 0..=segments {
        let u = i as f32 / segments as f32;
        let theta = u * 2.0 * PI;
        let cos = theta.cos();
        let sin = theta.sin();

        let norm = [cos, 0.0, sin];
        // Top ring vertex
        vertices.push(Vertex::new(
            [radius * cos, half_h, radius * sin],
            norm,
            [u, 0.0],
            [1.0, 1.0, 1.0, 1.0],
        ));
        // Bottom ring vertex
        vertices.push(Vertex::new(
            [radius * cos, -half_h, radius * sin],
            norm,
            [u, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ));
    }

    // Side indices (quads between top and bottom rings)
    for i in 0..segments {
        let top0 = side_start_idx + i * 2;
        let bot0 = top0 + 1;
        let top1 = side_start_idx + (i + 1) * 2;
        let bot1 = top1 + 1;

        indices.push(top0);
        indices.push(bot0);
        indices.push(top1);

        indices.push(top1);
        indices.push(bot0);
        indices.push(bot1);
    }

    // 2. Top cap
    let top_center_idx = vertices.len() as u32;
    vertices.push(Vertex::new(
        [0.0, half_h, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
        [1.0, 1.0, 1.0, 1.0],
    ));

    let top_rim_start = vertices.len() as u32;
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * 2.0 * PI;
        let cos = theta.cos();
        let sin = theta.sin();
        vertices.push(Vertex::new(
            [radius * cos, half_h, radius * sin],
            [0.0, 1.0, 0.0],
            [0.5 + 0.5 * cos, 0.5 + 0.5 * sin],
            [1.0, 1.0, 1.0, 1.0],
        ));
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(top_center_idx);
        indices.push(top_rim_start + i);
        indices.push(top_rim_start + next);
    }

    // 3. Bottom cap
    let bot_center_idx = vertices.len() as u32;
    vertices.push(Vertex::new(
        [0.0, -half_h, 0.0],
        [0.0, -1.0, 0.0],
        [0.5, 0.5],
        [1.0, 1.0, 1.0, 1.0],
    ));

    let bot_rim_start = vertices.len() as u32;
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * 2.0 * PI;
        let cos = theta.cos();
        let sin = theta.sin();
        vertices.push(Vertex::new(
            [radius * cos, -half_h, radius * sin],
            [0.0, -1.0, 0.0],
            [0.5 + 0.5 * cos, 0.5 + 0.5 * sin],
            [1.0, 1.0, 1.0, 1.0],
        ));
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(bot_center_idx);
        indices.push(bot_rim_start + next);
        indices.push(bot_rim_start + i);
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_cylinder_generation_validity() {
        let mesh = generate_cylinder(1.0, 2.0, 16);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());
    }
}
