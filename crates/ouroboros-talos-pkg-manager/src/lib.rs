use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub mod lockfile;
pub mod registry;
pub mod resolver;
pub mod store;

use lockfile::Lockfile;
use registry::RegistryClient;
use resolver::DependencyResolver;
use store::StoreManager;

/// Package manager for installing and managing dependencies
pub struct PackageManager {
    /// Project root directory
    root_dir: PathBuf,

    /// Store manager for package storage
    store: StoreManager,

    /// Registry client
    registry: RegistryClient,

    /// Dependency resolver
    resolver: DependencyResolver,
}

/// package.json structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(rename = "devDependencies", default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

impl PackageManager {
    /// Create a new package manager
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let store_dir = root_dir.join("node_modules").join(".talos-store");
        let store = StoreManager::new(store_dir)?;

        let registry = RegistryClient::new("https://registry.npmjs.org")?;
        let resolver = DependencyResolver::new();

        Ok(Self {
            root_dir,
            store,
            registry,
            resolver,
        })
    }

    /// Install dependencies from package.json
    pub async fn install(&self) -> Result<()> {
        tracing::info!("Installing dependencies...");

        // Read package.json
        let package_json = self.read_package_json()?;

        // Combine dependencies
        let mut all_deps = package_json.dependencies.clone();
        all_deps.extend(package_json.dev_dependencies.clone());

        // Resolve dependency tree
        let resolved = self.resolver.resolve(&all_deps, &self.registry).await?;

        // Install packages
        for (name, version) in &resolved {
            tracing::info!("Installing {}@{}", name, version);
            self.install_package(name, version).await?;
        }

        // Generate lockfile
        self.write_lockfile(&resolved)?;

        tracing::info!("✓ Dependencies installed successfully");
        Ok(())
    }

    /// Add a new dependency
    pub async fn add(&self, package: &str, dev: bool) -> Result<()> {
        tracing::info!("Adding package: {}", package);

        // Fetch latest version from registry
        let version = self.registry.get_latest_version(package).await?;

        // Update package.json
        let mut package_json = self.read_package_json()?;
        if dev {
            package_json.dev_dependencies.insert(package.to_string(), format!("^{}", version));
        } else {
            package_json.dependencies.insert(package.to_string(), format!("^{}", version));
        }
        self.write_package_json(&package_json)?;

        // Install
        self.install().await?;

        Ok(())
    }

    /// Remove a dependency
    pub async fn remove(&self, package: &str) -> Result<()> {
        tracing::info!("Removing package: {}", package);

        // Update package.json
        let mut package_json = self.read_package_json()?;
        package_json.dependencies.remove(package);
        package_json.dev_dependencies.remove(package);
        self.write_package_json(&package_json)?;

        // TODO: Clean up unused packages
        // self.prune().await?;

        Ok(())
    }

    /// Install a specific package
    async fn install_package(&self, name: &str, version: &str) -> Result<()> {
        // Download tarball
        let tarball = self.registry.download_package(name, version).await?;

        // Install to store
        self.store.install_package(name, version, &tarball)?;

        // Create link in node_modules
        let node_modules = self.root_dir.join("node_modules");
        self.store.link_package(name, &node_modules)?;

        Ok(())
    }

    /// Read package.json
    fn read_package_json(&self) -> Result<PackageJson> {
        let path = self.root_dir.join("package.json");
        let content = std::fs::read_to_string(path)?;
        let package: PackageJson = serde_json::from_str(&content)?;
        Ok(package)
    }

    /// Write package.json
    fn write_package_json(&self, package: &PackageJson) -> Result<()> {
        let path = self.root_dir.join("package.json");
        let content = serde_json::to_string_pretty(package)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Write lockfile
    fn write_lockfile(&self, resolved: &HashMap<String, String>) -> Result<()> {
        let lockfile = Lockfile::from_resolved(resolved);
        let path = self.root_dir.join("talos-lock.yaml");
        lockfile.write(&path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_json_parse() {
        let json = r#"{
            "name": "test-app",
            "version": "1.0.0",
            "dependencies": {
                "react": "^18.0.0"
            }
        }"#;

        let package: PackageJson = serde_json::from_str(json).unwrap();
        assert_eq!(package.name, "test-app");
        assert_eq!(package.dependencies.get("react"), Some(&"^18.0.0".to_string()));
    }
}
