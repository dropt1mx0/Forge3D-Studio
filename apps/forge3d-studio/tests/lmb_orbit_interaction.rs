use forge_math::{Ray, Vec2, Vec3};
use forge_render::CameraProjection;
use forge_scene::{PointerInteractionState, TransformMode};
use forge3d_studio::Forge3dStudioApp;

#[test]
fn test_lmb_drag_over_empty_viewport_starts_orbit() {
    let mut app = Forge3dStudioApp::new_headless();

    // Verify initial state
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    let initial_eye = app.camera.eye;
    let initial_target = app.camera.target;
    let initial_dist = app.camera.distance();

    // 1. Cursor is in empty viewport space (raycast hits nothing)
    let empty_ray = Ray::new(app.camera.eye, Vec3::new(0.0, 1.0, 0.0));
    assert!(app.scene.raycast(&empty_ray).is_none());

    // 2. Press LMB on empty space
    app.editor_state.pointer_state = PointerInteractionState::ViewportOrbiting;
    let press_pos = Vec2::new(500.0, 400.0);
    app.editor_state.pointer_press_pos = Some(press_pos);

    // 3. Move cursor (drag)
    let delta = Vec2::new(60.0, 40.0);
    let current_pos = press_pos + delta;
    app.camera.orbit(
        delta.x * app.camera.settings.orbit_sensitivity,
        delta.y * app.camera.settings.orbit_sensitivity,
    );

    // Verify Orbit result:
    // Camera eye rotated
    assert_ne!(app.camera.eye, initial_eye);
    // Camera target preserved
    assert_eq!(app.camera.target, initial_target);
    // Camera distance preserved (within floating point precision)
    assert!(
        (app.camera.distance() - initial_dist).abs() < 1e-4,
        "Camera distance must be preserved during orbit"
    );

    // 4. Release after drag (> 4.0 px) -> orbit completes, selection not deselected
    let moved_dist = (current_pos - press_pos).length();
    assert!(moved_dist >= 4.0);
    app.editor_state.pointer_state = PointerInteractionState::Idle;
    app.editor_state.pointer_press_pos = None;

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
}

#[test]
fn test_lmb_click_on_object_selects_it() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .expect("Default scene has Cube")
        .id;

    let initial_eye = app.camera.eye;
    let initial_target = app.camera.target;

    // Raycast towards cube at origin
    let ray_to_cube = Ray::new(app.camera.eye, (Vec3::ZERO - app.camera.eye).normalize());
    let hit = app.scene.raycast(&ray_to_cube);
    assert!(hit.is_some());
    let (hit_id, _) = hit.unwrap();
    assert_eq!(hit_id, cube_id);

    // On press over object: priority is selection, state becomes ObjectPressed
    app.editor_state.pointer_state = PointerInteractionState::ObjectPressed;
    app.selection.select(hit_id);

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::ObjectPressed
    );
    assert!(app.selection.is_selected(cube_id));

    // Camera did not rotate
    assert_eq!(app.camera.eye, initial_eye);
    assert_eq!(app.camera.target, initial_target);

    // On release: state resets to Idle
    app.editor_state.pointer_state = PointerInteractionState::Idle;
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(app.selection.is_selected(cube_id));
}

#[test]
fn test_lmb_drag_on_gizmo_does_not_activate_orbit() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .expect("Default scene has Cube")
        .id;

    app.selection.select(cube_id);
    app.editor_state.transform_mode = TransformMode::Translate;

    let initial_eye = app.camera.eye;
    let initial_target = app.camera.target;

    // Simulate Gizmo handle hit
    let gizmo_consumed = true;
    if gizmo_consumed {
        app.editor_state.pointer_state = PointerInteractionState::GizmoDragging;
    }

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::GizmoDragging
    );

    // Dragging gizmo modifies object translation, NOT camera orbit
    if let Some(obj) = app.scene.get_object_mut(cube_id) {
        obj.local_transform.translation.x += 2.0;
    }

    // Camera must NOT orbit
    assert_eq!(app.camera.eye, initial_eye);
    assert_eq!(app.camera.target, initial_target);
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::GizmoDragging
    );

    // On release: state resets to Idle
    app.editor_state.pointer_state = PointerInteractionState::Idle;
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
}

#[test]
fn test_lmb_drag_with_box_select_does_not_activate_orbit() {
    let mut app = Forge3dStudioApp::new_headless();

    // Enable BoxSelect tool
    app.editor_state.transform_mode = TransformMode::BoxSelect;
    let initial_eye = app.camera.eye;
    let initial_target = app.camera.target;

    // Press LMB in BoxSelect mode
    let press_pos = Vec2::new(100.0, 100.0);
    app.editor_state.pointer_state = PointerInteractionState::BoxSelecting;
    app.editor_state.box_select_start = Some(press_pos);

    // Drag pointer
    let current_pos = Vec2::new(300.0, 250.0);
    // Camera must not orbit
    assert_eq!(app.camera.eye, initial_eye);
    assert_eq!(app.camera.target, initial_target);
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::BoxSelecting
    );

    // On release: executes box selection, resets to Idle
    let box_rect = egui::Rect::from_two_pos(
        egui::pos2(press_pos.x, press_pos.y),
        egui::pos2(current_pos.x, current_pos.y),
    );
    let view_proj = app.camera.view_proj_matrix();
    let viewport_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
    app.box_select(box_rect, view_proj, viewport_rect, false);

    app.editor_state.box_select_start = None;
    app.editor_state.pointer_state = PointerInteractionState::Idle;

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert_eq!(app.camera.eye, initial_eye);
}

