# Cloud Storage Integration Guide

## Overview

**openvds-rs** focuses exclusively on the OpenVDS format specification and local filesystem I/O. Cloud storage integration (AWS S3, Azure Blob Storage, Google Cloud Storage, OSDU SeismicDMS) is intentionally left to consuming applications.

### Why This Design?

1. **Flexibility**: Applications can choose their preferred cloud SDKs and versions
2. **Authentication**: Cloud auth is highly application-specific (IAM roles, service principals, credentials, etc.)
3. **Dependencies**: Avoids forcing specific SDK versions on all users
4. **Simplicity**: Library stays focused on format handling
5. **Build Time**: Reduces compilation time for applications that don't need cloud storage

## The IOManager Trait

All storage operations in openvds-rs go through the `IOManager` trait:

```rust
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

## Implementation Examples

### AWS S3 Backend

Using the modern `aws-sdk-s3` (recommended):

```rust
use openvds::{IOManager, StorageBackend, Result, VdsError};
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use async_trait::async_trait;
use bytes::Bytes;

pub struct S3IOManager {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3IOManager {
    /// Create a new S3 IOManager
    ///
    /// # Example
    /// ```ignore
    /// let config = aws_config::load_from_env().await;
    /// let client = Client::new(&config);
    /// let io = S3IOManager::new(client, "my-bucket", "volumes/seismic/");
    /// ```
    pub fn new(client: Client, bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            client,
            bucket: bucket.into(),
            prefix: prefix.into(),
        }
    }

    fn full_key(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}{}", self.prefix, path)
        }
    }
}

#[async_trait]
impl IOManager for S3IOManager {
    async fn read(&self, path: &str) -> Result<Bytes> {
        let key = self.full_key(path);

        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("S3 GetObject failed: {}", e)
            )))?;

        let data = resp.body
            .collect()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read S3 response body: {}", e)
            )))?;

        Ok(data.into_bytes())
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        let key = self.full_key(path);
        let stream = ByteStream::from(data.to_vec());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(stream)
            .send()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("S3 PutObject failed: {}", e)
            )))?;

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let key = self.full_key(path);

        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a 404 Not Found
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(VdsError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("S3 HeadObject failed: {}", e)
                    )))
                }
            }
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let key = self.full_key(path);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("S3 DeleteObject failed: {}", e)
            )))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let full_prefix = self.full_key(prefix);
        let mut results = Vec::new();

        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&full_prefix)
            .send()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("S3 ListObjectsV2 failed: {}", e)
            )))?;

        if let Some(contents) = resp.contents() {
            for obj in contents {
                if let Some(key) = obj.key() {
                    // Strip the prefix to return relative paths
                    if let Some(relative) = key.strip_prefix(&full_prefix) {
                        results.push(relative.to_string());
                    }
                }
            }
        }

        Ok(results)
    }

    async fn size(&self, path: &str) -> Result<usize> {
        let key = self.full_key(path);

        let resp = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("S3 HeadObject failed: {}", e)
            )))?;

        Ok(resp.content_length().unwrap_or(0) as usize)
    }

    fn backend(&self) -> StorageBackend {
        StorageBackend::S3
    }
}
```

**Cargo.toml dependencies:**
```toml
[dependencies]
aws-config = "1.1"
aws-sdk-s3 = "1.17"
```

**Usage example:**
```rust
use aws_config;
use aws_sdk_s3::Client;
use openvds::{VolumeDataAccess, VdsMetadata};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load AWS configuration (from environment, profile, or IAM role)
    let config = aws_config::load_from_env().await;
    let s3_client = Client::new(&config);

    // Create S3 IOManager
    let io = Box::new(S3IOManager::new(
        s3_client,
        "my-seismic-data-bucket",
        "volumes/north-sea/"
    ));

    // Use with openvds
    let vds = VolumeDataAccess::open_with_io_manager(io, "volume-001").await?;

    // Read slices, etc.
    let data = vds.read_slice(&[0, 0, 0], &[100, 100, 1]).await?;

    Ok(())
}
```

### Azure Blob Storage Backend

Using `azure_storage` and `azure_storage_blobs`:

```rust
use openvds::{IOManager, StorageBackend, Result, VdsError};
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;

pub struct AzureIOManager {
    container_client: ContainerClient,
    prefix: String,
}

impl AzureIOManager {
    /// Create a new Azure Blob Storage IOManager
    ///
    /// # Example
    /// ```ignore
    /// let account = "mystorageaccount";
    /// let access_key = std::env::var("AZURE_STORAGE_ACCESS_KEY")?;
    /// let credentials = StorageCredentials::access_key(account, access_key);
    ///
    /// let io = AzureIOManager::new(credentials, "seismic-volumes", "north-sea/");
    /// ```
    pub fn new(
        credentials: StorageCredentials,
        container: impl Into<String>,
        prefix: impl Into<String>,
    ) -> Self {
        let container_name = container.into();
        let container_client = BlobServiceClient::new(
            credentials.account(),
            credentials
        ).container_client(&container_name);

        Self {
            container_client,
            prefix: prefix.into(),
        }
    }

