//! Integration tests for framework detection

use argus::types::{Framework, FrameworkDetector};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project directory
fn create_test_project() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[test]
fn test_django_detection_with_manage_py() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create Django project structure
    fs::write(root.join("manage.py"), "#!/usr/bin/env python\n# Django manage.py").unwrap();
    fs::create_dir(root.join("myproject")).unwrap();
    fs::write(
        root.join("myproject/settings.py"),
        "# Django settings\nINSTALLED_APPS = []"
    ).unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    let confidence = detection.confidence_for(&Framework::Django);
    println!("Django confidence: {}", confidence);
    assert!(detection.has_framework(&Framework::Django), "Should detect Django");
    assert!(confidence > 0.6, "Should have high confidence, got {}", confidence);
}

#[test]
fn test_django_detection_with_requirements() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create requirements.txt with Django
    fs::write(root.join("requirements.txt"), "django>=3.2\npsycopg2-binary\n").unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    assert!(detection.has_framework(&Framework::Django), "Should detect Django from requirements");
}

#[test]
fn test_flask_detection() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create Flask project structure
    fs::write(root.join("app.py"), "from flask import Flask\napp = Flask(__name__)").unwrap();
    fs::write(root.join("requirements.txt"), "flask>=2.0\nwerkzeug\n").unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    assert!(detection.has_framework(&Framework::Flask), "Should detect Flask");
    assert!(detection.confidence_for(&Framework::Flask) > 0.5);
}

#[test]
fn test_fastapi_detection() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create FastAPI project structure
    fs::write(root.join("main.py"), "from fastapi import FastAPI\napp = FastAPI()").unwrap();
    fs::write(root.join("requirements.txt"), "fastapi>=0.68\nuvicorn\n").unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    let confidence = detection.confidence_for(&Framework::FastAPI);
    println!("FastAPI confidence: {}", confidence);
    assert!(detection.has_framework(&Framework::FastAPI), "Should detect FastAPI");
    assert!(confidence > 0.5, "Expected confidence > 0.5, got {}", confidence);
}

#[test]
fn test_multiple_frameworks() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create a project with both Django and FastAPI (microservices)
    fs::write(root.join("manage.py"), "#!/usr/bin/env python").unwrap();
    fs::write(root.join("requirements.txt"), "django>=3.2\nfastapi>=0.68\n").unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    println!("Multiple frameworks - Django: {}, FastAPI: {}",
             detection.has_framework(&Framework::Django),
             detection.has_framework(&Framework::FastAPI));
    println!("Confidences - Django: {}, FastAPI: {}",
             detection.confidence_for(&Framework::Django),
             detection.confidence_for(&Framework::FastAPI));

    assert!(detection.has_framework(&Framework::Django), "Should detect Django");
    assert!(detection.has_framework(&Framework::FastAPI), "Should detect FastAPI");
}

#[test]
fn test_no_framework_detected() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Empty project
    fs::write(root.join("main.py"), "print('Hello')").unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    println!("Empty project detection - Django: {}, Flask: {}, FastAPI: {}",
             detection.has_framework(&Framework::Django),
             detection.has_framework(&Framework::Flask),
             detection.has_framework(&Framework::FastAPI));
    println!("Confidences - Django: {}, Flask: {}, FastAPI: {}",
             detection.confidence_for(&Framework::Django),
             detection.confidence_for(&Framework::Flask),
             detection.confidence_for(&Framework::FastAPI));

    assert!(!detection.has_framework(&Framework::Django));
    assert!(!detection.has_framework(&Framework::Flask));
    assert!(!detection.has_framework(&Framework::FastAPI), "FastAPI should not be detected in empty project");
}

#[test]
fn test_pyproject_toml_detection() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create pyproject.toml with Django
    let pyproject_content = r#"
[tool.poetry]
name = "myproject"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
django = "^4.0"
"#;
    fs::write(root.join("pyproject.toml"), pyproject_content).unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    assert!(detection.has_framework(&Framework::Django), "Should detect Django from pyproject.toml");
}

#[test]
fn test_confidence_scores() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create Django project with multiple indicators
    fs::write(root.join("manage.py"), "#!/usr/bin/env python").unwrap();
    fs::write(root.join("requirements.txt"), "django>=3.2").unwrap();
    fs::create_dir(root.join("app")).unwrap();
    fs::write(root.join("app/models.py"), "from django.db import models").unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    assert!(detection.has_framework(&Framework::Django));
    let confidence = detection.confidence_for(&Framework::Django);

    // Should have high confidence with multiple indicators
    assert!(confidence > 0.7, "Confidence should be > 0.7, got {}", confidence);
    assert!(confidence <= 1.0, "Confidence should not exceed 1.0");
}

#[test]
fn test_django_models_detection() {
    let temp_dir = create_test_project();
    let root = temp_dir.path().to_path_buf();

    // Create Django project with models
    fs::write(root.join("requirements.txt"), "django>=3.2").unwrap();
    fs::create_dir(root.join("myapp")).unwrap();
    fs::write(
        root.join("myapp/models.py"),
        "from django.db import models\n\nclass User(models.Model): pass"
    ).unwrap();

    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    assert!(detection.has_framework(&Framework::Django));
}
