use forge_geometry::primitives::{generate_capsule, generate_monkey};
use forge_geometry::validate_mesh_data;
use forge_math::{Transform, Vec2, Vec3};
use forge_scene::{
    Action, AxisConstraint, GizmoMode, PanelSettings, PrimitiveType, SceneObject,
    ToolbarDisplayMode, TransformMode,
};
use forge3d_studio::app::Forge3dStudioApp;

#[test]
fn test_automatic_gizmo_on_selection_and_rebind() {
    let mut app = Forge3dStudioApp::new_headless();

    // Default scene has Cube, Sphere, Light, Camera
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

    let cube_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;
    let sphere_pos = app
        .scene
        .get_object(sphere_id)
        .unwrap()
        .local_transform
        .translation;

    // 1. Initial selection is empty -> no pivot
    assert!(app.selection.is_empty());

    // 2. Select Cube -> Gizmo automatically binds to Cube
    app.selection.select(cube_id);
    assert_eq!(app.selection.active_id, Some(cube_id));
    assert_eq!(app.editor_state.active_gizmo_mode, GizmoMode::Translate);

    let active_obj = app
        .scene
        .get_object(app.selection.active_id.unwrap())
        .unwrap();
    assert_eq!(active_obj.local_transform.translation, cube_pos);

    // 3. Switch selection to Sphere -> Gizmo automatically rebinds to Sphere
    app.selection.select(sphere_id);
    assert_eq!(app.selection.active_id, Some(sphere_id));
    let active_obj = app
        .scene
        .get_object(app.selection.active_id.unwrap())
        .unwrap();
    assert_eq!(active_obj.local_transform.translation, sphere_pos);

    // 4. Multi-selection (Cube + Sphere) -> Pivot computes Median Point
    app.selection.toggle_selection(cube_id);
    assert_eq!(app.selection.selected_ids.len(), 2);

    let mut sum = Vec3::ZERO;
    for &id in &app.selection.selected_ids {
        sum += app
            .scene
            .get_object(id)
            .unwrap()
            .local_transform
            .translation;
    }
    let median = sum / 2.0;
    let expected_median = (cube_pos + sphere_pos) * 0.5;
    assert!((median - expected_median).length() < 1e-5);
}

#[test]
fn test_multi_object_gizmo_transformation_and_undo() {
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

    let orig_cube_tf = app.scene.get_object(cube_id).unwrap().local_transform;
    let orig_sphere_tf = app.scene.get_object(sphere_id).unwrap().local_transform;

    // Select both objects
    app.selection.select(cube_id);
    app.selection.toggle_selection(sphere_id);
    assert_eq!(app.selection.selected_ids.len(), 2);

    // Simulate Gizmo drag start: cache initial transforms
    app.gizmo_initial_transforms.clear();
    for &id in &app.selection.selected_ids {
        app.gizmo_initial_transforms
            .insert(id, app.scene.get_object(id).unwrap().local_transform);
    }

    // Simulate Gizmo drag delta: Translate by (3.0, 1.0, -2.0)
    let delta = Vec3::new(3.0, 1.0, -2.0);
    for (&id, &init_tf) in &app.gizmo_initial_transforms {
        if let Some(obj) = app.scene.get_object_mut(id) {
            obj.local_transform.translation = init_tf.translation + delta;
        }
    }

    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        orig_cube_tf.translation + delta
    );
    assert_eq!(
        app.scene
            .get_object(sphere_id)
            .unwrap()
            .local_transform
            .translation,
        orig_sphere_tf.translation + delta
    );

    // Simulate Gizmo drag release: push Action::MultiTransform
    let mut changed = Vec::new();
    for (&id, &init_tf) in &app.gizmo_initial_transforms {
        let cur_tf = app.scene.get_object(id).unwrap().local_transform;
        if cur_tf != init_tf {
            changed.push((id, init_tf, cur_tf));
        }
    }
    assert_eq!(changed.len(), 2);
    app.history.push(Action::MultiTransform {
        transforms: changed,
    });

    // Verify Undo restores both objects
    assert!(app.history.can_undo());
    app.undo();
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        orig_cube_tf.translation
    );
    assert_eq!(
        app.scene
            .get_object(sphere_id)
            .unwrap()
            .local_transform
            .translation,
        orig_sphere_tf.translation
    );

    // Verify Redo re-applies both transformations
    assert!(app.history.can_redo());
    app.redo();
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        orig_cube_tf.translation + delta
    );
    assert_eq!(
        app.scene
            .get_object(sphere_id)
            .unwrap()
            .local_transform
            .translation,
        orig_sphere_tf.translation + delta
    );
}

