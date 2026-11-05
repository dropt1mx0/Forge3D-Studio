use forge_geometry::primitives::{generate_cube, generate_uv_sphere};
use forge_io::{
    ForgeIoError, PlyExportOptions, PlyFormat, PlyImportOptions, export_mesh_ply, export_scene_ply,
    import_ply_from_bytes, parse_ply_from_bytes,
};
use forge_math::Vec3;
use forge_scene::{PrimitiveType, Scene, SceneObject};

#[test]
fn test_ply_ascii_cube_roundtrip() {
    let original = generate_cube(2.0);
    let export_opts = PlyExportOptions {
        format: PlyFormat::Ascii,
        apply_world_transform: false,
        export_normals: true,
        export_colors: true,
        export_uvs: true,
        visible_only: true,
        comments: vec!["Ascii cube test".to_string()],
    };

    let ply_bytes = export_mesh_ply(&original, &export_opts).expect("Export failed");
    assert!(ply_bytes.starts_with(b"ply\nformat ascii 1.0"));

    let doc = parse_ply_from_bytes(&ply_bytes).expect("Parse failed");
    assert_eq!(doc.format, PlyFormat::Ascii);
    assert_eq!(doc.vertices.len(), original.vertices.len());
    assert_eq!(doc.faces.len(), original.indices.len() / 3);

    let import_opts = PlyImportOptions::default();
    let imported = import_ply_from_bytes(&ply_bytes, &import_opts).expect("Import failed");
    assert_eq!(imported.objects.len(), 1);

    let imported_mesh = &imported.objects[0].mesh;
    assert_eq!(imported_mesh.vertices.len(), original.vertices.len());
    assert_eq!(imported_mesh.indices.len(), original.indices.len());

    assert!((original.aabb.min.x - imported_mesh.aabb.min.x).abs() < 1e-4);
    assert!((original.aabb.max.x - imported_mesh.aabb.max.x).abs() < 1e-4);
}

#[test]
fn test_ply_binary_le_cube_roundtrip() {
    let original = generate_cube(2.0);
    let export_opts = PlyExportOptions {
        format: PlyFormat::BinaryLittleEndian,
        apply_world_transform: false,
        export_normals: true,
        export_colors: true,
        export_uvs: true,
        visible_only: true,
        comments: vec!["Binary LE cube test".to_string()],
    };

    let ply_bytes = export_mesh_ply(&original, &export_opts).expect("Export failed");
    assert!(ply_bytes.starts_with(b"ply\nformat binary_little_endian 1.0"));

    let doc = parse_ply_from_bytes(&ply_bytes).expect("Parse failed");
    assert_eq!(doc.format, PlyFormat::BinaryLittleEndian);
    assert_eq!(doc.vertices.len(), original.vertices.len());
    assert_eq!(doc.faces.len(), original.indices.len() / 3);

    let import_opts = PlyImportOptions::default();
    let imported = import_ply_from_bytes(&ply_bytes, &import_opts).expect("Import failed");
    assert_eq!(imported.objects.len(), 1);

    let imported_mesh = &imported.objects[0].mesh;
    assert_eq!(imported_mesh.vertices.len(), original.vertices.len());
    assert_eq!(imported_mesh.indices.len(), original.indices.len());
    assert!(imported_mesh.validate().is_ok());
}

#[test]
fn test_ply_vertex_colors_roundtrip() {
    let mut original = generate_cube(1.0);
    // Assign unique vertex colors
    let total_v = original.vertices.len() as f32;
    for (i, v) in original.vertices.iter_mut().enumerate() {
        let t = i as f32 / total_v;
        v.color = [t, 1.0 - t, 0.5, 1.0];
    }

    let export_opts = PlyExportOptions {
        format: PlyFormat::BinaryLittleEndian,
        export_colors: true,
        ..Default::default()
    };
    let ply_bytes = export_mesh_ply(&original, &export_opts).unwrap();

    let imported = import_ply_from_bytes(&ply_bytes, &PlyImportOptions::default()).unwrap();
    let mesh = &imported.objects[0].mesh;

    for (v_orig, v_imp) in original.vertices.iter().zip(mesh.vertices.iter()) {
        // Quantized to u8 in PLY, so tolerance is ~1/255 ≈ 0.005
        assert!((v_orig.color[0] - v_imp.color[0]).abs() < 0.01);
        assert!((v_orig.color[1] - v_imp.color[1]).abs() < 0.01);
        assert!((v_orig.color[2] - v_imp.color[2]).abs() < 0.01);
    }
}

#[test]
fn test_ply_quad_and_ngon_triangulation() {
    let ply_ascii = r#"ply
format ascii 1.0
comment Quad and Pentagon test
element vertex 5
property float x
property float y
property float z
element face 2
property list uchar int vertex_indices
end_header
0 0 0
1 0 0
1 1 0
0 1 0
0.5 1.5 0
4 0 1 2 3
5 0 1 2 4 3
"#;

    let imported =
        import_ply_from_bytes(ply_ascii.as_bytes(), &PlyImportOptions::default()).unwrap();
    let mesh = &imported.objects[0].mesh;

    assert_eq!(mesh.vertices.len(), 5);
    // Quad yields 2 triangles (6 indices) + Pentagon yields 3 triangles (9 indices) = 15 indices
    assert_eq!(mesh.indices.len(), 15);
}

#[test]
fn test_ply_scene_export_with_transform() {
    let mut scene = Scene::new("PlyScene");

    let mut obj1 = SceneObject::new(
        "Sphere1",
        PrimitiveType::Sphere,
        generate_uv_sphere(1.0, 8, 4),
    );
    obj1.local_transform.translation = Vec3::new(5.0, 0.0, 0.0);
    scene.add_object(obj1);

    let export_opts = PlyExportOptions {
        format: PlyFormat::BinaryLittleEndian,
        apply_world_transform: true,
        ..Default::default()
    };

    let ply_bytes = export_scene_ply(&scene, &export_opts).unwrap();
    let imported = import_ply_from_bytes(&ply_bytes, &PlyImportOptions::default()).unwrap();
    let mesh = &imported.objects[0].mesh;

    assert!((mesh.aabb.center().x - 5.0).abs() < 1e-4);
}

#[test]
fn test_ply_invalid_header_missing_signature() {
    let invalid = b"not_ply\nformat ascii 1.0\nend_header\n";
    let err = import_ply_from_bytes(invalid, &PlyImportOptions::default()).unwrap_err();
    match err {
        ForgeIoError::InvalidHeader { format, message } => {
            assert_eq!(format, "PLY");
            assert!(message.contains("Expected 'ply' signature"));
        }
        other => panic!("Expected InvalidHeader error, got {:?}", other),
    }
}
