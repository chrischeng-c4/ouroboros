use anyhow::Result;
use std::path::Path;

pub mod image_processor;
pub mod types;

pub use types::{AssetOptions, AssetType, ProcessedAsset};

/// Asset processor for handling images, fonts, etc.
pub struct AssetProcessor {
    options: AssetOptions,
}

impl AssetProcessor {
    /// Create a new asset processor
    pub fn new(options: AssetOptions) -> Self {
        Self { options }
    }

    /// Process an asset file
    pub fn process(&self, path: &Path) -> Result<ProcessedAsset> {
        let asset_type = self.detect_type(path)?;

        match asset_type {
            AssetType::Image => self.process_image(path),
            AssetType::Font => self.process_font(path),
            AssetType::Other => self.process_generic(path),
        }
    }

    /// Detect asset type from file extension
    fn detect_type(&self, path: &Path) -> Result<AssetType> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => Ok(AssetType::Image),
            "woff" | "woff2" | "ttf" | "otf" | "eot" => Ok(AssetType::Font),
            _ => Ok(AssetType::Other),
        }
    }

    /// Process image asset
    fn process_image(&self, path: &Path) -> Result<ProcessedAsset> {
        tracing::debug!("Processing image: {:?}", path);

        if self.options.optimize_images {
            Ok(image_processor::optimize_image(path, &self.options)?)
        } else {
            self.process_generic(path)
        }
    }

    /// Process font asset
    fn process_font(&self, path: &Path) -> Result<ProcessedAsset> {
        tracing::debug!("Processing font: {:?}", path);
        self.process_generic(path)
    }

    /// Process generic asset (just copy)
    fn process_generic(&self, path: &Path) -> Result<ProcessedAsset> {
        let content = std::fs::read(path)?;
        let hash = self.compute_hash(&content);

        let filename = if self.options.hash_filenames {
            self.create_hashed_filename(path, &hash)
        } else {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        };

        Ok(ProcessedAsset {
            original_path: path.to_path_buf(),
            content,
            filename,
            hash,
            asset_type: self.detect_type(path)?,
        })
    }

    /// Compute content hash
    fn compute_hash(&self, content: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())[..8].to_string()
    }

    /// Create hashed filename
    fn create_hashed_filename(&self, path: &Path, hash: &str) -> String {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();

        format!("{}.{}.{}", stem, hash, ext)
    }
}

impl Default for AssetOptions {
    fn default() -> Self {
        Self {
            optimize_images: true,
            hash_filenames: true,
            max_image_size: 1024 * 1024, // 1MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_type() {
        let processor = AssetProcessor::new(AssetOptions::default());

        assert_eq!(
            processor.detect_type(Path::new("test.png")).unwrap(),
            AssetType::Image
        );
        assert_eq!(
            processor.detect_type(Path::new("test.woff")).unwrap(),
            AssetType::Font
        );
    }

    #[test]
    fn test_compute_hash() {
        let processor = AssetProcessor::new(AssetOptions::default());
        let hash = processor.compute_hash(b"test content");
        assert_eq!(hash.len(), 8);
    }
}
