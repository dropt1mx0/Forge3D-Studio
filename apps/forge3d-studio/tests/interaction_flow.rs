use forge_math::{Quat, Ray, Transform, Vec2, Vec3};
use forge_render::{CameraProjection, ViewPreset};
use forge_scene::{AxisConstraint, SceneObject, TransformMode};
use forge3d_studio::Forge3dStudioApp;

#[test]
fn test_simulation_object_picking_and_selection() {
    let mut app = Forge3dStudioApp::new_headless();

    // Default scene has Cube at origin (0, 0, 0)
    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .expect("Default scene must contain Cube")
        .id;

    // Camera looks from (4.5, 3.5, 5.5) towards (0.0, 0.5, 0.0)
    // Ray pointing directly at (0, 0, 0) from camera eye
    let ray_to_cube = Ray::new(app.camera.eye, (Vec3::ZERO - app.camera.eye).normalize());

    // 1. Raycast against scene
    let hit = app.scene.raycast(&ray_to_cube);
    assert!(hit.is_some(), "Ray directed at Cube must hit");
    let (hit_id, dist) = hit.unwrap();
    assert_eq!(hit_id, cube_id, "Hit object must be the Cube");
    assert!(dist > 0.0);

    // 2. Select Cube
    app.selection.select(cube_id);
    assert!(app.selection.is_selected(cube_id));
    assert_eq!(app.selection.active_id, Some(cube_id));
    assert_eq!(app.selection.selected_ids.len(), 1);

    // 3. Multi-selection: Shift-select Sphere
    let sphere_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Sphere")
        .expect("Default scene must contain Sphere")
        .id;
    app.selection.toggle_selection(sphere_id);
    assert!(app.selection.is_selected(cube_id));
    assert!(app.selection.is_selected(sphere_id));
    assert_eq!(app.selection.selected_ids.len(), 2);
    assert_eq!(app.selection.active_id, Some(sphere_id));

    // 4. Toggle off Sphere
    app.selection.toggle_selection(sphere_id);
    assert!(app.selection.is_selected(cube_id));
    assert!(!app.selection.is_selected(sphere_id));
    assert_eq!(app.selection.selected_ids.len(), 1);

    // 5. Deselect all on clicking empty space
    app.selection.deselect();
    assert!(app.selection.is_empty());
    assert_eq!(app.selection.active_id, None);
}

#[test]
fn test_simulation_modal_transform_g_x_5_enter_undo_redo() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    let initial_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;

    // Step 1: Select object
    app.selection.select(cube_id);

    // Step 2: Press 'G' (Grab / Translate)
    app.begin_modal_transform(TransformMode::Translate, Vec2::new(100.0, 100.0));
    assert!(app.editor_state.modal_transform.active);
    assert_eq!(
        app.editor_state.modal_transform.mode,
        TransformMode::Translate
    );

    // Step 3: Press 'X' (Lock to X axis)
    app.editor_state
        .modal_transform
        .set_or_toggle_axis(AxisConstraint::X);
    assert_eq!(
        app.editor_state.modal_transform.constraint,
        Some(AxisConstraint::X)
    );

    // Step 4: Type '5'
    app.editor_state.modal_transform.push_char('5');
    assert_eq!(app.editor_state.modal_transform.parsed_number(), Some(5.0));

    // Apply delta to initial transforms (simulates viewport modal loop)
    let num = app.editor_state.modal_transform.parsed_number().unwrap();
    let mut translate_delta = Vec3::ZERO;
    translate_delta.x = num;
    for (&id, &initial_tf) in &app.editor_state.modal_transform.initial_transforms {
        if let Some(obj) = app.scene.get_object_mut(id) {
            obj.local_transform.translation = initial_tf.translation + translate_delta;
        }
    }

    // Step 5: Press 'Enter' (Confirm)
    app.apply_modal_transform();
    assert!(!app.editor_state.modal_transform.active);

    // Verify object moved by +5 along X
    let current_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert_eq!(current_pos, initial_pos + Vec3::new(5.0, 0.0, 0.0));

    // Step 6: Press 'Ctrl+Z' (Undo)
    app.undo();
    let undone_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert_eq!(
        undone_pos, initial_pos,
        "Undo must restore initial position"
    );

    // Step 7: Press 'Ctrl+Shift+Z' (Redo)
    app.redo();
    let redone_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert_eq!(
        redone_pos,
        initial_pos + Vec3::new(5.0, 0.0, 0.0),
        "Redo must restore +5.0 position"
    );
}

