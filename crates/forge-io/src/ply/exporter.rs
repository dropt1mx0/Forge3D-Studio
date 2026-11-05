use std::fs::File;
use std::io::Write;
use std::path::Path;

use forge_geometry::{Mesh, Vertex};
use forge_math::{Mat4, Vec3};
use forge_scene::Scene;

use super::parser::PlyFormat;
use crate::error::{ForgeIoError, Result};

/// Configuration options controlling how PLY files are exported.
#[derive(Debug, Clone)]
pub struct PlyExportOptions {
    /// Format encoding: BinaryLittleEndian (fast, compact) or Ascii.
    pub format: PlyFormat,
    /// Apply object world transform to vertices.
    pub apply_world_transform: bool,
    /// Include surface normals in output.
    pub export_normals: bool,
    /// Include vertex RGBA colors in output.
    pub export_colors: bool,
    /// Include texture UV coordinates in output.
    pub export_uvs: bool,
    /// Only export visible objects when exporting a full scene.
    pub visible_only: bool,
    /// Custom comments recorded in the PLY header.
    pub comments: Vec<String>,
}

impl Default for PlyExportOptions {
    fn default() -> Self {
        Self {
            format: PlyFormat::BinaryLittleEndian,
            apply_world_transform: true,
            export_normals: true,
            export_colors: true,
            export_uvs: true,
            visible_only: true,
            comments: vec!["Exported from Forge3D Rust Edition".to_string()],
        }
    }
}

/// Exports a single Mesh to a PLY byte buffer.
pub fn export_mesh_ply(mesh: &Mesh, options: &PlyExportOptions) -> Result<Vec<u8>> {
    let vertices = prepare_vertices(&mesh.vertices, None);
    encode_ply(&vertices, &mesh.indices, options)
}

/// Exports an entire Scene to a PLY byte buffer.
pub fn export_scene_ply(scene: &Scene, options: &PlyExportOptions) -> Result<Vec<u8>> {
    let mut combined_vertices = Vec::new();
    let mut combined_indices = Vec::new();

    for obj in scene.objects.values() {
        if options.visible_only && !obj.visible {
            continue;
        }

        let world_mat = if options.apply_world_transform {
            Some(scene.world_transform(obj.id).to_matrix())
        } else {
            None
        };

        let base_idx = combined_vertices.len() as u32;
        let obj_verts = prepare_vertices(&obj.mesh.vertices, world_mat);
        combined_vertices.extend(obj_verts);

        for &idx in &obj.mesh.indices {
            combined_indices.push(base_idx + idx);
        }
    }

    encode_ply(&combined_vertices, &combined_indices, options)
}

/// Exports a Mesh directly to a PLY file on disk.
pub fn export_mesh_ply_to_file(
    mesh: &Mesh,
    path: impl AsRef<Path>,
    options: &PlyExportOptions,
) -> Result<()> {
    let data = export_mesh_ply(mesh, options)?;
    let mut file = File::create(path)?;
    file.write_all(&data)?;
    file.flush()?;
    Ok(())
}

/// Exports a full Scene directly to a PLY file on disk.
pub fn export_scene_ply_to_file(
    scene: &Scene,
    path: impl AsRef<Path>,
    options: &PlyExportOptions,
) -> Result<()> {
    let data = export_scene_ply(scene, options)?;
    let mut file = File::create(path)?;
    file.write_all(&data)?;
    file.flush()?;
    Ok(())
}

fn prepare_vertices(src: &[Vertex], world_mat: Option<Mat4>) -> Vec<Vertex> {
    src.iter()
        .map(|v| {
            let mut vert = *v;
            if let Some(mat) = world_mat {
                let p = mat.transform_point3(Vec3::from(vert.position));
                vert.position = p.into();

                let n = mat
                    .transform_vector3(Vec3::from(vert.normal))
                    .normalize_or_zero();
                vert.normal = n.into();
            }
            vert
        })
        .collect()
}

