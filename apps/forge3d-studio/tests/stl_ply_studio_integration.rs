use forge_geometry::primitives::generate_cube;
use forge_math::Vec3;
use forge_scene::{PrimitiveType, SceneObject};
use forge3d_studio::Forge3dStudioApp;
use std::io::Write;

fn create_temp_file(content: &[u8], filename: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("forge3d_{}_{}", std::process::id(), filename));
    let mut file = std::fs::File::create(&path).expect("Failed to create temp file");
    file.write_all(content)
        .expect("Failed to write temp content");
    path
}

#[test]
fn test_studio_import_stl_adds_to_scene_and_selection() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let ascii_stl = r#"solid SimpleTriangle
  facet normal 0 0 1
    outer loop
      vertex 0.0 0.0 0.0
      vertex 2.0 0.0 0.0
      vertex 1.0 2.0 0.0
    endloop
  endfacet
endsolid SimpleTriangle
"#;
    let temp_path = create_temp_file(ascii_stl.as_bytes(), "triangle.stl");
    let res = app.import_stl_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(res.is_ok(), "STL Import should succeed: {:?}", res);
    assert_eq!(app.scene.objects.len(), initial_count + 1);

    let imported_obj = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "SimpleTriangle")
        .expect("SimpleTriangle must be in scene");

    assert_eq!(imported_obj.mesh.indices.len(), 3);
    assert!(app.selection.is_selected(imported_obj.id));
    assert_eq!(app.selection.active_id, Some(imported_obj.id));
    assert!(app.status_message.contains("Imported STL"));
}

#[test]
fn test_studio_stl_import_undo_redo() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let ascii_stl = r#"solid UndoTriangle
  facet normal 0 0 1
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 0 1 0
    endloop
  endfacet
endsolid UndoTriangle
"#;
    let temp_path = create_temp_file(ascii_stl.as_bytes(), "undo_triangle.stl");
    app.import_stl_from_path(&temp_path).expect("Import failed");
    let _ = std::fs::remove_file(&temp_path);

    assert_eq!(app.scene.objects.len(), initial_count + 1);

    // Undo should remove the imported object
    app.undo();
    assert_eq!(app.scene.objects.len(), initial_count);

    // Redo should restore it
    app.redo();
    assert_eq!(app.scene.objects.len(), initial_count + 1);
}

#[test]
fn test_studio_export_scene_to_stl_and_reimport() {
    let mut app = Forge3dStudioApp::new_headless();
    app.scene.objects.clear();

    let cube = generate_cube(2.0);
    let mut obj = SceneObject::new("ExportCube", PrimitiveType::Cube, cube);
    obj.local_transform.translation = Vec3::new(3.0, 4.0, 5.0);
    app.scene.add_object(obj);

    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "forge3d_studio_stl_test_{}.stl",
        std::process::id()
    ));

    let export_res = app.export_stl_to_path(&temp_path, false);
    assert!(export_res.is_ok(), "Export failed: {:?}", export_res);

    // Re-import into a fresh app
    let mut app2 = Forge3dStudioApp::new_headless();
    app2.scene.objects.clear();

    let import_res = app2.import_stl_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(import_res.is_ok(), "Re-import failed: {:?}", import_res);
    assert_eq!(app2.scene.objects.len(), 1);

    let reimported_obj = app2.scene.objects.values().next().unwrap();
    // 12 triangles welded -> 8 vertices, 36 indices
    assert_eq!(reimported_obj.mesh.vertices.len(), 8);
    assert_eq!(reimported_obj.mesh.indices.len(), 36);

    // World transform baked: center at (3, 4, 5)
    let center = reimported_obj.mesh.aabb.center();
    assert!((center.x - 3.0).abs() < 1e-4);
    assert!((center.y - 4.0).abs() < 1e-4);
    assert!((center.z - 5.0).abs() < 1e-4);
}

