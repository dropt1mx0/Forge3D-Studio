use forge_geometry::primitives::generate_cube;
use forge_io::gltf::export_mesh_to_glb;
use forge_scene::{PbrMaterial, PrimitiveType, SceneObject};
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
fn test_studio_import_glb_adds_to_scene_and_selection() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let cube_mesh = generate_cube(1.5);
    let glb_bytes =
        export_mesh_to_glb(&cube_mesh, "StudioGlbCube").expect("GLB export of cube should succeed");

    let temp_path = create_temp_file(&glb_bytes, "test_cube.glb");
    let res = app.import_gltf_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(res.is_ok(), "glTF/GLB Import should succeed: {:?}", res);
    assert_eq!(app.scene.objects.len(), initial_count + 1);

    let imported_obj = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "StudioGlbCube")
        .expect("StudioGlbCube must be in scene");

    assert_eq!(imported_obj.mesh.vertices.len(), cube_mesh.vertices.len());
    assert!(app.selection.is_selected(imported_obj.id));
    assert_eq!(app.selection.active_id, Some(imported_obj.id));
    assert!(app.status_message.contains("Imported glTF/GLB"));
}

#[test]
fn test_studio_glb_import_undo_redo() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let cube_mesh = generate_cube(1.0);
    let glb_bytes = export_mesh_to_glb(&cube_mesh, "UndoGlbCube").unwrap();

    let temp_path = create_temp_file(&glb_bytes, "undo_cube.glb");
    app.import_gltf_from_path(&temp_path).unwrap();
    let _ = std::fs::remove_file(&temp_path);

    assert_eq!(app.scene.objects.len(), initial_count + 1);

    // Undo should remove the imported object
    app.undo();
    assert_eq!(app.scene.objects.len(), initial_count);
    assert!(app.scene.objects.values().all(|o| o.name != "UndoGlbCube"));

    // Redo should restore it
    app.redo();
    assert_eq!(app.scene.objects.len(), initial_count + 1);
    assert!(app.scene.objects.values().any(|o| o.name == "UndoGlbCube"));
}

#[test]
fn test_studio_export_scene_to_glb_and_reimport() {
    let mut app = Forge3dStudioApp::new_headless();
    app.scene.objects.clear();
    app.deselect_all();

    // Add material and object
    let mat = PbrMaterial {
        name: "StudioPbrMat".to_string(),
        base_color_factor: [0.2, 0.7, 0.9, 1.0],
        metallic_factor: 0.8,
        roughness_factor: 0.3,
        ..Default::default()
    };
    app.scene.materials.insert("StudioPbrMat".to_string(), mat);

    let mut obj = SceneObject::new(
        "ExportedGlbObj".to_string(),
        PrimitiveType::Cube,
        generate_cube(2.0),
    );
    obj.material_id = Some("StudioPbrMat".to_string());
    app.scene.add_object(obj);

    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("forge3d_{}_scene_export.glb", std::process::id()));

    let export_res = app.export_gltf_to_path(&temp_path, false);
    assert!(export_res.is_ok(), "GLB scene export should succeed");

    // Clear scene and re-import
    app.scene.objects.clear();
    app.scene.materials.clear();
    let import_res = app.import_gltf_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(import_res.is_ok(), "Re-import should succeed");
    assert_eq!(app.scene.objects.len(), 1);
    assert!(app.scene.materials.contains_key("StudioPbrMat"));

    let reimported_obj = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "ExportedGlbObj")
        .expect("ExportedGlbObj should exist");
    assert_eq!(reimported_obj.material_id.as_deref(), Some("StudioPbrMat"));
}
