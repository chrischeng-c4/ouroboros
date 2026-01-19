//! Response compression support
//!
//! Provides GZip and Deflate compression for HTTP responses.
//! Supports compression level configuration, minimum size thresholds,
//! and content-type filtering.

use std::collections::HashSet;
use std::io::{self, Write};
use flate2::write::{GzEncoder, DeflateEncoder};
use flate2::Compression as GzCompression;

// ============================================================================
// Compression Configuration
// ============================================================================

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// GZip compression (most compatible)
    #[default]
    Gzip,
    /// Deflate compression
    Deflate,
    /// Brotli compression (best ratio, modern browsers)
    Brotli,
}

impl CompressionAlgorithm {
    /// Get the content-encoding header value
    pub fn as_encoding(&self) -> Option<&'static str> {
        match self {
            CompressionAlgorithm::None => None,
            CompressionAlgorithm::Gzip => Some("gzip"),
            CompressionAlgorithm::Deflate => Some("deflate"),
            CompressionAlgorithm::Brotli => Some("br"),
        }
    }

    /// Parse from Accept-Encoding header value
    pub fn from_accept_encoding(header: &str) -> Self {
        // Parse quality values and find best match
        let mut best = (CompressionAlgorithm::None, 0.0f32);

        for part in header.split(',') {
            let part = part.trim();
            let (encoding, quality) = if let Some((enc, q)) = part.split_once(";q=") {
                (enc.trim(), q.parse().unwrap_or(1.0))
            } else {
                (part, 1.0)
            };

            let algo = match encoding {
                "br" => CompressionAlgorithm::Brotli,
                "gzip" => CompressionAlgorithm::Gzip,
                "deflate" => CompressionAlgorithm::Deflate,
                "*" => CompressionAlgorithm::Gzip, // Default for wildcard
                _ => continue,
            };

            if quality > best.1 {
                best = (algo, quality);
            }
        }

        best.0
    }
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Fastest compression (level 1)
    Fast,
    /// Default compression (level 6)
    Default,
    /// Best compression (level 9)
    Best,
    /// Custom level (1-9)
    Custom(u32),
}

impl CompressionLevel {
    /// Convert to numeric level (1-9)
    pub fn level(&self) -> u32 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 6,
            CompressionLevel::Best => 9,
            CompressionLevel::Custom(l) => *l,
        }
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        CompressionLevel::Default
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Compression level
    pub level: CompressionLevel,
    /// Minimum size to compress (bytes)
    pub minimum_size: usize,
    /// Content types to compress
    pub compress_types: HashSet<String>,
    /// Content types to never compress
    pub exclude_types: HashSet<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        let mut compress_types = HashSet::new();
        compress_types.insert("text/html".to_string());
        compress_types.insert("text/plain".to_string());
        compress_types.insert("text/css".to_string());
        compress_types.insert("text/javascript".to_string());
        compress_types.insert("application/javascript".to_string());
        compress_types.insert("application/json".to_string());
        compress_types.insert("application/xml".to_string());
        compress_types.insert("image/svg+xml".to_string());

        let mut exclude_types = HashSet::new();
        exclude_types.insert("image/jpeg".to_string());
        exclude_types.insert("image/png".to_string());
        exclude_types.insert("image/gif".to_string());
        exclude_types.insert("image/webp".to_string());
        exclude_types.insert("video/mp4".to_string());
        exclude_types.insert("audio/mpeg".to_string());

        Self {
            algorithm: CompressionAlgorithm::Gzip,
            level: CompressionLevel::Default,
            minimum_size: 1024, // 1KB minimum
            compress_types,
            exclude_types,
        }
    }
}

impl CompressionConfig {
    /// Create a new compression config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set algorithm
    pub fn algorithm(mut self, algorithm: CompressionAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set compression level
    pub fn level(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }

    /// Set minimum size to compress
    pub fn minimum_size(mut self, size: usize) -> Self {
        self.minimum_size = size;
        self
    }

    /// Add content type to compress
    pub fn compress_type(mut self, content_type: impl Into<String>) -> Self {
        self.compress_types.insert(content_type.into());
        self
    }

    /// Add content type to exclude
    pub fn exclude_type(mut self, content_type: impl Into<String>) -> Self {
        self.exclude_types.insert(content_type.into());
        self
    }

    /// Check if a content type should be compressed
    pub fn should_compress(&self, content_type: &str, size: usize) -> bool {
        // Check size threshold
        if size < self.minimum_size {
            return false;
        }

        // Check excluded types
        let base_type = content_type.split(';').next().unwrap_or(content_type).trim();
        if self.exclude_types.contains(base_type) {
            return false;
        }

        // Check if type is compressible
        if self.compress_types.contains(base_type) {
            return true;
        }

        // Compress text/* and application/json by default
        base_type.starts_with("text/") || base_type == "application/json"
    }
}

// ============================================================================
// Compression Functions
// ============================================================================

/// Compress data using the specified algorithm
pub fn compress(data: &[u8], config: &CompressionConfig) -> io::Result<Vec<u8>> {
    match config.algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Gzip => compress_gzip(data, config.level.level()),
        CompressionAlgorithm::Deflate => compress_deflate(data, config.level.level()),
        CompressionAlgorithm::Brotli => {
            // Fall back to gzip if brotli not available
            compress_gzip(data, config.level.level())
        }
    }
}

/// Compress data using GZip
pub fn compress_gzip(data: &[u8], level: u32) -> io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), GzCompression::new(level));
    encoder.write_all(data)?;
    encoder.finish()
}

/// Compress data using Deflate
pub fn compress_deflate(data: &[u8], level: u32) -> io::Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), GzCompression::new(level));
    encoder.write_all(data)?;
    encoder.finish()
}

