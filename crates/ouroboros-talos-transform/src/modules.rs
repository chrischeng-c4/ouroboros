use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

use crate::TransformResult;

/// Module mapping for resolving import paths
#[derive(Debug, Clone)]
pub enum ModuleMapping {
    /// Internal module with numeric ID
    Internal(usize),
    /// External module with package name
    External(String),
}

/// Transform ES6 module syntax (import/export) to CommonJS (require/module.exports)
pub fn transform_modules(
    source: &str,
    module_map: &HashMap<PathBuf, usize>,
) -> Result<TransformResult> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_javascript::LANGUAGE.into())?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse JavaScript"))?;

    let root = tree.root_node();

    // Transform the AST
    let transformed = transform_node(source, &root, module_map)?;

    Ok(TransformResult {
        code: transformed,
        source_map: None,
    })
}

/// Transform a single AST node
fn transform_node(
    source: &str,
    node: &Node,
    module_map: &HashMap<PathBuf, usize>,
) -> Result<String> {
    let mut result = String::new();
    let mut cursor = node.walk();
    let mut last_pos = node.start_byte();

    for child in node.children(&mut cursor) {
        // Preserve whitespace before this child
        if child.start_byte() > last_pos {
            result.push_str(&source[last_pos..child.start_byte()]);
        }

        match child.kind() {
            "import_statement" => {
                result.push_str(&transform_import(source, &child, module_map)?);
                // Don't add extra newline - it's already in the source
                last_pos = child.end_byte();
            }
            "export_statement" => {
                result.push_str(&transform_export(source, &child, module_map)?);
                // Don't add extra newline - it's already in the source
                last_pos = child.end_byte();
            }
            _ => {
                if child.child_count() > 0 {
                    result.push_str(&transform_node(source, &child, module_map)?);
                } else {
                    result.push_str(&source[child.byte_range()]);
                }
                last_pos = child.end_byte();
            }
        }
    }

    // Append any remaining source after the last child
    if last_pos < node.end_byte() {
        result.push_str(&source[last_pos..node.end_byte()]);
    }

    Ok(result)
}

/// Transform import statement to require()
fn transform_import(
    source: &str,
    node: &Node,
    module_map: &HashMap<PathBuf, usize>,
) -> Result<String> {
    let mut cursor = node.walk();
    let mut import_clause = None;
    let mut source_path = None;

    // Extract import clause and source
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_clause" => {
                import_clause = Some(child);
            }
            "string" => {
                let path_str = &source[child.byte_range()];
                // Remove quotes
                source_path = Some(path_str.trim_matches('"').trim_matches('\'').to_string());
            }
            _ => {}
        }
    }

    // Handle side-effect imports: import './styles.css'
    if import_clause.is_none() {
        if let Some(path) = source_path {
            return Ok(format!("require('{}');", path));
        }
        return Ok(String::new());
    }

    let import_clause = import_clause.unwrap();
    let source_path = source_path.ok_or_else(|| anyhow::anyhow!("Missing import source"))?;

    // Determine require target (numeric ID or string)
    let require_target = resolve_module_path(&source_path, module_map);

    // Parse import clause
    let import_spec = parse_import_clause(source, &import_clause)?;

    match import_spec {
        ImportSpec::DefaultImport(name) => {
            // import React from 'react'
            // → var React = require('react').default || require('react')
            Ok(format!(
                "var {} = {}[\"default\"] || {};",
                name, require_target, require_target
            ))
        }
        ImportSpec::NamespaceImport(name) => {
            // import * as utils from './utils'
            // → var utils = require(2)
            Ok(format!("var {} = {};", name, require_target))
        }
        ImportSpec::NamedImports(names) => {
            // import { useState, useEffect } from 'react'
            // → var useState = require('react').useState; var useEffect = require('react').useEffect;
            let requires: Vec<String> = names
                .iter()
                .map(|(imported, local)| {
                    format!(
                        "var {} = {}[\"{}\"];",
                        local,
                        require_target,
                        imported
                    )
                })
                .collect();
            Ok(requires.join(" "))
        }
        ImportSpec::Mixed(default_name, named_imports) => {
            // import React, { useState } from 'react'
            let mut statements = vec![format!(
                "var {} = {}[\"default\"] || {};",
                default_name, require_target, require_target
            )];
            for (imported, local) in named_imports {
                statements.push(format!(
                    "var {} = {}[\"{}\"];",
                    local, require_target, imported
                ));
            }
            Ok(statements.join(" "))
        }
    }
}

