# OpenVDS Rust - Quick Start

## What You Got

A pure OpenVDS format library in Rust - focused on local filesystem I/O with extensible storage abstraction.

## Try It Out

```bash
cd openvds-rs

# Check it compiles (needs Rust installed)
cargo check

# Run tests
cargo test

# Run examples
cargo run --example seismic_volume
cargo run --example concurrent_loading

# Build for release
cargo build --release
```

## Project Structure

```
openvds-rs/
├── Cargo.toml              # Dependencies & metadata
├── README.md               # User documentation
├── ARCHITECTURE.md         # Design decisions & internals
├── CLOUD_STORAGE.md        # Guide for implementing cloud backends
├── src/
│   ├── lib.rs             # Main entry point
│   ├── error.rs           # Error types
│   ├── types.rs           # Core data types
│   ├── compression.rs     # Compression algorithms
│   ├── layout.rs          # Volume layout & bricks
│   ├── io.rs              # I/O manager trait & filesystem impl
│   ├── metadata.rs        # VDS metadata
│   ├── access.rs          # Main API
│   └── utils.rs           # Helper functions
└── examples/
    ├── seismic_volume.rs      # Basic usage
    └── concurrent_loading.rs  # Async demo
```

## Key Files to Read

1. **README.md** - Overview and examples
2. **ARCHITECTURE.md** - Why Rust, design decisions
3. **src/access.rs** - Main API you'd use
4. **examples/** - Working code samples

## Next Steps

### Extending with Custom Storage

openvds-rs provides the format library. For cloud storage or custom backends, implement the `IOManager` trait:

```rust
use openvds::{IOManager, StorageBackend};
use async_trait::async_trait;
use bytes::Bytes;

pub struct MyCustomStorage {
    // Your storage client/config
}

#[async_trait]
impl IOManager for MyCustomStorage {
    async fn read(&self, path: &str) -> Result<Bytes> {
        // Implement read from your storage
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<()> {
        // Implement write to your storage
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        // Check if path exists
    }

    async fn delete(&self, path: &str) -> Result<()> {
        // Delete from storage
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        // List items with prefix
    }

    async fn size(&self, path: &str) -> Result<usize> {
        // Get size of item
    }

    fn backend(&self) -> StorageBackend {
        // Return appropriate backend type
    }
}
```

**See [CLOUD_STORAGE.md](CLOUD_STORAGE.md) for complete S3, Azure, and GCS examples.**

### Adding Write Operations

Contribute to openvds-rs by implementing:
- Brick writing in `VolumeDataAccess`
- Compression before write
- Metadata updates
- LOD generation

### Python Bindings

```rust
// Use PyO3 for Python bindings
#[pyclass]
struct PyVolumeDataAccess {
    inner: VolumeDataAccess,
}

#[pymethods]
impl PyVolumeDataAccess {
    #[new]
    fn new(url: String) -> PyResult<Self> {
        // Wrap async with pyo3-asyncio
    }
}
```

## Why This Approach

**For Performance:**
- ✅ 10-100x faster concurrent I/O potential
- ✅ 100x less memory overhead vs threads
- ✅ Native async throughout

**For Flexibility:**
- ✅ Choose your own cloud SDKs
- ✅ Application-specific auth
- ✅ No dependency lock-in

**For Safety:**
- ✅ No segfaults
- ✅ No data races
- ✅ No memory leaks

**For Development:**
- ✅ Better error messages
- ✅ Easier testing (mock IOManager)
- ✅ Cross-platform builds

## Questions?

Read ARCHITECTURE.md for:
- Performance comparisons
- Design rationale
- Implementation details
- Missing features roadmap

Enjoy! 🦀
