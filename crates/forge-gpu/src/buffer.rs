use bytemuck::Pod;
use wgpu::util::DeviceExt;

/// Typed wrapper for GPU vertex, index, or uniform buffers.
#[derive(Debug)]
pub struct GpuBuffer<T> {
    pub buffer: wgpu::Buffer,
    pub count: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Pod> GpuBuffer<T> {
    /// Creates a vertex buffer initialized with the given slice of elements.
    pub fn new_vertex_buffer(device: &wgpu::Device, label: &str, data: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            count: data.len(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates an index buffer initialized with index data.
    pub fn new_index_buffer(device: &wgpu::Device, label: &str, data: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            count: data.len(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a uniform buffer initialized with a single uniform struct.
    pub fn new_uniform_buffer(device: &wgpu::Device, label: &str, data: &T) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::bytes_of(data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            count: 1,
            _marker: std::marker::PhantomData,
        }
    }

    /// Updates buffer data on the GPU via queue write.
    pub fn update(&self, queue: &wgpu::Queue, data: &[T]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    /// Updates uniform data on the GPU via queue write.
    pub fn update_uniform(&self, queue: &wgpu::Queue, data: &T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}
