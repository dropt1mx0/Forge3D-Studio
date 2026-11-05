use std::io::Read;

use crate::error::{ForgeIoError, Result};

/// Encoding format of a PLY file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

/// Primitive data types supported by the PLY specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlyDataType {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

impl PlyDataType {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            PlyDataType::Int8 | PlyDataType::Uint8 => 1,
            PlyDataType::Int16 | PlyDataType::Uint16 => 2,
            PlyDataType::Int32 | PlyDataType::Uint32 | PlyDataType::Float32 => 4,
            PlyDataType::Float64 => 8,
        }
    }

    pub fn from_type_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "char" | "int8" => Some(PlyDataType::Int8),
            "uchar" | "uint8" => Some(PlyDataType::Uint8),
            "short" | "int16" => Some(PlyDataType::Int16),
            "ushort" | "uint16" => Some(PlyDataType::Uint16),
            "int" | "int32" => Some(PlyDataType::Int32),
            "uint" | "uint32" => Some(PlyDataType::Uint32),
            "float" | "float32" => Some(PlyDataType::Float32),
            "double" | "float64" => Some(PlyDataType::Float64),
            _ => None,
        }
    }
}

impl std::str::FromStr for PlyDataType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_type_name(s).ok_or(())
    }
}

/// A property declaration inside an element.
#[derive(Debug, Clone, PartialEq)]
pub enum PlyProperty {
    Scalar {
        name: String,
        data_type: PlyDataType,
    },
    List {
        name: String,
        count_type: PlyDataType,
        item_type: PlyDataType,
    },
}

impl PlyProperty {
    pub fn name(&self) -> &str {
        match self {
            PlyProperty::Scalar { name, .. } => name,
            PlyProperty::List { name, .. } => name,
        }
    }
}

/// An element definition in the PLY header (e.g. `element vertex 8`, `element face 6`).
#[derive(Debug, Clone, PartialEq)]
pub struct PlyElementDef {
    pub name: String,
    pub count: usize,
    pub properties: Vec<PlyProperty>,
}

/// A parsed PLY vertex record.
#[derive(Debug, Clone, PartialEq)]
pub struct PlyVertex {
    pub position: [f32; 3],
    pub normal: Option<[f32; 3]>,
    pub uv: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
}

/// A parsed PLY face record containing polygon vertex indices.
#[derive(Debug, Clone, PartialEq)]
pub struct PlyFace {
    pub indices: Vec<u32>,
}

/// A complete parsed PLY document.
#[derive(Debug, Clone, PartialEq)]
pub struct PlyDocument {
    pub format: PlyFormat,
    pub comments: Vec<String>,
    pub vertices: Vec<PlyVertex>,
    pub faces: Vec<PlyFace>,
    pub warnings: Vec<String>,
}

/// Parses a PLY document from a byte buffer or stream.
pub fn parse_ply(mut reader: impl Read) -> Result<PlyDocument> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    if buffer.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    parse_ply_from_bytes(&buffer)
}

/// Parses a PLY document from an in-memory byte slice.
pub fn parse_ply_from_bytes(bytes: &[u8]) -> Result<PlyDocument> {
    if bytes.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    // Locate "end_header\n" or "end_header\r\n"
    let header_end_marker = b"end_header";
    let header_end_pos = match find_subsequence(bytes, header_end_marker) {
        Some(pos) => pos,
        None => {
            return Err(ForgeIoError::InvalidHeader {
                format: "PLY",
                message: "Missing 'end_header' declaration".to_string(),
            });
        }
    };

    // Find the newline immediately following end_header
    let mut data_start = header_end_pos + header_end_marker.len();
    while data_start < bytes.len() && (bytes[data_start] == b'\r' || bytes[data_start] == b'\n') {
        data_start += 1;
    }

    let header_str =
        std::str::from_utf8(&bytes[..header_end_pos]).map_err(|_| ForgeIoError::InvalidHeader {
            format: "PLY",
            message: "Header is not valid UTF-8/ASCII".to_string(),
        })?;

    let (format, elements, comments) = parse_header(header_str)?;

    match format {
        PlyFormat::Ascii => parse_ascii_data(&bytes[data_start..], format, &elements, comments),
        PlyFormat::BinaryLittleEndian => {
            parse_binary_data(&bytes[data_start..], format, &elements, comments, false)
        }
        PlyFormat::BinaryBigEndian => {
            parse_binary_data(&bytes[data_start..], format, &elements, comments, true)
        }
    }
}

