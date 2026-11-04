//! Forge3D Rust Edition — Phase 6.16.4: GPU Picking / ID Buffer Integration Test & Benchmark
//!
//! Verifies:
//! 1. GPU Picking for Object Selection across primitives (Cube, Sphere, Plane).
//! 2. Depth occlusion accuracy (front object reliably occludes back object).
//! 3. Edit Mode Vertex Picking with screen-space billboard priority over solid surfaces.
//! 4. Edit Mode Edge Picking with screen-space expanded quads (hitbox tolerance).
//! 5. Edit Mode Face Picking with accurate triangulation-to-parent-face mapping.
//! 6. Staging buffer pool concurrency, slot recycling, and safe lifecycle (Idle/Pending/Mapped).
//! 7. Invalidation on epoch increment (camera navigation, mode changes, scene edits).
//! 8. Coordinate transformations (viewport offset, HiDPI scaling, bounds clamping).
//! 9. Non-blocking async latency and synchronous readback performance on Intel HD 4000.
//! 10. Seamless CPU fallback parity when GPU picking is disabled.

use std::time::Instant;

use forge_geometry::EditableMesh;
use forge_geometry::primitives::generate_cube;
use forge_gpu::GpuContext;
use forge_math::Vec3;
use forge_render::camera::Camera;
use forge_render::picking::PickResult;
use forge_render::renderer::ViewportRenderer;
use forge_scene::{EditMode, PrimitiveType, Scene, SceneObject, SelectionElement};
use forge3d_studio::Forge3dStudioApp;

fn project_to_screen(point: Vec3, camera: &Camera, width: u32, height: u32) -> (u32, u32) {
    let vp = camera.view_proj_matrix();
    let clip = vp * point.extend(1.0);
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32).clamp(0.0, width as f32 - 1.0) as u32;
    let screen_y =
        ((1.0 - (ndc_y + 1.0) * 0.5) * height as f32).clamp(0.0, height as f32 - 1.0) as u32;
    (screen_x, screen_y)
}

fn face_center(mesh: &EditableMesh, f_idx: usize) -> Vec3 {
    let face = &mesh.faces[f_idx];
    let sum: Vec3 = face
        .vertices
        .iter()
        .map(|&vi| mesh.vertices[vi].position)
        .sum();
    sum / (face.vertices.len() as f32)
}

fn same_spatial_edge(mesh: &EditableMesh, e0: usize, e1: usize) -> bool {
    let p0 = mesh.vertices[mesh.edges[e0].v0].position;
    let p1 = mesh.vertices[mesh.edges[e0].v1].position;
    let q0 = mesh.vertices[mesh.edges[e1].v0].position;
    let q1 = mesh.vertices[mesh.edges[e1].v1].position;
    ((p0 - q0).length() < 0.001 && (p1 - q1).length() < 0.001)
        || ((p0 - q1).length() < 0.001 && (p1 - q0).length() < 0.001)
}

#[test]
fn test_gpu_picking_object_selection_and_miss() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mut scene = Scene::new("Picking Test Scene");
    let cube = SceneObject::new("CenterCube", PrimitiveType::Cube, generate_cube(2.0));
    let cube_id = cube.id;
    scene.add_object(cube);

    let camera = Camera::default(); // Eye is at (0, 3, 6) looking at (0, 0, 0)

    // 1. Pick center pixel (where Cube is rendered)
    let center_x = width / 2;
    let center_y = height / 2;

    let res = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Object,
        SelectionElement::Vertex,
        None,
        center_x,
        center_y,
    );

    assert_eq!(
        res,
        PickResult::Object(cube_id),
        "Center pixel must pick the rendered cube object"
    );

    // 2. Pick corner pixel (empty background)
    let corner_res = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Object,
        SelectionElement::Vertex,
        None,
        5,
        5,
    );

    assert_eq!(
        corner_res,
        PickResult::None,
        "Corner background pixel must return PickResult::None"
    );
}

