use std::collections::HashSet;

use egui::Ui;
use forge_core::id::ObjectId;
use forge_scene::PrimitiveType;

use crate::app::Forge3dStudioApp;

/// Blender-style Outliner type filter modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutlinerTypeFilter {
    #[default]
    All,
    Meshes,
    Lights,
    Cameras,
}

/// Outliner search, filter, and column restriction visibility toggles.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinerFilterSettings {
    pub search_query: String,
    pub show_popover: bool,
    pub show_selectability_column: bool,
    pub show_visibility_column: bool,
    pub show_lock_column: bool,
    pub type_filter: OutlinerTypeFilter,
    pub only_selected: bool,
    pub only_visible: bool,
}

impl Default for OutlinerFilterSettings {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            show_popover: false,
            show_selectability_column: true,
            show_visibility_column: true,
            show_lock_column: true,
            type_filter: OutlinerTypeFilter::All,
            only_selected: false,
            only_visible: false,
        }
    }
}

pub fn render_outliner(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    // Top Tabs: File | Edit | View | Objects | Tags | Bookmarks
    egui::ScrollArea::horizontal()
        .id_salt("outliner_tabs_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 2.0);
                let tab_col = egui::Color32::from_rgb(160, 168, 185);
                ui.label(egui::RichText::new("File").size(11.0).color(tab_col));
                ui.label(egui::RichText::new("Edit").size(11.0).color(tab_col));
                ui.label(egui::RichText::new("View").size(11.0).color(tab_col));
                ui.label(
                    egui::RichText::new("Objects")
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::from_rgb(220, 230, 255)),
                );
                ui.label(egui::RichText::new("Tags").size(11.0).color(tab_col));
                ui.label(egui::RichText::new("Bookmarks").size(11.0).color(tab_col));
            });
        });
    ui.add_space(2.0);
    ui.separator();

    // Blender-style Search and Filter Bar
    let mut filter_button_rect = egui::Rect::NOTHING;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

        let search_width = (ui.available_width() - 80.0).max(80.0);
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut app.outliner_filter.search_query)
                .hint_text("🔍 Search...")
                .desired_width(search_width),
        );
        if search_response.on_hover_text("Search objects by name (case-insensitive)").has_focus() {
            ui.ctx().request_repaint();
        }

        if !app.outliner_filter.search_query.is_empty() {
            if ui.small_button("✕").on_hover_text("Clear search").clicked() {
                app.outliner_filter.search_query.clear();
            }
        }

        let is_filtered = !app.outliner_filter.search_query.is_empty()
            || app.outliner_filter.type_filter != OutlinerTypeFilter::All
            || app.outliner_filter.only_selected
            || app.outliner_filter.only_visible;

        let filter_btn = if is_filtered {
            egui::Button::new(
                egui::RichText::new("Filter ▾")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(130, 200, 255))
                    .strong(),
            )
            .fill(egui::Color32::from_rgb(30, 50, 80))
            .corner_radius(egui::CornerRadius::same(4))
        } else {
            egui::Button::new(egui::RichText::new("Filter ▾").size(11.0))
                .corner_radius(egui::CornerRadius::same(4))
        };

        let filter_resp = ui
            .add(filter_btn)
            .on_hover_text("Restriction Toggles & Filters (Blender-style)");
        filter_button_rect = filter_resp.rect;
        if filter_resp.clicked() {
            app.outliner_filter.show_popover = !app.outliner_filter.show_popover;
        }
    });

    // Render Filter Popover
    if app.outliner_filter.show_popover {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.outliner_filter.show_popover = false;
        }

        let popover_pos = filter_button_rect.left_bottom() + egui::vec2(-140.0, 4.0);
        egui::Area::new(ui.make_persistent_id("outliner_filter_popover"))
            .fixed_pos(popover_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(220.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Outliner Filters").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                app.outliner_filter.show_popover = false;
                            }
                        });
                    });
                    ui.separator();

                    ui.label(
                        egui::RichText::new("Restriction Toggles")
                            .color(egui::Color32::from_rgb(180, 195, 215))
                            .size(11.0)
                            .strong(),
                    );
                    ui.checkbox(
                        &mut app.outliner_filter.show_selectability_column,
                        "Selectable (↖)",
                    )
                    .on_hover_text("Toggle Selectable column visibility");
                    ui.checkbox(
                        &mut app.outliner_filter.show_visibility_column,
                        "Visibility (●)",
                    )
                    .on_hover_text("Toggle Viewport Visibility column");
                    ui.checkbox(
                        &mut app.outliner_filter.show_lock_column,
                        "Transform Lock ([-])",
                    )
                    .on_hover_text("Toggle Transform Lock column");

                    ui.separator();
                    ui.label(
                        egui::RichText::new("Filter by Type")
                            .color(egui::Color32::from_rgb(180, 195, 215))
                            .size(11.0)
                            .strong(),
                    );
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut app.outliner_filter.type_filter,
                            OutlinerTypeFilter::All,
                            "All",
                        );
                        ui.selectable_value(
                            &mut app.outliner_filter.type_filter,
                            OutlinerTypeFilter::Meshes,
                            "Mesh",
                        );
                        ui.selectable_value(
                            &mut app.outliner_filter.type_filter,
                            OutlinerTypeFilter::Lights,
                            "Light",
                        );
                        ui.selectable_value(
                            &mut app.outliner_filter.type_filter,
                            OutlinerTypeFilter::Cameras,
                            "Cam",
                        );
                    });

                    ui.separator();
                    ui.label(
                        egui::RichText::new("Display Options")
                            .color(egui::Color32::from_rgb(180, 195, 215))
                            .size(11.0)
                            .strong(),
                    );
                    ui.checkbox(&mut app.outliner_filter.only_selected, "Only Selected");
                    ui.checkbox(&mut app.outliner_filter.only_visible, "Only Visible");

                    ui.separator();
                    ui.label(
                        egui::RichText::new("Bulk Actions")
                            .color(egui::Color32::from_rgb(180, 195, 215))
                            .size(11.0)
                            .strong(),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .button("All Selectable")
                            .on_hover_text("Make all objects selectable in viewport")
                            .clicked()
                        {
                            app.make_all_objects_selectable(true);
                        }
                        if ui
                            .button("All Unselectable")
                            .on_hover_text("Make all objects unselectable in viewport")
                            .clicked()
                        {
                            app.make_all_objects_selectable(false);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .button("Show All")
                            .on_hover_text("Make all objects visible")
                            .clicked()
                        {
                            app.make_all_objects_visible(true);
                        }
                        if ui
                            .button("Reset Filters")
                            .on_hover_text("Clear search query and reset type filters")
                            .clicked()
                        {
                            app.outliner_filter.search_query.clear();
                            app.outliner_filter.type_filter = OutlinerTypeFilter::All;
                            app.outliner_filter.only_selected = false;
                            app.outliner_filter.only_visible = false;
                        }
                    });
                });
            });
    }

    ui.add_space(2.0);
    ui.separator();

    let mut to_delete: Option<ObjectId> = None;
    let mut to_select: Option<ObjectId> = None;
    let mut to_toggle_select: Option<ObjectId> = None;
    let mut to_activate_cam: Option<ObjectId> = None;
    let mut to_frame: Option<ObjectId> = None;

    egui::ScrollArea::vertical()
        .id_salt("outliner_tree_scroll")
        .show(ui, |ui| {
            if app.scene.objects.is_empty() {
                ui.label(
                    egui::RichText::new("Empty scene. Add objects from Create menu or toolbar.")
                        .weak()
                        .italics(),
                );
                return;
            }

            // 1. Render root objects (objects without parents) matching the filter
            let root_ids: Vec<ObjectId> = app
                .scene
                .objects
                .values()
                .filter(|o| o.parent.is_none())
                .map(|o| o.id)
                .filter(|&id| app.outliner_subtree_has_match(id))
                .collect();

            // 2. Render remaining orphaned objects matching the filter
            let mut visited: HashSet<ObjectId> = HashSet::new();
            let remaining_ids: Vec<ObjectId> = app
                .scene
                .objects
                .keys()
                .filter(|id| !visited.contains(id))
                .copied()
                .filter(|&id| app.outliner_subtree_has_match(id))
                .collect();

            if root_ids.is_empty() && remaining_ids.is_empty() {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("No objects match current filter.")
                            .weak()
                            .italics(),
                    );
                    if ui.button("Clear Filter").clicked() {
                        app.outliner_filter.search_query.clear();
                        app.outliner_filter.type_filter = OutlinerTypeFilter::All;
                        app.outliner_filter.only_selected = false;
                        app.outliner_filter.only_visible = false;
                    }
                });
                return;
            }

            for root_id in root_ids {
                render_object_node(
                    app,
                    ui,
                    root_id,
                    0,
                    &mut visited,
                    &mut to_select,
                    &mut to_toggle_select,
                    &mut to_delete,
                    &mut to_activate_cam,
                    &mut to_frame,
                );
            }

            for orphan_id in remaining_ids {
                render_object_node(
                    app,
                    ui,
                    orphan_id,
                    0,
                    &mut visited,
                    &mut to_select,
                    &mut to_toggle_select,
                    &mut to_delete,
                    &mut to_activate_cam,
                    &mut to_frame,
                );
            }
        });

    if let Some(id) = to_activate_cam {
        app.scene.set_active_camera(id);
        app.status_message = format!(
            "Set active camera: {}",
            app.scene.get_object(id).map_or("Camera", |o| &o.name)
        );
    }

    if let Some(id) = to_toggle_select {
        app.selection.toggle_selection(id);
    } else if let Some(id) = to_select {
        app.selection.select(id);
        if let Some(obj) = app.scene.get_object(id) {
            app.status_message = format!("Selected {}", obj.name);
        }
    }

    if to_frame.is_some() {
        app.frame_selected();
    }

    if let Some(id) = to_delete {
        app.delete_selected();
        if app.scene.objects.contains_key(&id) {
            app.scene.remove_object(id);
        }
        if app.selection.is_selected(id) {
            app.selection.deselect();
        }
        app.status_message = "Removed object".to_string();
    }
}

