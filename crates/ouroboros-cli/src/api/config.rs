//! pyproject.toml configuration handling for ob api
//!
//! Manages reading and writing the [tool.ouroboros] section.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Database type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DbType {
    #[default]
    Pg,
    Mongo,
}

impl std::fmt::Display for DbType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbType::Pg => write!(f, "pg"),
            DbType::Mongo => write!(f, "mongo"),
        }
    }
}

impl std::str::FromStr for DbType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pg" | "postgres" | "postgresql" => Ok(DbType::Pg),
            "mongo" | "mongodb" => Ok(DbType::Mongo),
            _ => anyhow::bail!("Unknown database type: {}. Use 'pg' or 'mongo'.", s),
        }
    }
}

/// Database connection config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_connections: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            min_connections: Some(1),
            max_connections: Some(10),
        }
    }
}

/// App configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Feature/Core module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db: Option<DbType>,
}

/// Codegen configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodegenConfig {
    #[serde(default = "default_test_framework")]
    pub test_framework: String,
    #[serde(default = "default_schema_style")]
    pub schema_style: String,
}

fn default_test_framework() -> String {
    "pytest".to_string()
}

fn default_schema_style() -> String {
    "pydantic".to_string()
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self {
            test_framework: default_test_framework(),
            schema_style: default_schema_style(),
        }
    }
}

/// Main ouroboros configuration in pyproject.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OuroborosConfig {
    #[serde(default)]
    pub default_db: DbType,

    #[serde(default)]
    pub migrations_dir: Option<String>,

    #[serde(default)]
    pub auto_migrate: bool,

    #[serde(default)]
    pub db: HashMap<String, DbConfig>,

    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,

    #[serde(default)]
    pub core: HashMap<String, ModuleConfig>,

    #[serde(default)]
    pub features: HashMap<String, ModuleConfig>,

    #[serde(default)]
    pub codegen: CodegenConfig,
}

impl OuroborosConfig {
    /// Create a new config with defaults for the given db type
    pub fn new(db_type: DbType) -> Self {
        let mut db = HashMap::new();

        match db_type {
            DbType::Pg => {
                db.insert(
                    "pg".to_string(),
                    DbConfig {
                        url: "postgresql://localhost:5432/mydb".to_string(),
                        min_connections: Some(1),
                        max_connections: Some(10),
                    },
                );
            }
            DbType::Mongo => {
                db.insert(
                    "mongo".to_string(),
                    DbConfig {
                        url: "mongodb://localhost:27017/mydb".to_string(),
                        min_connections: None,
                        max_connections: None,
                    },
                );
            }
        }

        Self {
            default_db: db_type,
            migrations_dir: Some("migrations".to_string()),
            auto_migrate: false,
            db,
            apps: HashMap::new(),
            core: HashMap::new(),
            features: HashMap::new(),
            codegen: CodegenConfig::default(),
        }
    }

    /// Add an app configuration
    pub fn add_app(&mut self, name: &str, port: Option<u16>, description: Option<String>) {
        self.apps.insert(
            name.to_string(),
            AppConfig { port, description },
        );
    }

    /// Add a core module configuration
    pub fn add_core(&mut self, name: &str, db: Option<DbType>) {
        self.core.insert(name.to_string(), ModuleConfig { db });
    }

    /// Add a feature module configuration
    pub fn add_feature(&mut self, name: &str, db: Option<DbType>) {
        self.features.insert(name.to_string(), ModuleConfig { db });
    }

    /// Get effective db type for a module (with fallback to default)
    pub fn get_db_for_module(&self, module_name: &str, is_core: bool) -> DbType {
        let modules = if is_core { &self.core } else { &self.features };
        modules
            .get(module_name)
            .and_then(|m| m.db)
            .unwrap_or(self.default_db)
    }
}

/// Wrapper for the pyproject.toml structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyProject {
    #[serde(default)]
    pub project: Option<ProjectMeta>,

    #[serde(default)]
    pub tool: Option<ToolSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(rename = "requires-python", skip_serializing_if = "Option::is_none")]
    pub requires_python: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolSection {
    #[serde(default)]
    pub ouroboros: Option<OuroborosConfig>,
}

