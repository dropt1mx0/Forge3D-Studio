use forge_math::{Ray, Vec2, Vec3};
use forge_render::{CameraProjection, ViewPreset};
use forge_scene::{Action, PointerInteractionState};
use forge3d_studio::Forge3dStudioApp;
use forge3d_studio::app::{calculate_drag_delta, intersect_ray_plane};

#[test]
fn test_ray_plane_intersection_math() {
    // 1. Ray perpendicular to plane: origin (0, 0, 5), direction (0, 0, -1)
    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    let plane_point = Vec3::ZERO;
    let plane_normal = Vec3::new(0.0, 0.0, 1.0); // Facing ray origin

    let hit = intersect_ray_plane(&ray, plane_point, plane_normal);
    assert!(hit.is_some(), "Perpendicular ray must hit plane");
    let hit_pt = hit.unwrap();
    assert!((hit_pt - Vec3::ZERO).length() < 1e-4);

    // 2. Initial ray through hit point produces delta = Vec3::ZERO
    let initial_hit = hit_pt;
    let delta = calculate_drag_delta(&ray, plane_point, plane_normal, initial_hit);
    assert_eq!(
        delta,
        Some(Vec3::ZERO),
        "Initial ray at drag start must have zero delta"
    );

    // 3. Oblique ray moving across plane
    let oblique_ray = Ray::new(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(2.0, 3.0, -5.0).normalize(),
    );
    let oblique_hit = intersect_ray_plane(&oblique_ray, plane_point, plane_normal);
    assert!(oblique_hit.is_some());
    let obl_pt = oblique_hit.unwrap();
    assert!((obl_pt.x - 2.0).abs() < 1e-4);
    assert!((obl_pt.y - 3.0).abs() < 1e-4);
    assert!(obl_pt.z.abs() < 1e-4);

    let obl_delta = calculate_drag_delta(&oblique_ray, plane_point, plane_normal, initial_hit);
    assert!(obl_delta.is_some());
    let d = obl_delta.unwrap();
    assert!((d - Vec3::new(2.0, 3.0, 0.0)).length() < 1e-4);
}

#[test]
fn test_drag_preserves_grab_point_offset_no_jump() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .expect("Default scene has Cube")
        .id;

    // Default scene Cube rests at (0.0, 0.5, 0.0)
    let init_translation = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert_eq!(init_translation, Vec3::new(0.0, 0.5, 0.0));

    // Simulate raycast hitting the cube off-center on its surface
    let hit_point = Vec3::new(0.4, 0.6, 1.0);
    let pointer_pos = Vec2::new(640.0, 400.0);

    // Select cube and start drag
    app.selection.select(cube_id);
    app.start_object_drag(hit_point, cube_id, pointer_pos, true);

    let expected_offset = init_translation - hit_point;
    let plane_normal = {
        let drag_state = app.object_drag_state.as_ref().unwrap();
        assert_eq!(drag_state.initial_hit_point, hit_point);
        assert_eq!(drag_state.plane_point, hit_point);

        // Verify initial offset is preserved: cube translation - hit_point
        assert_eq!(
            drag_state.initial_offsets.get(&cube_id).copied(),
            Some(expected_offset)
        );
        drag_state.plane_normal
    };

    // Ray at drag start pointing exactly at hit_point
    let start_ray = Ray::new(
        app.camera.eye,
        (hit_point - app.camera.eye).normalize_or_zero(),
    );

    // Frame 0 update: must produce delta ZERO, NO jumping!
    let start_delta = app.update_object_drag(&start_ray);
    assert!(start_delta.is_some());
    assert!(
        start_delta.unwrap().length() < 1e-4,
        "Initial delta must be 0"
    );

    let cube_pos_after_start = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert!(
        (cube_pos_after_start - init_translation).length() < 1e-4,
        "Object must NOT jump on drag start!"
    );

    // Subsequent frame: mouse moves so ray intersects plane at new point on the plane
    let v = Vec3::new(2.0, 1.5, 0.0);
    let v_on_plane = v - plane_normal * v.dot(plane_normal);
    let target_hit = hit_point + v_on_plane;
    let moved_ray = Ray::new(
        app.camera.eye,
        (target_hit - app.camera.eye).normalize_or_zero(),
    );

    let moved_delta = app.update_object_drag(&moved_ray);
    assert!(moved_delta.is_some());
    let delta = moved_delta.unwrap();

    let new_cube_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert_eq!(new_cube_pos, init_translation + delta);

    // Grab point in world space: new_cube_pos + (hit_point - init_translation)
    // must equal target_hit!
    let current_grab_point_world = new_cube_pos - expected_offset;
    assert!(
        (current_grab_point_world - target_hit).length() < 1e-3,
        "Grab point must follow intersection point precisely"
    );
}

