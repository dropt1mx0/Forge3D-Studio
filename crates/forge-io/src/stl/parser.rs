use std::io::Read;

use crate::error::{ForgeIoError, Result};

/// A single triangular facet in an STL file.
#[derive(Debug, Clone, PartialEq)]
pub struct StlFacet {
    /// Surface normal vector `[nx, ny, nz]`.
    pub normal: [f32; 3],
    /// Three 3D vertices `[v0, v1, v2]`.
    pub vertices: [[f32; 3]; 3],
    /// 16-bit attribute byte count (binary STL only, typically 0).
    pub attribute_byte_count: u16,
}

/// Encoding format of an STL file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StlFormat {
    Ascii,
    Binary,
}

/// In-memory representation of a parsed STL document.
#[derive(Debug, Clone, PartialEq)]
pub struct StlDocument {
    /// Solid name (from `solid <name>` or binary header).
    pub name: String,
    /// Detected format (ASCII or Binary).
    pub format: StlFormat,
    /// Triangle facets parsed from the document.
    pub facets: Vec<StlFacet>,
    /// Non-fatal diagnostic warnings.
    pub warnings: Vec<String>,
}

/// Parses an STL document from a byte buffer or stream, automatically detecting ASCII or Binary format.
pub fn parse_stl(mut reader: impl Read) -> Result<StlDocument> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    if buffer.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    if is_binary_stl(&buffer) {
        parse_binary_stl(&buffer)
    } else {
        parse_ascii_stl(&buffer)
    }
}

/// Parses an STL document from an in-memory byte slice.
pub fn parse_stl_from_bytes(bytes: &[u8]) -> Result<StlDocument> {
    if bytes.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    if is_binary_stl(bytes) {
        parse_binary_stl(bytes)
    } else {
        parse_ascii_stl(bytes)
    }
}

/// Robust heuristic to detect whether an STL byte buffer is binary or ASCII.
fn is_binary_stl(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }

    // Binary STL has an 80-byte header followed by a 4-byte little-endian triangle count.
    let count_bytes = [bytes[80], bytes[81], bytes[82], bytes[83]];
    let triangle_count = u32::from_le_bytes(count_bytes) as usize;

    let expected_binary_len = 84usize.saturating_add(triangle_count.saturating_mul(50));
    if bytes.len() == expected_binary_len {
        // Exact length match confirms binary STL with near 100% certainty,
        // even if the 80-byte header starts with "solid".
        return true;
    }

    // If file starts with ASCII "solid", check if it contains common ASCII keywords
    if bytes.starts_with(b"solid") {
        // Check for non-ASCII or null bytes in the first 512 bytes
        let check_len = bytes.len().min(512);
        if bytes[..check_len].contains(&0) {
            // Null bytes in the prefix indicate binary data
            return true;
        }

        if let Ok(text) = std::str::from_utf8(&bytes[..check_len]) {
            let lower = text.to_ascii_lowercase();
            if lower.contains("facet") || lower.contains("outer loop") || lower.contains("vertex") {
                return false;
            }
        }
    }

    // Fallback: if it contains any null bytes in the first 80 bytes, it's binary
    bytes[..80.min(bytes.len())].contains(&0)
}

/// Parses binary STL format from bytes.
fn parse_binary_stl(bytes: &[u8]) -> Result<StlDocument> {
    if bytes.len() < 84 {
        return Err(ForgeIoError::TruncatedData {
            format: "STL (Binary)",
            expected: 84,
            found: bytes.len(),
        });
    }

    // Header: 80 bytes
    let header_bytes = &bytes[0..80];
    let name = String::from_utf8_lossy(header_bytes)
        .trim_matches(char::from(0))
        .trim()
        .to_string();

    // Triangle count: 4 bytes (little endian)
    let triangle_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;

    let expected_len = 84usize.saturating_add(triangle_count.saturating_mul(50));
    if bytes.len() < expected_len {
        return Err(ForgeIoError::TruncatedData {
            format: "STL (Binary)",
            expected: expected_len,
            found: bytes.len(),
        });
    }

    let mut facets = Vec::with_capacity(triangle_count);
    let mut warnings = Vec::new();
    let mut offset = 84;

    for i in 0..triangle_count {
        if offset + 50 > bytes.len() {
            warnings.push(format!(
                "Truncated facet data at index {} of {}",
                i, triangle_count
            ));
            break;
        }

        let normal = [
            read_f32_le(&bytes[offset..offset + 4]),
            read_f32_le(&bytes[offset + 4..offset + 8]),
            read_f32_le(&bytes[offset + 8..offset + 12]),
        ];
        offset += 12;

        let v0 = [
            read_f32_le(&bytes[offset..offset + 4]),
            read_f32_le(&bytes[offset + 4..offset + 8]),
            read_f32_le(&bytes[offset + 8..offset + 12]),
        ];
        offset += 12;

        let v1 = [
            read_f32_le(&bytes[offset..offset + 4]),
            read_f32_le(&bytes[offset + 4..offset + 8]),
            read_f32_le(&bytes[offset + 8..offset + 12]),
        ];
        offset += 12;

        let v2 = [
            read_f32_le(&bytes[offset..offset + 4]),
            read_f32_le(&bytes[offset + 4..offset + 8]),
            read_f32_le(&bytes[offset + 8..offset + 12]),
        ];
        offset += 12;

        let attribute_byte_count = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;

        // Validate vertex coordinates are finite
        if !is_finite_vec3(v0) || !is_finite_vec3(v1) || !is_finite_vec3(v2) {
            warnings.push(format!(
                "Facet {} contains non-finite vertex coordinates",
                i
            ));
            continue;
        }

        facets.push(StlFacet {
            normal,
            vertices: [v0, v1, v2],
            attribute_byte_count,
        });
    }

    Ok(StlDocument {
        name: if name.is_empty() {
            "stl_binary_mesh".to_string()
        } else {
            name
        },
        format: StlFormat::Binary,
        facets,
        warnings,
    })
}

