//! Forge3D Rust Edition — Phase 6.16.3: GPU Vertex Handles Integration Test & Benchmark
//!
//! Verifies:
//! 1. GPU Vertex Handles rendering on standard primitives (Cube, Sphere, Plane).
//! 2. Scalability on dense meshes (10k, 50k, 100k vertices).
//! 3. Zero vertex buffer uploads during camera navigation (Orbit/Pan/Zoom).
//! 4. In-place state buffer updates (`u32` flags) on vertex selection and active vertex change.
//! 5. Zero 3D position re-uploads when only selection state changes.
//! 6. Independent toggles for GPU Wireframe and GPU Vertex Handles.
//! 7. Fallback to CPU overlay when GPU vertex handles are disabled.
//! 8. Offscreen pixel readback verification on GPU hardware.

use std::time::Instant;

use egui::{Pos2, Rect, Vec2 as EguiVec2};
use forge_geometry::EditableMesh;
use forge_geometry::primitives::{generate_cube, generate_plane, generate_uv_sphere};
use forge_gpu::GpuContext;
use forge_math::{Mat4, Vec3};
use forge_render::EditRenderOptions;
use forge_render::camera::Camera;
use forge_render::renderer::ViewportRenderer;
use forge_scene::{EditMode, PrimitiveType, Scene, SceneObject, SelectionElement, SelectionState};
use forge3d_studio::Forge3dStudioApp;
use forge3d_studio::ui::viewport::render_edit_mode_overlay;

fn make_camera_matrix(eye: Vec3, target: Vec3, aspect: f32) -> Mat4 {
    let view = Mat4::look_at_rh(eye, target, Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 1000.0);
    proj * view
}

#[test]
fn test_gpu_vertex_handles_primitives_lifecycle() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let target_desc = wgpu::TextureDescriptor {
        label: Some("Test Target Lifecycle"),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let target_tex = ctx.device.create_texture(&target_desc);
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let primitives = [
        ("Cube", PrimitiveType::Cube, generate_cube(1.0)),
        (
            "UV Sphere",
            PrimitiveType::Sphere,
            generate_uv_sphere(1.0, 16, 8),
        ),
        ("Plane", PrimitiveType::Plane, generate_plane(2.0, 4)),
    ];

    for (name, prim_type, mesh) in primitives {
        println!("Testing primitive: {name}");
        let mut scene = Scene::new("Test Scene");
        let obj = SceneObject::new(name, prim_type, mesh.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();
        let edit_mesh = EditableMesh::from_mesh(&mesh);

        let options = EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        };

        // 1. Enter Edit Mode -> initial GPU buffer allocation (vertices, edges, state)
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            800,
            600,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );

        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_reallocs, 1,
            "Failed initial vertex buffer allocation for {name}"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_draws, 1,
            "Failed Pass D draw for {name}"
        );
        let buf = renderer
            .edit_mesh_buffers
            .as_ref()
            .expect("Buffers must exist");
        assert_eq!(buf.vert_count, edit_mesh.vertices.len());
        assert_eq!(buf.edge_count, edit_mesh.edges.len());

        // 2. Subsequent frame without changes -> 0 allocations, 0 uploads
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            800,
            600,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0,
            "Static frame must not realloc vertex buffers for {name}"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_uploads, 0,
            "Static frame must not upload vertex buffers for {name}"
        );
        assert_eq!(renderer.last_frame_stats.edit_vertex_draws, 1);

        // 3. Exit Edit Mode -> immediately purged
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            800,
            600,
            &scene,
            &selection,
            &camera,
            false,
            None,
            options,
        );
        assert!(
            renderer.edit_mesh_buffers.is_none(),
            "Exit Edit Mode must purge edit buffers for {name}"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_draws, 0,
            "No vertex handles should be drawn outside Edit Mode"
        );
    }
}

