//! Volume data layout - manages how volumes are divided into chunks/bricks

use crate::error::{Result, VdsError};
use crate::types::{AxisDescriptor, DataType};
use serde::{Deserialize, Serialize};

/// Size of a brick in each dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrickSize {
    dims: [usize; 6],
}

impl BrickSize {
    /// Create a new brick size
    pub fn new(dims: [usize; 6]) -> Self {
        Self { dims }
    }

    /// Create a brick size for a specific dimensionality
    pub fn with_dimensionality(dimensionality: usize, size: usize) -> Result<Self> {
        if dimensionality > 6 {
            return Err(VdsError::InvalidDimensions(
                "Dimensionality must be <= 6".to_string(),
            ));
        }

        let mut dims = [1; 6];
        for item in dims.iter_mut().take(dimensionality) {
            *item = size;
        }
        Ok(Self { dims })
    }

    /// Get the size for a specific dimension
    pub fn get(&self, dim: usize) -> usize {
        if dim < 6 {
            self.dims[dim]
        } else {
            1
        }
    }

    /// Get all dimensions
    pub fn dims(&self) -> &[usize; 6] {
        &self.dims
    }

    /// Total number of voxels in a brick
    pub fn total_voxels(&self) -> usize {
        self.dims.iter().product()
    }
}

impl Default for BrickSize {
    fn default() -> Self {
        // Common default: 64x64x64 for 3D data
        Self::new([64, 64, 64, 1, 1, 1])
    }
}

/// Layout of volume data - describes how the volume is organized
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeDataLayout {
    /// Dimensionality (1-6)
    pub dimensionality: usize,

    /// Data type of the volume
    pub data_type: DataType,

    /// Axis descriptors for each dimension
    pub axes: Vec<AxisDescriptor>,

    /// Brick size for chunking
    pub brick_size: BrickSize,

    /// Number of LOD (Level of Detail) levels
    pub lod_levels: usize,

    /// Negative margin (overlap) in each dimension
    pub negative_margin: [usize; 6],

    /// Positive margin (overlap) in each dimension
    pub positive_margin: [usize; 6],
}

impl VolumeDataLayout {
    /// Create a new volume data layout
    pub fn new(
        dimensionality: usize,
        data_type: DataType,
        axes: Vec<AxisDescriptor>,
    ) -> Result<Self> {
        if dimensionality == 0 || dimensionality > 6 {
            return Err(VdsError::InvalidDimensions(
                "Dimensionality must be between 1 and 6".to_string(),
            ));
        }

        if axes.len() != dimensionality {
            return Err(VdsError::InvalidDimensions(
                "Number of axes must match dimensionality".to_string(),
            ));
        }

        Ok(Self {
            dimensionality,
            data_type,
            axes,
            brick_size: BrickSize::default(),
            lod_levels: 1,
            negative_margin: [0; 6],
            positive_margin: [0; 6],
        })
    }

    /// Set the brick size
    pub fn with_brick_size(mut self, brick_size: BrickSize) -> Self {
        self.brick_size = brick_size;
        self
    }

    /// Set the number of LOD levels
    pub fn with_lod_levels(mut self, lod_levels: usize) -> Self {
        self.lod_levels = lod_levels;
        self
    }

    /// Set margins
    pub fn with_margins(
        mut self,
        negative_margin: [usize; 6],
        positive_margin: [usize; 6],
    ) -> Self {
        self.negative_margin = negative_margin;
        self.positive_margin = positive_margin;
        self
    }

    /// Get the total size in each dimension
    pub fn size(&self) -> Vec<usize> {
        self.axes.iter().map(|a| a.num_samples).collect()
    }

    /// Get the number of bricks in each dimension
    pub fn brick_count(&self) -> Vec<usize> {
        self.axes
            .iter()
            .enumerate()
            .map(|(i, axis)| {
                let brick_dim = self.brick_size.get(i);
                (axis.num_samples + brick_dim - 1) / brick_dim
            })
            .collect()
    }

    /// Get the total number of bricks
    pub fn total_bricks(&self) -> usize {
        self.brick_count().iter().product()
    }

    /// Convert a brick index to brick coordinates
    pub fn brick_index_to_coords(&self, index: usize) -> Vec<usize> {
        let brick_count = self.brick_count();
        let mut coords = vec![0; self.dimensionality];
        let mut remaining = index;

        for (i, coord) in coords.iter_mut().enumerate().take(self.dimensionality) {
            let stride: usize = brick_count.iter().skip(i + 1).product();
            *coord = remaining / stride;
            remaining %= stride;
        }

        coords
    }

