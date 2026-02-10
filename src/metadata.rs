//! VDS metadata structures

use crate::compression::CompressionMethod;
use crate::layout::VolumeDataLayout;
use crate::types::ValueRange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// VDS file format version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VdsVersion {
    pub major: u16,
    pub minor: u16,
}

impl VdsVersion {
    pub const CURRENT: Self = Self { major: 3, minor: 0 };

    pub fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl Default for VdsVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

/// Complete metadata for a VDS volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VdsMetadata {
    /// Format version
    pub version: VdsVersion,

    /// Volume data layout
    pub layout: VolumeDataLayout,

    /// Compression method used for bricks
    pub compression: CompressionMethod,

    /// Compression tolerance (for lossy compression)
    pub compression_tolerance: f32,

    /// Value range of the data
    pub value_range: ValueRange,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub modified_at: DateTime<Utc>,

    /// Custom metadata key-value pairs
    pub custom_metadata: HashMap<String, String>,

    /// Survey/acquisition metadata (for seismic data)
    pub survey_metadata: Option<SurveyMetadata>,
}

impl VdsMetadata {
    /// Create new metadata
    pub fn new(layout: VolumeDataLayout) -> Self {
        let now = Utc::now();
        Self {
            version: VdsVersion::default(),
            layout,
            compression: CompressionMethod::Zstd,
            compression_tolerance: 0.0,
            value_range: ValueRange::new(0.0, 0.0),
            created_at: now,
            modified_at: now,
            custom_metadata: HashMap::new(),
            survey_metadata: None,
        }
    }

    /// Set compression method
    pub fn with_compression(mut self, method: CompressionMethod) -> Self {
        self.compression = method;
        self
    }

    /// Set compression tolerance
    pub fn with_compression_tolerance(mut self, tolerance: f32) -> Self {
        self.compression_tolerance = tolerance;
        self
    }

    /// Set value range
    pub fn with_value_range(mut self, range: ValueRange) -> Self {
        self.value_range = range;
        self
    }

    /// Add custom metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom_metadata.insert(key.into(), value.into());
    }

    /// Get custom metadata
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.custom_metadata.get(key).map(|s| s.as_str())
    }

    /// Set survey metadata
    pub fn with_survey_metadata(mut self, survey: SurveyMetadata) -> Self {
        self.survey_metadata = Some(survey);
        self
    }

    /// Update modification timestamp
    pub fn touch(&mut self) {
        self.modified_at = Utc::now();
    }
}

/// Survey/acquisition metadata for seismic data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyMetadata {
    /// Survey name
    pub survey_name: String,

    /// Survey type (e.g., "3D Seismic", "2D Seismic")
    pub survey_type: String,

    /// Acquisition date
    pub acquisition_date: Option<DateTime<Utc>>,

    /// Processing date
    pub processing_date: Option<DateTime<Utc>>,

    /// Company/contractor
    pub company: Option<String>,

    /// Geographic coordinate system
    pub coordinate_system: Option<String>,

    /// SEG-Y specific metadata
    pub segy_metadata: Option<SegyMetadata>,
}

/// SEG-Y specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegyMetadata {
    /// SEG-Y revision
    pub revision: u16,

    /// Text header
    pub text_header: Vec<String>,

    /// Binary header fields
    pub binary_header: HashMap<String, i32>,

    /// Trace header mappings
    pub trace_header_mappings: HashMap<String, String>,
}

impl SegyMetadata {
    pub fn new(revision: u16) -> Self {
        Self {
            revision,
            text_header: Vec::new(),
            binary_header: HashMap::new(),
            trace_header_mappings: HashMap::new(),
        }
    }
}

/// Brick metadata - stores information about individual bricks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrickMetadata {
    /// Brick index in the volume
    pub index: usize,

    /// Compressed size in bytes
    pub compressed_size: usize,

    /// Uncompressed size in bytes
    pub uncompressed_size: usize,

    /// Offset in the brick file (for packed formats)
    pub offset: Option<u64>,

    /// Checksum (CRC32 or similar)
    pub checksum: Option<u32>,

    /// Min/max values in this brick
    pub value_range: Option<ValueRange>,
}

impl BrickMetadata {
    pub fn new(index: usize, compressed_size: usize, uncompressed_size: usize) -> Self {
        Self {
            index,
            compressed_size,
            uncompressed_size,
            offset: None,
            checksum: None,
            value_range: None,
        }
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size == 0 {
            0.0
        } else {
            self.uncompressed_size as f64 / self.compressed_size as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AxisDescriptor, DataType};

    #[test]
    fn test_version_compatibility() {
        let v3_0 = VdsVersion::new(3, 0);
        let v3_1 = VdsVersion::new(3, 1);
        let v2_0 = VdsVersion::new(2, 0);

        assert!(v3_0.is_compatible(&v3_1));
        assert!(!v3_0.is_compatible(&v2_0));
    }

    #[test]
    fn test_metadata_creation() {
        let axes = vec![
            AxisDescriptor::new(1000, "Inline", "trace", 0.0, 999.0),
            AxisDescriptor::new(800, "Crossline", "trace", 0.0, 799.0),
            AxisDescriptor::new(500, "Depth", "ms", 0.0, 2000.0),
        ];

        let layout = VolumeDataLayout::new(3, DataType::F32, axes).unwrap();
        let mut metadata = VdsMetadata::new(layout)
            .with_compression(CompressionMethod::Zstd)
            .with_value_range(ValueRange::new(-1000.0, 1000.0));

        metadata.add_metadata("project", "North Sea Survey");
        assert_eq!(metadata.get_metadata("project"), Some("North Sea Survey"));
    }

    #[test]
    fn test_brick_metadata() {
        let brick = BrickMetadata::new(0, 10000, 100000);
        assert_eq!(brick.compression_ratio(), 10.0);
    }
}
