use std::sync::Arc;

use tracing::info;

use crate::adapter_info::GpuAdapterInfo;
use crate::error::{GpuError, Result};

/// Managed GPU context holding wgpu Instance, Adapter, Device, and Queue.
#[derive(Clone)]
pub struct GpuContext {
    pub instance: Arc<wgpu::Instance>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub adapter_info: GpuAdapterInfo,
}

impl GpuContext {
    /// Initializes a standalone GPU context for headless rendering or compute checks.
    pub fn init_headless() -> Result<Self> {
        let instance = wgpu::Instance::default();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .or_else(|_| {
            // Try fallback adapter if primary high-performance not available
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: true,
                apply_limit_buckets: false,
            }))
        })
        .map_err(|_| GpuError::AdapterNotFound)?;

        let info = adapter.get_info();
        let adapter_info = GpuAdapterInfo::from_wgpu(&info);
        info!("Selected GPU Adapter: {}", adapter_info);

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Forge3D Headless Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        }))
        .map_err(|e| GpuError::DeviceCreationFailed(e.to_string()))?;

        Ok(Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Constructs a `GpuContext` from an existing device and queue (e.g. from eframe/egui-wgpu).
    pub fn from_existing(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        adapter_info: GpuAdapterInfo,
    ) -> Self {
        let instance = wgpu::Instance::default();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .or_else(|_| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: true,
                apply_limit_buckets: false,
            }))
        })
        .expect("Must find a fallback adapter");

        Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device,
            queue,
            adapter_info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_gpu_init() {
        match GpuContext::init_headless() {
            Ok(ctx) => {
                println!("Successfully initialized GPU: {}", ctx.adapter_info);
                assert!(!ctx.adapter_info.name.is_empty());
            }
            Err(e) => {
                println!("Headless GPU init skipped (no adapter in test environment): {e}");
            }
        }
    }
}
