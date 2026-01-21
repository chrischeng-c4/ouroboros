use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Simplified package.json structure
#[derive(Debug, Deserialize)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub main: Option<String>,
    pub module: Option<String>,
    pub exports: Option<serde_json::Value>,
    pub dependencies: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Read package.json from path
pub fn read_package_json(path: &Path) -> Result<PackageJson> {
    let content = fs::read_to_string(path)?;
    let package: PackageJson = serde_json::from_str(&content)?;
    Ok(package)
}

/// Get the main entry point from package.json
pub fn get_package_main(path: &Path) -> Result<String> {
    let package = read_package_json(path)?;

    // Prefer "module" over "main" for ESM
    if let Some(module) = package.module {
        return Ok(module);
    }

    if let Some(main) = package.main {
        return Ok(main);
    }

    // Default to index.js
    Ok("index.js".to_string())
}

/// Resolve using package.json "exports" field (modern Node.js)
pub fn resolve_exports(
    package_json_path: &Path,
    subpath: Option<&str>,
) -> Result<Option<String>> {
    let package = read_package_json(package_json_path)?;

    let exports = match package.exports {
        Some(exports) => exports,
        None => return Ok(None), // No exports field
    };

    let subpath = subpath.unwrap_or(".");

    // Handle different exports formats
    match &exports {
        // String: "exports": "./index.js"
        serde_json::Value::String(path) if subpath == "." => {
            return Ok(Some(path.clone()));
        }

        // Object with conditional exports
        serde_json::Value::Object(map) => {
            // Check for direct subpath match
            if let Some(value) = map.get(subpath) {
                return resolve_export_value(value);
            }

            // Check for pattern matching (e.g., "./features/*")
            for (pattern, value) in map.iter() {
                if let Some(matched) = match_export_pattern(pattern, subpath) {
                    if let Some(resolved) = resolve_export_value(value)? {
                        // Replace * with matched part
                        let final_path = resolved.replace('*', &matched);
                        return Ok(Some(final_path));
                    }
                }
            }

            // Default export "."
            if subpath == "." && map.contains_key(".") {
                return resolve_export_value(&map["."]);
            }
        }

        _ => {}
    }

    Ok(None)
}

/// Resolve an export value (handles conditional exports)
fn resolve_export_value(value: &serde_json::Value) -> Result<Option<String>> {
    match value {
        // Simple string: "./dist/index.js"
        serde_json::Value::String(path) => Ok(Some(path.clone())),

        // Conditional exports: { "import": "./esm.js", "require": "./cjs.js" }
        serde_json::Value::Object(map) => {
            // Priority order: import > default > require
            for condition in &["import", "default", "require", "node", "browser"] {
                if let Some(v) = map.get(*condition) {
                    if let serde_json::Value::String(path) = v {
                        return Ok(Some(path.clone()));
                    }
                }
            }
            Ok(None)
        }

        _ => Ok(None),
    }
}

/// Match export pattern (e.g., "./features/*" matches "./features/foo")
fn match_export_pattern(pattern: &str, subpath: &str) -> Option<String> {
    if !pattern.contains('*') {
        return None;
    }

    let pattern_parts: Vec<&str> = pattern.split('*').collect();
    if pattern_parts.len() != 2 {
        return None; // Only support single wildcard
    }

    let (prefix, suffix) = (pattern_parts[0], pattern_parts[1]);

    if subpath.starts_with(prefix) && subpath.ends_with(suffix) {
        let start = prefix.len();
        let end = subpath.len() - suffix.len();
        if start <= end {
            return Some(subpath[start..end].to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_package_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"name": "test-package", "version": "1.0.0", "main": "dist/index.js"}}"#
        )
        .unwrap();

        let package = read_package_json(file.path()).unwrap();
        assert_eq!(package.name, Some("test-package".to_string()));
        assert_eq!(package.version, Some("1.0.0".to_string()));
        assert_eq!(package.main, Some("dist/index.js".to_string()));
    }

    #[test]
    fn test_get_package_main() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"name": "test", "main": "lib/index.js"}}"#
        )
        .unwrap();

        let main = get_package_main(file.path()).unwrap();
        assert_eq!(main, "lib/index.js");
    }

    #[test]
    fn test_resolve_exports_string() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"name": "test", "exports": "./dist/index.js"}}"#
        )
        .unwrap();

        let result = resolve_exports(file.path(), Some(".")).unwrap();
        assert_eq!(result, Some("./dist/index.js".to_string()));
    }

    #[test]
    fn test_resolve_exports_object() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
                "name": "test",
                "exports": {{
                    ".": "./dist/index.js",
                    "./package.json": "./package.json"
                }}
            }}"#
        )
        .unwrap();

        let result = resolve_exports(file.path(), Some(".")).unwrap();
        assert_eq!(result, Some("./dist/index.js".to_string()));

        let result2 = resolve_exports(file.path(), Some("./package.json")).unwrap();
        assert_eq!(result2, Some("./package.json".to_string()));
    }

    #[test]
    fn test_resolve_exports_conditional() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
                "name": "test",
                "exports": {{
                    ".": {{
                        "import": "./dist/esm/index.js",
                        "require": "./dist/cjs/index.js",
                        "default": "./dist/index.js"
                    }}
                }}
            }}"#
        )
        .unwrap();

        // Should prefer "import" over "require"
        let result = resolve_exports(file.path(), Some(".")).unwrap();
        assert_eq!(result, Some("./dist/esm/index.js".to_string()));
    }

    #[test]
    fn test_resolve_exports_pattern() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
                "name": "test",
                "exports": {{
                    "./features/*": "./dist/features/*.js"
                }}
            }}"#
        )
        .unwrap();

        let result = resolve_exports(file.path(), Some("./features/auth")).unwrap();
        assert_eq!(result, Some("./dist/features/auth.js".to_string()));
    }

    #[test]
    fn test_match_export_pattern() {
        assert_eq!(
            match_export_pattern("./features/*", "./features/auth"),
            Some("auth".to_string())
        );

        assert_eq!(
            match_export_pattern("./lib/*.js", "./lib/utils.js"),
            Some("utils".to_string())
        );

        assert_eq!(
            match_export_pattern("./foo/*", "./bar/baz"),
            None
        );
    }
}
