use ouroboros_talos_transform::{Transformer, TransformOptions};
use std::path::Path;

#[test]
fn debug_whitespace_preservation() {
    let source = r#"const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('Root element not found');
const root = document.getElementById('app');"#;

    let transformer = Transformer::new(TransformOptions::default());
    let result = transformer.transform_js(source, Path::new("test.tsx")).unwrap();

    println!("=== Original ===");
    println!("{}", source);
    println!("\n=== Transformed ===");
    println!("{}", result.code);

    // Check for newlines
    let original_newlines = source.chars().filter(|&c| c == '\n').count();
    let transformed_newlines = result.code.chars().filter(|&c| c == '\n').count();
    println!("Original newlines: {}", original_newlines);
    println!("Transformed newlines: {}", transformed_newlines);

    // Should preserve newlines
    assert_eq!(original_newlines, transformed_newlines, "Newlines were lost!");
}

#[test]
fn debug_whitespace_with_jsx() {
    let source = r#"const rootElement = document.getElementById('root');
const root = <div>Hello</div>;
console.log(root);"#;

    let transformer = Transformer::new(TransformOptions::default());
    let result = transformer.transform_js(source, Path::new("test.tsx")).unwrap();

    println!("\n=== With JSX - Original ===");
    println!("{}", source);
    println!("\n=== With JSX - Transformed ===");
    println!("{}", result.code);

    // Check for newlines
    let original_newlines = source.chars().filter(|&c| c == '\n').count();
    let transformed_newlines = result.code.chars().filter(|&c| c == '\n').count();
    println!("\nOriginal newlines: {}", original_newlines);
    println!("Transformed newlines: {}", transformed_newlines);

    // Should preserve newlines
    assert_eq!(original_newlines, transformed_newlines, "Newlines were lost in JSX transformation!");
}
