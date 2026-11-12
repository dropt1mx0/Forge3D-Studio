//! Interactive Shader Node Editor canvas with node graph visualization,
//! socket cabling, live WGSL code generation, and PBR material synchronization.

use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use forge_scene::shader_graph::{
    MathOp, NodeKind, NodeSocketRef, ShaderGraph, SocketType, VectorMathOp,
};

/// Interaction state for the interactive shader node editor.
#[derive(Debug, Clone)]
pub struct ShaderEditorState {
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_node: Option<u64>,
    pub dragging_node: Option<u64>,
    pub connecting_from: Option<NodeSocketRef>,
    pub show_add_menu: Option<Pos2>,
    pub show_wgsl_code: bool,
    pub auto_sync: bool,
    pub last_copied_time: Option<Instant>,
    pub cached_wgsl: String,
    pub wgsl_dirty: bool,
}

impl Default for ShaderEditorState {
    fn default() -> Self {
        Self {
            pan: Vec2::new(320.0, 240.0),
            zoom: 1.0,
            selected_node: None,
            dragging_node: None,
            connecting_from: None,
            show_add_menu: None,
            show_wgsl_code: false,
            auto_sync: true,
            last_copied_time: None,
            cached_wgsl: String::new(),
            wgsl_dirty: true,
        }
    }
}

fn socket_type_color(socket_type: SocketType) -> Color32 {
    match socket_type {
        SocketType::Float => Color32::from_rgb(165, 175, 185),
        SocketType::Color => Color32::from_rgb(235, 195, 55),
        SocketType::Vector => Color32::from_rgb(110, 140, 245),
        SocketType::Shader => Color32::from_rgb(95, 215, 115),
    }
}

fn category_color(category: &str) -> Color32 {
    match category {
        "Output" => Color32::from_rgb(160, 45, 45),
        "Shader" => Color32::from_rgb(45, 135, 65),
        "Texture" => Color32::from_rgb(175, 115, 25),
        "Color" => Color32::from_rgb(170, 145, 30),
        "Vector" => Color32::from_rgb(85, 60, 155),
        "Converter" => Color32::from_rgb(45, 95, 160),
        "Input" => Color32::from_rgb(35, 125, 135),
        _ => Color32::from_rgb(70, 75, 85),
    }
}

/// Renders the complete Shader Node Editor window.
pub fn render_shader_editor(
    app: &mut crate::app::Forge3dStudioApp,
    ctx: &egui::Context,
    open: &mut bool,
) {
    let mut is_open = *open;
    let window_title = format!("Shader Node Editor — {}", app.active_shader_graph.name);

    egui::Window::new(window_title)
        .open(&mut is_open)
        .resizable(true)
        .default_size(Vec2::new(880.0, 580.0))
        .default_pos(Pos2::new(180.0, 100.0))
        .show(ctx, |ui| {
            render_editor_contents(app, ui);
        });

    *open = is_open;
}

