use forge_core::error::{CoreError, Result};
use glam::Vec3;

use crate::vertex::Vertex;

/// Validates a mesh for common topological and numerical defects:
/// - No non-finite (NaN or infinite) coordinates or normals
/// - No out-of-range index references
/// - Valid triangle count (multiple of 3 indices)
/// - Flags degenerate (zero-area) triangles
pub fn validate_mesh_data(vertices: &[Vertex], indices: &[u32]) -> Result<()> {
    if indices.is_empty() {
        return Err(CoreError::GeometryError(
            "Index buffer is empty".to_string(),
        ));
    }

    if !indices.len().is_multiple_of(3) {
        return Err(CoreError::GeometryError(format!(
            "Indices length ({}) is not a multiple of 3",
            indices.len()
        )));
    }

    let vertex_count = vertices.len();

    // Check vertices for NaNs and non-finite numbers
    for (i, v) in vertices.iter().enumerate() {
        let pos = Vec3::from_array(v.position);
        let norm = Vec3::from_array(v.normal);

        if !pos.is_finite() {
            return Err(CoreError::GeometryError(format!(
                "Vertex {i} contains non-finite position: {:?}",
                v.position
            )));
        }

        if !norm.is_finite() {
            return Err(CoreError::GeometryError(format!(
                "Vertex {i} contains non-finite normal: {:?}",
                v.normal
            )));
        }
    }

    // Check index ranges and degenerates
    for (tri_idx, chunk) in indices.as_chunks::<3>().0.iter().enumerate() {
        let (i0, i1, i2) = (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);

        if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
            return Err(CoreError::GeometryError(format!(
                "Triangle {tri_idx} references out-of-bounds vertex index ({i0}, {i1}, {i2}) >= {vertex_count}"
            )));
        }

        if i0 == i1 || i1 == i2 || i0 == i2 {
            return Err(CoreError::GeometryError(format!(
                "Triangle {tri_idx} has duplicate vertex indices: ({i0}, {i1}, {i2})"
            )));
        }

        let p0 = Vec3::from_array(vertices[i0].position);
        let p1 = Vec3::from_array(vertices[i1].position);
        let p2 = Vec3::from_array(vertices[i2].position);

        let cross = (p1 - p0).cross(p2 - p0);
        if cross.length_squared() < 1e-12 {
            // Degenerate triangle warning/error
            return Err(CoreError::GeometryError(format!(
                "Triangle {tri_idx} is degenerate (area near zero)"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_triangle() {
        let vertices = vec![
            Vertex::with_pos_norm_uv([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::with_pos_norm_uv([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            Vertex::with_pos_norm_uv([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        assert!(validate_mesh_data(&vertices, &indices).is_ok());
    }

    #[test]
    fn test_nan_fails() {
        let vertices = vec![
            Vertex::with_pos_norm_uv([f32::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::with_pos_norm_uv([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            Vertex::with_pos_norm_uv([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        assert!(validate_mesh_data(&vertices, &indices).is_err());
    }

    #[test]
    fn test_out_of_bounds_index_fails() {
        let vertices = vec![
            Vertex::with_pos_norm_uv([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::with_pos_norm_uv([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            Vertex::with_pos_norm_uv([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];
        let indices = vec![0, 1, 5];
        assert!(validate_mesh_data(&vertices, &indices).is_err());
    }
}
