use egui::{DragValue, Slider, Ui};
use forge_core::id::ObjectId;
use forge_modifiers::{
    ArrayModifier, MirrorModifier, ModifierType, SolidifyModifier, SubdivisionModifier,
};
use forge_scene::{
    CameraData, CameraProjectionType, PivotPoint, PrimitiveType, TransformOrientation,
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

        let mut add_modifier_type = None;
        let mut remove_mod_idx = None;
        let mut apply_mod_idx = None;
        let mut move_up_idx = None;
        let mut move_down_idx = None;

        let is_active_cam = app.scene.active_camera_id == Some(selected_id);

        if let Some(obj) = app.scene.get_object_mut(selected_id) {
            // Object Name & Visibility & Lock
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut obj.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let lock_icon = if obj.locked { "🔒" } else { "🔓" };
                    if ui
                        .selectable_label(obj.locked, lock_icon)
                        .on_hover_text("Lock Object Transform")
                        .clicked()
                    {
                        obj.locked = !obj.locked;
                    }

                    let eye_icon = if obj.visible { "👁" } else { "🚫" };
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

            // Transform Section
            ui.label(egui::RichText::new("Transform").strong());

            // Orientation & Pivot options
            ui.horizontal(|ui| {
                ui.label("Orientation:");
                egui::ComboBox::from_id_salt("transform_orientation")
                    .selected_text(format!("{:?}", app.editor_state.transform_orientation))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.editor_state.transform_orientation,
                            TransformOrientation::Global,
                            "Global",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.transform_orientation,
                            TransformOrientation::Local,
                            "Local",
                        );
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Pivot Point:");
                egui::ComboBox::from_id_salt("pivot_point")
                    .selected_text(format!("{:?}", app.editor_state.pivot_point))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::MedianPoint,
                            "Median Point",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::ActiveElement,
                            "Active Element",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::Cursor3D,
                            "3D Cursor",
                        );
                        ui.selectable_value(
                            &mut app.editor_state.pivot_point,
                            PivotPoint::IndividualOrigins,
                            "Individual Origins",
                        );
                    });
            });

            ui.add_space(4.0);

            // 1. Position (Translation)
            ui.label("Position (m):");
            ui.horizontal(|ui| {
                // X
                let lock_x = obj.locks.lock_pos[0];
                if ui
                    .selectable_label(lock_x, if lock_x { "🔒" } else { "🔓" })
                    .on_hover_text("Lock X Position")
                    .clicked()
                {
                    obj.locks.lock_pos[0] = !lock_x;
                }
                ui.label("X:");
                ui.add_enabled(
                    !lock_x,
                    DragValue::new(&mut obj.local_transform.translation.x).speed(0.05),
                );
                if ui.button("↺").on_hover_text("Reset X to 0.0").clicked() {
                    obj.local_transform.translation.x = 0.0;
                }

                // Y
                let lock_y = obj.locks.lock_pos[1];
                if ui
                    .selectable_label(lock_y, if lock_y { "🔒" } else { "🔓" })
                    .on_hover_text("Lock Y Position")
                    .clicked()
                {
                    obj.locks.lock_pos[1] = !lock_y;
                }
                ui.label("Y:");
                ui.add_enabled(
                    !lock_y,
                    DragValue::new(&mut obj.local_transform.translation.y).speed(0.05),
                );
                if ui.button("↺").on_hover_text("Reset Y to 0.0").clicked() {
                    obj.local_transform.translation.y = 0.0;
                }

                // Z
                let lock_z = obj.locks.lock_pos[2];
                if ui
                    .selectable_label(lock_z, if lock_z { "🔒" } else { "🔓" })
                    .on_hover_text("Lock Z Position")
                    .clicked()
                {
                    obj.locks.lock_pos[2] = !lock_z;
                }
                ui.label("Z:");
                ui.add_enabled(
                    !lock_z,
                    DragValue::new(&mut obj.local_transform.translation.z).speed(0.05),
                );
                if ui.button("↺").on_hover_text("Reset Z to 0.0").clicked() {
                    obj.local_transform.translation.z = 0.0;
                }
            });

            // 2. Rotation (Degrees)
            let mut rot_deg = obj.local_transform.euler_angles_deg();
            ui.label("Rotation (deg):");
            let mut rot_changed = false;
            ui.horizontal(|ui| {
                // X
                let lock_rx = obj.locks.lock_rot[0];
                if ui
                    .selectable_label(lock_rx, if lock_rx { "🔒" } else { "🔓" })
                    .on_hover_text("Lock X Rotation")
                    .clicked()
                {
                    obj.locks.lock_rot[0] = !lock_rx;
                }
                ui.label("X:");
                if ui
                    .add_enabled(!lock_rx, DragValue::new(&mut rot_deg.x).speed(1.0))
                    .changed()
                {
                    rot_changed = true;
                }
                if ui
                    .button("↺")
                    .on_hover_text("Reset X rotation to 0°")
                    .clicked()
                {
                    rot_deg.x = 0.0;
                    rot_changed = true;
                }

                // Y
                let lock_ry = obj.locks.lock_rot[1];
                if ui
                    .selectable_label(lock_ry, if lock_ry { "🔒" } else { "🔓" })
                    .on_hover_text("Lock Y Rotation")
                    .clicked()
                {
                    obj.locks.lock_rot[1] = !lock_ry;
                }
                ui.label("Y:");
                if ui
                    .add_enabled(!lock_ry, DragValue::new(&mut rot_deg.y).speed(1.0))
                    .changed()
                {
                    rot_changed = true;
                }
                if ui
                    .button("↺")
                    .on_hover_text("Reset Y rotation to 0°")
                    .clicked()
                {
                    rot_deg.y = 0.0;
                    rot_changed = true;
                }

                // Z
                let lock_rz = obj.locks.lock_rot[2];
                if ui
                    .selectable_label(lock_rz, if lock_rz { "🔒" } else { "🔓" })
                    .on_hover_text("Lock Z Rotation")
                    .clicked()
                {
                    obj.locks.lock_rot[2] = !lock_rz;
                }
                ui.label("Z:");
                if ui
                    .add_enabled(!lock_rz, DragValue::new(&mut rot_deg.z).speed(1.0))
                    .changed()
                {
                    rot_changed = true;
                }
                if ui
                    .button("↺")
                    .on_hover_text("Reset Z rotation to 0°")
                    .clicked()
                {
                    rot_deg.z = 0.0;
                    rot_changed = true;
                }
            });
            if rot_changed {
                obj.local_transform.set_euler_angles_deg(rot_deg);
            }

            // 3. Scale
            ui.label("Scale:");
            ui.horizontal(|ui| {
                // X
                let lock_sx = obj.locks.lock_scale[0];
                if ui
                    .selectable_label(lock_sx, if lock_sx { "🔒" } else { "🔓" })
                    .on_hover_text("Lock X Scale")
                    .clicked()
                {
                    obj.locks.lock_scale[0] = !lock_sx;
                }
                ui.label("X:");
                ui.add_enabled(
                    !lock_sx,
                    DragValue::new(&mut obj.local_transform.scale.x)
                        .speed(0.05)
                        .range(0.001..=100.0),
                );
                if ui
                    .button("↺")
                    .on_hover_text("Reset X scale to 1.0")
                    .clicked()
                {
                    obj.local_transform.scale.x = 1.0;
                }

                // Y
                let lock_sy = obj.locks.lock_scale[1];
                if ui
                    .selectable_label(lock_sy, if lock_sy { "🔒" } else { "🔓" })
                    .on_hover_text("Lock Y Scale")
                    .clicked()
                {
                    obj.locks.lock_scale[1] = !lock_sy;
                }
                ui.label("Y:");
                ui.add_enabled(
                    !lock_sy,
                    DragValue::new(&mut obj.local_transform.scale.y)
                        .speed(0.05)
                        .range(0.001..=100.0),
                );
                if ui
                    .button("↺")
                    .on_hover_text("Reset Y scale to 1.0")
                    .clicked()
                {
                    obj.local_transform.scale.y = 1.0;
                }

                // Z
                let lock_sz = obj.locks.lock_scale[2];
                if ui
                    .selectable_label(lock_sz, if lock_sz { "🔒" } else { "🔓" })
                    .on_hover_text("Lock Z Scale")
                    .clicked()
                {
                    obj.locks.lock_scale[2] = !lock_sz;
                }
                ui.label("Z:");
                ui.add_enabled(
                    !lock_sz,
                    DragValue::new(&mut obj.local_transform.scale.z)
                        .speed(0.05)
                        .range(0.001..=100.0),
                );
                if ui
                    .button("↺")
                    .on_hover_text("Reset Z scale to 1.0")
                    .clicked()
                {
                    obj.local_transform.scale.z = 1.0;
                }
            });

            // 4. Dimensions
            let base_size = obj.mesh.aabb.size();
            let mut current_dim = base_size * obj.local_transform.scale.abs();
            ui.label("Dimensions (m):");
            ui.horizontal(|ui| {
                ui.label("X:");
                if ui
                    .add(
                        DragValue::new(&mut current_dim.x)
                            .speed(0.05)
                            .range(0.001..=1000.0),
                    )
                    .changed()
                    && base_size.x > 1e-4
                {
                    obj.local_transform.scale.x = current_dim.x / base_size.x;
                }
                ui.label("Y:");
                if ui
                    .add(
                        DragValue::new(&mut current_dim.y)
                            .speed(0.05)
                            .range(0.001..=1000.0),
                    )
                    .changed()
                    && base_size.y > 1e-4
                {
                    obj.local_transform.scale.y = current_dim.y / base_size.y;
                }
                ui.label("Z:");
                if ui
                    .add(
                        DragValue::new(&mut current_dim.z)
                            .speed(0.05)
                            .range(0.001..=1000.0),
                    )
                    .changed()
                    && base_size.z > 1e-4
                {
                    obj.local_transform.scale.z = current_dim.z / base_size.z;
                }
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
                    .button("📋 Copy")
                    .on_hover_text("Copy Transform to clipboard")
                    .clicked()
                {
                    copy_tf = true;
                }
                let has_clip = app.editor_state.transform_clipboard.is_some();
                if ui
                    .add_enabled(has_clip, egui::Button::new("📄 Paste"))
                    .on_hover_text("Paste Transform from clipboard")
                    .clicked()
                {
                    paste_tf = true;
                }
                if ui
                    .button("✔ Apply")
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
                ui.label(egui::RichText::new("📷 Camera Optics & Sensor").strong());

                ui.horizontal(|ui| {
                    if is_active_cam {
                        ui.label(
                            egui::RichText::new("★ Active Scene Camera")
                                .color(egui::Color32::from_rgb(255, 200, 50)),
                        );
                    } else if ui.button("★ Set as Active Camera").clicked() {
                        set_active_camera = Some(selected_id);
                    }

                    if ui
                        .button("📷 View Camera")
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
                        if ui.selectable_label(false, "Mirror").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Mirror(MirrorModifier::default()));
                        }
                        if ui.selectable_label(false, "Subdivision Surface").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Subdivide(SubdivisionModifier::default()));
                        }
                        if ui.selectable_label(false, "Solidify").clicked() {
                            add_modifier_type =
                                Some(ModifierType::Solidify(SolidifyModifier::default()));
                        }
                        if ui.selectable_label(false, "Array").clicked() {
                            add_modifier_type = Some(ModifierType::Array(ArrayModifier::default()));
                        }
                    });
            });

            let num_mods = obj.modifiers.len();
            for i in 0..num_mods {
                let entry = &mut obj.modifiers.modifiers[i];
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut entry.enabled, "");
                        ui.label(egui::RichText::new(&entry.name).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("❌").on_hover_text("Delete Modifier").clicked() {
                                remove_mod_idx = Some(i);
                            }
                            if ui.button("✔").on_hover_text("Apply to Mesh").clicked() {
                                apply_mod_idx = Some(i);
                            }
                            if i > 0 && ui.button("▲").clicked() {
                                move_up_idx = Some(i);
                            }
                            if i + 1 < num_mods && ui.button("▼").clicked() {
                                move_down_idx = Some(i);
                            }
                        });
                    });

                    // Modifier-specific parameters
                    match &mut entry.modifier {
                        ModifierType::Mirror(m) => {
                            ui.horizontal(|ui| {
                                ui.label("Axis:");
                                ui.checkbox(&mut m.axis_x, "X");
                                ui.checkbox(&mut m.axis_y, "Y");
                                ui.checkbox(&mut m.axis_z, "Z");
                            });
                        }
                        ModifierType::Subdivide(s) => {
                            ui.horizontal(|ui| {
                                ui.label("Levels:");
                                ui.add(egui::Slider::new(&mut s.levels, 1..=4));
                            });
                        }
                        ModifierType::Solidify(s) => {
                            ui.horizontal(|ui| {
                                ui.label("Thickness:");
                                ui.add(
                                    DragValue::new(&mut s.thickness)
                                        .speed(0.01)
                                        .range(0.0..=10.0),
                                );
                            });
                        }
                        ModifierType::Array(a) => {
                            ui.horizontal(|ui| {
                                ui.label("Count:");
                                ui.add(egui::Slider::new(&mut a.count, 1..=20));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Offset:");
                                ui.add(DragValue::new(&mut a.offset.x).speed(0.1));
                                ui.add(DragValue::new(&mut a.offset.y).speed(0.1));
                                ui.add(DragValue::new(&mut a.offset.z).speed(0.1));
                            });
                        }
                    }
                });
            }

            ui.add_space(8.0);
            ui.separator();

            // Material Section
            ui.label(egui::RichText::new("Material").strong());
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_rgba_unmultiplied(&mut obj.color);
            });

            ui.add_space(8.0);
            ui.separator();

            // Geometry Statistics
            ui.label(egui::RichText::new("Geometry Info").strong());
            let eval = obj.evaluated_mesh();
            ui.label(format!("Base Vertices: {}", obj.mesh.vertices.len()));
            ui.label(format!("Evaluated Vertices: {}", eval.vertices.len()));
            ui.label(format!("Triangles: {}", eval.indices.len() / 3));
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
        if let Some(mod_type) = add_modifier_type
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.add(mod_type);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
        if let Some(idx) = remove_mod_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.remove(idx);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
        if let Some(idx) = apply_mod_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.apply_to_mesh(idx, &mut obj.mesh);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
            app.status_message = "Applied modifier to base mesh".to_string();
        }
        if let Some(idx) = move_up_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.move_up(idx);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
        }
        if let Some(idx) = move_down_idx
            && let Some(obj) = app.scene.get_object_mut(selected_id)
        {
            obj.modifiers.move_down(idx);
            if let Some(r) = &mut app.renderer {
                r.invalidate_mesh(&selected_id);
            }
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
