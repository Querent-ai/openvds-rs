# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-10

### BREAKING CHANGES
- **Removed cloud storage backend features** (`aws`, `azure`, `gcs`) from Cargo.toml
- **Removed cloud SDK dependencies** (rusoto_core, rusoto_s3, azure_storage_blobs, google-cloud-storage)
- **Changed scope**: openvds-rs is now a pure OpenVDS format library focused on local filesystem I/O
- **Cloud storage integration**: Applications must now implement the `IOManager` trait for cloud storage needs
- **Default features**: Changed from `["aws", "azure", "gcs"]` to `[]` (no default features)

### Added
- `rust-toolchain.toml` file pinning to stable Rust channel for consistent development environment
- GitHub Actions CI pipeline with comprehensive testing:
  - Format checking with rustfmt
  - Linting with clippy across feature combinations
  - Cross-platform testing (Linux, macOS, Windows)
  - Rust version matrix (stable, beta, MSRV)
  - Documentation build verification
- `CLOUD_STORAGE.md` - comprehensive guide for implementing IOManager trait with cloud providers
- `examples/custom_io_backend.md` - documentation for custom storage backend implementation patterns
- MSRV (Minimum Supported Rust Version) policy: 1.70
- `CHANGELOG.md` to track project changes

### Changed
- Updated package description to clarify focus on local filesystem support
- Improved error messages when attempting to use cloud backend URLs (s3://, azure://, gs://, sd://)
  - Now clearly directs users to implement IOManager trait for cloud storage
- Updated README.md to reflect library scope and extensibility approach
- Updated ARCHITECTURE.md to remove cloud completion roadmap, added extension patterns
- Updated QUICKSTART.md with IOManager implementation guidance
- Updated module documentation in src/lib.rs to clarify scope
- Simplified `create_io_manager()` function in src/io.rs to only support filesystem
- Removed Cargo.toml keyword "cloud-storage"

### Removed
- Cloud SDK dependencies (outdated rusoto 0.48, azure_storage_blobs, google-cloud-storage)
- Cloud backend feature flags (aws, azure, gcs)
- Stub implementation file src/io/s3.rs
- Feature-based conditional compilation for cloud backends

### Rationale
This change refocuses openvds-rs as a generic, reusable OpenVDS format library:
- **Clearer responsibility**: Format handling vs. storage integration
- **Flexibility**: Applications choose their preferred cloud SDKs and versions
- **Reduced dependencies**: Faster builds, smaller binaries
- **Better auth**: Cloud authentication is application-specific
- **Maintainability**: Smaller surface area, focused scope

Applications needing cloud storage should implement the `IOManager` trait using their preferred cloud SDKs. See `CLOUD_STORAGE.md` for detailed implementation examples.

## [0.1.0] - 2026-02-09

### Added
- Initial implementation of OpenVDS format support
- Core data types: VdsMetadata, VolumeDataLayout, VolumeDataAccess
- Compression algorithms: Deflate (ZIP), Zstd, RLE
- Local filesystem I/O via FileSystemIOManager
- Async I/O throughout using tokio
- Support for up to 6D volumetric data
- Brick-based chunked layout for efficient random access
- IOManager trait for extensible storage backends
- Example: seismic_volume.rs demonstrating volume creation
- Example: concurrent_loading.rs demonstrating async benefits
- Comprehensive documentation: README.md, ARCHITECTURE.md, QUICKSTART.md

### Declared (but not implemented)
- Cloud backend features (aws, azure, gcs) - stub implementations only
- Write operations - planned but not yet implemented
- LOD (Level of Detail) generation - planned
- OSDU SeismicDMS integration - planned