#[test]
fn test_gpu_vertex_handles_scale_benchmark() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let target_desc = wgpu::TextureDescriptor {
        label: Some("Test Target Scale"),
        size: wgpu::Extent3d {
            width: 1280,
            height: 800,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let target_tex = ctx.device.create_texture(&target_desc);
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let test_meshes = [
        ("Plane 10k", generate_plane(10.0, 100)),
        ("Sphere 50k", generate_uv_sphere(2.0, 250, 200)),
        ("Sphere 100k", generate_uv_sphere(2.0, 360, 280)),
    ];

    println!(
        "\n{:<15} | {:<10} | {:<15} | {:<15} | {:<15}",
        "Mesh", "Vertices", "Init Upload(ms)", "Static Pass(ms)", "Camera Orbit(ms)"
    );
    println!("{:-<85}", "");

    for (name, raw_mesh) in test_meshes {
        let v_count = raw_mesh.vertices.len();
        let edit_mesh = EditableMesh::from_mesh(&raw_mesh);

        let mut scene = Scene::new("Test Scene");
        let obj = SceneObject::new(name, PrimitiveType::Plane, raw_mesh);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let mut camera = Camera::default();

        let options = EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        };

        // 1. Initial GPU Buffer Allocation & Upload
        let t0 = Instant::now();
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            1280,
            800,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );
        let init_dur = t0.elapsed();

        assert_eq!(renderer.last_frame_stats.edit_vertex_buffer_reallocs, 1);
        assert_eq!(renderer.last_frame_stats.edit_vertex_draws, 1);

        // 2. Static Render Pass
        let t1 = Instant::now();
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            1280,
            800,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );
        let render_dur = t1.elapsed();

        assert_eq!(renderer.last_frame_stats.edit_vertex_buffer_uploads, 0);
        assert_eq!(renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0);

        // 3. Camera Orbit (Zero buffer uploads!)
        camera.orbit(0.05, 0.05);
        let t2 = Instant::now();
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            1280,
            800,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );
        let orbit_dur = t2.elapsed();

        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_uploads, 0,
            "Camera movement must cause 0 vertex buffer uploads"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0,
            "Camera movement must cause 0 vertex buffer reallocations"
        );
        assert_eq!(renderer.last_frame_stats.edit_vertex_draws, 1);

        println!(
            "{:<15} | {:<10} | {:<15.2} | {:<15.3} | {:<15.3}",
            name,
            v_count,
            init_dur.as_secs_f64() * 1000.0,
            render_dur.as_secs_f64() * 1000.0,
            orbit_dur.as_secs_f64() * 1000.0
        );

        // Cleanup
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            1280,
            800,
            &scene,
            &selection,
            &camera,
            false,
            None,
            options,
        );
        assert!(renderer.edit_mesh_buffers.is_none());
    }
}

#[test]
fn test_gpu_vertex_handles_selection_and_active_state_sync() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let target_desc = wgpu::TextureDescriptor {
        label: Some("Test Target State Sync"),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let target_tex = ctx.device.create_texture(&target_desc);
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let mut scene = Scene::new("Test Scene");
    let cube = generate_cube(1.0);
    let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone());
    let id = obj.id;
    scene.add_object(obj);

    let selection = SelectionState::new();
    let camera = Camera::default();
    let mut edit_mesh = EditableMesh::from_mesh(&cube);

    // Frame 1: Initial upload
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        800,
        600,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        },
    );
    assert_eq!(renderer.last_frame_stats.edit_vertex_buffer_reallocs, 1);

    // Frame 2: Select vertex 2
    edit_mesh.vertices[2].selected = true;
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        800,
        600,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        },
    );

    assert_eq!(
        renderer.last_frame_stats.edit_vertex_buffer_uploads, 1,
        "Selecting vertex must update vertex_state_buffer in-place"
    );
    assert_eq!(
        renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0,
        "Selecting vertex must NOT reallocate buffers"
    );
    assert_eq!(
        renderer.last_frame_stats.edit_edge_buffer_uploads, 0,
        "Selecting vertex must NOT upload edge or position buffers"
    );

    // Frame 3: Designate vertex 2 as active vertex
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        800,
        600,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: Some(2),
        },
    );

    assert_eq!(
        renderer.last_frame_stats.edit_vertex_buffer_uploads, 1,
        "Active vertex change must update vertex_state_buffer in-place"
    );
    assert_eq!(renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0);
    assert_eq!(renderer.last_frame_stats.edit_edge_buffer_uploads, 0);

    // Frame 4: Identical frame -> 0 uploads
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        800,
        600,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: Some(2),
        },
    );
    assert_eq!(renderer.last_frame_stats.edit_vertex_buffer_uploads, 0);
}

