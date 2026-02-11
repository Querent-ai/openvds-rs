//! I/O managers for different storage backends

use crate::error::{Result, VdsError};
use async_trait::async_trait;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Storage backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    /// Local file system
    FileSystem,
    /// AWS S3
    S3,
    /// Azure Blob Storage
    Azure,
    /// Google Cloud Storage
    GCS,
    /// OSDU/DELFI Seismic DMS
    SeismicDMS,
}

impl StorageBackend {
    /// Parse storage backend from URL scheme
    pub fn from_url(url: &str) -> Result<Self> {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end];
            match scheme {
                "file" => Ok(StorageBackend::FileSystem),
                "s3" => Ok(StorageBackend::S3),
                "azure" | "azureSAS" => Ok(StorageBackend::Azure),
                "gs" => Ok(StorageBackend::GCS),
                "sd" => Ok(StorageBackend::SeismicDMS),
                _ => Err(VdsError::InvalidUrl(format!("Unknown scheme: {}", scheme))),
            }
        } else {
            // Assume file system if no scheme
            Ok(StorageBackend::FileSystem)
        }
    }
}

/// Trait for I/O operations with cloud storage or file systems
#[async_trait]
pub trait IOManager: Send + Sync {
    /// Read data from a path
    async fn read(&self, path: &str) -> Result<Bytes>;

    /// Write data to a path
    async fn write(&self, path: &str, data: &[u8]) -> Result<()>;

    /// Check if a path exists
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Delete data at a path
    async fn delete(&self, path: &str) -> Result<()>;

    /// List items with a given prefix
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Get the size of data at a path
    async fn size(&self, path: &str) -> Result<usize>;

    /// Get the backend type
    fn backend(&self) -> StorageBackend;
}

/// File system I/O manager
pub struct FileSystemIOManager {
    base_path: PathBuf,
}

impl FileSystemIOManager {
    /// Create a new file system I/O manager
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Get the full path for a relative path
    fn full_path(&self, path: &str) -> PathBuf {
        self.base_path.join(path)
    }
}

#[async_trait]
impl IOManager for FileSystemIOManager {
    async fn read(&self, path: &str) -> Result<Bytes> {
        let full_path = self.full_path(path);
        let data = fs::read(&full_path).await.map_err(VdsError::Io)?;
        Ok(Bytes::from(data))
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        let full_path = self.full_path(path);

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(VdsError::Io)?;
        }

        let mut file = fs::File::create(&full_path).await.map_err(VdsError::Io)?;
        file.write_all(data).await.map_err(VdsError::Io)?;
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.full_path(path);
        Ok(full_path.exists())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.full_path(path);
        fs::remove_file(&full_path).await.map_err(VdsError::Io)?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let full_path = self.full_path(prefix);
        let mut entries = Vec::new();

        if full_path.is_dir() {
            let mut read_dir = fs::read_dir(&full_path).await.map_err(VdsError::Io)?;

            while let Some(entry) = read_dir.next_entry().await.map_err(VdsError::Io)? {
                if let Some(name) = entry.file_name().to_str() {
                    entries.push(name.to_string());
                }
            }
        }

        Ok(entries)
    }

    async fn size(&self, path: &str) -> Result<usize> {
        let full_path = self.full_path(path);
        let metadata = fs::metadata(&full_path).await.map_err(VdsError::Io)?;
        Ok(metadata.len() as usize)
    }

    fn backend(&self) -> StorageBackend {
        StorageBackend::FileSystem
    }
}

/// Parse URL and create appropriate I/O manager
///
/// Only filesystem URLs are supported by openvds-rs. For cloud storage (S3, Azure, GCS, OSDU),
/// implement the `IOManager` trait in your application. See `CLOUD_STORAGE.md` for guidance.
pub async fn create_io_manager(url: &str) -> Result<Box<dyn IOManager>> {
    let backend = StorageBackend::from_url(url)?;

    match backend {
        StorageBackend::FileSystem => {
            // Extract path from file:// URL or use as-is
            let path = url.strip_prefix("file://").unwrap_or(url);
            Ok(Box::new(FileSystemIOManager::new(path)))
        }
        StorageBackend::S3 | StorageBackend::Azure | StorageBackend::GCS | StorageBackend::SeismicDMS => {
            Err(VdsError::Configuration(
                format!(
                    "Cloud backend {:?} is not supported by openvds-rs. \
                    Consuming applications should implement the IOManager trait \
                    for their cloud storage needs. See CLOUD_STORAGE.md for implementation examples.",
                    backend
                )
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_system_io() {
        let temp_dir = TempDir::new().unwrap();
        let io = FileSystemIOManager::new(temp_dir.path());

        // Write
        let data = b"Hello, OpenVDS!";
        io.write("test.dat", data).await.unwrap();

        // Read
        let read_data = io.read("test.dat").await.unwrap();
        assert_eq!(&read_data[..], data);

        // Exists
        assert!(io.exists("test.dat").await.unwrap());
        assert!(!io.exists("nonexistent.dat").await.unwrap());

        // Size
        assert_eq!(io.size("test.dat").await.unwrap(), data.len());

        // Delete
        io.delete("test.dat").await.unwrap();
        assert!(!io.exists("test.dat").await.unwrap());
    }

    #[test]
    fn test_backend_from_url() {
        assert_eq!(
            StorageBackend::from_url("file:///data/volume").unwrap(),
            StorageBackend::FileSystem
        );
        assert_eq!(
            StorageBackend::from_url("s3://bucket/volume").unwrap(),
            StorageBackend::S3
        );
        assert_eq!(
            StorageBackend::from_url("azure://container/volume").unwrap(),
            StorageBackend::Azure
        );
        assert_eq!(
            StorageBackend::from_url("gs://bucket/volume").unwrap(),
            StorageBackend::GCS
        );
    }
}
