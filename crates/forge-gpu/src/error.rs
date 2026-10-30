use thiserror::Error;

/// GPU layer errors.
#[derive(Debug, Error)]
pub enum GpuError {
    #[error("Failed to find a suitable GPU adapter matching backend constraints")]
    AdapterNotFound,

    #[error("Failed to create GPU device: {0}")]
    DeviceCreationFailed(String),

    #[error("Surface error: {0}")]
    SurfaceError(String),

    #[error("Buffer creation or update failure: {0}")]
    BufferError(String),

    #[error("Shader compilation error: {0}")]
    ShaderError(String),

    #[error("Pipeline creation error: {0}")]
    PipelineError(String),
}

pub type Result<T> = std::result::Result<T, GpuError>;
