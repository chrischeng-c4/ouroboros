//! Rust code checker

use crate::syntax::{Language, ParsedFile};
use crate::diagnostic::{Diagnostic, DiagnosticCategory, DiagnosticSeverity, Range};
use crate::LintConfig;

/// Rust checker
pub struct RustChecker;

impl RustChecker {
    pub fn new() -> Self {
        Self
    }

    /// Check for unsafe blocks
    fn check_unsafe_blocks(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "unsafe_block" {
                diagnostics.push(Diagnostic::new(
                    Range::from_node(node),
                    DiagnosticSeverity::Information,
                    "RS201",
                    DiagnosticCategory::Security,
                    "Unsafe block - ensure memory safety is manually verified",
                ));
            }
            true
        });

        diagnostics
    }

    /// Check for .clone() calls that might be unnecessary
    fn check_clone_usage(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "call_expression" {
                if let Some(func) = node.child_by_field_name("function") {
                    if func.kind() == "field_expression" {
                        if let Some(field) = func.child_by_field_name("field") {
                            if file.node_text(&field) == "clone" {
                                diagnostics.push(Diagnostic::new(
                                    Range::from_node(node),
                                    DiagnosticSeverity::Hint,
                                    "RS001",
                                    DiagnosticCategory::Style,
                                    "Consider if .clone() is necessary - borrowing may be more efficient",
                                ));
                            }
                        }
                    }
                }
            }
            true
        });

        diagnostics
    }

    /// Check for unwrap() calls
    fn check_unwrap(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "call_expression" {
                if let Some(func) = node.child_by_field_name("function") {
                    if func.kind() == "field_expression" {
                        if let Some(field) = func.child_by_field_name("field") {
                            let field_name = file.node_text(&field);
                            if field_name == "unwrap" || field_name == "expect" {
                                diagnostics.push(Diagnostic::warning(
                                    Range::from_node(node),
                                    "RS101",
                                    DiagnosticCategory::Logic,
                                    format!(
                                        ".{}() can panic - consider using ? or match for error handling",
                                        field_name
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            true
        });

        diagnostics
    }

    /// RS102: todo!/unimplemented! macros
    fn check_todo_macros(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "macro_invocation" {
                if let Some(macro_node) = node.child_by_field_name("macro") {
                    let macro_name = file.node_text(&macro_node);
                    if macro_name == "todo" || macro_name == "unimplemented" {
                        diagnostics.push(Diagnostic::warning(
                            Range::from_node(node),
                            "RS102",
                            DiagnosticCategory::Logic,
                            format!("{}! macro will panic at runtime", macro_name),
                        ));
                    }
                }
            }
            true
        });

        diagnostics
    }

    /// RS103: dbg! macro left in code
    fn check_dbg_macro(&self, file: &ParsedFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        file.walk(|node, _depth| {
            if node.kind() == "macro_invocation" {
                if let Some(macro_node) = node.child_by_field_name("macro") {
                    let macro_name = file.node_text(&macro_node);
                    if macro_name == "dbg" {
                        diagnostics.push(Diagnostic::new(
                            Range::from_node(node),
                            DiagnosticSeverity::Hint,
                            "RS103",
                            DiagnosticCategory::Style,
                            "dbg! macro should be removed in production",
                        ));
                    }
                }
            }
            true
        });

        diagnostics
    }
}

impl Default for RustChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Checker for RustChecker {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn check(&self, file: &ParsedFile, _config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for syntax errors
        if file.has_errors {
            file.walk(|node, _depth| {
                if node.is_error() || node.is_missing() {
                    diagnostics.push(Diagnostic::error(
                        Range::from_node(node),
                        "RS000",
                        DiagnosticCategory::Syntax,
                        "Syntax error",
                    ));
                }
                true
            });
        }

        diagnostics.extend(self.check_unsafe_blocks(file));
        diagnostics.extend(self.check_clone_usage(file));
        diagnostics.extend(self.check_unwrap(file));
        diagnostics.extend(self.check_todo_macros(file));
        diagnostics.extend(self.check_dbg_macro(file));

        diagnostics
    }

    fn available_rules(&self) -> Vec<&'static str> {
        vec![
            "RS000", // Syntax error
            "RS001", // Unnecessary clone
            "RS101", // unwrap/expect usage
            "RS102", // todo!/unimplemented!
            "RS103", // dbg! macro
            "RS201", // Unsafe block
        ]
    }
}