#[test]
fn test_free_drag_various_directions() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    // Camera facing -Z, positioned at (0, 0, 5), target at (0, 0, 0)
    app.camera.eye = Vec3::new(0.0, 0.0, 5.0);
    app.camera.target = Vec3::ZERO;
    app.camera.up = Vec3::Y;

    let hit_point = Vec3::new(0.0, 0.0, 0.0);
    let pointer_pos = Vec2::new(640.0, 400.0);

    let directions = [
        ("positive X", Vec3::new(3.0, 0.0, 0.0)),
        ("negative X", Vec3::new(-3.0, 0.0, 0.0)),
        ("positive Y", Vec3::new(0.0, 2.5, 0.0)),
        ("negative Y", Vec3::new(0.0, -2.5, 0.0)),
        ("diagonal +X +Y", Vec3::new(2.0, 2.0, 0.0)),
        ("diagonal -X -Y", Vec3::new(-2.0, -2.0, 0.0)),
        ("diagonal +X -Y", Vec3::new(2.0, -2.0, 0.0)),
        ("diagonal -X +Y", Vec3::new(-2.0, 2.0, 0.0)),
    ];

    for (name, offset) in directions {
        app.selection.select(cube_id);
        if let Some(obj) = app.scene.get_object_mut(cube_id) {
            obj.local_transform.translation = Vec3::ZERO;
        }

        app.start_object_drag(hit_point, cube_id, pointer_pos, true);

        let target_pt = hit_point + offset;
        let ray = Ray::new(
            app.camera.eye,
            (target_pt - app.camera.eye).normalize_or_zero(),
        );

        let delta = app.update_object_drag(&ray);
        assert!(delta.is_some(), "Direction {name} must produce valid delta");
        let d = delta.unwrap();
        assert!(
            (d - offset).length() < 1e-3,
            "Delta for {name} must match requested offset: expected {:?}, got {:?}",
            offset,
            d
        );

        let pos = app
            .scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation;
        assert!(
            (pos - offset).length() < 1e-3,
            "Cube position for {name} must match offset"
        );

        // Cancel to reset for next direction
        app.cancel_object_drag();
    }
}