#[test]
fn test_gpu_vertex_handles_independent_toggle_and_cpu_fallback() {
    let mut app = Forge3dStudioApp::new_headless();
    assert!(
        app.editor_state.use_gpu_wireframe,
        "use_gpu_wireframe should default to true"
    );
    assert!(
        app.editor_state.use_gpu_vertex_handles,
        "use_gpu_vertex_handles should default to true"
    );

    let raw_mesh = generate_cube(1.0);
    let obj = SceneObject::new("Cube", PrimitiveType::Cube, raw_mesh);
    let id = app.scene.add_object(obj);
    app.selection.select(id);
    app.toggle_edit_mode();
    assert_eq!(app.editor_state.edit_mode, EditMode::Edit);

    let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), EguiVec2::new(800.0, 600.0));
    let view_proj = make_camera_matrix(Vec3::new(3.0, 3.0, 3.0), Vec3::ZERO, 800.0 / 600.0);
    let egui_ctx = egui::Context::default();
    let painter = egui::Painter::new(egui_ctx, egui::LayerId::background(), rect);

    let (_, edit_mesh) = app.active_edit_mesh.as_ref().unwrap();

    // Mode 1: Both GPU Wireframe and GPU Vertex Handles active -> CPU overlay completely bypassed (0 vertices)
    app.editor_state.use_gpu_wireframe = true;
    app.editor_state.use_gpu_vertex_handles = true;
    app.edit_overlay_cache.reset();
    render_edit_mode_overlay(
        &mut app.edit_overlay_cache,
        &painter,
        edit_mesh,
        SelectionElement::Vertex,
        view_proj,
        rect,
        false,
        !app.editor_state.use_gpu_wireframe,
        !app.editor_state.use_gpu_vertex_handles,
    );
    assert_eq!(
        app.edit_overlay_cache.mesh.vertices.len(),
        0,
        "When both GPU wireframe and GPU handles are enabled, CPU overlay must generate 0 vertices"
    );

    // Mode 2: GPU Wireframe ON, GPU Handles OFF -> CPU overlay renders only vertex handles
    app.editor_state.use_gpu_wireframe = true;
    app.editor_state.use_gpu_vertex_handles = false;
    app.edit_overlay_cache.reset();
    render_edit_mode_overlay(
        &mut app.edit_overlay_cache,
        &painter,
        edit_mesh,
        SelectionElement::Vertex,
        view_proj,
        rect,
        false,
        !app.editor_state.use_gpu_wireframe,
        !app.editor_state.use_gpu_vertex_handles,
    );
    let verts_handles_only = app.edit_overlay_cache.mesh.vertices.len();
    assert!(
        verts_handles_only > 0,
        "CPU overlay must generate vertex handle quads when GPU vertex handles disabled"
    );

    // Mode 3: GPU Wireframe OFF, GPU Handles ON -> CPU overlay renders only edges
    app.editor_state.use_gpu_wireframe = false;
    app.editor_state.use_gpu_vertex_handles = true;
    app.edit_overlay_cache.reset();
    render_edit_mode_overlay(
        &mut app.edit_overlay_cache,
        &painter,
        edit_mesh,
        SelectionElement::Vertex,
        view_proj,
        rect,
        false,
        !app.editor_state.use_gpu_wireframe,
        !app.editor_state.use_gpu_vertex_handles,
    );
    let verts_edges_only = app.edit_overlay_cache.mesh.vertices.len();
    assert!(
        verts_edges_only > 0,
        "CPU overlay must generate edge quads when GPU wireframe disabled"
    );

    // Mode 4: Both GPU Wireframe and GPU Handles OFF -> CPU overlay renders both edges and vertices
    app.editor_state.use_gpu_wireframe = false;
    app.editor_state.use_gpu_vertex_handles = false;
    app.edit_overlay_cache.reset();
    render_edit_mode_overlay(
        &mut app.edit_overlay_cache,
        &painter,
        edit_mesh,
        SelectionElement::Vertex,
        view_proj,
        rect,
        false,
        !app.editor_state.use_gpu_wireframe,
        !app.editor_state.use_gpu_vertex_handles,
    );
    let verts_both = app.edit_overlay_cache.mesh.vertices.len();
    assert_eq!(
        verts_both,
        verts_handles_only + verts_edges_only,
        "Combined CPU fallback must equal the sum of edge quads and vertex quads"
    );
}

