//! UV Unwrapping, projection algorithms, and island packing tools.

use forge_math::{Vec2, Vec3};
use std::collections::{HashSet, VecDeque};

use crate::edit_mesh::EditableMesh;

/// Configuration options for Smart UV Projection.
#[derive(Debug, Clone)]
pub struct SmartProjectOptions {
    /// Angle limit in radians between adjacent faces to consider them part of the same island.
    pub angle_limit_rad: f32,
    /// Margin between packed UV islands in normalized [0, 1] texture coordinates.
    pub margin: f32,
}

impl Default for SmartProjectOptions {
    fn default() -> Self {
        Self {
            angle_limit_rad: 66.0_f32.to_radians(), // 66 degrees standard
            margin: 0.02,
        }
    }
}

/// Computes Cube / Box UV projection across all vertices of an EditableMesh.
pub fn box_project_uvs(mesh: &mut EditableMesh, scale: f32) {
    if mesh.vertices.is_empty() {
        return;
    }

    let mut min_p = Vec3::splat(f32::INFINITY);
    let mut max_p = Vec3::splat(f32::NEG_INFINITY);
    for v in &mesh.vertices {
        min_p = min_p.min(v.position);
        max_p = max_p.max(v.position);
    }
    let extent = (max_p - min_p).max(Vec3::splat(1e-4));

    for v in &mut mesh.vertices {
        let n = v.normal.abs();
        let p = v.position;

        let (u, v_coord) = if n.x >= n.y && n.x >= n.z {
            // Dominant X axis -> project onto YZ
            ((p.z - min_p.z) / extent.z, (p.y - min_p.y) / extent.y)
        } else if n.y >= n.x && n.y >= n.z {
            // Dominant Y axis -> project onto XZ
            ((p.x - min_p.x) / extent.x, (p.z - min_p.z) / extent.z)
        } else {
            // Dominant Z axis -> project onto XY
            ((p.x - min_p.x) / extent.x, (p.y - min_p.y) / extent.y)
        };

        v.uv = Vec2::new(u * scale, v_coord * scale);
    }
}

/// Computes Spherical UV projection centered around an origin point.
pub fn sphere_project_uvs(mesh: &mut EditableMesh, center: Vec3) {
    use std::f32::consts::PI;

    for v in &mut mesh.vertices {
        let dir = (v.position - center).normalize_or_zero();
        if dir.length_squared() < 1e-6 {
            v.uv = Vec2::new(0.5, 0.5);
            continue;
        }

        let u = 0.5 + dir.z.atan2(dir.x) / (2.0 * PI);
        let v_coord = 0.5 - dir.y.clamp(-1.0, 1.0).asin() / PI;

        v.uv = Vec2::new(u.clamp(0.0, 1.0), v_coord.clamp(0.0, 1.0));
    }
}

/// Computes Cylindrical UV projection around a central axis.
pub fn cylinder_project_uvs(mesh: &mut EditableMesh, center: Vec3, height: f32) {
    use std::f32::consts::PI;

    let h = height.max(1e-4);
    for v in &mut mesh.vertices {
        let rel = v.position - center;
        let u = 0.5 + rel.z.atan2(rel.x) / (2.0 * PI);
        let v_coord = (rel.y / h + 0.5).clamp(0.0, 1.0);

        v.uv = Vec2::new(u.clamp(0.0, 1.0), v_coord);
    }
}

/// Represents a clustered coplanar group of faces (UV Island) during projection.
struct UvIsland {
    face_indices: Vec<usize>,
    tangent: Vec3,
    bitangent: Vec3,
    min_2d: Vec2,
    max_2d: Vec2,
}

