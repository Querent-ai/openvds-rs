# OpenVDS-Rust

A pure Rust implementation of OpenVDS (Open Volume Data Store) - a specification for fast random access to multi-dimensional volumetric data.

## Why Rust?

This implementation focuses on **async-first I/O** performance:

- **True async I/O**: Handle thousands of concurrent brick reads efficiently
- **Zero-copy where possible**: Leverage Rust's `Bytes` and zero-copy deserialization
- **Memory safety**: No undefined behavior, data races, or memory leaks
- **Extensible I/O**: Trait-based storage abstraction for custom backends

### Performance Characteristics

```rust
// Efficient concurrent brick loading - single thread, 1000s of concurrent ops
let brick_futures: Vec<_> = brick_ids
    .iter()
    .map(|id| vds.read_brick(*id))
    .collect();
let bricks = futures::future::try_join_all(brick_futures).await?;
```

With C++ FFI, you'd need one thread per brick. With async Rust: **one thread, unlimited concurrency**.

## Features

- ✅ Up to 6D volumetric data support
- ✅ Multiple compression algorithms (Deflate, Zstd, RLE)
- ✅ Async I/O throughout
- ✅ Local filesystem backend
- ✅ Extensible IOManager trait for custom storage backends

## Cloud Storage Support

openvds-rs focuses on the OpenVDS format and local I/O. For cloud storage (S3, Azure, GCS, OSDU), implement the `IOManager` trait in your application using your preferred cloud SDK.

See [CLOUD_STORAGE.md](CLOUD_STORAGE.md) for detailed implementation examples.

## Quick Start

```toml
[dependencies]
openvds = "0.2"
tokio = { version = "1.0", features = ["full"] }
```

### Create a Volume

```rust
use openvds::{VolumeDataAccess, VolumeDataLayout, VdsMetadata, AxisDescriptor, DataType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define axes
    let axes = vec![
        AxisDescriptor::new(1000, "Inline", "trace", 0.0, 999.0),
        AxisDescriptor::new(800, "Crossline", "trace", 0.0, 799.0),
        AxisDescriptor::new(500, "Depth", "ms", 0.0, 2000.0),
    ];

    // Create layout
    let layout = VolumeDataLayout::new(3, DataType::F32, axes)?;
    let metadata = VdsMetadata::new(layout);

    // Create volume
    let vds = VolumeDataAccess::create("file:///data/my-volume", metadata).await?;
    
    Ok(())
}
```

### Read Data

```rust
use openvds::VolumeDataAccess;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open existing volume from filesystem
    let vds = VolumeDataAccess::open("file:///data/seismic-volume").await?;

    // Read a slice
    let min_coords = vec![0, 0, 100];
    let max_coords = vec![1000, 800, 101];
    let data = vds.read_slice(&min_coords, &max_coords).await?;

    println!("Read {} bytes", data.len());

    Ok(())
}
```

### Get Volume Info

```rust
let vds = VolumeDataAccess::open("file:///data/my-volume").await?;
let stats = vds.get_stats().await;
println!("{}", stats.summary());
// Output: 3D Volume: 400000000 voxels, 1664 bricks, 1.49 GB uncompressed (F32, Zstd)
```

## Architecture

```
┌─────────────────────────────────────────┐
│      VolumeDataAccess (Public API)      │
│  - open()  - read_slice()  - write()    │
└────────────────┬────────────────────────┘
                 │
    ┌────────────┼─────────────┐
    │            │             │
    ▼            ▼             ▼
┌─────────┐  ┌──────┐    ┌─────────┐
│ Layout  │  │  IO  │    │Compress │
│ Manager │  │ Mgr  │    │  -ion   │
└─────────┘  └───┬──┘    └─────────┘
                 │
                 ▼
          ┌──────────┐
          │FileSystem│  ← Implement IOManager
          └──────────┘    for cloud storage
```

### Key Components

- **VolumeDataLayout**: Manages brick dimensions, LOD levels, axis descriptors
- **IOManager**: Trait for storage backends (filesystem, S3, Azure, GCS)
- **Compression**: Pluggable compression (Deflate, Zstd, RLE)
- **Metadata**: Volume metadata, survey info, SEG-Y compatibility

## Design Decisions

### Async-First

All I/O operations are `async` to enable:
- Concurrent brick loading from cloud storage
- Non-blocking operations
- Efficient resource utilization

### Traits for Extensibility

```rust
#[async_trait]
pub trait IOManager: Send + Sync {
    async fn read(&self, path: &str) -> Result<Bytes>;
    async fn write(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    async fn size(&self, path: &str) -> Result<usize>;
    fn backend(&self) -> StorageBackend;
}
```

Implement this trait to add support for:
- Cloud object storage (S3, Azure Blob Storage, Google Cloud Storage)
- Network file systems (NFS, SMB)
- Custom caching layers
- Mock storage for testing
- OSDU SeismicDMS integration

See [CLOUD_STORAGE.md](CLOUD_STORAGE.md) for complete implementation examples.

### Type Safety

Rust's type system prevents:
- Buffer overflows
- Use-after-free
- Data races
- Integer overflow (in debug mode)

## Benchmarks

```bash
cargo bench
```

Expected performance (on modern hardware):
- Deflate decompression: ~500 MB/s per core
- Zstd decompression: ~800 MB/s per core
- Concurrent brick loading: Limited by network, not CPU

## Comparison with C++ OpenVDS

| Feature | C++ OpenVDS | Rust OpenVDS |
|---------|-------------|--------------|
| Async I/O | ❌ Blocking | ✅ Native async |
| Memory Safety | Manual | ✅ Guaranteed |
| Concurrent Bricks | Thread per request | ✅ Async task |
| Wavelet Compression | ✅ Proprietary | ❌ Not available |
| Cloud Backends | Built-in | ✅ Via IOManager trait |

## Roadmap

- [ ] Write operations support
- [ ] LOD (Level of Detail) generation
- [ ] SEG-Y import/export tools
- [ ] Python bindings (PyO3)
- [ ] WASM support for browser
- [ ] Advanced caching strategies
- [ ] Parallel decompression with Rayon
- [ ] Example IOManager implementations (S3, Azure, GCS)

## Testing

Sample VDS files for testing are available from the [OpenVDS test suite](https://community.opengroup.org/osdu/platform/domain-data-mgmt-services/seismic/open-vds/-/tree/master/tests/VDS?ref_type=heads).

## Contributing

Contributions welcome! This is a from-scratch implementation focusing on modern async Rust patterns.

## License

Apache 2.0 (same as original OpenVDS)
