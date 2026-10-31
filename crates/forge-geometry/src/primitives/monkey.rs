use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates a clean-room procedural stylized monkey / mascot mesh.
///
/// Designed as a recognizable, asymmetric-friendly test geometry featuring:
/// - Cranial dome (skull box)
/// - Protruding curved brow ridge
/// - Faceted ocular eyes / sockets
/// - Forward-projecting snout / muzzle
/// - Angled lateral ears
///
/// Scaled uniformly by `scale` (default ~1.0m bounding scale).
pub fn generate_monkey(scale: f32) -> Mesh {
    let s = scale.max(0.01);

    // Helper macro to define symmetric pairs and center vertices
    // Coordinate conventions: Right-handed, +Y Up, +Z Forward (Towards front of face), +X Right (User's right = Monkey's right)
    let raw_positions: Vec<[f32; 3]> = vec![
        // --- Cranium / Skull (0..8) ---
        [0.0, 0.8 * s, 0.1 * s],         // 0: Crown center
        [0.4 * s, 0.7 * s, -0.2 * s],    // 1: Crown right
        [-0.4 * s, 0.7 * s, -0.2 * s],   // 2: Crown left
        [0.0, 0.5 * s, -0.6 * s],        // 3: Occiput (back center)
        [0.55 * s, 0.3 * s, -0.4 * s],   // 4: Skull side right
        [-0.55 * s, 0.3 * s, -0.4 * s],  // 5: Skull side left
        [0.0, -0.2 * s, -0.4 * s],       // 6: Skull base (nape)
        [0.35 * s, -0.1 * s, -0.3 * s],  // 7: Base right
        [-0.35 * s, -0.1 * s, -0.3 * s], // 8: Base left
        // --- Brow Ridge (9..14) ---
        [0.0, 0.55 * s, 0.5 * s],       // 9: Brow center
        [0.35 * s, 0.5 * s, 0.48 * s],  // 10: Brow right inner
        [-0.35 * s, 0.5 * s, 0.48 * s], // 11: Brow left inner
        [0.6 * s, 0.4 * s, 0.35 * s],   // 12: Brow right temple
        [-0.6 * s, 0.4 * s, 0.35 * s],  // 13: Brow left temple
        [0.0, 0.35 * s, 0.4 * s],       // 14: Glabella (between eyes)
        // --- Eyes (15..20) ---
        [0.25 * s, 0.3 * s, 0.45 * s],   // 15: Right eye center
        [0.25 * s, 0.38 * s, 0.35 * s],  // 16: Right eye top
        [0.38 * s, 0.28 * s, 0.35 * s],  // 17: Right eye outer
        [-0.25 * s, 0.3 * s, 0.45 * s],  // 18: Left eye center
        [-0.25 * s, 0.38 * s, 0.35 * s], // 19: Left eye top
        [-0.38 * s, 0.28 * s, 0.35 * s], // 20: Left eye outer
        // --- Muzzle / Snout (21..28) ---
        [0.0, 0.15 * s, 0.75 * s],       // 21: Nose tip
        [0.2 * s, 0.12 * s, 0.65 * s],   // 22: Muzzle upper right
        [-0.2 * s, 0.12 * s, 0.65 * s],  // 23: Muzzle upper left
        [0.0, -0.1 * s, 0.7 * s],        // 24: Mouth center
        [0.22 * s, -0.1 * s, 0.55 * s],  // 25: Mouth corner right
        [-0.22 * s, -0.1 * s, 0.55 * s], // 26: Mouth corner left
        [0.0, -0.35 * s, 0.5 * s],       // 27: Chin center
        [0.15 * s, -0.32 * s, 0.4 * s],  // 28: Chin right
        [-0.15 * s, -0.32 * s, 0.4 * s], // 29: Chin left
        // --- Cheeks & Jowls (30..33) ---
        [0.45 * s, 0.05 * s, 0.3 * s],   // 30: Cheek right
        [-0.45 * s, 0.05 * s, 0.3 * s],  // 31: Cheek left
        [0.35 * s, -0.25 * s, 0.1 * s],  // 32: Jaw right
        [-0.35 * s, -0.25 * s, 0.1 * s], // 33: Jaw left
        // --- Ears (34..41) ---
        [0.7 * s, 0.35 * s, -0.05 * s],    // 34: Right ear top
        [0.9 * s, 0.2 * s, -0.1 * s],      // 35: Right ear outer tip
        [0.75 * s, -0.05 * s, -0.15 * s],  // 36: Right ear bottom
        [0.6 * s, 0.15 * s, -0.1 * s],     // 37: Right ear concha (inner base)
        [-0.7 * s, 0.35 * s, -0.05 * s],   // 38: Left ear top
        [-0.9 * s, 0.2 * s, -0.1 * s],     // 39: Left ear outer tip
        [-0.75 * s, -0.05 * s, -0.15 * s], // 40: Left ear bottom
        [-0.6 * s, 0.15 * s, -0.1 * s],    // 41: Left ear concha (inner base)
    ];

    // Face definitions (triangles with counter-clockwise outward winding)
    #[rustfmt::skip]
    let raw_triangles: Vec<[u32; 3]> = vec![
        // Cranium top / forehead
        [0, 10, 9],
        [0, 9, 11],
        [0, 12, 10],
        [0, 11, 13],
        [0, 1, 12],
        [0, 13, 2],
        // Cranium back
        [0, 3, 1],
        [0, 2, 3],
        [1, 3, 4],
        [2, 5, 3],
        [3, 6, 4],
        [3, 5, 6],
        [4, 6, 7],
        [5, 8, 6],

        // Brow to Eye socket & Glabella
        [9, 10, 14],
        [9, 14, 11],
        [10, 16, 14],
        [11, 14, 19],
        [10, 12, 17],
        [10, 17, 16],
        [11, 19, 20],
        [11, 20, 13],

        // Right eye dome
        [16, 17, 15],
        [17, 30, 15],
        [14, 16, 15],
        [14, 15, 21],
        [15, 22, 21],
        [15, 30, 22],

        // Left eye dome
        [19, 18, 20],
        [20, 18, 31],
        [14, 18, 19],
        [14, 21, 18],
        [18, 21, 23],
        [18, 23, 31],

        // Muzzle / Nose to Mouth
        [21, 22, 24],
        [21, 24, 23],
        [22, 25, 24],
        [23, 24, 26],

        // Mouth to Chin
        [24, 25, 27],
        [24, 27, 26],
        [25, 28, 27],
        [26, 27, 29],

        // Cheeks and Jaw
        [12, 4, 30],
        [13, 31, 5],
        [22, 30, 25],
        [23, 26, 31],
        [25, 30, 32],
        [26, 33, 31],
        [25, 32, 28],
        [26, 29, 33],
        [28, 32, 27],
        [29, 27, 33],

        // Jaw to Skull Base
        [30, 4, 7],
        [30, 7, 32],
        [31, 8, 5],
        [31, 33, 8],
        [32, 7, 6],
        [33, 6, 8],
        [32, 6, 27],
        [33, 27, 6],

        // Right Ear (Double-sided for volume and visual presence)
        [4, 34, 37],
        [34, 35, 37],
        [35, 36, 37],
        [36, 7, 37],
        // Right Ear back
        [4, 37, 34],
        [34, 37, 35],
        [35, 37, 36],
        [36, 37, 7],

        // Left Ear
        [5, 41, 38],
        [38, 41, 39],
        [39, 41, 40],
        [40, 41, 8],
        // Left Ear back
        [5, 38, 41],
        [38, 39, 41],
        [39, 40, 41],
        [40, 8, 41],
    ];

    // Compute smooth outward vertex normals via face normal accumulation
    let mut accumulated_normals = vec![[0.0_f32; 3]; raw_positions.len()];
    for tri in &raw_triangles {
        let p0 = raw_positions[tri[0] as usize];
        let p1 = raw_positions[tri[1] as usize];
        let p2 = raw_positions[tri[2] as usize];

        let v0 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v1 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

        let fnorm = [
            v0[1] * v1[2] - v0[2] * v1[1],
            v0[2] * v1[0] - v0[0] * v1[2],
            v0[0] * v1[1] - v0[1] * v1[0],
        ];

        for &idx in tri {
            let n = &mut accumulated_normals[idx as usize];
            n[0] += fnorm[0];
            n[1] += fnorm[1];
            n[2] += fnorm[2];
        }
    }

    let mut vertices = Vec::with_capacity(raw_positions.len());
    for (i, pos) in raw_positions.into_iter().enumerate() {
        let n = accumulated_normals[i];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        let normal = if len > 1e-6 {
            [n[0] / len, n[1] / len, n[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };

        // Spherical UV projection
        let u = 0.5 + pos[0].atan2(pos[2]) / (2.0 * std::f32::consts::PI);
        let v = 0.5 + (pos[1] / s).clamp(-1.0, 1.0) * 0.5;

        vertices.push(Vertex::new(pos, normal, [u, v], [0.85, 0.55, 0.25, 1.0]));
    }

    let mut indices = Vec::with_capacity(raw_triangles.len() * 3);
    for tri in raw_triangles {
        indices.push(tri[0]);
        indices.push(tri[1]);
        indices.push(tri[2]);
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_monkey_generation_validity() {
        let mesh = generate_monkey(1.0);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);

        // Verify geometry invariants (no NaNs, no out-of-bounds indices, no degenerate triangles)
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());

        // Verify bounding box scale
        let aabb = mesh.aabb;
        assert!(aabb.size().x > 1.5); // Ears spread laterally (>1.5m)
        assert!(aabb.size().y > 0.8);
        assert!(aabb.size().z > 0.8);
    }
}
