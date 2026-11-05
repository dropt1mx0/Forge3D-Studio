use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;

use super::parser::{PlyDocument, parse_ply, parse_ply_from_bytes};
use crate::error::{ForgeIoError, Result};
use crate::obj::{ImportedAsset, ImportedObject, triangulate_polygon_3d};

/// Configuration options controlling how PLY files are imported.
#[derive(Debug, Clone)]
pub struct PlyImportOptions {
    /// Global scale multiplier applied to vertex positions.
    pub scale: f32,
    /// Automatically calculate smooth vertex normals if missing from the PLY file.
    pub generate_normals_if_missing: bool,
    /// Default RGBA color assigned to vertices lacking explicit color attributes.
    pub default_color: [f32; 4],
    /// Custom object name overriding default filename/metadata.
    pub custom_name: Option<String>,
}

impl Default for PlyImportOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            generate_normals_if_missing: true,
            default_color: [0.85, 0.85, 0.9, 1.0],
            custom_name: None,
        }
    }
}

/// Imports a PLY asset from a file on disk.
pub fn import_ply(path: impl AsRef<Path>, options: &PlyImportOptions) -> Result<ImportedAsset> {
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let doc = parse_ply(reader)?;

    let name = options.custom_name.clone().unwrap_or_else(|| {
        path.as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("ply_mesh")
            .to_string()
    });

    build_mesh_from_ply(doc, name, options)
}

/// Imports a PLY asset from an in-memory byte slice.
pub fn import_ply_from_bytes(bytes: &[u8], options: &PlyImportOptions) -> Result<ImportedAsset> {
    let doc = parse_ply_from_bytes(bytes)?;
    let name = options
        .custom_name
        .clone()
        .unwrap_or_else(|| "ply_mesh".to_string());
    build_mesh_from_ply(doc, name, options)
}

fn build_mesh_from_ply(
    doc: PlyDocument,
    object_name: String,
    options: &PlyImportOptions,
) -> Result<ImportedAsset> {
    if doc.vertices.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    let mut warnings = doc.warnings;

    // Convert vertices
    let mut vertices: Vec<Vertex> = doc
        .vertices
        .iter()
        .map(|pv| {
            let pos = if (options.scale - 1.0).abs() > 1e-6 {
                [
                    pv.position[0] * options.scale,
                    pv.position[1] * options.scale,
                    pv.position[2] * options.scale,
                ]
            } else {
                pv.position
            };

            let norm = pv.normal.unwrap_or([0.0, 1.0, 0.0]);
            let uv = pv.uv.unwrap_or([0.0, 0.0]);
            let color = pv.color.unwrap_or(options.default_color);

            Vertex {
                position: pos,
                normal: norm,
                uv,
                color,
            }
        })
        .collect();

    let has_explicit_normals = doc.vertices.iter().any(|v| v.normal.is_some());

    // Triangulate faces
    let mut indices: Vec<u32> = Vec::new();

    for face in &doc.faces {
        let n = face.indices.len();
        if n < 3 {
            continue;
        }

        // Validate index bounds
        if face.indices.iter().any(|&i| i as usize >= vertices.len()) {
            warnings.push(format!(
                "Face with out-of-range index ignored: {:?}",
                face.indices
            ));
            continue;
        }

        if n == 3 {
            indices.push(face.indices[0]);
            indices.push(face.indices[1]);
            indices.push(face.indices[2]);
        } else if n == 4 {
            // Quad split
            indices.push(face.indices[0]);
            indices.push(face.indices[1]);
            indices.push(face.indices[2]);

            indices.push(face.indices[0]);
            indices.push(face.indices[2]);
            indices.push(face.indices[3]);
        } else {
            // Polygon Ear Clipping triangulation
            let polygon_positions: Vec<[f32; 3]> = face
                .indices
                .iter()
                .map(|&idx| vertices[idx as usize].position)
                .collect();

            let local_triangles = triangulate_polygon_3d(&polygon_positions);
            for [i0, i1, i2] in local_triangles {
                indices.push(face.indices[i0]);
                indices.push(face.indices[i1]);
                indices.push(face.indices[i2]);
            }
        }
    }

    if indices.is_empty() {
        return Err(ForgeIoError::NoValidGeometry(
            "PLY asset contained no valid triangle or polygon faces".to_string(),
        ));
    }

    // Recalculate smooth normals if needed
    if !has_explicit_normals && options.generate_normals_if_missing {
        recalculate_smooth_normals(&mut vertices, &indices);
    }

    let mesh = Mesh::new(vertices, indices);

    let imported_obj = ImportedObject {
        name: object_name,
        mesh,
        material_names: Vec::new(),
    };

    Ok(ImportedAsset {
        objects: vec![imported_obj],
        material_libraries: Vec::new(),
        warnings,
    })
}

fn recalculate_smooth_normals(vertices: &mut [Vertex], indices: &[u32]) {
    let mut accumulated = vec![Vec3::ZERO; vertices.len()];

    for chunk in indices.as_chunks::<3>().0 {
        let i0 = chunk[0] as usize;
        let i1 = chunk[1] as usize;
        let i2 = chunk[2] as usize;

        let p0 = Vec3::from(vertices[i0].position);
        let p1 = Vec3::from(vertices[i1].position);
        let p2 = Vec3::from(vertices[i2].position);

        let cross = (p1 - p0).cross(p2 - p0);
        if cross.length_squared() > 1e-12 {
            accumulated[i0] += cross;
            accumulated[i1] += cross;
            accumulated[i2] += cross;
        }
    }

    for (v, norm_sum) in vertices.iter_mut().zip(accumulated.iter()) {
        if norm_sum.length_squared() > 1e-12 {
            v.normal = norm_sum.normalize().into();
        } else {
            v.normal = [0.0, 1.0, 0.0];
        }
    }
}
