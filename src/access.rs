//! Volume data access - main API for reading/writing VDS volumes

use crate::compression::get_compressor;
use crate::error::{Result, VdsError};
use crate::io::{create_io_manager, IOManager};
use crate::layout::VolumeDataLayout;
use crate::metadata::VdsMetadata;
use crate::types::DataType;
use crate::utils::brick_path;
use bytes::Bytes;
use futures::future::try_join_all;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Main interface for accessing VDS volume data
pub struct VolumeDataAccess {
    /// Volume metadata
    metadata: Arc<RwLock<VdsMetadata>>,

    /// I/O manager for storage operations
    io_manager: Arc<Box<dyn IOManager>>,
}

impl VolumeDataAccess {
    /// Open an existing VDS volume
    pub async fn open(url: impl Into<String>) -> Result<Self> {
        let url = url.into();
        let io_manager = Arc::new(create_io_manager(&url).await?);

        // Read metadata
        let metadata_bytes = io_manager.read("metadata.json").await?;
        let metadata: VdsMetadata = serde_json::from_slice(&metadata_bytes)
            .map_err(|e| VdsError::Metadata(e.to_string()))?;

        Ok(Self {
            metadata: Arc::new(RwLock::new(metadata)),
            io_manager,
        })
    }

    /// Create a new VDS volume
    pub async fn create(url: impl Into<String>, metadata: VdsMetadata) -> Result<Self> {
        let url = url.into();
        let io_manager = Arc::new(create_io_manager(&url).await?);

        // Write initial metadata
        let metadata_json =
            serde_json::to_vec_pretty(&metadata).map_err(|e| VdsError::Metadata(e.to_string()))?;
        io_manager.write("metadata.json", &metadata_json).await?;

        Ok(Self {
            metadata: Arc::new(RwLock::new(metadata)),
            io_manager,
        })
    }

    /// Get the volume metadata
    pub fn metadata(&self) -> VdsMetadata {
        self.metadata.read().clone()
    }

    /// Get the volume layout
    pub fn layout(&self) -> VolumeDataLayout {
        self.metadata.read().layout.clone()
    }

    /// Read a slice of data
    ///
    /// # Arguments
    /// * `min_coords` - Minimum coordinates (inclusive)
    /// * `max_coords` - Maximum coordinates (exclusive)
    ///
    /// # Returns
    /// Raw bytes containing the data in the slice
    pub async fn read_slice(&self, min_coords: &[usize], max_coords: &[usize]) -> Result<Bytes> {
        let layout = self.layout();

        // Validate coordinates
        if min_coords.len() != layout.dimensionality || max_coords.len() != layout.dimensionality {
            return Err(VdsError::InvalidDimensions(
                "Coordinate dimensions don't match volume dimensionality".to_string(),
            ));
        }

        for i in 0..layout.dimensionality {
            if min_coords[i] >= max_coords[i] {
                return Err(VdsError::InvalidDimensions(
                    "Min coordinates must be less than max coordinates".to_string(),
                ));
            }
            if !layout.is_in_bounds(min_coords) || !layout.is_in_bounds(max_coords) {
                return Err(VdsError::OutOfBounds(
                    "Coordinates out of volume bounds".to_string(),
                ));
            }
        }

        // Determine which bricks overlap with the requested slice
        let brick_indices = self.get_overlapping_bricks(min_coords, max_coords);

        // Read all bricks concurrently
        let bricks = self.read_bricks(&brick_indices).await?;

        // Assemble the slice from bricks
        self.assemble_slice(min_coords, max_coords, &brick_indices, &bricks)
    }

    /// Write a slice of data
    pub async fn write_slice(
        &self,
        min_coords: &[usize],
        max_coords: &[usize],
        data: &[u8],
    ) -> Result<()> {
        let layout = self.layout();

        // Validate coordinates and data size
        if min_coords.len() != layout.dimensionality || max_coords.len() != layout.dimensionality {
            return Err(VdsError::InvalidDimensions(
                "Coordinate dimensions don't match volume dimensionality".to_string(),
            ));
        }

        let expected_voxels: usize = min_coords
            .iter()
            .zip(max_coords.iter())
            .map(|(min, max)| max - min)
            .product();
        let expected_bytes = expected_voxels * layout.data_type.size_in_bytes();

        if data.len() != expected_bytes {
            return Err(VdsError::InvalidDimensions(format!(
                "Data size mismatch: expected {} bytes, got {}",
                expected_bytes,
                data.len()
            )));
        }

        // This is a simplified implementation - in practice you'd need to:
        // 1. Read overlapping bricks
        // 2. Modify them with new data
        // 3. Write them back
        // For now, just return unimplemented
        Err(VdsError::Configuration(
            "Write operations not yet fully implemented".to_string(),
        ))
    }

    /// Read specific bricks by their indices
    async fn read_bricks(&self, indices: &[usize]) -> Result<HashMap<usize, Vec<u8>>> {
        {
            let metadata = self.metadata.read();
            let _compressor = get_compressor(metadata.compression);
        }

        // Read all bricks concurrently
        let futures: Vec<_> = indices
            .iter()
            .map(|&index| {
                let io_manager = Arc::clone(&self.io_manager);
                let compressor = get_compressor(self.metadata.read().compression);

                async move {
                    let path = brick_path(index, 0);
                    let compressed = io_manager.read(&path).await?;
                    let decompressed = compressor.decompress(&compressed, None)?;
                    Ok::<_, VdsError>((index, decompressed))
                }
            })
            .collect();

        let results = try_join_all(futures).await?;
        Ok(results.into_iter().collect())
    }