fn encode_ply(vertices: &[Vertex], indices: &[u32], options: &PlyExportOptions) -> Result<Vec<u8>> {
    let num_faces = indices.len() / 3;

    let format_str = match options.format {
        PlyFormat::Ascii => "format ascii 1.0",
        PlyFormat::BinaryLittleEndian => "format binary_little_endian 1.0",
        PlyFormat::BinaryBigEndian => {
            return Err(ForgeIoError::UnsupportedFormat {
                format: "PLY",
                message: "Big endian export is not supported".to_string(),
            });
        }
    };

    let mut header = String::new();
    header.push_str("ply\n");
    header.push_str(format_str);
    header.push('\n');

    for comment in &options.comments {
        header.push_str(&format!("comment {}\n", comment));
    }

    header.push_str(&format!("element vertex {}\n", vertices.len()));
    header.push_str("property float x\n");
    header.push_str("property float y\n");
    header.push_str("property float z\n");

    if options.export_normals {
        header.push_str("property float nx\n");
        header.push_str("property float ny\n");
        header.push_str("property float nz\n");
    }

    if options.export_uvs {
        header.push_str("property float s\n");
        header.push_str("property float t\n");
    }

    if options.export_colors {
        header.push_str("property uchar red\n");
        header.push_str("property uchar green\n");
        header.push_str("property uchar blue\n");
        header.push_str("property uchar alpha\n");
    }

    header.push_str(&format!("element face {}\n", num_faces));
    header.push_str("property list uchar int vertex_indices\n");
    header.push_str("end_header\n");

    match options.format {
        PlyFormat::Ascii => encode_ascii_ply(&header, vertices, indices, options),
        PlyFormat::BinaryLittleEndian => encode_binary_le_ply(&header, vertices, indices, options),
        PlyFormat::BinaryBigEndian => unreachable!(),
    }
}

fn encode_ascii_ply(
    header: &str,
    vertices: &[Vertex],
    indices: &[u32],
    options: &PlyExportOptions,
) -> Result<Vec<u8>> {
    let mut out = String::with_capacity(header.len() + vertices.len() * 64 + indices.len() * 16);
    out.push_str(header);

    for v in vertices {
        out.push_str(&format!(
            "{:.6} {:.6} {:.6}",
            v.position[0], v.position[1], v.position[2]
        ));

        if options.export_normals {
            out.push_str(&format!(
                " {:.6} {:.6} {:.6}",
                v.normal[0], v.normal[1], v.normal[2]
            ));
        }

        if options.export_uvs {
            out.push_str(&format!(" {:.6} {:.6}", v.uv[0], v.uv[1]));
        }

        if options.export_colors {
            let r = (v.color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (v.color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (v.color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            let a = (v.color[3].clamp(0.0, 1.0) * 255.0).round() as u8;
            out.push_str(&format!(" {} {} {} {}", r, g, b, a));
        }

        out.push('\n');
    }

    for chunk in indices.as_chunks::<3>().0 {
        out.push_str(&format!("3 {} {} {}\n", chunk[0], chunk[1], chunk[2]));
    }

    Ok(out.into_bytes())
}

fn encode_binary_le_ply(
    header: &str,
    vertices: &[Vertex],
    indices: &[u32],
    options: &PlyExportOptions,
) -> Result<Vec<u8>> {
    let vert_stride = 12
        + if options.export_normals { 12 } else { 0 }
        + if options.export_uvs { 8 } else { 0 }
        + if options.export_colors { 4 } else { 0 };

    let num_faces = indices.len() / 3;
    let face_stride = 1 + 12; // 1 byte count (3) + 3 * 4 bytes int

    let total_size = header.len() + vertices.len() * vert_stride + num_faces * face_stride;
    let mut out = Vec::with_capacity(total_size);
    out.extend_from_slice(header.as_bytes());

    for v in vertices {
        out.extend_from_slice(&v.position[0].to_le_bytes());
        out.extend_from_slice(&v.position[1].to_le_bytes());
        out.extend_from_slice(&v.position[2].to_le_bytes());

        if options.export_normals {
            out.extend_from_slice(&v.normal[0].to_le_bytes());
            out.extend_from_slice(&v.normal[1].to_le_bytes());
            out.extend_from_slice(&v.normal[2].to_le_bytes());
        }

        if options.export_uvs {
            out.extend_from_slice(&v.uv[0].to_le_bytes());
            out.extend_from_slice(&v.uv[1].to_le_bytes());
        }

        if options.export_colors {
            let r = (v.color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (v.color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (v.color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            let a = (v.color[3].clamp(0.0, 1.0) * 255.0).round() as u8;
            out.push(r);
            out.push(g);
            out.push(b);
            out.push(a);
        }
    }

    for chunk in indices.as_chunks::<3>().0 {
        out.push(3u8);
        out.extend_from_slice(&(chunk[0] as i32).to_le_bytes());
        out.extend_from_slice(&(chunk[1] as i32).to_le_bytes());
        out.extend_from_slice(&(chunk[2] as i32).to_le_bytes());
    }

    Ok(out)
}
