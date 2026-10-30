use thiserror::Error;

/// Core error types for Forge3D domain systems.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Geometry validation failure: {0}")]
    GeometryError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Standard Result type using `CoreError`.
pub type Result<T> = std::result::Result<T, CoreError>;