    /// Get brick indices that overlap with a slice
    fn get_overlapping_bricks(&self, min_coords: &[usize], max_coords: &[usize]) -> Vec<usize> {
        let layout = self.layout();
        let _brick_count = layout.brick_count();
        let mut brick_indices = Vec::new();

        // Calculate min/max brick coordinates
        let min_brick: Vec<usize> = min_coords
            .iter()
            .enumerate()
            .map(|(i, &coord)| coord / layout.brick_size.get(i))
            .collect();

        let max_brick: Vec<usize> = max_coords
            .iter()
            .enumerate()
            .map(|(i, &coord)| (coord - 1) / layout.brick_size.get(i))
            .collect();

        // Iterate through all overlapping bricks
        self.iterate_brick_range(&min_brick, &max_brick, &mut |coords| {
            brick_indices.push(layout.brick_coords_to_index(coords));
        });

        brick_indices
    }

    /// Iterate through a range of brick coordinates
    fn iterate_brick_range<F>(&self, min_brick: &[usize], max_brick: &[usize], callback: &mut F)
    where
        F: FnMut(&[usize]),
    {
        let layout = self.layout();
        let mut coords = min_brick.to_vec();

        loop {
            callback(&coords);

            // Increment coordinates
            let mut dim = layout.dimensionality - 1;
            loop {
                coords[dim] += 1;
                if coords[dim] <= max_brick[dim] {
                    break;
                }
                coords[dim] = min_brick[dim];
                if dim == 0 {
                    return;
                }
                dim -= 1;
            }
        }
    }

    /// Assemble a slice from brick data
    fn assemble_slice(
        &self,
        min_coords: &[usize],
        max_coords: &[usize],
        brick_indices: &[usize],
        bricks: &HashMap<usize, Vec<u8>>,
    ) -> Result<Bytes> {
        let layout = self.layout();

        // Calculate slice dimensions
        let slice_dims: Vec<usize> = min_coords
            .iter()
            .zip(max_coords.iter())
            .map(|(min, max)| max - min)
            .collect();

        let slice_voxels: usize = slice_dims.iter().product();
        let slice_bytes = slice_voxels * layout.data_type.size_in_bytes();
        let mut slice_data = vec![0u8; slice_bytes];

        // This is a simplified implementation
        // In practice, you'd need to properly copy voxels from bricks to the slice
        // accounting for brick boundaries, overlap, etc.

        // For now, just return the first brick's data or empty
        if let Some(&first_index) = brick_indices.first() {
            if let Some(brick_data) = bricks.get(&first_index) {
                let copy_len = slice_data.len().min(brick_data.len());
                slice_data[..copy_len].copy_from_slice(&brick_data[..copy_len]);
            }
        }

        Ok(Bytes::from(slice_data))
    }

    /// Get statistics about the volume
    pub async fn get_stats(&self) -> VolumeStats {
        let layout = self.layout();
        let metadata = self.metadata();

        VolumeStats {
            dimensionality: layout.dimensionality,
            total_voxels: layout.size().iter().product(),
            total_bricks: layout.total_bricks(),
            uncompressed_size: layout.total_size_bytes(),
            data_type: layout.data_type,
            compression_method: metadata.compression,
        }
    }
}

/// Volume statistics
#[derive(Debug, Clone)]
pub struct VolumeStats {
    pub dimensionality: usize,
    pub total_voxels: usize,
    pub total_bricks: usize,
    pub uncompressed_size: usize,
    pub data_type: DataType,
    pub compression_method: crate::compression::CompressionMethod,
}

impl VolumeStats {
    pub fn summary(&self) -> String {
        format!(
            "{}D Volume: {} voxels, {} bricks, {} uncompressed ({:?}, {:?})",
            self.dimensionality,
            self.total_voxels,
            self.total_bricks,
            crate::utils::format_bytes(self.uncompressed_size),
            self.data_type,
            self.compression_method,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AxisDescriptor;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_open_volume() {
        let temp_dir = TempDir::new().unwrap();
        let url = temp_dir.path().to_str().unwrap();

        // Create volume
        let axes = vec![
            AxisDescriptor::new(100, "X", "m", 0.0, 99.0),
            AxisDescriptor::new(100, "Y", "m", 0.0, 99.0),
            AxisDescriptor::new(100, "Z", "m", 0.0, 99.0),
        ];
        let layout = VolumeDataLayout::new(3, DataType::F32, axes).unwrap();
        let metadata = VdsMetadata::new(layout);

        let _vds = VolumeDataAccess::create(url, metadata).await.unwrap();

        // Open volume
        let vds = VolumeDataAccess::open(url).await.unwrap();
        let stats = vds.get_stats().await;
        assert_eq!(stats.dimensionality, 3);
        assert_eq!(stats.total_voxels, 100 * 100 * 100);
    }
}
