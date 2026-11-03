use forge_io::{ForgeIoError, parse_obj_from_str};

#[test]
fn test_basic_triangle_parsing() {
    let obj = r#"
# Simple Triangle
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
f 1/1/1 2/2/2 3/3/3
"#;

    let doc = parse_obj_from_str(obj).expect("Failed to parse valid OBJ");
    assert_eq!(doc.positions.len(), 3);
    assert_eq!(doc.tex_coords.len(), 3);
    assert_eq!(doc.normals.len(), 3);
    assert_eq!(doc.objects.len(), 1);
    assert_eq!(doc.objects[0].faces.len(), 1);

    let face = &doc.objects[0].faces[0];
    assert_eq!(face.vertices.len(), 3);
    assert_eq!(face.vertices[0].position_index, 0);
    assert_eq!(face.vertices[0].tex_coord_index, Some(0));
    assert_eq!(face.vertices[0].normal_index, Some(0));
    assert_eq!(face.vertices[1].position_index, 1);
    assert_eq!(face.vertices[2].position_index, 2);
}

#[test]
fn test_negative_relative_indices() {
    let obj = r#"
v -1.0 0.0 0.0
v  1.0 0.0 0.0
v  0.0 1.0 0.0
f -1 -2 -3
"#;

    let doc = parse_obj_from_str(obj).expect("Failed to parse negative indices");
    let face = &doc.objects[0].faces[0];
    // -1 = most recent = vertex 2 (0.0, 1.0, 0.0)
    // -2 = vertex 1 (1.0, 0.0, 0.0)
    // -3 = vertex 0 (-1.0, 0.0, 0.0)
    assert_eq!(face.vertices[0].position_index, 2);
    assert_eq!(face.vertices[1].position_index, 1);
    assert_eq!(face.vertices[2].position_index, 0);
}

#[test]
fn test_all_face_format_variants() {
    let obj = r#"
v 0 0 0
v 1 0 0
v 0 1 0
v 1 1 0
vt 0 0
vt 1 0
vn 0 0 1
# 1. v only
f 1 2 3
# 2. v/vt
f 1/1 2/2 3/1
# 3. v//vn
f 1//1 2//1 3//1
# 4. v/vt/vn
f 1/1/1 2/2/1 4/2/1
"#;

    let doc = parse_obj_from_str(obj).expect("Failed to parse varied face formats");
    assert_eq!(doc.objects[0].faces.len(), 4);

    let f1 = &doc.objects[0].faces[0];
    assert_eq!(f1.vertices[0].tex_coord_index, None);
    assert_eq!(f1.vertices[0].normal_index, None);

    let f2 = &doc.objects[0].faces[1];
    assert_eq!(f2.vertices[0].tex_coord_index, Some(0));
    assert_eq!(f2.vertices[0].normal_index, None);

    let f3 = &doc.objects[0].faces[2];
    assert_eq!(f3.vertices[0].tex_coord_index, None);
    assert_eq!(f3.vertices[0].normal_index, Some(0));

    let f4 = &doc.objects[0].faces[3];
    assert_eq!(f4.vertices[0].tex_coord_index, Some(0));
    assert_eq!(f4.vertices[0].normal_index, Some(0));
}

#[test]
fn test_multiple_objects_and_groups() {
    let obj = r#"
o FirstObject
g GroupAlpha
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3

o SecondObject
g GroupBeta GroupGamma
v 10 0 0
v 11 0 0
v 10 1 0
f 4 5 6
"#;

    let doc = parse_obj_from_str(obj).expect("Failed to parse multi-object OBJ");
    assert_eq!(doc.objects.len(), 2);
    assert_eq!(doc.objects[0].name, "FirstObject");
    assert_eq!(doc.objects[0].groups, vec!["GroupAlpha"]);
    assert_eq!(doc.objects[0].faces.len(), 1);

    assert_eq!(doc.objects[1].name, "SecondObject");
    assert_eq!(doc.objects[1].groups, vec!["GroupBeta", "GroupGamma"]);
    assert_eq!(doc.objects[1].faces.len(), 1);
}

#[test]
fn test_smoothing_groups_and_materials() {
    let obj = r#"
mtllib materials.mtl
v 0 0 0
v 1 0 0
v 0 1 0
s 1
usemtl Chrome
f 1 2 3
s off
usemtl MattePlastic
f 1 2 3
"#;

    let doc = parse_obj_from_str(obj).expect("Failed to parse materials/smoothing");
    assert_eq!(doc.material_libraries, vec!["materials.mtl"]);

    let faces = &doc.objects[0].faces;
    assert_eq!(faces[0].smoothing_group, Some(1));
    assert_eq!(faces[0].material, Some("Chrome".to_string()));

    assert_eq!(faces[1].smoothing_group, None);
    assert_eq!(faces[1].material, Some("MattePlastic".to_string()));
}

#[test]
fn test_error_on_zero_index() {
    let obj = r#"
v 0 0 0
v 1 0 0
v 0 1 0
f 0 1 2
"#;
    let res = parse_obj_from_str(obj);
    assert!(matches!(res, Err(ForgeIoError::ZeroIndex { .. })));
}

#[test]
fn test_error_on_out_of_bounds_index() {
    let obj = r#"
v 0 0 0
v 1 0 0
f 1 2 999
"#;
    let res = parse_obj_from_str(obj);
    assert!(matches!(res, Err(ForgeIoError::InvalidIndex { .. })));
}

#[test]
fn test_error_on_empty_file() {
    let obj = "# Only comments\n\n";
    let res = parse_obj_from_str(obj);
    assert!(matches!(res, Err(ForgeIoError::EmptyFile)));
}
