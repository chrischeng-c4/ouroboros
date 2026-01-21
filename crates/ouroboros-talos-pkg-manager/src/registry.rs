use anyhow::Result;
use serde::Deserialize;

/// NPM registry client
pub struct RegistryClient {
    client: reqwest::Client,
    registry_url: String,
}

/// Package metadata from registry
#[derive(Debug, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    #[serde(rename = "dist-tags")]
    pub dist_tags: std::collections::HashMap<String, String>,
    pub versions: std::collections::HashMap<String, VersionMetadata>,
}

/// Version metadata
#[derive(Debug, Deserialize)]
pub struct VersionMetadata {
    pub version: String,
    pub dist: DistInfo,
}

/// Distribution info
#[derive(Debug, Deserialize)]
pub struct DistInfo {
    pub tarball: String,
    pub shasum: String,
    pub integrity: Option<String>,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new(registry_url: &str) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            registry_url: registry_url.to_string(),
        })
    }

    /// Get package metadata
    pub async fn get_package_metadata(&self, name: &str) -> Result<PackageMetadata> {
        let url = format!("{}/{}", self.registry_url, name);
        tracing::debug!("Fetching metadata: {}", url);

        let response = self.client.get(&url).send().await?;
        let metadata: PackageMetadata = response.json().await?;

        Ok(metadata)
    }

    /// Get latest version of a package
    pub async fn get_latest_version(&self, name: &str) -> Result<String> {
        let metadata = self.get_package_metadata(name).await?;

        metadata
            .dist_tags
            .get("latest")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No latest version found for {}", name))
    }

    /// Download package tarball
    pub async fn download_package(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let metadata = self.get_package_metadata(name).await?;

        let version_meta = metadata
            .versions
            .get(version)
            .ok_or_else(|| anyhow::anyhow!("Version {} not found for {}", version, name))?;

        tracing::debug!("Downloading tarball: {}", version_meta.dist.tarball);

        let response = self.client.get(&version_meta.dist.tarball).send().await?;
        let bytes = response.bytes().await?;

        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_client_creation() {
        let client = RegistryClient::new("https://registry.npmjs.org").unwrap();
        assert_eq!(client.registry_url, "https://registry.npmjs.org");
    }
}