    fn full_path(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}{}", self.prefix, path)
        }
    }
}

#[async_trait]
impl IOManager for AzureIOManager {
    async fn read(&self, path: &str) -> Result<Bytes> {
        let blob_name = self.full_path(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        let mut stream = blob_client.get().into_stream();
        let mut data = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| VdsError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Azure blob read failed: {}", e)
                )))?;
            data.extend_from_slice(&chunk.data);
        }

        Ok(Bytes::from(data))
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        let blob_name = self.full_path(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        blob_client
            .put_block_blob(data.to_vec())
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Azure blob write failed: {}", e)
            )))?;

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let blob_name = self.full_path(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        match blob_client.get_properties().await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(VdsError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Azure blob exists check failed: {}", e)
                    )))
                }
            }
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let blob_name = self.full_path(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        blob_client
            .delete()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Azure blob delete failed: {}", e)
            )))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let full_prefix = self.full_path(prefix);
        let mut results = Vec::new();

        let mut stream = self.container_client
            .list_blobs()
            .prefix(&full_prefix)
            .into_stream();

        while let Some(response) = stream.next().await {
            let response = response
                .map_err(|e| VdsError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Azure list blobs failed: {}", e)
                )))?;

            for blob in response.blobs.blobs() {
                if let Some(relative) = blob.name.strip_prefix(&full_prefix) {
                    results.push(relative.to_string());
                }
            }
        }

        Ok(results)
    }

    async fn size(&self, path: &str) -> Result<usize> {
        let blob_name = self.full_path(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        let props = blob_client
            .get_properties()
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Azure get properties failed: {}", e)
            )))?;

        Ok(props.blob.properties.content_length as usize)
    }

    fn backend(&self) -> StorageBackend {
        StorageBackend::Azure
    }
}
```

**Cargo.toml dependencies:**
```toml
[dependencies]
azure_storage = "0.20"
azure_storage_blobs = "0.20"
```

### Google Cloud Storage Backend

Using `google-cloud-storage`:

```rust
use openvds::{IOManager, StorageBackend, Result, VdsError};
use google_cloud_storage::client::Client;
use google_cloud_storage::http::objects::{get::GetObjectRequest, upload::UploadObjectRequest};
use async_trait::async_trait;
use bytes::Bytes;

pub struct GcsIOManager {
    client: Client,
    bucket: String,
    prefix: String,
}

impl GcsIOManager {
    pub async fn new(bucket: impl Into<String>, prefix: impl Into<String>) -> Result<Self> {
        let client = Client::default()
            .await
            .map_err(|e| VdsError::Configuration(format!("GCS client init failed: {}", e)))?;

        Ok(Self {
            client,
            bucket: bucket.into(),
            prefix: prefix.into(),
        })
    }

    fn full_path(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}{}", self.prefix, path)
        }
    }
}

#[async_trait]
impl IOManager for GcsIOManager {
    async fn read(&self, path: &str) -> Result<Bytes> {
        let object_name = self.full_path(path);

        let data = self.client
            .download_object(&GetObjectRequest {
                bucket: self.bucket.clone(),
                object: object_name,
                ..Default::default()
            }, &Default::default())
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("GCS download failed: {}", e)
            )))?;

        Ok(Bytes::from(data))
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        let object_name = self.full_path(path);

        self.client
            .upload_object(&UploadObjectRequest {
                bucket: self.bucket.clone(),
                ..Default::default()
            }, data.to_vec(), &Default::default())
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("GCS upload failed: {}", e)
            )))?;

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let object_name = self.full_path(path);

        match self.client
            .get_object(&GetObjectRequest {
                bucket: self.bucket.clone(),
                object: object_name,
                ..Default::default()
            })
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(VdsError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("GCS exists check failed: {}", e)
                    )))
                }
            }
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let object_name = self.full_path(path);

        self.client
            .delete_object(&self.bucket, &object_name)
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("GCS delete failed: {}", e)
            )))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let full_prefix = self.full_path(prefix);

        let objects = self.client
            .list_objects(&self.bucket, Some(&full_prefix))
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("GCS list failed: {}", e)
            )))?;

        let results = objects
            .items
            .iter()
            .filter_map(|obj| {
                obj.name.strip_prefix(&full_prefix).map(|s| s.to_string())
            })
            .collect();

        Ok(results)
    }

    async fn size(&self, path: &str) -> Result<usize> {
        let object_name = self.full_path(path);

        let metadata = self.client
            .get_object(&GetObjectRequest {
                bucket: self.bucket.clone(),
                object: object_name,
                ..Default::default()
            })
            .await
            .map_err(|e| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("GCS metadata failed: {}", e)
            )))?;

        Ok(metadata.size as usize)
    }

    fn backend(&self) -> StorageBackend {
        StorageBackend::GCS
    }
}
```

## Authentication Patterns

### AWS Authentication

The AWS SDK automatically discovers credentials from:
1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS credentials file (`~/.aws/credentials`)
3. IAM roles (for EC2, ECS, Lambda)
4. Web identity tokens (for EKS)

```rust
// Default credential chain (recommended)
let config = aws_config::load_from_env().await;

