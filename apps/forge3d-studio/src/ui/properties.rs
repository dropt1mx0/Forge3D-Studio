use egui::{DragValue, Slider, Ui};
use forge_core::id::ObjectId;
use forge_modifiers::{
    ArrayModifier, BevelModifier, DecimateModifier, DisplaceModifier, MirrorModifier, ModifierType,
    SolidifyModifier, SubdivisionModifier, WeightedNormalModifier,
};
use forge_scene::{
    CameraData, CameraProjectionType, EditMode, PrimitiveType, TransformOrientation,
};

use crate::app::Forge3dStudioApp;

pub fn render_properties(app: &mut Forge3dStudioApp, ui: &mut Ui) {
    ui.heading("Properties");
    ui.separator();

    if let Some(selected_id) = app.selection.active_id {
        let mut apply_tf = false;
        let mut reset_tf = false;
        let mut copy_tf = false;
        let mut paste_tf = false;
        let mut set_origin_geo = false;
        let mut set_origin_cursor = false;
        let mut set_active_camera: Option<ObjectId> = None;
        let mut view_camera: Option<ObjectId> = None;
        let mut align_camera: Option<ObjectId> = None;
        let mut open_shader_editor = false;

        let mut add_modifier_type = None;
        let mut remove_mod_idx = None;
        let mut duplicate_mod_idx = None;
        let mut apply_mod_idx = None;
        let mut apply_all_mods = false;
        let mut clear_all_mods = false;
        let mut move_up_idx = None;
        let mut move_down_idx = None;

        let is_active_cam = app.scene.active_camera_id == Some(selected_id);

        if let Some(obj) = app.scene.get_object_mut(selected_id) {
            // Object Name & Visibility & Lock
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut obj.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let sel_label = if obj.selectable {
                        "[Selectable]"
                    } else {
                        "[Unselectable]"
                    };
                    if ui
                        .selectable_label(obj.selectable, sel_label)
                        .on_hover_text(
                            "Toggle Viewport Selectability (Exclude from Box Select / Click)",
                        )
                        .clicked()
                    {
                        obj.selectable = !obj.selectable;
                    }

                    let lock_icon = if obj.locked { "[Locked]" } else { "[Unlocked]" };
                    if ui
                        .selectable_label(obj.locked, lock_icon)
                        .on_hover_text("Lock Object Transform")
                        .clicked()
                    {
                        obj.locked = !obj.locked;
                    }

                    let eye_icon = if obj.visible { "Show" } else { "Hide" };
                    if ui
                        .selectable_label(obj.visible, eye_icon)
                        .on_hover_text("Toggle Visibility (H)")
                        .clicked()
                    {
                        obj.visible = !obj.visible;
                    }
                });
            });

            ui.add_space(6.0);
            ui.separator();

            // Top Tab Pills: [ Object Name ]  Basic  Coord  Object
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                let _ = ui.add(
                    egui::Button::new(
                        egui::RichText::new(&obj.name)
                            .strong()
                            .size(11.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(74, 137, 220))
                    .corner_radius(egui::CornerRadius::same(2)),
                );
                let _ = ui.small_button("Basic");
                let _ = ui.add(
                    egui::Button::new("Coord")
                        .fill(egui::Color32::from_rgb(55, 55, 55))
                        .corner_radius(egui::CornerRadius::same(2)),
                );
                let _ = ui.small_button("Object");
            });

            ui.add_space(4.0);

            let base_size = obj.mesh.aabb.size();
            let mut current_dim = base_size * obj.local_transform.scale.abs();
            let mut rot_deg = obj.local_transform.euler_angles_deg();
            let mut rot_changed = false;
            let mut dim_changed = false;

            // 3-Column Studio Coordinate Card (Position | Size | Rotation)
            egui::Frame::canvas(ui.style())
                .fill(egui::Color32::from_rgb(35, 35, 35))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 58)))
                .corner_radius(egui::CornerRadius::same(3))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.columns(3, |cols| {
                        // Column 0: Position
                        cols[0].vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                            ui.label(
                                egui::RichText::new("Position")
                                    .strong()
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(190, 190, 190)),
                            );
                            ui.add_space(2.0);

                            // Row X
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("X")
                                        .color(egui::Color32::from_rgb(239, 68, 68))
                                        .strong(),
                                );
                                ui.add(
                                    DragValue::new(&mut obj.local_transform.translation.x)
                                        .speed(0.05)
                                        .suffix(" m"),
                                );
                            });

                            // Row Y
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Y")
                                        .color(egui::Color32::from_rgb(34, 197, 94))
                                        .strong(),
                                );
                                ui.add(
                                    DragValue::new(&mut obj.local_transform.translation.y)
                                        .speed(0.05)
                                        .suffix(" m"),
                                );
                            });

                            // Row Z
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Z")
                                        .color(egui::Color32::from_rgb(59, 130, 246))
                                        .strong(),
                                );
                                ui.add(
                                    DragValue::new(&mut obj.local_transform.translation.z)
                                        .speed(0.05)
                                        .suffix(" m"),
                                );
                            });
                        });

                        // Column 1: Size
                        cols[1].vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                            ui.label(
                                egui::RichText::new("Size")
                                    .strong()
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(190, 190, 190)),
                            );
                            ui.add_space(2.0);

                            // Row X
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("X")
                                        .color(egui::Color32::from_rgb(239, 68, 68))
                                        .strong(),
                                );
                                if ui
                                    .add(
                                        DragValue::new(&mut current_dim.x)
                                            .speed(0.05)
                                            .range(0.001..=1000.0)
                                            .suffix(" m"),
                                    )
                                    .changed()
                                {
                                    dim_changed = true;
                                }
                            });

                            // Row Y
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Y")
                                        .color(egui::Color32::from_rgb(34, 197, 94))
                                        .strong(),
                                );
                                if ui
                                    .add(
                                        DragValue::new(&mut current_dim.y)
                                            .speed(0.05)
                                            .range(0.001..=1000.0)
                                            .suffix(" m"),
                                    )
                                    .changed()
                                {
                                    dim_changed = true;
                                }
                            });

                            // Row Z
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Z")
                                        .color(egui::Color32::from_rgb(59, 130, 246))
                                        .strong(),
                                );
                                if ui
                                    .add(
                                        DragValue::new(&mut current_dim.z)
                                            .speed(0.05)
                                            .range(0.001..=1000.0)
                                            .suffix(" m"),
                                    )
                                    .changed()
                                {
                                    dim_changed = true;
                                }
                            });
                        });

                        // Column 2: Rotation
                        cols[2].vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                            ui.label(
                                egui::RichText::new("Rotation")
                                    .strong()
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(190, 190, 190)),
                            );
                            ui.add_space(2.0);

                            // Row X
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("X")
                                        .color(egui::Color32::from_rgb(239, 68, 68))
                                        .strong(),
                                );
                                if ui
                                    .add(DragValue::new(&mut rot_deg.x).speed(1.0).suffix("°"))
                                    .changed()
                                {
                                    rot_changed = true;
                                }
                            });

                            // Row Y
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Y")
                                        .color(egui::Color32::from_rgb(34, 197, 94))
                                        .strong(),
                                );
                                if ui
                                    .add(DragValue::new(&mut rot_deg.y).speed(1.0).suffix("°"))
                                    .changed()
                                {
                                    rot_changed = true;
                                }
                            });

                            // Row Z
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Z")
                                        .color(egui::Color32::from_rgb(59, 130, 246))
                                        .strong(),
                                );
                                if ui
                                    .add(DragValue::new(&mut rot_deg.z).speed(1.0).suffix("°"))
                                    .changed()
                                {
                                    rot_changed = true;
                                }
                            });
                        });
                    });

                    if dim_changed {
                        if base_size.x > 1e-4 {
                            obj.local_transform.scale.x = current_dim.x / base_size.x;
                        }
                        if base_size.y > 1e-4 {
                            obj.local_transform.scale.y = current_dim.y / base_size.y;
                        }
                        if base_size.z > 1e-4 {
                            obj.local_transform.scale.z = current_dim.z / base_size.z;
                        }
                    }

                    if rot_changed {
                        obj.local_transform.set_euler_angles_deg(rot_deg);
                    }

                    ui.add_space(4.0);
                    ui.separator();

                    // Bottom card actions matching reference image: [ Object ▾ ]  [ Size ▾ ]  [ Apply ]
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("coord_mode_dropdown")
                            .selected_text("Object ▾")
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.editor_state.transform_orientation,
                                    TransformOrientation::Local,
                                    "Object (Local)",
                                );
                                ui.selectable_value(
                                    &mut app.editor_state.transform_orientation,
                                    TransformOrientation::Global,
                                    "World (Global)",
                                );
                            });

                        egui::ComboBox::from_id_salt("size_mode_dropdown")
                            .selected_text("Size ▾")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "World Bounding Size");
                                let _ = ui.selectable_label(false, "Local Scale Factor");
                            });

                        if ui
                            .button("Apply")
                            .on_hover_text("Apply Transform to Object Geometry")
                            .clicked()
                        {
                            apply_tf = true;
                        }

                        if ui
                            .button("↺ Reset")
                            .on_hover_text("Reset Transform")
                            .clicked()
                        {
                            reset_tf = true;
                        }
                    });
                });

            // Origin operations
            ui.horizontal(|ui| {
                ui.label("Origin:");
                if ui
                    .button("To Geometry")
                    .on_hover_text("Center origin to mesh bounding center")
                    .clicked()
                {
                    set_origin_geo = true;
                }
                if ui
                    .button("To 3D Cursor")
                    .on_hover_text("Move origin to current 3D cursor position")
                    .clicked()
                {
                    set_origin_cursor = true;
                }
            });

            // Transform Actions (Copy, Paste, Apply, Reset)
            ui.horizontal(|ui| {
                if ui
                    .button("Copy")
                    .on_hover_text("Copy Transform to clipboard")
                    .clicked()
                {
                    copy_tf = true;
                }
                let has_clip = app.editor_state.transform_clipboard.is_some();
                if ui
                    .add_enabled(has_clip, egui::Button::new("Paste"))
                    .on_hover_text("Paste Transform from clipboard")
                    .clicked()
                {
                    paste_tf = true;
                }
                if ui
                    .button("Apply")
                    .on_hover_text("Apply Transform to Geometry (Ctrl+A)")
                    .clicked()
                {
                    apply_tf = true;
                }
                if ui
                    .button("↺ Reset All")
                    .on_hover_text("Reset Translation, Rotation and Scale (Alt+G/R/S)")
                    .clicked()
                {
                    reset_tf = true;
                }
            });

            // Camera Properties Panel
            if obj.primitive_type == PrimitiveType::Camera || obj.camera_data.is_some() {
                let cam_data = obj.camera_data.get_or_insert_with(CameraData::default);

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Camera Optics & Sensor").strong());

                ui.horizontal(|ui| {
                    if is_active_cam {
                        ui.label(
                            egui::RichText::new("[Active Scene Camera]")
                                .color(egui::Color32::from_rgb(255, 200, 50)),
                        );
                    } else if ui.button("Set as Active Camera").clicked() {
                        set_active_camera = Some(selected_id);
                    }

                    if ui
                        .button("View Camera")
                        .on_hover_text("View through this camera (Numpad 0)")
                        .clicked()
                    {
                        view_camera = Some(selected_id);
                    }

                    if ui
                        .button("⤢ Align to View")
                        .on_hover_text("Align this camera to current viewpoint (Ctrl+Alt+Num0)")
                        .clicked()
                    {
                        align_camera = Some(selected_id);
                    }
                });

                // Projection
                ui.horizontal(|ui| {
                    ui.label("Projection:");
                    egui::ComboBox::from_id_salt("camera_projection_combo")
                        .selected_text(match cam_data.projection {
                            CameraProjectionType::Perspective => "Perspective",
                            CameraProjectionType::Orthographic => "Orthographic",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut cam_data.projection,
                                CameraProjectionType::Perspective,
                                "Perspective",
                            );
                            ui.selectable_value(
                                &mut cam_data.projection,
                                CameraProjectionType::Orthographic,
                                "Orthographic",
                            );
                        });
                });

                match cam_data.projection {
                    CameraProjectionType::Perspective => {
                        let mut fov = cam_data.fov_y_deg;
                        let mut focal = cam_data.focal_length_mm;

                        ui.horizontal(|ui| {
                            ui.label("Focal Length:");
                            if ui
                                .add(Slider::new(&mut focal, 10.0..=300.0).suffix(" mm"))
                                .changed()
                            {
                                cam_data.focal_length_mm = focal;
                                let fov_rad =
                                    2.0 * ((cam_data.sensor_height_mm * 0.5) / focal).atan();
                                cam_data.fov_y_deg = fov_rad.to_degrees().clamp(1.0, 170.0);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Field of View:");
                            if ui
                                .add(Slider::new(&mut fov, 10.0..=120.0).suffix("°"))
                                .changed()
                            {
                                cam_data.fov_y_deg = fov;
                                let half_fov_rad = (fov * 0.5).to_radians();
                                if half_fov_rad.tan().abs() > 1e-4 {
                                    cam_data.focal_length_mm =
                                        (cam_data.sensor_height_mm * 0.5) / half_fov_rad.tan();
                                }
                            }
                        });

                        // Sensor Size
                        ui.horizontal(|ui| {
                            ui.label("Sensor (W × H):");
                            ui.add(
                                DragValue::new(&mut cam_data.sensor_width_mm)
                                    .speed(0.1)
                                    .range(1.0..=100.0)
                                    .suffix(" mm"),
                            );
                            ui.label("×");
                            ui.add(
                                DragValue::new(&mut cam_data.sensor_height_mm)
                                    .speed(0.1)
                                    .range(1.0..=100.0)
                                    .suffix(" mm"),
                            );
                        });
                    }
                    CameraProjectionType::Orthographic => {
                        ui.horizontal(|ui| {
                            ui.label("Ortho Scale:");
                            ui.add(
                                DragValue::new(&mut cam_data.ortho_scale)
                                    .speed(0.1)
                                    .range(0.1..=500.0),
                            );
                        });
                    }
                }

                // Clipping
                ui.horizontal(|ui| {
                    ui.label("Clip Start:");
                    ui.add(
                        DragValue::new(&mut cam_data.clip_start)
                            .speed(0.01)
                            .range(0.001..=100.0)
                            .suffix(" m"),
                    );
                    ui.label("Clip End:");
                    ui.add(
                        DragValue::new(&mut cam_data.clip_end)
                            .speed(1.0)
                            .range(1.0..=100000.0)
                            .suffix(" m"),
                    );
                });

                // Depth of Field
                ui.collapsing("Depth of Field", |ui| {
                    ui.checkbox(&mut cam_data.dof_enabled, "Enable Depth of Field");
                    if cam_data.dof_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Focus Distance:");
                            ui.add(
                                DragValue::new(&mut cam_data.dof_distance)
                                    .speed(0.1)
                                    .range(0.1..=1000.0)
                                    .suffix(" m"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("F-Stop:");
                            ui.add(Slider::new(&mut cam_data.dof_fstop, 0.95..=32.0).prefix("f/"));
                        });
                    }
                });

                // Output Resolution
                ui.collapsing("Camera Resolution", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Resolution:");
                        ui.add(
                            DragValue::new(&mut cam_data.resolution[0])
                                .speed(1)
                                .range(32..=16384)
                                .prefix("X: "),
                        );
                        ui.add(
                            DragValue::new(&mut cam_data.resolution[1])
                                .speed(1)
                                .range(32..=16384)
                                .prefix("Y: "),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button("1080p").clicked() {
                            cam_data.resolution = [1920, 1080];
                        }
                        if ui.button("4K").clicked() {
                            cam_data.resolution = [3840, 2160];
                        }
                        if ui.button("Square 1:1").clicked() {
                            cam_data.resolution = [1080, 1080];
                        }
                    });
                });
            }

            ui.add_space(8.0);
            ui.separator();

            // Modifiers Section
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Modifiers").strong());
                egui::ComboBox::from_id_salt("add_modifier_combo")
                    .selected_text("+ Add Modifier")
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(false, "⇋ Mirror").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Mirror(MirrorModifier::default()));
                        }
                        if ui
                            .selectable_label(false, "▦ Subdivision Surface")
                            .clicked()
                        {
                            add_modifier_type =
                                Some(ModifierType::Subdivide(SubdivisionModifier::default()));
                        }
                        if ui.selectable_label(false, "▣ Solidify").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Solidify(SolidifyModifier::default()));
                        }
                        if ui.selectable_label(false, "☰ Array").clicked() {
                            add_modifier_type = Some(ModifierType::Array(ArrayModifier::default()));
                        }
                        if ui.selectable_label(false, "⊿ Bevel").clicked() {
                            add_modifier_type = Some(ModifierType::Bevel(BevelModifier::default()));
                        }
                        if ui.selectable_label(false, "▽ Decimate").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Decimate(DecimateModifier::default()));
                        }
                        if ui.selectable_label(false, "∿ Displace").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Displace(DisplaceModifier::default()));
                        }
                        if ui.selectable_label(false, "⬡ Weighted Normal").clicked() {
                            add_modifier_type = Some(ModifierType::WeightedNormal(
                                WeightedNormalModifier::default(),
                            ));
                        }
                    });

                if !obj.modifiers.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("Clear All")
                            .on_hover_text("Remove all modifiers from stack")
                            .clicked()
                        {
                            clear_all_mods = true;
                        }
                        let in_edit = app.editor_state.edit_mode == EditMode::Edit;
                        if ui
                            .add_enabled(!in_edit, egui::Button::new("Apply All"))
                            .on_hover_text(if in_edit {
                                "Cannot apply modifiers in Edit Mode"
                            } else {
                                "Bake all active modifiers sequentially into base mesh"
                            })
                            .clicked()
                        {
                            apply_all_mods = true;
                        }
                    });
                }
            });

            let mut modifier_changed = false;
            let num_mods = obj.modifiers.len();
            for i in 0..num_mods {
                let entry = &mut obj.modifiers.modifiers[i];
                let mod_icon = match &entry.modifier {
                    ModifierType::Mirror(_) => "⇋",
                    ModifierType::Subdivide(_) => "▦",
                    ModifierType::Solidify(_) => "▣",
                    ModifierType::Array(_) => "☰",
                    ModifierType::Bevel(_) => "⊿",
                    ModifierType::Decimate(_) => "▽",
                    ModifierType::Displace(_) => "∿",
                    ModifierType::WeightedNormal(_) => "⬡",
                };

                let card_id = ui.make_persistent_id(format!("mod_card_open_{}_{}", selected_id, i));
                let mut is_open = ui.data_mut(|d| d.get_temp::<bool>(card_id).unwrap_or(true));

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let chevron = if is_open { "▼" } else { "▶" };
                        if ui.small_button(chevron).on_hover_text("Toggle Expand/Collapse").clicked() {
                            is_open = !is_open;
                            ui.data_mut(|d| d.insert_temp(card_id, is_open));
                        }

                        if ui.checkbox(&mut entry.enabled, "")
                            .on_hover_text(if entry.enabled { "Disable modifier in viewport" } else { "Enable modifier in viewport" })
                            .changed()
                        {
                            modifier_changed = true;
                        }

                        ui.label(egui::RichText::new(format!("{} {}", mod_icon, entry.name)).strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").on_hover_text("Delete Modifier").clicked() {
                                remove_mod_idx = Some(i);
                            }
                            let in_edit_mode = app.editor_state.edit_mode == EditMode::Edit;
                            if ui.add_enabled(!in_edit_mode, egui::Button::new("✔ Apply"))
                                .on_hover_text(if in_edit_mode { "Cannot apply modifiers in Edit Mode" } else { "Bake modifier into base mesh" })
                                .clicked()
                            {
                                apply_mod_idx = Some(i);
                            }
                            if ui.button("⎘").on_hover_text("Duplicate Modifier").clicked() {
                                duplicate_mod_idx = Some(i);
                            }
                            if i + 1 < num_mods && ui.button("▼").on_hover_text("Move Down").clicked() {
                                move_down_idx = Some(i);
                            }
                            if i > 0 && ui.button("▲").on_hover_text("Move Up").clicked() {
                                move_up_idx = Some(i);
                            }
                        });
                    });

                    // Modifier-specific parameters
                    if is_open {
                        ui.add_space(2.0);
                        match &mut entry.modifier {
                            ModifierType::Mirror(m) => {
                                ui.horizontal(|ui| {
                                    ui.label("Axes:");
                                    if ui.checkbox(&mut m.axis_x, "X").changed() {
                                        modifier_changed = true;
                                    }
                                    if ui.checkbox(&mut m.axis_y, "Y").changed() {
                                        modifier_changed = true;
                                    }
                                    if ui.checkbox(&mut m.axis_z, "Z").changed() {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut m.merge, "Merge Vertices").changed() {
                                        modifier_changed = true;
                                    }
                                    if m.merge {
                                        ui.label("Threshold:");
                                        if ui.add(DragValue::new(&mut m.merge_threshold).speed(0.001).range(0.0001..=1.0)).changed() {
                                            modifier_changed = true;
                                        }
                                    }
                                });
                            }
                            ModifierType::Subdivide(s) => {
                                ui.horizontal(|ui| {
                                    ui.label("Levels:");
                                    if ui.add(Slider::new(&mut s.levels, 1..=4)).changed() {
                                        modifier_changed = true;
                                    }
                                });
                                if s.levels >= 3 {
                                    ui.label(
                                        egui::RichText::new("⚠ Higher subdivision increases vertex count exponentially.")
                                            .color(egui::Color32::from_rgb(240, 180, 50))
                                            .size(11.0),
                                    );
                                }
                            }
                            ModifierType::Solidify(s) => {
                                ui.horizontal(|ui| {
                                    ui.label("Thickness:");
                                    if ui
                                        .add(
                                            DragValue::new(&mut s.thickness)
                                                .speed(0.01)
                                                .range(-5.0..=10.0),
                                        )
                                        .changed()
                                    {
                                        modifier_changed = true;
                                    }
                                });
                            }
                            ModifierType::Array(a) => {
                                ui.horizontal(|ui| {
                                    ui.label("Count:");
                                    if ui.add(Slider::new(&mut a.count, 1..=50)).changed() {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Offset:");
                                    ui.label("X:");
                                    if ui.add(DragValue::new(&mut a.offset.x).speed(0.1)).changed() {
                                        modifier_changed = true;
                                    }
                                    ui.label("Y:");
                                    if ui.add(DragValue::new(&mut a.offset.y).speed(0.1)).changed() {
                                        modifier_changed = true;
                                    }
                                    ui.label("Z:");
                                    if ui.add(DragValue::new(&mut a.offset.z).speed(0.1)).changed() {
                                        modifier_changed = true;
                                    }
                                });
                                if ui.small_button("Reset Offset").clicked() {
                                    a.offset = forge_math::Vec3::new(2.0, 0.0, 0.0);
                                    modifier_changed = true;
                                }
                            }
                            ModifierType::Bevel(b) => {
                                ui.horizontal(|ui| {
                                    ui.label("Width:");
                                    if ui
                                        .add(
                                            DragValue::new(&mut b.width)
                                                .speed(0.01)
                                                .range(0.001..=5.0),
                                        )
                                        .changed()
                                    {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Segments:");
                                    if ui.add(Slider::new(&mut b.segments, 1..=6)).changed() {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Angle threshold:");
                                    let mut deg = b.angle_threshold.to_degrees();
                                    if ui.add(Slider::new(&mut deg, 0.0..=180.0).suffix("°")).changed() {
                                        b.angle_threshold = deg.to_radians();
                                        modifier_changed = true;
                                    }
                                });
                            }
                            ModifierType::Decimate(d) => {
                                let orig_tris = obj.mesh.indices.len() / 3;
                                let est_tris = (orig_tris as f32 * d.ratio).round() as usize;
                                ui.horizontal(|ui| {
                                    ui.label("Ratio:");
                                    if ui
                                        .add(Slider::new(&mut d.ratio, 0.05..=1.0).step_by(0.01))
                                        .changed()
                                    {
                                        modifier_changed = true;
                                    }
                                });
                                ui.label(
                                    egui::RichText::new(format!("Estimated triangles: ~{} (original: {})", est_tris, orig_tris))
                                        .weak()
                                        .size(11.0),
                                );
                            }
                            ModifierType::Displace(d) => {
                                ui.horizontal(|ui| {
                                    ui.label("Strength:");
                                    if ui
                                        .add(DragValue::new(&mut d.strength).speed(0.01).range(-5.0..=5.0))
                                        .changed()
                                    {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Midlevel:");
                                    if ui.add(Slider::new(&mut d.midlevel, 0.0..=1.0)).changed() {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Frequency:");
                                    if ui
                                        .add(
                                            DragValue::new(&mut d.frequency)
                                                .speed(0.1)
                                                .range(0.01..=30.0),
                                        )
                                        .changed()
                                    {
                                        modifier_changed = true;
                                    }
                                });
                            }
                            ModifierType::WeightedNormal(w) => {
                                ui.horizontal(|ui| {
                                    ui.label("Weight:");
                                    if ui
                                        .add(Slider::new(&mut w.weight, 0.0..=1.0).step_by(0.05))
                                        .changed()
                                    {
                                        modifier_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut w.keep_sharp, "Keep Sharp Corners").changed() {
                                        modifier_changed = true;
                                    }
                                });
                            }
                        }
                    }
                });
            }

            if modifier_changed {
                obj.invalidate_evaluated_mesh();
                if let Some(r) = &mut app.renderer {
                    r.invalidate_mesh(&selected_id);
                }
            }

            ui.add_space(8.0);
            ui.separator();

            // Material Section
            ui.label(egui::RichText::new("Material").strong());
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_rgba_unmultiplied(&mut obj.color);
            });
            ui.add_space(4.0);
            if ui
                .button("◫ Open Shader Node Editor")
                .on_hover_text("Open interactive node-based shader graph & WGSL generator")
                .clicked()
            {
                open_shader_editor = true;
            }

            ui.add_space(8.0);
            ui.separator();

            // Geometry Statistics
            ui.label(egui::RichText::new("Geometry Info").strong());
            let eval = obj.evaluated_mesh();
            ui.label(format!("Base Vertices: {}", obj.mesh.vertices.len()));
            ui.label(format!("Evaluated Vertices: {}", eval.vertices.len()));
            ui.label(format!("Triangles: {}", eval.indices.len() / 3));
        }

        if open_shader_editor {
            app.open_shader_editor_for_selected();
        }
        if copy_tf && let Some(obj) = app.scene.get_object(selected_id) {
            app.editor_state.transform_clipboard = Some(obj.local_transform);
            app.status_message = "Copied transform to clipboard".to_string();
        }
        if paste_tf
            && let Some(clip) = app.editor_state.transform_clipboard
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.local_transform = clip;
            app.status_message = "Pasted transform from clipboard".to_string();
        }
        if set_origin_geo {
            app.set_origin_selected_geometry();
        }
        if set_origin_cursor {
            app.set_origin_selected_cursor();
        }
        if let Some(cam_id) = set_active_camera {
            app.scene.set_active_camera(cam_id);
            app.status_message = "Set active camera".to_string();
        }
        if let Some(cam_id) = view_camera {
            app.scene.set_active_camera(cam_id);
            app.editor_state.is_camera_view = true;
            app.status_message = "Viewing through scene camera".to_string();
        }
        if align_camera.is_some() {
            app.align_active_camera_to_view();
        }
        if apply_tf {
            app.apply_transform_selected();
        }
        if reset_tf {
            app.scene.reset_transform_for_object(selected_id);
        }
        if let Some(mod_type) = add_modifier_type {
            app.add_modifier(selected_id, mod_type);
        }
        if let Some(idx) = remove_mod_idx {
            app.remove_modifier(selected_id, idx);
        }
        if let Some(idx) = duplicate_mod_idx {
            app.duplicate_modifier(selected_id, idx);
        }
        if let Some(idx) = apply_mod_idx {
            app.apply_modifier(selected_id, idx);
        }
        if apply_all_mods {
            app.apply_all_modifiers(selected_id);
        }
        if clear_all_mods {
            app.clear_all_modifiers(selected_id);
        }
        if let Some(idx) = move_up_idx {
            app.move_modifier_up(selected_id, idx);
        }
        if let Some(idx) = move_down_idx {
            app.move_modifier_down(selected_id, idx);
        }
    } else {
        ui.label("No object selected.");
        ui.add_space(8.0);
        ui.label(
            "Click an object in the 3D viewport or outliner to view and modify its properties.",
        );
        ui.add_space(16.0);
        ui.heading("Viewport Zoom Settings");
        ui.separator();
        if ui
            .checkbox(
                &mut app.editor_state.panel_settings.zoom_to_cursor,
                "Zoom to Cursor",
            )
            .on_hover_text("Zoom towards the 3D surface or focus plane under the mouse cursor")
            .changed()
        {
            app.camera.settings.zoom_to_cursor = app.editor_state.panel_settings.zoom_to_cursor;
            let _ = app.editor_state.panel_settings.save();
        }
        if ui
            .checkbox(
                &mut app.editor_state.panel_settings.smooth_zoom,
                "Smooth Zoom",
            )
            .on_hover_text(
                "Interpolate camera distance smoothly using frame-rate independent easing",
            )
            .changed()
        {
            app.camera.settings.smooth_zoom = app.editor_state.panel_settings.smooth_zoom;
            if !app.camera.settings.smooth_zoom {
                app.camera.sync_zoom_targets();
            }
            let _ = app.editor_state.panel_settings.save();
        }
        ui.horizontal(|ui| {
            ui.label("Sensitivity:");
            let mut sens = app.editor_state.panel_settings.zoom_sensitivity;
            if ui
                .add(
                    egui::Slider::new(&mut sens, 0.1..=3.0)
                        .step_by(0.1)
                        .text("x"),
                )
                .on_hover_text("Adjust mouse wheel zoom sensitivity multiplier (default 1.0x)")
                .changed()
            {
                app.editor_state.panel_settings.zoom_sensitivity = sens;
                app.camera.settings.zoom_sensitivity = sens;
                let _ = app.editor_state.panel_settings.save();
            }
        });
    }
}
