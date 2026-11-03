use std::io::Write;

use forge_geometry::Mesh;
use forge_math::Vec3;
use forge_scene::Scene;

use crate::error::{ForgeIoError, Result};

/// Configuration options for exporting Wavefront OBJ files.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Apply object world transformations (position, rotation, scale) to vertices.
    pub world_space: bool,
    /// Export surface normals (`vn`).
    pub export_normals: bool,
    /// Export texture coordinates (`vt`).
    pub export_uvs: bool,
    /// Custom header comment to include at the top of the file.
    pub header_comment: Option<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            world_space: true,
            export_normals: true,
            export_uvs: true,
            header_comment: None,
        }
    }
}

/// Exports a single `Mesh` to Wavefront OBJ format through a writer.
pub fn export_mesh_obj(mesh: &Mesh, object_name: &str, writer: &mut impl Write) -> Result<()> {
    let options = ExportOptions {
        world_space: false,
        ..Default::default()
    };
    write_header(writer, &options)?;
    write_single_mesh(mesh, object_name, 1, 1, 1, &options, None, writer)?;
    Ok(())
}

/// Exports a single `Mesh` to an in-memory OBJ formatted string.
pub fn export_mesh_obj_to_string(mesh: &Mesh, object_name: &str) -> Result<String> {
    let mut buf = Vec::new();
    export_mesh_obj(mesh, object_name, &mut buf)?;
    String::from_utf8(buf).map_err(|e| ForgeIoError::ExportError(e.to_string()))
}

/// Exports a complete `Scene` (or filtered selection) to Wavefront OBJ format through a writer.
pub fn export_scene_obj(
    scene: &Scene,
    options: &ExportOptions,
    writer: &mut impl Write,
) -> Result<()> {
    write_header(writer, options)?;

    let mut v_offset = 1usize;
    let mut vt_offset = 1usize;
    let mut vn_offset = 1usize;
    let mut exported_count = 0;

    for (&id, obj) in &scene.objects {
        if obj.mesh.vertices.is_empty() || obj.mesh.indices.is_empty() {
            continue;
        }

        let world_tf = if options.world_space {
            Some(scene.world_transform(id))
        } else {
            None
        };

        let (new_v, new_vt, new_vn) = write_single_mesh(
            &obj.mesh, &obj.name, v_offset, vt_offset, vn_offset, options, world_tf, writer,
        )?;

        v_offset = new_v;
        vt_offset = new_vt;
        vn_offset = new_vn;
        exported_count += 1;
    }

    if exported_count == 0 {
        return Err(ForgeIoError::NoValidGeometry(
            "Scene contains no mesh objects to export".to_string(),
        ));
    }

    Ok(())
}

/// Exports a complete `Scene` to an in-memory OBJ formatted string.
pub fn export_scene_obj_to_string(scene: &Scene, options: &ExportOptions) -> Result<String> {
    let mut buf = Vec::new();
    export_scene_obj(scene, options, &mut buf)?;
    String::from_utf8(buf).map_err(|e| ForgeIoError::ExportError(e.to_string()))
}

fn write_header(writer: &mut impl Write, options: &ExportOptions) -> Result<()> {
    writeln!(writer, "# Forge3D Rust Edition — Wavefront OBJ Exporter")?;
    writeln!(writer, "# https://github.com/forge3d/forge3d")?;
    if let Some(comment) = &options.header_comment {
        writeln!(writer, "# {comment}")?;
    }
    writeln!(writer)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_single_mesh(
    mesh: &Mesh,
    name: &str,
    v_offset: usize,
    vt_offset: usize,
    vn_offset: usize,
    options: &ExportOptions,
    world_transform: Option<forge_math::Transform>,
    writer: &mut impl Write,
) -> Result<(usize, usize, usize)> {
    let clean_name = name.replace(|c: char| c.is_whitespace(), "_");
    writeln!(writer, "o {clean_name}")?;

    let (transform_mat, normal_mat) = if let Some(tf) = world_transform {
        let m = tf.to_matrix();
        let nm = m.inverse().transpose();
        (Some(m), Some(nm))
    } else {
        (None, None)
    };

    // 1. Write positions (v)
    for v in &mesh.vertices {
        let p = if let Some(mat) = transform_mat {
            mat.transform_point3(Vec3::from_array(v.position))
        } else {
            Vec3::from_array(v.position)
        };
        writeln!(writer, "v {:.6} {:.6} {:.6}", p.x, p.y, p.z)?;
    }

    // 2. Write texture coordinates (vt)
    if options.export_uvs {
        for v in &mesh.vertices {
            writeln!(writer, "vt {:.6} {:.6}", v.uv[0], v.uv[1])?;
        }
    }

    // 3. Write normals (vn)
    if options.export_normals {
        for v in &mesh.vertices {
            let n = if let Some(nmat) = normal_mat {
                nmat.transform_vector3(Vec3::from_array(v.normal))
                    .normalize_or_zero()
            } else {
                Vec3::from_array(v.normal)
            };
            writeln!(writer, "vn {:.6} {:.6} {:.6}", n.x, n.y, n.z)?;
        }
    }

    // 4. Write faces (f)
    for chunk in mesh.indices.as_chunks::<3>().0 {
        let i0 = chunk[0] as usize;
        let i1 = chunk[1] as usize;
        let i2 = chunk[2] as usize;

        let v0 = v_offset + i0;
        let v1 = v_offset + i1;
        let v2 = v_offset + i2;

        match (options.export_uvs, options.export_normals) {
            (true, true) => {
                let vt0 = vt_offset + i0;
                let vt1 = vt_offset + i1;
                let vt2 = vt_offset + i2;
                let vn0 = vn_offset + i0;
                let vn1 = vn_offset + i1;
                let vn2 = vn_offset + i2;
                writeln!(
                    writer,
                    "f {v0}/{vt0}/{vn0} {v1}/{vt1}/{vn1} {v2}/{vt2}/{vn2}"
                )?;
            }
            (true, false) => {
                let vt0 = vt_offset + i0;
                let vt1 = vt_offset + i1;
                let vt2 = vt_offset + i2;
                writeln!(writer, "f {v0}/{vt0} {v1}/{vt1} {v2}/{vt2}")?;
            }
            (false, true) => {
                let vn0 = vn_offset + i0;
                let vn1 = vn_offset + i1;
                let vn2 = vn_offset + i2;
                writeln!(writer, "f {v0}//{vn0} {v1}//{vn1} {v2}//{vn2}")?;
            }
            (false, false) => {
                writeln!(writer, "f {v0} {v1} {v2}")?;
            }
        }
    }

    writeln!(writer)?;

    let vert_count = mesh.vertices.len();
    Ok((
        v_offset + vert_count,
        vt_offset + vert_count,
        vn_offset + vert_count,
    ))
}
