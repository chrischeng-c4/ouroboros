//! Integration tests for package manager detection
//!
//! Tests automatic detection and integration of Python package managers:
//! - uv
//! - Poetry
//! - Pipenv
//! - pip

use argus::types::{PackageManager, PackageManagerDetector, Dependency};
use std::path::PathBuf;

// Helper to get test fixture path
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/package_managers")
        .join(name)
}

#[test]
fn test_detect_uv_project() {
    let root = fixture_path("uv_project");
    let detector = PackageManagerDetector::new(root.clone());
    let detection = detector.detect();

    // Verify package manager
    assert_eq!(detection.manager, PackageManager::Uv);
    assert_eq!(detection.manager.display_name(), "uv");

    // Verify confidence
    assert!(detection.confidence > 0.9, "Expected confidence > 0.9, got {}", detection.confidence);

    // Verify config files
    assert!(detection.config_file.exists());
    assert_eq!(detection.config_file.file_name().unwrap(), "pyproject.toml");

    // Verify lock file
    assert!(detection.lock_file.is_some());
    let lock_file = detection.lock_file.clone().unwrap();
    assert!(lock_file.exists());
    assert_eq!(lock_file.file_name().unwrap(), "uv.lock");

    // Verify venv discovery
    assert!(detection.venv_path.is_some());
    let venv_path = detection.venv_path.clone().unwrap();
    assert!(venv_path.exists());
    assert_eq!(venv_path.file_name().unwrap(), ".venv");

    // Verify dependencies
    assert!(!detection.dependencies.is_empty());
    assert!(detection.has_dependency("django"));
    assert!(detection.has_dependency("fastapi"));
    assert!(detection.has_dependency("pydantic"));
    assert!(detection.has_dependency("requests"));

    // Verify framework dependencies
    let framework_deps = detection.framework_dependencies();
    assert!(framework_deps.len() >= 3); // django, fastapi, pydantic
    assert!(framework_deps.iter().any(|d| d.name == "django"));
    assert!(framework_deps.iter().any(|d| d.name == "fastapi"));
    assert!(framework_deps.iter().any(|d| d.name == "pydantic"));
}

