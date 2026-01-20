/// Integration tests for bundler

use ouroboros_talos_bundler::{Bundler, BundleOptions};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_simple_bundle() {
    let temp_dir = TempDir::new().unwrap();
    let entry_path = temp_dir.path().join("entry.js");

    // Create a simple entry file
    fs::write(
        &entry_path,
        r#"
        const message = "Hello from Talos!";
        console.log(message);
        "#,
    )
    .unwrap();

    // Create bundler
    let options = BundleOptions::default();
    let bundler = Bundler::new(options).unwrap();

    // Bundle
    let result = bundler.bundle(entry_path).await;

    assert!(result.is_ok(), "Bundling should succeed");
    let output = result.unwrap();

    // Check output contains runtime
    assert!(output.code.contains("__talos__"));
    assert!(output.code.contains("define"));
    assert!(output.code.contains("require"));

    // Check output contains our code
    assert!(output.code.contains("Hello from Talos!"));
}

#[tokio::test]
async fn test_multi_module_bundle() {
    let temp_dir = TempDir::new().unwrap();

    // Create util.js
    let util_path = temp_dir.path().join("util.js");
    fs::write(
        &util_path,
        r#"
        export function greet(name) {
            return "Hello, " + name;
        }
        "#,
    )
    .unwrap();

    // Create entry.js that imports util.js
    let entry_path = temp_dir.path().join("entry.js");
    fs::write(
        &entry_path,
        r#"
        import { greet } from './util.js';
        const message = greet("World");
        console.log(message);
        "#,
    )
    .unwrap();

    // Create bundler
    let options = BundleOptions::default();
    let bundler = Bundler::new(options).unwrap();

    // Bundle
    let result = bundler.bundle(entry_path).await;

    if let Err(ref e) = result {
        eprintln!("Bundle error: {:?}", e);
    }

    assert!(result.is_ok(), "Multi-module bundling should succeed");
    let output = result.unwrap();

    // Check output contains both modules
    assert!(output.code.contains("greet"));
    assert!(output.code.contains("World"));
}

#[tokio::test]
async fn test_jsx_bundle() {
    let temp_dir = TempDir::new().unwrap();
    let entry_path = temp_dir.path().join("App.jsx");

    // Create JSX file
    fs::write(
        &entry_path,
        r#"
        const App = () => {
            return <div>Hello JSX!</div>;
        };
        "#,
    )
    .unwrap();

    // Create bundler
    let options = BundleOptions::default();
    let bundler = Bundler::new(options).unwrap();

    // Bundle
    let result = bundler.bundle(entry_path).await;

    assert!(result.is_ok(), "JSX bundling should succeed");
    let output = result.unwrap();

    // Check JSX was transformed (should contain jsx() or React.createElement)
    assert!(
        output.code.contains("jsx(") || output.code.contains("React.createElement"),
        "JSX should be transformed"
    );
}

#[tokio::test]
async fn test_circular_dependency_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a.js
    let a_path = temp_dir.path().join("a.js");
    fs::write(
        &a_path,
        r#"
        import { funcB } from './b.js';
        export function funcA() { return funcB(); }
        "#,
    )
    .unwrap();

    // Create b.js that imports a.js (creating a cycle)
    let b_path = temp_dir.path().join("b.js");
    fs::write(
        &b_path,
        r#"
        import { funcA } from './a.js';
        export function funcB() { return funcA(); }
        "#,
    )
    .unwrap();

    // Create bundler
    let options = BundleOptions::default();
    let bundler = Bundler::new(options).unwrap();

    // Bundle should fail with circular dependency error
    let result = bundler.bundle(a_path).await;

    assert!(result.is_err(), "Should detect circular dependency");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Circular") || error.to_string().contains("cycle"),
        "Error should mention circular dependency"
    );
}