fn parse_header(header_str: &str) -> Result<(PlyFormat, Vec<PlyElementDef>, Vec<String>)> {
    let mut lines = header_str.lines();

    // Line 1 must be "ply"
    let first = lines.next().map(|s| s.trim()).unwrap_or("");
    if first != "ply" {
        return Err(ForgeIoError::InvalidHeader {
            format: "PLY",
            message: format!("Expected 'ply' signature on first line, found '{}'", first),
        });
    }

    let mut format = None;
    let mut elements: Vec<PlyElementDef> = Vec::new();
    let mut comments = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "format" => {
                if tokens.len() < 2 {
                    return Err(ForgeIoError::InvalidHeader {
                        format: "PLY",
                        message: "Malformed format line in header".to_string(),
                    });
                }
                format = match tokens[1] {
                    "ascii" => Some(PlyFormat::Ascii),
                    "binary_little_endian" => Some(PlyFormat::BinaryLittleEndian),
                    "binary_big_endian" => Some(PlyFormat::BinaryBigEndian),
                    other => {
                        return Err(ForgeIoError::UnsupportedFormat {
                            format: "PLY",
                            message: format!("Unsupported PLY format '{}'", other),
                        });
                    }
                };
            }
            "comment" => {
                comments.push(trimmed[7..].trim().to_string());
            }
            "element" => {
                if tokens.len() < 3 {
                    return Err(ForgeIoError::InvalidHeader {
                        format: "PLY",
                        message: "Malformed element declaration in header".to_string(),
                    });
                }
                let name = tokens[1].to_string();
                let count =
                    tokens[2]
                        .parse::<usize>()
                        .map_err(|e| ForgeIoError::InvalidHeader {
                            format: "PLY",
                            message: format!("Invalid element count for '{}': {}", name, e),
                        })?;
                elements.push(PlyElementDef {
                    name,
                    count,
                    properties: Vec::new(),
                });
            }
            "property" => {
                let current_elem =
                    elements
                        .last_mut()
                        .ok_or_else(|| ForgeIoError::InvalidHeader {
                            format: "PLY",
                            message: "Property declared before any element in header".to_string(),
                        })?;

                if tokens.len() >= 3 && tokens[1] == "list" {
                    if tokens.len() < 5 {
                        return Err(ForgeIoError::InvalidHeader {
                            format: "PLY",
                            message: "Malformed property list declaration".to_string(),
                        });
                    }
                    let count_type = PlyDataType::from_type_name(tokens[2]).ok_or_else(|| {
                        ForgeIoError::InvalidHeader {
                            format: "PLY",
                            message: format!("Unknown list count type '{}'", tokens[2]),
                        }
                    })?;
                    let item_type = PlyDataType::from_type_name(tokens[3]).ok_or_else(|| {
                        ForgeIoError::InvalidHeader {
                            format: "PLY",
                            message: format!("Unknown list item type '{}'", tokens[3]),
                        }
                    })?;
                    let name = tokens[4].to_string();
                    current_elem.properties.push(PlyProperty::List {
                        name,
                        count_type,
                        item_type,
                    });
                } else if tokens.len() >= 3 {
                    let data_type = PlyDataType::from_type_name(tokens[1]).ok_or_else(|| {
                        ForgeIoError::InvalidHeader {
                            format: "PLY",
                            message: format!("Unknown property type '{}'", tokens[1]),
                        }
                    })?;
                    let name = tokens[2].to_string();
                    current_elem
                        .properties
                        .push(PlyProperty::Scalar { name, data_type });
                }
            }
            _ => {
                // Ignore other lines like obj_info
            }
        }
    }

    let format = format.ok_or_else(|| ForgeIoError::InvalidHeader {
        format: "PLY",
        message: "Missing 'format' specification in header".to_string(),
    })?;

    Ok((format, elements, comments))
}

