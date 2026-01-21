use anyhow::Result;
use std::path::Path;

use crate::{AssetOptions, ProcessedAsset, AssetType};

/// Optimize image file
pub fn optimize_image(path: &Path, options: &AssetOptions) -> Result<ProcessedAsset> {
    tracing::debug!("Optimizing image: {:?}", path);

    // Read image
    let img = image::open(path)?;

    // Check size
    let original_size = std::fs::metadata(path)?.len() as usize;
    if original_size > options.max_image_size {
        tracing::warn!(
            "Image exceeds max size: {} > {}",
            original_size,
            options.max_image_size
        );
    }

    // TODO: Implement actual optimization
    // 1. Resize if too large
    // 2. Convert format if needed
    // 3. Compress

    // For now, just read as-is
    let content = std::fs::read(path)?;

    // Compute hash
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = format!("{:x}", hasher.finalize())[..8].to_string();

    let filename = if options.hash_filenames {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        format!("{}.{}.{}", stem, hash, ext)
    } else {
        path.file_name().unwrap().to_string_lossy().to_string()
    };

    Ok(ProcessedAsset {
        original_path: path.to_path_buf(),
        content,
        filename,
        hash,
        asset_type: AssetType::Image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_placeholder() {
        // Placeholder test
        assert!(true);
    }
}