fn render_editor_contents(app: &mut crate::app::Forge3dStudioApp, ui: &mut egui::Ui) {
    let mut graph_modified = false;

    // 1. Top Toolbar
    ui.horizontal(|ui| {
        if ui
            .button(egui::RichText::new("➕ Add Node").strong())
            .clicked()
        {
            let center = ui.clip_rect().center();
            app.shader_editor_state.show_add_menu = Some(center);
        }

        if let Some(sel_id) = app.shader_editor_state.selected_node {
            if ui.button("🗑 Delete Node").clicked() {
                app.active_shader_graph.remove_node(sel_id);
                app.shader_editor_state.selected_node = None;
                graph_modified = true;
            }
        }

        ui.separator();

        if ui.button("⟳ Sync to Material").clicked() {
            app.sync_active_shader_graph_to_material();
            app.status_message = "Synchronized shader graph to viewport material".into();
        }

        ui.checkbox(&mut app.shader_editor_state.auto_sync, "Auto-Sync PBR");

        ui.separator();

        let wgsl_btn_label = if app.shader_editor_state.show_wgsl_code {
            "▾ Hide WGSL Code"
        } else {
            "▸ View WGSL Code"
        };
        if ui.button(wgsl_btn_label).clicked() {
            app.shader_editor_state.show_wgsl_code = !app.shader_editor_state.show_wgsl_code;
        }

        ui.separator();

        if ui.button("⌖ Center View").clicked() {
            app.shader_editor_state.pan = Vec2::new(ui.available_width() * 0.45, 200.0);
            app.shader_editor_state.zoom = 1.0;
        }

        ui.label(format!(
            "Zoom: {:.0}%",
            app.shader_editor_state.zoom * 100.0
        ));
    });

    ui.separator();

    // 2. Main Area: Canvas (and optional split WGSL preview)
    let available_height = ui.available_height();
    let (canvas_height, wgsl_height) = if app.shader_editor_state.show_wgsl_code {
        (available_height * 0.65, available_height * 0.35)
    } else {
        (available_height, 0.0)
    };

    // Render Canvas
    render_canvas(app, ui, canvas_height, &mut graph_modified);

    // Render WGSL Code Drawer if expanded
    if app.shader_editor_state.show_wgsl_code {
        ui.separator();
        render_wgsl_drawer(app, ui, wgsl_height);
    }

    // Auto-sync if graph changed
    if graph_modified {
        app.shader_editor_state.wgsl_dirty = true;
        if app.shader_editor_state.auto_sync {
            app.sync_active_shader_graph_to_material();
        }
    }
}

