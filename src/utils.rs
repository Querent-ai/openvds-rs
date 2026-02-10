//! Utility functions

use crate::error::{Result, VdsError};
use std::mem;

/// Convert raw bytes to typed data
pub fn bytes_to_typed_data<T: Copy>(bytes: &[u8]) -> Result<Vec<T>> {
    if bytes.len() % mem::size_of::<T>() != 0 {
        return Err(VdsError::InvalidFormat(
            "Byte length not aligned with data type size".to_string(),
        ));
    }

    let count = bytes.len() / mem::size_of::<T>();
    let mut data = Vec::with_capacity(count);

    unsafe {
        let ptr = bytes.as_ptr() as *const T;
        for i in 0..count {
            data.push(*ptr.add(i));
        }
    }

    Ok(data)
}

/// Convert typed data to raw bytes
pub fn typed_data_to_bytes<T: Copy>(data: &[T]) -> Vec<u8> {
    let byte_len = std::mem::size_of_val(data);
    let mut bytes = Vec::with_capacity(byte_len);

    unsafe {
        let ptr = data.as_ptr() as *const u8;
        for i in 0..byte_len {
            bytes.push(*ptr.add(i));
        }
    }

    bytes
}

/// Calculate checksum (CRC32) for data
pub fn calculate_checksum(data: &[u8]) -> u32 {
    // Simple CRC32 implementation
    let mut crc = 0xFFFFFFFFu32;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

/// Verify checksum
pub fn verify_checksum(data: &[u8], expected: u32) -> bool {
    calculate_checksum(data) == expected
}

/// Format byte size in human-readable form
pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Parse brick path from index
pub fn brick_path(index: usize, lod_level: usize) -> String {
    format!("bricks/lod{}/{:08}.brick", lod_level, index)
}

/// Align value to power of 2
pub fn align_to_power_of_2(value: usize, alignment: usize) -> usize {
    debug_assert!(alignment.is_power_of_two());
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_conversion() {
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let bytes = typed_data_to_bytes(&data);
        assert_eq!(bytes.len(), data.len() * 4);

        let recovered: Vec<f32> = bytes_to_typed_data(&bytes).unwrap();
        assert_eq!(data, recovered);
    }

    #[test]
    fn test_checksum() {
        let data = b"Hello, world!";
        let checksum = calculate_checksum(data);
        assert!(verify_checksum(data, checksum));
        assert!(!verify_checksum(data, checksum + 1));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_brick_path() {
        assert_eq!(brick_path(0, 0), "bricks/lod0/00000000.brick");
        assert_eq!(brick_path(42, 2), "bricks/lod2/00000042.brick");
        assert_eq!(brick_path(1234567, 0), "bricks/lod0/01234567.brick");
    }

    #[test]
    fn test_align_to_power_of_2() {
        assert_eq!(align_to_power_of_2(0, 16), 0);
        assert_eq!(align_to_power_of_2(1, 16), 16);
        assert_eq!(align_to_power_of_2(15, 16), 16);
        assert_eq!(align_to_power_of_2(16, 16), 16);
        assert_eq!(align_to_power_of_2(17, 16), 32);
    }
}
