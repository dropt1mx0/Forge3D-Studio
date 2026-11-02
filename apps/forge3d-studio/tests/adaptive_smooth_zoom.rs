use eframe::egui::{self, Pos2, Rect, Vec2 as EguiVec2};
use forge_math::{Vec2, Vec3};
use forge_render::camera::{BASE_ZOOM_SENSITIVITY, MAX_CAMERA_DISTANCE, MIN_CAMERA_DISTANCE};
use forge3d_studio::app::Forge3dStudioApp;

fn make_test_input() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            Pos2::ZERO,
            EguiVec2::new(1280.0, 800.0),
        )),
        ..Default::default()
    }
}

#[test]
fn test_small_and_fast_scroll_delta() {
    let mut app = Forge3dStudioApp::new_headless();
    app.camera.settings.smooth_zoom = false;

    let d_start = app.camera.distance();

    // Small scroll in (+5.0)
    app.camera.zoom_adaptive(5.0, None);
    let d_small = app.camera.distance();
    assert!(d_small < d_start);
    let small_ratio = d_small / d_start;
    assert!((small_ratio - (-5.0 * BASE_ZOOM_SENSITIVITY).exp()).abs() < 1e-4);

    // Fast scroll in (+150.0)
    app.camera.zoom_adaptive(150.0, None);
    let d_fast = app.camera.distance();
    assert!(d_fast < d_small);
    let fast_ratio = d_fast / d_small;
    assert!(fast_ratio < 0.85);

    // Zoom back out (-155.0)
    app.camera.zoom_adaptive(-155.0, None);
    let d_out = app.camera.distance();
    assert!(d_out > d_fast);
    assert!((d_out - d_start).abs() < 0.1);
}

#[test]
fn test_close_and_far_distance_bounds() {
    let mut app = Forge3dStudioApp::new_headless();
    app.camera.settings.smooth_zoom = false;

    // Zoom in from standard distance until clamped
    for _ in 0..150 {
        app.camera.zoom_adaptive(200.0, None);
    }
    assert!(
        app.camera.distance() >= MIN_CAMERA_DISTANCE - 1e-4,
        "Distance must not penetrate below MIN_CAMERA_DISTANCE: {}",
        app.camera.distance()
    );

    // Zoom out until clamped
    for _ in 0..200 {
        app.camera.zoom_adaptive(-200.0, None);
    }
    assert!(
        app.camera.distance() <= MAX_CAMERA_DISTANCE + 1e-4,
        "Distance must clamp at MAX_CAMERA_DISTANCE: {}",
        app.camera.distance()
    );
}

#[test]
fn test_zoom_to_cursor_geometry_hit() {
    let mut app = Forge3dStudioApp::new_headless();
    app.camera.settings.smooth_zoom = false;
    app.camera.settings.zoom_to_cursor = true;

    // Raycast towards default Cube at origin
    let ray = app
        .camera
        .screen_to_ray(Vec2::new(640.0, 400.0), Vec2::new(1280.0, 800.0));
    let hit = app.scene.raycast(&ray);
    assert!(
        hit.is_some(),
        "Center ray must hit the default scene object"
    );
    let (_id, dist) = hit.unwrap();
    let hit_point = ray.point_at(dist);

    let ray_dir_before = (hit_point - app.camera.eye).normalize();
    let eye_dist_before = (hit_point - app.camera.eye).length();

    app.camera.zoom_adaptive(45.0, Some(hit_point));

    let ray_dir_after = (hit_point - app.camera.eye).normalize();
    let eye_dist_after = (hit_point - app.camera.eye).length();

    // Collinear ray invariant
    assert!(
        (ray_dir_after - ray_dir_before).length() < 1e-4,
        "Look direction towards cursor hit point must remain collinear"
    );
    assert!(eye_dist_after < eye_dist_before);
    assert!(app.camera.target.is_finite());
}

#[test]
fn test_zoom_to_cursor_empty_scene_focal_plane_fallback() {
    let mut app = Forge3dStudioApp::new_headless();
    app.camera.settings.smooth_zoom = false;
    app.camera.settings.zoom_to_cursor = true;

    // Intentionally cast a ray pointing away from objects (top-right corner of viewport)
    let corner_ray = app
        .camera
        .screen_to_ray(Vec2::new(1200.0, 50.0), Vec2::new(1280.0, 800.0));
    assert!(
        app.scene.raycast(&corner_ray).is_none(),
        "Corner ray in empty space must miss scene objects"
    );

    // Fallback: intersect with focus plane
    let plane_point = app.camera.target;
    let plane_normal = -app.camera.forward();
    let focal_hit =
        forge3d_studio::app::intersect_ray_plane(&corner_ray, plane_point, plane_normal);
    assert!(
        focal_hit.is_some(),
        "Focus plane intersection must succeed for valid camera rays"
    );

    let hit = focal_hit.unwrap();
    let dist_before = app.camera.distance();
    app.camera.zoom_adaptive(30.0, Some(hit));
    let dist_after = app.camera.distance();

    assert!(dist_after < dist_before);
    assert!(app.camera.eye.is_finite());
    assert!(app.camera.target.is_finite());
}

