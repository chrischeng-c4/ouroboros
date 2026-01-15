//! Morton encoding (Z-order curve) for 2D coordinate mapping
//!
//! Maps 2D coordinates (row, col) to 1D keys while preserving spatial locality.
//! This enables efficient range queries on the underlying KV store.

use serde::{Deserialize, Serialize};

/// Morton-encoded key for 2D coordinates
///
/// Uses Z-order curve to map (row, col) pairs to a single u64 key.
/// This encoding preserves spatial locality, making range queries efficient.
///
/// # Example
///
/// ```rust,ignore
/// let key = MortonKey::encode(10, 20);
/// let (row, col) = key.decode();
/// assert_eq!((row, col), (10, 20));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MortonKey(u64);

impl MortonKey {
    /// Encode 2D coordinates into a Morton key
    ///
    /// # Arguments
    ///
    /// * `row` - Row coordinate (0-2^32)
    /// * `col` - Column coordinate (0-2^32)
    ///
    /// # Returns
    ///
    /// Morton-encoded key that preserves spatial locality
    pub fn encode(row: u32, col: u32) -> Self {
        // TODO: Implement Morton encoding
        // - Interleave bits of row and col
        // - Use bit manipulation for efficiency
        // - Return MortonKey(encoded_value)
        todo!("Implement Morton encoding")
    }

    /// Decode Morton key back to 2D coordinates
    ///
    /// # Returns
    ///
    /// Tuple of (row, col)
    pub fn decode(&self) -> (u32, u32) {
        // TODO: Implement Morton decoding
        // - Deinterleave bits
        // - Extract row and col
        todo!("Implement Morton decoding")
    }

    /// Get the raw u64 value
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Create from raw u64 value
    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Calculate Morton key range for a rectangular region
    ///
    /// # Arguments
    ///
    /// * `start_row` - Starting row (inclusive)
    /// * `start_col` - Starting column (inclusive)
    /// * `end_row` - Ending row (inclusive)
    /// * `end_col` - Ending column (inclusive)
    ///
    /// # Returns
    ///
    /// Vector of (start_key, end_key) ranges that cover the rectangle
    pub fn range_for_rect(
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<(MortonKey, MortonKey)> {
        // TODO: Implement range calculation
        // - Calculate Morton keys for rectangle corners
        // - Break into multiple ranges if needed (Z-curve doesn't fully preserve rectangles)
        // - Return ranges for efficient KV store scanning
        todo!("Implement range_for_rect")
    }

    /// Check if a point is within a rectangular region
    pub fn is_in_rect(&self, start_row: u32, start_col: u32, end_row: u32, end_col: u32) -> bool {
        let (row, col) = self.decode();
        row >= start_row && row <= end_row && col >= start_col && col <= end_col
    }
}

impl std::fmt::Display for MortonKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (row, col) = self.decode();
        write!(f, "MortonKey({}, {}) = 0x{:016x}", row, col, self.0)
    }
}

/// Helper function to interleave bits of two u32 values
fn interleave_bits(x: u32, y: u32) -> u64 {
    // TODO: Implement bit interleaving
    // - Spread bits of x and y
    // - Combine into single u64
    todo!("Implement interleave_bits")
}

/// Helper function to deinterleave bits
fn deinterleave_bits(morton: u64) -> (u32, u32) {
    // TODO: Implement bit deinterleaving
    // - Extract even bits for x
    // - Extract odd bits for y
    todo!("Implement deinterleave_bits")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morton_encoding_placeholder() {
        // TODO: Add tests once implementation is complete
        // Test cases:
        // - encode/decode round-trip
        // - spatial locality preservation
        // - range calculation
    }
}
