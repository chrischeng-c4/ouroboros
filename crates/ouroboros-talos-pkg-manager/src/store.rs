use anyhow::Result;
use std::path::{Path, PathBuf};

/// Store manager for content-addressable package storage
pub struct StoreManager {
    store_path: PathBuf,
}

impl StoreManager {
    /// Create a new store manager
    pub fn new(store_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&store_path)?;
        Ok(Self { store_path })
    }

    /// Install package to store
    pub fn install_package(&self, name: &str, version: &str, tarball: &[u8]) -> Result<()> {
        tracing::debug!("Installing {}@{} to store", name, version);

        // Create package directory
        let package_dir = self.store_path.join(format!("{}@{}", name, version));
        std::fs::create_dir_all(&package_dir)?;

        // TODO: Extract tarball to package directory
        // For now, just write placeholder
        let placeholder = package_dir.join(".talos-placeholder");
        std::fs::write(placeholder, tarball)?;

        Ok(())
    }

    /// Link package from store to node_modules
    pub fn link_package(&self, name: &str, node_modules: &Path) -> Result<()> {
        tracing::debug!("Linking {} to node_modules", name);

        std::fs::create_dir_all(node_modules)?;

        // TODO: Create hard links from store to node_modules
        // For now, just create placeholder directory
        let package_dir = node_modules.join(name);
        std::fs::create_dir_all(&package_dir)?;

        let placeholder = package_dir.join("index.js");
        std::fs::write(placeholder, format!("// Placeholder for {}\n", name))?;

        Ok(())
    }

    /// Get package path in store
    pub fn get_package_path(&self, name: &str, version: &str) -> PathBuf {
        self.store_path.join(format!("{}@{}", name, version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_creation() {
        let dir = tempdir().unwrap();
        let store = StoreManager::new(dir.path().to_path_buf()).unwrap();
        assert!(store.store_path.exists());
    }
}
