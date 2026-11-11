use eframe::egui;
use crate::app::Forge3dStudioApp;

fn format_relative_time(timestamp: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if timestamp == 0 || now < timestamp {
        return "recently".to_string();
    }

    let diff = now - timestamp;
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} min ago", diff / 60)
    } else if diff < 86400 {
        format!("{} h ago", diff / 3600)
    } else if diff < 604800 {
        format!("{} d ago", diff / 86400)
    } else {
        "over a week ago".to_string()
    }
}

fn pill_button(ui: &mut egui::Ui, text: &str, is_light: bool) -> egui::Response {
    let padding = egui::vec2(20.0, 9.0);
    let font_id = egui::FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id.clone(), egui::Color32::WHITE);
    let desired_size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    let (bg_color, stroke_color, text_color) = if response.hovered() {
        if is_light {
            (
                egui::Color32::from_rgb(235, 240, 252),
                egui::Color32::from_rgb(255, 140, 0),
                egui::Color32::from_rgb(15, 20, 30),
            )
        } else {
            (
                egui::Color32::from_rgb(32, 35, 46),
                egui::Color32::from_rgb(255, 140, 0),
                egui::Color32::from_rgb(255, 255, 255),
            )
        }
    } else {
        if is_light {
            (
                egui::Color32::from_rgb(240, 243, 250),
                egui::Color32::from_rgb(205, 212, 226),
                egui::Color32::from_rgb(45, 50, 65),
            )
        } else {
            (
                egui::Color32::from_rgb(20, 22, 28),
                egui::Color32::from_rgb(46, 50, 62),
                egui::Color32::from_rgb(180, 185, 196),
            )
        }
    };

    ui.painter().rect(
        rect,
        egui::CornerRadius::same(18),
        bg_color,
        egui::Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );

    let text_pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, text_color);
    ui.painter().galley(text_pos, galley, text_color);

    response
}

