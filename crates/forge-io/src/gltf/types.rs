use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF" in little-endian
pub const GLB_VERSION_2: u32 = 2;
pub const CHUNK_TYPE_JSON: u32 = 0x4E4F_534A; // "JSON" in little-endian
pub const CHUNK_TYPE_BIN: u32 = 0x004E_4942; // "BIN\0" in little-endian

pub const COMPONENT_TYPE_BYTE: u32 = 5120;
pub const COMPONENT_TYPE_UNSIGNED_BYTE: u32 = 5121;
pub const COMPONENT_TYPE_SHORT: u32 = 5122;
pub const COMPONENT_TYPE_UNSIGNED_SHORT: u32 = 5123;
pub const COMPONENT_TYPE_UNSIGNED_INT: u32 = 5125;
pub const COMPONENT_TYPE_FLOAT: u32 = 5126;

pub const PRIMITIVE_MODE_POINTS: u32 = 0;
pub const PRIMITIVE_MODE_LINES: u32 = 1;
pub const PRIMITIVE_MODE_LINE_LOOP: u32 = 2;
pub const PRIMITIVE_MODE_LINE_STRIP: u32 = 3;
pub const PRIMITIVE_MODE_TRIANGLES: u32 = 4;
pub const PRIMITIVE_MODE_TRIANGLE_STRIP: u32 = 5;
pub const PRIMITIVE_MODE_TRIANGLE_FAN: u32 = 6;

pub const TARGET_ARRAY_BUFFER: u32 = 34962;
pub const TARGET_ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// Top-level glTF 2.0 document structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfRoot {
    pub asset: GltfAsset,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<GltfScene>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<GltfNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub meshes: Vec<GltfMesh>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<GltfMaterial>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accessors: Vec<GltfAccessor>,
    #[serde(rename = "bufferViews", default, skip_serializing_if = "Vec::is_empty")]
    pub buffer_views: Vec<GltfBufferView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buffers: Vec<GltfBuffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfAsset {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfScene {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<[f32; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfMesh {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primitives: Vec<GltfPrimitive>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfPrimitive {
    pub attributes: HashMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indices: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfMaterial {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        rename = "pbrMetallicRoughness",
        skip_serializing_if = "Option::is_none"
    )]
    pub pbr_metallic_roughness: Option<GltfPbrMetallicRoughness>,
    #[serde(rename = "emissiveFactor", skip_serializing_if = "Option::is_none")]
    pub emissive_factor: Option<[f32; 3]>,
    #[serde(rename = "doubleSided", skip_serializing_if = "Option::is_none")]
    pub double_sided: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfPbrMetallicRoughness {
    #[serde(rename = "baseColorFactor", skip_serializing_if = "Option::is_none")]
    pub base_color_factor: Option<[f32; 4]>,
    #[serde(rename = "metallicFactor", skip_serializing_if = "Option::is_none")]
    pub metallic_factor: Option<f32>,
    #[serde(rename = "roughnessFactor", skip_serializing_if = "Option::is_none")]
    pub roughness_factor: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfAccessor {
    #[serde(rename = "bufferView", skip_serializing_if = "Option::is_none")]
    pub buffer_view: Option<usize>,
    #[serde(rename = "byteOffset", skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<usize>,
    #[serde(rename = "componentType")]
    pub component_type: u32,
    pub count: usize,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfBufferView {
    pub buffer: usize,
    #[serde(rename = "byteOffset", skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<usize>,
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    #[serde(rename = "byteStride", skip_serializing_if = "Option::is_none")]
    pub byte_stride: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GltfBuffer {
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}