fn parse_ascii_data(
    data: &[u8],
    format: PlyFormat,
    elements: &[PlyElementDef],
    comments: Vec<String>,
) -> Result<PlyDocument> {
    let text = std::str::from_utf8(data).map_err(|_| ForgeIoError::ParseError {
        line: 1,
        message: "PLY ASCII data contains invalid UTF-8".to_string(),
    })?;

    let mut lines = text.lines();
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let warnings = Vec::new();

    for elem in elements {
        match elem.name.as_str() {
            "vertex" => {
                vertices.reserve(elem.count);
                for line_idx in 0..elem.count {
                    let line = lines.next().ok_or(ForgeIoError::TruncatedData {
                        format: "PLY (ASCII)",
                        expected: elem.count,
                        found: line_idx,
                    })?;
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    let v = parse_ascii_vertex_row(&tokens, &elem.properties)?;
                    vertices.push(v);
                }
            }
            "face" => {
                faces.reserve(elem.count);
                for line_idx in 0..elem.count {
                    let line = lines.next().ok_or(ForgeIoError::TruncatedData {
                        format: "PLY (ASCII)",
                        expected: elem.count,
                        found: line_idx,
                    })?;
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    let f = parse_ascii_face_row(&tokens, &elem.properties)?;
                    faces.push(f);
                }
            }
            _ => {
                // Skip unknown elements line by line
                for _ in 0..elem.count {
                    let _ = lines.next();
                }
            }
        }
    }

    Ok(PlyDocument {
        format,
        comments,
        vertices,
        faces,
        warnings,
    })
}

fn parse_ascii_vertex_row(tokens: &[&str], properties: &[PlyProperty]) -> Result<PlyVertex> {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut z = 0.0f32;
    let mut nx = None;
    let mut ny = None;
    let mut nz = None;
    let mut u = None;
    let mut v = None;
    let mut r = None;
    let mut g = None;
    let mut b = None;
    let mut a = None;

    let mut token_idx = 0;
    for prop in properties {
        match prop {
            PlyProperty::Scalar { name, .. } => {
                if token_idx >= tokens.len() {
                    break;
                }
                let val_str = tokens[token_idx];
                token_idx += 1;

                match name.to_ascii_lowercase().as_str() {
                    "x" => x = val_str.parse::<f32>().unwrap_or(0.0),
                    "y" => y = val_str.parse::<f32>().unwrap_or(0.0),
                    "z" => z = val_str.parse::<f32>().unwrap_or(0.0),
                    "nx" => nx = val_str.parse::<f32>().ok(),
                    "ny" => ny = val_str.parse::<f32>().ok(),
                    "nz" => nz = val_str.parse::<f32>().ok(),
                    "s" | "u" | "texture_u" => u = val_str.parse::<f32>().ok(),
                    "t" | "v" | "texture_v" => v = val_str.parse::<f32>().ok(),
                    "red" | "r" | "diffuse_red" => r = parse_color_channel(val_str),
                    "green" | "g" | "diffuse_green" => g = parse_color_channel(val_str),
                    "blue" | "b" | "diffuse_blue" => b = parse_color_channel(val_str),
                    "alpha" | "a" => a = parse_color_channel(val_str),
                    _ => {}
                }
            }
            PlyProperty::List { .. } => {
                // Vertex property lists (rare, advance according to count)
                if token_idx < tokens.len() {
                    let count = tokens[token_idx].parse::<usize>().unwrap_or(0);
                    token_idx += 1 + count;
                }
            }
        }
    }

    let normal = match (nx, ny, nz) {
        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
        _ => None,
    };

    let uv = match (u, v) {
        (Some(u_val), Some(v_val)) => Some([u_val, v_val]),
        _ => None,
    };

    let color = match (r, g, b) {
        (Some(red), Some(green), Some(blue)) => Some([red, green, blue, a.unwrap_or(1.0)]),
        _ => None,
    };

    Ok(PlyVertex {
        position: [x, y, z],
        normal,
        uv,
        color,
    })
}

fn parse_color_channel(s: &str) -> Option<f32> {
    if let Ok(val) = s.parse::<u8>() {
        Some(val as f32 / 255.0)
    } else if let Ok(val) = s.parse::<f32>() {
        Some(if val > 1.0 { val / 255.0 } else { val })
    } else {
        None
    }
}

