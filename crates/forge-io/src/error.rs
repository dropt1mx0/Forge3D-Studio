use forge_core::error::CoreError;
use thiserror::Error;

/// Error types occurring during 3D asset import and export operations.
#[derive(Debug, Error)]
pub enum ForgeIoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid {element} index {index} on line {line} (available elements: {count})")]
    InvalidIndex {
        line: usize,
        index: i64,
        count: usize,
        element: &'static str,
    },

    #[error(
        "Zero index encountered for {element} on line {line} (OBJ format uses 1-based indexing)"
    )]
    ZeroIndex { line: usize, element: &'static str },

    #[error("Parse error on line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Input asset is empty or contains no readable data")]
    EmptyFile,

    #[error("No valid geometry could be constructed from asset: {0}")]
    NoValidGeometry(String),

    #[error("Invalid asset header in {format}: {message}")]
    InvalidHeader {
        format: &'static str,
        message: String,
    },

    #[error("Unsupported asset format or feature in {format}: {message}")]
    UnsupportedFormat {
        format: &'static str,
        message: String,
    },

    #[error("Truncated data encountered in {format}: expected {expected} bytes, found {found}")]
    TruncatedData {
        format: &'static str,
        expected: usize,
        found: usize,
    },

    #[error("Export error: {0}")]
    ExportError(String),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

/// Convenience result type for I/O operations.
pub type Result<T> = std::result::Result<T, ForgeIoError>;
