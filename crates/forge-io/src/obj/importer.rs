use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use forge_core::id::ObjectId;
use forge_geometry::{Mesh, Vertex};
use forge_math::Transform;
use forge_scene::{PrimitiveType, SceneObject};

use super::parser::{ObjDocument, parse_obj, parse_obj_from_str};
use super::triangulation::triangulate_polygon_3d;
use crate::error::{ForgeIoError, Result};

/// Configuration options controlling how OBJ data is parsed and imported.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// If normals are absent from the OBJ file, automatically calculate smooth surface normals.
    pub generate_normals_if_missing: bool,
    /// Default vertex color to assign if OBJ material colors are not provided.
    pub default_color: [f32; 4],
    /// Global uniform scale multiplier applied to all vertex positions.
    pub scale: f32,
    /// If true, merge all objects and groups into a single combined Mesh.
    pub merge_objects: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            generate_normals_if_missing: true,
            default_color: [0.8, 0.8, 0.85, 1.0],
            scale: 1.0,
            merge_objects: false,
        }
    }
}

/// A discrete geometric mesh imported from an OBJ file with metadata.
#[derive(Debug, Clone)]
pub struct ImportedObject {
    pub name: String,
    pub mesh: Mesh,
    pub material_names: Vec<String>,
}

/// High-level result of importing a Wavefront OBJ asset.
#[derive(Debug, Clone)]
pub struct ImportedAsset {
    pub objects: Vec<ImportedObject>,
    pub material_libraries: Vec<String>,
    pub warnings: Vec<String>,
}

impl ImportedAsset {
    /// Total number of unique vertices across all imported objects.
    pub fn total_vertices(&self) -> usize {
        self.objects.iter().map(|o| o.mesh.vertices.len()).sum()
    }

    /// Total number of triangles across all imported objects.
    pub fn total_triangles(&self) -> usize {
        self.objects.iter().map(|o| o.mesh.indices.len() / 3).sum()
    }

    /// Converts imported objects into Forge3D `SceneObject` instances ready for insertion into a `Scene`.
    pub fn into_scene_objects(self) -> Vec<SceneObject> {
        self.objects
            .into_iter()
            .map(|io| {
                let mut obj = SceneObject::new(io.name, PrimitiveType::Custom, io.mesh);
                obj.id = ObjectId::new();
                obj.local_transform = Transform::IDENTITY;
                obj
            })
            .collect()
    }
}

/// Imports a Wavefront OBJ file from disk with specified options.
pub fn import_obj(path: impl AsRef<Path>, options: &ImportOptions) -> Result<ImportedAsset> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let doc = parse_obj(reader)?;
    build_asset_from_doc(doc, options)
}

/// Imports a Wavefront OBJ from an in-memory string with specified options.
pub fn import_obj_from_str(obj_str: &str, options: &ImportOptions) -> Result<ImportedAsset> {
    let doc = parse_obj_from_str(obj_str)?;
    build_asset_from_doc(doc, options)
}

