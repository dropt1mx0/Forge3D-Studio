//! # GPU Picking / ID Buffer Pipeline
//!
//! Provides hardware-accelerated integer ID buffer rendering and non-blocking
//! single-pixel asynchronous readbacks for sub-millisecond picking of 3D objects,
//! vertices, edges, and polygon faces without CPU raycasting stalls.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bytemuck::bytes_of;
use forge_core::id::ObjectId;
use forge_geometry::EditableMesh;
use forge_gpu::GpuBuffer;
use forge_math::Vec3;
use forge_scene::{EditMode, Scene, SelectionElement};

use crate::camera::Camera;
use crate::shaders::PICKING_SHADER;
use crate::uniforms::{CameraUniform, PickingUniform};

// =========================================================================
// 1. ID ENCODING CONSTANTS AND BITMASKS
// =========================================================================

pub const ID_TYPE_NONE: u32 = 0;
pub const ID_TYPE_OBJECT: u32 = 1;
pub const ID_TYPE_VERTEX: u32 = 2;
pub const ID_TYPE_EDGE: u32 = 3;
pub const ID_TYPE_FACE: u32 = 4;

/// Upper 4 bits reserved for element type; lower 28 bits for index.
pub const ID_TYPE_SHIFT: u32 = 28;
pub const ID_INDEX_MASK: u32 = 0x0FFF_FFFF; // 268,435,455 elements max

/// Result of a GPU Picking query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickResult {
    None,
    Object(ObjectId),
    Vertex(usize),
    Edge(usize),
    Face(usize),
}

/// Encodes an element type tag and index into an unambiguous 32-bit integer ID.
///
/// Returns an error if `index` exceeds the 28-bit limit (`268,435,455`), preventing
/// silent data truncation or identifier collisions.
pub fn encode_id(element_type: u32, index: u32) -> Result<u32, &'static str> {
    if index > ID_INDEX_MASK {
        return Err("Element index exceeds 28-bit limit (268,435,455)");
    }
    Ok(((element_type & 0xF) << ID_TYPE_SHIFT) | (index & ID_INDEX_MASK))
}

/// Encodes an `ObjectId` into a 32-bit integer ID.
///
/// Validates that `ObjectId.raw()` does not overflow 28 bits.
pub fn encode_object_id(id: ObjectId) -> Result<u32, &'static str> {
    let raw = id.raw();
    if raw > ID_INDEX_MASK as u64 {
        return Err("ObjectId exceeds 28-bit limit (268,435,455)");
    }
    encode_id(ID_TYPE_OBJECT, raw as u32)
}

/// Decodes a 32-bit ID value read back from the GPU ID Buffer into a `PickResult`.
pub fn decode_id(raw: u32) -> PickResult {
    let element_type = raw >> ID_TYPE_SHIFT;
    let index = (raw & ID_INDEX_MASK) as usize;

    match element_type {
        ID_TYPE_OBJECT => PickResult::Object(ObjectId::from_raw(index as u64)),
        ID_TYPE_VERTEX => PickResult::Vertex(index),
        ID_TYPE_EDGE => PickResult::Edge(index),
        ID_TYPE_FACE => PickResult::Face(index),
        _ => PickResult::None,
    }
}

// =========================================================================
// 2. COORDINATE TRANSFORMATIONS & DPI SCALING
// =========================================================================

/// Converts screen-space mouse coordinates from UI logical points into discrete
/// physical pixel coordinates within the target GPU ID texture.
///
/// Accounts for:
/// - Viewport origin offset (`viewport_rect.min`)
/// - DPI scaling ratio (`texture_width / viewport_rect.width()`)
/// - Boundary clamping to `[0, texture_width - 1]` and `[0, texture_height - 1]`
///
/// Returns `None` if the mouse position is outside the viewport rect or dimensions are zero.
pub fn screen_to_texture_coords(
    mouse_screen_pos: (f32, f32),
    viewport_rect_min: (f32, f32),
    viewport_rect_size: (f32, f32),
    texture_width: u32,
    texture_height: u32,
) -> Option<(u32, u32)> {
    let (mouse_x, mouse_y) = mouse_screen_pos;
    let (min_x, min_y) = viewport_rect_min;
    let (vp_w, vp_h) = viewport_rect_size;

    if vp_w <= 0.0 || vp_h <= 0.0 || texture_width == 0 || texture_height == 0 {
        return None;
    }

    // Check bounds in screen space
    if mouse_x < min_x || mouse_x >= min_x + vp_w || mouse_y < min_y || mouse_y >= min_y + vp_h {
        return None;
    }

    let rel_x = mouse_x - min_x;
    let rel_y = mouse_y - min_y;

    let u = rel_x / vp_w;
    let v = rel_y / vp_h;

    let tex_x = (u * texture_width as f32).floor() as i32;
    let tex_y = (v * texture_height as f32).floor() as i32;

    let clamped_x = tex_x.clamp(0, texture_width as i32 - 1) as u32;
    let clamped_y = tex_y.clamp(0, texture_height as i32 - 1) as u32;

    Some((clamped_x, clamped_y))
}

