use ouroboros_talos_transform::{Transformer, TransformOptions};
use std::path::Path;

#[test]
fn test_jsx_transform_basic() {
    let transformer = Transformer::new(TransformOptions::default());

    let jsx_code = r#"
        const App = () => <div>Hello World</div>;
    "#;

    let result = transformer
        .transform_js(jsx_code, Path::new("test.jsx"))
        .unwrap();

    // For now, just verify it doesn't panic
    // Once SWC is fully integrated, we can verify the actual transformation
    assert!(!result.code.is_empty());
}

#[test]
fn test_typescript_transform_basic() {
    let transformer = Transformer::new(TransformOptions::default());

    let ts_code = r#"
        const x: number = 42;
        const y: string = "hello";
    "#;

    let result = transformer
        .transform_js(ts_code, Path::new("test.ts"))
        .unwrap();

    assert!(!result.code.is_empty());
}

#[test]
fn test_tsx_transform_basic() {
    let transformer = Transformer::new(TransformOptions::default());

    let tsx_code = r#"
        interface Props {
            name: string;
        }

        const Component = ({ name }: Props) => <div>{name}</div>;
    "#;

    let result = transformer
        .transform_js(tsx_code, Path::new("test.tsx"))
        .unwrap();

    assert!(!result.code.is_empty());
}

#[test]
fn test_css_transform_basic() {
    let transformer = Transformer::new(TransformOptions::default());

    let css_code = r#"
        .container {
            display: flex;
            color: red;
        }
    "#;

    let result = transformer.transform_css(css_code).unwrap();

    assert!(!result.code.is_empty());
    // Should contain CSS injection code
    assert!(result.code.contains("createElement('style')"));
    assert!(result.code.contains("appendChild"));
    assert!(result.code.contains(".container"));
}

#[test]
fn test_plain_js() {
    let transformer = Transformer::new(TransformOptions::default());

    let js_code = r#"
        const x = 42;
        console.log(x);
    "#;

    let result = transformer
        .transform_js(js_code, Path::new("test.js"))
        .unwrap();

    // Plain JS should pass through unchanged
    assert_eq!(result.code.trim(), js_code.trim());
}