#[test]
fn test_studio_export_selected_stl() {
    let mut app = Forge3dStudioApp::new_headless();
    app.scene.objects.clear();

    let obj1 = SceneObject::new("SelectedCube", PrimitiveType::Cube, generate_cube(1.0));
    let id1 = app.scene.add_object(obj1);

    let obj2 = SceneObject::new("IgnoredCube", PrimitiveType::Cube, generate_cube(1.0));
    app.scene.add_object(obj2);

    app.selection.select(id1);

    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "forge3d_selected_stl_test_{}.stl",
        std::process::id()
    ));

    let export_res = app.export_stl_to_path(&temp_path, true);
    assert!(
        export_res.is_ok(),
        "Export selected failed: {:?}",
        export_res
    );

    let mut app2 = Forge3dStudioApp::new_headless();
    app2.scene.objects.clear();
    app2.import_stl_from_path(&temp_path)
        .expect("Reimport failed");
    let _ = std::fs::remove_file(&temp_path);

    assert_eq!(app2.scene.objects.len(), 1);
    let reimported = app2.scene.objects.values().next().unwrap();
    assert_eq!(reimported.mesh.indices.len(), 36);
}

#[test]
fn test_studio_import_ply_adds_to_scene_and_selection() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let ply_content = r#"ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
element face 1
property list uchar int vertex_indices
end_header
0.0 0.0 0.0
2.0 0.0 0.0
1.0 2.0 0.0
3 0 1 2
"#;
    let temp_path = create_temp_file(ply_content.as_bytes(), "ply_tri.ply");
    let res = app.import_ply_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(res.is_ok(), "PLY Import should succeed: {:?}", res);
    assert_eq!(app.scene.objects.len(), initial_count + 1);

    let imported_obj = app
        .scene
        .objects
        .values()
        .find(|o| o.name.contains("ply_tri"))
        .expect("ply_tri must be in scene");

    assert_eq!(imported_obj.mesh.indices.len(), 3);
    assert!(app.selection.is_selected(imported_obj.id));
    assert_eq!(app.selection.active_id, Some(imported_obj.id));
    assert!(app.status_message.contains("Imported PLY"));
}

#[test]
fn test_studio_ply_import_undo_redo() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let ply_content = r#"ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
element face 1
property list uchar int vertex_indices
end_header
0 0 0
1 0 0
0 1 0
3 0 1 2
"#;
    let temp_path = create_temp_file(ply_content.as_bytes(), "undo_ply.ply");
    app.import_ply_from_path(&temp_path).expect("Import failed");
    let _ = std::fs::remove_file(&temp_path);

    assert_eq!(app.scene.objects.len(), initial_count + 1);

    app.undo();
    assert_eq!(app.scene.objects.len(), initial_count);

    app.redo();
    assert_eq!(app.scene.objects.len(), initial_count + 1);
}

#[test]
fn test_studio_export_scene_to_ply_and_reimport() {
    let mut app = Forge3dStudioApp::new_headless();
    app.scene.objects.clear();

    let cube = generate_cube(2.0);
    let mut obj = SceneObject::new("ExportPlyCube", PrimitiveType::Cube, cube);
    obj.local_transform.translation = Vec3::new(-2.0, 1.0, 4.0);
    app.scene.add_object(obj);

    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "forge3d_studio_ply_test_{}.ply",
        std::process::id()
    ));

    let export_res = app.export_ply_to_path(&temp_path, false);
    assert!(export_res.is_ok(), "PLY Export failed: {:?}", export_res);

    let mut app2 = Forge3dStudioApp::new_headless();
    app2.scene.objects.clear();

    let import_res = app2.import_ply_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(import_res.is_ok(), "PLY Re-import failed: {:?}", import_res);
    assert_eq!(app2.scene.objects.len(), 1);

    let reimported_obj = app2.scene.objects.values().next().unwrap();
    assert_eq!(reimported_obj.mesh.indices.len(), 36);

    let center = reimported_obj.mesh.aabb.center();
    assert!((center.x - (-2.0)).abs() < 1e-4);
    assert!((center.y - 1.0).abs() < 1e-4);
    assert!((center.z - 4.0).abs() < 1e-4);
}
