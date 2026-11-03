use forge_math::{Vec2, Vec3};

/// Triangulates a 3D polygon into a series of triangles using 2D projected Ear Clipping.
///
/// Returns a list of vertex index triplets `[i0, i1, i2]` indexing into the input `positions` array.
pub fn triangulate_polygon_3d(positions: &[[f32; 3]]) -> Vec<[usize; 3]> {
    let n = positions.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }

    // 1. Calculate polygon normal using Newell's method
    let mut normal = Vec3::ZERO;
    for i in 0..n {
        let next = (i + 1) % n;
        let pi = Vec3::from_array(positions[i]);
        let pj = Vec3::from_array(positions[next]);
        normal.x += (pi.y - pj.y) * (pi.z + pj.z);
        normal.y += (pi.z - pj.z) * (pi.x + pj.x);
        normal.z += (pi.x - pj.x) * (pi.y + pj.y);
    }

    // Degenerate plane fallback (fan triangulation)
    if normal.length_squared() < 1e-12 {
        let mut fan = Vec::with_capacity(n - 2);
        for i in 1..(n - 1) {
            fan.push([0, i, i + 1]);
        }
        return fan;
    }

    // 2. Project 3D points onto the 2D plane with largest normal component
    let ax = normal.x.abs();
    let ay = normal.y.abs();
    let az = normal.z.abs();
    let (u, v) = if ax >= ay && ax >= az {
        (1, 2) // Project to YZ
    } else if ay >= ax && ay >= az {
        (0, 2) // Project to XZ
    } else {
        (0, 1) // Project to XY
    };

    let pts: Vec<Vec2> = positions.iter().map(|p| Vec2::new(p[u], p[v])).collect();

    // 3. Compute 2D signed area to determine winding
    let mut signed_area = 0.0f32;
    for i in 0..n {
        let next = (i + 1) % n;
        signed_area += pts[i].x * pts[next].y - pts[next].x * pts[i].y;
    }
    let is_ccw = signed_area > 0.0;

    // 4. Ear clipping loop
    let mut remaining: Vec<usize> = (0..n).collect();
    let mut result = Vec::with_capacity(n - 2);
    let mut stalled_attempts = 0;

    while remaining.len() > 3 {
        let rem_len = remaining.len();
        let mut ear_found = false;

        for i in 0..rem_len {
            let prev_idx = remaining[(i + rem_len - 1) % rem_len];
            let curr_idx = remaining[i];
            let next_idx = remaining[(i + 1) % rem_len];

            let a = pts[prev_idx];
            let b = pts[curr_idx];
            let c = pts[next_idx];

            // Check if vertex forms a convex angle
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            let is_convex = if is_ccw { cross > 1e-7 } else { cross < -1e-7 };

            if !is_convex {
                continue;
            }

            // Check if any other remaining vertex is inside the candidate triangle
            let mut has_interior_vertex = false;
            for &test_idx in &remaining {
                if test_idx == prev_idx || test_idx == curr_idx || test_idx == next_idx {
                    continue;
                }
                if point_in_triangle_2d(pts[test_idx], a, b, c) {
                    has_interior_vertex = true;
                    break;
                }
            }

            if !has_interior_vertex {
                result.push([prev_idx, curr_idx, next_idx]);
                remaining.remove(i);
                ear_found = true;
                stalled_attempts = 0;
                break;
            }
        }

        if !ear_found {
            stalled_attempts += 1;
            // Force progress on non-simple or numerically degenerate polygons
            if stalled_attempts > 1 {
                result.push([remaining[0], remaining[1], remaining[2]]);
                remaining.remove(1);
                stalled_attempts = 0;
            } else {
                result.push([remaining[0], remaining[1], remaining[2]]);
                remaining.remove(1);
            }
        }
    }

    if remaining.len() == 3 {
        result.push([remaining[0], remaining[1], remaining[2]]);
    }

    result
}

fn point_in_triangle_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let cross1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let cross2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let cross3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);

    let has_neg = (cross1 < -1e-7) || (cross2 < -1e-7) || (cross3 < -1e-7);
    let has_pos = (cross1 > 1e-7) || (cross2 > 1e-7) || (cross3 > 1e-7);

    !(has_neg && has_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangulate_triangle() {
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let tris = triangulate_polygon_3d(&pts);
        assert_eq!(tris.len(), 1);
        assert_eq!(tris[0], [0, 1, 2]);
    }

    #[test]
    fn test_triangulate_convex_quad() {
        let pts = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let tris = triangulate_polygon_3d(&pts);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_triangulate_concave_quad_dart() {
        // Arrowhead / dart shape with reflex vertex at [0.5, 0.5, 0.0]
        let pts = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 0.5, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let tris = triangulate_polygon_3d(&pts);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_triangulate_concave_l_shape() {
        // 6-vertex L shape in XZ plane
        let pts = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 2.0],
            [0.0, 0.0, 2.0],
        ];
        let tris = triangulate_polygon_3d(&pts);
        assert_eq!(tris.len(), 4);
    }
}