// ============================================================================
// Compression Result
// ============================================================================

/// Result of compression operation
#[derive(Debug)]
pub struct CompressionResult {
    /// Compressed data
    pub data: Vec<u8>,
    /// Original size
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Algorithm used
    pub algorithm: CompressionAlgorithm,
}

impl CompressionResult {
    /// Get compression ratio (0.0 - 1.0, lower is better)
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 {
            1.0
        } else {
            self.compressed_size as f64 / self.original_size as f64
        }
    }

    /// Get savings percentage (0-100, higher is better)
    pub fn savings_percent(&self) -> f64 {
        (1.0 - self.ratio()) * 100.0
    }

    /// Check if compression was effective
    pub fn is_effective(&self) -> bool {
        self.compressed_size < self.original_size
    }
}

/// Compress data and return result with metadata
pub fn compress_with_result(data: &[u8], config: &CompressionConfig) -> io::Result<CompressionResult> {
    let compressed = compress(data, config)?;

    Ok(CompressionResult {
        original_size: data.len(),
        compressed_size: compressed.len(),
        algorithm: config.algorithm,
        data: compressed,
    })
}

// ============================================================================
// Response Compression Helper
// ============================================================================

/// Helper to compress response bodies
pub struct ResponseCompressor {
    config: CompressionConfig,
}

impl ResponseCompressor {
    /// Create a new response compressor
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Create with default GZip config
    pub fn gzip() -> Self {
        Self::new(CompressionConfig::new().algorithm(CompressionAlgorithm::Gzip))
    }

    /// Create with default Deflate config
    pub fn deflate() -> Self {
        Self::new(CompressionConfig::new().algorithm(CompressionAlgorithm::Deflate))
    }

    /// Determine compression algorithm from Accept-Encoding header
    pub fn negotiate(&self, accept_encoding: Option<&str>) -> CompressionAlgorithm {
        match accept_encoding {
            Some(header) => CompressionAlgorithm::from_accept_encoding(header),
            None => CompressionAlgorithm::None,
        }
    }

    /// Compress response body if appropriate
    pub fn compress_response(
        &self,
        body: &[u8],
        content_type: &str,
        accept_encoding: Option<&str>,
    ) -> Option<CompressionResult> {
        // Check if should compress
        if !self.config.should_compress(content_type, body.len()) {
            return None;
        }

        // Negotiate algorithm
        let algorithm = self.negotiate(accept_encoding);
        if algorithm == CompressionAlgorithm::None {
            return None;
        }

        // Create config with negotiated algorithm
        let config = CompressionConfig {
            algorithm,
            ..self.config.clone()
        };

        // Compress
        match compress_with_result(body, &config) {
            Ok(result) if result.is_effective() => Some(result),
            _ => None,
        }
    }

    /// Get the config
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_algorithm_from_accept() {
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("gzip, deflate"),
            CompressionAlgorithm::Gzip
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("br, gzip"),
            CompressionAlgorithm::Brotli
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("gzip;q=0.5, br;q=1.0"),
            CompressionAlgorithm::Brotli
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("identity"),
            CompressionAlgorithm::None
        );
    }

    #[test]
    fn test_compression_level() {
        assert_eq!(CompressionLevel::Fast.level(), 1);
        assert_eq!(CompressionLevel::Default.level(), 6);
        assert_eq!(CompressionLevel::Best.level(), 9);
        assert_eq!(CompressionLevel::Custom(3).level(), 3);
    }

    #[test]
    fn test_should_compress() {
        let config = CompressionConfig::default();

        // Should compress
        assert!(config.should_compress("text/html", 2000));
        assert!(config.should_compress("application/json", 2000));
        assert!(config.should_compress("text/plain; charset=utf-8", 2000));

        // Should not compress (too small)
        assert!(!config.should_compress("text/html", 500));

        // Should not compress (excluded types)
        assert!(!config.should_compress("image/jpeg", 10000));
        assert!(!config.should_compress("image/png", 10000));
    }

    #[test]
    fn test_gzip_compression() {
        let data = b"Hello, World! ".repeat(100);
        let compressed = compress_gzip(&data, 6).unwrap();

        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_deflate_compression() {
        let data = b"Hello, World! ".repeat(100);
        let compressed = compress_deflate(&data, 6).unwrap();

        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_compression_result() {
        let config = CompressionConfig::new();
        let data = b"Hello, World! ".repeat(100);
        let result = compress_with_result(&data, &config).unwrap();

        assert!(result.is_effective());
        assert!(result.ratio() < 1.0);
        assert!(result.savings_percent() > 0.0);
    }

    #[test]
    fn test_response_compressor() {
        let compressor = ResponseCompressor::gzip();

        // Should compress JSON
        let body = b"{ \"data\": \"test\" }".repeat(100);
        let result = compressor.compress_response(
            &body,
            "application/json",
            Some("gzip, deflate"),
        );
        assert!(result.is_some());

        // Should not compress JPEG
        let result = compressor.compress_response(
            &body,
            "image/jpeg",
            Some("gzip, deflate"),
        );
        assert!(result.is_none());

        // Should not compress without Accept-Encoding
        let result = compressor.compress_response(
            &body,
            "application/json",
            None,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_compression_config_builder() {
        let config = CompressionConfig::new()
            .algorithm(CompressionAlgorithm::Gzip)
            .level(CompressionLevel::Best)
            .minimum_size(500)
            .compress_type("application/octet-stream");

        assert_eq!(config.algorithm, CompressionAlgorithm::Gzip);
        assert_eq!(config.level, CompressionLevel::Best);
        assert_eq!(config.minimum_size, 500);
        assert!(config.compress_types.contains("application/octet-stream"));
    }
}
