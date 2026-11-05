use forge_geometry::{generate_cube, generate_uv_sphere};
use forge_io::gltf::{
    GltfExportOptions, GltfImportOptions, export_mesh_to_glb, export_scene_to_glb,
    export_scene_to_gltf_embedded, import_gltf_from_bytes,
};
use forge_math::{Quat, Vec3};
use forge_scene::{PbrMaterial, PrimitiveType, Scene, SceneObject};

#[test]
fn test_glb_single_cube_roundtrip() {
    let cube_mesh = generate_cube(2.0);
    let original_vert_count = cube_mesh.vertices.len();
    let original_idx_count = cube_mesh.indices.len();

    let glb_bytes =
        export_mesh_to_glb(&cube_mesh, "TestCube").expect("GLB export of cube should succeed");
    assert!(glb_bytes.len() > 12, "GLB must have valid length");

    let result = import_gltf_from_bytes(&glb_bytes, &GltfImportOptions::default(), None)
        .expect("GLB import should succeed");
    assert_eq!(result.objects.len(), 1, "Should have 1 imported object");

    let imported_obj = &result.objects[0];
    assert_eq!(imported_obj.name, "TestCube");
    assert_eq!(imported_obj.mesh.vertices.len(), original_vert_count);
    assert_eq!(imported_obj.mesh.indices.len(), original_idx_count);

    // Verify coordinates match within tolerance
    for (orig, imp) in cube_mesh
        .vertices
        .iter()
        .zip(imported_obj.mesh.vertices.iter())
    {
        assert!((orig.position[0] - imp.position[0]).abs() < 1e-5);
        assert!((orig.position[1] - imp.position[1]).abs() < 1e-5);
        assert!((orig.position[2] - imp.position[2]).abs() < 1e-5);
    }
}

#[test]
fn test_gltf_embedded_sphere_roundtrip() {
    let sphere_mesh = generate_uv_sphere(1.5, 16, 16);
    let mut scene = Scene::new("TestScene");
    let mut obj = SceneObject::new(
        "EmbeddedSphere".to_string(),
        PrimitiveType::Custom,
        sphere_mesh.clone(),
    );
    obj.local_transform.translation = Vec3::new(10.0, -5.0, 3.0);
    obj.local_transform.scale = Vec3::new(2.0, 2.0, 2.0);
    scene.add_object(obj);

    let json_str = export_scene_to_gltf_embedded(&scene, &GltfExportOptions::default())
        .expect("Embedded glTF export should succeed");
    assert!(
        json_str.contains("data:application/octet-stream;base64,"),
        "Must contain data URI"
    );

    let result = import_gltf_from_bytes(json_str.as_bytes(), &GltfImportOptions::default(), None)
        .expect("Embedded glTF import should succeed");
    assert_eq!(result.objects.len(), 1);

    let imported_obj = &result.objects[0];
    assert_eq!(imported_obj.name, "EmbeddedSphere");
    assert_eq!(imported_obj.mesh.vertices.len(), sphere_mesh.vertices.len());
    assert_eq!(imported_obj.mesh.indices.len(), sphere_mesh.indices.len());

    // Verify transform preservation
    assert!((imported_obj.local_transform.translation.x - 10.0).abs() < 1e-4);
    assert!((imported_obj.local_transform.translation.y - (-5.0)).abs() < 1e-4);
    assert!((imported_obj.local_transform.translation.z - 3.0).abs() < 1e-4);
    assert!((imported_obj.local_transform.scale.x - 2.0).abs() < 1e-4);
}

#[test]
fn test_glb_multi_object_and_pbr_material_preservation() {
    let mut scene = Scene::new("MaterialScene");

    // Create a custom material
    let gold_mat = PbrMaterial {
        name: "Gold".to_string(),
        base_color_factor: [1.0, 0.85, 0.57, 1.0],
        metallic_factor: 0.95,
        roughness_factor: 0.15,
        emissive_factor: [0.1, 0.05, 0.0],
        ..Default::default()
    };
    scene.materials.insert("Gold".to_string(), gold_mat);

    let mut obj1 = SceneObject::new("CubeA".to_string(), PrimitiveType::Cube, generate_cube(1.0));
    obj1.material_id = Some("Gold".to_string());
    obj1.local_transform.translation = Vec3::new(-2.0, 0.0, 0.0);
    scene.add_object(obj1);

    let mut obj2 = SceneObject::new(
        "SphereB".to_string(),
        PrimitiveType::Sphere,
        generate_uv_sphere(0.8, 8, 8),
    );
    obj2.local_transform.translation = Vec3::new(2.0, 0.0, 0.0);
    obj2.local_transform.rotation = Quat::from_rotation_y(1.57);
    scene.add_object(obj2);

    let glb_bytes = export_scene_to_glb(&scene, &GltfExportOptions::default())
        .expect("Multi-object GLB export should succeed");

    let result = import_gltf_from_bytes(&glb_bytes, &GltfImportOptions::default(), None)
        .expect("Multi-object GLB import should succeed");

    assert_eq!(result.objects.len(), 2, "Should import both objects");
    assert_eq!(
        result.materials.len(),
        1,
        "Should preserve the Gold material"
    );

    let imported_gold = &result.materials[0];
    assert_eq!(imported_gold.name, "Gold");
    assert!((imported_gold.base_color_factor[0] - 1.0).abs() < 1e-3);
    assert!((imported_gold.base_color_factor[1] - 0.85).abs() < 1e-3);
    assert!((imported_gold.metallic_factor - 0.95).abs() < 1e-3);
    assert!((imported_gold.roughness_factor - 0.15).abs() < 1e-3);

    // Verify object 1 has the Gold material assigned
    let obj1_imported = result
        .objects
        .iter()
        .find(|o| o.name == "CubeA")
        .expect("CubeA should be present");
    assert_eq!(obj1_imported.material_id.as_deref(), Some("Gold"));
    assert!((obj1_imported.local_transform.translation.x - (-2.0)).abs() < 1e-4);
}

#[test]
fn test_glb_error_handling() {
    // 1. Empty bytes
    let err = import_gltf_from_bytes(&[], &GltfImportOptions::default(), None).unwrap_err();
    assert!(matches!(err, forge_io::ForgeIoError::EmptyFile));

    // 2. Truncated header
    let truncated = [0x67, 0x6C, 0x54, 0x46]; // only 4 bytes
    let err = import_gltf_from_bytes(&truncated, &GltfImportOptions::default(), None).unwrap_err();
    assert!(matches!(err, forge_io::ForgeIoError::TruncatedData { .. }));

    // 3. Invalid non-JSON binary data
    let mut invalid_magic = [0u8; 12];
    invalid_magic[0] = 0xAA;
    invalid_magic[1] = 0xBB;
    let err =
        import_gltf_from_bytes(&invalid_magic, &GltfImportOptions::default(), None).unwrap_err();
    assert!(matches!(err, forge_io::ForgeIoError::ParseError { .. }));

    // 4. Unsupported GLB version (version 99)
    let mut bad_version = [0u8; 24];
    bad_version[0..4].copy_from_slice(&0x4654_6C67u32.to_le_bytes());
    bad_version[4..8].copy_from_slice(&99u32.to_le_bytes());
    bad_version[8..12].copy_from_slice(&24u32.to_le_bytes());
    let err =
        import_gltf_from_bytes(&bad_version, &GltfImportOptions::default(), None).unwrap_err();
    assert!(matches!(
        err,
        forge_io::ForgeIoError::UnsupportedFormat { .. }
    ));
}
