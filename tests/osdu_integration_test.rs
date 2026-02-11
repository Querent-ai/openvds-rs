//! Integration tests using real OSDU OpenVDS test data
//!
//! These tests use actual seismic data chunks from the OpenVDS reference implementation
//! to validate compatibility with the OSDU data platform.

use openvds::{
    compression::{get_compressor, CompressionMethod},
    layout::BrickSize,
    types::{AxisDescriptor, DataType},
    VolumeDataLayout,
};
use std::fs;
use std::path::PathBuf;

/// Get the path to test data directory
fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

/// Test that we can parse the real OSDU VolumeDataLayout.json
#[test]
fn test_parse_osdu_volume_layout() {
    let layout_path = test_data_dir().join("VolumeDataLayout.json");
    let json_data = fs::read_to_string(&layout_path).expect("Failed to read VolumeDataLayout.json");

    // Parse the JSON to verify structure
    let parsed: serde_json::Value =
        serde_json::from_str(&json_data).expect("Failed to parse VolumeDataLayout.json");

    // Verify key fields exist
    assert!(parsed["axisDescriptors"].is_array());
    assert!(parsed["channelDescriptors"].is_array());
    assert!(parsed["layoutDescriptor"].is_object());
    assert!(parsed["metadata"].is_array());

    // Verify axis descriptors
    let axes = parsed["axisDescriptors"].as_array().unwrap();
    assert_eq!(axes.len(), 3); // 3D seismic data

    // Sample axis (time/depth)
    assert_eq!(axes[0]["name"], "Sample");
    assert_eq!(axes[0]["numSamples"], 1126);
    assert_eq!(axes[0]["unit"], "ms");

    // Crossline axis
    assert_eq!(axes[1]["name"], "Crossline");
    assert_eq!(axes[1]["numSamples"], 605);

    // Inline axis
    assert_eq!(axes[2]["name"], "Inline");
    assert_eq!(axes[2]["numSamples"], 385);

    println!("✓ Successfully parsed OSDU VolumeDataLayout.json");
    println!(
        "  Dimensions: {} x {} x {}",
        axes[0]["numSamples"], axes[1]["numSamples"], axes[2]["numSamples"]
    );
}

/// Test decompression of uncompressed chunk data
#[test]
fn test_decompress_none_chunk() {
    let chunk_path = test_data_dir().join("chunk.CompressionMethod_None");

    if !chunk_path.exists() {
        eprintln!("Skipping test: chunk file not found at {:?}", chunk_path);
        return;
    }

    let chunk_data = fs::read(&chunk_path).expect("Failed to read chunk file");

    println!("✓ Read uncompressed chunk: {} bytes", chunk_data.len());

    // No compression - data should be raw
    let compressor = get_compressor(CompressionMethod::None);
    let decompressed = compressor
        .decompress(&chunk_data, None)
        .expect("Failed to decompress None chunk");

    // Should be identical for no compression
    assert_eq!(chunk_data, decompressed);
    println!("✓ Verified uncompressed chunk data");
}

/// Test decompression of ZIP compressed chunk
#[test]
fn test_decompress_zip_chunk() {
    let chunk_path = test_data_dir().join("chunk.CompressionMethod_Zip");

    if !chunk_path.exists() {
        eprintln!("Skipping test: chunk file not found at {:?}", chunk_path);
        return;
    }

    let chunk_data = fs::read(&chunk_path).expect("Failed to read chunk file");

    println!("✓ Read ZIP compressed chunk: {} bytes", chunk_data.len());

    // OpenVDS chunk format has a 24-byte header:
    // - 3x uint32: dimensions (100x100x100)
    // - 1x uint32: data type (3 = float32)
    // - 1x uint32: compression method (1 = deflate)
    // - 1x uint32: reserved
    // The actual compressed data starts at byte 24
    const HEADER_SIZE: usize = 24;

    if chunk_data.len() <= HEADER_SIZE {
        panic!("Chunk file too small: {} bytes", chunk_data.len());
    }

    // Skip the header and decompress just the compressed data
    let compressed_data = &chunk_data[HEADER_SIZE..];
    println!("  Header size: {} bytes", HEADER_SIZE);
    println!("  Compressed data: {} bytes", compressed_data.len());

    let compressor = get_compressor(CompressionMethod::Deflate);
    let decompressed = compressor
        .decompress(compressed_data, None)
        .expect("Failed to decompress ZIP chunk");

    println!("✓ Decompressed ZIP chunk: {} bytes", decompressed.len());
    assert!(
        !decompressed.is_empty(),
        "Decompressed data should not be empty"
    );

    // Expected size for 100x100x100 float32 data
    let expected_size = 100 * 100 * 100 * 4; // 4 bytes per float32
    assert_eq!(
        decompressed.len(),
        expected_size,
        "Decompressed size should be {} bytes for 100x100x100 float32 data",
        expected_size
    );
}

