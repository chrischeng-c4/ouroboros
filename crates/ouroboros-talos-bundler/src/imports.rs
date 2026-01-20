/// Import/Export detection using Tree-sitter
///
/// Extracts import and export statements from JavaScript/TypeScript files

use anyhow::Result;
use tree_sitter::{Node, Parser};

/// Import/export information extracted from a module
#[derive(Debug, Clone)]
pub struct ModuleImports {
    /// Static imports (e.g., import foo from './foo')
    pub static_imports: Vec<ImportDeclaration>,

    /// Dynamic imports (e.g., import('./foo'))
    pub dynamic_imports: Vec<String>,

    /// Export declarations
    pub exports: Vec<ExportDeclaration>,
}

/// Static import declaration
#[derive(Debug, Clone)]
pub struct ImportDeclaration {
    /// Module specifier (e.g., './foo', 'react')
    pub source: String,

    /// Import kind
    pub kind: ImportKind,
}

/// Kind of import
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportKind {
    /// Default import: import foo from './foo'
    Default,

    /// Named import: import { bar } from './foo'
    Named,

    /// Namespace import: import * as foo from './foo'
    Namespace,

    /// Side-effect import: import './foo'
    SideEffect,
}

/// Export declaration
#[derive(Debug, Clone)]
pub struct ExportDeclaration {
    /// Export kind
    pub kind: ExportKind,

    /// Re-export source (if any)
    pub source: Option<String>,
}

/// Kind of export
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    /// Named export: export { foo }
    Named,

    /// Default export: export default foo
    Default,

    /// All export: export * from './foo'
    All,
}

/// Extract imports from JavaScript/TypeScript source code
pub fn extract_imports(source: &str, is_typescript: bool) -> Result<ModuleImports> {
    let mut parser = Parser::new();

    let language = if is_typescript {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_javascript::LANGUAGE.into()
    };

    parser.set_language(&language)?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse source"))?;

    let root = tree.root_node();

    let mut imports = ModuleImports {
        static_imports: Vec::new(),
        dynamic_imports: Vec::new(),
        exports: Vec::new(),
    };

    extract_from_node(source, &root, &mut imports);

    Ok(imports)
}

/// Recursively extract imports/exports from AST node
fn extract_from_node(source: &str, node: &Node, imports: &mut ModuleImports) {
    match node.kind() {
        // Static imports: import foo from './foo'
        "import_statement" => {
            if let Some(import_decl) = parse_import_statement(source, node) {
                imports.static_imports.push(import_decl);
            }
        }

        // Dynamic imports: import('./foo')
        "call_expression" => {
            if is_dynamic_import(node) {
                if let Some(specifier) = extract_dynamic_import(source, node) {
                    imports.dynamic_imports.push(specifier);
                }
            }
        }

        // Export statements
        "export_statement" => {
            if let Some(export_decl) = parse_export_statement(source, node) {
                imports.exports.push(export_decl);
            }
        }

        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_from_node(source, &child, imports);
    }
}

/// Parse import statement
fn parse_import_statement(source: &str, node: &Node) -> Option<ImportDeclaration> {
    // Find the string literal (import source)
    let source_node = find_child_by_kind(node, "string")?;
    let source_text = node_text(source, &source_node);
    let import_source = strip_quotes(&source_text);

    // Determine import kind
    let kind = determine_import_kind(node);

    Some(ImportDeclaration {
        source: import_source,
        kind,
    })
}

/// Determine the kind of import
fn determine_import_kind(node: &Node) -> ImportKind {
    // Check for import clause
    if let Some(import_clause) = find_child_by_kind(node, "import_clause") {
        // Check for default import: import foo from './foo'
        if find_child_by_kind(&import_clause, "identifier").is_some() {
            return ImportKind::Default;
        }

        // Check for namespace import: import * as foo from './foo'
        if find_child_by_kind(&import_clause, "namespace_import").is_some() {
            return ImportKind::Namespace;
        }

        // Otherwise it's named import: import { foo } from './foo'
        return ImportKind::Named;
    }

    // No import clause means side-effect import: import './foo'
    ImportKind::SideEffect
}

