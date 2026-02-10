# Custom IOManager Implementation Guide

This guide shows how to implement the `IOManager` trait for custom storage backends.

## Overview

The `IOManager` trait provides the storage abstraction for openvds-rs. By implementing this trait, you can add support for:

- Cloud object storage (S3, Azure Blob Storage, Google Cloud Storage)
- Network file systems (NFS, SMB/CIFS)
- Custom caching layers
- Mock storage for testing
- OSDU SeismicDMS integration
- Any other storage system

## The IOManager Trait

```rust
use async_trait::async_trait;
use bytes::Bytes;
use openvds::{Result, StorageBackend};

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
```

## Minimal Example: In-Memory Storage

```rust
use openvds::{IOManager, StorageBackend, Result, VdsError};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct InMemoryIOManager {
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl InMemoryIOManager {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl IOManager for InMemoryIOManager {
    async fn read(&self, path: &str) -> Result<Bytes> {
        let storage = self.storage.lock().unwrap();
        storage
            .get(path)
            .map(|data| Bytes::from(data.clone()))
            .ok_or_else(|| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path not found: {}", path)
            )))
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.contains_key(path))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(path);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let storage = self.storage.lock().unwrap();
        let results: Vec<String> = storage
            .keys()
            .filter(|k| k.starts_with(prefix))
            .map(|k| k.strip_prefix(prefix).unwrap_or(k).to_string())
            .collect();
        Ok(results)
    }

    async fn size(&self, path: &str) -> Result<usize> {
        let storage = self.storage.lock().unwrap();
        storage
            .get(path)
            .map(|data| data.len())
            .ok_or_else(|| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path not found: {}", path)
            )))
    }

    fn backend(&self) -> StorageBackend {
        StorageBackend::FileSystem
    }
}
```

## Using Your Custom IOManager

```rust
use openvds::{VolumeDataAccess, VdsMetadata, VolumeDataLayout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your custom IOManager
    let io = Box::new(InMemoryIOManager::new());

    // Create metadata
    let layout = VolumeDataLayout::new(/* ... */)?;
    let metadata = VdsMetadata::new(layout);

    // Use it with VolumeDataAccess
    // Note: You'd need to add this method to VolumeDataAccess
    // or manually construct with your IOManager
    let vds = VolumeDataAccess::create_with_io_manager(io, metadata).await?;

    // Now use VDS operations normally
    let data = vds.read_slice(&[0, 0, 0], &[100, 100, 1]).await?;

    Ok(())
}
```

## Testing with Mock Storage

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_volume_operations() {
        let io = Box::new(InMemoryIOManager::new());

        // Write some test data
        io.write("metadata.json", b"{}").await.unwrap();

        // Verify it exists
        assert!(io.exists("metadata.json").await.unwrap());

        // Read it back
        let data = io.read("metadata.json").await.unwrap();
        assert_eq!(&data[..], b"{}");

        // List files
        let files = io.list("").await.unwrap();
        assert_eq!(files.len(), 1);
    }
}
```

## Real-World Examples

For production-ready cloud storage implementations, see:

- **[CLOUD_STORAGE.md](../CLOUD_STORAGE.md)** - Complete S3, Azure, and GCS implementations
- **S3 Example** - Using modern `aws-sdk-s3` with authentication
- **Azure Example** - Using `azure_storage_blobs` with SAS tokens
- **GCS Example** - Using `google-cloud-storage` with service accounts

## Best Practices

### 1. Error Handling

Convert storage-specific errors to VdsError:

```rust
async fn read(&self, path: &str) -> Result<Bytes> {
    self.s3_client
        .get_object()
        .bucket(&self.bucket)
        .key(path)
        .send()
        .await
        .map_err(|e| VdsError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("S3 error: {}", e)
        )))?;
    // ...
}
```

### 2. Path Handling

Handle prefixes consistently:

```rust
fn full_path(&self, path: &str) -> String {
    if self.prefix.is_empty() {
        path.to_string()
    } else {
        format!("{}/{}", self.prefix.trim_end_matches('/'), path)
    }
}
```

### 3. Async All the Way

Never block in IOManager methods - use async APIs:

```rust
// Good: async I/O
async fn read(&self, path: &str) -> Result<Bytes> {
    tokio::fs::read(path).await?
}

// Bad: blocking I/O
async fn read(&self, path: &str) -> Result<Bytes> {
    std::fs::read(path)?  // Blocks the async runtime!
}
```

### 4. Thread Safety

IOManager must be Send + Sync:

```rust
// Use Arc<Mutex<>> for shared mutable state
storage: Arc<Mutex<HashMap<String, Vec<u8>>>>

// Or use async-aware locks
storage: Arc<tokio::sync::RwLock<HashMap<String, Vec<u8>>>>
```

### 5. Performance

Add caching for frequently accessed data:

```rust
pub struct CachedIOManager<T: IOManager> {
    inner: T,
    cache: Arc<Mutex<lru::LruCache<String, Bytes>>>,
}

#[async_trait]
impl<T: IOManager + Send + Sync> IOManager for CachedIOManager<T> {
    async fn read(&self, path: &str) -> Result<Bytes> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(data) = cache.get(path) {
                return Ok(data.clone());
            }
        }

        // Cache miss - fetch from backing storage
        let data = self.inner.read(path).await?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(path.to_string(), data.clone());
        }

        Ok(data)
    }

    // Delegate other methods to inner
    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        self.inner.write(path, data).await
    }

    // ... other methods
}
```

## Integration with VolumeDataAccess

Currently, VolumeDataAccess uses `create_io_manager()` internally. To use a custom IOManager, you may need to:

1. Add a constructor that accepts `Box<dyn IOManager>`
2. Or modify the volume URL parsing to support custom schemes

Example addition to VolumeDataAccess:

```rust
impl VolumeDataAccess {
    /// Create a volume with a custom IOManager
    pub async fn create_with_io_manager(
        io_manager: Box<dyn IOManager>,
        metadata: VdsMetadata,
    ) -> Result<Self> {
        // Implementation that uses the provided IOManager
        // instead of calling create_io_manager()
    }

    /// Open a volume with a custom IOManager
    pub async fn open_with_io_manager(
        io_manager: Box<dyn IOManager>,
        volume_id: &str,
    ) -> Result<Self> {
        // Implementation that uses the provided IOManager
    }
}
```

## Additional Resources

- [async-trait crate](https://docs.rs/async-trait/) - For async trait methods
- [bytes crate](https://docs.rs/bytes/) - Efficient byte handling
- [tokio documentation](https://docs.rs/tokio/) - Async runtime
- [CLOUD_STORAGE.md](../CLOUD_STORAGE.md) - Complete cloud storage examples
