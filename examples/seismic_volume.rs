//! Example: Create a seismic volume and demonstrate concurrent brick access
//!
//! Run with: cargo run --example seismic_volume

use openvds::{
    AxisDescriptor, BrickSize, CompressionMethod, DataType, VdsMetadata, VolumeDataAccess,
    VolumeDataLayout,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenVDS Rust Example: Seismic Volume");
    println!("=====================================\n");

    // Define a 3D seismic volume
    let axes = vec![
        AxisDescriptor::new(1000, "Inline", "trace", 1000.0, 1999.0),
        AxisDescriptor::new(800, "Crossline", "trace", 2000.0, 2799.0),
        AxisDescriptor::new(500, "Depth", "ms", 0.0, 2000.0),
    ];

    println!("Volume dimensions:");
    println!(
        "  Inline:    {} traces ({:.0} - {:.0})",
        axes[0].num_samples, axes[0].coord_min, axes[0].coord_max
    );
    println!(
        "  Crossline: {} traces ({:.0} - {:.0})",
        axes[1].num_samples, axes[1].coord_min, axes[1].coord_max
    );
    println!(
        "  Depth:     {} samples ({:.0} - {:.0} ms)",
        axes[2].num_samples, axes[2].coord_min, axes[2].coord_max
    );
    println!();

    // Create volume layout with 64x64x64 bricks
    let layout = VolumeDataLayout::new(3, DataType::F32, axes)?
        .with_brick_size(BrickSize::new([64, 64, 64, 1, 1, 1]))
        .with_lod_levels(1);

    println!("Layout info:");
    println!("  {}", layout.summary());
    println!("  Brick size: 64 x 64 x 64");
    println!("  Brick count: {:?}", layout.brick_count());
    println!();

    // Create metadata with Zstd compression
    let mut metadata = VdsMetadata::new(layout).with_compression(CompressionMethod::Zstd);

    metadata.add_metadata("survey", "North Sea 3D");
    metadata.add_metadata("acquisition_year", "2023");
    metadata.add_metadata("processing_contractor", "ExampleCorp");

    // Create volume in a temp directory (uses local filesystem)
    // Note: For cloud storage (S3, Azure, GCS), implement the IOManager trait
    // in your application. See CLOUD_STORAGE.md for examples.
    let temp_dir = tempfile::tempdir()?;
    let volume_path = temp_dir.path().join("seismic-volume");
    println!("Creating volume at: {}", volume_path.display());

    let vds = VolumeDataAccess::create(volume_path.to_str().unwrap(), metadata).await?;

    println!("✓ Volume created successfully\n");

    // Get statistics
    let stats = vds.get_stats().await;
    println!("Volume statistics:");
    println!("  {}", stats.summary());
    println!();

    // Demonstrate async concurrent slice reads (would fail without actual data)
    println!("Demonstrating async API:");
    println!("  (Note: reads would fail as we haven't written data)");

    // This shows the API - in practice you'd write data first
    let slice_futures = vec![
        vds.read_slice(&[0, 0, 0], &[100, 100, 1]),
        vds.read_slice(&[100, 0, 0], &[200, 100, 1]),
        vds.read_slice(&[200, 0, 0], &[300, 100, 1]),
    ];

    println!("  Launching 3 concurrent slice reads...");

    // This is the power of async - all three reads execute concurrently
    // With blocking I/O you'd need 3 threads or sequential reads
    match futures::future::try_join_all(slice_futures).await {
        Ok(slices) => {
            println!("  ✓ Read {} slices concurrently", slices.len());
            for (i, slice) in slices.iter().enumerate() {
                println!("    Slice {}: {} bytes", i, slice.len());
            }
        }
        Err(e) => {
            println!("  ✗ Expected error (no data written): {}", e);
        }
    }

    println!("\n✓ Example complete!");
    Ok(())
}