fn render_canvas(
    app: &mut crate::app::Forge3dStudioApp,
    ui: &mut egui::Ui,
    height: f32,
    graph_modified: &mut bool,
) {
    let (canvas_rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );

    let painter = ui.painter_at(canvas_rect);

    // Pan with Middle Mouse Drag or Canvas Drag
    if response.dragged_by(egui::PointerButton::Middle)
        || (response.dragged_by(egui::PointerButton::Primary)
            && app.shader_editor_state.dragging_node.is_none()
            && app.shader_editor_state.connecting_from.is_none())
    {
        app.shader_editor_state.pan += response.drag_delta();
    }

    // Zoom with Scroll Wheel
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let zoom_factor = if scroll > 0.0 { 1.1 } else { 0.9 };
            let old_zoom = app.shader_editor_state.zoom;
            let new_zoom = (old_zoom * zoom_factor).clamp(0.4, 2.5);

            if let Some(mouse_pos) = response.hover_pos() {
                let mouse_rel = mouse_pos - canvas_rect.min - app.shader_editor_state.pan;
                app.shader_editor_state.pan += mouse_rel * (1.0 - new_zoom / old_zoom);
            }
            app.shader_editor_state.zoom = new_zoom;
        }

        // Shortcut Shift+A to open Add menu at mouse pos
        if ui.input(|i| i.modifiers.shift && i.key_pressed(egui::Key::A)) {
            app.shader_editor_state.show_add_menu = response.hover_pos();
        }

        // Delete key removes selected node
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::X)) {
            if let Some(sel_id) = app.shader_editor_state.selected_node {
                app.active_shader_graph.remove_node(sel_id);
                app.shader_editor_state.selected_node = None;
                *graph_modified = true;
            }
        }
    }

    // Cancel cable drag on right click or escape
    if ui.input(|i| i.key_pressed(egui::Key::Escape) || i.pointer.secondary_clicked()) {
        app.shader_editor_state.connecting_from = None;
        app.shader_editor_state.show_add_menu = None;
    }

    // 1. Draw Canvas Background & Grid
    painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(24, 26, 30));

    let pan = app.shader_editor_state.pan;
    let zoom = app.shader_editor_state.zoom;
    let grid_spacing = 40.0 * zoom;

    let start_x = canvas_rect.min.x + (pan.x % grid_spacing);
    let start_y = canvas_rect.min.y + (pan.y % grid_spacing);

    let grid_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12));
    let mut x = start_x;
    while x < canvas_rect.max.x {
        painter.line_segment([Pos2::new(x, canvas_rect.min.y), Pos2::new(x, canvas_rect.max.y)], grid_stroke);
        x += grid_spacing;
    }
    let mut y = start_y;
    while y < canvas_rect.max.y {
        painter.line_segment([Pos2::new(canvas_rect.min.x, y), Pos2::new(canvas_rect.max.x, y)], grid_stroke);
        y += grid_spacing;
    }

    // Coordinate conversion functions
    let to_screen = |graph_pos: [f32; 2]| -> Pos2 {
        Pos2::new(
            canvas_rect.min.x + pan.x + graph_pos[0] * zoom,
            canvas_rect.min.y + pan.y + graph_pos[1] * zoom,
        )
    };

    let to_graph = |screen_pos: Pos2| -> [f32; 2] {
        [
            (screen_pos.x - canvas_rect.min.x - pan.x) / zoom,
            (screen_pos.y - canvas_rect.min.y - pan.y) / zoom,
        ]
    };

    // Collect socket positions for drawing cables: (node_id, socket_id, is_output) -> Pos2
    let mut socket_positions = std::collections::HashMap::new();

    // 2. Render Existing Cables
    for conn in &app.active_shader_graph.connections {
        // Find positions
        let p_from = find_socket_pos(
            &app.active_shader_graph,
            conn.from.node_id,
            &conn.from.socket_id,
            true,
            to_screen,
        );
        let p_to = find_socket_pos(
            &app.active_shader_graph,
            conn.to.node_id,
            &conn.to.socket_id,
            false,
            to_screen,
        );

        if let (Some(p1), Some(p2)) = (p_from, p_to) {
            let socket_type = app
                .active_shader_graph
                .find_node(conn.from.node_id)
                .and_then(|n| n.outputs.iter().find(|o| o.id == conn.from.socket_id))
                .map(|o| o.socket_type)
                .unwrap_or(SocketType::Float);

            let wire_color = socket_type_color(socket_type);
            draw_bezier_wire(&painter, p1, p2, wire_color, 2.5 * zoom);
        }
    }

    // 3. Render In-Progress Dragged Cable
    if let Some(from) = &app.shader_editor_state.connecting_from {
        let p_from = find_socket_pos(
            &app.active_shader_graph,
            from.node_id,
            &from.socket_id,
            true,
            to_screen,
        );
        if let (Some(p1), Some(mouse_pos)) = (p_from, ui.input(|i| i.pointer.hover_pos())) {
            let socket_type = app
                .active_shader_graph
                .find_node(from.node_id)
                .and_then(|n| n.outputs.iter().find(|o| o.id == from.socket_id))
                .map(|o| o.socket_type)
                .unwrap_or(SocketType::Float);

            let wire_color = socket_type_color(socket_type);
            draw_bezier_wire(&painter, p1, mouse_pos, wire_color, 2.0 * zoom);
        }
    }

    // 4. Render Nodes
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let mut pending_connect = None;
    let mut pending_disconnect = None;
    let mut pending_drag_node = None;

    let active_inputs: std::collections::HashSet<(u64, String)> = app
        .active_shader_graph
        .connections
        .iter()
        .map(|c| (c.to.node_id, c.to.socket_id.clone()))
        .collect();

    for node in &mut app.active_shader_graph.nodes {
        let node_id = node.id;
        let is_selected = app.shader_editor_state.selected_node == Some(node_id);

        let screen_origin = to_screen(node.position);
        let node_width = 190.0 * zoom;
        let header_height = 28.0 * zoom;

        let row_height = 24.0 * zoom;
        let total_rows = node.inputs.len().max(node.outputs.len()).max(1);
        let body_height = total_rows as f32 * row_height + 16.0 * zoom;
        let node_rect = Rect::from_min_size(
            screen_origin,
            Vec2::new(node_width, header_height + body_height),
        );

        // Header rect
        let header_rect = Rect::from_min_size(screen_origin, Vec2::new(node_width, header_height));

        // Node card background
        let bg_color = Color32::from_rgb(34, 38, 46);
        let border_color = if is_selected {
            Color32::from_rgb(255, 150, 40)
        } else {
            Color32::from_rgb(60, 66, 78)
        };
        let border_width = if is_selected { 2.0 } else { 1.0 };

        painter.rect_filled(node_rect, 6.0, bg_color);
        painter.rect_stroke(
            node_rect,
            6.0,
            Stroke::new(border_width, border_color),
            egui::StrokeKind::Outside,
        );

        // Header background
        let h_color = category_color(node.kind.category());
        painter.rect_filled(header_rect, 6.0, h_color);
        // Header title text
        painter.text(
            header_rect.left_center() + Vec2::new(8.0 * zoom, 0.0),
            egui::Align2::LEFT_CENTER,
            &node.name,
            egui::FontId::proportional(13.0 * zoom.max(0.7)),
            Color32::WHITE,
        );

        // Header dragging & selection
        if let Some(pos) = pointer_pos {
            if header_rect.contains(pos) && ui.input(|i| i.pointer.primary_clicked()) {
                app.shader_editor_state.selected_node = Some(node_id);
                pending_drag_node = Some(node_id);
            }
        }

        if app.shader_editor_state.dragging_node == Some(node_id) {
            let delta = ui.input(|i| i.pointer.delta());
            node.position[0] += delta.x / zoom;
            node.position[1] += delta.y / zoom;
        }

        // Render Inputs
        let mut row_y = header_rect.max.y + 12.0 * zoom;
        for input in &mut node.inputs {
            let socket_center = Pos2::new(node_rect.min.x, row_y);
            socket_positions.insert((node_id, input.id.clone(), false), socket_center);

            let sock_color = socket_type_color(input.socket_type);
            let has_conn = active_inputs.contains(&(node_id, input.id.clone()));

            // Socket pin
            let pin_radius = 5.0 * zoom;
            if has_conn {
                painter.circle_filled(socket_center, pin_radius, sock_color);
            } else {
                painter.circle_filled(socket_center, pin_radius, Color32::from_rgb(45, 48, 55));
                painter.circle_stroke(socket_center, pin_radius, Stroke::new(1.5, sock_color));
            }

            // Socket Name
            painter.text(
                socket_center + Vec2::new(10.0 * zoom, 0.0),
                egui::Align2::LEFT_CENTER,
                &input.name,
                egui::FontId::proportional(11.0 * zoom.max(0.7)),
                Color32::from_rgb(210, 215, 225),
            );

            // Interaction on input socket pin
            if let Some(pos) = pointer_pos {
                if pos.distance(socket_center) <= 10.0 * zoom {
                    painter.circle_stroke(socket_center, pin_radius + 2.0, Stroke::new(1.5, Color32::WHITE));

                    // Click to connect or disconnect
                    if ui.input(|i| i.pointer.primary_clicked() || i.pointer.primary_released()) {
                        if let Some(from) = &app.shader_editor_state.connecting_from {
                            pending_connect = Some((from.clone(), NodeSocketRef::new(node_id, input.id.clone())));
                        } else if has_conn {
                            // Disconnect input
                            pending_disconnect = Some(NodeSocketRef::new(node_id, input.id.clone()));
                        }
                    }
                }
            }

            row_y += row_height;
        }

        // Render Outputs
        let mut row_y = header_rect.max.y + 12.0 * zoom;
        for output in &node.outputs {
            let socket_center = Pos2::new(node_rect.max.x, row_y);
            socket_positions.insert((node_id, output.id.clone(), true), socket_center);

            let sock_color = socket_type_color(output.socket_type);

            // Socket pin
            let pin_radius = 5.0 * zoom;
            painter.circle_filled(socket_center, pin_radius, sock_color);

            // Socket Name
            painter.text(
                socket_center - Vec2::new(10.0 * zoom, 0.0),
                egui::Align2::RIGHT_CENTER,
                &output.name,
                egui::FontId::proportional(11.0 * zoom.max(0.7)),
                Color32::from_rgb(210, 215, 225),
            );

            // Interaction on output socket pin
            if let Some(pos) = pointer_pos {
                if pos.distance(socket_center) <= 10.0 * zoom {
                    painter.circle_stroke(socket_center, pin_radius + 2.0, Stroke::new(1.5, Color32::WHITE));

                    if ui.input(|i| i.pointer.primary_clicked()) {
                        app.shader_editor_state.connecting_from = Some(NodeSocketRef::new(node_id, output.id.clone()));
                    }
                }
            }

            row_y += row_height;
        }
    }

    // Process pending disconnect
    if let Some(target) = pending_disconnect {
        app.active_shader_graph.disconnect_input(&target);
        *graph_modified = true;
    }

    // Process drag node state
    if let Some(id) = pending_drag_node {
        app.shader_editor_state.dragging_node = Some(id);
    }
    if ui.input(|i| !i.pointer.primary_down()) {
        app.shader_editor_state.dragging_node = None;
    }

    // Process pending connection
    let had_pending_connect = pending_connect.is_some();
    if let Some((from, to)) = pending_connect {
        if from.node_id != to.node_id {
            app.active_shader_graph.connect(from, to);
            *graph_modified = true;
        }
        app.shader_editor_state.connecting_from = None;
    }

    // Release cable on empty canvas cancels connection
    if app.shader_editor_state.connecting_from.is_some()
        && ui.input(|i| i.pointer.primary_released())
        && !had_pending_connect
    {
        app.shader_editor_state.connecting_from = None;
    }

    // 5. Add Node Popup Menu
    if let Some(popup_pos) = app.shader_editor_state.show_add_menu {
        egui::Area::new(egui::Id::unique("shader_add_node_popup"))
            .fixed_pos(popup_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_max_width(220.0);
                    ui.label(egui::RichText::new("➕ Add Shader Node").strong());
                    ui.separator();

                    let graph_click_pos = to_graph(popup_pos);

                    let mut add_kind = None;

                    ui.menu_button("Input ▸", |ui| {
                        if ui.button("UV Map").clicked() {
                            add_kind = Some(NodeKind::UvMap);
                            ui.close();
                        }
                        if ui.button("Geometry").clicked() {
                            add_kind = Some(NodeKind::Geometry);
                            ui.close();
                        }
                    });

                    ui.menu_button("Texture ▸", |ui| {
                        if ui.button("Noise Texture").clicked() {
                            add_kind = Some(NodeKind::NoiseTexture);
                            ui.close();
                        }
                        if ui.button("Voronoi Texture").clicked() {
                            add_kind = Some(NodeKind::VoronoiTexture);
                            ui.close();
                        }
                        if ui.button("Checker Texture").clicked() {
                            add_kind = Some(NodeKind::CheckerTexture);
                            ui.close();
                        }
                        if ui.button("Image Texture").clicked() {
                            add_kind = Some(NodeKind::ImageTexture {
                                file_path: String::new(),
                            });
                            ui.close();
                        }
                    });

                    ui.menu_button("Color ▸", |ui| {
                        if ui.button("Mix Color").clicked() {
                            add_kind = Some(NodeKind::MixColor);
                            ui.close();
                        }
                        if ui.button("Color Ramp").clicked() {
                            add_kind = Some(NodeKind::ColorRamp {
                                stops: vec![
                                    (0.0, [0.0, 0.0, 0.0, 1.0]),
                                    (1.0, [1.0, 1.0, 1.0, 1.0]),
                                ],
                            });
                            ui.close();
                        }
                        if ui.button("RGB Color").clicked() {
                            add_kind = Some(NodeKind::Rgb {
                                color: [1.0, 1.0, 1.0, 1.0],
                            });
                            ui.close();
                        }
                    });

                    ui.menu_button("Vector ▸", |ui| {
                        if ui.button("Vector Math").clicked() {
                            add_kind = Some(NodeKind::VectorMath {
                                op: VectorMathOp::Add,
                            });
                            ui.close();
                        }
                        if ui.button("Mapping").clicked() {
                            add_kind = Some(NodeKind::Mapping {
                                translation: [0.0; 3],
                                rotation: [0.0; 3],
                                scale: [1.0; 3],
                            });
                            ui.close();
                        }
                        if ui.button("Normal Map").clicked() {
                            add_kind = Some(NodeKind::NormalMap);
                            ui.close();
                        }
                    });

                    ui.menu_button("Converter ▸", |ui| {
                        if ui.button("Math").clicked() {
                            add_kind = Some(NodeKind::Math { op: MathOp::Add });
                            ui.close();
                        }
                        if ui.button("Value").clicked() {
                            add_kind = Some(NodeKind::Value { value: 1.0 });
                            ui.close();
                        }
                        if ui.button("Separate RGB").clicked() {
                            add_kind = Some(NodeKind::SeparateRgb);
                            ui.close();
                        }
                        if ui.button("Combine RGB").clicked() {
                            add_kind = Some(NodeKind::CombineRgb);
                            ui.close();
                        }
                    });

                    ui.menu_button("Shader ▸", |ui| {
                        if ui.button("Principled BSDF").clicked() {
                            add_kind = Some(NodeKind::PrincipledBsdf);
                            ui.close();
                        }
                        if ui.button("Emission").clicked() {
                            add_kind = Some(NodeKind::Emission);
                            ui.close();
                        }
                    });

                    ui.menu_button("Output ▸", |ui| {
                        if ui.button("Material Output").clicked() {
                            add_kind = Some(NodeKind::MaterialOutput);
                            ui.close();
                        }
                    });

                    if let Some(kind) = add_kind {
                        let new_id = app.active_shader_graph.add_node(kind, graph_click_pos);
                        app.shader_editor_state.selected_node = Some(new_id);
                        app.shader_editor_state.show_add_menu = None;
                        *graph_modified = true;
                    }
                });
            });
    }
}

