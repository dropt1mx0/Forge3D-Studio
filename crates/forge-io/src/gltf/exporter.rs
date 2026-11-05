use super::base64::encode_base64;
use super::types::*;
use crate::error::{ForgeIoError, Result};
use forge_core::ObjectId;
use forge_geometry::Mesh;
use forge_scene::{PrimitiveType, Scene, SceneObject};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// glTF 2.0 export target format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfExportFormat {
    /// Binary self-contained container (.glb) with JSON and BIN chunks.
    Glb,
    /// glTF 2.0 JSON text with embedded Base64 binary data URI (.gltf).
    GltfEmbedded,
}

/// Options configuring glTF 2.0 / GLB asset export.
#[derive(Debug, Clone)]
pub struct GltfExportOptions {
    pub format: GltfExportFormat,
    pub export_selected_only: bool,
    pub selected_ids: Option<HashSet<ObjectId>>,
    pub include_normals: bool,
    pub include_uvs: bool,
    pub include_colors: bool,
    pub include_materials: bool,
}

impl Default for GltfExportOptions {
    fn default() -> Self {
        Self {
            format: GltfExportFormat::Glb,
            export_selected_only: false,
            selected_ids: None,
            include_normals: true,
            include_uvs: true,
            include_colors: true,
            include_materials: true,
        }
    }
}

/// Exports a Scene to a GLB binary container.
pub fn export_scene_to_glb(scene: &Scene, options: &GltfExportOptions) -> Result<Vec<u8>> {
    let mut glb_options = options.clone();
    glb_options.format = GltfExportFormat::Glb;
    let (root, bin) = build_gltf_structures(scene, &glb_options)?;
    assemble_glb_container(&root, &bin)
}

/// Exports a Scene to a glTF 2.0 JSON string with embedded Base64 buffer.
pub fn export_scene_to_gltf_embedded(scene: &Scene, options: &GltfExportOptions) -> Result<String> {
    let mut embedded_options = options.clone();
    embedded_options.format = GltfExportFormat::GltfEmbedded;
    let (mut root, bin) = build_gltf_structures(scene, &embedded_options)?;

    let b64 = encode_base64(&bin);
    let data_uri = format!("data:application/octet-stream;base64,{b64}");
    if let Some(buf) = root.buffers.first_mut() {
        buf.uri = Some(data_uri);
    }

    serde_json::to_string_pretty(&root).map_err(ForgeIoError::from)
}

/// Exports a single Mesh to a GLB binary container.
pub fn export_mesh_to_glb(mesh: &Mesh, name: &str) -> Result<Vec<u8>> {
    let mut scene = Scene::new("ExportScene");
    let obj = SceneObject::new(name.to_string(), PrimitiveType::Custom, mesh.clone());
    scene.add_object(obj);
    export_scene_to_glb(&scene, &GltfExportOptions::default())
}

/// Exports a Scene to file (determining format from extension).
pub fn export_scene_gltf_to_file(
    scene: &Scene,
    path: impl AsRef<Path>,
    options: &GltfExportOptions,
) -> Result<()> {
    let p = path.as_ref();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "glb" || options.format == GltfExportFormat::Glb {
        let bytes = export_scene_to_glb(scene, options)?;
        std::fs::write(p, bytes)?;
    } else {
        let json_str = export_scene_to_gltf_embedded(scene, options)?;
        std::fs::write(p, json_str)?;
    }

    Ok(())
}

