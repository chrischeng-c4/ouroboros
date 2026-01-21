use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub mod package;

/// Module resolver implementing Node.js resolution algorithm
pub struct ModuleResolver {
    options: ResolveOptions,
}

/// Module resolution options
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Base directories to search for modules
    pub base_dirs: Vec<PathBuf>,

    /// Extensions to try when resolving
    pub extensions: Vec<String>,

    /// Whether to resolve index files
    pub resolve_index: bool,

    /// Alias mappings (e.g., "@" -> "src")
    pub alias: Vec<(String, PathBuf)>,

    /// External modules that should not be bundled
    pub externals: HashSet<String>,
}

/// Resolved module information
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// Full path to the module
    pub path: PathBuf,

    /// Module type
    pub kind: ResolveKind,

    /// Whether this is an external module
    pub is_external: bool,
}

/// Module resolution kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveKind {
    /// Relative import (./foo, ../bar)
    Relative,

    /// Absolute import (/foo/bar)
    Absolute,

    /// Package import (react, lodash)
    Package,

    /// Alias import (@/components)
    Alias,
}

/// Parse package specifier into package name and subpath
///
/// Examples:
/// - "react" -> ("react", None)
/// - "react/jsx-runtime" -> ("react", Some("./jsx-runtime"))
/// - "@babel/core" -> ("@babel/core", None)
/// - "@babel/core/lib" -> ("@babel/core", Some("./lib"))
fn parse_package_specifier(specifier: &str) -> (String, Option<String>) {
    // Handle scoped packages (@org/package)
    if specifier.starts_with('@') {
        // Find second slash
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();
        match parts.len() {
            2 => {
                // @org/package
                (specifier.to_string(), None)
            }
            3 => {
                // @org/package/subpath
                let package_name = format!("{}/{}", parts[0], parts[1]);
                let subpath = format!("./{}", parts[2]);
                (package_name, Some(subpath))
            }
            _ => (specifier.to_string(), None),
        }
    } else {
        // Regular package
        match specifier.split_once('/') {
            Some((pkg, rest)) => {
                (pkg.to_string(), Some(format!("./{}", rest)))
            }
            None => (specifier.to_string(), None),
        }
    }
}

impl ModuleResolver {
    /// Create a new module resolver
    pub fn new(options: ResolveOptions) -> Result<Self> {
        Ok(Self { options })
    }

    /// Resolve a module specifier
    pub fn resolve(&self, specifier: &str, from: &Path) -> Result<ResolvedModule> {
        tracing::debug!("Resolving '{}' from {:?}", specifier, from);

        // Check if external
        if self.is_external(specifier) {
            return Ok(ResolvedModule {
                path: PathBuf::from(specifier),
                kind: ResolveKind::Package,
                is_external: true,
            });
        }

        // Determine resolution kind
        let kind = self.detect_kind(specifier);

        // Resolve based on kind
        let path = match kind {
            ResolveKind::Relative => self.resolve_relative(specifier, from)?,
            ResolveKind::Absolute => self.resolve_absolute(specifier)?,
            ResolveKind::Package => self.resolve_package(specifier, from)?,
            ResolveKind::Alias => self.resolve_alias(specifier, from)?,
        };

        Ok(ResolvedModule {
            path,
            kind,
            is_external: false,
        })
    }

    /// Detect the kind of module specifier
    fn detect_kind(&self, specifier: &str) -> ResolveKind {
        if specifier.starts_with("./") || specifier.starts_with("../") {
            ResolveKind::Relative
        } else if specifier.starts_with('/') {
            ResolveKind::Absolute
        } else if self.is_alias(specifier) {
            ResolveKind::Alias
        } else {
            ResolveKind::Package
        }
    }

    /// Check if specifier matches an alias
    fn is_alias(&self, specifier: &str) -> bool {
        self.options
            .alias
            .iter()
            .any(|(prefix, _)| specifier.starts_with(prefix))
    }

    /// Check if module is external
    fn is_external(&self, specifier: &str) -> bool {
        self.options.externals.contains(specifier)
            || self.options.externals.iter().any(|ext| {
                specifier.starts_with(&format!("{}/", ext))
            })
    }

    /// Resolve relative import
    fn resolve_relative(&self, specifier: &str, from: &Path) -> Result<PathBuf> {
        let base_dir = from.parent().unwrap_or(Path::new("."));
        let candidate = base_dir.join(specifier);
        self.try_extensions(&candidate)
    }

    /// Resolve absolute import
    fn resolve_absolute(&self, specifier: &str) -> Result<PathBuf> {
        let candidate = PathBuf::from(specifier);
        self.try_extensions(&candidate)
    }

    /// Resolve package import from node_modules
    fn resolve_package(&self, specifier: &str, from: &Path) -> Result<PathBuf> {
        // Parse package name and subpath
        // Examples:
        // - "react" -> ("react", None)
        // - "react/jsx-runtime" -> ("react", Some("./jsx-runtime"))
        // - "@babel/core" -> ("@babel/core", None)
        // - "@babel/core/lib" -> ("@babel/core", Some("./lib"))

        let (package_name, subpath) = parse_package_specifier(specifier);

        let mut current = from.parent();

        while let Some(dir) = current {
            let node_modules = dir.join("node_modules");
            if node_modules.exists() {
                let package_dir = node_modules.join(&package_name);
                if package_dir.exists() {
                    // Try to resolve package with subpath
                    if let Ok(resolved) = self.resolve_package_dir(&package_dir, subpath.as_deref()) {
                        return Ok(resolved);
                    }
                }
            }
            current = dir.parent();
        }

        anyhow::bail!("Cannot resolve package: {}", specifier)
    }