/// Parses ASCII STL format from bytes.
fn parse_ascii_stl(bytes: &[u8]) -> Result<StlDocument> {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(ForgeIoError::ParseError {
                line: 1,
                message: "File contains invalid UTF-8 sequences for ASCII STL".to_string(),
            });
        }
    };

    let mut solid_name = "stl_ascii_mesh".to_string();
    let mut facets = Vec::new();
    let mut warnings = Vec::new();

    let mut current_normal: Option<[f32; 3]> = None;
    let mut current_vertices: Vec<[f32; 3]> = Vec::with_capacity(3);

    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut tokens = trimmed.split_whitespace();
        let first_token = match tokens.next() {
            Some(t) => t.to_ascii_lowercase(),
            None => continue,
        };

        match first_token.as_str() {
            "solid" => {
                if let Some(name) = tokens.next() {
                    solid_name = name.to_string();
                }
            }
            "facet" => {
                // Expected: facet normal <nx> <ny> <nz>
                let sub = tokens.next().map(|s| s.to_ascii_lowercase());
                if sub.as_deref() == Some("normal") {
                    let nx = parse_float_token(tokens.next(), line_idx + 1)?;
                    let ny = parse_float_token(tokens.next(), line_idx + 1)?;
                    let nz = parse_float_token(tokens.next(), line_idx + 1)?;
                    current_normal = Some([nx, ny, nz]);
                } else {
                    current_normal = Some([0.0, 0.0, 0.0]);
                }
                current_vertices.clear();
            }
            "outer" => {
                // outer loop
            }
            "vertex" => {
                let x = parse_float_token(tokens.next(), line_idx + 1)?;
                let y = parse_float_token(tokens.next(), line_idx + 1)?;
                let z = parse_float_token(tokens.next(), line_idx + 1)?;
                let pos = [x, y, z];
                if is_finite_vec3(pos) {
                    current_vertices.push(pos);
                } else {
                    warnings.push(format!(
                        "Line {}: non-finite vertex coordinates ignored",
                        line_idx + 1
                    ));
                }
            }
            "endloop" => {
                // end of outer loop
            }
            "endfacet" => {
                if current_vertices.len() == 3 {
                    let normal = current_normal.unwrap_or([0.0, 0.0, 0.0]);
                    facets.push(StlFacet {
                        normal,
                        vertices: [
                            current_vertices[0],
                            current_vertices[1],
                            current_vertices[2],
                        ],
                        attribute_byte_count: 0,
                    });
                } else if !current_vertices.is_empty() {
                    warnings.push(format!(
                        "Line {}: facet ended with {} vertices instead of 3",
                        line_idx + 1,
                        current_vertices.len()
                    ));
                }
                current_normal = None;
                current_vertices.clear();
            }
            "endsolid" => {
                break;
            }
            _ => {
                // Ignore unrecognized keywords gracefully
            }
        }
    }

    Ok(StlDocument {
        name: solid_name,
        format: StlFormat::Ascii,
        facets,
        warnings,
    })
}

#[inline(always)]
fn read_f32_le(slice: &[u8]) -> f32 {
    f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]])
}

#[inline(always)]
fn is_finite_vec3(v: [f32; 3]) -> bool {
    v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
}

fn parse_float_token(token: Option<&str>, line: usize) -> Result<f32> {
    match token {
        Some(t) => t.parse::<f32>().map_err(|e| ForgeIoError::ParseError {
            line,
            message: format!("Failed to parse float token '{}': {}", t, e),
        }),
        None => Err(ForgeIoError::ParseError {
            line,
            message: "Unexpected end of tokens while parsing float".to_string(),
        }),
    }
}