#[test]
fn test_simulation_modal_rotate_r_y_90_enter() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    let initial_rot = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .rotation;

    app.selection.select(cube_id);

    // Press 'R'
    app.begin_modal_transform(TransformMode::Rotate, Vec2::new(100.0, 100.0));
    assert_eq!(app.editor_state.modal_transform.mode, TransformMode::Rotate);

    // Press 'Y'
    app.editor_state
        .modal_transform
        .set_or_toggle_axis(AxisConstraint::Y);
    assert_eq!(
        app.editor_state.modal_transform.constraint,
        Some(AxisConstraint::Y)
    );

    // Type '90'
    app.editor_state.modal_transform.push_char('9');
    app.editor_state.modal_transform.push_char('0');
    assert_eq!(app.editor_state.modal_transform.parsed_number(), Some(90.0));

    let angle_rad = 90.0_f32.to_radians();
    let rot_quat = Quat::from_axis_angle(Vec3::Y, angle_rad);
    for (&id, &initial_tf) in &app.editor_state.modal_transform.initial_transforms {
        if let Some(obj) = app.scene.get_object_mut(id) {
            obj.local_transform.rotation = rot_quat * initial_tf.rotation;
        }
    }

    app.apply_modal_transform();
    let rotated = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .rotation;
    assert!((rotated.dot(rot_quat * initial_rot) - 1.0).abs() < 1e-4);

    // Undo
    app.undo();
    let restored = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .rotation;
    assert_eq!(restored, initial_rot);
}

#[test]
fn test_simulation_modal_scale_s_2_enter() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    let initial_scale = app.scene.get_object(cube_id).unwrap().local_transform.scale;

    app.selection.select(cube_id);

    // Press 'S'
    app.begin_modal_transform(TransformMode::Scale, Vec2::new(100.0, 100.0));
    assert_eq!(app.editor_state.modal_transform.mode, TransformMode::Scale);

    // Type '2'
    app.editor_state.modal_transform.push_char('2');
    assert_eq!(app.editor_state.modal_transform.parsed_number(), Some(2.0));

    let scale_factor = 2.0;
    for (&id, &initial_tf) in &app.editor_state.modal_transform.initial_transforms {
        if let Some(obj) = app.scene.get_object_mut(id) {
            obj.local_transform.scale = initial_tf.scale * Vec3::splat(scale_factor);
        }
    }

    app.apply_modal_transform();
    let scaled = app.scene.get_object(cube_id).unwrap().local_transform.scale;
    assert_eq!(scaled, initial_scale * 2.0);

    // Undo
    app.undo();
    assert_eq!(
        app.scene.get_object(cube_id).unwrap().local_transform.scale,
        initial_scale
    );
}