// Explicit credentials
let config = aws_config::from_env()
    .credentials_provider(Credentials::new(
        access_key_id,
        secret_access_key,
        None, // session_token
        None, // expiry
        "manual"
    ))
    .load()
    .await;
```

### Azure Authentication

```rust
use azure_storage::StorageCredentials;

// From environment variable
let credentials = StorageCredentials::access_key(
    account_name,
    std::env::var("AZURE_STORAGE_ACCESS_KEY")?
);

// Using SAS token
let credentials = StorageCredentials::sas_token(sas_token)?;

// Using Azure AD (service principal)
let credentials = StorageCredentials::token_credential(token_credential);
```

### GCS Authentication

```rust
// Uses Application Default Credentials (ADC)
// 1. GOOGLE_APPLICATION_CREDENTIALS environment variable
// 2. gcloud CLI configuration
// 3. GCE metadata service

let client = Client::default().await?;
```

## Testing Strategies

### Unit Testing with Mocks

```rust
use openvds::IOManager;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct MockIOManager {
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockIOManager {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl IOManager for MockIOManager {
    async fn read(&self, path: &str) -> Result<Bytes> {
        let storage = self.storage.lock().unwrap();
        storage.get(path)
            .map(|data| Bytes::from(data.clone()))
            .ok_or_else(|| VdsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Not found"
            )))
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    // ... implement other methods
}

#[tokio::test]
async fn test_volume_with_mock_storage() {
    let io = Box::new(MockIOManager::new());
    let metadata = VdsMetadata::new(/* ... */);

    let vds = VolumeDataAccess::create_with_io_manager(io, metadata).await.unwrap();
    // Test operations...
}
```

### Integration Testing with LocalStack (AWS)

```bash
# Start LocalStack
docker run -d -p 4566:4566 localstack/localstack

# Set environment
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
```

```rust
#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_s3_integration() {
    let config = aws_config::from_env()
        .endpoint_url("http://localhost:4566")
        .load()
        .await;

    let client = Client::new(&config);
    // ... test with real S3 API calls to LocalStack
}
```

### Integration Testing with Azurite (Azure)

```bash
# Start Azurite
docker run -d -p 10000:10000 mcr.microsoft.com/azure-storage/azurite \
    azurite-blob --blobHost 0.0.0.0
```

## Performance Considerations

### Concurrent Brick Loading

The async nature of openvds-rs shines with cloud storage:

```rust
use futures::stream::{self, StreamExt};

// Load 100 bricks concurrently
let brick_futures: Vec<_> = brick_indices
    .iter()
    .map(|idx| async {
        vds.read_brick(*idx).await
    })
    .collect();

// Execute with concurrency limit of 32
let results = stream::iter(brick_futures)
    .buffer_unordered(32)
    .collect::<Vec<_>>()
    .await;
```

### Caching Layer

Add a caching layer to reduce cloud API calls:

```rust
use lru::LruCache;

pub struct CachedIOManager<T: IOManager> {
    inner: T,
    cache: Arc<Mutex<LruCache<String, Bytes>>>,
}

impl<T: IOManager> CachedIOManager<T> {
    pub fn new(inner: T, capacity: usize) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }
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

        // Cache miss - fetch from cloud
        let data = self.inner.read(path).await?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(path.to_string(), data.clone());
        }

        Ok(data)
    }

    // ... delegate other methods
}
```

## Error Handling Best Practices

### Retry Logic

```rust
use tokio::time::{sleep, Duration};

async fn read_with_retry<M: IOManager>(
    io: &M,
    path: &str,
    max_retries: u32,
) -> Result<Bytes> {
    let mut attempts = 0;

    loop {
        match io.read(path).await {
            Ok(data) => return Ok(data),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                let backoff = Duration::from_millis(100 * 2_u64.pow(attempts));
                eprintln!("Retry {} after {:?}: {}", attempts, backoff, e);
                sleep(backoff).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Exponential Backoff with Jitter

```rust
use rand::Rng;

fn backoff_duration(attempt: u32) -> Duration {
    let base_ms = 100 * 2_u64.pow(attempt);
    let jitter = rand::thread_rng().gen_range(0..base_ms / 2);
    Duration::from_millis(base_ms + jitter)
}
```

## Complete Example Application

See [examples/s3_volume_loader.rs](examples/s3_volume_loader.rs) for a complete working example that:
- Loads AWS credentials from environment
- Creates an S3 IOManager
- Opens a VDS volume from S3
- Performs concurrent slice reads
- Demonstrates error handling and retries

## Additional Resources

- [AWS SDK for Rust Documentation](https://docs.aws.amazon.com/sdk-for-rust/)
- [Azure SDK for Rust](https://github.com/Azure/azure-sdk-for-rust)
- [Google Cloud Client Libraries for Rust](https://github.com/googleapis/google-cloud-rust)
- [OpenVDS Specification](https://community.opengroup.org/osdu/platform/domain-data-mgmt-services/seismic/open-vds)
