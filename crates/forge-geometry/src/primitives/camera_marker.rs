use crate::validation::validate_mesh_data;
use crate::{Mesh, Vertex};
use forge_math::Vec3;

/// Generates a 3D procedural camera marker mesh representing a physical camera body with a lens frustum.
/// The camera looks along the negative Z-axis (-Z), with +Y being up.
pub fn generate_camera_marker() -> Mesh {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let body_color = [0.35, 0.38, 0.45, 1.0];
    let lens_color = [0.15, 0.18, 0.22, 1.0];
    let up_indicator_color = [0.95, 0.55, 0.15, 1.0]; // Bright orange triangle pointing up

    // 1. Camera Body Box: from X in [-0.3, 0.3], Y in [-0.2, 0.2], Z in [0.0, 0.5]
    let hx = 0.3;
    let hy = 0.2;
    let z0 = 0.0;
    let z1 = 0.5;

    // Helper to add a quad
    let mut add_quad = |p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, normal: Vec3, color: [f32; 4]| {
        let base = vertices.len() as u32;
        vertices.push(Vertex::new(
            p0.to_array(),
            normal.to_array(),
            [0.0, 0.0],
            color,
        ));
        vertices.push(Vertex::new(
            p1.to_array(),
            normal.to_array(),
            [1.0, 0.0],
            color,
        ));
        vertices.push(Vertex::new(
            p2.to_array(),
            normal.to_array(),
            [1.0, 1.0],
            color,
        ));
        vertices.push(Vertex::new(
            p3.to_array(),
            normal.to_array(),
            [0.0, 1.0],
            color,
        ));
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    // Body faces
    // Front face (at Z = 0.0, facing -Z)
    add_quad(
        Vec3::new(-hx, -hy, z0),
        Vec3::new(hx, -hy, z0),
        Vec3::new(hx, hy, z0),
        Vec3::new(-hx, hy, z0),
        -Vec3::Z,
        body_color,
    );
    // Back face (at Z = z1, facing +Z)
    add_quad(
        Vec3::new(hx, -hy, z1),
        Vec3::new(-hx, -hy, z1),
        Vec3::new(-hx, hy, z1),
        Vec3::new(hx, hy, z1),
        Vec3::Z,
        body_color,
    );
    // Left face (-X)
    add_quad(
        Vec3::new(-hx, -hy, z1),
        Vec3::new(-hx, -hy, z0),
        Vec3::new(-hx, hy, z0),
        Vec3::new(-hx, hy, z1),
        -Vec3::X,
        body_color,
    );
    // Right face (+X)
    add_quad(
        Vec3::new(hx, -hy, z0),
        Vec3::new(hx, -hy, z1),
        Vec3::new(hx, hy, z1),
        Vec3::new(hx, hy, z0),
        Vec3::X,
        body_color,
    );
    // Top face (+Y)
    add_quad(
        Vec3::new(-hx, hy, z0),
        Vec3::new(hx, hy, z0),
        Vec3::new(hx, hy, z1),
        Vec3::new(-hx, hy, z1),
        Vec3::Y,
        body_color,
    );
    // Bottom face (-Y)
    add_quad(
        Vec3::new(-hx, -hy, z1),
        Vec3::new(hx, -hy, z1),
        Vec3::new(hx, -hy, z0),
        Vec3::new(-hx, -hy, z0),
        -Vec3::Y,
        body_color,
    );

    // 2. Lens Frustum (extending from Z = 0.0 to Z = -0.4, flaring outwards)
    let lx0 = 0.18;
    let ly0 = 0.14;
    let lx1 = 0.28;
    let ly1 = 0.22;
    let lz = -0.4;

    // Lens front cap (facing -Z)
    add_quad(
        Vec3::new(-lx1, -ly1, lz),
        Vec3::new(lx1, -ly1, lz),
        Vec3::new(lx1, ly1, lz),
        Vec3::new(-lx1, ly1, lz),
        -Vec3::Z,
        lens_color,
    );

    // Lens sides (frustum taper)
    // Lens top
    add_quad(
        Vec3::new(-lx0, ly0, z0),
        Vec3::new(lx0, ly0, z0),
        Vec3::new(lx1, ly1, lz),
        Vec3::new(-lx1, ly1, lz),
        Vec3::new(0.0, 0.7, -0.3).normalize(),
        lens_color,
    );
    // Lens bottom
    add_quad(
        Vec3::new(-lx1, -ly1, lz),
        Vec3::new(lx1, -ly1, lz),
        Vec3::new(lx0, -ly0, z0),
        Vec3::new(-lx0, -ly0, z0),
        Vec3::new(0.0, -0.7, -0.3).normalize(),
        lens_color,
    );
    // Lens left
    add_quad(
        Vec3::new(-lx1, -ly1, lz),
        Vec3::new(-lx0, -ly0, z0),
        Vec3::new(-lx0, ly0, z0),
        Vec3::new(-lx1, ly1, lz),
        Vec3::new(-0.7, 0.0, -0.3).normalize(),
        lens_color,
    );
    // Lens right
    add_quad(
        Vec3::new(lx0, -ly0, z0),
        Vec3::new(lx1, -ly1, lz),
        Vec3::new(lx1, ly1, lz),
        Vec3::new(lx0, ly0, z0),
        Vec3::new(0.7, 0.0, -0.3).normalize(),
        lens_color,
    );

    // 3. Up Indicator (triangle sitting atop camera body, pointing +Y and forward)
    let tri_base = vertices.len() as u32;
    let up_tip = Vec3::new(0.0, hy + 0.22, 0.15);
    let up_left = Vec3::new(-0.12, hy, 0.35);
    let up_right = Vec3::new(0.12, hy, 0.35);

    vertices.push(Vertex::new(
        up_tip.to_array(),
        [0.0, 0.7, 0.7],
        [0.5, 1.0],
        up_indicator_color,
    ));
    vertices.push(Vertex::new(
        up_left.to_array(),
        [0.0, 0.7, 0.7],
        [0.0, 0.0],
        up_indicator_color,
    ));
    vertices.push(Vertex::new(
        up_right.to_array(),
        [0.0, 0.7, 0.7],
        [1.0, 0.0],
        up_indicator_color,
    ));
    indices.extend_from_slice(&[tri_base, tri_base + 1, tri_base + 2]);

    validate_mesh_data(&vertices, &indices)
        .expect("Camera marker mesh must be topologically valid");
    Mesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_marker_mesh_validity() {
        let mesh = generate_camera_marker();
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(!mesh.aabb.is_empty());
    }
}
