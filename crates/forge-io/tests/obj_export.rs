use forge_geometry::primitives::generate_cube;
use forge_io::{ExportOptions, export_mesh_obj_to_string, export_scene_obj_to_string};
use forge_math::{Transform, Vec3};
use forge_scene::{PrimitiveType, Scene, SceneObject};

#[test]
fn test_export_single_mesh() {
    let cube_mesh = generate_cube(2.0);
    let obj_str = export_mesh_obj_to_string(&cube_mesh, "UnitCube").expect("Failed to export mesh");

    assert!(obj_str.contains("o UnitCube"));
    assert!(obj_str.contains("v "));
    assert!(obj_str.contains("vt "));
    assert!(obj_str.contains("vn "));
    assert!(obj_str.contains("f "));

    // 24 vertices, 12 triangles
    let v_count = obj_str.lines().filter(|l| l.starts_with("v ")).count();
    let f_count = obj_str.lines().filter(|l| l.starts_with("f ")).count();
    assert_eq!(v_count, 24);
    assert_eq!(f_count, 12);
}

#[test]
fn test_export_options_toggles() {
    let cube_mesh = generate_cube(1.0);
    let options = ExportOptions {
        export_uvs: false,
        export_normals: false,
        ..Default::default()
    };

    let mut scene = Scene::new("TestScene");
    let obj = SceneObject::new("MinimalCube", PrimitiveType::Cube, cube_mesh);
    scene.add_object(obj);

    let obj_str = export_scene_obj_to_string(&scene, &options).expect("Export failed");
    assert!(!obj_str.contains("vt "));
    assert!(!obj_str.contains("vn "));

    // Face lines should be "f v0 v1 v2" without slashes
    for line in obj_str.lines().filter(|l| l.starts_with("f ")) {
        assert!(!line.contains('/'));
    }
}

#[test]
fn test_export_scene_world_transforms() {
    let mut scene = Scene::new("WorldScene");
    let cube = generate_cube(1.0);

    let mut obj1 = SceneObject::new("CubeAtOrigin", PrimitiveType::Cube, cube.clone());
    obj1.local_transform = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
    scene.add_object(obj1);

    let options = ExportOptions {
        world_space: true,
        ..Default::default()
    };

    let obj_str = export_scene_obj_to_string(&scene, &options).expect("Export failed");
    assert!(obj_str.contains("o CubeAtOrigin"));

    // Check that vertices reflect translation around X = 10.0 (from 9.5 to 10.5)
    let mut has_translated_x = false;
    for line in obj_str.lines().filter(|l| l.starts_with("v ")) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let x: f32 = parts[1].parse().unwrap();
        if x > 9.0 && x < 11.0 {
            has_translated_x = true;
        }
    }
    assert!(has_translated_x);
}
