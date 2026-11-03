use thiserror::Error;

use super::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};

/// Errors encountered during topology building, traversal, validation, and operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TopologyError {
    #[error("Invalid handle: {0}")]
    InvalidHandle(String),

    #[error("Invalid vertex handle: {0}")]
    InvalidVertex(VertexHandle),

    #[error("Invalid half-edge handle: {0}")]
    InvalidHalfEdge(HalfEdgeHandle),

    #[error("Invalid edge handle: {0}")]
    InvalidEdge(EdgeHandle),

    #[error("Invalid face handle: {0}")]
    InvalidFace(FaceHandle),

    #[error("Non-manifold geometry detected: {0}")]
    NonManifold(String),

    #[error("Degenerate geometry: {0}")]
    Degenerate(String),

    #[error("Topological invariant violated: {0}")]
    InvariantViolation(String),

    #[error("Illegal topological operation: {0}")]
    InvalidOperation(String),

    #[error("Empty mesh: cannot perform operation")]
    EmptyMesh,

    #[error("Topology builder error: {0}")]
    BuildError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("Missing connection: {0}")]
    MissingConnection(String),

    #[error("Broken cycle: {0}")]
    BrokenCycle(String),

    #[error("Traversal limit exceeded: {0}")]
    TraversalLimitExceeded(String),

    #[error("Invalid boundary loop: {0}")]
    InvalidBoundaryLoop(String),
}