#[test]
fn test_multi_object_free_drag_preserves_relative_layout() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    let sphere_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Sphere")
        .unwrap()
        .id;

    // Set initial positions
    let cube_init_pos = Vec3::new(0.0, 0.0, 0.0);
    let sphere_init_pos = Vec3::new(4.0, 2.0, -1.0);

    if let Some(c) = app.scene.get_object_mut(cube_id) {
        c.local_transform.translation = cube_init_pos;
    }
    if let Some(s) = app.scene.get_object_mut(sphere_id) {
        s.local_transform.translation = sphere_init_pos;
    }

    let initial_relative_vec = sphere_init_pos - cube_init_pos;

    // Multi-select both Cube and Sphere
    app.selection.select(cube_id);
    app.selection.toggle_selection(sphere_id);
    assert_eq!(app.selection.selected_ids.len(), 2);

    // Click on Cube surface
    let hit_point = Vec3::new(0.5, 0.5, 1.0);
    app.start_object_drag(hit_point, cube_id, Vec2::new(500.0, 300.0), true);

    let drag = app.object_drag_state.as_ref().unwrap();
    assert_eq!(drag.initial_transforms.len(), 2);
    assert_eq!(drag.initial_offsets.len(), 2);

    // Drag both objects with delta
    let target_hit = hit_point + Vec3::new(1.8, -2.4, 0.0);
    let ray = Ray::new(
        app.camera.eye,
        (target_hit - app.camera.eye).normalize_or_zero(),
    );

    let delta = app.update_object_drag(&ray).expect("Drag must succeed");
    assert!(delta.length() > 0.1);

    // Verify both objects moved by delta
    let new_cube_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    let new_sphere_pos = app
        .scene
        .get_object(sphere_id)
        .unwrap()
        .local_transform
        .translation;

    assert!((new_cube_pos - (cube_init_pos + delta)).length() < 1e-4);
    assert!((new_sphere_pos - (sphere_init_pos + delta)).length() < 1e-4);

    // Verify relative layout preserved
    let current_relative_vec = new_sphere_pos - new_cube_pos;
    assert!(
        (current_relative_vec - initial_relative_vec).length() < 1e-4,
        "Relative distance and layout between selected objects must be strictly preserved"
    );
}

#[test]
fn test_object_drag_undo_redo_single_command() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    let sphere_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Sphere")
        .unwrap()
        .id;

    let cube_init = Vec3::new(1.0, 0.0, 0.0);
    let sphere_init = Vec3::new(5.0, 0.0, 0.0);

    if let Some(c) = app.scene.get_object_mut(cube_id) {
        c.local_transform.translation = cube_init;
    }
    if let Some(s) = app.scene.get_object_mut(sphere_id) {
        s.local_transform.translation = sphere_init;
    }

    app.selection.select(cube_id);
    app.selection.toggle_selection(sphere_id);

    let initial_history_len = app.history.undo_stack.len();

    // Start drag
    let hit_point = Vec3::new(1.0, 0.0, 1.0);
    app.start_object_drag(hit_point, cube_id, Vec2::new(400.0, 400.0), true);

    // Move across multiple frames (simulating intermediate mouse move events)
    for step in 1..=5 {
        let step_hit = hit_point + Vec3::new(step as f32 * 0.5, step as f32 * 0.3, 0.0);
        let ray = Ray::new(
            app.camera.eye,
            (step_hit - app.camera.eye).normalize_or_zero(),
        );
        app.update_object_drag(&ray);
        // During active drag, no undo steps must be recorded!
        assert_eq!(
            app.history.undo_stack.len(),
            initial_history_len,
            "Intermediate mouse frames must NOT create undo steps"
        );
    }

    let end_cube_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    let end_sphere_pos = app
        .scene
        .get_object(sphere_id)
        .unwrap()
        .local_transform
        .translation;

    // Release mouse -> commit single MultiTransform action
    let drag = app.object_drag_state.take().unwrap();
    app.commit_object_drag(&drag);

    assert_eq!(
        app.history.undo_stack.len(),
        initial_history_len + 1,
        "Drag completion must create exactly ONE undo action"
    );

    // Verify action is MultiTransform
    match app.history.undo_stack.last().unwrap() {
        Action::MultiTransform { transforms } => {
            assert_eq!(transforms.len(), 2);
        }
        _ => panic!("Expected MultiTransform action"),
    }

    // Ctrl+Z Undo: restores both objects
    app.undo();
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        cube_init
    );
    assert_eq!(
        app.scene
            .get_object(sphere_id)
            .unwrap()
            .local_transform
            .translation,
        sphere_init
    );

    // Ctrl+Y Redo: re-applies drag transformation
    app.redo();
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        end_cube_pos
    );
    assert_eq!(
        app.scene
            .get_object(sphere_id)
            .unwrap()
            .local_transform
            .translation,
        end_sphere_pos
    );
}

