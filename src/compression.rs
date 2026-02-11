//! Compression and decompression for VDS data

use crate::error::{Result, VdsError};
use flate2::read::{DeflateDecoder, DeflateEncoder};
use flate2::Compression as FlateCompression;
use serde::{Deserialize, Serialize};
use std::io::Read;

/// Compression methods supported by VDS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CompressionMethod {
    /// No compression
    None = 0,
    /// Deflate/ZIP compression
    Deflate = 1,
    /// Run-length encoding
    RLE = 2,
    /// Zstandard compression
    Zstd = 3,
    /// Wavelet compression (Bluware proprietary - placeholder)
    Wavelet = 4,
}

impl CompressionMethod {
    /// Get the method from a byte value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CompressionMethod::None),
            1 => Some(CompressionMethod::Deflate),
            2 => Some(CompressionMethod::RLE),
            3 => Some(CompressionMethod::Zstd),
            4 => Some(CompressionMethod::Wavelet),
            _ => None,
        }
    }
}

/// Compression level (0-9, where 0 is no compression and 9 is maximum)
#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    pub fn new(level: u8) -> Self {
        Self(level.min(9))
    }

    pub fn none() -> Self {
        Self(0)
    }

    pub fn fast() -> Self {
        Self(1)
    }

    pub fn best() -> Self {
        Self(9)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self(6)
    }
}

/// Trait for compression/decompression operations
pub trait Compressor: Send + Sync {
    /// Compress data
    fn compress(&self, data: &[u8], level: CompressionLevel) -> Result<Vec<u8>>;

    /// Decompress data
    fn decompress(&self, data: &[u8], expected_size: Option<usize>) -> Result<Vec<u8>>;

    /// Get the compression method
    fn method(&self) -> CompressionMethod;
}

/// No compression
#[derive(Debug, Default)]
pub struct NoneCompressor;

impl Compressor for NoneCompressor {
    fn compress(&self, data: &[u8], _level: CompressionLevel) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8], _expected_size: Option<usize>) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn method(&self) -> CompressionMethod {
        CompressionMethod::None
    }
}

/// Deflate compression
#[derive(Debug, Default)]
pub struct DeflateCompressor;

impl Compressor for DeflateCompressor {
    fn compress(&self, data: &[u8], level: CompressionLevel) -> Result<Vec<u8>> {
        let mut encoder = DeflateEncoder::new(data, FlateCompression::new(level.value() as u32));
        let mut compressed = Vec::new();
        encoder
            .read_to_end(&mut compressed)
            .map_err(|e| VdsError::Compression(e.to_string()))?;
        Ok(compressed)
    }

    fn decompress(&self, data: &[u8], expected_size: Option<usize>) -> Result<Vec<u8>> {
        let mut decoder = DeflateDecoder::new(data);
        let mut decompressed = if let Some(size) = expected_size {
            Vec::with_capacity(size)
        } else {
            Vec::new()
        };
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| VdsError::Decompression(e.to_string()))?;
        Ok(decompressed)
    }

    fn method(&self) -> CompressionMethod {
        CompressionMethod::Deflate
    }
}

/// Zstandard compression
#[derive(Debug, Default)]
pub struct ZstdCompressor;

impl Compressor for ZstdCompressor {
    fn compress(&self, data: &[u8], level: CompressionLevel) -> Result<Vec<u8>> {
        zstd::encode_all(data, level.value() as i32)
            .map_err(|e| VdsError::Compression(e.to_string()))
    }

    fn decompress(&self, data: &[u8], _expected_size: Option<usize>) -> Result<Vec<u8>> {
        zstd::decode_all(data).map_err(|e| VdsError::Decompression(e.to_string()))
    }

    fn method(&self) -> CompressionMethod {
        CompressionMethod::Zstd
    }
}

/// Run-length encoding compressor
#[derive(Debug, Default)]
pub struct RLECompressor;

impl RLECompressor {
    fn compress_internal(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut compressed = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let byte = data[i];
            let mut count = 1usize;

            // Count consecutive identical bytes
            while i + count < data.len() && data[i + count] == byte && count < 255 {
                count += 1;
            }

            // Encode: count (1 byte) + value (1 byte)
            compressed.push(count as u8);
            compressed.push(byte);

            i += count;
        }

        compressed
    }

    fn decompress_internal(data: &[u8]) -> Result<Vec<u8>> {
        if data.len() % 2 != 0 {
            return Err(VdsError::Decompression(
                "RLE data must have even length".to_string(),
            ));
        }

        let mut decompressed = Vec::new();

        for chunk in data.chunks_exact(2) {
            let count = chunk[0] as usize;
            let value = chunk[1];
            decompressed.extend(std::iter::repeat(value).take(count));
        }

        Ok(decompressed)
    }
}

impl Compressor for RLECompressor {
    fn compress(&self, data: &[u8], _level: CompressionLevel) -> Result<Vec<u8>> {
        Ok(Self::compress_internal(data))
    }

    fn decompress(&self, data: &[u8], _expected_size: Option<usize>) -> Result<Vec<u8>> {
        Self::decompress_internal(data)
    }

    fn method(&self) -> CompressionMethod {
        CompressionMethod::RLE
    }
}

/// Get a compressor for a given method
pub fn get_compressor(method: CompressionMethod) -> Box<dyn Compressor> {
    match method {
        CompressionMethod::None => Box::new(NoneCompressor),
        CompressionMethod::Deflate => Box::new(DeflateCompressor),
        CompressionMethod::RLE => Box::new(RLECompressor),
        CompressionMethod::Zstd => Box::new(ZstdCompressor),
        CompressionMethod::Wavelet => {
            // Placeholder - would need to implement Bluware's wavelet algorithm
            Box::new(NoneCompressor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_compression() {
        let compressor = NoneCompressor;
        let data = b"Hello, world!";
        let compressed = compressor
            .compress(data, CompressionLevel::default())
            .unwrap();
        assert_eq!(compressed, data);
        let decompressed = compressor.decompress(&compressed, None).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_deflate() {
        let compressor = DeflateCompressor;
        let data = b"Hello, world! ".repeat(100);
        let compressed = compressor
            .compress(&data, CompressionLevel::default())
            .unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = compressor
            .decompress(&compressed, Some(data.len()))
            .unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd() {
        let compressor = ZstdCompressor;
        let data = b"Hello, world! ".repeat(100);
        let compressed = compressor
            .compress(&data, CompressionLevel::default())
            .unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = compressor.decompress(&compressed, None).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_rle() {
        let compressor = RLECompressor;
        let data = vec![1u8; 100];
        let compressed = compressor
            .compress(&data, CompressionLevel::default())
            .unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = compressor.decompress(&compressed, None).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_rle_mixed() {
        let compressor = RLECompressor;
        let mut data = vec![1u8; 50];
        data.extend(vec![2u8; 50]);
        let compressed = compressor
            .compress(&data, CompressionLevel::default())
            .unwrap();
        let decompressed = compressor.decompress(&compressed, None).unwrap();
        assert_eq!(decompressed, data);
    }
}