// =========================================================================
// 3. STAGING BUFFER POOL WITH EXPLICIT STATE MACHINE
// =========================================================================

/// Lifecycle state of a staging buffer slot in the readback pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingSlotStatus {
    /// Available for a new readback copy command.
    Idle,
    /// Copy command submitted; waiting for GPU execution and map completion.
    CopyPending,
    /// Buffer mapped on CPU; ready to read and must be unmapped before reuse.
    Mapped,
    /// Invalidated while map_async was in-flight. Cannot be reused until map completes and is unmapped.
    InvalidatedPending,
}

/// A pre-allocated staging buffer slot aligned for 1-pixel GPU readback.
pub struct StagingBufferSlot {
    pub buffer: wgpu::Buffer,
    pub status: StagingSlotStatus,
    pub request_id: u64,
    pub epoch: u64,
    pub map_ready: Arc<AtomicBool>,
}

impl StagingBufferSlot {
    pub fn new(device: &wgpu::Device, label: &str) -> Self {
        // Minimum wgpu copy row alignment is 256 bytes
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            status: StagingSlotStatus::Idle,
            request_id: 0,
            epoch: 0,
            map_ready: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Pool of staging buffers preventing buffer collisions during overlapping readback requests.
pub struct StagingBufferPool {
    slots: Vec<StagingBufferSlot>,
}

impl StagingBufferPool {
    pub fn new(device: &wgpu::Device, capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for i in 0..capacity {
            slots.push(StagingBufferSlot::new(
                device,
                &format!("Picking Staging Slot {}", i),
            ));
        }
        Self { slots }
    }

    /// Cleans up any mapped or invalidated slots whose map_async has completed.
    pub fn cleanup_ready_slots(&mut self) {
        for slot in &mut self.slots {
            if (slot.status == StagingSlotStatus::InvalidatedPending
                || slot.status == StagingSlotStatus::Mapped)
                && slot.map_ready.load(Ordering::SeqCst)
            {
                slot.buffer.unmap();
                slot.status = StagingSlotStatus::Idle;
                slot.map_ready.store(false, Ordering::SeqCst);
            }
        }
    }

    /// Acquires an idle slot from the pool. Returns `None` if all slots are actively in use.
    pub fn acquire_idle_slot(&mut self) -> Option<usize> {
        self.cleanup_ready_slots();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.status == StagingSlotStatus::Idle {
                if slot.map_ready.load(Ordering::SeqCst) {
                    slot.buffer.unmap();
                    slot.map_ready.store(false, Ordering::SeqCst);
                }
                return Some(i);
            }
        }
        None
    }

    /// Marks a slot as `CopyPending` with associated request metadata.
    pub fn mark_copy_pending(&mut self, slot_idx: usize, request_id: u64, epoch: u64) {
        if let Some(slot) = self.slots.get_mut(slot_idx) {
            slot.status = StagingSlotStatus::CopyPending;
            slot.request_id = request_id;
            slot.epoch = epoch;
            slot.map_ready.store(false, Ordering::SeqCst);
        }
    }

    /// Safely releases a slot, unmapping the buffer if it was mapped, and resetting to Idle.
    pub fn release_slot(&mut self, slot_idx: usize) {
        if let Some(slot) = self.slots.get_mut(slot_idx) {
            if slot.status == StagingSlotStatus::Mapped || slot.map_ready.load(Ordering::SeqCst) {
                slot.buffer.unmap();
            }
            slot.status = StagingSlotStatus::Idle;
            slot.map_ready.store(false, Ordering::SeqCst);
        }
    }

    /// Invalidates all pending or mapped slots whose epoch is older than `current_epoch`.
    pub fn invalidate_older_than(&mut self, current_epoch: u64) {
        for slot in &mut self.slots {
            if slot.epoch < current_epoch {
                match slot.status {
                    StagingSlotStatus::Idle => {}
                    StagingSlotStatus::CopyPending => {
                        // Cannot unmap yet because map_async is still in-flight on GPU.
                        // Mark as InvalidatedPending so acquire_idle_slot won't touch it until map finishes.
                        slot.status = StagingSlotStatus::InvalidatedPending;
                    }
                    StagingSlotStatus::Mapped => {
                        slot.buffer.unmap();
                        slot.status = StagingSlotStatus::Idle;
                        slot.map_ready.store(false, Ordering::SeqCst);
                    }
                    StagingSlotStatus::InvalidatedPending => {
                        if slot.map_ready.load(Ordering::SeqCst) {
                            slot.buffer.unmap();
                            slot.status = StagingSlotStatus::Idle;
                            slot.map_ready.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }
}

// =========================================================================
// 4. PICKING PIPELINES & PASS EXECUTOR
// =========================================================================

/// Special vertex format for screen-space expanded edge picking quads.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EdgePickingVertex {
    pub pos_a: [f32; 3],
    pub pos_b: [f32; 3],
    pub quad_corner: [f32; 2], // [-1..1]
    pub edge_id: u32,
}

/// Special vertex format for face picking triangles carrying parent face ID.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FacePickingVertex {
    pub position: [f32; 3],
    pub face_id: u32,
}

/// GPU Pipelines specialized for rendering IDs into an `R32Uint` texture target.
pub struct EditPickingPipeline {
    pub camera_layout: wgpu::BindGroupLayout,
    pub cfg_layout: wgpu::BindGroupLayout,
    pub mesh_pipeline: wgpu::RenderPipeline,
    pub vertex_pipeline: wgpu::RenderPipeline,
    pub edge_pipeline: wgpu::RenderPipeline,
    pub face_pipeline: wgpu::RenderPipeline,
}

impl EditPickingPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Picking WGSL Shader Module"),
            source: wgpu::ShaderSource::Wgsl(PICKING_SHADER.into()),
        });

        // Group 0: CameraUniform
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Picking Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Group 1: PickingUniform
        let cfg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Picking Config Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Picking Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&cfg_layout)],
            immediate_size: 0,
        });

        // 1. Mesh / Object picking pipeline
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Mesh Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_mesh"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 0,
                            shader_location: 1,
                        }],
                    }),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_mesh"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // 2. Vertex picking pipeline (instanced billboard quads)
        let vertex_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Vertex Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    // Slot 0: Static unit quad corners
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    }),
                    // Slot 1: Vertex center 3D position
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        }],
                    }),
                    // Slot 2: Encoded vertex ID (u32)
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 0,
                            shader_location: 2,
                        }],
                    }),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_vertex"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // 3. Edge picking pipeline (screen-space expanded quads)
        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Edge Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_edge"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<EdgePickingVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 32,
                            shader_location: 3,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_edge"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // 4. Face picking pipeline (triangulated faces carrying face ID)
        let face_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Face Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_face"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<FacePickingVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_face"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            camera_layout,
            cfg_layout,
            mesh_pipeline,
            vertex_pipeline,
            edge_pipeline,
            face_pipeline,
        }
    }
}