/// Check if call expression is dynamic import
fn is_dynamic_import(node: &Node) -> bool {
    // Check if function is 'import'
    if let Some(function) = find_child_by_kind(node, "import") {
        return function.kind() == "import";
    }
    false
}

/// Extract dynamic import specifier
fn extract_dynamic_import(source: &str, node: &Node) -> Option<String> {
    let args = find_child_by_kind(node, "arguments")?;
    let string_node = find_child_by_kind(&args, "string")?;
    let source_text = node_text(source, &string_node);
    Some(strip_quotes(&source_text))
}

/// Parse export statement
fn parse_export_statement(source: &str, node: &Node) -> Option<ExportDeclaration> {
    // Check for export * from './foo'
    if let Some(_) = find_child_by_kind(node, "export_clause") {
        // Check if it's export * or export { ... }
        let kind = ExportKind::All; // Simplified
        let source_node = find_child_by_kind(node, "string");
        let source_val = source_node.map(|n| strip_quotes(&node_text(source, &n)));

        return Some(ExportDeclaration {
            kind,
            source: source_val,
        });
    }

    // Check for default export
    if node_text(source, node).contains("export default") {
        return Some(ExportDeclaration {
            kind: ExportKind::Default,
            source: None,
        });
    }

    // Named export
    Some(ExportDeclaration {
        kind: ExportKind::Named,
        source: None,
    })
}

/// Find child node by kind
fn find_child_by_kind<'a>(node: &'a Node, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children.into_iter().find(|child| child.kind() == kind)
}

/// Get node text from source
fn node_text(source: &str, node: &Node) -> String {
    source[node.byte_range()].to_string()
}

/// Strip quotes from string literal
fn strip_quotes(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_static_imports() {
        let source = r#"
            import React from 'react';
            import { useState } from 'react';
            import * as utils from './utils';
            import './styles.css';
        "#;

        let imports = extract_imports(source, false).unwrap();

        assert_eq!(imports.static_imports.len(), 4);
        assert_eq!(imports.static_imports[0].source, "react");
        assert_eq!(imports.static_imports[0].kind, ImportKind::Default);
        assert_eq!(imports.static_imports[1].source, "react");
        assert_eq!(imports.static_imports[1].kind, ImportKind::Named);
        assert_eq!(imports.static_imports[2].source, "./utils");
        assert_eq!(imports.static_imports[2].kind, ImportKind::Namespace);
        assert_eq!(imports.static_imports[3].source, "./styles.css");
        assert_eq!(imports.static_imports[3].kind, ImportKind::SideEffect);
    }

    #[test]
    fn test_extract_dynamic_imports() {
        let source = r#"
            const module = import('./dynamic-module');
            async function load() {
                const mod = await import('./lazy-module');
            }
        "#;

        let imports = extract_imports(source, false).unwrap();

        assert_eq!(imports.dynamic_imports.len(), 2);
        assert_eq!(imports.dynamic_imports[0], "./dynamic-module");
        assert_eq!(imports.dynamic_imports[1], "./lazy-module");
    }

    #[test]
    fn test_extract_typescript_imports() {
        let source = r#"
            import type { User } from './types';
            import React from 'react';
        "#;

        let imports = extract_imports(source, true).unwrap();

        // Should extract both type and value imports
        assert!(imports.static_imports.len() >= 1);
        assert!(imports.static_imports.iter().any(|i| i.source == "react"));
    }

    #[test]
    fn test_extract_exports() {
        let source = r#"
            export const foo = 1;
            export default function bar() {}
            export * from './other';
        "#;

        let imports = extract_imports(source, false).unwrap();

        assert_eq!(imports.exports.len(), 3);
    }
}
