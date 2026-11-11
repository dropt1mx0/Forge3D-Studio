use egui::Ui;
use forge_scene::EditMode;

use crate::app::Forge3dStudioApp;

pub fn render_status_bar(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    let is_light = app.editor_state.panel_settings.is_light_theme;

    ui.horizontal(|ui| {
        // --- Bottom-left: Exit Button ---
        let exit_text = "⏻ Exit";
        let font_id = egui::FontId::proportional(11.5);
        let text_galley = ui.painter().layout_no_wrap(
            exit_text.to_string(),
            font_id.clone(),
            egui::Color32::WHITE,
        );
        let btn_size = text_galley.size() + egui::vec2(16.0, 6.0);
        let (exit_rect, exit_resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());

        let (exit_bg, exit_border, exit_fg) = if exit_resp.hovered() {
            if is_light {
                (
                    egui::Color32::from_rgb(254, 226, 226),
                    egui::Color32::from_rgb(239, 68, 68),
                    egui::Color32::from_rgb(220, 38, 38),
                )
            } else {
                (
                    egui::Color32::from_rgb(52, 22, 25),
                    egui::Color32::from_rgb(220, 38, 38),
                    egui::Color32::from_rgb(255, 120, 120),
                )
            }
        } else {
            if is_light {
                (
                    egui::Color32::from_rgb(235, 238, 246),
                    egui::Color32::from_rgb(212, 218, 230),
                    egui::Color32::from_rgb(180, 40, 40),
                )
            } else {
                (
                    egui::Color32::from_rgb(24, 26, 32),
                    egui::Color32::from_rgb(46, 50, 62),
                    egui::Color32::from_rgb(220, 100, 100),
                )
            }
        };

        ui.painter().rect(
            exit_rect,
            egui::CornerRadius::same(4),
            exit_bg,
            egui::Stroke::new(1.0, exit_border),
            egui::StrokeKind::Inside,
        );
        let text_pos = egui::pos2(
            exit_rect.center().x - text_galley.size().x / 2.0,
            exit_rect.center().y - text_galley.size().y / 2.0,
        );
        let galley = ui.painter().layout_no_wrap(exit_text.to_string(), font_id, exit_fg);
        ui.painter().galley(text_pos, galley, exit_fg);

        let mut action_project_hub = false;

        let exit_resp =
            exit_resp.on_hover_text("Close Forge3D Studio (Right-click: Return to Project Hub)");
        if exit_resp.clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        exit_resp.context_menu(|ui| {
            if ui.button("🏠 Project Hub...").clicked() {
                action_project_hub = true;
                ui.close();
            }
            ui.separator();
            if ui.button("⏻ Close Forge3D Studio").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close();
            }
        });

        if action_project_hub {
            app.is_in_startup_hub = true;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Status message and active mode on left
        let mode_tag = match app.editor_state.edit_mode {
            EditMode::Object => "[Object Mode]".to_string(),
            EditMode::Edit => format!("[Edit Mode: {:?}]", app.editor_state.selection_element),
            EditMode::Sculpt => "[Sculpt Mode]".to_string(),
        };
        ui.label(egui::RichText::new(mode_tag).strong());
        ui.label(&app.status_message);

        if app.editor_state.snap_settings.enabled {
            ui.separator();
            ui.label(format!(
                "Snap: {:.2}m",
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