/// Transform export statement to module.exports
fn transform_export(
    source: &str,
    node: &Node,
    _module_map: &HashMap<PathBuf, usize>,
) -> Result<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "export" => continue,
            "default" => {
                // export default App
                // → module.exports.default = App; module.exports = App;
                let value = extract_export_value(source, node)?;
                return Ok(format!(
                    "module.exports[\"default\"] = {}; module.exports = {};",
                    value, value
                ));
            }
            "lexical_declaration" | "variable_declaration" | "function_declaration"
            | "class_declaration" => {
                // export const foo = 1; or export function bar() {}
                let declaration = &source[child.byte_range()];
                let export_names = extract_declaration_names(&child, source)?;

                let mut result = String::new();
                result.push_str(declaration);
                result.push_str("; ");

                for name in export_names {
                    result.push_str(&format!("module.exports[\"{}\"] = {}; ", name, name));
                }

                return Ok(result);
            }
            "export_clause" => {
                // export { foo, bar }
                let names = parse_export_clause(source, &child)?;
                let exports: Vec<String> = names
                    .iter()
                    .map(|(local, exported)| {
                        format!("module.exports[\"{}\"] = {};", exported, local)
                    })
                    .collect();
                return Ok(exports.join(" "));
            }
            _ => {}
        }
    }

    Ok(String::new())
}

/// Resolve module path to require() target
fn resolve_module_path(path: &str, module_map: &HashMap<PathBuf, usize>) -> String {
    // Try to find in module map
    let path_buf = PathBuf::from(path);
    if let Some(&id) = module_map.get(&path_buf) {
        return format!("require({})", id);
    }

    // Check for relative paths
    if path.starts_with('.') {
        // Try common extensions
        for ext in &["", ".js", ".jsx", ".ts", ".tsx"] {
            let mut test_path = path_buf.clone();
            if !ext.is_empty() {
                test_path.set_extension(&ext[1..]);
            }
            if let Some(&id) = module_map.get(&test_path) {
                return format!("require({})", id);
            }
        }
    }

    // External module (npm package)
    format!("require('{}')", path)
}

/// Extract value from export default statement
fn extract_export_value(source: &str, node: &Node) -> Result<String> {
    let mut cursor = node.walk();
    let mut found_default = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "default" {
            found_default = true;
            continue;
        }
        if found_default && child.kind() != "export" && child.kind() != ";" {
            return Ok(source[child.byte_range()].to_string());
        }
    }

    Err(anyhow::anyhow!("Could not extract export default value"))
}

/// Extract names from declaration (const, function, class)
fn extract_declaration_names(node: &Node, source: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "variable_declarator" => {
                // const foo = 1
                if let Some(name_node) = child.child_by_field_name("name") {
                    names.push(source[name_node.byte_range()].to_string());
                }
            }
            "function_declaration" | "class_declaration" => {
                // function foo() {} or class Bar {}
                if let Some(name_node) = child.child_by_field_name("name") {
                    names.push(source[name_node.byte_range()].to_string());
                }
            }
            "identifier" => {
                // Direct identifier in declaration
                names.push(source[child.byte_range()].to_string());
            }
            _ => {
                // Recurse for nested structures
                if child.child_count() > 0 {
                    names.extend(extract_declaration_names(&child, source)?);
                }
            }
        }
    }

    Ok(names)
}

/// Parse export clause: export { foo, bar as baz }
fn parse_export_clause(source: &str, clause: &Node) -> Result<Vec<(String, String)>> {
    let mut exports = Vec::new();
    let mut cursor = clause.walk();

    for child in clause.children(&mut cursor) {
        if child.kind() == "export_specifier" {
            let (local, exported) = parse_export_specifier(source, &child)?;
            exports.push((local, exported));
        }
    }

    Ok(exports)
}

/// Parse single export specifier
fn parse_export_specifier(source: &str, node: &Node) -> Result<(String, String)> {
    let mut local = None;
    let mut exported = None;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if local.is_none() {
                    local = Some(source[child.byte_range()].to_string());
                } else {
                    exported = Some(source[child.byte_range()].to_string());
                }
            }
            _ => {}
        }
    }

    let local = local.ok_or_else(|| anyhow::anyhow!("Missing local name in export specifier"))?;
    let exported = exported.unwrap_or_else(|| local.clone());

    Ok((local, exported))
}

