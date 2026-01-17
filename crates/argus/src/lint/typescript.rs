//! TypeScript code checker

use crate::syntax::{Language, ParsedFile};
use crate::diagnostic::{Diagnostic, DiagnosticCategory, Range, TextEdit};
use crate::LintConfig;

/// TypeScript checker
pub struct TypeScriptChecker;

impl TypeScriptChecker {
    pub fn new() -> Self {
        Self
    }

    /// Check for non-null assertions (!)
    fn check_non_null_assertion(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "non_null_expression" {
                diagnostics.push(Diagnostic::warning(
                    Range::from_node(node),
                    "TS102",
                    DiagnosticCategory::Type,
                    "Non-null assertion (!) bypasses TypeScript's null checks",
                ));
            }
            true
        });

        diagnostics
    }

    /// Check for type assertions (as Type)
    fn check_type_assertion(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "as_expression" {
                diagnostics.push(Diagnostic::new(
                    Range::from_node(node),
                    crate::diagnostic::DiagnosticSeverity::Information,
                    "TS002",
                    DiagnosticCategory::Type,
                    "Type assertion may hide type errors",
                ));
            }
            true
        });

        diagnostics
    }

    /// Check for any type usage
    fn check_any_type(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "predefined_type" {
                let text = file.node_text(node);
                if text == "any" {
                    diagnostics.push(Diagnostic::warning(
                        Range::from_node(node),
                        "TS001",
                        DiagnosticCategory::Type,
                        "Avoid using 'any' type - use 'unknown' or a more specific type",
                    ));
                }
            }
            true
        });

        diagnostics
    }

    /// TS103: console.log statements left in code
    fn check_console_log(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "call_expression" {
                if let Some(func) = node.child_by_field_name("function") {
                    if func.kind() == "member_expression" {
                        let text = file.node_text(&func);
                        if text.starts_with("console.") {
                            diagnostics.push(Diagnostic::new(
                                Range::from_node(node),
                                crate::diagnostic::DiagnosticSeverity::Hint,
                                "TS103",
                                DiagnosticCategory::Style,
                                format!("'{}' statement should be removed in production", text),
                            ));
                        }
                    }
                }
            }
            true
        });

        diagnostics
    }

    /// TS104: Prefer const over let when variable is never reassigned
    fn check_prefer_const(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        use std::collections::{HashMap, HashSet};

        let mut diagnostics = Vec::new();
        // Store: name -> (name_range, declaration_range)
        let mut let_vars: HashMap<String, (Range, Range)> = HashMap::new();
        let mut reassigned: HashSet<String> = HashSet::new();

        // First pass: collect let declarations and reassignments
        file.walk(|node, _depth| {
            // Collect let declarations
            if node.kind() == "lexical_declaration" {
                let text = file.node_text(node);
                if text.starts_with("let ") {
                    let decl_range = Range::from_node(node);
                    // Find variable name
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declarator" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                let name = file.node_text(&name_node).to_string();
                                let_vars.insert(name, (Range::from_node(&name_node), decl_range.clone()));
                            }
                        }
                    }
                }
            }

            // Track reassignments
            if node.kind() == "assignment_expression" {
                if let Some(left) = node.child_by_field_name("left") {
                    if left.kind() == "identifier" {
                        reassigned.insert(file.node_text(&left).to_string());
                    }
                }
            }
            if node.kind() == "update_expression" {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        reassigned.insert(file.node_text(&child).to_string());
                    }
                }
            }

            true
        });

        // Report let declarations that are never reassigned
        for (name, (name_range, decl_range)) in let_vars {
            if !reassigned.contains(&name) {
                // Create fix: replace "let" with "const" in the declaration
                let fix_range = Range::new(
                    decl_range.start,
                    crate::diagnostic::Position::new(decl_range.start.line, decl_range.start.character + 3),
                );
                diagnostics.push(
                    Diagnostic::new(
                        name_range,
                        crate::diagnostic::DiagnosticSeverity::Hint,
                        "TS104",
                        DiagnosticCategory::Style,
                        format!("'{}' is never reassigned, use const instead", name),
                    )
                    .with_fix("Replace 'let' with 'const'", vec![
                        TextEdit { range: fix_range, new_text: "const".to_string() }
                    ])
                );
            }
        }

        diagnostics
    }
}

impl Default for TypeScriptChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Checker for TypeScriptChecker {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn check(&self, file: &ParsedFile, _config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for syntax errors
        if file.has_errors {
            file.walk(|node, _depth| {
                if node.is_error() || node.is_missing() {
                    diagnostics.push(Diagnostic::error(
                        Range::from_node(node),
                        "TS000",
                        DiagnosticCategory::Syntax,
                        "Syntax error",
                    ));
                }
                true
            });
        }

        diagnostics.extend(self.check_any_type(file));
        diagnostics.extend(self.check_non_null_assertion(file));
        diagnostics.extend(self.check_type_assertion(file));
        diagnostics.extend(self.check_console_log(file));
        diagnostics.extend(self.check_prefer_const(file));

        diagnostics
    }

    fn available_rules(&self) -> Vec<&'static str> {
        vec![
            "TS000", // Syntax error
            "TS001", // any type
            "TS002", // Type assertion
            "TS102", // Non-null assertion
            "TS103", // console.log
            "TS104", // Prefer const
        ]
    }
}
