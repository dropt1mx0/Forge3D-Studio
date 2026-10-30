use std::f32::consts::PI;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a triangulated 3D cone with base on -height/2 and apex on +height/2 along Y.
pub fn generate_cone(radius: f32, height: f32, segments: u32) -> Mesh {
    let segments = segments.max(3);
    let half_h = height * 0.5;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let slope = radius / height;

    // 1. Side triangles
    for i in 0..segments {
        let u0 = i as f32 / segments as f32;
        let u1 = (i + 1) as f32 / segments as f32;
        let theta0 = u0 * 2.0 * PI;
        let theta1 = u1 * 2.0 * PI;

        let mid_theta = (theta0 + theta1) * 0.5;
        let apex_norm = [mid_theta.cos(), slope, mid_theta.sin()];

        let v_apex = vertices.len() as u32;
        vertices.push(Vertex::new(
            [0.0, half_h, 0.0],
            apex_norm,
            [(u0 + u1) * 0.5, 0.0],
            [1.0, 1.0, 1.0, 1.0],
        ));

        let v0 = vertices.len() as u32;
        vertices.push(Vertex::new(
            [radius * theta0.cos(), -half_h, radius * theta0.sin()],
            [theta0.cos(), slope, theta0.sin()],
            [u0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ));

        let v1 = vertices.len() as u32;
        vertices.push(Vertex::new(
            [radius * theta1.cos(), -half_h, radius * theta1.sin()],
            [theta1.cos(), slope, theta1.sin()],
            [u1, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ));

        indices.push(v_apex);
        indices.push(v0);
        indices.push(v1);
    }

    // 2. Base cap
    let base_center = vertices.len() as u32;
    vertices.push(Vertex::new(
        [0.0, -half_h, 0.0],
        [0.0, -1.0, 0.0],
        [0.5, 0.5],
        [1.0, 1.0, 1.0, 1.0],
    ));

    let rim_start = vertices.len() as u32;
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
        indices.push(base_center);
        indices.push(rim_start + next);
        indices.push(rim_start + i);
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_cone_generation_validity() {
        let mesh = generate_cone(1.0, 2.0, 16);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());
    }
}
