use std::collections::{HashMap, HashSet};

use forge_core::id::ObjectId;
use forge_geometry::primitives::generate_grid_lines;
use forge_geometry::{EditableMesh, LineVertex, Vertex};
use forge_gpu::GpuBuffer;
use forge_math::{Aabb, Mat4, Vec3};
use forge_scene::{
    EditMode, LightData, LightKind, PbrMaterial, Scene, SelectionElement, SelectionState,
};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::picking::{GpuPickingContext, PickResult};
use crate::pipelines::{DEPTH_FORMAT, RenderPipelines};
use crate::textures::{GpuMaterial, create_default_material, create_material};
use crate::uniforms::{CameraUniform, EditModeUniform, ObjectUniform};

const MAX_SCENE_LIGHTS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLight {
    position_type: [f32; 4],
    direction_intensity: [f32; 4],
    color_range: [f32; 4],
    spot_cones: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightingUniform {
    sky_color_intensity: [f32; 4],
    ground_color: [f32; 4],
    light_count: u32,
    shadow_light_index: u32,
    padding: [u32; 2],
    shadow_view_proj: [[f32; 4]; 4],
    lights: [GpuLight; MAX_SCENE_LIGHTS],
}

impl LightingUniform {
    fn from_scene(scene: &Scene) -> Self {
        let mut uniform = Self {
            sky_color_intensity: [
                scene.world_lighting.sky_color[0],
                scene.world_lighting.sky_color[1],
                scene.world_lighting.sky_color[2],
                scene.world_lighting.intensity.max(0.0),
            ],
            ground_color: [
                scene.world_lighting.ground_color[0],
                scene.world_lighting.ground_color[1],
                scene.world_lighting.ground_color[2],
                0.0,
            ],
            light_count: 0,
            shadow_light_index: u32::MAX,
            padding: [0; 2],
            shadow_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            lights: [GpuLight::default(); MAX_SCENE_LIGHTS],
        };

        let mut light_objects: Vec<_> = scene
            .objects
            .iter()
            .filter(|(_, object)| object.primitive_type == forge_scene::PrimitiveType::Light)
            .filter_map(|(&id, object)| {
                object
                    .light_data
                    .filter(|light| light.enabled)
                    .map(|light| (id, light))
            })
            .collect();
        light_objects.sort_by_key(|(id, _)| *id);
        let mut shadow_direction = None;
        for (id, light) in light_objects {
            let index = uniform.light_count as usize;
            if index >= MAX_SCENE_LIGHTS {
                break;
            }
            let transform = scene.world_transform(id);
            let forward = (transform.rotation * Vec3::NEG_Z).normalize_or_zero();
            uniform.lights[index] = gpu_light(light, transform.translation, forward);
            if shadow_direction.is_none() && light.kind == LightKind::Directional {
                uniform.shadow_light_index = index as u32;
                shadow_direction = Some(forward);
            }
            uniform.light_count += 1;
        }
        if let Some(direction) = shadow_direction {
            let mut bounds = Aabb::EMPTY;
            for (&id, object) in &scene.objects {
                if object.visible
                    && object.primitive_type != forge_scene::PrimitiveType::Light
                    && let Some(object_bounds) = scene.world_aabb(id)
                {
                    bounds = bounds.union(&object_bounds);
                }
            }
            if bounds.is_empty() {
                bounds = Aabb::from_center_extents(Vec3::ZERO, Vec3::splat(2.0));
            }
            let center = bounds.center();
            let radius = bounds.extents().length().max(1.0) + 0.5;
            let eye = center - direction * radius * 2.0;
            let up = if direction.dot(Vec3::Y).abs() > 0.95 {
                Vec3::X
            } else {
                Vec3::Y
            };
            let view = Mat4::look_at_rh(eye, center, up);
            let projection =
                Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.1, radius * 4.0);
            uniform.shadow_view_proj = (projection * view).to_cols_array_2d();
        }
        uniform
    }
}

impl Default for GpuLight {
    fn default() -> Self {
        Self {
            position_type: [0.0; 4],
            direction_intensity: [0.0; 4],
            color_range: [0.0; 4],
            spot_cones: [0.0; 4],
        }
    }
}

fn gpu_light(light: LightData, position: Vec3, forward: Vec3) -> GpuLight {
    let kind = match light.kind {
        LightKind::Directional => 0.0,
        LightKind::Point => 1.0,
        LightKind::Spot => 2.0,
    };
    GpuLight {
        position_type: [position.x, position.y, position.z, kind],
        direction_intensity: [forward.x, forward.y, forward.z, light.intensity.max(0.0)],
        color_range: [
            light.color[0].max(0.0),
            light.color[1].max(0.0),
            light.color[2].max(0.0),
            light.range.max(0.001),
        ],
        spot_cones: [
            light.inner_cone_angle_deg.to_radians().cos(),
            light.outer_cone_angle_deg.to_radians().cos(),
            0.0,
            0.0,
        ],
    }
}

struct GpuMeshEntry {
    vertex_buffer: GpuBuffer<Vertex>,
    index_buffer: GpuBuffer<u32>,
    object_uniform_buffer: wgpu::Buffer,
    object_bind_group: wgpu::BindGroup,
    generation: u64,
    cached_uniform: ObjectUniform,
}

/// Options controlling Edit Mode wireframe and vertex handles rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditRenderOptions {
    pub draw_edges: bool,
    pub draw_vertices: bool,
    pub is_vertex_mode: bool,
    pub active_vertex: Option<usize>,
}

impl Default for EditRenderOptions {
    fn default() -> Self {
        Self {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        }
    }
}

/// Compute a 64-bit fingerprint of vertex selection and active vertex state.
pub fn compute_selection_hash(edit_mesh: &EditableMesh, active_vertex: Option<usize>) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for (i, v) in edit_mesh.vertices.iter().enumerate() {
        if v.selected {
            hash ^= (i as u64).wrapping_mul(0x517cc1b727220a95);
            hash = hash.rotate_left(13).wrapping_add(1);
        }
    }
    if let Some(av) = active_vertex {
        hash ^= (av as u64).wrapping_mul(0x9e3779b97f4a7c15);
        hash = hash.rotate_left(17).wrapping_add(2);
    }
    hash
}