/// Cached GPU mesh picking buffers for an object in the scene.
pub struct PickingMeshEntry {
    pub vertex_buffer: GpuBuffer<[f32; 3]>,
    pub id_buffer: GpuBuffer<u32>,
    pub index_buffer: GpuBuffer<u32>,
    pub generation: u64,
    pub world_matrix: [[f32; 4]; 4],
}

/// Cached GPU picking geometry for the active mesh in Edit Mode.
pub struct PickingEditGeometryCache {
    pub object_id: ObjectId,
    pub generation: u64,
    pub vert_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub world_matrix: [[f32; 4]; 4],
    // Vertex picking buffers:
    pub vertex_pos_buffer: GpuBuffer<[f32; 3]>,
    pub vertex_id_buffer: GpuBuffer<u32>,
    // Edge picking buffers:
    pub edge_quad_buffer: GpuBuffer<EdgePickingVertex>,
    pub edge_index_buffer: GpuBuffer<u32>,
    // Face picking buffers:
    pub face_vert_buffer: GpuBuffer<FacePickingVertex>,
    pub face_index_buffer: GpuBuffer<u32>,
}

/// Managed GPU Picking context with cached ID target textures and staging readback pool.
pub struct GpuPickingContext {
    pub pipelines: EditPickingPipeline,
    pub id_texture: Option<wgpu::Texture>,
    pub id_view: Option<wgpu::TextureView>,
    pub depth_texture: Option<wgpu::Texture>,
    pub depth_view: Option<wgpu::TextureView>,
    pub texture_size: (u32, u32),
    pub staging_pool: StagingBufferPool,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub cfg_buffer: wgpu::Buffer,
    pub cfg_bind_group: wgpu::BindGroup,
    pub unit_quad_vertex_buffer: GpuBuffer<[f32; 2]>,
    pub unit_quad_index_buffer: GpuBuffer<u32>,
    pub mesh_cache: HashMap<ObjectId, PickingMeshEntry>,
    pub edit_cache: Option<PickingEditGeometryCache>,
    pub next_request_id: u64,
    pub invalidation_epoch: u64,
}

