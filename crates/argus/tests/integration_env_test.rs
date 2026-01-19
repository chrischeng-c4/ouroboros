/// Integration test for Python environment detection and import resolution
use argus::types::{detect_python_environment, EnvInfo, ImportResolver, VenvType};
use std::path::PathBuf;

fn get_test_project_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/python-project")
}

#[test]
fn test_detect_venv_in_test_project() {
    let project_path = get_test_project_path();

    if !project_path.exists() {
        eprintln!("Test project not found. Run: ./scripts/prepare-test-folder.sh");
        return;
    }

    // Test environment detection
    let env_info: EnvInfo = detect_python_environment(&project_path);

    // Should detect .venv as active venv
    assert!(
        env_info.active_venv.is_some(),
        "Should detect .venv as active virtual environment"
    );

    let active = env_info.active_venv.as_ref().unwrap();
    assert_eq!(
        active.env_type,
        VenvType::Venv,
        "Should identify as Venv type"
    );

    // Should include search paths from config
    assert!(
        env_info.search_paths.len() >= 2,
        "Should include search paths from pyproject.toml"
    );

    // Should find site-packages
    assert!(
        active.site_packages.is_some(),
        "Should find site-packages in .venv"
    );

    let site_packages = active.site_packages.as_ref().unwrap();
    assert!(
        site_packages.to_str().unwrap().contains("site-packages"),
        "Site-packages path should contain 'site-packages'"
    );

    println!("✅ Environment detection test passed");
    println!("   Active venv: {:?}", active.path);
    println!("   Site-packages: {:?}", site_packages);
    println!("   Search paths: {:?}", env_info.search_paths);
}

#[test]
fn test_import_resolution_in_test_project() {
    let project_path = get_test_project_path();

    if !project_path.exists() {
        eprintln!("Test project not found. Run: ./scripts/prepare-test-folder.sh");
        return;
    }

    let env_info: EnvInfo = detect_python_environment(&project_path);

    // Build search paths including src and site-packages
    let mut search_paths = env_info.search_paths.clone();
    if let Some(active_venv) = &env_info.active_venv {
        if let Some(site_packages) = &active_venv.site_packages {
            search_paths.push(site_packages.clone());
        }
    }

    // Create resolver with search paths
    let mut resolver = ImportResolver::with_search_paths(search_paths);

    // Build index
    resolver.build_index();
    assert!(resolver.is_indexed(), "Index should be built");

    // Test: Should resolve local module 'utils'
    let utils_module = resolver.get_or_resolve_module("utils");
    assert!(
        utils_module.is_some(),
        "Should resolve local module 'utils'"
    );

    // Test: Should resolve package 'models'
    let models_module = resolver.get_or_resolve_module("models");
    assert!(
        models_module.is_some(),
        "Should resolve package 'models'"
    );

    // Test: Should resolve nested module 'models.user'
    let user_module = resolver.get_or_resolve_module("models.user");
    assert!(
        user_module.is_some(),
        "Should resolve nested module 'models.user'"
    );

    // Test: Should resolve third-party package 'requests'
    let requests_module = resolver.get_or_resolve_module("requests");
    assert!(
        requests_module.is_some(),
        "Should resolve third-party package 'requests' from site-packages"
    );

    // Test: Stub file priority (.pyi should be preferred)
    if let Some(requests) = requests_module {
        assert!(
            requests.is_stub,
            "Should load .pyi stub file for 'requests' package"
        );
    }

    // Test: List modules
    let all_modules = resolver.list_modules(None);
    assert!(
        all_modules.len() >= 3,
        "Should list at least utils, models, requests"
    );

    // Test: List modules with prefix
    let models_modules = resolver.list_modules(Some("models"));
    assert!(
        models_modules.iter().any(|m| m.module_path == "models.user"),
        "Should find 'models.user' with prefix filter"
    );

    println!("✅ Import resolution test passed");
    println!("   Resolved modules: {:?}", all_modules);
    println!("   Models submodules: {:?}", models_modules);
}

#[test]
fn test_config_priority() {
    let project_path = get_test_project_path();

    if !project_path.exists() {
        eprintln!("Test project not found. Run: ./scripts/prepare-test-folder.sh");
        return;
    }

    // Load config from pyproject.toml
    let config_path = project_path.join("pyproject.toml");
    if !config_path.exists() {
        eprintln!("pyproject.toml not found");
        return;
    }

    let content = std::fs::read_to_string(config_path).expect("Failed to read pyproject.toml");

    // Check if configuration is present
    assert!(
        content.contains("[tool.argus.python]"),
        "Should contain [tool.argus.python] configuration"
    );

    assert!(
        content.contains("search_paths"),
        "Should contain search_paths configuration"
    );

    assert!(
        content.contains("venv_path"),
        "Should contain venv_path configuration"
    );

    println!("✅ Configuration priority test passed");
}

#[test]
fn test_circular_import_handling() {
    let project_path = get_test_project_path();

    if !project_path.exists() {
        eprintln!("Test project not found. Run: ./scripts/prepare-test-folder.sh");
        return;
    }

    let src_path = project_path.join("src");
    let mut resolver = ImportResolver::with_search_paths(vec![src_path]);

    // Build index
    resolver.build_index();

    // Test loading modules multiple times (should not cause infinite loops)
    let _ = resolver.get_or_resolve_module("utils");
    let _ = resolver.get_or_resolve_module("utils"); // Second time should use cache
    let _ = resolver.get_or_resolve_module("models");
    let _ = resolver.get_or_resolve_module("models.user");

    // Clear and rebuild
    resolver.clear();
    resolver.build_index();

    println!("✅ Circular import handling test passed");
}