    /// Convert brick coordinates to a brick index
    pub fn brick_coords_to_index(&self, coords: &[usize]) -> usize {
        let brick_count = self.brick_count();
        let mut index = 0;

        for (i, &coord) in coords.iter().enumerate().take(self.dimensionality) {
            let stride: usize = brick_count.iter().skip(i + 1).product();
            index += coord * stride;
        }

        index
    }

    /// Get the data range for a brick (in voxel coordinates)
    pub fn brick_data_range(&self, brick_coords: &[usize]) -> Vec<(usize, usize)> {
        brick_coords
            .iter()
            .enumerate()
            .map(|(i, &coord)| {
                let brick_dim = self.brick_size.get(i);
                let start = coord * brick_dim;
                let end = (start + brick_dim).min(self.axes[i].num_samples);
                (start, end)
            })
            .collect()
    }

    /// Calculate the size in bytes of a single brick
    pub fn brick_size_bytes(&self) -> usize {
        self.brick_size.total_voxels() * self.data_type.size_in_bytes()
    }

    /// Calculate the total volume size in bytes (uncompressed)
    pub fn total_size_bytes(&self) -> usize {
        let total_voxels: usize = self.axes.iter().map(|a| a.num_samples).product();
        total_voxels * self.data_type.size_in_bytes()
    }

    /// Check if coordinates are within bounds
    pub fn is_in_bounds(&self, coords: &[usize]) -> bool {
        if coords.len() != self.dimensionality {
            return false;
        }

        coords
            .iter()
            .zip(self.axes.iter())
            .all(|(&coord, axis)| coord < axis.num_samples)
    }

    /// Get a summary string of the layout
    pub fn summary(&self) -> String {
        let size_str = self
            .size()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" x ");

        format!(
            "{}D Volume: {} ({:?}), {} bricks, {:.2} MB uncompressed",
            self.dimensionality,
            size_str,
            self.data_type,
            self.total_bricks(),
            self.total_size_bytes() as f64 / (1024.0 * 1024.0)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_layout() -> VolumeDataLayout {
        let axes = vec![
            AxisDescriptor::new(1000, "Inline", "trace", 0.0, 999.0),
            AxisDescriptor::new(800, "Crossline", "trace", 0.0, 799.0),
            AxisDescriptor::new(500, "Depth", "ms", 0.0, 2000.0),
        ];

        VolumeDataLayout::new(3, DataType::F32, axes)
            .unwrap()
            .with_brick_size(BrickSize::new([64, 64, 64, 1, 1, 1]))
    }

    #[test]
    fn test_layout_creation() {
        let layout = create_test_layout();
        assert_eq!(layout.dimensionality, 3);
        assert_eq!(layout.data_type, DataType::F32);
        assert_eq!(layout.size(), vec![1000, 800, 500]);
    }

    #[test]
    fn test_brick_count() {
        let layout = create_test_layout();
        let brick_count = layout.brick_count();
        assert_eq!(brick_count, vec![16, 13, 8]); // ceil(1000/64), ceil(800/64), ceil(500/64)
    }

    #[test]
    fn test_total_bricks() {
        let layout = create_test_layout();
        assert_eq!(layout.total_bricks(), 16 * 13 * 8);
    }

    #[test]
    fn test_brick_index_conversion() {
        let layout = create_test_layout();
        let coords = vec![5, 7, 3];
        let index = layout.brick_coords_to_index(&coords);
        let recovered = layout.brick_index_to_coords(index);
        assert_eq!(coords, recovered);
    }

    #[test]
    fn test_brick_data_range() {
        let layout = create_test_layout();
        let coords = vec![0, 0, 0];
        let range = layout.brick_data_range(&coords);
        assert_eq!(range, vec![(0, 64), (0, 64), (0, 64)]);

        // Last brick should be trimmed
        let coords = vec![15, 12, 7];
        let range = layout.brick_data_range(&coords);
        assert_eq!(range, vec![(960, 1000), (768, 800), (448, 500)]);
    }

    #[test]
    fn test_is_in_bounds() {
        let layout = create_test_layout();
        assert!(layout.is_in_bounds(&[0, 0, 0]));
        assert!(layout.is_in_bounds(&[999, 799, 499]));
        assert!(!layout.is_in_bounds(&[1000, 0, 0]));
        assert!(!layout.is_in_bounds(&[0, 800, 0]));
    }
}
