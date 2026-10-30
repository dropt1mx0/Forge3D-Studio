use egui::Ui;
use forge_core::id::ObjectId;
use forge_scene::PrimitiveType;

use crate::app::Forge3dStudioApp;

pub fn render_outliner(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.heading("Scene Outliner");
    ui.separator();

    let mut to_delete: Option<ObjectId> = None;
    let mut to_select: Option<ObjectId> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        let object_ids: Vec<ObjectId> = app.scene.objects.keys().copied().collect();

        if object_ids.is_empty() {
            ui.label("Empty scene. Add objects from the toolbar.");
            return;
        }

        for id in object_ids {
            if let Some(obj) = app.scene.objects.get_mut(&id) {
                let is_selected = app.selection.is_selected(id);

                ui.horizontal(|ui| {
                    // Visibility toggle icon
                    let eye_icon = if obj.visible { "👁" } else { "🚫" };
                    if ui.small_button(eye_icon).clicked() {
                        obj.visible = !obj.visible;
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
                        PrimitiveType::Custom => "📐",
                    };

                    let label_text = format!("{icon} {}", obj.name);
                    let row = ui.selectable_label(is_selected, label_text);

                    if row.clicked() {
                        to_select = Some(id);
                    }

                    // Quick delete button on right
                    if ui
                        .small_button("✕")
                        .on_hover_text("Delete object")
                        .clicked()
                    {
                        to_delete = Some(id);
                    }
                });
            }
        }
    });

    if let Some(id) = to_select {
        app.selection.select(id);
        if let Some(obj) = app.scene.get_object(id) {
            app.status_message = format!("Selected {}", obj.name);
        }
    }

    if let Some(id) = to_delete {
        app.scene.remove_object(id);
        if app.selection.is_selected(id) {
            app.selection.deselect();
        }
        app.status_message = "Removed object".to_string();
    }
}
