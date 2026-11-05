use super::base64::decode_base64;
use super::types::*;
use crate::error::{ForgeIoError, Result};
use forge_geometry::{Mesh, Vertex};
use forge_math::{Quat, Transform, Vec3};
use forge_scene::{PbrMaterial, PrimitiveType, SceneObject};
use std::path::Path;

/// Options configuring glTF 2.0 / GLB asset import.
#[derive(Debug, Clone)]
pub struct GltfImportOptions {
    /// Whether to generate normals if the glTF asset does not supply them.
    pub calculate_missing_normals: bool,
    /// Whether to invert the V coordinate of UV texture coordinates.
    pub flip_v_coordinates: bool,
}

impl Default for GltfImportOptions {
    fn default() -> Self {
        Self {
            calculate_missing_normals: true,
            flip_v_coordinates: false,
        }
    }
}

/// The result of importing a glTF 2.0 / GLB asset.
#[derive(Debug, Clone)]
pub struct GltfImportResult {
    pub objects: Vec<SceneObject>,
    pub materials: Vec<PbrMaterial>,
}

/// Imports glTF 2.0 or GLB from file path.
pub fn import_gltf(
    path: impl AsRef<Path>,
    options: &GltfImportOptions,
) -> Result<GltfImportResult> {
    let p = path.as_ref();
    let bytes = std::fs::read(p)?;
    let base_dir = p.parent();
    import_gltf_from_bytes(&bytes, options, base_dir)
}

/// Imports glTF 2.0 or GLB from raw bytes.
pub fn import_gltf_from_bytes(
    bytes: &[u8],
    options: &GltfImportOptions,
    base_dir: Option<&Path>,
) -> Result<GltfImportResult> {
    if bytes.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    let (root, buffers) = if is_glb(bytes) {
        parse_glb_container(bytes)?
    } else {
        parse_gltf_json_container(bytes, base_dir)?
    };

    convert_gltf_to_scene(root, &buffers, options)
}

fn is_glb(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == GLB_MAGIC
}