/// Renders the clean, modern Startup Hub matching the minimalist reference design.
pub fn render_welcome_hub(app: &mut Forge3dStudioApp, ui: &mut egui::Ui) {
    let mut action_new_scene = false;
    let mut action_open_forge = false;
    let mut action_import = false;
    let mut action_showcase = false;
    let mut action_exit = false;
    let mut action_toggle_theme = false;
    let mut project_to_open: Option<String> = None;
    let mut project_to_remove: Option<String> = None;

    let is_light = app.editor_state.panel_settings.is_light_theme;

    // Bottom Bar: Exit button in the bottom-left corner and Theme toggle in the bottom-right corner
    egui::Panel::bottom("welcome_bottom_bar")
        .frame(
            egui::Frame::new()
                .fill(if is_light {
                    egui::Color32::from_rgb(240, 242, 248)
                } else {
                    egui::Color32::from_rgb(12, 13, 17)
                })
                .stroke(egui::Stroke::new(
                    1.0,
                    if is_light {
                        egui::Color32::from_rgb(220, 224, 235)
                    } else {
                        egui::Color32::from_rgb(26, 28, 36)
                    },
                ))
                .inner_margin(egui::Margin::symmetric(18, 10)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // --- Bottom-left: Exit Button ---
                let exit_text = "⏻  Exit";
                let font_id = egui::FontId::proportional(12.0);
                let text_galley = ui.painter().layout_no_wrap(
                    exit_text.to_string(),
                    font_id.clone(),
                    egui::Color32::WHITE,
                );
                let btn_size = text_galley.size() + egui::vec2(24.0, 10.0);
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
                            egui::Color32::from_rgb(232, 235, 244),
                            egui::Color32::from_rgb(212, 218, 230),
                            egui::Color32::from_rgb(160, 40, 40),
                        )
                    } else {
                        (
                            egui::Color32::from_rgb(20, 22, 28),
                            egui::Color32::from_rgb(42, 46, 56),
                            egui::Color32::from_rgb(210, 90, 90),
                        )
                    }
                };

                ui.painter().rect(
                    exit_rect,
                    egui::CornerRadius::same(6),
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

                if exit_resp.on_hover_text("Close Forge3D Studio").clicked() {
                    action_exit = true;
                }

                // --- Bottom-right: Theme Switcher ---
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let full_theme_text = if is_light {
                        "🌙  Dark Theme"
                    } else {
                        "☀️  Light Theme"
                    };
                    let font_id = egui::FontId::proportional(12.0);
                    let theme_galley = ui.painter().layout_no_wrap(
                        full_theme_text.to_string(),
                        font_id.clone(),
                        egui::Color32::WHITE,
                    );
                    let btn_size = theme_galley.size() + egui::vec2(22.0, 10.0);
                    let (theme_rect, theme_resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());

                    let (theme_bg, theme_border, theme_fg) = if theme_resp.hovered() {
                        if is_light {
                            (
                                egui::Color32::from_rgb(230, 235, 248),
                                egui::Color32::from_rgb(37, 99, 235),
                                egui::Color32::from_rgb(20, 25, 40),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(34, 38, 50),
                                egui::Color32::from_rgb(255, 140, 0),
                                egui::Color32::WHITE,
                            )
                        }
                    } else {
                        if is_light {
                            (
                                egui::Color32::from_rgb(235, 238, 246),
                                egui::Color32::from_rgb(208, 215, 228),
                                egui::Color32::from_rgb(45, 52, 68),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(20, 22, 28),
                                egui::Color32::from_rgb(42, 46, 58),
                                egui::Color32::from_rgb(180, 185, 196),
                            )
                        }
                    };

                    ui.painter().rect(
                        theme_rect,
                        egui::CornerRadius::same(14),
                        theme_bg,
                        egui::Stroke::new(1.0, theme_border),
                        egui::StrokeKind::Inside,
                    );
                    let text_pos = egui::pos2(
                        theme_rect.center().x - theme_galley.size().x / 2.0,
                        theme_rect.center().y - theme_galley.size().y / 2.0,
                    );
                    let galley = ui.painter().layout_no_wrap(full_theme_text.to_string(), font_id, theme_fg);
                    ui.painter().galley(text_pos, galley, theme_fg);

                    if theme_resp
                        .on_hover_text("Toggle between Light and Dark interface theme")
                        .clicked()
                    {
                        action_toggle_theme = true;
                    }
                });
            });
        });

    // Central Area: Clean Minimalist Layout
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(if is_light {
            egui::Color32::from_rgb(248, 249, 252)
        } else {
            egui::Color32::from_rgb(15, 16, 21)
        }))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                // Vertical top spacing: centers the content elegantly on screen
                let total_h = ui.available_height();
                let top_spacing = (total_h * 0.16).clamp(20.0, 120.0);
                ui.add_space(top_spacing);

                // --- Mascot with perfectly centered, continuous Gaussian bloom ---
                let mascot_size = egui::vec2(110.0, 80.3);
                let (rect, _response) = ui.allocate_exact_size(mascot_size, egui::Sense::hover());

                // The creature's body center is at 36% horizontal, 58% vertical of mascot rect
                let glow_center = egui::pos2(
                    rect.left() + rect.width() * 0.36,
                    rect.top() + rect.height() * 0.58,
                );

                // Render continuous, hardware-filtered smooth radial bloom (no hard steps or rings)
                if let Some(glow_tex) = &app.glow_texture {
                    let glow_size = egui::vec2(220.0, 220.0);
                    let glow_rect = egui::Rect::from_center_size(glow_center, glow_size);
                    egui::Image::new((glow_tex.id(), glow_size))
                        .paint_at(ui, glow_rect);
                }

                if let Some(mascot_tex) = &app.mascot_texture {
                    egui::Image::new((mascot_tex.id(), mascot_size))
                        .paint_at(ui, rect);
                }

                // Generous, breathable spacing between mascot and first text
                ui.add_space(38.0);

                // --- Title ---
                ui.label(
                    egui::RichText::new("Forge3D Studio")
                        .size(26.0)
                        .strong()
                        .color(if is_light {
                            egui::Color32::from_rgb(25, 28, 36)
                        } else {
                            egui::Color32::from_rgb(240, 242, 246)
                        }),
                );

                ui.add_space(8.0);

                // --- Subtitle ---
                ui.label(
                    egui::RichText::new("High-performance Rust 3D engine and modeling studio")
                        .size(13.0)
                        .color(if is_light {
                            egui::Color32::from_rgb(95, 102, 118)
                        } else {
                            egui::Color32::from_rgb(138, 143, 155)
                        }),
                );

                ui.add_space(8.0);

                // --- Working directory context ---
                let cwd = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "Forge3D-Rust-Edition".to_string());

                let display_cwd = if cwd.len() > 64 {
                    format!("...{}", &cwd[cwd.len() - 60..])
                } else {
                    cwd
                };

                ui.label(
                    egui::RichText::new(format!("{display_cwd} is active"))
                        .monospace()
                        .size(11.0)
                        .color(if is_light {
                            egui::Color32::from_rgb(130, 138, 155)
                        } else {
                            egui::Color32::from_rgb(90, 95, 108)
                        }),
                );

                ui.add_space(28.0);

                // --- Action Pill Buttons ---
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                    let row_w = 480.0;
                    let pad = ((ui.available_width() - row_w) / 2.0).max(0.0);
                    ui.add_space(pad);

                    if pill_button(ui, "New Scene", is_light).clicked() {
                        action_new_scene = true;
                    }
                    if pill_button(ui, "Open Project", is_light).clicked() {
                        action_open_forge = true;
                    }
                    if pill_button(ui, "Import 3D", is_light).clicked() {
                        action_import = true;
                    }
                    if pill_button(ui, "Showcase Villa", is_light).clicked() {
                        action_showcase = true;
                    }
                });

                // --- Recent Projects List ---
                let recent_projects = &app.editor_state.panel_settings.recent_projects;
                if !recent_projects.is_empty() {
                    ui.add_space(34.0);

                    ui.label(
                        egui::RichText::new("RECENT PROJECTS")
                            .size(10.5)
                            .strong()
                            .color(if is_light {
                                egui::Color32::from_rgb(115, 122, 138)
                            } else {
                                egui::Color32::from_rgb(105, 110, 122)
                            }),
                    );

                    ui.add_space(10.0);

                    let list_width = 520.0;
                    let pad = ((ui.available_width() - list_width) / 2.0).max(0.0);

                    ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("welcome_recents_scroll")
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for item in recent_projects.iter().take(6) {
                                        let item_path = item.path.clone();
                                        let (row_rect, row_resp) = ui.allocate_exact_size(
                                             egui::vec2(list_width, 36.0),
                                            egui::Sense::click(),
                                        );

                                        let is_hovered = row_resp.hovered();
                                        let bg = if is_hovered {
                                            if is_light {
                                                egui::Color32::from_rgb(234, 238, 248)
                                            } else {
                                                egui::Color32::from_rgb(26, 28, 36)
                                            }
                                        } else {
                                            if is_light {
                                                egui::Color32::from_rgb(242, 245, 251)
                                            } else {
                                                egui::Color32::from_rgb(19, 21, 27)
                                            }
                                        };
                                        let border = if is_hovered {
                                            if is_light {
                                                egui::Color32::from_rgb(190, 200, 220)
                                            } else {
                                                egui::Color32::from_rgb(60, 65, 80)
                                            }
                                        } else {
                                            if is_light {
                                                egui::Color32::from_rgb(220, 225, 238)
                                            } else {
                                                egui::Color32::from_rgb(32, 35, 44)
                                            }
                                        };

                                        ui.painter().rect(
                                            row_rect,
                                            egui::CornerRadius::same(6),
                                            bg,
                                            egui::Stroke::new(1.0, border),
                                            egui::StrokeKind::Inside,
                                        );

                                        // Left: project name
                                        let name_pos = row_rect.left_center() + egui::vec2(14.0, -7.0);
                                        let name_galley = ui.painter().layout_no_wrap(
                                            item.name.clone(),
                                            egui::FontId::proportional(12.5),
                                            if is_light {
                                                egui::Color32::from_rgb(25, 28, 36)
                                            } else {
                                                egui::Color32::from_rgb(230, 232, 238)
                                            },
                                        );
                                        ui.painter().galley(
                                            name_pos,
                                            name_galley,
                                            if is_light {
                                                egui::Color32::from_rgb(25, 28, 36)
                                            } else {
                                                egui::Color32::WHITE
                                            },
                                        );

                                        // Right: relative time
                                        let time_str = format_relative_time(item.timestamp);
                                        let time_galley = ui.painter().layout_no_wrap(
                                            time_str,
                                            egui::FontId::proportional(11.0),
                                            if is_light {
                                                egui::Color32::from_rgb(115, 122, 138)
                                            } else {
                                                egui::Color32::from_rgb(120, 125, 138)
                                            },
                                        );
                                        let time_pos = egui::pos2(
                                            row_rect.right() - time_galley.size().x - 36.0,
                                            row_rect.center().y - time_galley.size().y / 2.0,
                                        );
                                        ui.painter().galley(
                                            time_pos,
                                            time_galley,
                                            if is_light {
                                                egui::Color32::from_rgb(115, 122, 138)
                                            } else {
                                                egui::Color32::from_rgb(120, 125, 138)
                                            },
                                        );

                                        // Remove button (✕)
                                        let remove_rect = egui::Rect::from_center_size(
                                            egui::pos2(row_rect.right() - 18.0, row_rect.center().y),
                                            egui::vec2(18.0, 18.0),
                                        );
                                        let remove_resp = ui.allocate_rect(remove_rect, egui::Sense::click());
                                        if remove_resp.hovered() {
                                            ui.painter().rect_filled(
                                                remove_rect,
                                                egui::CornerRadius::same(4),
                                                if is_light {
                                                    egui::Color32::from_rgb(220, 225, 236)
                                                } else {
                                                    egui::Color32::from_rgb(45, 48, 60)
                                                },
                                            );
                                        }
                                        let remove_galley = ui.painter().layout_no_wrap(
                                            "✕".to_string(),
                                            egui::FontId::proportional(11.0),
                                            if remove_resp.hovered() {
                                                if is_light {
                                                    egui::Color32::from_rgb(15, 20, 30)
                                                } else {
                                                    egui::Color32::WHITE
                                                }
                                            } else {
                                                if is_light {
                                                    egui::Color32::from_rgb(130, 136, 150)
                                                } else {
                                                    egui::Color32::from_gray(120)
                                                }
                                            },
                                        );
                                        let remove_text_pos = remove_rect.center() - remove_galley.size() / 2.0;
                                        ui.painter().galley(
                                            remove_text_pos,
                                            remove_galley,
                                            if is_light {
                                                egui::Color32::from_rgb(15, 20, 30)
                                            } else {
                                                egui::Color32::WHITE
                                            },
                                        );

                                        if remove_resp.clicked() {
                                            project_to_remove = Some(item_path.clone());
                                        } else if row_resp.clicked() {
                                            project_to_open = Some(item_path);
                                        }

                                        ui.add_space(4.0);
                                    }
                                });
                        });
                    });
                }
            });
        });

    if action_exit {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }

    if action_toggle_theme {
        app.toggle_theme(ui.ctx());
    }

    if let Some(path) = project_to_remove {
        app.editor_state.panel_settings.remove_recent_project(&path);
    }

    if let Some(path_str) = project_to_open {
        let path = std::path::PathBuf::from(&path_str);
        if let Err(e) = app.open_or_import_file(&path) {
            app.status_message = format!("Error opening file: {e}");
        } else {
            app.is_in_startup_hub = false;
        }
    }

    if action_new_scene {
        app.start_new_scene();
    }

    if action_showcase {
        app.load_showcase_and_start();
    }

    if action_open_forge {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Forge3D Project", &["forge"])
            .set_title("Open Forge3D Project")
            .pick_file()
        {
            if let Err(e) = app.open_project_from_path(&path) {
                app.status_message = format!("Error opening project: {e}");
            } else {
                app.is_in_startup_hub = false;
            }
        }
    }

    if action_import {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("3D Models (OBJ, glTF, STL, PLY)", &["obj", "gltf", "glb", "stl", "ply"])
            .set_title("Import 3D Model")
            .pick_file()
        {
            if let Err(e) = app.open_or_import_file(&path) {
                app.status_message = format!("Error importing file: {e}");
            } else {
                app.is_in_startup_hub = false;
            }
        }
    }
}
