use egui::Ui;
use forge_scene::EditMode;

use crate::app::Forge3dStudioApp;

pub fn render_status_bar(app: &Forge3dStudioApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        // Status message and active mode on left
        let mode_tag = match app.editor_state.edit_mode {
            EditMode::Object => "[Object Mode]".to_string(),
            EditMode::Edit => format!("[Edit Mode: {:?}]", app.editor_state.selection_element),
        };
        ui.label(egui::RichText::new(mode_tag).strong());
        ui.label(&app.status_message);

        if app.editor_state.snap_settings.enabled {
            ui.separator();
            ui.label(format!(
                "🧲 Snap: {:.2}m",
                app.editor_state.snap_settings.grid_increment
            ));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Right side stats:
            // 1. GPU Adapter
            if let Some(gpu) = &app.gpu_info {
                ui.label(format!("GPU: {} ({})", gpu.name, gpu.backend));
            } else {
                ui.label("GPU: Initializing...");
            }

            ui.separator();

            // 2. Performance (FPS & Frame time)
            ui.label(format!("{:.1} FPS ({:.1} ms)", app.fps, app.frame_time_ms));

            ui.separator();

            // 3. Geometry metrics
            let total_verts: usize = app
                .scene
                .objects
                .values()
                .map(|o| o.mesh.vertices.len())
                .sum();
            let total_tris: usize = app
                .scene
                .objects
                .values()
                .map(|o| o.mesh.indices.len() / 3)
                .sum();
            ui.label(format!("Tris: {total_tris} | Verts: {total_verts}"));
        });
    });
}