fn build_gltf_structures(
    scene: &Scene,
    options: &GltfExportOptions,
) -> Result<(GltfRoot, Vec<u8>)> {
    let objects: Vec<&SceneObject> = scene
        .objects
        .values()
        .filter(|obj| {
            if !options.export_selected_only {
                return true;
            }
            if let Some(selected) = &options.selected_ids {
                selected.contains(&obj.id)
            } else {
                true
            }
        })
        .collect();

    if objects.is_empty() {
        return Err(ForgeIoError::ExportError(
            "No objects available to export to glTF".to_string(),
        ));
    }

    let mut root = GltfRoot {
        asset: GltfAsset {
            version: "2.0".to_string(),
            generator: Some("Forge3D Rust Edition".to_string()),
        },
        scene: Some(0),
        scenes: vec![GltfScene {
            name: Some("Scene".to_string()),
            nodes: Vec::new(),
        }],
        nodes: Vec::new(),
        meshes: Vec::new(),
        materials: Vec::new(),
        accessors: Vec::new(),
        buffer_views: Vec::new(),
        buffers: Vec::new(),
    };

    let mut bin = Vec::new();
    let mut material_map: HashMap<String, usize> = HashMap::new();

    // 1. Process Materials if enabled
    if options.include_materials {
        for (mat_id, mat) in &scene.materials {
            let pbr_mr = GltfPbrMetallicRoughness {
                base_color_factor: Some(mat.base_color_factor),
                metallic_factor: Some(mat.metallic_factor),
                roughness_factor: Some(mat.roughness_factor),
            };

            let emissive_sq = mat.emissive_factor[0] * mat.emissive_factor[0]
                + mat.emissive_factor[1] * mat.emissive_factor[1]
                + mat.emissive_factor[2] * mat.emissive_factor[2];

            let emissive = if emissive_sq > 1e-5 {
                Some(mat.emissive_factor)
            } else {
                None
            };

            let mat_idx = root.materials.len();
            root.materials.push(GltfMaterial {
                name: Some(mat.name.clone()),
                pbr_metallic_roughness: Some(pbr_mr),
                emissive_factor: emissive,
                double_sided: Some(true),
            });
            material_map.insert(mat_id.clone(), mat_idx);
            material_map.insert(mat.name.clone(), mat_idx);
        }
    }

    // 2. Process Objects and Meshes
    for (node_idx, obj) in objects.iter().enumerate() {
        let mesh = &obj.mesh;
        if mesh.vertices.is_empty() {
            continue;
        }

        let mut attributes = HashMap::new();

        // A. POSITION Accessor
        let pos_bv_idx = root.buffer_views.len();
        align_4(&mut bin);
        let pos_start = bin.len();
        let mut min_pos = [f64::INFINITY; 3];
        let mut max_pos = [f64::NEG_INFINITY; 3];

        for v in &mesh.vertices {
            let pos = v.position;
            min_pos[0] = min_pos[0].min(pos[0] as f64);
            min_pos[1] = min_pos[1].min(pos[1] as f64);
            min_pos[2] = min_pos[2].min(pos[2] as f64);
            max_pos[0] = max_pos[0].max(pos[0] as f64);
            max_pos[1] = max_pos[1].max(pos[1] as f64);
            max_pos[2] = max_pos[2].max(pos[2] as f64);

            bin.extend_from_slice(&pos[0].to_le_bytes());
            bin.extend_from_slice(&pos[1].to_le_bytes());
            bin.extend_from_slice(&pos[2].to_le_bytes());
        }
        let pos_len = bin.len() - pos_start;

        root.buffer_views.push(GltfBufferView {
            buffer: 0,
            byte_offset: Some(pos_start),
            byte_length: pos_len,
            byte_stride: None,
            target: Some(TARGET_ARRAY_BUFFER),
        });

        let pos_acc_idx = root.accessors.len();
        root.accessors.push(GltfAccessor {
            buffer_view: Some(pos_bv_idx),
            byte_offset: Some(0),
            component_type: COMPONENT_TYPE_FLOAT,
            count: mesh.vertices.len(),
            type_name: "VEC3".to_string(),
            min: Some(min_pos.to_vec()),
            max: Some(max_pos.to_vec()),
        });
        attributes.insert("POSITION".to_string(), pos_acc_idx);

        // B. NORMAL Accessor
        if options.include_normals {
            let norm_bv_idx = root.buffer_views.len();
            align_4(&mut bin);
            let norm_start = bin.len();
            for v in &mesh.vertices {
                bin.extend_from_slice(&v.normal[0].to_le_bytes());
                bin.extend_from_slice(&v.normal[1].to_le_bytes());
                bin.extend_from_slice(&v.normal[2].to_le_bytes());
            }
            let norm_len = bin.len() - norm_start;

            root.buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: Some(norm_start),
                byte_length: norm_len,
                byte_stride: None,
                target: Some(TARGET_ARRAY_BUFFER),
            });

            let norm_acc_idx = root.accessors.len();
            root.accessors.push(GltfAccessor {
                buffer_view: Some(norm_bv_idx),
                byte_offset: Some(0),
                component_type: COMPONENT_TYPE_FLOAT,
                count: mesh.vertices.len(),
                type_name: "VEC3".to_string(),
                min: None,
                max: None,
            });
            attributes.insert("NORMAL".to_string(), norm_acc_idx);
        }

        // C. TEXCOORD_0 Accessor
        if options.include_uvs {
            let uv_bv_idx = root.buffer_views.len();
            align_4(&mut bin);
            let uv_start = bin.len();
            for v in &mesh.vertices {
                bin.extend_from_slice(&v.uv[0].to_le_bytes());
                bin.extend_from_slice(&v.uv[1].to_le_bytes());
            }
            let uv_len = bin.len() - uv_start;

            root.buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: Some(uv_start),
                byte_length: uv_len,
                byte_stride: None,
                target: Some(TARGET_ARRAY_BUFFER),
            });

            let uv_acc_idx = root.accessors.len();
            root.accessors.push(GltfAccessor {
                buffer_view: Some(uv_bv_idx),
                byte_offset: Some(0),
                component_type: COMPONENT_TYPE_FLOAT,
                count: mesh.vertices.len(),
                type_name: "VEC2".to_string(),
                min: None,
                max: None,
            });
            attributes.insert("TEXCOORD_0".to_string(), uv_acc_idx);
        }

        // D. COLOR_0 Accessor
        if options.include_colors {
            let col_bv_idx = root.buffer_views.len();
            align_4(&mut bin);
            let col_start = bin.len();
            for v in &mesh.vertices {
                bin.extend_from_slice(&v.color[0].to_le_bytes());
                bin.extend_from_slice(&v.color[1].to_le_bytes());
                bin.extend_from_slice(&v.color[2].to_le_bytes());
                bin.extend_from_slice(&v.color[3].to_le_bytes());
            }
            let col_len = bin.len() - col_start;

            root.buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: Some(col_start),
                byte_length: col_len,
                byte_stride: None,
                target: Some(TARGET_ARRAY_BUFFER),
            });

            let col_acc_idx = root.accessors.len();
            root.accessors.push(GltfAccessor {
                buffer_view: Some(col_bv_idx),
                byte_offset: Some(0),
                component_type: COMPONENT_TYPE_FLOAT,
                count: mesh.vertices.len(),
                type_name: "VEC4".to_string(),
                min: None,
                max: None,
            });
            attributes.insert("COLOR_0".to_string(), col_acc_idx);
        }

        // E. INDICES Accessor
        let ind_acc_idx = if !mesh.indices.is_empty() {
            let ind_bv_idx = root.buffer_views.len();
            align_4(&mut bin);
            let ind_start = bin.len();
            for &idx in &mesh.indices {
                bin.extend_from_slice(&idx.to_le_bytes());
            }
            let ind_len = bin.len() - ind_start;

            root.buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: Some(ind_start),
                byte_length: ind_len,
                byte_stride: None,
                target: Some(TARGET_ELEMENT_ARRAY_BUFFER),
            });

            let acc_idx = root.accessors.len();
            root.accessors.push(GltfAccessor {
                buffer_view: Some(ind_bv_idx),
                byte_offset: Some(0),
                component_type: COMPONENT_TYPE_UNSIGNED_INT,
                count: mesh.indices.len(),
                type_name: "SCALAR".to_string(),
                min: None,
                max: None,
            });
            Some(acc_idx)
        } else {
            None
        };

        // Material Mapping
        let mat_idx = obj
            .material_id
            .as_ref()
            .and_then(|id| material_map.get(id).copied());

        let mesh_idx = root.meshes.len();
        root.meshes.push(GltfMesh {
            name: Some(obj.name.clone()),
            primitives: vec![GltfPrimitive {
                attributes,
                indices: ind_acc_idx,
                material: mat_idx,
                mode: Some(PRIMITIVE_MODE_TRIANGLES),
            }],
        });

        // Node Definition
        let translation = [
            obj.local_transform.translation.x,
            obj.local_transform.translation.y,
            obj.local_transform.translation.z,
        ];
        let rotation = [
            obj.local_transform.rotation.x,
            obj.local_transform.rotation.y,
            obj.local_transform.rotation.z,
            obj.local_transform.rotation.w,
        ];
        let scale = [
            obj.local_transform.scale.x,
            obj.local_transform.scale.y,
            obj.local_transform.scale.z,
        ];

        let has_translation = obj.local_transform.translation.length_squared() > 1e-6;
        let has_rotation = (obj.local_transform.rotation.w - 1.0).abs() > 1e-5
            || obj.local_transform.rotation.x.abs() > 1e-5
            || obj.local_transform.rotation.y.abs() > 1e-5
            || obj.local_transform.rotation.z.abs() > 1e-5;
        let has_scale = (obj.local_transform.scale - forge_math::Vec3::ONE).length_squared() > 1e-6;

        root.nodes.push(GltfNode {
            name: Some(obj.name.clone()),
            mesh: Some(mesh_idx),
            translation: if has_translation {
                Some(translation)
            } else {
                None
            },
            rotation: if has_rotation { Some(rotation) } else { None },
            scale: if has_scale { Some(scale) } else { None },
            children: Vec::new(),
        });

        root.scenes[0].nodes.push(node_idx);
    }

    align_4(&mut bin);
    root.buffers.push(GltfBuffer {
        byte_length: bin.len(),
        uri: None,
    });

    Ok((root, bin))
}

