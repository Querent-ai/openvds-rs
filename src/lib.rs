//! OpenVDS - Open Volume Data Store
//!
//! A pure Rust implementation of the OpenVDS specification for fast random access
//! to multi-dimensional volumetric data.
//!
//! # Features
//!
//! - Support for up to 6D volumetric data
//! - Multiple compression algorithms (Deflate, Zstd, RLE)
//! - Local filesystem backend (implement IOManager trait for cloud storage)
//! - Chunked/bricked data layout for efficient random access
//! - Async I/O throughout
//!
//! # Cloud Storage
//!
//! openvds-rs focuses on the OpenVDS format. For cloud storage (S3, Azure, GCS),
//! implement the `IOManager` trait in your application using your preferred cloud SDK.
//!
//! See the `CLOUD_STORAGE.md` file for detailed implementation examples.
//!
//! # Example
//!
//! ```rust,ignore
//! use openvds::{VolumeDataLayout, VolumeDataAccess};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a VDS volume from local filesystem
//! let vds = VolumeDataAccess::open("file:///data/seismic-volume").await?;
//!
//! // Read a slice of data
//! let data = vds.read_slice(&[0, 0, 100], &[1000, 1000, 1]).await?;
//! # Ok(())
//! # }
//! ```

pub mod access;
pub mod compression;
pub mod error;
pub mod io;
pub mod layout;
pub mod metadata;
pub mod types;
pub mod utils;

// Re-exports
pub use access::VolumeDataAccess;
pub use compression::{CompressionMethod, Compressor};
pub use error::{Result, VdsError};
pub use io::{IOManager, StorageBackend};
pub use layout::{BrickSize, VolumeDataLayout};
pub use metadata::VdsMetadata;
pub use types::{AxisDescriptor, DataType, Dimension};

/// Version of the OpenVDS implementation
pub const OPENVDS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Magic number for VDS format
pub const VDS_MAGIC: &[u8; 4] = b"VDS\0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!OPENVDS_VERSION.is_empty());
    }
}
