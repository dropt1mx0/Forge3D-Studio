use std::collections::HashMap;

use forge_core::id::ObjectId;
use forge_geometry::primitives::generate_grid_lines;
use forge_geometry::{LineVertex, Vertex};
use forge_gpu::GpuBuffer;
use forge_math::Vec3;
use forge_scene::{Scene, SelectionState};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::pipelines::{DEPTH_FORMAT, RenderPipelines};
use crate::uniforms::{CameraUniform, ObjectUniform};

struct GpuMeshEntry {
    vertex_buffer: GpuBuffer<Vertex>,
    index_buffer: GpuBuffer<u32>,
    object_uniform_buffer: wgpu::Buffer,
    object_bind_group: wgpu::BindGroup,
    generation: u64,
}

/// The main 3D Viewport Renderer orchestrating pipelines, GPU buffers, depth testing, and passes.
pub struct ViewportRenderer {
    pub pipelines: RenderPipelines,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub grid_buffer: GpuBuffer<LineVertex>,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    depth_size: (u32, u32),
    mesh_cache: HashMap<ObjectId, GpuMeshEntry>,
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

        // Generate ground grid lines (size: 20x20m, step: 1m)
        let grid_lines = generate_grid_lines(10.0, 1.0);
        let grid_buffer = GpuBuffer::new_vertex_buffer(device, "Grid Vertex Buffer", &grid_lines);

        Self {
            pipelines,
            camera_buffer,
            camera_bind_group,
            grid_buffer,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            mesh_cache: HashMap::new(),
        }
    }

    /// Invalidate GPU cache for a specific object mesh.
    pub fn invalidate_mesh(&mut self, id: &ObjectId) {
        self.mesh_cache.remove(id);
    }

    /// Clear all cached GPU meshes.
    pub fn invalidate_all_meshes(&mut self) {
        self.mesh_cache.clear();
    }

    /// Re-allocates the depth texture if the viewport dimensions change.
    fn ensure_depth_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.depth_size != (width, height) || self.depth_texture.is_none() {
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
        if viewport_width == 0 || viewport_height == 0 {
            return;
        }

        // 1. Ensure depth texture matches viewport size
        self.ensure_depth_texture(device, viewport_width, viewport_height);

        // 2. Update Camera Uniform
        let cam_uniform = CameraUniform::new(camera.view_proj_matrix(), camera.eye);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cam_uniform));

        // 3. Sync GPU mesh cache with scene objects
        self.mesh_cache
            .retain(|id, _| scene.objects.contains_key(id));

        for (&id, obj) in &scene.objects {
            if !obj.visible {
                continue;
            }

            let world_tf = scene.world_transform(id);
            let is_selected = selection.is_selected(id);
            let obj_uniform = ObjectUniform::new(world_tf.to_matrix(), obj.color, is_selected);

            if let Some(entry) = self.mesh_cache.get_mut(&id) {
                // If mesh geometry or modifiers changed, re-evaluate and re-upload buffers
                if entry.generation != obj.generation {
                    let eval_mesh = obj.evaluated_mesh();
                    if entry.vertex_buffer.count != eval_mesh.vertices.len()
                        || entry.index_buffer.count != eval_mesh.indices.len()
                    {
                        entry.vertex_buffer = GpuBuffer::new_vertex_buffer(
                            device,
                            &format!("{}_vbuf", obj.name),
                            &eval_mesh.vertices,
                        );
                        entry.index_buffer = GpuBuffer::new_index_buffer(
                            device,
                            &format!("{}_ibuf", obj.name),
                            &eval_mesh.indices,
                        );
                    } else {
                        // Update vertex buffer in-place if count matches
                        queue.write_buffer(
                            &entry.vertex_buffer.buffer,
                            0,
                            bytemuck::cast_slice(&eval_mesh.vertices),
                        );
                    }
                    entry.generation = obj.generation;
                }

                // Update object uniform
                queue.write_buffer(
                    &entry.object_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&obj_uniform),
                );
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

                let u_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{}_ubuf", obj.name)),
                    contents: bytemuck::bytes_of(&obj_uniform),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

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
                    },
                );
            }
        }

        // 4. Command Encoder & Render Pass
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Viewport Render Encoder"),
        });

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

            for (&id, obj) in &scene.objects {
                if !obj.visible {
                    continue;
                }

                if let Some(entry) = self.mesh_cache.get(&id) {
                    pass.set_bind_group(1, &entry.object_bind_group, &[]);
                    pass.set_vertex_buffer(0, entry.vertex_buffer.buffer.slice(..));
                    pass.set_index_buffer(
                        entry.index_buffer.buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..entry.index_buffer.count as u32, 0, 0..1);
                }
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::generate_cube;
    use forge_gpu::GpuContext;
    use forge_scene::{PrimitiveType, Scene, SceneObject, SelectionState};

    #[test]
    fn test_renderer_mesh_cache_reuse_and_invalidation() {
        let Ok(ctx) = GpuContext::init_headless() else {
            // Headless adapter might not be available in headless CI
            return;
        };

        let mut renderer = ViewportRenderer::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut scene = Scene::new("Test Scene");
        let cube = generate_cube(1.0);
        let obj = SceneObject::new("TestCube", PrimitiveType::Cube, cube);
        let id = obj.id;
        scene.add_object(obj);

        let selection = SelectionState::new();
        let camera = Camera::default();

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

        // 1. Initial render uploads mesh to GPU cache
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
        let entry1_gen = renderer.mesh_cache.get(&id).unwrap().generation;
        assert_eq!(entry1_gen, 0);

        // 2. Second render reuses GPU cache with zero re-evaluation
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
        assert_eq!(renderer.mesh_cache.get(&id).unwrap().generation, entry1_gen);

        // 3. Mutating object base mesh increments generation
        let obj_mut = scene.get_object_mut(id).unwrap();
        let new_cube = generate_cube(2.0);
        obj_mut.set_mesh(new_cube);
        assert_eq!(obj_mut.generation, 1);

        // 4. Render pass detects generation change and updates GPU cache
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
        assert_eq!(renderer.mesh_cache.get(&id).unwrap().generation, 1);

        // 5. Invalidate mesh explicitly clears cache
        renderer.invalidate_mesh(&id);
        assert!(!renderer.mesh_cache.contains_key(&id));
    }
}
