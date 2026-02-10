//! Core data types for OpenVDS

use serde::{Deserialize, Serialize};
use std::fmt;

/// Data types supported by VDS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DataType {
    /// 1-bit boolean
    U1 = 0,
    /// Unsigned 8-bit integer
    U8 = 1,
    /// Unsigned 16-bit integer
    U16 = 2,
    /// Unsigned 32-bit integer
    U32 = 3,
    /// Unsigned 64-bit integer
    U64 = 4,
    /// Signed 8-bit integer
    I8 = 5,
    /// Signed 16-bit integer
    I16 = 6,
    /// Signed 32-bit integer
    I32 = 7,
    /// Signed 64-bit integer
    I64 = 8,
    /// 32-bit floating point
    F32 = 9,
    /// 64-bit floating point
    F64 = 10,
}

impl DataType {
    /// Size in bytes of this data type
    pub fn size_in_bytes(&self) -> usize {
        match self {
            DataType::U1 => 1, // Stored as full bytes
            DataType::U8 | DataType::I8 => 1,
            DataType::U16 | DataType::I16 => 2,
            DataType::U32 | DataType::I32 | DataType::F32 => 4,
            DataType::U64 | DataType::I64 | DataType::F64 => 8,
        }
    }

    /// Check if this is a floating point type
    pub fn is_float(&self) -> bool {
        matches!(self, DataType::F32 | DataType::F64)
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        !self.is_float()
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Dimension in a volume (up to 6D)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Dimension {
    /// Dimension 0 (typically inline for seismic)
    Dim0 = 0,
    /// Dimension 1 (typically crossline for seismic)
    Dim1 = 1,
    /// Dimension 2 (typically depth/time for seismic)
    Dim2 = 2,
    /// Dimension 3
    Dim3 = 3,
    /// Dimension 4
    Dim4 = 4,
    /// Dimension 5
    Dim5 = 5,
}

impl Dimension {
    /// Convert from usize index
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Dimension::Dim0),
            1 => Some(Dimension::Dim1),
            2 => Some(Dimension::Dim2),
            3 => Some(Dimension::Dim3),
            4 => Some(Dimension::Dim4),
            5 => Some(Dimension::Dim5),
            _ => None,
        }
    }

    /// Convert to usize index
    pub fn to_index(&self) -> usize {
        *self as usize
    }
}

/// Axis descriptor with name, unit, and coordinate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisDescriptor {
    /// Number of samples along this axis
    pub num_samples: usize,
    /// Name of the axis (e.g., "Inline", "Crossline", "Depth")
    pub name: String,
    /// Unit of measurement (e.g., "m", "ms", "ft")
    pub unit: String,
    /// Coordinate minimum
    pub coord_min: f64,
    /// Coordinate maximum
    pub coord_max: f64,
}

impl AxisDescriptor {
    /// Create a new axis descriptor
    pub fn new(
        num_samples: usize,
        name: impl Into<String>,
        unit: impl Into<String>,
        coord_min: f64,
        coord_max: f64,
    ) -> Self {
        Self {
            num_samples,
            name: name.into(),
            unit: unit.into(),
            coord_min,
            coord_max,
        }
    }

    /// Get the step size between samples
    pub fn step_size(&self) -> f64 {
        if self.num_samples <= 1 {
            0.0
        } else {
            (self.coord_max - self.coord_min) / (self.num_samples - 1) as f64
        }
    }

    /// Convert sample index to coordinate
    pub fn index_to_coord(&self, index: usize) -> f64 {
        self.coord_min + index as f64 * self.step_size()
    }

    /// Convert coordinate to sample index (nearest)
    pub fn coord_to_index(&self, coord: f64) -> usize {
        let normalized = (coord - self.coord_min) / self.step_size();
        normalized
            .round()
            .max(0.0)
            .min((self.num_samples - 1) as f64) as usize
    }
}

/// Value range for a volume
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValueRange {
    pub min: f64,
    pub max: f64,
}

impl ValueRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn is_valid(&self) -> bool {
        self.min.is_finite() && self.max.is_finite() && self.min <= self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_sizes() {
        assert_eq!(DataType::U8.size_in_bytes(), 1);
        assert_eq!(DataType::U16.size_in_bytes(), 2);
        assert_eq!(DataType::F32.size_in_bytes(), 4);
        assert_eq!(DataType::F64.size_in_bytes(), 8);
    }

    #[test]
    fn test_dimension_conversion() {
        assert_eq!(Dimension::from_index(0), Some(Dimension::Dim0));
        assert_eq!(Dimension::from_index(5), Some(Dimension::Dim5));
        assert_eq!(Dimension::from_index(6), None);

        assert_eq!(Dimension::Dim0.to_index(), 0);
        assert_eq!(Dimension::Dim5.to_index(), 5);
    }

    #[test]
    fn test_axis_descriptor() {
        let axis = AxisDescriptor::new(101, "Depth", "m", 0.0, 1000.0);
        assert_eq!(axis.step_size(), 10.0);
        assert_eq!(axis.index_to_coord(0), 0.0);
        assert_eq!(axis.index_to_coord(100), 1000.0);
        assert_eq!(axis.coord_to_index(500.0), 50);
    }
}