impl GpuPickingContext {
    pub fn new(device: &wgpu::Device) -> Self {
        let pipelines = EditPickingPipeline::new(device);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Picking Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Picking Camera Bind Group"),
            layout: &pipelines.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let cfg_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Picking Config Buffer"),
            size: std::mem::size_of::<PickingUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cfg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Picking Config Bind Group"),
            layout: &pipelines.cfg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cfg_buffer.as_entire_binding(),
            }],
        });

        let quad_vertices: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let quad_indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let unit_quad_vertex_buffer =
            GpuBuffer::new_vertex_buffer(device, "Picking Quad Vertex Buffer", &quad_vertices);
        let unit_quad_index_buffer =
            GpuBuffer::new_index_buffer(device, "Picking Quad Index Buffer", &quad_indices);

        Self {
            pipelines,
            id_texture: None,
            id_view: None,
            depth_texture: None,
            depth_view: None,
            texture_size: (0, 0),
            staging_pool: StagingBufferPool::new(device, 4),
            camera_buffer,
            camera_bind_group,
            cfg_buffer,
            cfg_bind_group,
            unit_quad_vertex_buffer,
            unit_quad_index_buffer,
            mesh_cache: HashMap::new(),
            edit_cache: None,
            next_request_id: 1,
            invalidation_epoch: 1,
        }
    }

    /// Increments the epoch, immediately invalidating any pending readback results.
    pub fn invalidate(&mut self) {
        self.invalidation_epoch = self.invalidation_epoch.wrapping_add(1);
        self.staging_pool
            .invalidate_older_than(self.invalidation_epoch);
    }

    /// Invalidates cached picking buffers for a specific object and drops stale readbacks.
    pub fn invalidate_mesh(&mut self, id: &ObjectId) {
        self.mesh_cache.remove(id);
        if let Some(c) = &self.edit_cache
            && c.object_id == *id
        {
            self.edit_cache = None;
        }
        self.invalidate();
    }

    /// Clears all cached GPU picking geometry and invalidates all pending queries.
    pub fn invalidate_all(&mut self) {
        self.mesh_cache.clear();
        self.edit_cache = None;
        self.invalidate();
    }

    /// Returns true if the picking mesh cache contains an entry for the given object.
    pub fn has_cached_mesh(&self, id: &ObjectId) -> bool {
        self.mesh_cache.contains_key(id)
    }

    /// Ensures that the ID and depth textures match the viewport resolution.
    pub fn ensure_textures(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        if self.texture_size != (width, height)
            || self.id_texture.is_none()
            || self.depth_texture.is_none()
        {
            let id_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Picking ID Render Target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Uint,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let id_view = id_tex.create_view(&wgpu::TextureViewDescriptor::default());

            let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Picking Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

            self.id_texture = Some(id_tex);
            self.id_view = Some(id_view);
            self.depth_texture = Some(depth_tex);
            self.depth_view = Some(depth_view);
            self.texture_size = (width, height);
            self.invalidate();
        }
    }

    /// Renders the picking pass into the ID texture and schedules a non-blocking 1-pixel copy.
    ///
    /// Returns the assigned `request_id` if successfully scheduled, or `None` if pool is full.
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
        self.ensure_textures(device, width, height);

        let slot_idx = self.staging_pool.acquire_idle_slot()?;
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);

        // Update Camera Uniform
        let cam_uniform = CameraUniform::new(camera.view_proj_matrix(), camera.eye);
        queue.write_buffer(&self.camera_buffer, 0, bytes_of(&cam_uniform));

        let id_view = self.id_view.as_ref()?;
        let depth_view = self.depth_view.as_ref()?;
        let id_texture = self.id_texture.as_ref()?;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Picking Render & Copy Encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Picking ID Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: id_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }), // 0 = Background / Miss
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

            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            let cfg = PickingUniform::new(0, [width as f32, height as f32], 6.0);
            queue.write_buffer(&self.cfg_buffer, 0, bytes_of(&cfg));
            pass.set_bind_group(1, &self.cfg_bind_group, &[]);

            match edit_mode {
                EditMode::Object => {
                    // Render all visible scene objects with their encoded ObjectId as instance attribute
                    pass.set_pipeline(&self.pipelines.mesh_pipeline);
                    self.mesh_cache
                        .retain(|id, _| scene.objects.contains_key(id));

                    for (&id, obj) in &scene.objects {
                        if !obj.visible || !obj.selectable {
                            continue;
                        }
                        let Ok(encoded_id) = encode_object_id(id) else {
                            continue;
                        };

                        let world_tf = scene.world_transform(id);
                        let world_mat = world_tf.to_matrix().to_cols_array_2d();

                        let needs_update = match self.mesh_cache.get(&id) {
                            Some(entry) => {
                                entry.generation != obj.generation
                                    || entry.world_matrix != world_mat
                            }
                            None => true,
                        };

                        if needs_update {
                            let eval_mesh = obj.evaluated_mesh();
                            let world_positions: Vec<[f32; 3]> = eval_mesh
                                .vertices
                                .iter()
                                .map(|v| {
                                    world_tf
                                        .transform_point(Vec3::from_array(v.position))
                                        .to_array()
                                })
                                .collect();

                            if !world_positions.is_empty() && !eval_mesh.indices.is_empty() {
                                let v_buf = GpuBuffer::new_vertex_buffer(
                                    device,
                                    "Picking Mesh VBuf",
                                    &world_positions,
                                );
                                let id_buf = GpuBuffer::new_vertex_buffer(
                                    device,
                                    "Picking Mesh IDBuf",
                                    &[encoded_id],
                                );
                                let i_buf = GpuBuffer::new_index_buffer(
                                    device,
                                    "Picking Mesh IBuf",
                                    &eval_mesh.indices,
                                );
                                self.mesh_cache.insert(
                                    id,
                                    PickingMeshEntry {
                                        vertex_buffer: v_buf,
                                        id_buffer: id_buf,
                                        index_buffer: i_buf,
                                        generation: obj.generation,
                                        world_matrix: world_mat,
                                    },
                                );
                            }
                        }

                        if let Some(entry) = self.mesh_cache.get(&id) {
                            pass.set_vertex_buffer(0, entry.vertex_buffer.buffer.slice(..));
                            pass.set_vertex_buffer(1, entry.id_buffer.buffer.slice(..));
                            pass.set_index_buffer(
                                entry.index_buffer.buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            pass.draw_indexed(0..entry.index_buffer.count as u32, 0, 0..1);
                        }
                    }
                }
                EditMode::Edit | EditMode::Sculpt => {
                    // 1. Render all solid scene meshes first to populate the depth buffer and provide occlusion
                    pass.set_pipeline(&self.pipelines.mesh_pipeline);
                    self.mesh_cache
                        .retain(|id, _| scene.objects.contains_key(id));

                    for (&id, obj) in &scene.objects {
                        if !obj.visible || !obj.selectable {
                            continue;
                        }
                        let Ok(encoded_id) = encode_object_id(id) else {
                            continue;
                        };

                        let world_tf = scene.world_transform(id);
                        let world_mat = world_tf.to_matrix().to_cols_array_2d();

                        let needs_update = match self.mesh_cache.get(&id) {
                            Some(entry) => {
                                entry.generation != obj.generation
                                    || entry.world_matrix != world_mat
                            }
                            None => true,
                        };

                        if needs_update {
                            let eval_mesh = obj.evaluated_mesh();
                            let world_positions: Vec<[f32; 3]> = eval_mesh
                                .vertices
                                .iter()
                                .map(|v| {
                                    world_tf
                                        .transform_point(Vec3::from_array(v.position))
                                        .to_array()
                                })
                                .collect();

                            if !world_positions.is_empty() && !eval_mesh.indices.is_empty() {
                                let v_buf = GpuBuffer::new_vertex_buffer(
                                    device,
                                    "Picking Mesh VBuf",
                                    &world_positions,
                                );
                                let id_buf = GpuBuffer::new_vertex_buffer(
                                    device,
                                    "Picking Mesh IDBuf",
                                    &[encoded_id],
                                );
                                let i_buf = GpuBuffer::new_index_buffer(
                                    device,
                                    "Picking Mesh IBuf",
                                    &eval_mesh.indices,
                                );
                                self.mesh_cache.insert(
                                    id,
                                    PickingMeshEntry {
                                        vertex_buffer: v_buf,
                                        id_buffer: id_buf,
                                        index_buffer: i_buf,
                                        generation: obj.generation,
                                        world_matrix: world_mat,
                                    },
                                );
                            }
                        }

                        if let Some(entry) = self.mesh_cache.get(&id) {
                            pass.set_vertex_buffer(0, entry.vertex_buffer.buffer.slice(..));
                            pass.set_vertex_buffer(1, entry.id_buffer.buffer.slice(..));
                            pass.set_index_buffer(
                                entry.index_buffer.buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            pass.draw_indexed(0..entry.index_buffer.count as u32, 0, 0..1);
                        }
                    }

                    // 2. Render edit mode handles/elements with depth offset on top of the solid geometry
                    if let Some((&edit_id, edit_mesh)) = active_edit_mesh {
                        let world_tf = scene.world_transform(edit_id);
                        let world_mat = world_tf.to_matrix().to_cols_array_2d();
                        let obj_gen = scene.get_object(edit_id).map(|o| o.generation).unwrap_or(0);

                        let v_len = edit_mesh.vertices.len();
                        let e_len = edit_mesh.edges.len();
                        let f_len = edit_mesh.faces.len();

                        let needs_update = match &self.edit_cache {
                            Some(c) => {
                                c.object_id != edit_id
                                    || c.generation != obj_gen
                                    || c.vert_count != v_len
                                    || c.edge_count != e_len
                                    || c.face_count != f_len
                                    || c.world_matrix != world_mat
                            }
                            None => true,
                        };

                        if needs_update {
                            let world_positions: Vec<Vec3> = edit_mesh
                                .vertices
                                .iter()
                                .map(|v| world_tf.transform_point(v.position))
                                .collect();

                            // Vertex picking buffers
                            let pos_arrays: Vec<[f32; 3]> =
                                world_positions.iter().map(|p| p.to_array()).collect();
                            let id_arrays: Vec<u32> = (0..v_len as u32)
                                .map(|i| encode_id(ID_TYPE_VERTEX, i).unwrap_or(0))
                                .collect();
                            let vertex_pos_buffer =
                                GpuBuffer::new_vertex_buffer(device, "Pick Vert Pos", &pos_arrays);
                            let vertex_id_buffer =
                                GpuBuffer::new_vertex_buffer(device, "Pick Vert IDs", &id_arrays);

                            // Edge picking buffers (expanded quads)
                            let mut edge_quads = Vec::with_capacity(e_len * 4);
                            let mut edge_indices = Vec::with_capacity(e_len * 6);
                            for (e_idx, edge) in edit_mesh.edges.iter().enumerate() {
                                if edge.v0 >= world_positions.len()
                                    || edge.v1 >= world_positions.len()
                                {
                                    continue;
                                }
                                let pa = world_positions[edge.v0].to_array();
                                let pb = world_positions[edge.v1].to_array();
                                let eid = encode_id(ID_TYPE_EDGE, e_idx as u32).unwrap_or(0);

                                let base = edge_quads.len() as u32;
                                edge_quads.push(EdgePickingVertex {
                                    pos_a: pa,
                                    pos_b: pb,
                                    quad_corner: [-1.0, -1.0],
                                    edge_id: eid,
                                });
                                edge_quads.push(EdgePickingVertex {
                                    pos_a: pa,
                                    pos_b: pb,
                                    quad_corner: [1.0, -1.0],
                                    edge_id: eid,
                                });
                                edge_quads.push(EdgePickingVertex {
                                    pos_a: pa,
                                    pos_b: pb,
                                    quad_corner: [1.0, 1.0],
                                    edge_id: eid,
                                });
                                edge_quads.push(EdgePickingVertex {
                                    pos_a: pa,
                                    pos_b: pb,
                                    quad_corner: [-1.0, 1.0],
                                    edge_id: eid,
                                });

                                edge_indices.extend_from_slice(&[
                                    base,
                                    base + 1,
                                    base + 2,
                                    base,
                                    base + 2,
                                    base + 3,
                                ]);
                            }
                            let edge_quad_buffer = GpuBuffer::new_vertex_buffer(
                                device,
                                "Pick Edge Quads",
                                &edge_quads,
                            );
                            let edge_index_buffer = GpuBuffer::new_index_buffer(
                                device,
                                "Pick Edge Indices",
                                &edge_indices,
                            );

                            // Face picking buffers
                            let mut face_verts = Vec::new();
                            let mut face_indices = Vec::new();
                            for (f_idx, face) in edit_mesh.faces.iter().enumerate() {
                                if face.vertices.len() < 3 {
                                    continue;
                                }
                                let fid = encode_id(ID_TYPE_FACE, f_idx as u32).unwrap_or(0);
                                let start_v = face_verts.len() as u32;

                                for &vi in &face.vertices {
                                    if vi < world_positions.len() {
                                        face_verts.push(FacePickingVertex {
                                            position: world_positions[vi].to_array(),
                                            face_id: fid,
                                        });
                                    }
                                }

                                let n = face.vertices.len() as u32;
                                for i in 1..(n - 1) {
                                    face_indices.push(start_v);
                                    face_indices.push(start_v + i);
                                    face_indices.push(start_v + i + 1);
                                }
                            }
                            let face_vert_buffer = GpuBuffer::new_vertex_buffer(
                                device,
                                "Pick Face Verts",
                                &face_verts,
                            );
                            let face_index_buffer = GpuBuffer::new_index_buffer(
                                device,
                                "Pick Face Indices",
                                &face_indices,
                            );

                            self.edit_cache = Some(PickingEditGeometryCache {
                                object_id: edit_id,
                                generation: obj_gen,
                                vert_count: v_len,
                                edge_count: e_len,
                                face_count: f_len,
                                world_matrix: world_mat,
                                vertex_pos_buffer,
                                vertex_id_buffer,
                                edge_quad_buffer,
                                edge_index_buffer,
                                face_vert_buffer,
                                face_index_buffer,
                            });
                        }

                        // Draw from cached picking buffers
                        if let Some(cache) = &self.edit_cache {
                            match selection_element {
                                SelectionElement::Vertex => {
                                    if cache.vert_count > 0 {
                                        pass.set_pipeline(&self.pipelines.vertex_pipeline);
                                        pass.set_vertex_buffer(
                                            0,
                                            self.unit_quad_vertex_buffer.buffer.slice(..),
                                        );
                                        pass.set_vertex_buffer(
                                            1,
                                            cache.vertex_pos_buffer.buffer.slice(..),
                                        );
                                        pass.set_vertex_buffer(
                                            2,
                                            cache.vertex_id_buffer.buffer.slice(..),
                                        );
                                        pass.set_index_buffer(
                                            self.unit_quad_index_buffer.buffer.slice(..),
                                            wgpu::IndexFormat::Uint32,
                                        );
                                        pass.draw_indexed(0..6, 0, 0..cache.vert_count as u32);
                                    }
                                }
                                SelectionElement::Edge => {
                                    if cache.edge_index_buffer.count > 0 {
                                        pass.set_pipeline(&self.pipelines.edge_pipeline);
                                        pass.set_vertex_buffer(
                                            0,
                                            cache.edge_quad_buffer.buffer.slice(..),
                                        );
                                        pass.set_index_buffer(
                                            cache.edge_index_buffer.buffer.slice(..),
                                            wgpu::IndexFormat::Uint32,
                                        );
                                        pass.draw_indexed(
                                            0..cache.edge_index_buffer.count as u32,
                                            0,
                                            0..1,
                                        );
                                    }
                                }
                                SelectionElement::Face => {
                                    if cache.face_index_buffer.count > 0 {
                                        pass.set_pipeline(&self.pipelines.face_pipeline);
                                        pass.set_vertex_buffer(
                                            0,
                                            cache.face_vert_buffer.buffer.slice(..),
                                        );
                                        pass.set_index_buffer(
                                            cache.face_index_buffer.buffer.slice(..),
                                            wgpu::IndexFormat::Uint32,
                                        );
                                        pass.draw_indexed(
                                            0..cache.face_index_buffer.count as u32,
                                            0,
                                            0..1,
                                        );
                                    }
                                }
                            }
                        }
                    } else if self.edit_cache.is_some() {
                        self.edit_cache = None;
                    }
                }
            }
        }

        // Copy exactly 1 pixel (clamped) to the acquired staging slot buffer
        let target_x = pixel_x.clamp(0, width.saturating_sub(1));
        let target_y = pixel_y.clamp(0, height.saturating_sub(1));

        let slot = &self.staging_pool.slots[slot_idx];
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: id_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: target_x,
                    y: target_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &slot.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Mark slot CopyPending
        self.staging_pool
            .mark_copy_pending(slot_idx, request_id, self.invalidation_epoch);

        // Schedule async mapping of 4 bytes
        let map_ready_flag = Arc::clone(&self.staging_pool.slots[slot_idx].map_ready);
        self.staging_pool.slots[slot_idx]
            .buffer
            .slice(0..4)
            .map_async(wgpu::MapMode::Read, move |res| {
                if res.is_ok() {
                    map_ready_flag.store(true, Ordering::SeqCst);
                }
            });

        Some(request_id)
    }

    /// Non-blocking check for a completed pick result.
    ///
    /// If ready, reads the 32-bit ID, unmaps the staging buffer, resets the slot to Idle,
    /// and decodes the result. Discards stale results automatically.
    pub fn poll_pick_result(&mut self, request_id: u64) -> Option<PickResult> {
        let slot_idx = self
            .staging_pool
            .slots
            .iter()
            .position(|s| s.request_id == request_id)?;

        let slot = &mut self.staging_pool.slots[slot_idx];

        // Check epoch validity
        if slot.epoch != self.invalidation_epoch {
            if slot.status == StagingSlotStatus::Mapped || slot.map_ready.load(Ordering::SeqCst) {
                slot.buffer.unmap();
                slot.status = StagingSlotStatus::Idle;
                slot.map_ready.store(false, Ordering::SeqCst);
            } else {
                slot.status = StagingSlotStatus::InvalidatedPending;
            }
            return None;
        }

        // Check if map_async has completed
        if !slot.map_ready.load(Ordering::SeqCst) {
            return None;
        }

        slot.status = StagingSlotStatus::Mapped;

        let raw_id = {
            let data = slot.buffer.slice(0..4).get_mapped_range();
            let bytes: [u8; 4] = match data {
                Ok(ref d) => d[0..4].try_into().unwrap_or([0, 0, 0, 0]),
                Err(_) => [0, 0, 0, 0],
            };
            u32::from_ne_bytes(bytes)
        };

        // Unmap and free slot back to Idle
        self.staging_pool.release_slot(slot_idx);

        Some(decode_id(raw_id))
    }

    /// Synchronous picking query (for tests and pixel readback verification).
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
        let Some(req_id) = self.schedule_pick_request(
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
        ) else {
            return PickResult::None;
        };

        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        self.poll_pick_result(req_id).unwrap_or(PickResult::None)
    }
}

