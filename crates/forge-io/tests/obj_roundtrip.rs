use forge_geometry::primitives::{generate_cube, generate_monkey, generate_uv_sphere};
use forge_io::{
    ExportOptions, ImportOptions, export_mesh_obj_to_string, export_scene_obj_to_string,
    import_obj_from_str,
};
use forge_math::{Transform, Vec3};
use forge_scene::{PrimitiveType, Scene, SceneObject};

#[test]
fn test_cube_roundtrip() {
    let original = generate_cube(2.0);
    let obj_str = export_mesh_obj_to_string(&original, "Cube").expect("Export failed");

    let import_opts = ImportOptions::default();
    let imported = import_obj_from_str(&obj_str, &import_opts).expect("Import failed");

    assert_eq!(imported.objects.len(), 1);
    let imported_mesh = &imported.objects[0].mesh;

    // Both should have 12 triangles
    assert_eq!(imported_mesh.indices.len(), original.indices.len());

    // Both should have matching AABB
    let orig_aabb = original.aabb;
    let imp_aabb = imported_mesh.aabb;

    assert!((orig_aabb.min.x - imp_aabb.min.x).abs() < 1e-4);
    assert!((orig_aabb.max.x - imp_aabb.max.x).abs() < 1e-4);
    assert!((orig_aabb.min.y - imp_aabb.min.y).abs() < 1e-4);
    assert!((orig_aabb.max.y - imp_aabb.max.y).abs() < 1e-4);
    assert!((orig_aabb.min.z - imp_aabb.min.z).abs() < 1e-4);
    assert!((orig_aabb.max.z - imp_aabb.max.z).abs() < 1e-4);
}

#[test]
fn test_monkey_roundtrip() {
    let original = generate_monkey(1.0);
    let obj_str = export_mesh_obj_to_string(&original, "Suzanne").expect("Export failed");

    let import_opts = ImportOptions::default();
    let imported = import_obj_from_str(&obj_str, &import_opts).expect("Import failed");

    assert_eq!(imported.objects.len(), 1);
    let imported_mesh = &imported.objects[0].mesh;

    assert_eq!(imported_mesh.indices.len(), original.indices.len());
    assert!(!imported_mesh.aabb.is_empty());
    assert!(imported_mesh.validate().is_ok());
}

#[test]
fn test_uv_sphere_roundtrip() {
    let original = generate_uv_sphere(1.0, 16, 8);
    let obj_str = export_mesh_obj_to_string(&original, "Sphere").expect("Export failed");

    let import_opts = ImportOptions::default();
    let imported = import_obj_from_str(&obj_str, &import_opts).expect("Import failed");

    assert_eq!(imported.objects.len(), 1);
    let imported_mesh = &imported.objects[0].mesh;

    assert_eq!(imported_mesh.indices.len(), original.indices.len());
    assert!(imported_mesh.validate().is_ok());
}

#[test]
fn test_multi_object_scene_roundtrip() {
    let mut scene = Scene::new("TestScene");

    let cube = generate_cube(1.0);
    let mut obj1 = SceneObject::new("AlphaCube", PrimitiveType::Cube, cube);
    obj1.local_transform = Transform::from_translation(Vec3::new(-5.0, 0.0, 0.0));
    scene.add_object(obj1);

    let monkey = generate_monkey(1.0);
    let mut obj2 = SceneObject::new("BetaMonkey", PrimitiveType::Monkey, monkey);
    obj2.local_transform = Transform::from_translation(Vec3::new(5.0, 0.0, 0.0));
    scene.add_object(obj2);

    let export_opts = ExportOptions {
        world_space: true,
        ..Default::default()
    };
    let obj_str = export_scene_obj_to_string(&scene, &export_opts).expect("Scene export failed");

    let import_opts = ImportOptions::default();
    let imported = import_obj_from_str(&obj_str, &import_opts).expect("Scene import failed");

    assert_eq!(imported.objects.len(), 2);
    let imported_names: Vec<&str> = imported.objects.iter().map(|o| o.name.as_str()).collect();
    assert!(imported_names.contains(&"AlphaCube"));
    assert!(imported_names.contains(&"BetaMonkey"));

    // Verify world space separation: AlphaCube should have negative X, BetaMonkey positive X
    let alpha = imported
        .objects
        .iter()
        .find(|o| o.name == "AlphaCube")
        .unwrap();
    let beta = imported
        .objects
        .iter()
        .find(|o| o.name == "BetaMonkey")
        .unwrap();

    assert!(alpha.mesh.aabb.center().x < -4.0);
    assert!(beta.mesh.aabb.center().x > 4.0);
}
