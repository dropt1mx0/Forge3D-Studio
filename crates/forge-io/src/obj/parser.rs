use std::io::BufRead;

use crate::error::{ForgeIoError, Result};

/// A single face-vertex reference with resolved 0-based indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjFaceVertex {
    pub position_index: usize,
    pub tex_coord_index: Option<usize>,
    pub normal_index: Option<usize>,
}

/// A polygon face definition containing 3 or more vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjFace {
    pub vertices: Vec<ObjFaceVertex>,
    pub material: Option<String>,
    pub smoothing_group: Option<u32>,
}

/// A logical object within the OBJ document containing faces.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjObject {
    pub name: String,
    pub faces: Vec<ObjFace>,
    pub groups: Vec<String>,
}

impl ObjObject {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            faces: Vec::new(),
            groups: Vec::new(),
        }
    }
}

/// Parsed in-memory representation of a Wavefront OBJ document.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjDocument {
    pub positions: Vec<[f32; 3]>,
    pub tex_coords: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,
    pub objects: Vec<ObjObject>,
    pub material_libraries: Vec<String>,
    pub warnings: Vec<String>,
}

impl Default for ObjDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjDocument {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            tex_coords: Vec::new(),
            normals: Vec::new(),
            objects: Vec::new(),
            material_libraries: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Parses an OBJ document from any buffered reader.
pub fn parse_obj(reader: impl BufRead) -> Result<ObjDocument> {
    let mut doc = ObjDocument::new();
    let mut current_object = ObjObject::new("default");
    let mut current_material: Option<String> = None;
    let mut current_smoothing: Option<u32> = None;
    let mut current_groups: Vec<String> = Vec::new();
    let mut has_explicit_objects = false;

    for (line_idx_0, line_result) in reader.lines().enumerate() {
        let line_num = line_idx_0 + 1;
        let line = line_result?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut tokens = trimmed.split_whitespace();
        let directive = match tokens.next() {
            Some(d) => d,
            None => continue,
        };

        match directive {
            "v" => {
                let x = parse_float(tokens.next(), line_num, "v.x")?;
                let y = parse_float(tokens.next(), line_num, "v.y")?;
                let z = parse_float(tokens.next(), line_num, "v.z")?;
                let w = if let Some(w_str) = tokens.next() {
                    parse_float(Some(w_str), line_num, "v.w")?
                } else {
                    1.0
                };

                let pos = if w != 1.0 && w != 0.0 {
                    [x / w, y / w, z / w]
                } else {
                    [x, y, z]
                };
                doc.positions.push(pos);
            }

            "vt" => {
                let u = parse_float(tokens.next(), line_num, "vt.u")?;
                let v = if let Some(v_str) = tokens.next() {
                    parse_float(Some(v_str), line_num, "vt.v")?
                } else {
                    0.0
                };
                doc.tex_coords.push([u, v]);
            }

            "vn" => {
                let x = parse_float(tokens.next(), line_num, "vn.x")?;
                let y = parse_float(tokens.next(), line_num, "vn.y")?;
                let z = parse_float(tokens.next(), line_num, "vn.z")?;
                doc.normals.push([x, y, z]);
            }

            "f" => {
                let mut face_verts = Vec::new();

                for token in tokens {
                    let face_v = parse_face_vertex(token, line_num, &doc)?;
                    face_verts.push(face_v);
                }

                if face_verts.len() < 3 {
                    doc.warnings.push(format!(
                        "Line {line_num}: Face has fewer than 3 vertices (found {}), skipping",
                        face_verts.len()
                    ));
                    continue;
                }

                current_object.faces.push(ObjFace {
                    vertices: face_verts,
                    material: current_material.clone(),
                    smoothing_group: current_smoothing,
                });
            }

            "o" => {
                let name = tokens.collect::<Vec<_>>().join(" ");
                let obj_name = if name.is_empty() {
                    format!("Object_{}", doc.objects.len() + 1)
                } else {
                    name
                };

                if !current_object.faces.is_empty() {
                    doc.objects.push(current_object);
                }
                current_object = ObjObject::new(obj_name);
                current_object.groups = current_groups.clone();
                has_explicit_objects = true;
            }

            "g" => {
                let group_names: Vec<String> = tokens.map(|s| s.to_string()).collect();
                current_groups = group_names.clone();

                // If groups are used as object dividers in absence of 'o' directives
                if !has_explicit_objects && !current_object.faces.is_empty() {
                    doc.objects.push(current_object);
                    let name = if group_names.is_empty() {
                        format!("Group_{}", doc.objects.len() + 1)
                    } else {
                        group_names.join("_")
                    };
                    current_object = ObjObject::new(name);
                }
                current_object.groups = group_names;
            }

            "s" => {
                if let Some(token) = tokens.next() {
                    if token == "off" || token == "0" {
                        current_smoothing = None;
                    } else if let Ok(group_num) = token.parse::<u32>() {
                        current_smoothing = Some(group_num);
                    } else {
                        doc.warnings.push(format!(
                            "Line {line_num}: Unrecognized smoothing group '{token}'"
                        ));
                    }
                }
            }

            "mtllib" => {
                for lib in tokens {
                    doc.material_libraries.push(lib.to_string());
                }
            }

            "usemtl" => {
                if let Some(mat_name) = tokens.next() {
                    current_material = Some(mat_name.to_string());
                }
            }

            _ => {
                // Safely ignore unknown or unsupported directives (e.g. 'l', 'vp', 'cstype')
            }
        }
    }

    // Push the trailing object if it contains faces
    if !current_object.faces.is_empty() {
        doc.objects.push(current_object);
    }

    if doc.positions.is_empty() && doc.objects.is_empty() {
        return Err(ForgeIoError::EmptyFile);
    }

    Ok(doc)
}

/// Parses an OBJ document directly from a string slice.
pub fn parse_obj_from_str(obj_str: &str) -> Result<ObjDocument> {
    parse_obj(std::io::Cursor::new(obj_str))
}

fn parse_float(val: Option<&str>, line: usize, name: &'static str) -> Result<f32> {
    let s = val.ok_or_else(|| ForgeIoError::ParseError {
        line,
        message: format!("Missing float value for {name}"),
    })?;

    let parsed = s.parse::<f32>().map_err(|e| ForgeIoError::ParseError {
        line,
        message: format!("Invalid float value '{s}' for {name}: {e}"),
    })?;

    if !parsed.is_finite() {
        return Err(ForgeIoError::ParseError {
            line,
            message: format!("Float value for {name} is non-finite ({parsed})"),
        });
    }

    Ok(parsed)
}

/// Parses a face vertex element string (e.g. `1`, `1/2`, `1//3`, `1/2/3`) with 1-based or negative indexing.
fn parse_face_vertex(token: &str, line: usize, doc: &ObjDocument) -> Result<ObjFaceVertex> {
    let parts: Vec<&str> = token.split('/').collect();

    // 1. Position Index (mandatory)
    let pos_token = parts.first().copied().unwrap_or("");
    let pos_idx_raw: i64 = pos_token
        .parse::<i64>()
        .map_err(|e| ForgeIoError::ParseError {
            line,
            message: format!("Invalid vertex index '{pos_token}': {e}"),
        })?;

    let position_index = resolve_index(pos_idx_raw, doc.positions.len(), line, "vertex")?;

    // 2. Texture Coordinate Index (optional)
    let tex_coord_index = if parts.len() > 1 && !parts[1].is_empty() {
        let vt_token = parts[1];
        let vt_idx_raw: i64 = vt_token
            .parse::<i64>()
            .map_err(|e| ForgeIoError::ParseError {
                line,
                message: format!("Invalid texture index '{vt_token}': {e}"),
            })?;
        Some(resolve_index(
            vt_idx_raw,
            doc.tex_coords.len(),
            line,
            "texture coordinate",
        )?)
    } else {
        None
    };

    // 3. Normal Index (optional)
    let normal_index = if parts.len() > 2 && !parts[2].is_empty() {
        let vn_token = parts[2];
        let vn_idx_raw: i64 = vn_token
            .parse::<i64>()
            .map_err(|e| ForgeIoError::ParseError {
                line,
                message: format!("Invalid normal index '{vn_token}': {e}"),
            })?;
        Some(resolve_index(
            vn_idx_raw,
            doc.normals.len(),
            line,
            "normal",
        )?)
    } else {
        None
    };

    Ok(ObjFaceVertex {
        position_index,
        tex_coord_index,
        normal_index,
    })
}

/// Resolves a 1-based (positive) or relative (negative) OBJ index into a 0-based absolute array index.
fn resolve_index(raw: i64, count: usize, line: usize, element: &'static str) -> Result<usize> {
    if raw == 0 {
        return Err(ForgeIoError::ZeroIndex { line, element });
    }

    if raw > 0 {
        let idx = raw as usize;
        if idx > count {
            return Err(ForgeIoError::InvalidIndex {
                line,
                index: raw,
                count,
                element,
            });
        }
        Ok(idx - 1)
    } else {
        // Negative relative index: -1 means the most recently defined element
        let offset = (-raw) as usize;
        if offset > count {
            return Err(ForgeIoError::InvalidIndex {
                line,
                index: raw,
                count,
                element,
            });
        }
        Ok(count - offset)
    }
}