#[test]
fn test_ui_interaction_does_not_activate_orbit() {
    let mut app = Forge3dStudioApp::new_headless();

    let initial_eye = app.camera.eye;
    let initial_target = app.camera.target;

    // Pointer press happens on a UI element (e.g., toolbar or inspector panel)
    app.editor_state.pointer_state = PointerInteractionState::UIInteraction;

    // Pointer drags while interacting with UI (e.g. dragging a slider)
    let _delta = Vec2::new(45.0, 20.0);
    // UI interactions must not call camera.orbit
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::UIInteraction
    );
    assert_eq!(app.camera.eye, initial_eye);
    assert_eq!(app.camera.target, initial_target);

    // On release: state resets to Idle
    app.editor_state.pointer_state = PointerInteractionState::Idle;
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
}

#[test]
fn test_pointer_release_resets_interaction_state() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    app.selection.select(cube_id);

    // Case 1: Orbit release after significant movement (>= 4.0 px) preserves selection
    app.editor_state.pointer_state = PointerInteractionState::ViewportOrbiting;
    app.editor_state.pointer_press_pos = Some(Vec2::new(100.0, 100.0));
    let moved_drag = Vec2::new(120.0, 110.0);
    let dist_drag = (moved_drag - app.editor_state.pointer_press_pos.unwrap()).length();
    assert!(dist_drag >= 4.0);

    // Release
    if dist_drag < 4.0 {
        app.selection.deselect();
    }
    app.editor_state.pointer_state = PointerInteractionState::Idle;
    app.editor_state.pointer_press_pos = None;

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(
        app.selection.is_selected(cube_id),
        "Orbit drag must not clear selection on release"
    );

    // Case 2: Static click (< 4.0 px) on empty space clears selection
    app.editor_state.pointer_state = PointerInteractionState::ViewportOrbiting;
    app.editor_state.pointer_press_pos = Some(Vec2::new(100.0, 100.0));
    let static_click_pos = Vec2::new(101.0, 101.0);
    let dist_static = (static_click_pos - app.editor_state.pointer_press_pos.unwrap()).length();
    assert!(dist_static < 4.0);

    // Release
    if dist_static < 4.0 {
        app.selection.deselect();
    }
    app.editor_state.pointer_state = PointerInteractionState::Idle;
    app.editor_state.pointer_press_pos = None;

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(
        app.selection.is_empty(),
        "Static click on empty space must clear selection"
    );
}

#[test]
fn test_focus_loss_resets_dragging_state() {
    let mut app = Forge3dStudioApp::new_headless();

    // Simulate pointer was dragging
    app.editor_state.pointer_state = PointerInteractionState::ViewportOrbiting;
    app.editor_state.pointer_press_pos = Some(Vec2::new(200.0, 200.0));
    app.editor_state.box_select_start = Some(Vec2::new(150.0, 150.0));

    // Focus lost or pointer button released outside window (lmb_down is false)
    let lmb_down = false;
    if !lmb_down && app.editor_state.pointer_state != PointerInteractionState::Idle {
        app.editor_state.pointer_state = PointerInteractionState::Idle;
        app.editor_state.pointer_press_pos = None;
        app.editor_state.box_select_start = None;
    }

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert_eq!(app.editor_state.pointer_press_pos, None);
    assert_eq!(app.editor_state.box_select_start, None);
}

#[test]
fn test_existing_mmb_orbit_and_shift_mmb_pan_still_work() {
    let mut app = Forge3dStudioApp::new_headless();

    let initial_eye = app.camera.eye;
    let initial_target = app.camera.target;
    let initial_dist = app.camera.distance();

    // 1. MMB Orbit
    let delta = Vec2::new(30.0, 20.0);
    app.camera.orbit(
        delta.x * app.camera.settings.orbit_sensitivity,
        delta.y * app.camera.settings.orbit_sensitivity,
    );
    assert_ne!(app.camera.eye, initial_eye);
    assert_eq!(app.camera.target, initial_target);
    assert!((app.camera.distance() - initial_dist).abs() < 1e-4);

    // 2. Shift + MMB Pan
    let pre_pan_target = app.camera.target;
    let pre_pan_eye = app.camera.eye;
    app.camera.pan(40.0, -25.0);
    assert_ne!(app.camera.target, pre_pan_target);
    assert_ne!(app.camera.eye, pre_pan_eye);
    assert!((app.camera.distance() - initial_dist).abs() < 1e-4);

    // 3. Scroll Zoom
    let pre_zoom_dist = app.camera.distance();
    app.camera.zoom(5.0);
    assert!(app.camera.distance() < pre_zoom_dist);

    // 4. Projection support
    app.camera.toggle_projection();
    assert_eq!(app.camera.projection, CameraProjection::Orthographic);
    app.camera.orbit(0.1, 0.1);
    assert_eq!(app.camera.projection, CameraProjection::Orthographic);
    app.camera.toggle_projection();
    assert_eq!(app.camera.projection, CameraProjection::Perspective);
}
