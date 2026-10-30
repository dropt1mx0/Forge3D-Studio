use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a flat subdivided plane centered at origin lying on the XZ ground plane.
/// Normal points straight up (+Y).
pub fn generate_plane(size: f32, subdivisions: u32) -> Mesh {
    let sub = subdivisions.max(1);
    let half = size * 0.5;

    let mut vertices = Vec::with_capacity(((sub + 1) * (sub + 1)) as usize);
    let mut indices = Vec::with_capacity((sub * sub * 6) as usize);

    for z in 0..=sub {
        let v = z as f32 / sub as f32;
        let pz = -half + v * size;

        for x in 0..=sub {
            let u = x as f32 / sub as f32;
            let px = -half + u * size;

            vertices.push(Vertex::with_pos_norm_uv(
                [px, 0.0, pz],
                [0.0, 1.0, 0.0],
                [u, v],
            ));
        }
    }

    let stride = sub + 1;
    for z in 0..sub {
        for x in 0..sub {
            let row1 = z * stride + x;
            let row2 = (z + 1) * stride + x;

            // Quad split into 2 triangles
            indices.push(row1);
            indices.push(row2);
            indices.push(row1 + 1);

            indices.push(row1 + 1);
            indices.push(row2);
            indices.push(row2 + 1);
        }
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_generation() {
        let plane = generate_plane(10.0, 2);
        assert_eq!(plane.vertices.len(), 9); // (2+1)^2
        assert_eq!(plane.indices.len(), 24); // 2*2*6
        assert!(plane.validate().is_ok());
    }
}