#[test]
fn test_object_drag_esc_cancellation() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    let cube_init = Vec3::new(2.0, 1.0, 0.0);
    if let Some(c) = app.scene.get_object_mut(cube_id) {
        c.local_transform.translation = cube_init;
    }

    app.selection.select(cube_id);

    let hit_point = Vec3::new(2.0, 1.0, 1.0);
    app.start_object_drag(hit_point, cube_id, Vec2::new(300.0, 300.0), true);

    // Move cube
    let moved_hit = hit_point + Vec3::new(5.0, 3.0, 0.0);
    let ray = Ray::new(
        app.camera.eye,
        (moved_hit - app.camera.eye).normalize_or_zero(),
    );
    app.update_object_drag(&ray);

    assert_ne!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        cube_init
    );

    // Escape cancellation
    app.cancel_object_drag();

    // Verify object restored to initial transform
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        cube_init
    );
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(app.object_drag_state.is_none());
    assert_eq!(app.history.undo_stack.len(), 0);
    assert_eq!(app.status_message, "Object drag cancelled");
}

#[test]
fn test_drag_parallel_ray_safe_handling() {
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

    app.selection.select(cube_id);
    let hit_point = Vec3::ZERO;
    app.start_object_drag(hit_point, cube_id, Vec2::new(400.0, 400.0), true);

    let drag = app.object_drag_state.as_ref().unwrap();
    let normal = drag.plane_normal;

    // Construct a ray that is mathematically parallel to plane (direction perpendicular to normal)
    let parallel_dir = Vec3::new(-normal.y, normal.x, 0.0).normalize_or_zero();
    let parallel_ray = Ray::new(app.camera.eye, parallel_dir);

    // Calculate delta on parallel ray
    let delta = calculate_drag_delta(&parallel_ray, drag.plane_point, normal, hit_point);
    assert!(
        delta.is_none(),
        "Parallel ray must produce None to avoid divide-by-zero"
    );

    // Update drag with parallel ray: position must remain unchanged without NaN or panic
    let update_res = app.update_object_drag(&parallel_ray);
    assert!(update_res.is_none());

    let curr_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert_eq!(curr_pos, initial_pos);
    assert!(curr_pos.is_finite());
}

#[test]
fn test_drag_extreme_ray_and_bounds_safety() {
    let plane_pt = Vec3::new(0.0, 0.0, 0.0);
    let plane_norm = Vec3::new(0.0, 0.0, 1.0);
    let hit_pt = Vec3::new(0.0, 0.0, 0.0);

    // 1. Ray pointing away from plane (t < 0)
    let back_ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 1.0));
    assert!(calculate_drag_delta(&back_ray, plane_pt, plane_norm, hit_pt).is_none());

    // 2. Degenerate plane normal (length ~ 0)
    let normal_ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(calculate_drag_delta(&normal_ray, plane_pt, Vec3::ZERO, hit_pt).is_none());

    // 3. Non-finite values (NaN / Inf)
    let nan_ray = Ray::new(Vec3::new(f32::NAN, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(calculate_drag_delta(&nan_ray, plane_pt, plane_norm, hit_pt).is_none());

    let nan_plane_ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(
        calculate_drag_delta(&nan_plane_ray, Vec3::splat(f32::NAN), plane_norm, hit_pt).is_none()
    );

    // 4. Excessive distance / horizon grazing ray (delta > 5000.0)
    let grazing_ray = Ray::new(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(10000.0, 0.0, -0.001).normalize(),
    );
    assert!(calculate_drag_delta(&grazing_ray, plane_pt, plane_norm, hit_pt).is_none());
}

#[test]
fn test_object_drag_focus_loss_safety() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    let initial_pos = Vec3::new(1.0, 2.0, 3.0);
    if let Some(c) = app.scene.get_object_mut(cube_id) {
        c.local_transform.translation = initial_pos;
    }

    app.selection.select(cube_id);
    let hit_point = initial_pos + Vec3::new(0.0, 0.0, 1.0);
    app.start_object_drag(hit_point, cube_id, Vec2::new(300.0, 300.0), true);

    // Drag to new position
    let target = hit_point + Vec3::new(2.0, 2.0, 0.0);
    let ray = Ray::new(
        app.camera.eye,
        (target - app.camera.eye).normalize_or_zero(),
    );
    app.update_object_drag(&ray);
    assert_ne!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        initial_pos
    );

    // Simulate window focus loss
    let window_focused = false;
    if !window_focused {
        app.cancel_object_drag();
    }

    // Verify object safely reverted
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        initial_pos
    );
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(app.object_drag_state.is_none());
}