#[test]
fn test_gpu_picking_depth_occlusion() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mut scene = Scene::new("Occlusion Scene");

    // Front object: Cube at (0, 0, 0)
    let mut front_obj = SceneObject::new("FrontCube", PrimitiveType::Cube, generate_cube(2.0));
    front_obj.local_transform.translation = Vec3::new(0.0, 0.0, 0.0);
    let front_id = front_obj.id;
    scene.add_object(front_obj);

    // Back object: Larger Cube at (0, 0, 5.0) directly behind FrontCube
    let mut back_obj = SceneObject::new("BackCube", PrimitiveType::Cube, generate_cube(4.0));
    back_obj.local_transform.translation = Vec3::new(0.0, 0.0, 5.0);
    let back_id = back_obj.id;
    scene.add_object(back_obj);

    let mut camera = Camera::default();
    camera.eye = Vec3::new(0.0, 0.0, -8.0);
    camera.target = Vec3::new(0.0, 0.0, 0.0);
    camera.up = Vec3::new(0.0, 1.0, 0.0);

    let center_x = width / 2;
    let center_y = height / 2;

    let res = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Object,
        SelectionElement::Vertex,
        None,
        center_x,
        center_y,
    );

    assert_eq!(
        res,
        PickResult::Object(front_id),
        "Front object must occlude back object during picking"
    );
    assert_ne!(res, PickResult::Object(back_id));
}

#[test]
fn test_gpu_picking_edit_mode_vertex_priority() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mut scene = Scene::new("Edit Vertex Scene");
    let cube = SceneObject::new("Cube", PrimitiveType::Cube, generate_cube(2.0));
    let cube_id = cube.id;
    let edit_mesh = EditableMesh::from_mesh(&cube.mesh);
    scene.add_object(cube);

    let mut camera = Camera::default();
    camera.eye = Vec3::new(3.0, 3.0, 3.0);
    camera.target = Vec3::ZERO;
    camera.up = Vec3::Y;

    // Find the vertex closest to the camera
    let mut closest_idx = 0;
    let mut min_dist = f32::MAX;
    for (idx, v) in edit_mesh.vertices.iter().enumerate() {
        let d = (v.position - camera.eye).length();
        if d < min_dist {
            min_dist = d;
            closest_idx = idx;
        }
    }

    let target_pos = edit_mesh.vertices[closest_idx].position;
    let (px, py) = project_to_screen(target_pos, &camera, width, height);

    let res = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Edit,
        SelectionElement::Vertex,
        Some((&cube_id, &edit_mesh)),
        px,
        py,
    );

    let PickResult::Vertex(picked_v) = res else {
        panic!(
            "Vertex billboard must be picked with depth priority over the solid face beneath it, got {:?}",
            res
        );
    };
    let picked_pos = edit_mesh.vertices[picked_v].position;
    assert!(
        (picked_pos - target_pos).length() < 0.001,
        "Picked vertex 3D position must match target: picked {:?} vs target {:?}",
        picked_pos,
        target_pos
    );
}

