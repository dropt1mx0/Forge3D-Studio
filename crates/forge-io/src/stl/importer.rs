use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;

use super::parser::{StlDocument, StlFacet, parse_stl, parse_stl_from_bytes};
use crate::error::{ForgeIoError, Result};
use crate::obj::{ImportedAsset, ImportedObject};

/// Configuration options controlling how STL files are imported.
#[derive(Debug, Clone)]
pub struct StlImportOptions {
    /// Global scale factor applied to vertex positions.
    pub scale: f32,
    /// Whether to weld co-located vertices within tolerance to form a shared-index manifold mesh.
    pub weld_vertices: bool,
    /// Spatial quantization tolerance for vertex welding (default: 1e-5).
    pub weld_tolerance: f32,
    /// Recalculate smooth vertex normals from facet geometry.
    pub recalculate_normals: bool,
    /// Default vertex RGBA color assigned to the mesh.
    pub default_color: [f32; 4],
    /// Custom object name overriding the STL header name.
    pub custom_name: Option<String>,
}

impl Default for StlImportOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            weld_vertices: true,
            weld_tolerance: 1e-5,
            recalculate_normals: true,
            default_color: [0.75, 0.75, 0.8, 1.0],
            custom_name: None,
        }
    }
}

/// Imports an STL asset from a file on disk.
pub fn import_stl(path: impl AsRef<Path>, options: &StlImportOptions) -> Result<ImportedAsset> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let doc = parse_stl(reader)?;
    build_mesh_from_stl(doc, options)
}

/// Imports an STL asset from an in-memory byte slice.
pub fn import_stl_from_bytes(bytes: &[u8], options: &StlImportOptions) -> Result<ImportedAsset> {
    let doc = parse_stl_from_bytes(bytes)?;
    build_mesh_from_stl(doc, options)
}

fn build_mesh_from_stl(doc: StlDocument, options: &StlImportOptions) -> Result<ImportedAsset> {
    if doc.facets.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    let object_name = options
        .custom_name
        .clone()
        .unwrap_or_else(|| doc.name.clone());

    let mesh = if options.weld_vertices {
        build_welded_mesh(&doc.facets, options)?
    } else {
        build_unwelded_mesh(&doc.facets, options)?
    };

    let imported_obj = ImportedObject {
        name: object_name,
        mesh,
        material_names: Vec::new(),
    };

    Ok(ImportedAsset {
        objects: vec![imported_obj],
        material_libraries: Vec::new(),
        warnings: doc.warnings,
    })
}

/// Builds an unwelded mesh where each triangle facet has 3 independent vertices.
fn build_unwelded_mesh(facets: &[StlFacet], options: &StlImportOptions) -> Result<Mesh> {
    let vertex_count = facets.len() * 3;
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(vertex_count);

    for (facet_idx, facet) in facets.iter().enumerate() {
        let v0 = scale_pos(facet.vertices[0], options.scale);
        let v1 = scale_pos(facet.vertices[1], options.scale);
        let v2 = scale_pos(facet.vertices[2], options.scale);

        let facet_normal = compute_or_validate_normal(facet.normal, v0, v1, v2);

        let base_idx = (facet_idx * 3) as u32;
        vertices.push(Vertex {
            position: v0,
            normal: facet_normal,
            uv: [0.0, 0.0],
            color: options.default_color,
        });
        vertices.push(Vertex {
            position: v1,
            normal: facet_normal,
            uv: [1.0, 0.0],
            color: options.default_color,
        });
        vertices.push(Vertex {
            position: v2,
            normal: facet_normal,
            uv: [0.5, 1.0],
            color: options.default_color,
        });

        indices.push(base_idx);
        indices.push(base_idx + 1);
        indices.push(base_idx + 2);
    }

    let mesh = Mesh::new(vertices, indices);
    Ok(mesh)
}

