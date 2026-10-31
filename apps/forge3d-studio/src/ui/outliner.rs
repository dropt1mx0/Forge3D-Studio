use std::collections::HashSet;

use egui::Ui;
use forge_core::id::ObjectId;
use forge_scene::PrimitiveType;

use crate::app::Forge3dStudioApp;

pub fn render_outliner(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.heading("Scene Outliner");
    ui.separator();

    let mut to_delete: Option<ObjectId> = None;
    let mut to_select: Option<ObjectId> = None;
    let mut to_toggle_select: Option<ObjectId> = None;
    let mut to_activate_cam: Option<ObjectId> = None;
    let mut to_frame: Option<ObjectId> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        if app.scene.objects.is_empty() {
            ui.label("Empty scene. Add objects from the toolbar or Add menu.");
            return;
        }

        let mut visited: HashSet<ObjectId> = HashSet::new();

        // 1. Render root objects (objects without parents)
        let root_ids: Vec<ObjectId> = app
            .scene
            .objects
            .values()
            .filter(|o| o.parent.is_none())
            .map(|o| o.id)
            .collect();

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

        // 2. Render any remaining orphaned objects (e.g. invalid parent reference)
        let remaining_ids: Vec<ObjectId> = app
            .scene
            .objects
            .keys()
            .filter(|id| !visited.contains(id))
            .copied()
            .collect();

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

            // Visibility toggle icon
            let eye_icon = if obj.visible { "👁" } else { "🚫" };
            if ui
                .small_button(eye_icon)
                .on_hover_text("Toggle Visibility (H)")
                .clicked()
            {
                obj.visible = !obj.visible;
            }

            // Lock toggle icon
            let lock_icon = if obj.locked { "🔒" } else { "🔓" };
            if ui
                .small_button(lock_icon)
                .on_hover_text("Toggle Transform Lock")
                .clicked()
            {
                obj.locked = !obj.locked;
            }

            // Primitive icon
            let icon = match obj.primitive_type {
                PrimitiveType::Cube => "📦",
                PrimitiveType::Sphere | PrimitiveType::Icosphere => "🌐",
                PrimitiveType::Plane | PrimitiveType::Grid => "⬛",
                PrimitiveType::Circle => "⭕",
                PrimitiveType::Cylinder => "🛢",
                PrimitiveType::Cone => "▲",
                PrimitiveType::Torus => "🍩",
                PrimitiveType::Capsule => "💊",
                PrimitiveType::Monkey => "🐵",
                PrimitiveType::Camera => "📷",
                PrimitiveType::Custom => "📐",
            };

            let mut label_text = format!("{icon} {}", obj.name);
            if obj.primitive_type == PrimitiveType::Camera && is_active_cam {
                label_text.push_str(" ★");
            }

            let row = ui.selectable_label(is_selected, label_text);

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

    // Render children recursively
    for child_id in children {
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