fn parse_glb_container(bytes: &[u8]) -> Result<(GltfRoot, Vec<Vec<u8>>)> {
    if bytes.len() < 12 {
        return Err(ForgeIoError::TruncatedData {
            format: "GLB",
            expected: 12,
            found: bytes.len(),
        });
    }

    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let total_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;

    if magic != GLB_MAGIC {
        return Err(ForgeIoError::InvalidHeader {
            format: "GLB",
            message: format!("Invalid magic: expected 0x{GLB_MAGIC:08X}, found 0x{magic:08X}"),
        });
    }

    if version != GLB_VERSION_2 {
        return Err(ForgeIoError::UnsupportedFormat {
            format: "GLB",
            message: format!("Only glTF 2.0 (version 2) is supported, found version {version}"),
        });
    }

    if bytes.len() < total_len {
        return Err(ForgeIoError::TruncatedData {
            format: "GLB",
            expected: total_len,
            found: bytes.len(),
        });
    }

    let mut cursor = 12;
    let mut json_root: Option<GltfRoot> = None;
    let mut bin_chunk: Option<Vec<u8>> = None;

    while cursor + 8 <= total_len {
        let chunk_len = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        let chunk_type = u32::from_le_bytes([
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;

        if cursor + chunk_len > bytes.len() {
            return Err(ForgeIoError::TruncatedData {
                format: "GLB",
                expected: cursor + chunk_len,
                found: bytes.len(),
            });
        }

        let chunk_bytes = &bytes[cursor..cursor + chunk_len];
        cursor += chunk_len;

        match chunk_type {
            CHUNK_TYPE_JSON => {
                let json_str =
                    std::str::from_utf8(chunk_bytes).map_err(|e| ForgeIoError::ParseError {
                        line: 0,
                        message: format!("Invalid UTF-8 in glTF JSON chunk: {e}"),
                    })?;
                let root: GltfRoot = serde_json::from_str(json_str)?;
                json_root = Some(root);
            }
            CHUNK_TYPE_BIN => {
                bin_chunk = Some(chunk_bytes.to_vec());
            }
            _ => {
                // Unknown chunk type - safely skip according to glTF spec
            }
        }
    }

    let root = json_root.ok_or_else(|| ForgeIoError::ParseError {
        line: 0,
        message: "Missing JSON chunk in GLB container".to_string(),
    })?;

    let mut buffers = Vec::new();
    if let Some(bin) = bin_chunk {
        buffers.push(bin);
    }

    Ok((root, buffers))
}

fn parse_gltf_json_container(
    bytes: &[u8],
    base_dir: Option<&Path>,
) -> Result<(GltfRoot, Vec<Vec<u8>>)> {
    let json_str = std::str::from_utf8(bytes).map_err(|e| ForgeIoError::ParseError {
        line: 0,
        message: format!("Invalid UTF-8 in glTF file: {e}"),
    })?;
    let root: GltfRoot = serde_json::from_str(json_str)?;

    let mut buffers = Vec::with_capacity(root.buffers.len());
    for buffer in &root.buffers {
        if let Some(uri) = &buffer.uri {
            if uri.starts_with("data:") {
                if let Some(comma_idx) = uri.find(',') {
                    let header = &uri[..comma_idx];
                    let payload = &uri[comma_idx + 1..];
                    if header.contains(";base64") {
                        let decoded =
                            decode_base64(payload).ok_or_else(|| ForgeIoError::ParseError {
                                line: 0,
                                message: "Malformed Base64 in glTF data URI".to_string(),
                            })?;
                        buffers.push(decoded);
                    } else {
                        buffers.push(payload.as_bytes().to_vec());
                    }
                } else {
                    return Err(ForgeIoError::ParseError {
                        line: 0,
                        message: "Invalid data URI in glTF buffer".to_string(),
                    });
                }
            } else if let Some(dir) = base_dir {
                let file_path = dir.join(uri);
                let data = std::fs::read(&file_path)?;
                buffers.push(data);
            } else {
                return Err(ForgeIoError::UnsupportedFormat {
                    format: "glTF",
                    message: format!("External buffer reference {uri} without base directory"),
                });
            }
        } else {
            buffers.push(Vec::new());
        }
    }

    Ok((root, buffers))
}

fn convert_gltf_to_scene(
    root: GltfRoot,
    buffers: &[Vec<u8>],
    options: &GltfImportOptions,
) -> Result<GltfImportResult> {
    // 1. Convert Materials
    let mut materials = Vec::with_capacity(root.materials.len());
    let mut material_id_map = Vec::with_capacity(root.materials.len());

    for (i, mat) in root.materials.iter().enumerate() {
        let name = mat
            .name
            .clone()
            .unwrap_or_else(|| format!("GltfMaterial_{i}"));
        let mut pbr = PbrMaterial {
            name: name.clone(),
            ..Default::default()
        };

        if let Some(pbr_mr) = &mat.pbr_metallic_roughness {
            if let Some(bc) = pbr_mr.base_color_factor {
                pbr.base_color_factor = bc;
            }
            if let Some(m) = pbr_mr.metallic_factor {
                pbr.metallic_factor = m;
            }
            if let Some(r) = pbr_mr.roughness_factor {
                pbr.roughness_factor = r;
            }
        }

        if let Some(em) = mat.emissive_factor {
            pbr.emissive_factor = em;
        }

        material_id_map.push(pbr.name.clone());
        materials.push(pbr);
    }

    // 2. Convert Nodes and Meshes
    let mut objects = Vec::new();

    for (node_idx, node) in root.nodes.iter().enumerate() {
        if let Some(mesh_idx) = node.mesh {
            if mesh_idx >= root.meshes.len() {
                continue;
            }
            let gltf_mesh = &root.meshes[mesh_idx];

            let transform = {
                let translation = node.translation.map(Vec3::from_array).unwrap_or(Vec3::ZERO);
                let rotation = node
                    .rotation
                    .map(|r| Quat::from_xyzw(r[0], r[1], r[2], r[3]))
                    .unwrap_or(Quat::IDENTITY);
                let scale = node.scale.map(Vec3::from_array).unwrap_or(Vec3::ONE);
                Transform {
                    translation,
                    rotation,
                    scale,
                }
            };

            for (prim_idx, prim) in gltf_mesh.primitives.iter().enumerate() {
                let mode = prim.mode.unwrap_or(PRIMITIVE_MODE_TRIANGLES);
                if mode != PRIMITIVE_MODE_TRIANGLES {
                    continue;
                }

                let pos_accessor_idx = match prim.attributes.get("POSITION") {
                    Some(&idx) => idx,
                    None => continue,
                };

                let positions = read_vec3_accessor(&root, buffers, pos_accessor_idx)?;
                if positions.is_empty() {
                    continue;
                }

                let normals = if let Some(&norm_idx) = prim.attributes.get("NORMAL") {
                    read_vec3_accessor(&root, buffers, norm_idx)?
                } else if options.calculate_missing_normals {
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                } else {
                    Vec::new()
                };

                let uvs = if let Some(&uv_idx) = prim.attributes.get("TEXCOORD_0") {
                    let mut read_uvs = read_vec2_accessor(&root, buffers, uv_idx)?;
                    if options.flip_v_coordinates {
                        for uv in &mut read_uvs {
                            uv[1] = 1.0 - uv[1];
                        }
                    }
                    read_uvs
                } else {
                    Vec::new()
                };

                let colors = if let Some(&col_idx) = prim.attributes.get("COLOR_0") {
                    read_vec4_accessor(&root, buffers, col_idx).unwrap_or_default()
                } else {
                    Vec::new()
                };

                let indices = if let Some(ind_accessor_idx) = prim.indices {
                    read_indices_accessor(&root, buffers, ind_accessor_idx)?
                } else {
                    (0..positions.len() as u32).collect()
                };

                let mut calculated_normals = normals;
                if calculated_normals.len() != positions.len() && options.calculate_missing_normals
                {
                    calculated_normals = calculate_flat_normals(&positions, &indices);
                }

                let mut vertices = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    let pos = positions[i];
                    let norm = if i < calculated_normals.len() {
                        calculated_normals[i]
                    } else {
                        [0.0, 1.0, 0.0]
                    };
                    let uv = if i < uvs.len() { uvs[i] } else { [0.0, 0.0] };
                    let color = if i < colors.len() {
                        colors[i]
                    } else {
                        [1.0, 1.0, 1.0, 1.0]
                    };
                    vertices.push(Vertex {
                        position: pos,
                        normal: norm,
                        uv,
                        color,
                    });
                }

                let mesh = Mesh::new(vertices, indices);

                let object_name = node
                    .name
                    .clone()
                    .or_else(|| gltf_mesh.name.clone())
                    .unwrap_or_else(|| format!("GltfObject_{node_idx}_{prim_idx}"));

                let mat_id = prim
                    .material
                    .and_then(|idx| material_id_map.get(idx).cloned());

                let mut scene_obj = SceneObject::new(object_name, PrimitiveType::Custom, mesh);
                scene_obj.local_transform = transform;
                scene_obj.material_id = mat_id;
                objects.push(scene_obj);
            }
        }
    }

    if objects.is_empty() {
        return Err(ForgeIoError::NoValidGeometry(
            "glTF asset contained no convertible triangle mesh primitives".to_string(),
        ));
    }

    Ok(GltfImportResult { objects, materials })
}

fn get_accessor_slice<'a>(
    root: &GltfRoot,
    buffers: &'a [Vec<u8>],
    accessor_idx: usize,
) -> Result<(&'a [u8], usize, usize, u32)> {
    let accessor = root
        .accessors
        .get(accessor_idx)
        .ok_or_else(|| ForgeIoError::ParseError {
            line: 0,
            message: format!("Accessor index {accessor_idx} out of range"),
        })?;

    let bv_idx = accessor
        .buffer_view
        .ok_or_else(|| ForgeIoError::UnsupportedFormat {
            format: "glTF",
            message: "Sparse accessors without bufferView are not supported".to_string(),
        })?;

    let bv = root
        .buffer_views
        .get(bv_idx)
        .ok_or_else(|| ForgeIoError::ParseError {
            line: 0,
            message: format!("BufferView index {bv_idx} out of range"),
        })?;

    let buffer = buffers
        .get(bv.buffer)
        .ok_or_else(|| ForgeIoError::ParseError {
            line: 0,
            message: format!("Buffer index {} out of range", bv.buffer),
        })?;

    let start = bv.byte_offset.unwrap_or(0) + accessor.byte_offset.unwrap_or(0);
    if start > buffer.len() {
        return Err(ForgeIoError::TruncatedData {
            format: "glTF",
            expected: start,
            found: buffer.len(),
        });
    }

    Ok((
        &buffer[start..],
        accessor.count,
        bv.byte_stride.unwrap_or(0),
        accessor.component_type,
    ))
}