/// GPU buffer allocation for active Edit Mode wireframe mesh and vertex handles.
pub struct GpuEditMeshBuffers {
    pub object_id: ObjectId,
    pub vertex_buffer: GpuBuffer<[f32; 3]>,
    pub index_buffer: GpuBuffer<u32>,
    pub vertex_state_buffer: GpuBuffer<u32>,
    pub generation: u64,
    pub vert_count: usize,
    pub edge_count: usize,
    pub selection_hash: u64,
}

impl GpuEditMeshBuffers {
    pub fn new(
        device: &wgpu::Device,
        object_id: ObjectId,
        positions: &[[f32; 3]],
        indices: &[u32],
        flags: &[u32],
        generation: u64,
        selection_hash: u64,
    ) -> Self {
        let vertex_buffer =
            GpuBuffer::new_vertex_buffer(device, "Edit Mode Vertex Buffer", positions);
        let index_buffer =
            GpuBuffer::new_index_buffer(device, "Edit Mode Edge Index Buffer", indices);
        let vertex_state_buffer =
            GpuBuffer::new_vertex_buffer(device, "Edit Mode Vertex State Buffer", flags);
        Self {
            object_id,
            vertex_buffer,
            index_buffer,
            vertex_state_buffer,
            generation,
            vert_count: positions.len(),
            edge_count: indices.len() / 2,
            selection_hash,
        }
    }
}

/// Statistics collected during a single viewport render pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RenderStats {
    pub vertex_buffer_uploads: usize,
    pub vertex_buffer_reallocs: usize,
    pub index_buffer_uploads: usize,
    pub index_buffer_reallocs: usize,
    pub uniform_buffer_uploads: usize,
    pub drawn_objects: usize,
    pub edit_edge_buffer_uploads: usize,
    pub edit_edge_buffer_reallocs: usize,
    pub edit_edge_draws: usize,
    pub edit_vertex_buffer_uploads: usize,
    pub edit_vertex_buffer_reallocs: usize,
    pub edit_vertex_draws: usize,
}

/// The main 3D Viewport Renderer orchestrating pipelines, GPU buffers, depth testing, and passes.
pub struct ViewportRenderer {
    pub pipelines: RenderPipelines,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    lighting_buffer: wgpu::Buffer,
    lighting_bind_group: wgpu::BindGroup,
    shadow_camera_buffer: wgpu::Buffer,
    shadow_camera_bind_group: wgpu::BindGroup,
    _shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    _shadow_sampler: wgpu::Sampler,
    pub grid_buffer: GpuBuffer<LineVertex>,
    pub unit_quad_vertex_buffer: GpuBuffer<[f32; 2]>,
    pub unit_quad_index_buffer: GpuBuffer<u32>,
    pub edit_uniform_buffer: wgpu::Buffer,
    pub edit_bind_group: wgpu::BindGroup,
    cached_edit_uniform: EditModeUniform,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    depth_size: (u32, u32),
    mesh_cache: HashMap<ObjectId, GpuMeshEntry>,
    default_material: Option<GpuMaterial>,
    scene_materials: HashMap<String, (PbrMaterial, GpuMaterial)>,
    failed_materials: HashMap<String, PbrMaterial>,
    pub edit_mesh_buffers: Option<GpuEditMeshBuffers>,
    pub picking_ctx: GpuPickingContext,
    pub last_frame_stats: RenderStats,
}

impl ViewportRenderer {
    /// Creates a new viewport renderer for the given target surface/texture format.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let pipelines = RenderPipelines::new(device, target_format);

