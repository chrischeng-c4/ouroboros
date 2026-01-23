use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Lockfile structure (talos-lock.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(rename = "lockfileVersion")]
    pub lockfile_version: String,

    #[serde(default)]
    pub packages: HashMap<String, PackageEntry>,
}

/// Package entry in lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
}

/// Package resolution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub integrity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tarball: Option<String>,
}

impl Lockfile {
    /// Create a new empty lockfile
    pub fn new() -> Self {
        Self {
            lockfile_version: "1.0".to_string(),
            packages: HashMap::new(),
        }
    }

    /// Create lockfile from resolved dependencies
    pub fn from_resolved(resolved: &HashMap<String, String>) -> Self {
        let mut lockfile = Self::new();

        for (name, version) in resolved {
            let key = format!("/{}@{}", name, version);
            lockfile.packages.insert(
                key,
                PackageEntry {
                    version: version.clone(),
                    resolution: None,
                    dependencies: None,
                },
            );
        }

        lockfile
    }

    /// Read lockfile from path
    pub fn read(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lockfile: Lockfile = serde_yaml::from_str(&content)?;
        Ok(lockfile)
    }

    /// Write lockfile to path
    pub fn write(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_creation() {
        let lockfile = Lockfile::new();
        assert_eq!(lockfile.lockfile_version, "1.0");
        assert_eq!(lockfile.packages.len(), 0);
    }

    #[test]
    fn test_from_resolved() {
        let mut resolved = HashMap::new();
        resolved.insert("react".to_string(), "18.2.0".to_string());

        let lockfile = Lockfile::from_resolved(&resolved);
        assert_eq!(lockfile.packages.len(), 1);
        assert!(lockfile.packages.contains_key("/react@18.2.0"));
    }
}