fn read_vec3_accessor(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    accessor_idx: usize,
) -> Result<Vec<[f32; 3]>> {
    let (slice, count, stride, comp_type) = get_accessor_slice(root, buffers, accessor_idx)?;
    if comp_type != COMPONENT_TYPE_FLOAT {
        return Err(ForgeIoError::UnsupportedFormat {
            format: "glTF",
            message: format!("VEC3 accessor component type {comp_type} not supported"),
        });
    }

    let elem_stride = if stride > 0 { stride } else { 12 };
    let mut out = Vec::with_capacity(count);

    for i in 0..count {
        let offset = i * elem_stride;
        if offset + 12 > slice.len() {
            break;
        }
        let x = f32::from_le_bytes(slice[offset..offset + 4].try_into().unwrap());
        let y = f32::from_le_bytes(slice[offset + 4..offset + 8].try_into().unwrap());
        let z = f32::from_le_bytes(slice[offset + 8..offset + 12].try_into().unwrap());
        out.push([x, y, z]);
    }

    Ok(out)
}

fn read_vec2_accessor(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    accessor_idx: usize,
) -> Result<Vec<[f32; 2]>> {
    let (slice, count, stride, comp_type) = get_accessor_slice(root, buffers, accessor_idx)?;
    if comp_type != COMPONENT_TYPE_FLOAT {
        return Err(ForgeIoError::UnsupportedFormat {
            format: "glTF",
            message: format!("VEC2 accessor component type {comp_type} not supported"),
        });
    }

    let elem_stride = if stride > 0 { stride } else { 8 };
    let mut out = Vec::with_capacity(count);

    for i in 0..count {
        let offset = i * elem_stride;
        if offset + 8 > slice.len() {
            break;
        }
        let u = f32::from_le_bytes(slice[offset..offset + 4].try_into().unwrap());
        let v = f32::from_le_bytes(slice[offset + 4..offset + 8].try_into().unwrap());
        out.push([u, v]);
    }

    Ok(out)
}