/// Test decompression of RLE compressed chunk
#[test]
fn test_decompress_rle_chunk() {
    let chunk_path = test_data_dir().join("chunk.CompressionMethod_RLE");

    if !chunk_path.exists() {
        eprintln!("Skipping test: chunk file not found at {:?}", chunk_path);
        return;
    }

    let chunk_data = fs::read(&chunk_path).expect("Failed to read chunk file");

    println!("✓ Read RLE compressed chunk: {} bytes", chunk_data.len());

    let compressor = get_compressor(CompressionMethod::RLE);
    let decompressed = compressor
        .decompress(&chunk_data, None)
        .expect("Failed to decompress RLE chunk");

    println!("✓ Decompressed RLE chunk: {} bytes", decompressed.len());
    assert!(
        !decompressed.is_empty(),
        "Decompressed data should not be empty"
    );
}

/// Test creating a layout compatible with OSDU seismic data
#[test]
fn test_create_osdu_compatible_layout() {
    // Create axes matching the OSDU test data
    let axes = vec![
        AxisDescriptor::new(1126, "Sample", "ms", 0.0, 4500.0),
        AxisDescriptor::new(605, "Crossline", "unitless", 1932.0, 2536.0),
        AxisDescriptor::new(385, "Inline", "unitless", 9985.0, 10369.0),
    ];

    let layout = VolumeDataLayout::new(3, DataType::F32, axes)
        .expect("Failed to create layout")
        .with_brick_size(BrickSize::new([128, 128, 128, 1, 1, 1]))
        .with_lod_levels(2);

    // Verify layout properties
    assert_eq!(layout.dimensionality, 3);
    assert_eq!(layout.data_type, DataType::F32);
    assert_eq!(layout.lod_levels, 2);
    assert_eq!(layout.size(), vec![1126, 605, 385]);

    // Calculate brick count
    let brick_count = layout.brick_count();
    println!("✓ Created OSDU-compatible layout");
    println!("  Brick count: {:?}", brick_count);
    println!("  Total bricks: {}", layout.total_bricks());
    println!("  Brick size: {} bytes", layout.brick_size_bytes());
    println!(
        "  Total size: {:.2} MB",
        layout.total_size_bytes() as f64 / (1024.0 * 1024.0)
    );
}

/// Test that chunk files for different data types exist and are readable
#[test]
fn test_chunk_files_exist() {
    let test_dir = test_data_dir();

    let chunk_files = vec![
        "chunk.CompressionMethod_None",
        "chunk.CompressionMethod_RLE",
        "chunk.CompressionMethod_Zip",
        "chunk.U8.CompressionMethod_None",
        "chunk.U16.CompressionMethod_None",
    ];

    for chunk_file in chunk_files {
        let path = test_dir.join(chunk_file);
        if path.exists() {
            let metadata = fs::metadata(&path).expect("Failed to read file metadata");
            println!("✓ Found chunk: {} ({} bytes)", chunk_file, metadata.len());
        } else {
            eprintln!("⚠ Chunk not found: {}", chunk_file);
        }
    }
}

/// Benchmark: Compare compression ratios for different methods
#[test]
fn test_compression_comparison() {
    let test_dir = test_data_dir();

    let compression_methods = vec![
        ("None", "chunk.CompressionMethod_None"),
        ("ZIP", "chunk.CompressionMethod_Zip"),
        ("RLE", "chunk.CompressionMethod_RLE"),
    ];

    let mut uncompressed_size = 0;

    println!("\n=== Compression Comparison ===");
    for (method, filename) in compression_methods {
        let path = test_dir.join(filename);
        if !path.exists() {
            continue;
        }

        let size = fs::metadata(&path).unwrap().len();

        if method == "None" {
            uncompressed_size = size;
            println!("{:6} : {:8} bytes (baseline)", method, size);
        } else if uncompressed_size > 0 {
            let ratio = uncompressed_size as f64 / size as f64;
            println!(
                "{:6} : {:8} bytes ({:.2}x compression)",
                method, size, ratio
            );
        }
    }
}