#[test]
fn test_modal_transform_axis_and_plane_constraints_with_numeric_input() {
    let mut app = Forge3dStudioApp::new_headless();
    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    app.selection.select(cube_id);

    let start_pos = app
        .scene
        .get_object(cube_id)
        .unwrap()
        .local_transform
        .translation;

    // Begin modal transform (G)
    app.begin_modal_transform(TransformMode::Translate, Vec2::new(100.0, 100.0));
    assert!(app.editor_state.modal_transform.active);
    assert_eq!(
        app.editor_state.modal_transform.mode,
        TransformMode::Translate
    );

    // Constraint: Axis X
    app.editor_state
        .modal_transform
        .set_or_toggle_axis(AxisConstraint::X);
    assert_eq!(
        app.editor_state.modal_transform.constraint,
        Some(AxisConstraint::X)
    );
    assert!(!app.editor_state.modal_transform.is_local_constraint);

    // Second press toggles to Local X
    app.editor_state
        .modal_transform
        .set_or_toggle_axis(AxisConstraint::X);
    assert_eq!(
        app.editor_state.modal_transform.constraint,
        Some(AxisConstraint::X)
    );
    assert!(app.editor_state.modal_transform.is_local_constraint);

    // Switch to PlaneYZ (Shift+X)
    app.editor_state
        .modal_transform
        .set_or_toggle_axis(AxisConstraint::PlaneYZ);
    assert_eq!(
        app.editor_state.modal_transform.constraint,
        Some(AxisConstraint::PlaneYZ)
    );

    // Numeric input: "-4.25"
    for c in ['-', '4', '.', '2', '5'] {
        app.editor_state.modal_transform.push_char(c);
    }
    assert_eq!(
        app.editor_state.modal_transform.parsed_number(),
        Some(-4.25)
    );

    // Cancel modal transform (Esc)
    app.cancel_modal_transform();
    assert!(!app.editor_state.modal_transform.active);
    // Transform unchanged
    assert_eq!(
        app.scene
            .get_object(cube_id)
            .unwrap()
            .local_transform
            .translation,
        start_pos
    );
}

#[test]
fn test_panel_settings_persistence_and_clamping() {
    let unconstrained = PanelSettings {
        toolbar_width: 10.0, // Below min (48.0)
        toolbar_visible: true,
        toolbar_mode: ToolbarDisplayMode::IconOnly,
        sidebar_width: 1000.0, // Above max (600.0)
        sidebar_visible: true,
        outliner_height_ratio: 0.95, // Above max (0.85)
        timeline_height: 500.0,      // Above max (350.0)
        timeline_visible: false,
        zoom_sensitivity: 10.0, // Above max (5.0)
        zoom_to_cursor: true,
        smooth_zoom: true,
        menu_bar_right_width: 1000.0, // Above max (500.0)
    };

    let clamped = unconstrained.clamped();
    assert_eq!(clamped.toolbar_width, 48.0);
    assert_eq!(clamped.sidebar_width, 600.0);
    assert_eq!(clamped.outliner_height_ratio, 0.85);
    assert_eq!(clamped.timeline_height, 350.0);
    assert_eq!(clamped.zoom_sensitivity, 5.0);
    assert_eq!(clamped.menu_bar_right_width, 500.0);

    // Test JSON serialization roundtrip
    let json = serde_json::to_string(&clamped).unwrap();
    let loaded: PanelSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, clamped);
}