fn parse_ascii_face_row(tokens: &[&str], properties: &[PlyProperty]) -> Result<PlyFace> {
    let mut token_idx = 0;
    for prop in properties {
        match prop {
            PlyProperty::List { name, .. } => {
                if token_idx >= tokens.len() {
                    break;
                }
                let count =
                    tokens[token_idx]
                        .parse::<usize>()
                        .map_err(|e| ForgeIoError::ParseError {
                            line: 0,
                            message: format!("Failed to parse face vertex count: {}", e),
                        })?;
                token_idx += 1;

                if name == "vertex_indices" || name == "vertex_index" {
                    let mut indices = Vec::with_capacity(count);
                    for _ in 0..count {
                        if token_idx >= tokens.len() {
                            break;
                        }
                        let idx = tokens[token_idx].parse::<u32>().map_err(|e| {
                            ForgeIoError::ParseError {
                                line: 0,
                                message: format!("Failed to parse vertex index: {}", e),
                            }
                        })?;
                        indices.push(idx);
                        token_idx += 1;
                    }
                    return Ok(PlyFace { indices });
                } else {
                    token_idx += count;
                }
            }
            PlyProperty::Scalar { .. } => {
                token_idx += 1;
            }
        }
    }

    Ok(PlyFace {
        indices: Vec::new(),
    })
}

fn parse_binary_data(
    data: &[u8],
    format: PlyFormat,
    elements: &[PlyElementDef],
    comments: Vec<String>,
    is_big_endian: bool,
) -> Result<PlyDocument> {
    let mut offset = 0;
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let warnings = Vec::new();

    for elem in elements {
        match elem.name.as_str() {
            "vertex" => {
                vertices.reserve(elem.count);
                for _ in 0..elem.count {
                    let v = parse_binary_vertex_row(
                        data,
                        &mut offset,
                        &elem.properties,
                        is_big_endian,
                    )?;
                    vertices.push(v);
                }
            }
            "face" => {
                faces.reserve(elem.count);
                for _ in 0..elem.count {
                    let f =
                        parse_binary_face_row(data, &mut offset, &elem.properties, is_big_endian)?;
                    faces.push(f);
                }
            }
            _ => {
                // Skip unknown binary elements
                for _ in 0..elem.count {
                    skip_binary_element_row(data, &mut offset, &elem.properties, is_big_endian)?;
                }
            }
        }
    }

    Ok(PlyDocument {
        format,
        comments,
        vertices,
        faces,
        warnings,
    })
}

fn parse_binary_vertex_row(
    data: &[u8],
    offset: &mut usize,
    properties: &[PlyProperty],
    is_big_endian: bool,
) -> Result<PlyVertex> {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut z = 0.0f32;
    let mut nx = None;
    let mut ny = None;
    let mut nz = None;
    let mut u = None;
    let mut v = None;
    let mut r = None;
    let mut g = None;
    let mut b = None;
    let mut a = None;

    for prop in properties {
        match prop {
            PlyProperty::Scalar { name, data_type } => {
                let val_f64 = read_binary_scalar(data, offset, *data_type, is_big_endian)?;
                let val_f32 = val_f64 as f32;

                match name.to_ascii_lowercase().as_str() {
                    "x" => x = val_f32,
                    "y" => y = val_f32,
                    "z" => z = val_f32,
                    "nx" => nx = Some(val_f32),
                    "ny" => ny = Some(val_f32),
                    "nz" => nz = Some(val_f32),
                    "s" | "u" | "texture_u" => u = Some(val_f32),
                    "t" | "v" | "texture_v" => v = Some(val_f32),
                    "red" | "r" | "diffuse_red" => {
                        r = Some(normalize_color_scalar(val_f64, *data_type))
                    }
                    "green" | "g" | "diffuse_green" => {
                        g = Some(normalize_color_scalar(val_f64, *data_type))
                    }
                    "blue" | "b" | "diffuse_blue" => {
                        b = Some(normalize_color_scalar(val_f64, *data_type))
                    }
                    "alpha" | "a" => a = Some(normalize_color_scalar(val_f64, *data_type)),
                    _ => {}
                }
            }
            PlyProperty::List {
                count_type,
                item_type,
                ..
            } => {
                let count = read_binary_scalar(data, offset, *count_type, is_big_endian)? as usize;
                let item_size = item_type.size_in_bytes();
                *offset += count * item_size;
            }
        }
    }

    let normal = match (nx, ny, nz) {
        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
        _ => None,
    };

    let uv = match (u, v) {
        (Some(u_val), Some(v_val)) => Some([u_val, v_val]),
        _ => None,
    };

    let color = match (r, g, b) {
        (Some(red), Some(green), Some(blue)) => Some([red, green, blue, a.unwrap_or(1.0)]),
        _ => None,
    };

    Ok(PlyVertex {
        position: [x, y, z],
        normal,
        uv,
        color,
    })
}

