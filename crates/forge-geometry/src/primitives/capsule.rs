use std::f32::consts::PI;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a triangulated 3D capsule centered at origin, aligned along the Y axis.
///
/// A capsule consists of a cylindrical body with hemispherical caps on both ends.
/// - `radius`: radius of the capsule body and end caps.
/// - `cylinder_height`: height of the middle cylindrical section. Total capsule height is `cylinder_height + 2.0 * radius`.
/// - `sectors`: number of radial subdivisions around the Y axis (minimum 3).
/// - `rings`: number of latitude rings per hemisphere (minimum 2).
pub fn generate_capsule(radius: f32, cylinder_height: f32, sectors: u32, rings: u32) -> Mesh {
    let sectors = sectors.max(3);
    let rings = rings.max(2);
    let half_h = cylinder_height.max(0.0) * 0.5;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Total rings:
    // Top hemisphere: ring 0 (pole) to ring `rings` (equator at +half_h)
    // Bottom hemisphere: ring `rings` (equator at -half_h) to ring `2*rings` (south pole)

    // 1. Top hemisphere vertices
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let phi = v * (PI * 0.5); // 0 (north pole) to PI/2 (equator)

        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        let y = half_h + radius * cos_phi;

        for s in 0..=sectors {
            let u = s as f32 / sectors as f32;
            let theta = u * 2.0 * PI;
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            let x = radius * sin_phi * cos_theta;
            let z = radius * sin_phi * sin_theta;

            let nx = sin_phi * cos_theta;
            let ny = cos_phi;
            let nz = sin_phi * sin_theta;

            vertices.push(Vertex::with_pos_norm_uv(
                [x, y, z],
                [nx, ny, nz],
                [u, v * 0.5],
            ));
        }
    }

    // 2. Bottom hemisphere vertices
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let phi = (PI * 0.5) + v * (PI * 0.5); // PI/2 (equator) to PI (south pole)

        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        let y = -half_h + radius * cos_phi;

        for s in 0..=sectors {
            let u = s as f32 / sectors as f32;
            let theta = u * 2.0 * PI;
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            let x = radius * sin_phi * cos_theta;
            let z = radius * sin_phi * sin_theta;

            let nx = sin_phi * cos_theta;
            let ny = cos_phi;
            let nz = sin_phi * sin_theta;

            vertices.push(Vertex::with_pos_norm_uv(
                [x, y, z],
                [nx, ny, nz],
                [u, 0.5 + v * 0.5],
            ));
        }
    }

    let stride = sectors + 1;

    // 3. Connect top hemisphere (skip degenerate triangle at north pole r == 0)
    for r in 0..rings {
        let k1 = r * stride;
        let k2 = k1 + stride;

        for s in 0..sectors {
            if r != 0 {
                indices.push(k1 + s);
                indices.push(k2 + s);
                indices.push(k1 + s + 1);
            }

            indices.push(k1 + s + 1);
            indices.push(k2 + s);
            indices.push(k2 + s + 1);
        }
    }

    // 4. Connect middle cylinder wall (from top equator to bottom equator)
    let top_eq = rings * stride;
    let bot_eq = (rings + 1) * stride;

    for s in 0..sectors {
        let top0 = top_eq + s;
        let top1 = top_eq + s + 1;
        let bot0 = bot_eq + s;
        let bot1 = bot_eq + s + 1;

        indices.push(top0);
        indices.push(bot0);
        indices.push(top1);

        indices.push(top1);
        indices.push(bot0);
        indices.push(bot1);
    }

    // 5. Connect bottom hemisphere (skip degenerate triangle at south pole r == rings - 1)
    for r in 0..rings {
        let k1 = bot_eq + r * stride;
        let k2 = k1 + stride;

        for s in 0..sectors {
            indices.push(k1 + s);
            indices.push(k2 + s);
            indices.push(k1 + s + 1);

            if r != (rings - 1) {
                indices.push(k1 + s + 1);
                indices.push(k2 + s);
                indices.push(k2 + s + 1);
            }
        }
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_capsule_generation_validity() {
        let mesh = generate_capsule(0.5, 1.0, 16, 8);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);

        // Verify geometry invariants (no NaNs, no degeneracies)
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());

        // Verify bounding box height is approx cylinder_height + 2 * radius = 2.0
        let aabb = mesh.aabb;
        assert!((aabb.size().y - 2.0).abs() < 1e-3);
        assert!((aabb.size().x - 1.0).abs() < 1e-3);
    }
}