/// Smart UV Projection: segments mesh into planar face clusters and packs islands.
pub fn smart_project_uvs(mesh: &mut EditableMesh, options: &SmartProjectOptions) {
    if mesh.faces.is_empty() || mesh.vertices.is_empty() {
        return;
    }

    let cos_limit = options.angle_limit_rad.cos();

    // 1. Build face adjacency graph via shared edges
    let mut edge_to_faces: std::collections::HashMap<(usize, usize), Vec<usize>> =
        std::collections::HashMap::new();

    for (f_idx, face) in mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        for i in 0..n {
            let v0 = face.vertices[i];
            let v1 = face.vertices[(i + 1) % n];
            let key = (v0.min(v1), v0.max(v1));
            edge_to_faces.entry(key).or_default().push(f_idx);
        }
    }

    let mut face_adj: Vec<Vec<usize>> = vec![Vec::new(); mesh.faces.len()];
    for faces in edge_to_faces.values() {
        if faces.len() == 2 {
            face_adj[faces[0]].push(faces[1]);
            face_adj[faces[1]].push(faces[0]);
        }
    }

    // 2. Partition faces into islands based on normal similarity
    let mut visited: HashSet<usize> = HashSet::new();
    let mut islands: Vec<UvIsland> = Vec::new();

    for start_f in 0..mesh.faces.len() {
        if visited.contains(&start_f) {
            continue;
        }

        let mut island_faces = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_f);
        visited.insert(start_f);

        let base_normal = mesh.faces[start_f].normal.normalize_or_zero();
        let mut accumulated_normal = Vec3::ZERO;

        while let Some(curr) = queue.pop_front() {
            island_faces.push(curr);
            accumulated_normal += mesh.faces[curr].normal;

            for &neighbor in &face_adj[curr] {
                if !visited.contains(&neighbor) {
                    let n_norm = mesh.faces[neighbor].normal.normalize_or_zero();
                    if base_normal.dot(n_norm) >= cos_limit {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        let avg_normal = accumulated_normal.normalize_or_zero();
        let normal = if avg_normal.length_squared() > 1e-5 {
            avg_normal
        } else {
            Vec3::Y
        };

        // Construct orthonormal basis for the island
        let up = if normal.y.abs() < 0.99 {
            Vec3::Y
        } else {
            Vec3::Z
        };
        let tangent = normal.cross(up).normalize_or_zero();
        let bitangent = normal.cross(tangent).normalize_or_zero();

        islands.push(UvIsland {
            face_indices: island_faces,
            tangent,
            bitangent,
            min_2d: Vec2::splat(f32::INFINITY),
            max_2d: Vec2::splat(f32::NEG_INFINITY),
        });
    }

    // 3. Project each island into local 2D coordinates
    for island in &mut islands {
        let mut min_2d = Vec2::splat(f32::INFINITY);
        let mut max_2d = Vec2::splat(f32::NEG_INFINITY);

        for &f_idx in &island.face_indices {
            for &v_idx in &mesh.faces[f_idx].vertices {
                let p = mesh.vertices[v_idx].position;
                let u = p.dot(island.tangent);
                let v = p.dot(island.bitangent);
                min_2d = min_2d.min(Vec2::new(u, v));
                max_2d = max_2d.max(Vec2::new(u, v));
            }
        }

        island.min_2d = min_2d;
        island.max_2d = max_2d;
    }

    // 4. Pack islands into [0, 1] x [0, 1] UV square
    let num_islands = islands.len();
    if num_islands == 0 {
        return;
    }

    let cols = (num_islands as f32).sqrt().ceil() as usize;
    let rows = num_islands.div_ceil(cols);

    let margin = options.margin.clamp(0.001, 0.2);
    let cell_w = (1.0 - margin * (cols + 1) as f32) / cols as f32;
    let cell_h = (1.0 - margin * (rows + 1) as f32) / rows as f32;

    for (idx, island) in islands.iter().enumerate() {
        let col = idx % cols;
        let row = idx / cols;

        let cell_min_u = margin + col as f32 * (cell_w + margin);
        let cell_min_v = margin + row as f32 * (cell_h + margin);

        let size = (island.max_2d - island.min_2d).max(Vec2::splat(1e-4));
        let scale_u = cell_w / size.x;
        let scale_v = cell_h / size.y;
        let scale = scale_u.min(scale_v);

        let offset_u = cell_min_u + (cell_w - size.x * scale) * 0.5;
        let offset_v = cell_min_v + (cell_h - size.y * scale) * 0.5;

        for &f_idx in &island.face_indices {
            for &v_idx in &mesh.faces[f_idx].vertices {
                let p = mesh.vertices[v_idx].position;
                let raw_u = p.dot(island.tangent);
                let raw_v = p.dot(island.bitangent);

                let final_u = offset_u + (raw_u - island.min_2d.x) * scale;
                let final_v = offset_v + (raw_v - island.min_2d.y) * scale;

                mesh.vertices[v_idx].uv = Vec2::new(final_u, final_v);
            }
        }
    }
}

/// Normalizes and packs existing UV coordinates within [margin, 1 - margin].
pub fn pack_uv_islands(mesh: &mut EditableMesh, margin: f32) {
    if mesh.vertices.is_empty() {
        return;
    }

    let mut min_uv = Vec2::splat(f32::INFINITY);
    let mut max_uv = Vec2::splat(f32::NEG_INFINITY);

    for v in &mesh.vertices {
        min_uv = min_uv.min(v.uv);
        max_uv = max_uv.max(v.uv);
    }

    let size = (max_uv - min_uv).max(Vec2::splat(1e-4));
    let usable = (1.0 - 2.0 * margin).max(0.1);

    let scale_u = usable / size.x;
    let scale_v = usable / size.y;
    let scale = scale_u.min(scale_v);

    let offset_u = margin + (usable - size.x * scale) * 0.5;
    let offset_v = margin + (usable - size.y * scale) * 0.5;

    for v in &mut mesh.vertices {
        let u = offset_u + (v.uv.x - min_uv.x) * scale;
        let v_coord = offset_v + (v.uv.y - min_uv.y) * scale;
        v.uv = Vec2::new(u, v_coord);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_mesh::EditableMesh;
    use crate::primitives::generate_cube;

    #[test]
    fn test_box_project_uvs() {
        let cube = generate_cube(2.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube);

        box_project_uvs(&mut edit_mesh, 1.0);

        for v in &edit_mesh.vertices {
            assert!(
                v.uv.x >= -1e-4 && v.uv.x <= 1.0 + 1e-4,
                "U must be within [0, 1]"
            );
            assert!(
                v.uv.y >= -1e-4 && v.uv.y <= 1.0 + 1e-4,
                "V must be within [0, 1]"
            );
        }
    }

    #[test]
    fn test_sphere_project_uvs() {
        let cube = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube);

        sphere_project_uvs(&mut edit_mesh, Vec3::ZERO);

        for v in &edit_mesh.vertices {
            assert!(
                v.uv.x >= 0.0 && v.uv.x <= 1.0,
                "Spherical U must be in [0, 1]"
            );
            assert!(
                v.uv.y >= 0.0 && v.uv.y <= 1.0,
                "Spherical V must be in [0, 1]"
            );
        }
    }

    #[test]
    fn test_smart_project_cube() {
        let cube = generate_cube(2.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube);

        smart_project_uvs(&mut edit_mesh, &SmartProjectOptions::default());

        // All UVs must lie in [0, 1]
        for v in &edit_mesh.vertices {
            assert!(v.uv.x >= 0.0 && v.uv.x <= 1.0, "Smart U in [0, 1]");
            assert!(v.uv.y >= 0.0 && v.uv.y <= 1.0, "Smart V in [0, 1]");
        }
    }

    #[test]
    fn test_pack_uv_islands() {
        let cube = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube);

        // Distort UVs outside [0, 1]
        for v in &mut edit_mesh.vertices {
            v.uv = v.uv * 10.0 + Vec2::new(5.0, -3.0);
        }

        pack_uv_islands(&mut edit_mesh, 0.05);

        for v in &edit_mesh.vertices {
            assert!(v.uv.x >= 0.04 && v.uv.x <= 0.96);
            assert!(v.uv.y >= 0.04 && v.uv.y <= 0.96);
        }
    }
}