fn align_4(vec: &mut Vec<u8>) {
    let rem = vec.len() % 4;
    if rem != 0 {
        vec.resize(vec.len() + (4 - rem), 0);
    }
}

fn assemble_glb_container(root: &GltfRoot, bin: &[u8]) -> Result<Vec<u8>> {
    let json_bytes = serde_json::to_vec(root)?;
    let json_pad = (4 - (json_bytes.len() % 4)) % 4;
    let json_chunk_len = json_bytes.len() + json_pad;

    let bin_pad = (4 - (bin.len() % 4)) % 4;
    let bin_chunk_len = bin.len() + bin_pad;

    let total_len = 12 + 8 + json_chunk_len + 8 + bin_chunk_len;
    let mut glb = Vec::with_capacity(total_len);

    // 1. GLB Header (12 bytes)
    glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    glb.extend_from_slice(&GLB_VERSION_2.to_le_bytes());
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());

    // 2. JSON Chunk (8 bytes header + padded data)
    glb.extend_from_slice(&(json_chunk_len as u32).to_le_bytes());
    glb.extend_from_slice(&CHUNK_TYPE_JSON.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.resize(glb.len() + json_pad, b' '); // Pad JSON with spaces (0x20)

    // 3. BIN Chunk (8 bytes header + padded data)
    glb.extend_from_slice(&(bin_chunk_len as u32).to_le_bytes());
    glb.extend_from_slice(&CHUNK_TYPE_BIN.to_le_bytes());
    glb.extend_from_slice(bin);
    glb.resize(glb.len() + bin_pad, 0x00); // Pad BIN with zeros (0x00)

    Ok(glb)
}