/// Builds a welded mesh using spatial quantization, accumulating area-weighted vertex normals.
fn build_welded_mesh(facets: &[StlFacet], options: &StlImportOptions) -> Result<Mesh> {
    let quant_factor = if options.weld_tolerance > 0.0 {
        (1.0 / options.weld_tolerance).max(1.0)
    } else {
        100_000.0
    };

    let mut vertex_map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut accumulated_normals: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::with_capacity(facets.len() * 3);

    for facet in facets {
        let v0 = scale_pos(facet.vertices[0], options.scale);
        let v1 = scale_pos(facet.vertices[1], options.scale);
        let v2 = scale_pos(facet.vertices[2], options.scale);

        // Calculate face normal and cross product (weighted by 2x triangle area)
        let edge1 = Vec3::from(v1) - Vec3::from(v0);
        let edge2 = Vec3::from(v2) - Vec3::from(v0);
        let cross = edge1.cross(edge2);
        let face_normal = if cross.length_squared() > 1e-12 {
            cross
        } else if is_valid_normal(facet.normal) {
            Vec3::from(facet.normal)
        } else {
            Vec3::Y
        };

        let mut facet_indices = [0u32; 3];
        for (i, &pos) in [v0, v1, v2].iter().enumerate() {
            let key = quantize_pos(pos, quant_factor);
            let idx = match vertex_map.get(&key) {
                Some(&existing_idx) => {
                    accumulated_normals[existing_idx as usize] += face_normal;
                    existing_idx
                }
                None => {
                    let new_idx = positions.len() as u32;
                    positions.push(pos);
                    accumulated_normals.push(face_normal);
                    vertex_map.insert(key, new_idx);
                    new_idx
                }
            };
            facet_indices[i] = idx;
        }

        // Avoid degenerate collapsed triangle indices in welded mesh
        if facet_indices[0] != facet_indices[1]
            && facet_indices[1] != facet_indices[2]
            && facet_indices[0] != facet_indices[2]
        {
            indices.push(facet_indices[0]);
            indices.push(facet_indices[1]);
            indices.push(facet_indices[2]);
        }
    }

    if positions.is_empty() {
        return Err(ForgeIoError::NoValidGeometry(
            "All facets in STL were degenerate or collapsed".to_string(),
        ));
    }

    let vertices: Vec<Vertex> = positions
        .into_iter()
        .zip(accumulated_normals)
        .map(|(pos, normal_sum)| {
            let n = if options.recalculate_normals && normal_sum.length_squared() > 1e-12 {
                normal_sum.normalize().into()
            } else {
                [0.0, 1.0, 0.0]
            };
            Vertex {
                position: pos,
                normal: n,
                uv: [0.0, 0.0],
                color: options.default_color,
            }
        })
        .collect();

    let mesh = Mesh::new(vertices, indices);
    Ok(mesh)
}

#[inline(always)]
fn scale_pos(pos: [f32; 3], scale: f32) -> [f32; 3] {
    if (scale - 1.0).abs() < 1e-6 {
        pos
    } else {
        [pos[0] * scale, pos[1] * scale, pos[2] * scale]
    }
}

#[inline(always)]
fn quantize_pos(pos: [f32; 3], factor: f32) -> (i64, i64, i64) {
    (
        (pos[0] * factor).round() as i64,
        (pos[1] * factor).round() as i64,
        (pos[2] * factor).round() as i64,
    )
}

#[inline(always)]
fn is_valid_normal(n: [f32; 3]) -> bool {
    let len_sq = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
    len_sq > 1e-6 && len_sq < 100.0 && n[0].is_finite() && n[1].is_finite() && n[2].is_finite()
}

fn compute_or_validate_normal(
    reported: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> [f32; 3] {
    if is_valid_normal(reported) {
        let len =
            (reported[0] * reported[0] + reported[1] * reported[1] + reported[2] * reported[2])
                .sqrt();
        [reported[0] / len, reported[1] / len, reported[2] / len]
    } else {
        let e1 = Vec3::from(v1) - Vec3::from(v0);
        let e2 = Vec3::from(v2) - Vec3::from(v0);
        let cross = e1.cross(e2);
        if cross.length_squared() > 1e-12 {
            cross.normalize().into()
        } else {
            [0.0, 1.0, 0.0]
        }
    }
}