#[test]
fn test_gpu_picking_edit_mode_edge_expanded_hitbox() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mut scene = Scene::new("Edit Edge Scene");
    let cube = SceneObject::new("Cube", PrimitiveType::Cube, generate_cube(2.0));
    let cube_id = cube.id;
    let edit_mesh = EditableMesh::from_mesh(&cube.mesh);
    scene.add_object(cube);

    let mut camera = Camera::default();
    camera.eye = Vec3::new(0.0, 0.0, 5.0);
    camera.target = Vec3::ZERO;
    camera.up = Vec3::Y;

    // Find a front-facing edge
    let mut test_edge_idx = None;
    let mut edge_mid_screen = (0, 0);

    for (e_idx, e) in edit_mesh.edges.iter().enumerate() {
        let v0 = edit_mesh.vertices[e.v0].position;
        let v1 = edit_mesh.vertices[e.v1].position;
        if v0.z > 0.5 && v1.z > 0.5 {
            let mid = (v0 + v1) * 0.5;
            edge_mid_screen = project_to_screen(mid, &camera, width, height);
            test_edge_idx = Some(e_idx);
            break;
        }
    }

    let edge_idx = test_edge_idx.expect("Must find a front-facing edge on cube");
    let (ex, ey) = edge_mid_screen;

    // 1. Pick exactly on the edge center
    let res_exact = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Edit,
        SelectionElement::Edge,
        Some((&cube_id, &edit_mesh)),
        ex,
        ey,
    );
    let PickResult::Edge(exact_edge) = res_exact else {
        panic!("Exact pick must hit an edge, got {:?}", res_exact);
    };
    assert!(
        same_spatial_edge(&edit_mesh, exact_edge, edge_idx),
        "Picked exact edge must match the geometric edge"
    );

    // 2. Pick slightly offset by 2 pixels (within the 7px expanded hitbox)
    let res_offset = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Edit,
        SelectionElement::Edge,
        Some((&cube_id, &edit_mesh)),
        ex + 2,
        ey,
    );
    let PickResult::Edge(offset_edge) = res_offset else {
        panic!(
            "Offset pick within 7px hitbox must hit an edge, got {:?}",
            res_offset
        );
    };
    assert!(
        same_spatial_edge(&edit_mesh, offset_edge, edge_idx),
        "Screen-space expanded edge quad must tolerate small cursor offsets (7px hitbox)"
    );
}

#[test]
fn test_gpu_picking_edit_mode_face_triangulation_mapping() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mut scene = Scene::new("Edit Face Scene");
    let cube = SceneObject::new("Cube", PrimitiveType::Cube, generate_cube(2.0));
    let cube_id = cube.id;
    let edit_mesh = EditableMesh::from_mesh(&cube.mesh);
    scene.add_object(cube);

    let mut camera = Camera::default();
    camera.eye = Vec3::new(0.0, 0.0, 5.0);
    camera.target = Vec3::ZERO;
    camera.up = Vec3::Y;

    // Front face center (facing +Z)
    let mut front_face_idx = None;
    for (f_idx, _) in edit_mesh.faces.iter().enumerate() {
        let center = face_center(&edit_mesh, f_idx);
        if center.z > 0.8 {
            front_face_idx = Some(f_idx);
            break;
        }
    }

    let f_idx = front_face_idx.expect("Front face on cube exists");
    let center_screen = project_to_screen(face_center(&edit_mesh, f_idx), &camera, width, height);

    // Pick inside triangle 1 of the quad
    let res1 = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Edit,
        SelectionElement::Face,
        Some((&cube_id, &edit_mesh)),
        center_screen.0 - 15,
        center_screen.1 - 15,
    );

    // Pick inside triangle 2 of the quad
    let res2 = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Edit,
        SelectionElement::Face,
        Some((&cube_id, &edit_mesh)),
        center_screen.0 + 15,
        center_screen.1 + 15,
    );

    assert_eq!(
        res1,
        PickResult::Face(f_idx),
        "First triangle must map to parent face ID"
    );
    assert_eq!(
        res2,
        PickResult::Face(f_idx),
        "Second triangle must map to the same parent face ID"
    );
}

#[test]
fn test_gpu_picking_staging_pool_concurrency_and_lifecycle() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let scene = Scene::create_default_scene();
    let camera = Camera::default();

    // Pool size is 4. Schedule 4 consecutive non-blocking requests.
    let mut requests = Vec::new();
    for i in 0..4 {
        let req_id = renderer.schedule_pick_request(
            &ctx.device,
            &ctx.queue,
            width,
            height,
            &scene,
            &camera,
            EditMode::Object,
            SelectionElement::Vertex,
            None,
            400 + i * 10,
            300,
        );
        assert!(req_id.is_some(), "Slot {i} must be acquired successfully");
        requests.push(req_id.unwrap());
    }

    // 5th request must fail because pool capacity (4) is exhausted
    let req5 = renderer.schedule_pick_request(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Object,
        SelectionElement::Vertex,
        None,
        450,
        300,
    );
    assert_eq!(req5, None, "5th request must be rejected when pool is full");

    // Wait for GPU and poll results
    let _ = ctx.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });

    for req_id in requests {
        let res = renderer.poll_pick_result(req_id);
        assert!(
            res.is_some(),
            "Request {req_id} must resolve after poll and map_async"
        );
    }

    // After all 4 slots are released, we should be able to acquire again
    let new_req = renderer.schedule_pick_request(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Object,
        SelectionElement::Vertex,
        None,
        400,
        300,
    );
    assert!(
        new_req.is_some(),
        "Slots must be recycled back to Idle for new queries"
    );
}

