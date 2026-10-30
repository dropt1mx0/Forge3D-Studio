use std::f32::consts::PI;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a triangulated 3D torus in the XZ plane centered at the origin.
/// `major_radius`: distance from tube center to torus center.
/// `minor_radius`: radius of the tube.
/// `major_segments`: radial subdivisions around the main ring.
/// `minor_segments`: radial subdivisions around the tube.
pub fn generate_torus(
    major_radius: f32,
    minor_radius: f32,
    major_segments: u32,
    minor_segments: u32,
) -> Mesh {
    let major_segments = major_segments.max(3);
    let minor_segments = minor_segments.max(3);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=major_segments {
        let u = i as f32 / major_segments as f32;
        let theta = u * 2.0 * PI;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        for j in 0..=minor_segments {
            let v = j as f32 / minor_segments as f32;
            let phi = v * 2.0 * PI;
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            let x = (major_radius + minor_radius * cos_phi) * cos_theta;
            let y = minor_radius * sin_phi;
            let z = (major_radius + minor_radius * cos_phi) * sin_theta;

            let nx = cos_phi * cos_theta;
            let ny = sin_phi;
            let nz = cos_phi * sin_theta;

            vertices.push(Vertex::new(
                [x, y, z],
                [nx, ny, nz],
                [u, v],
                [1.0, 1.0, 1.0, 1.0],
            ));
        }
    }

    let stride = minor_segments + 1;
    for i in 0..major_segments {
        for j in 0..minor_segments {
            let first = i * stride + j;
            let second = (i + 1) * stride + j;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_torus_generation_validity() {
        let mesh = generate_torus(1.5, 0.4, 16, 12);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());
    }
}