impl PyProject {
    /// Create a new pyproject.toml structure
    pub fn new(project_name: &str, db_type: DbType) -> Self {
        let db_dep = match db_type {
            DbType::Pg => "ouroboros[pg]>=0.1.0",
            DbType::Mongo => "ouroboros[mongo]>=0.1.0",
        };

        Self {
            project: Some(ProjectMeta {
                name: project_name.to_string(),
                version: "0.1.0".to_string(),
                requires_python: Some(">=3.11".to_string()),
                dependencies: vec![db_dep.to_string()],
            }),
            tool: Some(ToolSection {
                ouroboros: Some(OuroborosConfig::new(db_type)),
            }),
        }
    }

    /// Load from a pyproject.toml file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }

    /// Save to a pyproject.toml file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize pyproject.toml")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write {}", path.display()))
    }

    /// Get ouroboros config, creating default if not present
    pub fn ouroboros(&self) -> &OuroborosConfig {
        static DEFAULT: std::sync::OnceLock<OuroborosConfig> = std::sync::OnceLock::new();
        self.tool
            .as_ref()
            .and_then(|t| t.ouroboros.as_ref())
            .unwrap_or_else(|| DEFAULT.get_or_init(OuroborosConfig::default))
    }

    /// Get mutable ouroboros config
    pub fn ouroboros_mut(&mut self) -> &mut OuroborosConfig {
        self.tool
            .get_or_insert_with(ToolSection::default)
            .ouroboros
            .get_or_insert_with(OuroborosConfig::default)
    }
}

/// Find and load pyproject.toml from current or parent directories
pub fn find_pyproject(start_dir: &Path) -> Result<(std::path::PathBuf, PyProject)> {
    let mut current = start_dir.to_path_buf();

    loop {
        let pyproject_path = current.join("pyproject.toml");
        if pyproject_path.exists() {
            let pyproject = PyProject::load(&pyproject_path)?;
            return Ok((pyproject_path, pyproject));
        }

        if !current.pop() {
            anyhow::bail!(
                "pyproject.toml not found. Run 'ob api init' first or navigate to project directory."
            );
        }
    }
}

/// Load or create pyproject.toml
pub fn load_or_create_pyproject(dir: &Path, project_name: &str, db_type: DbType) -> Result<PyProject> {
    let pyproject_path = dir.join("pyproject.toml");

    if pyproject_path.exists() {
        PyProject::load(&pyproject_path)
    } else {
        Ok(PyProject::new(project_name, db_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_type_parse() {
        assert_eq!("pg".parse::<DbType>().unwrap(), DbType::Pg);
        assert_eq!("postgres".parse::<DbType>().unwrap(), DbType::Pg);
        assert_eq!("mongo".parse::<DbType>().unwrap(), DbType::Mongo);
        assert_eq!("mongodb".parse::<DbType>().unwrap(), DbType::Mongo);
    }

    #[test]
    fn test_ouroboros_config_new() {
        let config = OuroborosConfig::new(DbType::Pg);
        assert_eq!(config.default_db, DbType::Pg);
        assert!(config.db.contains_key("pg"));
    }

    #[test]
    fn test_pyproject_serialize() {
        let pyproject = PyProject::new("test-api", DbType::Pg);
        let toml_str = toml::to_string_pretty(&pyproject).unwrap();
        assert!(toml_str.contains("[project]"));
        assert!(toml_str.contains("[tool.ouroboros]"));
    }

    #[test]
    fn test_add_app() {
        let mut config = OuroborosConfig::new(DbType::Pg);
        config.add_app("admin", Some(8001), Some("Admin API".to_string()));
        assert!(config.apps.contains_key("admin"));
        assert_eq!(config.apps["admin"].port, Some(8001));
    }

    #[test]
    fn test_add_feature() {
        let mut config = OuroborosConfig::new(DbType::Pg);
        config.add_feature("orders", None);
        config.add_feature("logs", Some(DbType::Mongo));

        assert_eq!(config.get_db_for_module("orders", false), DbType::Pg);
        assert_eq!(config.get_db_for_module("logs", false), DbType::Mongo);
    }
}
