use std::fs::File;
use std::io::Write;
use std::path::Path;

use forge_geometry::Mesh;
use forge_math::{Mat4, Vec3};
use forge_scene::Scene;

use super::parser::StlFormat;
use crate::error::{ForgeIoError, Result};

/// Configuration options controlling how STL files are exported.
#[derive(Debug, Clone)]
pub struct StlExportOptions {
    /// Format encoding: Binary (compact, standard) or Ascii.
    pub format: StlFormat,
    /// Whether to bake the object's world transformation into the exported vertices.
    pub apply_world_transform: bool,
    /// Solid name recorded in the STL header.
    pub solid_name: String,
    /// Whether to export only visible objects when exporting a full Scene.
    pub visible_only: bool,
}

impl Default for StlExportOptions {
    fn default() -> Self {
        Self {
            format: StlFormat::Binary,
            apply_world_transform: true,
            solid_name: "forge3d_model".to_string(),
            visible_only: true,
        }
    }
}

/// Exports a single Mesh to STL byte buffer.
pub fn export_mesh_stl(mesh: &Mesh, options: &StlExportOptions) -> Result<Vec<u8>> {
    let triangles = extract_mesh_triangles(mesh, None);
    encode_triangles(&triangles, options)
}

/// Exports all eligible objects from a Scene to an STL byte buffer.
pub fn export_scene_stl(scene: &Scene, options: &StlExportOptions) -> Result<Vec<u8>> {
    let mut all_triangles = Vec::new();

    for obj in scene.objects.values() {
        if options.visible_only && !obj.visible {
            continue;
        }

        let world_mat = if options.apply_world_transform {
            Some(scene.world_transform(obj.id).to_matrix())
        } else {
            None
        };

        let obj_triangles = extract_mesh_triangles(&obj.mesh, world_mat);
        all_triangles.extend(obj_triangles);
    }

    encode_triangles(&all_triangles, options)
}

/// Exports a single Mesh directly to a file on disk.
pub fn export_mesh_stl_to_file(
    mesh: &Mesh,
    path: impl AsRef<Path>,
    options: &StlExportOptions,
) -> Result<()> {
    let data = export_mesh_stl(mesh, options)?;
    let mut file = File::create(path)?;
    file.write_all(&data)?;
    file.flush()?;
    Ok(())
}

/// Exports a full Scene directly to a file on disk.
pub fn export_scene_stl_to_file(
    scene: &Scene,
    path: impl AsRef<Path>,
    options: &StlExportOptions,
) -> Result<()> {
    let data = export_scene_stl(scene, options)?;
    let mut file = File::create(path)?;
    file.write_all(&data)?;
    file.flush()?;
    Ok(())
}

struct TriangleFacet {
    normal: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
}

fn extract_mesh_triangles(mesh: &Mesh, world_mat: Option<Mat4>) -> Vec<TriangleFacet> {
    let num_triangles = mesh.indices.len() / 3;
    let mut facets = Vec::with_capacity(num_triangles);

    for chunk in mesh.indices.as_chunks::<3>().0 {
        let i0 = chunk[0] as usize;
        let i1 = chunk[1] as usize;
        let i2 = chunk[2] as usize;

        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let mut p0 = Vec3::from(mesh.vertices[i0].position);
        let mut p1 = Vec3::from(mesh.vertices[i1].position);
        let mut p2 = Vec3::from(mesh.vertices[i2].position);

        if let Some(mat) = world_mat {
            p0 = mat.transform_point3(p0);
            p1 = mat.transform_point3(p1);
            p2 = mat.transform_point3(p2);
        }

        let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero().into();

        facets.push(TriangleFacet {
            normal,
            v0: p0.into(),
            v1: p1.into(),
            v2: p2.into(),
        });
    }

    facets
}

fn encode_triangles(triangles: &[TriangleFacet], options: &StlExportOptions) -> Result<Vec<u8>> {
    match options.format {
        StlFormat::Binary => encode_binary_stl(triangles, options),
        StlFormat::Ascii => encode_ascii_stl(triangles, options),
    }
}

fn encode_binary_stl(triangles: &[TriangleFacet], options: &StlExportOptions) -> Result<Vec<u8>> {
    let triangle_count = triangles.len();
    if triangle_count > u32::MAX as usize {
        return Err(ForgeIoError::ExportError(format!(
            "Triangle count ({}) exceeds 32-bit binary STL limit",
            triangle_count
        )));
    }

    let total_size = 84 + triangle_count * 50;
    let mut buffer = Vec::with_capacity(total_size);

    // 80-byte header
    let header_text = format!("Forge3D Rust Edition STL Export - {}", options.solid_name);
    let mut header = [0u8; 80];
    let copy_len = header_text.len().min(79);
    header[..copy_len].copy_from_slice(&header_text.as_bytes()[..copy_len]);
    buffer.extend_from_slice(&header);

    // 4-byte triangle count
    buffer.extend_from_slice(&(triangle_count as u32).to_le_bytes());

    // Facet records
    for tri in triangles {
        // Normal
        buffer.extend_from_slice(&tri.normal[0].to_le_bytes());
        buffer.extend_from_slice(&tri.normal[1].to_le_bytes());
        buffer.extend_from_slice(&tri.normal[2].to_le_bytes());

        // Vertex 0
        buffer.extend_from_slice(&tri.v0[0].to_le_bytes());
        buffer.extend_from_slice(&tri.v0[1].to_le_bytes());
        buffer.extend_from_slice(&tri.v0[2].to_le_bytes());

        // Vertex 1
        buffer.extend_from_slice(&tri.v1[0].to_le_bytes());
        buffer.extend_from_slice(&tri.v1[1].to_le_bytes());
        buffer.extend_from_slice(&tri.v1[2].to_le_bytes());

        // Vertex 2
        buffer.extend_from_slice(&tri.v2[0].to_le_bytes());
        buffer.extend_from_slice(&tri.v2[1].to_le_bytes());
        buffer.extend_from_slice(&tri.v2[2].to_le_bytes());

        // 2-byte attribute byte count
        buffer.extend_from_slice(&0u16.to_le_bytes());
    }

    Ok(buffer)
}

fn encode_ascii_stl(triangles: &[TriangleFacet], options: &StlExportOptions) -> Result<Vec<u8>> {
    let mut out = String::with_capacity(triangles.len() * 200 + 100);
    let solid_name = if options.solid_name.is_empty() {
        "forge3d_model"
    } else {
        &options.solid_name
    };

    out.push_str(&format!("solid {}\n", solid_name));

    for tri in triangles {
        out.push_str(&format!(
            "  facet normal {:.6e} {:.6e} {:.6e}\n",
            tri.normal[0], tri.normal[1], tri.normal[2]
        ));
        out.push_str("    outer loop\n");
        out.push_str(&format!(
            "      vertex {:.6e} {:.6e} {:.6e}\n",
            tri.v0[0], tri.v0[1], tri.v0[2]
        ));
        out.push_str(&format!(
            "      vertex {:.6e} {:.6e} {:.6e}\n",
            tri.v1[0], tri.v1[1], tri.v1[2]
        ));
        out.push_str(&format!(
            "      vertex {:.6e} {:.6e} {:.6e}\n",
            tri.v2[0], tri.v2[1], tri.v2[2]
        ));
        out.push_str("    endloop\n");
        out.push_str("  endfacet\n");
    }

    out.push_str(&format!("endsolid {}\n", solid_name));

    Ok(out.into_bytes())
}
