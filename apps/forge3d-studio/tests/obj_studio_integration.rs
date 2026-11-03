use forge_math::{Transform, Vec3};
use forge_scene::{PrimitiveType, SceneObject};
use forge3d_studio::Forge3dStudioApp;
use std::io::Write;

fn create_temp_obj_file(content: &str, filename: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("forge3d_{}_{}", std::process::id(), filename));
    let mut file = std::fs::File::create(&path).expect("Failed to create temp OBJ file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp OBJ content");
    path
}

#[test]
fn test_studio_import_obj_adds_to_scene_and_selection() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let obj_content = r#"
o ImportedStatue
v 0.0 0.0 0.0
v 2.0 0.0 0.0
v 1.0 2.0 0.0
vn 0.0 0.0 1.0
f 1//1 2//1 3//1
"#;
    let temp_path = create_temp_obj_file(obj_content, "statue.obj");
    let res = app.import_obj_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(res.is_ok(), "Import should succeed: {:?}", res);
    assert_eq!(app.scene.objects.len(), initial_count + 1);

    // Verify object presence and attributes
    let imported_obj = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "ImportedStatue")
        .expect("ImportedStatue must be in scene");

    assert_eq!(imported_obj.mesh.indices.len(), 3);
    assert!(app.selection.is_selected(imported_obj.id));
    assert_eq!(app.selection.active_id, Some(imported_obj.id));
    assert!(app.status_message.contains("Imported"));
}

#[test]
fn test_studio_import_multi_object_single_undo_redo() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let obj_content = r#"
o FirstMesh
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3

o SecondMesh
v 2 0 0
v 3 0 0
v 2 1 0
f 4 5 6
"#;
    let temp_path = create_temp_obj_file(obj_content, "multi.obj");
    let res = app.import_obj_from_path(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    assert!(res.is_ok());
    assert_eq!(app.scene.objects.len(), initial_count + 2);

    // 1. Single Undo should remove BOTH imported objects at once
    app.undo();
    assert_eq!(app.scene.objects.len(), initial_count);
    assert!(
        app.scene
            .objects
            .values()
            .all(|o| o.name != "FirstMesh" && o.name != "SecondMesh")
    );
    assert!(app.status_message.contains("Undo"));

    // 2. Single Redo should restore BOTH objects
    app.redo();
    assert_eq!(app.scene.objects.len(), initial_count + 2);
    assert!(app.scene.objects.values().any(|o| o.name == "FirstMesh"));
    assert!(app.scene.objects.values().any(|o| o.name == "SecondMesh"));
    assert!(app.status_message.contains("Redo"));
}

#[test]
fn test_studio_export_scene_to_obj_and_reimport() {
    let mut app = Forge3dStudioApp::new_headless();
    // Clear default scene to have a clean slate
    app.scene.objects.clear();
    app.selection.deselect();

    // Add 2 objects with distinct transforms
    let mut obj1 = SceneObject::new(
        "HeroBox",
        PrimitiveType::Custom,
        forge_geometry::primitives::generate_cube(2.0),
    );
    obj1.local_transform = Transform::from_translation(Vec3::new(-10.0, 0.0, 0.0));
    app.scene.add_object(obj1);

    let mut obj2 = SceneObject::new(
        "SideMonkey",
        PrimitiveType::Custom,
        forge_geometry::primitives::generate_monkey(1.5),
    );
    obj2.local_transform = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
    app.scene.add_object(obj2);

    let mut export_path = std::env::temp_dir();
    export_path.push(format!("forge3d_test_export_{}.obj", std::process::id()));

    let export_res = app.export_obj_to_path(&export_path, false);
    assert!(
        export_res.is_ok(),
        "Export should succeed: {:?}",
        export_res
    );

    // Re-import into a new app instance
    let mut app2 = Forge3dStudioApp::new_headless();
    app2.scene.objects.clear();
    let import_res = app2.import_obj_from_path(&export_path);
    let _ = std::fs::remove_file(&export_path);

    assert!(
        import_res.is_ok(),
        "Re-import should succeed: {:?}",
        import_res
    );
    assert_eq!(app2.scene.objects.len(), 2);

    let re_box = app2
        .scene
        .objects
        .values()
        .find(|o| o.name == "HeroBox")
        .unwrap();
    let re_monkey = app2
        .scene
        .objects
        .values()
        .find(|o| o.name == "SideMonkey")
        .unwrap();

    assert!(re_box.mesh.aabb.center().x < -8.0);
    assert!(re_monkey.mesh.aabb.center().x > 8.0);
}

#[test]
fn test_studio_export_selected_only() {
    let mut app = Forge3dStudioApp::new_headless();
    app.scene.objects.clear();

    let obj1 = SceneObject::new(
        "SelectedObject",
        PrimitiveType::Custom,
        forge_geometry::primitives::generate_cube(1.0),
    );
    let id1 = app.scene.add_object(obj1);

    let obj2 = SceneObject::new(
        "IgnoredObject",
        PrimitiveType::Custom,
        forge_geometry::primitives::generate_cube(1.0),
    );
    app.scene.add_object(obj2);

    // Select only obj1
    app.selection.select(id1);

    let mut export_path = std::env::temp_dir();
    export_path.push(format!(
        "forge3d_test_sel_export_{}.obj",
        std::process::id()
    ));

    let export_res = app.export_obj_to_path(&export_path, true);
    assert!(export_res.is_ok());

    let mut app2 = Forge3dStudioApp::new_headless();
    app2.scene.objects.clear();
    let import_res = app2.import_obj_from_path(&export_path);
    let _ = std::fs::remove_file(&export_path);

    assert!(import_res.is_ok());
    assert_eq!(app2.scene.objects.len(), 1);
    assert_eq!(
        app2.scene.objects.values().next().unwrap().name,
        "SelectedObject"
    );
}

#[test]
fn test_studio_import_invalid_file_preserves_scene() {
    let mut app = Forge3dStudioApp::new_headless();
    let initial_count = app.scene.objects.len();

    let non_existent_path = std::path::PathBuf::from("/path/to/definitely_missing_file.obj");
    let res = app.import_obj_from_path(&non_existent_path);

    assert!(res.is_err());
    assert_eq!(app.scene.objects.len(), initial_count);
    assert!(app.status_message.contains("Import failed"));
}
