use forge_io::{ImportOptions, import_obj_from_str};
use forge_math::Vec3;

const OBJ_CUBE: &str = r#"
# 2x2x2 Cube centered at origin
o Box
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
# Front
f 5 6 7 8
# Back
f 2 1 4 3
# Top
f 4 8 7 3
# Bottom
f 1 2 6 5
# Right
f 6 2 3 7
# Left
f 1 5 8 4
"#;

#[test]
fn test_import_cube() {
    let options = ImportOptions::default();
    let asset = import_obj_from_str(OBJ_CUBE, &options).expect("Failed to import cube");

    assert_eq!(asset.objects.len(), 1);
    let obj = &asset.objects[0];
    assert_eq!(obj.name, "Box");

    // 6 quad faces triangulated = 12 triangles = 36 indices
    assert_eq!(obj.mesh.indices.len(), 36);
    assert_eq!(asset.total_triangles(), 12);

    // Verify normals were calculated since OBJ had none
    for v in &obj.mesh.vertices {
        let n = Vec3::from_array(v.normal);
        assert!(n.length() > 0.99 && n.length() < 1.01);
        assert!(!n.x.is_nan() && !n.y.is_nan() && !n.z.is_nan());
    }

    // Verify AABB
    let aabb = &obj.mesh.aabb;
    assert!((aabb.min.x - (-1.0)).abs() < 1e-4);
    assert!((aabb.max.x - 1.0).abs() < 1e-4);
    assert!((aabb.min.y - (-1.0)).abs() < 1e-4);
    assert!((aabb.max.y - 1.0).abs() < 1e-4);
    assert!((aabb.min.z - (-1.0)).abs() < 1e-4);
    assert!((aabb.max.z - 1.0).abs() < 1e-4);
}

#[test]
fn test_import_multi_object() {
    let obj = r#"
o Pyramid
v 0 1 0
v -1 0 -1
v 1 0 -1
v 1 0 1
v -1 0 1
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 2
f 5 4 3 2

o Wedge
v 10 0 0
v 12 0 0
v 10 2 0
f 6 7 8
"#;

    let options = ImportOptions::default();
    let asset = import_obj_from_str(obj, &options).expect("Failed to import multi-object");
    assert_eq!(asset.objects.len(), 2);
    assert_eq!(asset.objects[0].name, "Pyramid");
    assert_eq!(asset.objects[1].name, "Wedge");

    assert!(!asset.objects[0].mesh.indices.is_empty());
    assert!(!asset.objects[1].mesh.indices.is_empty());
}

#[test]
fn test_import_merged_objects() {
    let obj = r#"
o ObjA
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3

o ObjB
v 2 0 0
v 3 0 0
v 2 1 0
f 4 5 6
"#;

    let options = ImportOptions {
        merge_objects: true,
        ..Default::default()
    };
    let asset = import_obj_from_str(obj, &options).expect("Failed to import merged");
    assert_eq!(asset.objects.len(), 1);
    assert_eq!(asset.objects[0].name, "Combined_Mesh");
    assert_eq!(asset.total_triangles(), 2);
}

#[test]
fn test_import_preserves_uv_seams() {
    // Two triangles sharing position v 1, but with different UVs
    let obj = r#"
v 0 0 0
v 1 0 0
v 0 1 0
v -1 0 0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
vt 0.5 0.5
# Face 1 uses UV 1
f 1/1 2/2 3/3
# Face 2 uses UV 4 for vertex 1
f 1/4 3/3 4/2
"#;

    let options = ImportOptions::default();
    let asset = import_obj_from_str(obj, &options).expect("Failed to import with UV seams");
    let mesh = &asset.objects[0].mesh;

    // Vertex 1 has 2 distinct UVs, so it must produce 2 distinct vertices
    let v1_instances: Vec<_> = mesh
        .vertices
        .iter()
        .filter(|v| v.position == [0.0, 0.0, 0.0])
        .collect();
    assert_eq!(v1_instances.len(), 2);
    assert_ne!(v1_instances[0].uv, v1_instances[1].uv);
}

#[test]
fn test_import_into_scene_objects() {
    let options = ImportOptions::default();
    let asset = import_obj_from_str(OBJ_CUBE, &options).expect("Failed to import");
    let scene_objects = asset.into_scene_objects();

    assert_eq!(scene_objects.len(), 1);
    assert_eq!(scene_objects[0].name, "Box");
    assert_eq!(
        scene_objects[0].primitive_type,
        forge_scene::PrimitiveType::Custom
    );
    assert_eq!(scene_objects[0].mesh.indices.len(), 36);
}