fn find_socket_pos(
    graph: &ShaderGraph,
    node_id: u64,
    socket_id: &str,
    is_output: bool,
    to_screen: impl Fn([f32; 2]) -> Pos2,
) -> Option<Pos2> {
    let node = graph.find_node(node_id)?;
    let screen_origin = to_screen(node.position);
    let node_width = 190.0; // unscaled reference width
    let header_height = 28.0;
    let row_height = 24.0;

    let index = if is_output {
        node.outputs.iter().position(|o| o.id == socket_id)?
    } else {
        node.inputs.iter().position(|i| i.id == socket_id)?
    };

    let y = screen_origin.y + header_height + 12.0 + (index as f32) * row_height;
    let x = if is_output {
        screen_origin.x + node_width
    } else {
        screen_origin.x
    };

    Some(Pos2::new(x, y))
}

fn draw_bezier_wire(
    painter: &egui::Painter,
    p1: Pos2,
    p2: Pos2,
    color: Color32,
    width: f32,
) {
    let dx = (p2.x - p1.x).abs().max(40.0) * 0.5;
    let c1 = p1 + Vec2::new(dx, 0.0);
    let c2 = p2 - Vec2::new(dx, 0.0);

    let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
        [p1, c1, c2, p2],
        false,
        Color32::TRANSPARENT,
        Stroke::new(width, color),
    );
    painter.add(bezier);
}

fn render_wgsl_drawer(
    app: &mut crate::app::Forge3dStudioApp,
    ui: &mut egui::Ui,
    height: f32,
) {
    if app.shader_editor_state.wgsl_dirty || app.shader_editor_state.cached_wgsl.is_empty() {
        app.shader_editor_state.cached_wgsl = app.active_shader_graph.compile_to_wgsl();
        app.shader_editor_state.wgsl_dirty = false;
    }

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📄 Generated WGSL Fragment Shader").strong());

        let copy_label = if let Some(t) = app.shader_editor_state.last_copied_time {
            if t.elapsed().as_secs_f32() < 2.0 {
                "✓ Copied!"
            } else {
                "📋 Copy WGSL Code"
            }
        } else {
            "📋 Copy WGSL Code"
        };

        if ui.button(copy_label).clicked() {
            ui.ctx().copy_text(app.shader_editor_state.cached_wgsl.clone());
            app.shader_editor_state.last_copied_time = Some(Instant::now());
            app.status_message = "WGSL shader code copied to clipboard".into();
        }
    });

    egui::ScrollArea::vertical()
        .max_height(height - 30.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.shader_editor_state.cached_wgsl.as_str())
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });
}
