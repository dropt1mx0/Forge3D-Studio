//! Pure-CPU uniform Voxel Remesher for sculpting and polygon regularization.
//!
//! Evaluates the geometry surface on a uniform 3D voxel grid and extracts a clean,
//! manifold quad/triangle mesh using dual surface projection.

use std::collections::HashMap;

use forge_math::Vec3;

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Computes the closest 3D point on triangle `(a, b, c)` to query point `p`.
///
/// Implements Christer Ericson's Voronoi region method (robust against degenerate triangles).
pub fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }

    let denom = va + vb + vc;
    if denom.abs() < 1e-6 {
        return (a + b + c) / 3.0;
    }
    let v = vb / denom;
    let w = vc / denom;
    a + ab * v + ac * w
}

/// Remeshes an arbitrary triangle mesh into an evenly-spaced, uniform manifold surface.
///
/// `voxel_size`: Target edge length and cell spacing (in world/object units).
pub fn voxel_remesh(mesh: &Mesh, voxel_size: f32) -> Mesh {
    if mesh.vertices.len() < 3 || mesh.indices.len() < 3 {
        return mesh.clone();
    }

    let voxel_size = voxel_size.clamp(0.01, 2.0);

    // 1. Compute bounds and grid dimensions
    let aabb = mesh.aabb;
    let padding = voxel_size * 1.5;
    let origin = aabb.min - Vec3::splat(padding);
    let extent = aabb.max - aabb.min + Vec3::splat(padding * 2.0);

    let dim_x = ((extent.x / voxel_size).ceil() as usize).clamp(4, 96);
    let dim_y = ((extent.y / voxel_size).ceil() as usize).clamp(4, 96);
    let dim_z = ((extent.z / voxel_size).ceil() as usize).clamp(4, 96);

    let cell_to_world = |x: usize, y: usize, z: usize| -> Vec3 {
        Vec3::new(
            origin.x + (x as f32 + 0.5) * voxel_size,
            origin.y + (y as f32 + 0.5) * voxel_size,
            origin.z + (z as f32 + 0.5) * voxel_size,
        )
    };

    // 2. Track cells near surface: (x, y, z) -> (closest_point_sum, count, min_dist_sq, avg_normal)
    type SurfaceCellData = (Vec3, f32, f32, Vec3);
    let mut surface_cells: HashMap<(usize, usize, usize), SurfaceCellData> = HashMap::new();
    let threshold_sq = (voxel_size * 0.85) * (voxel_size * 0.85);

    for tri_chunk in mesh.indices.as_chunks::<3>().0 {
        let (i0, i1, i2) = (
            tri_chunk[0] as usize,
            tri_chunk[1] as usize,
            tri_chunk[2] as usize,
        );
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let p0 = Vec3::from_array(mesh.vertices[i0].position);
        let p1 = Vec3::from_array(mesh.vertices[i1].position);
        let p2 = Vec3::from_array(mesh.vertices[i2].position);

        let tri_min = p0.min(p1.min(p2)) - Vec3::splat(voxel_size);
        let tri_max = p0.max(p1.max(p2)) + Vec3::splat(voxel_size);

        let tri_normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();

        let gx0 = (((tri_min.x - origin.x) / voxel_size).floor() as isize)
            .clamp(0, dim_x as isize - 1) as usize;
        let gx1 = (((tri_max.x - origin.x) / voxel_size).ceil() as isize)
            .clamp(0, dim_x as isize - 1) as usize;
        let gy0 = (((tri_min.y - origin.y) / voxel_size).floor() as isize)
            .clamp(0, dim_y as isize - 1) as usize;
        let gy1 = (((tri_max.y - origin.y) / voxel_size).ceil() as isize)
            .clamp(0, dim_y as isize - 1) as usize;
        let gz0 = (((tri_min.z - origin.z) / voxel_size).floor() as isize)
            .clamp(0, dim_z as isize - 1) as usize;
        let gz1 = (((tri_max.z - origin.z) / voxel_size).ceil() as isize)
            .clamp(0, dim_z as isize - 1) as usize;

        for z in gz0..=gz1 {
            for y in gy0..=gy1 {
                for x in gx0..=gx1 {
                    let cell_p = cell_to_world(x, y, z);
                    let cp = closest_point_on_triangle(cell_p, p0, p1, p2);
                    let dist_sq = (cell_p - cp).length_squared();

                    if dist_sq <= threshold_sq {
                        let entry = surface_cells.entry((x, y, z)).or_insert((
                            Vec3::ZERO,
                            0.0,
                            f32::INFINITY,
                            Vec3::ZERO,
                        ));
                        entry.0 += cp;
                        entry.1 += 1.0;
                        if dist_sq < entry.2 {
                            entry.2 = dist_sq;
                        }
                        entry.3 += tri_normal;
                    }
                }
            }
        }
    }

    if surface_cells.is_empty() {
        return mesh.clone();
    }

    // 3. Generate vertices from active cells
    let mut out_vertices: Vec<Vertex> = Vec::with_capacity(surface_cells.len());
    let mut cell_to_vert_idx: HashMap<(usize, usize, usize), u32> =
        HashMap::with_capacity(surface_cells.len());

    for (&(x, y, z), &(cp_sum, count, _, norm_sum)) in &surface_cells {
        let pos = cp_sum / count.max(1.0);
        let normal = norm_sum.normalize_or_zero();

        // Box planar UV projection
        let abs_n = normal.abs();
        let uv = if abs_n.x >= abs_n.y && abs_n.x >= abs_n.z {
            [pos.y, pos.z]
        } else if abs_n.y >= abs_n.x && abs_n.y >= abs_n.z {
            [pos.x, pos.z]
        } else {
            [pos.x, pos.y]
        };

        let vert_idx = out_vertices.len() as u32;
        out_vertices.push(Vertex {
            position: pos.to_array(),
            normal: normal.to_array(),
            uv,
            color: [1.0, 1.0, 1.0, 1.0],
        });
        cell_to_vert_idx.insert((x, y, z), vert_idx);
    }

    // 4. Generate Quads (as two triangles each) across adjacent 2x2 cell faces
    let mut out_indices: Vec<u32> = Vec::new();

    let add_quad =
        |v0: u32, v1: u32, v2: u32, v3: u32, indices: &mut Vec<u32>, verts: &[Vertex]| {
            let p0 = Vec3::from_array(verts[v0 as usize].position);
            let p1 = Vec3::from_array(verts[v1 as usize].position);
            let p2 = Vec3::from_array(verts[v2 as usize].position);

            let n = (p1 - p0).cross(p2 - p0);
            let avg_n = Vec3::from_array(verts[v0 as usize].normal)
                + Vec3::from_array(verts[v1 as usize].normal)
                + Vec3::from_array(verts[v2 as usize].normal)
                + Vec3::from_array(verts[v3 as usize].normal);

            if n.dot(avg_n) >= 0.0 {
                indices.push(v0);
                indices.push(v1);
                indices.push(v2);

                indices.push(v0);
                indices.push(v2);
                indices.push(v3);
            } else {
                indices.push(v0);
                indices.push(v2);
                indices.push(v1);

                indices.push(v0);
                indices.push(v3);
                indices.push(v2);
            }
        };

    for &(x, y, z) in cell_to_vert_idx.keys() {
        let v000 = cell_to_vert_idx.get(&(x, y, z)).copied();

        if let Some(v0) = v000 {
            // XY Plane Quad: (x,y), (x+1,y), (x+1,y+1), (x,y+1)
            if let (Some(v1), Some(v2), Some(v3)) = (
                cell_to_vert_idx.get(&(x + 1, y, z)).copied(),
                cell_to_vert_idx.get(&(x + 1, y + 1, z)).copied(),
                cell_to_vert_idx.get(&(x, y + 1, z)).copied(),
            ) {
                add_quad(v0, v1, v2, v3, &mut out_indices, &out_vertices);
            }

            // XZ Plane Quad: (x,z), (x+1,z), (x+1,z+1), (x,z+1)
            if let (Some(v1), Some(v2), Some(v3)) = (
                cell_to_vert_idx.get(&(x + 1, y, z)).copied(),
                cell_to_vert_idx.get(&(x + 1, y, z + 1)).copied(),
                cell_to_vert_idx.get(&(x, y, z + 1)).copied(),
            ) {
                add_quad(v0, v1, v2, v3, &mut out_indices, &out_vertices);
            }

            // YZ Plane Quad: (y,z), (y+1,z), (y+1,z+1), (y,z+1)
            if let (Some(v1), Some(v2), Some(v3)) = (
                cell_to_vert_idx.get(&(x, y + 1, z)).copied(),
                cell_to_vert_idx.get(&(x, y + 1, z + 1)).copied(),
                cell_to_vert_idx.get(&(x, y, z + 1)).copied(),
            ) {
                add_quad(v0, v1, v2, v3, &mut out_indices, &out_vertices);
            }
        }
    }

    if out_indices.is_empty() {
        return mesh.clone();
    }

    // 5. Recompute smooth averaged normals
    let mut smooth_normals = vec![Vec3::ZERO; out_vertices.len()];
    for chunk in out_indices.as_chunks::<3>().0 {
        let (i0, i1, i2) = (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);
        let p0 = Vec3::from_array(out_vertices[i0].position);
        let p1 = Vec3::from_array(out_vertices[i1].position);
        let p2 = Vec3::from_array(out_vertices[i2].position);
        let fnorm = (p1 - p0).cross(p2 - p0);
        smooth_normals[i0] += fnorm;
        smooth_normals[i1] += fnorm;
        smooth_normals[i2] += fnorm;
    }

    for (v, norm) in out_vertices.iter_mut().zip(smooth_normals.iter()) {
        let n = norm.normalize_or_zero();
        if n != Vec3::ZERO {
            v.normal = n.to_array();
        }
    }

    let mut remeshed = Mesh::new(out_vertices, out_indices);
    remeshed.recalculate_aabb();
    remeshed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{generate_cube, generate_uv_sphere};

    #[test]
    fn test_closest_point_on_triangle() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 2.0, 0.0);

        // Point directly inside
        let p_in = Vec3::new(0.5, 0.5, 1.0);
        let cp_in = closest_point_on_triangle(p_in, a, b, c);
        assert!((cp_in.x - 0.5).abs() < 1e-4);
        assert!((cp_in.y - 0.5).abs() < 1e-4);
        assert_eq!(cp_in.z, 0.0);

        // Point outside vertex a
        let p_out = Vec3::new(-1.0, -1.0, 0.0);
        let cp_out = closest_point_on_triangle(p_out, a, b, c);
        assert_eq!(cp_out, a);
    }

    #[test]
    fn test_voxel_remesh_cube() {
        let cube = generate_cube(1.0);
        let remeshed = voxel_remesh(&cube, 0.25);

        assert!(!remeshed.vertices.is_empty());
        assert!(!remeshed.indices.is_empty());
        assert_eq!(remeshed.indices.len() % 3, 0);

        // Bounds of remeshed mesh should approximate original 1.0 cube
        assert!((remeshed.aabb.min.x - cube.aabb.min.x).abs() < 0.25);
        assert!((remeshed.aabb.max.x - cube.aabb.max.x).abs() < 0.25);
    }

    #[test]
    fn test_voxel_remesh_sphere() {
        let sphere = generate_uv_sphere(1.0, 16, 16);
        let remeshed = voxel_remesh(&sphere, 0.2);

        assert!(!remeshed.vertices.is_empty());
        assert!(!remeshed.indices.is_empty());
        assert!(remeshed.vertices.len() > 20);
    }
}