#[test]
fn test_gpu_vertex_handles_offscreen_pixel_readback() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 128;
    let height = 128;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let target_desc = wgpu::TextureDescriptor {
        label: Some("Test Target Readback"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    let target_tex = ctx.device.create_texture(&target_desc);
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let mut scene = Scene::new("Test Scene");
    let cube = generate_cube(1.0);
    let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone())
        .with_color([0.2, 0.2, 0.2, 1.0]);
    let id = obj.id;
    scene.add_object(obj);

    let selection = SelectionState::new();
    let mut camera = Camera::default();
    camera.eye = Vec3::new(2.0, 2.0, 2.5);
    camera.target = Vec3::ZERO;
    camera.up = Vec3::new(0.0, 1.0, 0.0);

    let mut edit_mesh = EditableMesh::from_mesh(&cube);

    // Helper to read back pixels from GPU
    let read_pixels = || -> Vec<u8> {
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
        let unpadded_bytes = width * 4;
        let padded_bytes = ((unpadded_bytes + align - 1) / align) * align;
        let buffer_size = (padded_bytes * height) as wgpu::BufferAddress;

        let staging = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Readback Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        ctx.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            tx.send(res).unwrap();
        });
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        rx.recv().unwrap().unwrap();

        let data = slice.get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bytes) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        staging.unmap();
        pixels
    };

    // 1. Render in Edit Mode with only wireframe edges (handles OFF)
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        width,
        height,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: false,
            is_vertex_mode: true,
            active_vertex: None,
        },
    );
    let pixels_edges_only = read_pixels();

    // 2. Render with both wireframe edges AND vertex handles (handles ON)
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        width,
        height,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        },
    );
    let pixels_with_handles = read_pixels();

    let diff_count = pixels_edges_only
        .chunks(4)
        .zip(pixels_with_handles.chunks(4))
        .filter(|(a, b)| a != b)
        .count();

    println!("Pixel differences between edges-only and edges+handles: {diff_count}");
    assert!(
        diff_count > 20,
        "GPU vertex handles must rasterize visible billboard markers, found {diff_count} differing pixels"
    );

    // 3. Render with active vertex highlight (vertex 0)
    edit_mesh.vertices[0].selected = true;
    renderer.render_with_edit_mesh(
        &ctx.device,
        &ctx.queue,
        &target_view,
        width,
        height,
        &scene,
        &selection,
        &camera,
        false,
        Some((&id, &edit_mesh)),
        EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: Some(0),
        },
    );
    let pixels_with_active = read_pixels();

    let diff_active = pixels_with_handles
        .chunks(4)
        .zip(pixels_with_active.chunks(4))
        .filter(|(a, b)| a != b)
        .count();

    println!("Pixel differences when selecting active vertex: {diff_active}");
    assert!(
        diff_active > 0,
        "Active vertex highlight must produce distinct pixel colors on screen"
    );
}