// =========================================================================
// 5. UNIT TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_encoding_and_decoding() {
        assert_eq!(decode_id(0), PickResult::None);

        // Object ID
        let obj_id = ObjectId::from_raw(42);
        let enc_obj = encode_object_id(obj_id).unwrap();
        assert_eq!(decode_id(enc_obj), PickResult::Object(obj_id));

        // Vertex ID
        let enc_v = encode_id(ID_TYPE_VERTEX, 1024).unwrap();
        assert_eq!(decode_id(enc_v), PickResult::Vertex(1024));

        // Edge ID
        let enc_e = encode_id(ID_TYPE_EDGE, 8192).unwrap();
        assert_eq!(decode_id(enc_e), PickResult::Edge(8192));

        // Face ID
        let enc_f = encode_id(ID_TYPE_FACE, 32768).unwrap();
        assert_eq!(decode_id(enc_f), PickResult::Face(32768));
    }

    #[test]
    fn test_id_encoding_overflow_validation() {
        // Exceeds 28 bits: 0x1000_0000 = 268,435,456
        let res = encode_id(ID_TYPE_VERTEX, 0x1000_0000);
        assert!(res.is_err(), "Must reject index > 28 bits");

        let huge_obj = ObjectId::from_raw(0x2000_0000);
        assert!(encode_object_id(huge_obj).is_err());
    }

    #[test]
    fn test_screen_to_texture_coordinate_conversion() {
        // Viewport: pos (100, 50), size (800, 600), Texture size (1600, 1200) [2x DPI]
        let min = (100.0, 50.0);
        let size = (800.0, 600.0);

        // Center click (500, 350) -> rel (400, 300) -> 0.5 * 1600 = 800, 0.5 * 1200 = 600
        let coords = screen_to_texture_coords((500.0, 350.0), min, size, 1600, 1200);
        assert_eq!(coords, Some((800, 600)));

        // Top-left click (100, 50) -> (0, 0)
        let tl = screen_to_texture_coords((100.0, 50.0), min, size, 1600, 1200);
        assert_eq!(tl, Some((0, 0)));

        // Out of bounds clicks
        assert_eq!(
            screen_to_texture_coords((99.0, 50.0), min, size, 1600, 1200),
            None
        );
        assert_eq!(
            screen_to_texture_coords((901.0, 350.0), min, size, 1600, 1200),
            None
        );
    }

    #[test]
    fn test_staging_pool_lifecycle() {
        let Ok(ctx) = forge_gpu::GpuContext::init_headless() else {
            return;
        };

        let mut pool = StagingBufferPool::new(&ctx.device, 2);
        assert_eq!(pool.slot_count(), 2);

        // Acquire slot 0
        let s0 = pool.acquire_idle_slot().unwrap();
        assert_eq!(s0, 0);
        pool.mark_copy_pending(s0, 1, 10);

        // Acquire slot 1
        let s1 = pool.acquire_idle_slot().unwrap();
        assert_eq!(s1, 1);
        pool.mark_copy_pending(s1, 2, 10);

        // Pool full -> returns None
        assert!(pool.acquire_idle_slot().is_none());

        // Release slot 0
        pool.release_slot(s0);
        let s0_again = pool.acquire_idle_slot().unwrap();
        assert_eq!(s0_again, 0);

        // Invalidate older epoch - since slot 1 is CopyPending, it becomes InvalidatedPending
        // to protect against premature buffer reuse while map_async is active.
        pool.invalidate_older_than(11);
        assert_eq!(pool.slots[1].status, StagingSlotStatus::InvalidatedPending);

        // When GPU mapping completes, cleanup_ready_slots unmaps and returns it to Idle safely.
        let ready_flag = Arc::clone(&pool.slots[1].map_ready);
        pool.slots[1]
            .buffer
            .slice(0..4)
            .map_async(wgpu::MapMode::Read, move |res| {
                if res.is_ok() {
                    ready_flag.store(true, Ordering::SeqCst);
                }
            });
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        assert!(pool.slots[1].map_ready.load(Ordering::SeqCst));
        pool.cleanup_ready_slots();
        assert_eq!(pool.slots[1].status, StagingSlotStatus::Idle);
    }
}