/// Import specification types
#[derive(Debug)]
enum ImportSpec {
    /// import React from 'react'
    DefaultImport(String),
    /// import * as utils from './utils'
    NamespaceImport(String),
    /// import { foo, bar } from './module'
    NamedImports(Vec<(String, String)>),
    /// import React, { useState } from 'react'
    Mixed(String, Vec<(String, String)>),
}

/// Parse import clause
fn parse_import_clause(source: &str, clause: &Node) -> Result<ImportSpec> {
    let mut cursor = clause.walk();
    let mut default_import = None;
    let mut namespace_import = None;
    let mut named_imports = Vec::new();

    for child in clause.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                // Default import
                default_import = Some(source[child.byte_range()].to_string());
            }
            "namespace_import" => {
                // import * as name
                namespace_import = Some(parse_namespace_import(source, &child)?);
            }
            "named_imports" => {
                // import { ... }
                named_imports = parse_named_imports(source, &child)?;
            }
            _ => {}
        }
    }

    // Determine import type
    match (default_import, namespace_import, named_imports.is_empty()) {
        (Some(default), None, true) => Ok(ImportSpec::DefaultImport(default)),
        (None, Some(namespace), _) => Ok(ImportSpec::NamespaceImport(namespace)),
        (None, None, false) => Ok(ImportSpec::NamedImports(named_imports)),
        (Some(default), None, false) => Ok(ImportSpec::Mixed(default, named_imports)),
        _ => Err(anyhow::anyhow!("Invalid import clause")),
    }
}

/// Parse namespace import: * as name
fn parse_namespace_import(source: &str, node: &Node) -> Result<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Ok(source[child.byte_range()].to_string());
        }
    }

    Err(anyhow::anyhow!("Missing identifier in namespace import"))
}

/// Parse named imports: { foo, bar as baz }
fn parse_named_imports(source: &str, node: &Node) -> Result<Vec<(String, String)>> {
    let mut imports = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "import_specifier" {
            let (imported, local) = parse_import_specifier(source, &child)?;
            imports.push((imported, local));
        }
    }

    Ok(imports)
}

/// Parse import specifier: foo or bar as baz
fn parse_import_specifier(source: &str, node: &Node) -> Result<(String, String)> {
    let mut imported = None;
    let mut local = None;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if imported.is_none() {
                    imported = Some(source[child.byte_range()].to_string());
                } else {
                    local = Some(source[child.byte_range()].to_string());
                }
            }
            _ => {}
        }
    }

    let imported =
        imported.ok_or_else(|| anyhow::anyhow!("Missing imported name in import specifier"))?;
    let local = local.unwrap_or_else(|| imported.clone());

    Ok((imported, local))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_module_map() -> HashMap<PathBuf, usize> {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("./utils.js"), 1);
        map.insert(PathBuf::from("./components/Button.jsx"), 2);
        map
    }

    #[test]
    fn test_import_default() {
        let source = "import React from 'react';";
        let map = HashMap::new();
        let result = transform_modules(source, &map).unwrap();
        assert!(result.code.contains("var React"));
        assert!(result.code.contains("require('react')"));
    }

    #[test]
    fn test_import_named() {
        let source = "import { useState, useEffect } from 'react';";
        let map = HashMap::new();
        let result = transform_modules(source, &map).unwrap();
        assert!(result.code.contains("var useState"));
        assert!(result.code.contains("var useEffect"));
    }

    #[test]
    fn test_import_namespace() {
        let source = "import * as utils from './utils.js';";
        let map = test_module_map();
        let result = transform_modules(source, &map).unwrap();
        assert!(result.code.contains("var utils"));
        assert!(result.code.contains("require(1)"));
    }

    #[test]
    fn test_export_default() {
        let source = "export default App;";
        let map = HashMap::new();
        let result = transform_modules(source, &map).unwrap();
        assert!(result.code.contains("module.exports"));
        assert!(result.code.contains("App"));
    }

    #[test]
    fn test_export_named() {
        let source = "export const foo = 1;";
        let map = HashMap::new();
        let result = transform_modules(source, &map).unwrap();
        assert!(result.code.contains("const foo"));
        assert!(result.code.contains("module.exports"));
    }

    #[test]
    fn test_side_effect_import() {
        let source = "import './styles.css';";
        let map = HashMap::new();
        let result = transform_modules(source, &map).unwrap();
        assert!(result.code.contains("require('./styles.css')"));
    }
}
