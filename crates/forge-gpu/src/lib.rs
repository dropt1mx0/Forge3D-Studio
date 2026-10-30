//! # Forge3D GPU
//!
//! wgpu device, queue, surface, buffer wrappers, and runtime adapter capability discovery.

pub mod adapter_info;
pub mod buffer;
pub mod context;
pub mod error;

pub use adapter_info::GpuAdapterInfo;
pub use buffer::GpuBuffer;
pub use context::GpuContext;
pub use error::{GpuError, Result};