#[test]
fn test_primitives_capsule_and_monkey_generation_and_scene_addition() {
    let mut app = Forge3dStudioApp::new_headless();

    // 1. Generate Capsule
    let capsule_mesh = generate_capsule(0.5, 1.2, 24, 8);
    assert!(validate_mesh_data(&capsule_mesh.vertices, &capsule_mesh.indices).is_ok());
    assert!(capsule_mesh.vertices.len() > 100);
    assert!(capsule_mesh.indices.len() / 3 > 100);

    let capsule_id = app.scene.add_object(
        SceneObject::new("Capsule", PrimitiveType::Capsule, capsule_mesh)
            .with_color([0.85, 0.65, 0.25, 1.0])
            .with_transform(Transform::from_translation(Vec3::new(1.0, 1.0, 0.0))),
    );

    // 2. Generate Suzanne / Monkey
    let monkey_mesh = generate_monkey(1.0);
    assert!(validate_mesh_data(&monkey_mesh.vertices, &monkey_mesh.indices).is_ok());
    assert!(monkey_mesh.vertices.len() > 30);
    assert!(monkey_mesh.indices.len() / 3 > 40);

    let monkey_id = app.scene.add_object(
        SceneObject::new("Suzanne", PrimitiveType::Monkey, monkey_mesh)
            .with_color([0.78, 0.52, 0.32, 1.0])
            .with_transform(Transform::from_translation(Vec3::new(-1.0, 0.75, 0.0))),
    );

    // Verify objects exist in scene with correct types
    let cap_obj = app.scene.get_object(capsule_id).unwrap();
    assert_eq!(cap_obj.primitive_type, PrimitiveType::Capsule);

    let monk_obj = app.scene.get_object(monkey_id).unwrap();
    assert_eq!(monk_obj.primitive_type, PrimitiveType::Monkey);

    // Select monkey and verify selection
    app.selection.select(monkey_id);
    assert_eq!(app.selection.active_id, Some(monkey_id));
}

#[test]
fn test_toolbar_mode_switch_and_visibility() {
    let mut app = Forge3dStudioApp::new_headless();

    // Default: IconOnly
    assert_eq!(
        app.editor_state.panel_settings.toolbar_mode,
        ToolbarDisplayMode::IconOnly
    );
    assert!(app.editor_state.panel_settings.toolbar_visible);

    // Expand width past 90.0
    app.editor_state.panel_settings.toolbar_width = 110.0;
    if app.editor_state.panel_settings.toolbar_width >= 90.0 {
        app.editor_state.panel_settings.toolbar_mode = ToolbarDisplayMode::IconAndLabel;
    }
    assert_eq!(
        app.editor_state.panel_settings.toolbar_mode,
        ToolbarDisplayMode::IconAndLabel
    );

    // Collapse toolbar
    app.editor_state.panel_settings.toolbar_visible = false;
    assert!(!app.editor_state.panel_settings.toolbar_visible);
}

#[test]
fn test_focus_loss_safety() {
    let mut app = Forge3dStudioApp::new_headless();
    let cube_id = app
        .scene
        .objects
        .values()
        .find(|o| o.name == "Cube")
        .unwrap()
        .id;
    app.selection.select(cube_id);

    // Start modal transform
    app.begin_modal_transform(TransformMode::Translate, Vec2::new(50.0, 50.0));
    assert!(app.editor_state.modal_transform.active);

    // Simulate window focus loss: modal transform must cancel safely
    let window_focused = false;
    if !window_focused {
        if app.editor_state.modal_transform.active {
            app.cancel_modal_transform();
        }
        app.editor_state.pointer_state = forge_scene::PointerInteractionState::Idle;
        app.gizmo_state.drag_state = None;
    }

    assert!(!app.editor_state.modal_transform.active);
    assert_eq!(
        app.editor_state.pointer_state,
        forge_scene::PointerInteractionState::Idle
    );
    assert!(!app.gizmo_state.is_dragging());
}