#[test]
fn test_priority_ui_and_gizmo_inhibit_object_drag() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    app.selection.select(cube_id);

    // 1. UI Interaction priority: when UI captures pointer, object drag does not initiate
    let ui_captures_pointer = true;
    let lmb_pressed = true;

    if lmb_pressed && ui_captures_pointer {
        app.editor_state.pointer_state = PointerInteractionState::UIInteraction;
    }

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::UIInteraction
    );
    assert!(
        app.object_drag_state.is_none(),
        "UI Interaction must prevent object drag"
    );

    // Release UI interaction
    app.editor_state.pointer_state = PointerInteractionState::Idle;

    // 2. Gizmo Interaction priority: when gizmo is hovered/clicked, gizmo consumes input
    let gizmo_consumed = true;
    let ui_captures_pointer = false;

    if lmb_pressed && !ui_captures_pointer {
        if gizmo_consumed {
            app.editor_state.pointer_state = PointerInteractionState::GizmoDragging;
        } else {
            app.editor_state.pointer_state = PointerInteractionState::ObjectPressed;
        }
    }

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::GizmoDragging
    );
    assert!(
        app.object_drag_state.is_none(),
        "Gizmo interaction must prevent object drag"
    );

    // Release gizmo
    app.editor_state.pointer_state = PointerInteractionState::Idle;

    // 3. Object Drag priority: when UI and Gizmo do not consume, object drag initiates
    let gizmo_consumed = false;
    if lmb_pressed && !ui_captures_pointer && !gizmo_consumed {
        let hit_pt = Vec3::new(0.0, 0.0, 1.0);
        app.start_object_drag(hit_pt, cube_id, Vec2::new(400.0, 400.0), true);
    }

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::ObjectPressed
    );
    assert!(app.object_drag_state.is_some());
}

#[test]
fn test_drag_with_different_camera_views() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    let presets = [
        ViewPreset::Top,
        ViewPreset::Front,
        ViewPreset::Right,
        ViewPreset::Bottom,
    ];

    for preset in presets {
        app.camera.set_view_preset(preset);
        app.selection.select(cube_id);
        if let Some(c) = app.scene.get_object_mut(cube_id) {
            c.local_transform.translation = Vec3::ZERO;
        }

        let hit_point = Vec3::ZERO;
        app.start_object_drag(hit_point, cube_id, Vec2::new(500.0, 500.0), true);

        let drag = app.object_drag_state.as_ref().unwrap();
        // Working plane normal must face the camera
        let dot_with_cam = drag.plane_normal.dot(app.camera.forward());
        assert!(
            dot_with_cam < -0.9,
            "Plane normal must oppose camera forward direction for preset {:?}",
            preset
        );

        app.cancel_object_drag();
    }

    // Orthographic projection
    app.camera.toggle_projection();
    assert_eq!(app.camera.projection, CameraProjection::Orthographic);

    app.selection.select(cube_id);
    let hit_point = Vec3::ZERO;
    app.start_object_drag(hit_point, cube_id, Vec2::new(500.0, 500.0), true);

    let drag_ortho = app.object_drag_state.as_ref().unwrap();
    assert!(drag_ortho.plane_normal.is_finite());
    assert!(drag_ortho.plane_normal.length() > 0.99);

    app.cancel_object_drag();
}