/// Converts a parsed `ObjDocument` into an `ImportedAsset` with triangulated meshes.
fn build_asset_from_doc(doc: ObjDocument, options: &ImportOptions) -> Result<ImportedAsset> {
    if doc.positions.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    let mut warnings = doc.warnings;
    let mut imported_objects = Vec::new();

    // Prepare scaled positions if scale != 1.0
    let positions: Vec<[f32; 3]> = if (options.scale - 1.0).abs() > 1e-6 {
        doc.positions
            .iter()
            .map(|p| {
                [
                    p[0] * options.scale,
                    p[1] * options.scale,
                    p[2] * options.scale,
                ]
            })
            .collect()
    } else {
        doc.positions
    };

    if options.merge_objects && !doc.objects.is_empty() {
        // Merge all faces into a single unified object
        let mut combined_faces = Vec::new();
        let mut all_materials = Vec::new();
        for obj in &doc.objects {
            for f in &obj.faces {
                if let Some(m) = &f.material
                    && !all_materials.contains(m)
                {
                    all_materials.push(m.clone());
                }
                combined_faces.push(f.clone());
            }
        }

        if let Some(mesh) = convert_faces_to_mesh(
            &combined_faces,
            &positions,
            &doc.tex_coords,
            &doc.normals,
            options,
            &mut warnings,
        )? {
            imported_objects.push(ImportedObject {
                name: "Combined_Mesh".to_string(),
                mesh,
                material_names: all_materials,
            });
        }
    } else {
        // Maintain distinct objects
        for (idx, obj) in doc.objects.iter().enumerate() {
            if obj.faces.is_empty() {
                continue;
            }

            let mut object_materials = Vec::new();
            for f in &obj.faces {
                if let Some(m) = &f.material
                    && !object_materials.contains(m)
                {
                    object_materials.push(m.clone());
                }
            }

            let name = if obj.name.trim().is_empty() {
                format!("Imported_Object_{}", idx + 1)
            } else {
                obj.name.clone()
            };

            if let Some(mesh) = convert_faces_to_mesh(
                &obj.faces,
                &positions,
                &doc.tex_coords,
                &doc.normals,
                options,
                &mut warnings,
            )? {
                imported_objects.push(ImportedObject {
                    name,
                    mesh,
                    material_names: object_materials,
                });
            }
        }
    }

    if imported_objects.is_empty() {
        return Err(ForgeIoError::NoValidGeometry(
            "OBJ document contained no valid faces or triangulated geometry".to_string(),
        ));
    }

    Ok(ImportedAsset {
        objects: imported_objects,
        material_libraries: doc.material_libraries,
        warnings,
    })
}

/// Converts a set of OBJ faces into a welded, triangulated `Mesh`.
fn convert_faces_to_mesh(
    faces: &[super::parser::ObjFace],
    positions: &[[f32; 3]],
    tex_coords: &[[f32; 2]],
    normals: &[[f32; 3]],
    options: &ImportOptions,
    warnings: &mut Vec<String>,
) -> Result<Option<Mesh>> {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut vertex_map: HashMap<(usize, Option<usize>, Option<usize>), u32> = HashMap::new();
    let mut any_missing_normals = false;

    for face in faces {
        if face.vertices.len() < 3 {
            continue;
        }

        // Collect 3D positions for triangulation
        let face_positions: Vec<[f32; 3]> = face
            .vertices
            .iter()
            .map(|fv| positions[fv.position_index])
            .collect();

        // Perform 3D Ear Clipping triangulation
        let triangles = triangulate_polygon_3d(&face_positions);

        for tri in triangles {
            for &corner_idx in &tri {
                let fv = &face.vertices[corner_idx];
                let key = (fv.position_index, fv.tex_coord_index, fv.normal_index);

                if let Some(&existing_idx) = vertex_map.get(&key) {
                    indices.push(existing_idx);
                } else {
                    let new_idx = vertices.len() as u32;

                    let pos = positions[fv.position_index];
                    let uv = fv
                        .tex_coord_index
                        .and_then(|idx| tex_coords.get(idx).copied())
                        .unwrap_or([0.0, 0.0]);

                    let normal = if let Some(n_idx) = fv.normal_index {
                        normals.get(n_idx).copied().unwrap_or([0.0, 0.0, 0.0])
                    } else {
                        any_missing_normals = true;
                        [0.0, 0.0, 0.0]
                    };

                    let vertex = Vertex::new(pos, normal, uv, options.default_color);
                    vertices.push(vertex);
                    vertex_map.insert(key, new_idx);
                    indices.push(new_idx);
                }
            }
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Ok(None);
    }

    let mut mesh = Mesh::new(vertices, indices);

    // Calculate normals if they were absent and requested
    if any_missing_normals && options.generate_normals_if_missing {
        mesh.recalculate_normals();
    }

    mesh.recalculate_aabb();

    // Verify numerical invariants
    if let Err(e) = mesh.validate() {
        warnings.push(format!("Mesh validation notice: {e}"));
    }

    Ok(Some(mesh))
}