    /// Resolve from package directory
    fn resolve_package_dir(&self, package_dir: &Path, subpath: Option<&str>) -> Result<PathBuf> {
        let package_json = package_dir.join("package.json");

        // Try modern "exports" field first
        if package_json.exists() {
            if let Ok(Some(export_path)) = package::resolve_exports(&package_json, subpath) {
                // exports paths are relative to package directory
                let resolved_path = package_dir.join(export_path.trim_start_matches('.').trim_start_matches('/'));
                if let Ok(resolved) = self.try_extensions(&resolved_path) {
                    return Ok(resolved);
                }
                // If exports specifies exact file, try it directly
                if resolved_path.exists() {
                    return Ok(resolved_path);
                }
            }
        }

        // If subpath is specified but exports didn't match, try direct resolution
        if let Some(sub) = subpath {
            let subpath_resolved = package_dir.join(sub.trim_start_matches('.').trim_start_matches('/'));
            if let Ok(resolved) = self.try_extensions(&subpath_resolved) {
                return Ok(resolved);
            }
        }

        // Fall back to legacy "main" field (only for root import)
        if subpath.is_none() || subpath == Some(".") {
            if package_json.exists() {
                if let Ok(main) = package::get_package_main(&package_json) {
                    let main_path = package_dir.join(main);
                    if let Ok(resolved) = self.try_extensions(&main_path) {
                        return Ok(resolved);
                    }
                }
            }

            // Try index file
            if self.options.resolve_index {
                let index = package_dir.join("index");
                if let Ok(resolved) = self.try_extensions(&index) {
                    return Ok(resolved);
                }
            }
        }

        anyhow::bail!("Cannot resolve package directory: {:?} with subpath: {:?}", package_dir, subpath)
    }

    /// Resolve alias import
    fn resolve_alias(&self, specifier: &str, _from: &Path) -> Result<PathBuf> {
        for (prefix, target) in &self.options.alias {
            if specifier.starts_with(prefix) {
                let rest = &specifier[prefix.len()..];
                let candidate = target.join(rest.trim_start_matches('/'));
                return self.try_extensions(&candidate);
            }
        }

        anyhow::bail!("No matching alias for: {}", specifier)
    }

    /// Try different file extensions
    fn try_extensions(&self, base: &Path) -> Result<PathBuf> {
        // Try exact path first
        if base.exists() && base.is_file() {
            return Ok(base.to_path_buf());
        }

        // Try with extensions
        for ext in &self.options.extensions {
            let with_ext = base.with_extension(ext.trim_start_matches('.'));
            if with_ext.exists() && with_ext.is_file() {
                return Ok(with_ext);
            }
        }

        // Try as directory with index
        if base.is_dir() && self.options.resolve_index {
            for ext in &self.options.extensions {
                let index = base.join(format!("index.{}", ext.trim_start_matches('.')));
                if index.exists() && index.is_file() {
                    return Ok(index);
                }
            }
        }

        anyhow::bail!("Cannot resolve: {:?}", base)
    }
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            base_dirs: vec![PathBuf::from(".")],
            extensions: vec![
                "js".to_string(),
                "jsx".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "json".to_string(),
            ],
            resolve_index: true,
            alias: Vec::new(),
            externals: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_kind() {
        let resolver = ModuleResolver::new(ResolveOptions::default()).unwrap();

        assert_eq!(resolver.detect_kind("./foo"), ResolveKind::Relative);
        assert_eq!(resolver.detect_kind("../bar"), ResolveKind::Relative);
        assert_eq!(resolver.detect_kind("/abs/path"), ResolveKind::Absolute);
        assert_eq!(resolver.detect_kind("react"), ResolveKind::Package);
    }

    #[test]
    fn test_is_external() {
        let mut options = ResolveOptions::default();
        options.externals.insert("react".to_string());
        options.externals.insert("react-dom".to_string());

        let resolver = ModuleResolver::new(options).unwrap();

        assert!(resolver.is_external("react"));
        assert!(resolver.is_external("react-dom"));
        assert!(resolver.is_external("react-dom/client"));
        assert!(!resolver.is_external("./foo"));
    }

    #[test]
    fn test_parse_package_specifier() {
        // Regular package
        assert_eq!(
            parse_package_specifier("react"),
            ("react".to_string(), None)
        );

        // Regular package with subpath
        assert_eq!(
            parse_package_specifier("react/jsx-runtime"),
            ("react".to_string(), Some("./jsx-runtime".to_string()))
        );

        // Scoped package
        assert_eq!(
            parse_package_specifier("@babel/core"),
            ("@babel/core".to_string(), None)
        );

        // Scoped package with subpath
        assert_eq!(
            parse_package_specifier("@babel/core/lib/config"),
            ("@babel/core".to_string(), Some("./lib/config".to_string()))
        );
    }

    #[test]
    fn test_parse_package_specifier_edge_cases() {
        // Deep subpath
        assert_eq!(
            parse_package_specifier("lodash/fp/map"),
            ("lodash".to_string(), Some("./fp/map".to_string()))
        );

        // Scoped package with deep subpath
        assert_eq!(
            parse_package_specifier("@org/pkg/a/b/c"),
            ("@org/pkg".to_string(), Some("./a/b/c".to_string()))
        );
    }
}
