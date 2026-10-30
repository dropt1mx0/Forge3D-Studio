use std::f32::consts::PI;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a UV sphere centered at origin with given radius, sectors (longitude), and stacks (latitude).
pub fn generate_uv_sphere(radius: f32, sectors: u32, stacks: u32) -> Mesh {
    let sectors = sectors.max(3);
    let stacks = stacks.max(2);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=stacks {
        let v = i as f32 / stacks as f32;
        let phi = v * PI; // 0 (top pole) to PI (bottom pole)

        for j in 0..=sectors {
            let u = j as f32 / sectors as f32;
            let theta = u * 2.0 * PI; // 0 to 2*PI

            let x = theta.cos() * phi.sin();
            let y = phi.cos();
            let z = theta.sin() * phi.sin();

            let position = [radius * x, radius * y, radius * z];
            let normal = [x, y, z];
            let uv = [u, v];

            vertices.push(Vertex::with_pos_norm_uv(position, normal, uv));
        }
    }

    for i in 0..stacks {
        let k1 = i * (sectors + 1);
        let k2 = k1 + sectors + 1;

        for j in 0..sectors {
            if i != 0 {
                indices.push(k1 + j);
                indices.push(k2 + j);
                indices.push(k1 + j + 1);
            }

            if i != (stacks - 1) {
                indices.push(k1 + j + 1);
                indices.push(k2 + j);
                indices.push(k2 + j + 1);
            }
        }
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_generation() {
        let sphere = generate_uv_sphere(1.0, 16, 8);
        assert!(!sphere.vertices.is_empty());
        assert!(!sphere.indices.is_empty());
        assert!(sphere.validate().is_ok());
    }
}