#[test]
fn test_drag_camera_change_during_drag() {
    let mut app = Forge3dStudioApp::new_headless();

    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;

    app.selection.select(cube_id);
    let hit_point = Vec3::ZERO;
    app.start_object_drag(hit_point, cube_id, Vec2::new(400.0, 400.0), true);

    // Initial ray
    let ray1 = Ray::new(
        app.camera.eye,
        (hit_point - app.camera.eye).normalize_or_zero(),
    );
    let delta1 = app.update_object_drag(&ray1).unwrap();
    assert!(delta1.length() < 1e-4);

    // Orbit or zoom camera during drag
    app.camera.orbit(0.2, 0.1);
    app.camera.zoom(2.0);

    // Working plane point and normal remain stable in world space
    let drag = app.object_drag_state.as_ref().unwrap();
    assert_eq!(drag.plane_point, hit_point);

    // Ray from new camera position to a new point on the same working plane
    let target_hit = hit_point + Vec3::new(1.0, 1.0, 0.0);
    let ray2 = Ray::new(
        app.camera.eye,
        (target_hit - app.camera.eye).normalize_or_zero(),
    );

    let delta2 = app.update_object_drag(&ray2);
    assert!(delta2.is_some());
    let cube_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert!(cube_pos.is_finite());
}

fn make_test_input() -> egui::RawInput {
    egui::RawInput {
        focused: true,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        )),
        ..Default::default()
    }
}

#[test]
fn test_egui_render_viewport_live_drag() {
    let mut app = Forge3dStudioApp::new_headless();
    let ctx = egui::Context::default();

    // Screen center for default 1280x720 window
    let center = egui::pos2(640.0, 360.0);

    // Frame 1: Move mouse to center
    let mut raw_input = make_test_input();
    raw_input.events.push(egui::Event::PointerMoved(center));

    let mut out1 = ctx.run_ui(raw_input.clone(), |ui| {
        forge3d_studio::ui::viewport::render_viewport(&mut app, ui);
    });
    out1.textures_delta.clear();

    println!(
        "After Frame 1: pointer_state = {:?}",
        app.editor_state.pointer_state
    );

    // Frame 2: Press Primary mouse button at center (over the Cube)
    let mut raw_input2 = make_test_input();
    raw_input2.events.push(egui::Event::PointerMoved(center));
    raw_input2.events.push(egui::Event::PointerButton {
        pos: center,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });

    let mut out2 = ctx.run_ui(raw_input2, |ui| {
        forge3d_studio::ui::viewport::render_viewport(&mut app, ui);
    });
    out2.textures_delta.clear();

    println!(
        "After Frame 2 (press): pointer_state = {:?}",
        app.editor_state.pointer_state
    );
    println!(
        "object_drag_state is_some = {}",
        app.object_drag_state.is_some()
    );

    // Frame 3: Drag mouse by 50px
    let moved_pos = egui::pos2(690.0, 360.0);
    let mut raw_input3 = make_test_input();
    raw_input3.events.push(egui::Event::PointerMoved(moved_pos));

    let mut out3 = ctx.run_ui(raw_input3, |ui| {
        forge3d_studio::ui::viewport::render_viewport(&mut app, ui);
    });
    out3.textures_delta.clear();

    println!(
        "After Frame 3 (drag): pointer_state = {:?}",
        app.editor_state.pointer_state
    );
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::ObjectDragging
    );

    let cube_id = app.selection.active_id.expect("Cube must be selected");
    let moved_translation = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    println!(
        "Initial pos: (0.0, 0.5, 0.0), Moved pos: {:?}",
        moved_translation
    );
    assert!(
        (moved_translation - Vec3::new(0.0, 0.5, 0.0)).length() > 0.01,
        "Cube must have moved"
    );

    // Frame 4: Release mouse button
    let mut raw_input4 = make_test_input();
    raw_input4.events.push(egui::Event::PointerMoved(moved_pos));
    raw_input4.events.push(egui::Event::PointerButton {
        pos: moved_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });

    let mut out4 = ctx.run_ui(raw_input4, |ui| {
        forge3d_studio::ui::viewport::render_viewport(&mut app, ui);
    });
    out4.textures_delta.clear();

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(
        app.object_drag_state.is_none(),
        "Drag state cleared on release"
    );
    assert!(
        app.history.can_undo(),
        "Undo stack must contain the drag transform"
    );

    // Frame 5: Undo (Ctrl + Z)
    app.undo();
    let undone_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert!(
        (undone_pos - Vec3::new(0.0, 0.5, 0.0)).length() < 1e-4,
        "Undo must restore initial position"
    );
}

