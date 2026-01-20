use ouroboros_talos_transform::{Transformer, TransformOptions};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_jsx_with_es6_modules() {
    let source = r#"import React, { useState } from 'react';

const App = () => <div>Hello</div>;

export default App;"#;

    let transformer = Transformer::new(TransformOptions::default());
    let module_map = HashMap::new();

    let result = transformer.transform_js_with_context(
        source,
        &PathBuf::from("test.jsx"),
        &module_map
    ).unwrap();

    println!("Transformed code:\n{}", result.code);

    // Should contain CommonJS require
    assert!(result.code.contains("var React") || result.code.contains("require"));

    // Should contain JSX transformation
    assert!(result.code.contains("jsx(") || result.code.contains("jsxs("));

    // Should NOT contain raw import/export
    assert!(!result.code.contains("import React"));
    assert!(!result.code.contains("export default"));
}

#[test]
fn test_plain_js_with_es6_modules() {
    let source = r#"import { foo } from './bar';
export const baz = 42;"#;

    let transformer = Transformer::new(TransformOptions::default());
    let mut module_map = HashMap::new();
    module_map.insert(PathBuf::from("./bar"), 1);

    let result = transformer.transform_js_with_context(
        source,
        &PathBuf::from("test.js"),
        &module_map
    ).unwrap();

    println!("Transformed code:\n{}", result.code);

    // Should contain CommonJS
    assert!(result.code.contains("var foo"));
    assert!(result.code.contains("require"));
    assert!(result.code.contains("module.exports"));

    // Should NOT contain raw import/export
    assert!(!result.code.contains("import {"));
    assert!(!result.code.contains("export const"));
}
