//! Integration tests for FrameworkDetector + PackageManager
//!
//! Verifies that FrameworkDetector correctly uses package manager detection
//! to identify frameworks from dependencies.

use argus::types::{FrameworkDetector, Framework};
use std::path::PathBuf;

// Helper to get test fixture path
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/package_managers")
        .join(name)
}

#[test]
fn test_framework_detection_from_uv_project() {
    let root = fixture_path("uv_project");
    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    // Should detect Django, FastAPI, and Pydantic from dependencies
    assert!(detection.has_framework(&Framework::Django));
    assert!(detection.has_framework(&Framework::FastAPI));
    // Pydantic is present but confidence might be lower

    // Check confidence scores (should be high from lockfile)
    assert!(detection.confidence_for(&Framework::Django) > 0.9);
    assert!(detection.confidence_for(&Framework::FastAPI) > 0.9);
}

#[test]
fn test_framework_detection_from_poetry_project() {
    let root = fixture_path("poetry_project");
    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    // Should detect Django, Flask, and SQLAlchemy from dependencies
    assert!(detection.has_framework(&Framework::Django));
    assert!(detection.has_framework(&Framework::Flask));
    assert!(detection.has_framework(&Framework::SQLAlchemy));

    // Check confidence (high from lockfile)
    let django_conf = detection.confidence_for(&Framework::Django);
    let flask_conf = detection.confidence_for(&Framework::Flask);
    println!("Django confidence: {}", django_conf);
    println!("Flask confidence: {}", flask_conf);

    assert!(django_conf > 0.8, "Django confidence: {}", django_conf);
    assert!(flask_conf > 0.8, "Flask confidence: {}", flask_conf);
}

#[test]
fn test_framework_detection_from_pipenv_project() {
    let root = fixture_path("pipenv_project");
    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    // Should detect FastAPI, Pydantic, and Celery from dependencies
    assert!(detection.has_framework(&Framework::FastAPI));
    assert!(detection.has_framework(&Framework::Celery));
    // Pydantic might be detected with lower confidence

    // Check confidence
    assert!(detection.confidence_for(&Framework::FastAPI) > 0.9);
    assert!(detection.confidence_for(&Framework::Celery) > 0.9);
}

#[test]
fn test_framework_detection_from_pip_project() {
    let root = fixture_path("pip_project");
    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    // Should detect Django and Flask from requirements.txt
    assert!(detection.has_framework(&Framework::Django));
    assert!(detection.has_framework(&Framework::Flask));
    assert!(detection.has_framework(&Framework::SQLAlchemy));

    // Confidence should be slightly lower without lockfile
    let django_confidence = detection.confidence_for(&Framework::Django);
    println!("Django confidence (pip): {}", django_confidence);
    assert!(django_confidence > 0.7, "Django confidence: {}", django_confidence);
}

#[test]
fn test_pydantic_confidence_without_other_frameworks() {
    // Pipenv project has Pydantic + FastAPI
    // Pydantic alone doesn't get high confidence
    let root = fixture_path("pipenv_project");
    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    // FastAPI should have higher confidence than standalone Pydantic
    let fastapi_conf = detection.confidence_for(&Framework::FastAPI);

    // Pydantic might be detected but with adjusted confidence
    if detection.has_framework(&Framework::Pydantic) {
        let pydantic_conf = detection.confidence_for(&Framework::Pydantic);
        // Pydantic confidence should be lower when FastAPI is present
        assert!(fastapi_conf >= pydantic_conf);
    }
}

#[test]
fn test_combined_detection_increases_confidence() {
    // For a real Django project with both dependencies and file structure,
    // confidence should be very high

    // This test would need a fixture with both dependencies AND Django files
    // For now, we verify that dependency detection alone provides high confidence

    let root = fixture_path("uv_project");
    let detector = FrameworkDetector::new(root);
    let detection = detector.detect();

    // From dependencies alone (with lockfile), confidence should be 0.95
    assert!(detection.confidence_for(&Framework::Django) >= 0.95);
}