fn normalize_color_scalar(val: f64, data_type: PlyDataType) -> f32 {
    match data_type {
        PlyDataType::Uint8 | PlyDataType::Int8 => (val as f32 / 255.0).clamp(0.0, 1.0),
        PlyDataType::Float32 | PlyDataType::Float64 => {
            if val > 1.0 {
                (val as f32 / 255.0).clamp(0.0, 1.0)
            } else {
                (val as f32).clamp(0.0, 1.0)
            }
        }
        _ => (val as f32 / 255.0).clamp(0.0, 1.0),
    }
}

fn parse_binary_face_row(
    data: &[u8],
    offset: &mut usize,
    properties: &[PlyProperty],
    is_big_endian: bool,
) -> Result<PlyFace> {
    let mut face_indices = Vec::new();

    for prop in properties {
        match prop {
            PlyProperty::List {
                name,
                count_type,
                item_type,
            } => {
                let count = read_binary_scalar(data, offset, *count_type, is_big_endian)? as usize;
                let mut list = Vec::with_capacity(count);
                for _ in 0..count {
                    let idx = read_binary_scalar(data, offset, *item_type, is_big_endian)? as u32;
                    list.push(idx);
                }
                if name == "vertex_indices" || name == "vertex_index" {
                    face_indices = list;
                }
            }
            PlyProperty::Scalar { data_type, .. } => {
                let _ = read_binary_scalar(data, offset, *data_type, is_big_endian)?;
            }
        }
    }

    Ok(PlyFace {
        indices: face_indices,
    })
}

fn skip_binary_element_row(
    data: &[u8],
    offset: &mut usize,
    properties: &[PlyProperty],
    is_big_endian: bool,
) -> Result<()> {
    for prop in properties {
        match prop {
            PlyProperty::Scalar { data_type, .. } => {
                *offset += data_type.size_in_bytes();
            }
            PlyProperty::List {
                count_type,
                item_type,
                ..
            } => {
                let count = read_binary_scalar(data, offset, *count_type, is_big_endian)? as usize;
                *offset += count * item_type.size_in_bytes();
            }
        }
    }
    Ok(())
}

fn read_binary_scalar(
    data: &[u8],
    offset: &mut usize,
    data_type: PlyDataType,
    is_big_endian: bool,
) -> Result<f64> {
    let size = data_type.size_in_bytes();
    if *offset + size > data.len() {
        return Err(ForgeIoError::TruncatedData {
            format: "PLY (Binary)",
            expected: *offset + size,
            found: data.len(),
        });
    }

    let slice = &data[*offset..*offset + size];
    *offset += size;

    let val = match (data_type, is_big_endian) {
        (PlyDataType::Int8, _) => slice[0] as i8 as f64,
        (PlyDataType::Uint8, _) => slice[0] as f64,
        (PlyDataType::Int16, false) => i16::from_le_bytes([slice[0], slice[1]]) as f64,
        (PlyDataType::Int16, true) => i16::from_be_bytes([slice[0], slice[1]]) as f64,
        (PlyDataType::Uint16, false) => u16::from_le_bytes([slice[0], slice[1]]) as f64,
        (PlyDataType::Uint16, true) => u16::from_be_bytes([slice[0], slice[1]]) as f64,
        (PlyDataType::Int32, false) => {
            i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyDataType::Int32, true) => {
            i32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyDataType::Uint32, false) => {
            u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyDataType::Uint32, true) => {
            u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyDataType::Float32, false) => {
            f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyDataType::Float32, true) => {
            f32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        (PlyDataType::Float64, false) => f64::from_le_bytes([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ]),
        (PlyDataType::Float64, true) => f64::from_be_bytes([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ]),
    };

    Ok(val)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
