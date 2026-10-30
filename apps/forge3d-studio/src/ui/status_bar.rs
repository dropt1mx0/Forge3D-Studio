use egui::Ui;

use crate::app::Forge3dStudioApp;

pub fn render_status_bar(app: &Forge3dStudioApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        // Status message on left
        ui.label(format!("Status: {}", app.status_message));

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