        // Initial camera uniform buffer
        let dummy_cam = CameraUniform::new(forge_math::Mat4::IDENTITY, Vec3::ZERO);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::bytes_of(&dummy_cam),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &pipelines.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let initial_lighting = LightingUniform::from_scene(&Scene::default());
        let lighting_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Lighting Uniform Buffer"),
            contents: bytemuck::bytes_of(&initial_lighting),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Directional Shadow Map"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Directional Shadow Comparison Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Lighting Bind Group"),
            layout: &pipelines.lighting_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lighting_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        let shadow_camera_uniform = CameraUniform::new(Mat4::IDENTITY, Vec3::ZERO);
        let shadow_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shadow Camera Uniform Buffer"),
            contents: bytemuck::bytes_of(&shadow_camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shadow_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Camera Bind Group"),
            layout: &pipelines.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_camera_buffer.as_entire_binding(),
            }],
        });

        // Generate ground grid lines (size: 20x20m, step: 1m)
        let grid_lines = generate_grid_lines(10.0, 1.0);
        let grid_buffer = GpuBuffer::new_vertex_buffer(device, "Grid Vertex Buffer", &grid_lines);

        // Static billboard unit quad geometry ([-1..1] 2D coordinates)
        let quad_vertices: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let quad_indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let unit_quad_vertex_buffer =
            GpuBuffer::new_vertex_buffer(device, "Unit Quad Vertex Buffer", &quad_vertices);
        let unit_quad_index_buffer =
            GpuBuffer::new_index_buffer(device, "Unit Quad Index Buffer", &quad_indices);

        // Edit Mode configuration uniform buffer (Group 2)
        let cached_edit_uniform = EditModeUniform::new([800.0, 600.0], 6.0, true);
        let edit_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edit Mode Uniform Buffer"),
            contents: bytemuck::bytes_of(&cached_edit_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let edit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Edit Bind Group"),
            layout: &pipelines.edit_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: edit_uniform_buffer.as_entire_binding(),
            }],
        });

        let picking_ctx = GpuPickingContext::new(device);

        Self {
            pipelines,
            camera_buffer,
            camera_bind_group,
            lighting_buffer,
            lighting_bind_group,
            shadow_camera_buffer,
            shadow_camera_bind_group,
            _shadow_texture: shadow_texture,
            shadow_view,
            _shadow_sampler: shadow_sampler,
            grid_buffer,
            unit_quad_vertex_buffer,
            unit_quad_index_buffer,
            edit_uniform_buffer,
            edit_bind_group,
            cached_edit_uniform,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            mesh_cache: HashMap::new(),
            default_material: None,
            scene_materials: HashMap::new(),
            failed_materials: HashMap::new(),
            edit_mesh_buffers: None,
            picking_ctx,
            last_frame_stats: RenderStats::default(),
        }
    }

    /// Invalidate GPU cache for a specific object mesh.
    pub fn invalidate_mesh(&mut self, id: &ObjectId) {
        self.mesh_cache.remove(id);
        self.picking_ctx.invalidate_mesh(id);
    }

    /// Clear all cached GPU meshes.
    pub fn invalidate_all_meshes(&mut self) {
        self.mesh_cache.clear();
        self.edit_mesh_buffers = None;
        self.picking_ctx.invalidate_all();
    }

    /// Returns true if the GPU mesh cache contains an entry for the given object.
    pub fn has_cached_mesh(&self, id: &ObjectId) -> bool {
        self.mesh_cache.contains_key(id)
    }

    /// Re-allocates the depth texture if the viewport dimensions change.
    fn ensure_depth_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.depth_size != (width, height) || self.depth_texture.is_none() {
            self.picking_ctx.invalidate();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Viewport Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_texture = Some(texture);
            self.depth_view = Some(view);
            self.depth_size = (width, height);
        }
    }

    /// Schedules an asynchronous picking request for a single pixel (screen coordinates transformed to texture).
    #[allow(clippy::too_many_arguments)]
    pub fn schedule_pick_request(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        scene: &Scene,
        camera: &Camera,
        edit_mode: EditMode,
        selection_element: SelectionElement,
        active_edit_mesh: Option<(&ObjectId, &EditableMesh)>,
        pixel_x: u32,
        pixel_y: u32,
    ) -> Option<u64> {
        self.picking_ctx.schedule_pick_request(
            device,
            queue,
            width,
            height,
            scene,
            camera,
            edit_mode,
            selection_element,
            active_edit_mesh,
            pixel_x,
            pixel_y,
        )
    }

    /// Polls a pending picking request. Returns None if still pending or invalidated.
    pub fn poll_pick_result(&mut self, request_id: u64) -> Option<PickResult> {
        self.picking_ctx.poll_pick_result(request_id)
    }

    /// Synchronous picking query (waits for GPU completion).
    #[allow(clippy::too_many_arguments)]
    pub fn pick_sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        scene: &Scene,
        camera: &Camera,
        edit_mode: EditMode,
        selection_element: SelectionElement,
        active_edit_mesh: Option<(&ObjectId, &EditableMesh)>,
        pixel_x: u32,
        pixel_y: u32,
    ) -> PickResult {
        self.picking_ctx.pick_sync(
            device,
            queue,
            width,
            height,
            scene,
            camera,
            edit_mode,
            selection_element,
            active_edit_mesh,
            pixel_x,
            pixel_y,
        )
    }

    /// Invalidate any pending picking queries (e.g. when camera moves or active object changes).
    pub fn invalidate_picking(&mut self) {
        self.picking_ctx.invalidate();
    }

    /// Renders the scene into the specified target texture view.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        viewport_width: u32,
        viewport_height: u32,
        scene: &Scene,
        selection: &SelectionState,
        camera: &Camera,
        show_grid: bool,
    ) {
        self.render_with_edit_mesh(
            device,
            queue,
            target_view,
            viewport_width,
            viewport_height,
            scene,
            selection,
            camera,
            show_grid,
            None,
            EditRenderOptions::default(),
        );
    }

    /// Renders the scene and optional GPU Edit Mode wireframe and vertex handles into the specified target texture view.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_edit_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        viewport_width: u32,
        viewport_height: u32,
        scene: &Scene,
        selection: &SelectionState,
        camera: &Camera,
        show_grid: bool,
        active_edit_mesh: Option<(&ObjectId, &EditableMesh)>,
        options: EditRenderOptions,
    ) {
        if viewport_width == 0 || viewport_height == 0 {
            return;
        }

        if self.default_material.is_none() {
            self.default_material = Some(create_default_material(
                device,
                queue,
                &self.pipelines.material_bind_group_layout,
            ));
        }
        let needed_materials: HashSet<&str> = scene
            .objects
            .values()
            .filter(|object| object.visible)
            .filter_map(|object| object.material_id.as_deref())
            .collect();
        self.scene_materials
            .retain(|id, _| scene.materials.contains_key(id));
        self.failed_materials
            .retain(|id, definition| scene.materials.get(id) == Some(definition));
        for id in needed_materials {
            let Some(definition) = scene.materials.get(id) else {
                continue;
            };
            let recreate_textures = self
                .scene_materials
                .get(id)
                .is_none_or(|(cached, _)| cached.textures != definition.textures);
            if recreate_textures {
                if self.failed_materials.get(id) == Some(definition) {
                    continue;
                }
                match create_material(
                    device,
                    queue,
                    &self.pipelines.material_bind_group_layout,
                    definition,
                ) {
                    Ok((gpu_material, resolution, gpu_bytes)) => {
                        tracing::info!(
                            material = %definition.name,
                            resolution = ?resolution,
                            gpu_mib = gpu_bytes as f64 / (1024.0 * 1024.0),
                            "Loaded PBR material maps"
                        );
                        self.scene_materials
                            .insert(id.to_string(), (definition.clone(), gpu_material));
                        self.failed_materials.remove(id);
                    }
                    Err(error) => {
                        tracing::warn!(material = %definition.name, "PBR material maps failed to load: {error}");
                        self.failed_materials
                            .insert(id.to_string(), definition.clone());
                    }
                }
            } else if let Some((cached, gpu_material)) = self.scene_materials.get_mut(id)
                && cached != definition
            {
                let uniform = crate::textures::material_uniform(definition);
                queue.write_buffer(
                    &gpu_material.uniform_buffer,
                    0,
                    bytemuck::bytes_of(&uniform),
                );
                *cached = definition.clone();
            }
        }

        let mut stats = RenderStats::default();

        // 1. Ensure depth texture matches viewport size
        self.ensure_depth_texture(device, viewport_width, viewport_height);

        // 2. Update Camera Uniform
        let cam_uniform = CameraUniform::new(camera.view_proj_matrix(), camera.eye);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cam_uniform));
        let lighting = LightingUniform::from_scene(scene);
        queue.write_buffer(&self.lighting_buffer, 0, bytemuck::bytes_of(&lighting));
        let shadow_camera_uniform = CameraUniform {
            view_proj: lighting.shadow_view_proj,
            camera_pos: [0.0; 4],
        };
        queue.write_buffer(
            &self.shadow_camera_buffer,
            0,
            bytemuck::bytes_of(&shadow_camera_uniform),
        );

        // 3. Sync GPU mesh cache with scene objects
        self.mesh_cache
            .retain(|id, _| scene.objects.contains_key(id));

        for (&id, obj) in &scene.objects {
            if !obj.visible || obj.primitive_type == forge_scene::PrimitiveType::Light {
                continue;
            }

            let world_tf = scene.world_transform(id);
            let is_selected = selection.is_selected(id);
            let obj_uniform = ObjectUniform::new(world_tf.to_matrix(), obj.color, is_selected);

            if let Some(entry) = self.mesh_cache.get_mut(&id) {
                // If mesh geometry or modifiers changed, re-evaluate and re-upload buffers
                if entry.generation != obj.generation {
                    let eval_mesh = obj.evaluated_mesh();

                    let v_count_changed = entry.vertex_buffer.count != eval_mesh.vertices.len();
                    if v_count_changed {
                        entry.vertex_buffer = GpuBuffer::new_vertex_buffer(
                            device,
                            &format!("{}_vbuf", obj.name),
                            &eval_mesh.vertices,
                        );
                        stats.vertex_buffer_reallocs += 1;
                    } else {
                        queue.write_buffer(
                            &entry.vertex_buffer.buffer,
                            0,
                            bytemuck::cast_slice(&eval_mesh.vertices),
                        );
                        stats.vertex_buffer_uploads += 1;
                    }

                    let i_count_changed = entry.index_buffer.count != eval_mesh.indices.len();
                    if i_count_changed {
                        entry.index_buffer = GpuBuffer::new_index_buffer(
                            device,
                            &format!("{}_ibuf", obj.name),
                            &eval_mesh.indices,
                        );
                        stats.index_buffer_reallocs += 1;
                    } else {
                        queue.write_buffer(
                            &entry.index_buffer.buffer,
                            0,
                            bytemuck::cast_slice(&eval_mesh.indices),
                        );
                        stats.index_buffer_uploads += 1;
                    }

                    entry.generation = obj.generation;
                }

                // Update object uniform only if transform, color, or selection changed
                if entry.cached_uniform != obj_uniform {
                    queue.write_buffer(
                        &entry.object_uniform_buffer,
                        0,
                        bytemuck::bytes_of(&obj_uniform),
                    );
                    entry.cached_uniform = obj_uniform;
                    stats.uniform_buffer_uploads += 1;
                }
            } else {
                // Upload new mesh to GPU
                let eval_mesh = obj.evaluated_mesh();
                let v_buf = GpuBuffer::new_vertex_buffer(
                    device,
                    &format!("{}_vbuf", obj.name),
                    &eval_mesh.vertices,
                );
                let i_buf = GpuBuffer::new_index_buffer(
                    device,
                    &format!("{}_ibuf", obj.name),
                    &eval_mesh.indices,
                );
                stats.vertex_buffer_reallocs += 1;
                stats.index_buffer_reallocs += 1;

                let u_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{}_ubuf", obj.name)),
                    contents: bytemuck::bytes_of(&obj_uniform),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
                stats.uniform_buffer_uploads += 1;

                let b_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("{}_bgroup", obj.name)),
                    layout: &self.pipelines.object_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: u_buf.as_entire_binding(),
                    }],
                });

                self.mesh_cache.insert(
                    id,
                    GpuMeshEntry {
                        vertex_buffer: v_buf,
                        index_buffer: i_buf,
                        object_uniform_buffer: u_buf,
                        object_bind_group: b_group,
                        generation: obj.generation,
                        cached_uniform: obj_uniform,
                    },
                );
            }
        }

        // 3.5. Sync GPU Edit Mode wireframe and vertex buffers
        if (options.draw_edges || options.draw_vertices)
            && let Some((&edit_id, edit_mesh)) = active_edit_mesh
        {
            // Update Edit Mode uniform buffer (Group 2)
            let edit_uniform = EditModeUniform::new(
                [viewport_width as f32, viewport_height as f32],
                6.0,
                options.is_vertex_mode,
            );
            if edit_uniform != self.cached_edit_uniform {
                queue.write_buffer(
                    &self.edit_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&edit_uniform),
                );
                self.cached_edit_uniform = edit_uniform;
                stats.uniform_buffer_uploads += 1;
            }

            let obj = scene.get_object(edit_id);
            let obj_gen = obj.map(|o| o.generation).unwrap_or(0);
            let sel_hash = compute_selection_hash(edit_mesh, options.active_vertex);

            let need_new_buffers = match &self.edit_mesh_buffers {
                Some(buf) => buf.object_id != edit_id,
                None => true,
            };

            if need_new_buffers {
                let positions: Vec<[f32; 3]> = edit_mesh
                    .vertices
                    .iter()
                    .map(|v| v.position.to_array())
                    .collect();
                let indices: Vec<u32> = edit_mesh
                    .edges
                    .iter()
                    .flat_map(|e| [e.v0 as u32, e.v1 as u32])
                    .collect();
                let flags: Vec<u32> = edit_mesh
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let mut f = if v.selected { 1u32 } else { 0u32 };
                        if options.active_vertex == Some(i) {
                            f |= 2u32;
                        }
                        f
                    })
                    .collect();

                self.edit_mesh_buffers = Some(GpuEditMeshBuffers::new(
                    device, edit_id, &positions, &indices, &flags, obj_gen, sel_hash,
                ));
                stats.edit_edge_buffer_reallocs += 1;
                stats.edit_vertex_buffer_reallocs += 1;
            } else if let Some(buf) = &mut self.edit_mesh_buffers {
                let v_len = edit_mesh.vertices.len();
                let e_len = edit_mesh.edges.len();

                let topo_changed = e_len != buf.edge_count;
                let vert_count_changed = v_len != buf.vert_count;

                if topo_changed {
                    let indices: Vec<u32> = edit_mesh
                        .edges
                        .iter()
                        .flat_map(|e| [e.v0 as u32, e.v1 as u32])
                        .collect();
                    buf.index_buffer = GpuBuffer::new_index_buffer(
                        device,
                        "Edit Mode Edge Index Buffer",
                        &indices,
                    );
                    buf.edge_count = e_len;
                    stats.edit_edge_buffer_reallocs += 1;
                }

                if vert_count_changed {
                    let positions: Vec<[f32; 3]> = edit_mesh
                        .vertices
                        .iter()
                        .map(|v| v.position.to_array())
                        .collect();
                    let flags: Vec<u32> = edit_mesh
                        .vertices
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let mut f = if v.selected { 1u32 } else { 0u32 };
                            if options.active_vertex == Some(i) {
                                f |= 2u32;
                            }
                            f
                        })
                        .collect();

                    buf.vertex_buffer =
                        GpuBuffer::new_vertex_buffer(device, "Edit Mode Vertex Buffer", &positions);
                    buf.vertex_state_buffer = GpuBuffer::new_vertex_buffer(
                        device,
                        "Edit Mode Vertex State Buffer",
                        &flags,
                    );
                    buf.vert_count = v_len;
                    buf.generation = obj_gen;
                    buf.selection_hash = sel_hash;
                    stats.edit_edge_buffer_reallocs += 1;
                    stats.edit_vertex_buffer_reallocs += 1;
                } else {
                    if buf.generation != obj_gen {
                        let positions: Vec<[f32; 3]> = edit_mesh
                            .vertices
                            .iter()
                            .map(|v| v.position.to_array())
                            .collect();
                        queue.write_buffer(
                            &buf.vertex_buffer.buffer,
                            0,
                            bytemuck::cast_slice(&positions),
                        );
                        buf.generation = obj_gen;
                        stats.edit_edge_buffer_uploads += 1;
                    }

                    if buf.selection_hash != sel_hash {
                        let flags: Vec<u32> = edit_mesh
                            .vertices
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                let mut f = if v.selected { 1u32 } else { 0u32 };
                                if options.active_vertex == Some(i) {
                                    f |= 2u32;
                                }
                                f
                            })
                            .collect();
                        queue.write_buffer(
                            &buf.vertex_state_buffer.buffer,
                            0,
                            bytemuck::cast_slice(&flags),
                        );
                        buf.selection_hash = sel_hash;
                        stats.edit_vertex_buffer_uploads += 1;
                    }
                }
            }
        } else if self.edit_mesh_buffers.is_some() {
            self.edit_mesh_buffers = None;
            self.picking_ctx.invalidate();
        }

        // 4. Command Encoder & Render Pass
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Viewport Render Encoder"),
        });

        if lighting.shadow_light_index != u32::MAX {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Directional Shadow Map Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            shadow_pass.set_pipeline(&self.pipelines.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.shadow_camera_bind_group, &[]);
            for (&id, object) in &scene.objects {
                if !object.visible || object.primitive_type == forge_scene::PrimitiveType::Light {
                    continue;
                }
                if let Some(entry) = self.mesh_cache.get(&id) {
                    shadow_pass.set_bind_group(1, &entry.object_bind_group, &[]);
                    shadow_pass.set_vertex_buffer(0, entry.vertex_buffer.buffer.slice(..));
                    shadow_pass.set_index_buffer(
                        entry.index_buffer.buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    shadow_pass.draw_indexed(0..entry.index_buffer.count as u32, 0, 0..1);
                }
            }
        }

        {
            let depth_view = self
                .depth_view
                .as_ref()
                .expect("Depth view must be initialized");

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Viewport 3D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        // Dark graphite background (theme #1e2024)
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.13,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Pass A: Render ground grid lines
            if show_grid {
                pass.set_pipeline(&self.pipelines.grid_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.grid_buffer.buffer.slice(..));
                pass.draw(0..self.grid_buffer.count as u32, 0..1);
            }

            // Pass B: Render 3D objects with depth test & lighting
            pass.set_pipeline(&self.pipelines.mesh_pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(3, &self.lighting_bind_group, &[]);

            for (&id, obj) in &scene.objects {
                if !obj.visible || obj.primitive_type == forge_scene::PrimitiveType::Light {
                    continue;
                }

                if let Some(entry) = self.mesh_cache.get(&id) {
                    pass.set_bind_group(1, &entry.object_bind_group, &[]);
                    let material = obj
                        .material_id
                        .as_ref()
                        .and_then(|id| self.scene_materials.get(id).map(|(_, mat)| mat))
                        .or(self.default_material.as_ref())
                        .expect("default viewport material must be initialized");
                    pass.set_bind_group(2, &material.bind_group, &[]);
                    pass.set_vertex_buffer(0, entry.vertex_buffer.buffer.slice(..));
                    pass.set_index_buffer(
                        entry.index_buffer.buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..entry.index_buffer.count as u32, 0, 0..1);
                    stats.drawn_objects += 1;
                }
            }

            // Pass C: Render Edit Mode Wireframe Edges
            if options.draw_edges
                && let Some((&edit_id, _)) = active_edit_mesh
                && let Some(edit_buffers) = &self.edit_mesh_buffers
                && edit_buffers.object_id == edit_id
                && edit_buffers.index_buffer.count > 0
                && let Some(entry) = self.mesh_cache.get(&edit_id)
            {
                pass.set_pipeline(&self.pipelines.edit_edge_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &entry.object_bind_group, &[]);
                pass.set_vertex_buffer(0, edit_buffers.vertex_buffer.buffer.slice(..));
                pass.set_index_buffer(
                    edit_buffers.index_buffer.buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..edit_buffers.index_buffer.count as u32, 0, 0..1);
                stats.edit_edge_draws += 1;
            }

            // Pass D: Render Edit Mode Vertex Handles
            if options.draw_vertices
                && let Some((&edit_id, _)) = active_edit_mesh
                && let Some(edit_buffers) = &self.edit_mesh_buffers
                && edit_buffers.object_id == edit_id
                && edit_buffers.vert_count > 0
                && let Some(entry) = self.mesh_cache.get(&edit_id)
            {
                pass.set_pipeline(&self.pipelines.edit_vertex_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &entry.object_bind_group, &[]);
                pass.set_bind_group(2, &self.edit_bind_group, &[]);
                pass.set_vertex_buffer(0, self.unit_quad_vertex_buffer.buffer.slice(..));
                pass.set_vertex_buffer(1, edit_buffers.vertex_buffer.buffer.slice(..));
                pass.set_vertex_buffer(2, edit_buffers.vertex_state_buffer.buffer.slice(..));
                pass.set_index_buffer(
                    self.unit_quad_index_buffer.buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..6, 0, 0..edit_buffers.vert_count as u32);
                stats.edit_vertex_draws += 1;
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        self.last_frame_stats = stats;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::{generate_cube, generate_uv_sphere};
    use forge_geometry::{EditEdge, EditableMesh};
    use forge_gpu::GpuContext;
    use forge_modifiers::{ModifierType, SubdivisionModifier};
    use forge_scene::{
        Action, HistoryStack, LightData, LightKind, PrimitiveType, Scene, SceneObject,
        SelectionState,
    };

    fn setup_test_env() -> Option<(GpuContext, ViewportRenderer, wgpu::TextureView)> {
        let ctx = GpuContext::init_headless().ok()?;
        let renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let target_desc = wgpu::TextureDescriptor {
            label: Some("Test Target"),
            size: wgpu::Extent3d {
                width: 100,
                height: 100,
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
        Some((ctx, renderer, target_view))
    }

    #[test]
    fn scene_lights_pack_all_supported_kinds_and_respect_visibility() {
        let mut scene = Scene::new("Lighting test");
        for (name, kind) in [
            ("Sun", LightKind::Directional),
            ("Bulb", LightKind::Point),
            ("Spot", LightKind::Spot),
        ] {
            scene.add_object(SceneObject::new_light(
                name,
                LightData {
                    kind,
                    intensity: 5.0,
                    ..LightData::default()
                },
                generate_uv_sphere(0.1, 8, 6),
            ));
        }

        let uniform = LightingUniform::from_scene(&scene);
        assert_eq!(uniform.light_count, 3);
        let mut kinds: Vec<_> = uniform.lights[..3]
            .iter()
            .map(|light| light.position_type[3] as u8)
            .collect();
        kinds.sort_unstable();
        assert_eq!(kinds, [0, 1, 2]);
        assert!(
            uniform.lights[..3]
                .iter()
                .all(|light| light.direction_intensity[3] == 5.0)
        );

        let first_id = *scene.objects.keys().next().unwrap();
        scene
            .objects
            .get_mut(&first_id)
            .unwrap()
            .light_data
            .as_mut()
            .unwrap()
            .enabled = false;
        assert_eq!(LightingUniform::from_scene(&scene).light_count, 2);
    }

    #[test]
    fn test_renderer_static_mesh_zero_buffer_updates() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();

        // Frame 1: Initial upload
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(renderer.last_frame_stats.vertex_buffer_reallocs, 1);
        assert_eq!(renderer.last_frame_stats.index_buffer_reallocs, 1);
        assert_eq!(renderer.last_frame_stats.uniform_buffer_uploads, 1);

        // Frame 2: Static mesh re-render (Zero updates!)
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(
            renderer.last_frame_stats.vertex_buffer_uploads, 0,
            "Static mesh must not upload vertices"
        );
        assert_eq!(
            renderer.last_frame_stats.vertex_buffer_reallocs, 0,
            "Static mesh must not realloc vertices"
        );
        assert_eq!(
            renderer.last_frame_stats.index_buffer_uploads, 0,
            "Static mesh must not upload indices"
        );
        assert_eq!(
            renderer.last_frame_stats.index_buffer_reallocs, 0,
            "Static mesh must not realloc indices"
        );
        assert_eq!(
            renderer.last_frame_stats.uniform_buffer_uploads, 0,
            "Static object must not upload uniform"
        );
        assert_eq!(renderer.last_frame_stats.drawn_objects, 1);
    }

    #[test]
    fn test_renderer_transform_update_independent_of_geometry() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();

        // Frame 1: Initial render
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );

        // Frame 2: Move object in world space (transform changed, geometry untouched)
        let obj_mut = scene.get_object_mut(id).unwrap();
        obj_mut.local_transform.translation = forge_math::Vec3::new(10.0, 5.0, -2.0);

        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(
            renderer.last_frame_stats.uniform_buffer_uploads, 1,
            "Uniform must update on transform change"
        );
        assert_eq!(
            renderer.last_frame_stats.vertex_buffer_uploads, 0,
            "Geometry must not be uploaded on transform change"
        );
        assert_eq!(renderer.last_frame_stats.vertex_buffer_reallocs, 0);
        assert_eq!(renderer.last_frame_stats.index_buffer_uploads, 0);
        assert_eq!(renderer.last_frame_stats.index_buffer_reallocs, 0);
    }

    #[test]
    fn test_renderer_geometry_change_in_place_without_realloc() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();

        // Frame 1: Initial render
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );

        // Frame 2: Deform vertices in-place (same vertex count, modified positions)
        let obj_mut = scene.get_object_mut(id).unwrap();
        obj_mut.mesh_mut().vertices[0].position[0] += 2.0;

        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(
            renderer.last_frame_stats.vertex_buffer_uploads, 1,
            "In-place vertex update must write to existing buffer"
        );
        assert_eq!(
            renderer.last_frame_stats.vertex_buffer_reallocs, 0,
            "In-place vertex update must NOT reallocate buffer"
        );
        assert_eq!(renderer.last_frame_stats.index_buffer_uploads, 1);
        assert_eq!(renderer.last_frame_stats.index_buffer_reallocs, 0);
    }

    #[test]
    fn test_renderer_vertex_and_index_count_change_reallocates() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();

        // Frame 1: Initial render of base cube
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );

        // Frame 2: Add Subdivision modifier (increases vertex and triangle count)
        let obj_mut = scene.get_object_mut(id).unwrap();
        obj_mut
            .modifiers_mut()
            .add(ModifierType::Subdivide(SubdivisionModifier::new(1)));

        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(
            renderer.last_frame_stats.vertex_buffer_reallocs, 1,
            "Count change must reallocate vertex buffer"
        );
        assert_eq!(
            renderer.last_frame_stats.index_buffer_reallocs, 1,
            "Count change must reallocate index buffer"
        );
        assert!(renderer.mesh_cache.get(&id).unwrap().vertex_buffer.count > 24);
    }

    #[test]
    fn test_renderer_object_deletion_cleans_gpu_cache() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();

        // Frame 1: Render object
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert!(renderer.mesh_cache.contains_key(&id));
        assert_eq!(renderer.last_frame_stats.drawn_objects, 1);

        // Frame 2: Remove object from scene
        scene.remove_object(id);
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert!(
            !renderer.mesh_cache.contains_key(&id),
            "Deleted object must be purged from mesh_cache"
        );
        assert_eq!(renderer.last_frame_stats.drawn_objects, 0);
    }

    #[test]
    fn test_showcase_uploads_polyhaven_brown_floor_tiles_material() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let scene = Scene::create_showcase_scene();
        let selection = SelectionState::new();
        let camera = Camera::default();
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );

        assert!(
            renderer
                .scene_materials
                .contains_key(forge_scene::BROWN_FLOOR_TILES_MATERIAL_ID),
            "the showcase must upload its scene-defined PBR material"
        );
        assert!(
            !renderer
                .failed_materials
                .contains_key(forge_scene::BROWN_FLOOR_TILES_MATERIAL_ID)
        );
        for id in [
            forge_scene::BROWN_FLOOR_TILES_MATERIAL_ID,
            "polyhaven/bark_brown_01",
            "polyhaven/wooden_planks",
            "polyhaven/mossy_rock",
        ] {
            assert!(renderer.scene_materials.contains_key(id));
            assert!(!renderer.failed_materials.contains_key(id));
        }
        let visible_meshes = scene
            .objects
            .values()
            .filter(|object| {
                object.visible && object.primitive_type != forge_scene::PrimitiveType::Light
            })
            .count();
        assert_eq!(renderer.last_frame_stats.drawn_objects, visible_meshes);
    }

    #[test]
    fn test_renderer_undo_redo_sync() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let initial_cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, initial_cube.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();
        let mut history = HistoryStack::new();

        // Frame 1: Initial render
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );

        // Modify mesh and push to history
        let sphere = generate_uv_sphere(1.0, 16, 8);
        history.push(Action::modify_mesh(
            id,
            initial_cube.clone(),
            sphere.clone(),
            "Modify Mesh to Sphere",
        ));
        scene.get_object_mut(id).unwrap().set_mesh(sphere);

        // Frame 2: Render modified mesh (sphere)
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(renderer.last_frame_stats.vertex_buffer_reallocs, 1);

        // Perform Undo -> restores initial_cube
        history.undo(&mut scene);
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert_eq!(
            renderer.mesh_cache.get(&id).unwrap().vertex_buffer.count,
            initial_cube.vertices.len()
        );

        // Perform Redo -> restores sphere
        history.redo(&mut scene);
        renderer.render(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
        );
        assert!(
            renderer.mesh_cache.get(&id).unwrap().vertex_buffer.count > initial_cube.vertices.len()
        );
    }

    fn setup_pixel_test_env(
        width: u32,
        height: u32,
    ) -> Option<(
        GpuContext,
        ViewportRenderer,
        wgpu::Texture,
        wgpu::TextureView,
    )> {
        let ctx = GpuContext::init_headless().ok()?;
        let renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let target_desc = wgpu::TextureDescriptor {
            label: Some("Test Target Pixel"),
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
        Some((ctx, renderer, target_tex, target_view))
    }

    fn read_texture_pixels(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * height) as wgpu::BufferAddress;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            tx.send(res).unwrap();
        });
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        staging_buffer.unmap();
        pixels
    }

    #[test]
    fn test_renderer_gpu_edit_wireframe_initial_upload() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();
        let edit_mesh = EditableMesh::from_mesh(&cube);

        // Render with active edit mesh (First frame: initial allocation)
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_reallocs, 1,
            "Initial edit wireframe must allocate GPU buffers"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_uploads, 0,
            "Initial allocation creates buffers directly without redundant write_buffer"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_draws, 1,
            "Edit wireframe must be drawn in Pass C"
        );
        let buf = renderer
            .edit_mesh_buffers
            .as_ref()
            .expect("Buffers must exist");
        assert_eq!(buf.object_id, id);
        assert_eq!(buf.vert_count, edit_mesh.vertices.len());
        assert_eq!(buf.edge_count, edit_mesh.edges.len());
    }

    #[test]
    fn test_renderer_gpu_edit_wireframe_zero_buffer_updates_on_camera_move() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let mut camera = Camera::default();
        let edit_mesh = EditableMesh::from_mesh(&cube);

        // Frame 1: Initial upload
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        // Frame 2: Camera Orbit / Pan / Zoom (Geometry is unchanged)
        camera.orbit(0.1, 0.2);
        camera.pan(1.0, -1.0);
        camera.zoom(0.5);

        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_uploads, 0,
            "Camera movement must perform zero edit vertex buffer uploads"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_reallocs, 0,
            "Camera movement must perform zero edit buffer reallocations"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_draws, 1,
            "Edit wireframe must still be drawn"
        );
    }

    #[test]
    fn test_renderer_gpu_edit_wireframe_in_place_vertex_update() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

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
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        // Frame 2: Deform vertices in-place (same topology & count, new positions)
        edit_mesh.vertices[0].position.x += 2.0;
        let obj_mut = scene.get_object_mut(id).unwrap();
        obj_mut.generation += 1;

        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_uploads, 1,
            "In-place vertex position update must write buffer in-place"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_reallocs, 0,
            "In-place vertex position update must NOT reallocate buffers"
        );
        assert_eq!(renderer.last_frame_stats.edit_edge_draws, 1);
    }

    #[test]
    fn test_renderer_gpu_edit_wireframe_topology_realloc() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

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
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        // Frame 2: Change topology by adding an edge
        edit_mesh.edges.push(EditEdge::new(0, 2));
        let obj_mut = scene.get_object_mut(id).unwrap();
        obj_mut.generation += 1;

        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );

        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_reallocs, 1,
            "Topology edge count change must reallocate edge index buffer"
        );
        assert_eq!(renderer.last_frame_stats.edit_edge_draws, 1);
    }

    #[test]
    fn test_renderer_gpu_edit_wireframe_cleanup_on_exit() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();
        let edit_mesh = EditableMesh::from_mesh(&cube);

        // Frame 1: In Edit Mode
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            EditRenderOptions::default(),
        );
        assert!(renderer.edit_mesh_buffers.is_some());
        assert_eq!(renderer.last_frame_stats.edit_edge_draws, 1);

        // Frame 2: Exit Edit Mode (active_edit_mesh is None)
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            None,
            EditRenderOptions::default(),
        );
        assert!(
            renderer.edit_mesh_buffers.is_none(),
            "Exiting Edit Mode must immediately drop GPU edit mesh buffers"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_draws, 0,
            "No wireframe lines drawn when not in Edit Mode"
        );
    }

    #[test]
    fn test_renderer_gpu_edit_wireframe_visual_pixel_test() {
        let width = 128;
        let height = 128;
        let Some((ctx, mut renderer, target_tex, target_view)) =
            setup_pixel_test_env(width, height)
        else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone())
            .with_color([0.85, 0.45, 0.25, 1.0]);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera {
            eye: forge_math::Vec3::new(2.0, 2.0, 2.5),
            target: forge_math::Vec3::ZERO,
            up: forge_math::Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        };

        // 1. Render in Object Mode (solid mesh only, no wireframe)
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
            None,
            EditRenderOptions::default(),
        );
        let pixels_obj_mode =
            read_texture_pixels(&ctx.device, &ctx.queue, &target_tex, width, height);

        // 2. Render in Edit Mode with GPU wireframe edges (only edges, no vertices)
        let edit_mesh = EditableMesh::from_mesh(&cube);
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
                is_vertex_mode: false,
                active_vertex: None,
            },
        );
        let pixels_edit_mode =
            read_texture_pixels(&ctx.device, &ctx.queue, &target_tex, width, height);

        // Compare pixels: GPU wireframe must draw lines on top of the solid mesh
        let differing_pixels = pixels_obj_mode
            .chunks(4)
            .zip(pixels_edit_mode.chunks(4))
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            differing_pixels > 20,
            "GPU wireframe must rasterize visible line fragments on screen, found {differing_pixels} differing pixels"
        );
    }

    #[test]
    fn test_renderer_gpu_edit_vertex_initial_upload() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();
        let edit_mesh = EditableMesh::from_mesh(&cube);

        // Frame 1: Initial upload with vertex handles enabled
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
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
            renderer.last_frame_stats.edit_vertex_buffer_reallocs, 1,
            "Initial vertex state buffer must be allocated"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_uploads, 0,
            "Initial allocation creates buffer without redundant write_buffer"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_draws, 1,
            "Vertex handles must be drawn in Pass D"
        );
    }

    #[test]
    fn test_renderer_gpu_edit_vertex_zero_updates_on_camera_move() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone());
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let mut camera = Camera::default();
        let edit_mesh = EditableMesh::from_mesh(&cube);

        let options = EditRenderOptions {
            draw_edges: true,
            draw_vertices: true,
            is_vertex_mode: true,
            active_vertex: None,
        };

        // Frame 1: Initial upload
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );

        // Frame 2: Camera movement (geometry & selection unchanged)
        camera.orbit(0.2, -0.3);
        camera.pan(0.5, 0.5);
        camera.zoom(0.8);

        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
            &scene,
            &selection,
            &camera,
            false,
            Some((&id, &edit_mesh)),
            options,
        );

        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_uploads, 0,
            "Camera movement must cause zero vertex state buffer uploads"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0,
            "Camera movement must cause zero vertex state buffer reallocations"
        );
        assert_eq!(renderer.last_frame_stats.edit_vertex_draws, 1);
    }

    #[test]
    fn test_renderer_gpu_edit_vertex_selection_and_active_update() {
        let Some((ctx, mut renderer, target_view)) = setup_test_env() else {
            return;
        };

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
            100,
            100,
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

        // Frame 2: Select vertex 0 and make it active
        edit_mesh.vertices[0].selected = true;
        renderer.render_with_edit_mesh(
            &ctx.device,
            &ctx.queue,
            &target_view,
            100,
            100,
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

        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_uploads, 1,
            "Selection/active vertex change must update vertex_state_buffer in-place"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_vertex_buffer_reallocs, 0,
            "Selection update must NOT reallocate buffers"
        );
        assert_eq!(
            renderer.last_frame_stats.edit_edge_buffer_uploads, 0,
            "Selection update must NOT upload vertex positions"
        );
    }

    #[test]
    fn test_renderer_gpu_edit_vertex_visual_pixel_test() {
        let width = 128;
        let height = 128;
        let Some((ctx, mut renderer, target_tex, target_view)) =
            setup_pixel_test_env(width, height)
        else {
            return;
        };

        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube.clone())
            .with_color([0.2, 0.2, 0.2, 1.0]);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera {
            eye: forge_math::Vec3::new(2.0, 2.0, 2.5),
            target: forge_math::Vec3::ZERO,
            up: forge_math::Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        };

        let edit_mesh = EditableMesh::from_mesh(&cube);

        // 1. Render with wireframe edges only (no vertex handles)
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
        let pixels_wireframe_only =
            read_texture_pixels(&ctx.device, &ctx.queue, &target_tex, width, height);

        // 2. Render with both wireframe edges AND vertex handles
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
        let pixels_with_handles =
            read_texture_pixels(&ctx.device, &ctx.queue, &target_tex, width, height);

        // Compare pixels: Vertex handles must draw distinct billboard markers on top of wireframe
        let differing_pixels = pixels_wireframe_only
            .chunks(4)
            .zip(pixels_with_handles.chunks(4))
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            differing_pixels > 20,
            "GPU vertex handles must rasterize visible billboard markers, found {differing_pixels} differing pixels"
        );
    }
}