#[test]
fn test_simulation_camera_navigation() {
    let mut app = Forge3dStudioApp::new_headless();
    let start_eye = app.camera.eye;
    let start_target = app.camera.target;

    // 1. Orbit
    app.camera.orbit(0.2, 0.1);
    assert_ne!(app.camera.eye, start_eye);
    assert_eq!(
        app.camera.target, start_target,
        "Orbit must keep pivot fixed"
    );

    // 2. Pan
    let eye_before_pan = app.camera.eye;
    let target_before_pan = app.camera.target;
    app.camera.pan(10.0, 5.0);
    assert_ne!(app.camera.eye, eye_before_pan);
    assert_ne!(app.camera.target, target_before_pan);

    // 3. Zoom
    let dist_before_zoom = (app.camera.eye - app.camera.target).length();
    app.camera.zoom(1.0); // Zoom in
    let dist_after_zoom = (app.camera.eye - app.camera.target).length();
    assert!(
        dist_after_zoom < dist_before_zoom,
        "Positive zoom factor must decrease distance to target"
    );

    // 4. Presets
    let current_target = app.camera.target;
    app.camera.set_view_preset(ViewPreset::Front);
    assert_eq!(app.camera.target, current_target);
    assert!(app.camera.eye.z > app.camera.target.z);

    app.camera.set_view_preset(ViewPreset::Top);
    assert!(app.camera.eye.y > app.camera.target.y);

    // 5. Projection toggle
    assert_eq!(app.camera.projection, CameraProjection::Perspective);
    app.camera.toggle_projection();
    assert_eq!(app.camera.projection, CameraProjection::Orthographic);

    // 6. Reset
    app.camera.reset_view();
    assert_eq!(app.camera.projection, CameraProjection::Perspective);
    assert_eq!(app.camera.target, Vec3::new(0.0, 0.5, 0.0));
}

#[test]
fn test_simulation_selection_tools_all_none_invert_box() {
    let mut app = Forge3dStudioApp::new_headless();
    let total_visible = app.scene.objects.values().filter(|o| o.visible).count();

    // 1. Select All ('A')
    app.select_all();
    assert_eq!(app.selection.selected_ids.len(), total_visible);

    // 2. Deselect All ('Alt+A')
    app.deselect_all();
    assert!(app.selection.is_empty());

    // 3. Invert Selection ('Ctrl+I')
    app.invert_selection();
    assert_eq!(app.selection.selected_ids.len(), total_visible);
    app.invert_selection();
    assert!(app.selection.is_empty());

    // 4. Box Select
    // Frame all objects so every object in the scene is within the camera frustum
    app.frame_all();
    let view_proj = app.camera.view_proj_matrix();
    let viewport_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
    // Full screen box must select all visible selectable objects
    let total_selectable = app.scene.objects.values().filter(|o| o.visible && o.selectable).count();
    app.box_select(viewport_rect, view_proj, viewport_rect, false);
    assert_eq!(app.selection.selected_ids.len(), total_selectable);
}

#[test]
fn test_simulation_scene_camera_object() {
    let mut app = Forge3dStudioApp::new_headless();

    // Add physical camera object to scene
    let cam_obj = SceneObject::new_camera("Studio Camera")
        .with_transform(Transform::from_translation(Vec3::new(0.0, 2.0, 8.0)));
    let cam_id = app.scene.add_object(cam_obj);

    // Make it active camera
    app.scene.set_active_camera(cam_id);
    assert_eq!(app.scene.active_camera_id, Some(cam_id));

    // Align active camera to view
    app.align_active_camera_to_view();
    let cam_tf = app.scene.world_transform(cam_id);
    assert!((cam_tf.translation - app.camera.eye).length() < 1e-4);

    // View through active camera
    app.toggle_camera_view();
    assert!(app.editor_state.is_camera_view);

    // Exit camera view
    app.toggle_camera_view();
    assert!(!app.editor_state.is_camera_view);
}

#[test]
fn test_simulation_scene_persistence() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    // Modify Cube transform
    if let Some(obj) = app.scene.get_object_mut(cube_id) {
        obj.local_transform.translation = Vec3::new(3.5, 7.0, -2.2);
        obj.local_transform.scale = Vec3::new(1.5, 2.0, 0.5);
    }

    // Save project to string
    let serialized = app.save_to_string().expect("Scene must serialize cleanly");
    assert!(!serialized.is_empty());

    // Create fresh app and load
    let mut new_app = Forge3dStudioApp::new_headless();
    new_app
        .load_from_string(&serialized)
        .expect("Scene must load cleanly");

    let loaded_cube = new_app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .expect("Loaded scene must contain Cube");

    assert!((loaded_cube.local_transform.translation - Vec3::new(3.5, 7.0, -2.2)).length() < 1e-4);
    assert!((loaded_cube.local_transform.scale - Vec3::new(1.5, 2.0, 0.5)).length() < 1e-4);
}
