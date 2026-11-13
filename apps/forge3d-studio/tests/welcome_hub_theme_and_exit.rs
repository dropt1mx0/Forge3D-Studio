use eframe::egui;
use forge3d_studio::app::Forge3dStudioApp;

#[test]
fn test_welcome_hub_theme_and_exit() {
    let mut app = Forge3dStudioApp::new_headless();
    app.is_in_startup_hub = true;
    assert!(app.is_in_startup_hub, "App is in startup hub");

    let ctx = egui::Context::default();
    let initial_theme = app.editor_state.panel_settings.is_light_theme;

    // 1. Initial UI pass in startup hub
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_app_ui(ui);
    });
    out.textures_delta.clear();

    // 2. Toggle Theme
    app.toggle_theme(&ctx);
    assert_eq!(
        app.editor_state.panel_settings.is_light_theme, !initial_theme,
        "Theme flag must be flipped after toggle_theme"
    );

    // 3. UI pass in toggled theme
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_app_ui(ui);
    });
    out.textures_delta.clear();

    // 4. Toggle back
    app.toggle_theme(&ctx);
    assert_eq!(
        app.editor_state.panel_settings.is_light_theme, initial_theme,
        "Theme flag must flip back to original"
    );

    // 5. Transition to active editor and verify rendering in both themes
    app.start_new_scene();
    assert!(!app.is_in_startup_hub, "Should now be in editor");

    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_app_ui(ui);
    });
    out.textures_delta.clear();

    app.toggle_theme(&ctx);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_app_ui(ui);
    });
    out.textures_delta.clear();
}
