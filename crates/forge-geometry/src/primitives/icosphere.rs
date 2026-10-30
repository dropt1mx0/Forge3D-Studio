use std::collections::HashMap;

use forge_math::Vec3;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Generates an icosphere by recursively subdividing an icosahedron.
/// `subdivisions`: number of subdivision iterations (0..=5).
pub fn generate_icosphere(radius: f32, subdivisions: u32) -> Mesh {
    let subdivisions = subdivisions.min(5);

    // Golden ratio
    let t = (1.0 + 5.0_f32.sqrt()) / 2.0;

    let mut base_positions = vec![
        Vec3::new(-1.0, t, 0.0).normalize(),
        Vec3::new(1.0, t, 0.0).normalize(),
        Vec3::new(-1.0, -t, 0.0).normalize(),
        Vec3::new(1.0, -t, 0.0).normalize(),
        Vec3::new(0.0, -1.0, t).normalize(),
        Vec3::new(0.0, 1.0, t).normalize(),
        Vec3::new(0.0, -1.0, -t).normalize(),
        Vec3::new(0.0, 1.0, -t).normalize(),
        Vec3::new(t, 0.0, -1.0).normalize(),
        Vec3::new(t, 0.0, 1.0).normalize(),
        Vec3::new(-t, 0.0, -1.0).normalize(),
        Vec3::new(-t, 0.0, 1.0).normalize(),
    ];

    let mut faces = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    let mut midpoint_cache = HashMap::new();

    let mut get_midpoint = |p1: usize, p2: usize, positions: &mut Vec<Vec3>| -> usize {
        let key = if p1 < p2 { (p1, p2) } else { (p2, p1) };
        if let Some(&idx) = midpoint_cache.get(&key) {
            return idx;
        }

        let mid = (positions[p1] + positions[p2]).normalize();
        let idx = positions.len();
        positions.push(mid);
        midpoint_cache.insert(key, idx);
        idx
    };

    for _ in 0..subdivisions {
        let mut new_faces = Vec::with_capacity(faces.len() * 4);
        for face in &faces {
            let a = get_midpoint(face[0], face[1], &mut base_positions);
            let b = get_midpoint(face[1], face[2], &mut base_positions);
            let c = get_midpoint(face[2], face[0], &mut base_positions);

            new_faces.push([face[0], a, c]);
            new_faces.push([face[1], b, a]);
            new_faces.push([face[2], c, b]);
            new_faces.push([a, b, c]);
        }
        faces = new_faces;
    }

    let mut vertices = Vec::with_capacity(base_positions.len());
    for p in &base_positions {
        let norm = *p;
        let pos = norm * radius;
        let u = 0.5 + norm.z.atan2(norm.x) / (2.0 * std::f32::consts::PI);
        let v = 0.5 - norm.y.asin() / std::f32::consts::PI;

        vertices.push(Vertex::new(
            pos.to_array(),
            norm.to_array(),
            [u, v],
            [1.0, 1.0, 1.0, 1.0],
        ));
    }

    let mut indices = Vec::with_capacity(faces.len() * 3);
    for face in &faces {
        indices.push(face[0] as u32);
        indices.push(face[1] as u32);
        indices.push(face[2] as u32);
    }

    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_mesh_data;

    #[test]
    fn test_icosphere_generation_validity() {
        let mesh = generate_icosphere(1.0, 2);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(validate_mesh_data(&mesh.vertices, &mesh.indices).is_ok());
    }
}
