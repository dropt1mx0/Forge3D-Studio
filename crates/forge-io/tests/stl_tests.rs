use forge_geometry::primitives::{generate_cube, generate_uv_sphere};
use forge_io::{
    ForgeIoError, StlExportOptions, StlFormat, StlImportOptions, export_mesh_stl, export_scene_stl,
    import_stl_from_bytes, parse_stl_from_bytes,
};
use forge_math::Vec3;
use forge_scene::{PrimitiveType, Scene, SceneObject};

#[test]
fn test_stl_ascii_cube_roundtrip() {
    let original = generate_cube(2.0);
    let export_opts = StlExportOptions {
        format: StlFormat::Ascii,
        apply_world_transform: false,
        solid_name: "AsciiCube".to_string(),
        visible_only: true,
    };

    let stl_bytes = export_mesh_stl(&original, &export_opts).expect("Export failed");
    assert!(stl_bytes.starts_with(b"solid AsciiCube"));

    let doc = parse_stl_from_bytes(&stl_bytes).expect("Parse failed");
    assert_eq!(doc.format, StlFormat::Ascii);
    assert_eq!(doc.facets.len(), 12); // Cube has 12 triangles

    let import_opts = StlImportOptions {
        weld_vertices: true,
        ..Default::default()
    };
    let imported = import_stl_from_bytes(&stl_bytes, &import_opts).expect("Import failed");
    assert_eq!(imported.objects.len(), 1);
    let imported_mesh = &imported.objects[0].mesh;

    // Cube has 8 unique vertices when welded
    assert_eq!(imported_mesh.vertices.len(), 8);
    assert_eq!(imported_mesh.indices.len(), 36);

    // Verify bounds match
    assert!((original.aabb.min.x - imported_mesh.aabb.min.x).abs() < 1e-4);
    assert!((original.aabb.max.x - imported_mesh.aabb.max.x).abs() < 1e-4);
}

#[test]
fn test_stl_binary_cube_roundtrip() {
    let original = generate_cube(2.0);
    let export_opts = StlExportOptions {
        format: StlFormat::Binary,
        apply_world_transform: false,
        solid_name: "BinaryCube".to_string(),
        visible_only: true,
    };

    let stl_bytes = export_mesh_stl(&original, &export_opts).expect("Export failed");
    // Binary STL size: 84 + 12 * 50 = 684 bytes
    assert_eq!(stl_bytes.len(), 84 + 12 * 50);

    let doc = parse_stl_from_bytes(&stl_bytes).expect("Parse failed");
    assert_eq!(doc.format, StlFormat::Binary);
    assert_eq!(doc.facets.len(), 12);

    let import_opts = StlImportOptions {
        weld_vertices: true,
        ..Default::default()
    };
    let imported = import_stl_from_bytes(&stl_bytes, &import_opts).expect("Import failed");
    assert_eq!(imported.objects.len(), 1);
    let imported_mesh = &imported.objects[0].mesh;

    assert_eq!(imported_mesh.vertices.len(), 8);
    assert_eq!(imported_mesh.indices.len(), 36);
    assert!(imported_mesh.validate().is_ok());
}

#[test]
fn test_stl_unwelded_mode() {
    let original = generate_cube(1.0);
    let export_opts = StlExportOptions {
        format: StlFormat::Binary,
        ..Default::default()
    };
    let stl_bytes = export_mesh_stl(&original, &export_opts).expect("Export failed");

    let import_opts = StlImportOptions {
        weld_vertices: false,
        ..Default::default()
    };
    let imported = import_stl_from_bytes(&stl_bytes, &import_opts).expect("Import failed");
    let imported_mesh = &imported.objects[0].mesh;

    // 12 triangles * 3 = 36 vertices without welding
    assert_eq!(imported_mesh.vertices.len(), 36);
    assert_eq!(imported_mesh.indices.len(), 36);
}

#[test]
fn test_stl_scene_export_with_transform() {
    let mut scene = Scene::new("StlScene");

    let mut obj1 = SceneObject::new("Cube1", PrimitiveType::Cube, generate_cube(1.0));
    obj1.local_transform.translation = Vec3::new(10.0, 0.0, 0.0);
    scene.add_object(obj1);

    let mut obj2 = SceneObject::new("Cube2", PrimitiveType::Cube, generate_cube(1.0));
    obj2.local_transform.translation = Vec3::new(-10.0, 0.0, 0.0);
    scene.add_object(obj2);

    let export_opts = StlExportOptions {
        format: StlFormat::Binary,
        apply_world_transform: true,
        solid_name: "TwoCubes".to_string(),
        visible_only: true,
    };

    let stl_bytes = export_scene_stl(&scene, &export_opts).expect("Scene export failed");
    // 2 cubes * 12 triangles = 24 triangles -> 84 + 24 * 50 = 1284 bytes
    assert_eq!(stl_bytes.len(), 84 + 24 * 50);

    let imported = import_stl_from_bytes(&stl_bytes, &StlImportOptions::default()).unwrap();
    let mesh = &imported.objects[0].mesh;

    // Center of cubes at x=10 and x=-10 with half-size 0.5 -> min.x = -10.5, max.x = 10.5
    assert!((mesh.aabb.min.x - (-10.5)).abs() < 1e-4);
    assert!((mesh.aabb.max.x - 10.5).abs() < 1e-4);
}

#[test]
fn test_stl_empty_error() {
    let empty_bytes = b"";
    let err = import_stl_from_bytes(empty_bytes, &StlImportOptions::default()).unwrap_err();
    match err {
        ForgeIoError::EmptyFile => {}
        other => panic!("Expected EmptyFile error, got {:?}", other),
    }
}

#[test]
fn test_stl_truncated_binary_error() {
    let mut truncated = vec![0u8; 84]; // 84 bytes with triangle_count = 10, but no facet data
    truncated[80] = 10;
    let err = import_stl_from_bytes(&truncated, &StlImportOptions::default()).unwrap_err();
    match err {
        ForgeIoError::TruncatedData {
            format,
            expected,
            found,
        } => {
            assert_eq!(format, "STL (Binary)");
            assert_eq!(expected, 84 + 10 * 50);
            assert_eq!(found, 84);
        }
        other => panic!("Expected TruncatedData error, got {:?}", other),
    }
}

#[test]
fn test_stl_sphere_smooth_normals() {
    let sphere = generate_uv_sphere(1.0, 16, 8);
    let stl_bytes = export_mesh_stl(
        &sphere,
        &StlExportOptions {
            format: StlFormat::Binary,
            ..Default::default()
        },
    )
    .unwrap();

    let imported = import_stl_from_bytes(
        &stl_bytes,
        &StlImportOptions {
            weld_vertices: true,
            recalculate_normals: true,
            ..Default::default()
        },
    )
    .unwrap();

    let mesh = &imported.objects[0].mesh;
    for v in &mesh.vertices {
        let norm_len =
            (v.normal[0] * v.normal[0] + v.normal[1] * v.normal[1] + v.normal[2] * v.normal[2])
                .sqrt();
        assert!((norm_len - 1.0).abs() < 1e-3, "Normal must be normalized");
    }
}