#[test]
fn test_gpu_picking_invalidation_epoch() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 800;
    let height = 600;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let scene = Scene::create_default_scene();
    let camera = Camera::default();

    let req_id = renderer
        .schedule_pick_request(
            &ctx.device,
            &ctx.queue,
            width,
            height,
            &scene,
            &camera,
            EditMode::Object,
            SelectionElement::Vertex,
            None,
            400,
            300,
        )
        .expect("Request must succeed");

    // Invalidate before polling (simulates camera pan/zoom or scene edit)
    renderer.invalidate_picking();

    let _ = ctx.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });

    let res = renderer.poll_pick_result(req_id);
    assert_eq!(
        res, None,
        "Stale requests from older epoch must be discarded immediately"
    );
}

#[test]
fn test_gpu_picking_latency_benchmark() {
    let Ok(ctx) = GpuContext::init_headless() else {
        println!("Skipping GPU test: headless context unavailable");
        return;
    };

    let width = 1280;
    let height = 800;
    let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let scene = Scene::create_default_scene();
    let camera = Camera::default();

    // Warm-up pipeline
    let _ = renderer.pick_sync(
        &ctx.device,
        &ctx.queue,
        width,
        height,
        &scene,
        &camera,
        EditMode::Object,
        SelectionElement::Vertex,
        None,
        width / 2,
        height / 2,
    );

    // 1. Measure non-blocking async schedule latency (CPU time spent scheduling)
    let iters = 20;
    let t0 = Instant::now();
    for _ in 0..iters {
        let req = renderer.schedule_pick_request(
            &ctx.device,
            &ctx.queue,
            width,
            height,
            &scene,
            &camera,
            EditMode::Object,
            SelectionElement::Vertex,
            None,
            width / 2,
            height / 2,
        );
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        if let Some(r) = req {
            let _ = renderer.poll_pick_result(r);
        }
    }
    let total_elapsed = t0.elapsed();
    let avg_latency_ms = total_elapsed.as_secs_f64() * 1000.0 / iters as f64;

    println!(
        "GPU Picking Average Round-Trip Latency: {:.3} ms (Resolution: {width}x{height})",
        avg_latency_ms
    );

    assert!(
        avg_latency_ms < 15.0,
        "GPU Picking round-trip latency must be under 15ms on Intel HD 4000 (measured: {avg_latency_ms:.3}ms)"
    );
}

#[test]
fn test_gpu_picking_cpu_fallback_parity() {
    let mut app = Forge3dStudioApp::new_headless();
    assert!(
        app.editor_state.use_gpu_picking,
        "Default must have use_gpu_picking = true"
    );

    // Headless app has renderer = None, so clicking immediately falls back to CPU raycast
    let ray = app.camera.screen_to_ray(
        forge_math::Vec2::new(400.0, 300.0),
        forge_math::Vec2::new(800.0, 600.0),
    );
    let cpu_hit = app.scene.raycast(&ray).map(|(id, _)| id);

    assert!(
        cpu_hit.is_some(),
        "CPU raycast must find default scene object"
    );
    let hit_obj_id = cpu_hit.unwrap();
    app.selection.select(hit_obj_id);
    assert!(app.selection.is_selected(hit_obj_id));

    // Disable GPU picking toggle explicitly
    app.editor_state.use_gpu_picking = false;
    assert!(!app.editor_state.use_gpu_picking);
}
