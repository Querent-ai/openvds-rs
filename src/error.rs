//! Error types for OpenVDS operations

use thiserror::Error;

/// Main error type for VDS operations
#[derive(Error, Debug)]
pub enum VdsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid VDS format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported VDS version: {0}")]
    UnsupportedVersion(u32),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),

    #[error("Out of bounds: {0}")]
    OutOfBounds(String),

    #[error("Storage backend error: {0}")]
    StorageBackend(String),

    #[error("Metadata error: {0}")]
    Metadata(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid data type")]
    InvalidDataType,

    #[error("Invalid axis: {0}")]
    InvalidAxis(usize),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Specialized Result type for VDS operations
pub type Result<T> = std::result::Result<T, VdsError>;

impl From<bincode::Error> for VdsError {
    fn from(err: bincode::Error) -> Self {
        VdsError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for VdsError {
    fn from(err: serde_json::Error) -> Self {
        VdsError::Serialization(err.to_string())
    }
}