fn read_vec4_accessor(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    accessor_idx: usize,
) -> Result<Vec<[f32; 4]>> {
    let (slice, count, stride, comp_type) = get_accessor_slice(root, buffers, accessor_idx)?;
    let mut out = Vec::with_capacity(count);

    match comp_type {
        COMPONENT_TYPE_FLOAT => {
            let elem_stride = if stride > 0 { stride } else { 16 };
            for i in 0..count {
                let offset = i * elem_stride;
                if offset + 16 > slice.len() {
                    break;
                }
                let x = f32::from_le_bytes(slice[offset..offset + 4].try_into().unwrap());
                let y = f32::from_le_bytes(slice[offset + 4..offset + 8].try_into().unwrap());
                let z = f32::from_le_bytes(slice[offset + 8..offset + 12].try_into().unwrap());
                let w = f32::from_le_bytes(slice[offset + 12..offset + 16].try_into().unwrap());
                out.push([x, y, z, w]);
            }
        }
        COMPONENT_TYPE_UNSIGNED_BYTE => {
            let elem_stride = if stride > 0 { stride } else { 4 };
            for i in 0..count {
                let offset = i * elem_stride;
                if offset + 4 > slice.len() {
                    break;
                }
                let x = slice[offset] as f32 / 255.0;
                let y = slice[offset + 1] as f32 / 255.0;
                let z = slice[offset + 2] as f32 / 255.0;
                let w = slice[offset + 3] as f32 / 255.0;
                out.push([x, y, z, w]);
            }
        }
        _ => {}
    }

    Ok(out)
}

fn read_indices_accessor(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    accessor_idx: usize,
) -> Result<Vec<u32>> {
    let (slice, count, _, comp_type) = get_accessor_slice(root, buffers, accessor_idx)?;
    let mut out = Vec::with_capacity(count);

    match comp_type {
        COMPONENT_TYPE_UNSIGNED_INT => {
            for i in 0..count {
                let offset = i * 4;
                if offset + 4 > slice.len() {
                    break;
                }
                let idx = u32::from_le_bytes(slice[offset..offset + 4].try_into().unwrap());
                out.push(idx);
            }
        }
        COMPONENT_TYPE_UNSIGNED_SHORT => {
            for i in 0..count {
                let offset = i * 2;
                if offset + 2 > slice.len() {
                    break;
                }
                let idx = u16::from_le_bytes(slice[offset..offset + 2].try_into().unwrap()) as u32;
                out.push(idx);
            }
        }
        COMPONENT_TYPE_UNSIGNED_BYTE => {
            for i in 0..count {
                if i >= slice.len() {
                    break;
                }
                out.push(slice[i] as u32);
            }
        }
        _ => {
            return Err(ForgeIoError::UnsupportedFormat {
                format: "glTF",
                message: format!("Index component type {comp_type} not supported"),
            });
        }
    }

    Ok(out)
}

fn calculate_flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for chunk in indices.as_chunks::<3>().0 {
        let i0 = chunk[0] as usize;
        let i1 = chunk[1] as usize;
        let i2 = chunk[2] as usize;
        if i0 < positions.len() && i1 < positions.len() && i2 < positions.len() {
            let v0 = Vec3::from_array(positions[i0]);
            let v1 = Vec3::from_array(positions[i1]);
            let v2 = Vec3::from_array(positions[i2]);
            let n = (v1 - v0).cross(v2 - v0);
            normals[i0] += n;
            normals[i1] += n;
            normals[i2] += n;
        }
    }
    normals
        .into_iter()
        .map(|n| {
            let norm = n.normalize_or_zero();
            if norm.length_squared() == 0.0 {
                [0.0, 1.0, 0.0]
            } else {
                [norm.x, norm.y, norm.z]
            }
        })
        .collect()
}