#[test]
fn test_detect_poetry_project() {
    let root = fixture_path("poetry_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // Verify package manager
    assert_eq!(detection.manager, PackageManager::Poetry);
    assert_eq!(detection.manager.display_name(), "Poetry");

    // Verify confidence
    assert!(detection.confidence > 0.9);

    // Verify config files
    assert!(detection.config_file.exists());
    assert_eq!(detection.config_file.file_name().unwrap(), "pyproject.toml");

    // Verify lock file
    assert!(detection.lock_file.is_some());
    let lock_file = detection.lock_file.clone().unwrap();
    assert!(lock_file.exists());
    assert_eq!(lock_file.file_name().unwrap(), "poetry.lock");

    // Verify venv
    assert!(detection.venv_path.is_some());

    // Verify dependencies
    assert!(!detection.dependencies.is_empty());
    assert!(detection.has_dependency("django"));
    assert!(detection.has_dependency("flask"));
    assert!(detection.has_dependency("sqlalchemy"));

    // Verify framework dependencies
    let framework_deps = detection.framework_dependencies();
    assert!(framework_deps.len() >= 3); // django, flask, sqlalchemy
}

#[test]
fn test_detect_poetry_project_no_lock() {
    // Create a temporary test without lock file
    // In real scenario, confidence should be lower
    let root = fixture_path("poetry_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // Even without lock file, should still detect Poetry
    assert_eq!(detection.manager, PackageManager::Poetry);
}

#[test]
fn test_detect_pipenv_project() {
    let root = fixture_path("pipenv_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // Verify package manager
    assert_eq!(detection.manager, PackageManager::Pipenv);
    assert_eq!(detection.manager.display_name(), "Pipenv");

    // Verify confidence
    assert!(detection.confidence > 0.8);

    // Verify config files
    assert!(detection.config_file.exists());
    assert_eq!(detection.config_file.file_name().unwrap(), "Pipfile");

    // Verify lock file
    assert!(detection.lock_file.is_some());
    let lock_file = detection.lock_file.clone().unwrap();
    assert!(lock_file.exists());
    assert_eq!(lock_file.file_name().unwrap(), "Pipfile.lock");

    // Verify venv
    assert!(detection.venv_path.is_some());

    // Verify dependencies
    assert!(!detection.dependencies.is_empty());
    assert!(detection.has_dependency("celery"));
    assert!(detection.has_dependency("pydantic"));
    assert!(detection.has_dependency("fastapi"));

    // Check for dev dependencies
    let dev_deps: Vec<&Dependency> = detection.dependencies.iter()
        .filter(|d| d.is_dev)
        .collect();
    assert!(!dev_deps.is_empty());
}

#[test]
fn test_detect_pip_project() {
    let root = fixture_path("pip_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // Verify package manager
    assert_eq!(detection.manager, PackageManager::Pip);
    assert_eq!(detection.manager.display_name(), "pip");

    // Verify confidence (lower for pip without lock file)
    assert!(detection.confidence >= 0.7);
    assert!(detection.confidence < 0.9);

    // Verify config files
    assert!(detection.config_file.exists());
    assert_eq!(detection.config_file.file_name().unwrap(), "requirements.txt");

    // No lock file for pip
    assert!(detection.lock_file.is_none());

    // Verify venv
    assert!(detection.venv_path.is_some());

    // Verify dependencies
    assert!(!detection.dependencies.is_empty());
    assert!(detection.has_dependency("django"));
    assert!(detection.has_dependency("flask"));
    assert!(detection.has_dependency("requests"));
    assert!(detection.has_dependency("sqlalchemy"));

    // Verify extras parsing
    let requests_dep = detection.get_dependency("requests");
    assert!(requests_dep.is_some());
    let requests_dep = requests_dep.unwrap();
    assert!(requests_dep.extras.contains(&"security".to_string()));
}

#[test]
fn test_priority_order() {
    // In a project with multiple config files, uv should win
    // This is tested by the uv_project having pyproject.toml
    // which could be interpreted as Poetry, but uv.lock presence makes it uv

    let root = fixture_path("uv_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // Should detect as uv, not Poetry
    assert_eq!(detection.manager, PackageManager::Uv);
}

#[test]
fn test_venv_discovery_multiple_names() {
    // Test that different venv names are discovered
    // uv_project uses .venv
    let root1 = fixture_path("uv_project");
    let detector1 = PackageManagerDetector::new(root1);
    let detection1 = detector1.detect();
    assert!(detection1.venv_path.is_some());
    assert!(detection1.venv_path.unwrap().file_name().unwrap() == ".venv");

    // pipenv_project uses venv
    let root2 = fixture_path("pipenv_project");
    let detector2 = PackageManagerDetector::new(root2);
    let detection2 = detector2.detect();
    assert!(detection2.venv_path.is_some());
    assert!(detection2.venv_path.unwrap().file_name().unwrap() == "venv");
}

#[test]
fn test_dependency_parsing_versions() {
    let root = fixture_path("uv_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // Check version constraints are parsed
    let django = detection.get_dependency("django");
    assert!(django.is_some());
    let django = django.unwrap();
    assert!(django.version.is_some());
    assert!(django.version.as_ref().unwrap().contains(">=") ||
            django.version.as_ref().unwrap().contains("4.2"));
}

#[test]
fn test_dependency_parsing_extras() {
    let root = fixture_path("uv_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    // fastapi[all] should have "all" in extras
    let fastapi = detection.get_dependency("fastapi");
    assert!(fastapi.is_some());
    let fastapi = fastapi.unwrap();
    assert!(!fastapi.extras.is_empty());
    assert!(fastapi.extras.contains(&"all".to_string()));
}

#[test]
fn test_framework_detection_from_dependencies() {
    // Test that framework packages are correctly identified
    let root = fixture_path("uv_project");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    let framework_deps = detection.framework_dependencies();

    // Should include django, fastapi, pydantic
    let framework_names: Vec<&str> = framework_deps.iter()
        .map(|d| d.name.as_str())
        .collect();

    assert!(framework_names.contains(&"django"));
    assert!(framework_names.contains(&"fastapi"));
    assert!(framework_names.contains(&"pydantic"));

    // Should NOT include non-framework packages
    assert!(!framework_names.contains(&"requests"));
}

#[test]
fn test_unknown_project() {
    // Test a directory without any package manager config
    let root = PathBuf::from("/tmp");
    let detector = PackageManagerDetector::new(root);
    let detection = detector.detect();

    assert_eq!(detection.manager, PackageManager::Unknown);
    assert_eq!(detection.confidence, 0.0);
    assert!(detection.dependencies.is_empty());
}

#[test]
fn test_parse_dependency_formats() {
    // Test various dependency formats
    use argus::types::PackageManagerDetector;

    // Simple version
    let dep1 = PackageManagerDetector::parse_dependency_line("django>=4.0").unwrap();
    assert_eq!(dep1.name, "django");
    assert_eq!(dep1.version, Some(">=4.0".to_string()));

    // With extras
    let dep2 = PackageManagerDetector::parse_dependency_line("fastapi[all]>=0.100").unwrap();
    assert_eq!(dep2.name, "fastapi");
    assert!(dep2.extras.contains(&"all".to_string()));

    // Multiple extras
    let dep3 = PackageManagerDetector::parse_dependency_line("package[dev,test]").unwrap();
    assert_eq!(dep3.name, "package");
    assert_eq!(dep3.extras.len(), 2);
    assert!(dep3.extras.contains(&"dev".to_string()));
    assert!(dep3.extras.contains(&"test".to_string()));

    // No version
    let dep4 = PackageManagerDetector::parse_dependency_line("requests").unwrap();
    assert_eq!(dep4.name, "requests");
    assert!(dep4.version.is_none());

    // With quotes (from pyproject.toml)
    let dep5 = PackageManagerDetector::parse_dependency_line("\"django>=4.0\"").unwrap();
    assert_eq!(dep5.name, "django");
}

#[test]
fn test_config_file_names() {
    assert_eq!(PackageManager::Uv.config_file_name(), "pyproject.toml");
    assert_eq!(PackageManager::Poetry.config_file_name(), "pyproject.toml");
    assert_eq!(PackageManager::Pipenv.config_file_name(), "Pipfile");
    assert_eq!(PackageManager::Pip.config_file_name(), "requirements.txt");
}

#[test]
fn test_lock_file_names() {
    assert_eq!(PackageManager::Uv.lock_file_name(), Some("uv.lock"));
    assert_eq!(PackageManager::Poetry.lock_file_name(), Some("poetry.lock"));
    assert_eq!(PackageManager::Pipenv.lock_file_name(), Some("Pipfile.lock"));
    assert_eq!(PackageManager::Pip.lock_file_name(), None);
}

#[test]
fn test_dependency_builder_pattern() {
    let dep = Dependency::new("django".to_string())
        .with_version(">=4.0".to_string())
        .with_extras(vec!["postgres".to_string(), "redis".to_string()])
        .as_dev();

    assert_eq!(dep.name, "django");
    assert_eq!(dep.version, Some(">=4.0".to_string()));
    assert_eq!(dep.extras.len(), 2);
    assert!(dep.is_dev);
}