#[test]
fn test_different_camera_angles_and_projections() {
    let mut app = Forge3dStudioApp::new_headless();
    app.camera.settings.smooth_zoom = false;

    // 1. Perspective at top preset
    app.camera
        .set_view_preset(forge_render::camera::ViewPreset::Top);
    let d0 = app.camera.distance();
    app.camera.zoom_adaptive(25.0, None);
    assert!(app.camera.distance() < d0);
    assert!(app.camera.eye.is_finite());

    // 2. Perspective at front preset
    app.camera
        .set_view_preset(forge_render::camera::ViewPreset::Front);
    let d1 = app.camera.distance();
    app.camera.zoom_adaptive(-25.0, None);
    assert!(app.camera.distance() > d1);
    assert!(app.camera.eye.is_finite());

    // 3. Orthographic mode
    app.camera.toggle_projection();
    assert_eq!(
        app.camera.projection,
        forge_render::camera::CameraProjection::Orthographic
    );
    let ortho_before = app.camera.ortho_size;
    app.camera.zoom_adaptive(30.0, None);
    assert!(app.camera.ortho_size < ortho_before);
    app.camera.zoom_adaptive(-30.0, None);
    assert!((app.camera.ortho_size - ortho_before).abs() < 0.05);
}

#[test]
fn test_nan_infinity_robustness() {
    let mut app = Forge3dStudioApp::new_headless();
    let original_eye = app.camera.eye;
    let original_target = app.camera.target;

    app.camera.zoom_adaptive(f32::NAN, None);
    assert_eq!(app.camera.eye, original_eye);
    assert_eq!(app.camera.target, original_target);

    app.camera.zoom_adaptive(f32::INFINITY, None);
    assert_eq!(app.camera.eye, original_eye);

    app.camera.zoom_adaptive(f32::NEG_INFINITY, None);
    assert_eq!(app.camera.eye, original_eye);

    app.camera
        .zoom_adaptive(20.0, Some(Vec3::new(f32::NAN, f32::NAN, f32::NAN)));
    assert!(app.camera.eye.is_finite());
    assert!(app.camera.target.is_finite());
}

#[test]
fn test_navigation_compatibility_and_smooth_zoom() {
    let mut app = Forge3dStudioApp::new_headless();
    app.camera.settings.smooth_zoom = true;

    // Start a smooth zoom
    app.camera.zoom_adaptive(50.0, None);
    assert!(app.camera.is_smooth_zooming);

    // User starts orbit: must cancel smooth zoom immediately without fighting
    app.camera.orbit(0.1, 0.05);
    assert!(!app.camera.is_smooth_zooming);

    // Start another smooth zoom
    app.camera.zoom_adaptive(50.0, None);
    assert!(app.camera.is_smooth_zooming);

    // User pans: must cancel smooth zoom
    app.camera.pan(10.0, -5.0);
    assert!(!app.camera.is_smooth_zooming);

    // User dollies: must cancel smooth zoom
    app.camera.zoom_adaptive(50.0, None);
    assert!(app.camera.is_smooth_zooming);
    app.camera.dolly(1.0);
    assert!(!app.camera.is_smooth_zooming);
}

#[test]
fn test_settings_serialization_roundtrip() {
    let settings = forge_scene::PanelSettings {
        zoom_sensitivity: 2.4,
        zoom_to_cursor: false,
        smooth_zoom: false,
        ..Default::default()
    };

    let json = serde_json::to_string(&settings).expect("Serialization must succeed");
    let deserialized: forge_scene::PanelSettings =
        serde_json::from_str(&json).expect("Deserialization must succeed");

    assert_eq!(deserialized.zoom_sensitivity, 2.4);
    assert!(!deserialized.zoom_to_cursor);
    assert!(!deserialized.smooth_zoom);

    // Verify clamping resilience
    let clamped_extreme = forge_scene::PanelSettings {
        zoom_sensitivity: 100.0,
        ..settings
    }
    .clamped();
    assert_eq!(clamped_extreme.zoom_sensitivity, 5.0);
}

#[test]
fn test_live_egui_viewport_smooth_scroll_wheel() {
    let mut app = Forge3dStudioApp::new_headless();
    let ctx = egui::Context::default();

    let initial_dist = app.camera.distance();

    // Frame 1: Mouse over central viewport, scroll wheel rolled forward (+30.0)
    let mut input = make_test_input();
    input
        .events
        .push(egui::Event::PointerMoved(Pos2::new(600.0, 400.0)));
    input.events.push(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 30.0),
        modifiers: egui::Modifiers::default(),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    });

    let mut out = ctx.run_ui(input, |ui| {
        app.render_app_ui(ui);
    });
    out.textures_delta.clear();

    // Verify smooth zoom is triggered
    assert!(app.camera.is_smooth_zooming);

    // Run 15 consecutive frames to let smooth zoom converge
    for _ in 0..15 {
        let input = make_test_input();
        let mut out = ctx.run_ui(input, |ui| {
            app.render_app_ui(ui);
        });
        out.textures_delta.clear();
    }

    assert!(
        app.camera.distance() < initial_dist,
        "Distance after smooth zoom must be smaller than initial"
    );
}