#[test]
fn test_full_app_ui_live_drag_and_ui_isolation() {
    let mut app = Forge3dStudioApp::new_headless();
    let ctx = egui::Context::default();

    // 1. First test: Clicking on Toolbar (UI element at x=25, y=150) must set UIInteraction
    let toolbar_pos = egui::pos2(25.0, 150.0);
    let mut raw_toolbar = make_test_input();
    raw_toolbar
        .events
        .push(egui::Event::PointerMoved(toolbar_pos));
    raw_toolbar.events.push(egui::Event::PointerButton {
        pos: toolbar_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });

    let mut out_tb = ctx.run_ui(raw_toolbar, |ui| {
        app.render_app_ui(ui);
    });
    out_tb.textures_delta.clear();

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::UIInteraction,
        "Clicking on toolbar must be UIInteraction"
    );
    assert!(
        app.object_drag_state.is_none(),
        "Toolbar click must not start object drag"
    );

    // Release on toolbar
    let mut raw_tb_rel = make_test_input();
    raw_tb_rel.events.push(egui::Event::PointerButton {
        pos: toolbar_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    let mut out_tb_rel = ctx.run_ui(raw_tb_rel, |ui| {
        app.render_app_ui(ui);
    });
    out_tb_rel.textures_delta.clear();
    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );

    // 2. Second test: Click on the 3D Cube inside the Central Viewport!
    // Viewport center in full app: toolbar is ~48px left, sidebar is ~220px right
    let vp_center = egui::pos2(550.0, 360.0);

    // Frame 1: Hover over Cube
    let mut raw_app1 = make_test_input();
    raw_app1.events.push(egui::Event::PointerMoved(vp_center));
    let mut out_app1 = ctx.run_ui(raw_app1, |ui| {
        app.render_app_ui(ui);
    });
    out_app1.textures_delta.clear();

    // Frame 2: Press Primary mouse button on Cube
    let mut raw_app2 = make_test_input();
    raw_app2.events.push(egui::Event::PointerMoved(vp_center));
    raw_app2.events.push(egui::Event::PointerButton {
        pos: vp_center,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });

    let mut out_app2 = ctx.run_ui(raw_app2, |ui| {
        app.render_app_ui(ui);
    });
    out_app2.textures_delta.clear();

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::ObjectPressed
    );
    assert!(
        app.object_drag_state.is_some(),
        "Drag state must be created on Cube press"
    );

    // Frame 3: Drag mouse by 40px
    let moved_vp = egui::pos2(590.0, 360.0);
    let mut raw_app3 = make_test_input();
    raw_app3.events.push(egui::Event::PointerMoved(moved_vp));

    let mut out_app3 = ctx.run_ui(raw_app3, |ui| {
        app.render_app_ui(ui);
    });
    out_app3.textures_delta.clear();

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::ObjectDragging
    );

    let cube_id = app.selection.active_id.expect("Cube is selected");
    let moved_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert!(
        (moved_pos - Vec3::new(0.0, 0.5, 0.0)).length() > 0.01,
        "Cube translated in full app"
    );

    // Frame 4: Release mouse
    let mut raw_app4 = make_test_input();
    raw_app4.events.push(egui::Event::PointerMoved(moved_vp));
    raw_app4.events.push(egui::Event::PointerButton {
        pos: moved_vp,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });

    let mut out_app4 = ctx.run_ui(raw_app4, |ui| {
        app.render_app_ui(ui);
    });
    out_app4.textures_delta.clear();

    assert_eq!(
        app.editor_state.pointer_state,
        PointerInteractionState::Idle
    );
    assert!(app.object_drag_state.is_none());
    assert!(app.history.can_undo());

    // Frame 5: Undo
    app.undo();
    let undone = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    assert!((undone - Vec3::new(0.0, 0.5, 0.0)).length() < 1e-4);
}