#[allow(clippy::too_many_arguments)]
fn render_object_node(
    app: &mut Forge3dStudioApp,
    ui: &mut Ui,
    id: ObjectId,
    depth: usize,
    visited: &mut HashSet<ObjectId>,
    to_select: &mut Option<ObjectId>,
    to_toggle_select: &mut Option<ObjectId>,
    to_delete: &mut Option<ObjectId>,
    to_activate_cam: &mut Option<ObjectId>,
    to_frame: &mut Option<ObjectId>,
) {
    if visited.contains(&id) {
        return;
    }
    visited.insert(id);

    let is_selected = app.selection.is_selected(id);
    let is_active_cam = app.scene.active_camera_id == Some(id);
    let children: Vec<ObjectId>;

    if let Some(obj) = app.scene.objects.get_mut(&id) {
        children = obj.children.clone();

        ui.horizontal(|ui| {
            // Indentation for hierarchy
            if depth > 0 {
                ui.add_space((depth as f32) * 14.0);
                ui.label("└");
            }

            // Visibility toggle icon (if column enabled)
            if app.outliner_filter.show_visibility_column {
                let eye_icon = if obj.visible { "●" } else { "○" };
                if ui
                    .small_button(eye_icon)
                    .on_hover_text("Toggle Visibility (H)")
                    .clicked()
                {
                    obj.visible = !obj.visible;
                }
            }

            // Selectability toggle icon (Blender-style cursor arrow, if column enabled)
            if app.outliner_filter.show_selectability_column {
                let sel_icon = if obj.selectable { "↖" } else { "⊘" };
                if ui
                    .small_button(sel_icon)
                    .on_hover_text("Toggle Viewport Selectability (Ignore in Box Select / Click)")
                    .clicked()
                {
                    obj.selectable = !obj.selectable;
                }
            }

            // Lock toggle icon (if column enabled)
            if app.outliner_filter.show_lock_column {
                let lock_icon = if obj.locked { "[L]" } else { "[-]" };
                if ui
                    .small_button(lock_icon)
                    .on_hover_text("Toggle Transform Lock")
                    .clicked()
                {
                    obj.locked = !obj.locked;
                }
            }

            // Primitive type badge
            let tag = match obj.primitive_type {
                PrimitiveType::Cube => "[Cube]",
                PrimitiveType::Sphere | PrimitiveType::Icosphere => "[Sphere]",
                PrimitiveType::Plane | PrimitiveType::Grid => "[Plane]",
                PrimitiveType::Circle => "[Circle]",
                PrimitiveType::Cylinder => "[Cyl]",
                PrimitiveType::Cone => "[Cone]",
                PrimitiveType::Torus => "[Torus]",
                PrimitiveType::Capsule => "[Capsule]",
                PrimitiveType::Monkey => "[Suzanne]",
                PrimitiveType::Camera => "[Cam]",
                PrimitiveType::Light => "[Light]",
                PrimitiveType::Custom => "[Mesh]",
            };

            let mut label_text = format!("{tag} {}", obj.name);
            if obj.primitive_type == PrimitiveType::Camera && is_active_cam {
                label_text.push_str(" [Active]");
            }

            let has_children = !children.is_empty();
            let expand_icon = if has_children { "▾ " } else { "  " };

            let text_color = if is_selected {
                egui::Color32::WHITE
            } else {
                ui.visuals().text_color()
            };

            let row_btn = if is_selected {
                egui::Button::new(
                    egui::RichText::new(format!("{expand_icon}{label_text}"))
                        .color(text_color)
                        .strong(),
                )
                .fill(egui::Color32::from_rgb(37, 99, 235))
                .corner_radius(egui::CornerRadius::same(5))
            } else {
                egui::Button::new(
                    egui::RichText::new(format!("{expand_icon}{label_text}")).color(text_color),
                )
                .fill(egui::Color32::TRANSPARENT)
                .corner_radius(egui::CornerRadius::same(5))
            };

            let row = ui.add(row_btn);

            if row.clicked() {
                if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                    *to_toggle_select = Some(id);
                } else {
                    *to_select = Some(id);
                }
            }

            if row.double_clicked() {
                *to_select = Some(id);
                *to_frame = Some(id);
            }

            // Camera activate button
            if obj.primitive_type == PrimitiveType::Camera
                && !is_active_cam
                && ui
                    .small_button("Set Active")
                    .on_hover_text("Set as active scene camera")
                    .clicked()
            {
                *to_activate_cam = Some(id);
            }

            // Quick delete button on right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("✕")
                    .on_hover_text("Delete object (X / Del)")
                    .clicked()
                {
                    *to_delete = Some(id);
                }
            });
        });
    } else {
        return;
    }

    // Render children recursively (only those matching the filter)
    for child_id in children {
        if app.outliner_subtree_has_match(child_id) {
            render_object_node(
                app,
                ui,
                child_id,
                depth + 1,
                visited,
                to_select,
                to_toggle_select,
                to_delete,
                to_activate_cam,
                to_frame,
            );
        }
    }
}
