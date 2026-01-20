//! Integration tests for extract variable and extract function refactoring (Phase 3 M3.2)

use argus::types::{RefactoringEngine, RefactorRequest, RefactorKind, RefactorOptions, Span};
use std::path::PathBuf;

/// Helper to apply text edits to source code
fn apply_edits(source: &str, mut result: argus::types::RefactorResult) -> String {
    // Sort edits by position (reverse order to apply from end to start)
    let file_path = PathBuf::from("test.py");
    if let Some(edits) = result.file_edits.get_mut(&file_path) {
        edits.sort_by(|a, b| {
            // Sort by start position in reverse, but if same start, sort by end in reverse
            match b.span.start.cmp(&a.span.start) {
                std::cmp::Ordering::Equal => b.span.end.cmp(&a.span.end),
                other => other,
            }
        });

        let mut modified = source.to_string();
        for edit in edits {
            let before = &modified[..edit.span.start];
            let after = &modified[edit.span.end..];
            modified = format!("{}{}{}", before, edit.new_text, after);
        }
        modified
    } else {
        source.to_string()
    }
}

#[test]
fn test_extract_variable_simple_expression() {
    let source = r#"result = 1 + 2"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "temp".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(9, 14),  // "1 + 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    assert!(result.has_changes(), "Should have changes");

    let modified = apply_edits(source, result);

    // Should insert assignment before and replace expression
    assert!(modified.contains("temp = 1 + 2"), "Should contain variable assignment: {}", modified);
    assert!(modified.contains("result = temp"), "Should replace expression with variable: {}", modified);
}

#[test]
fn test_extract_variable_function_call() {
    let source = r#"x = len("hello")"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "length".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(4, 16),  // len("hello")
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("length = len(\"hello\")"), "Should extract function call: {}", modified);
}

#[test]
fn test_extract_variable_preserves_indentation() {
    let source = r#"def func():
    result = 1 + 2"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "sum_val".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(25, 30),  // "1 + 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);

    // Check that indentation is preserved
    assert!(modified.contains("    sum_val = 1 + 2"), "Should preserve indentation: {}", modified);
}

#[test]
fn test_extract_variable_multiline() {
    let source = r#"x = 1
y = x + 2
z = y * 3"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "intermediate".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(10, 15),  // "x + 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("intermediate = x + 2"));
    assert!(modified.contains("y = intermediate"));
}

#[test]
fn test_extract_function_simple_statement() {
    let source = r#"print("hello")
print("world")"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractFunction {
            name: "greet".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(0, 14),  // print("hello")
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    assert!(result.has_changes(), "Should have changes");

    let modified = apply_edits(source, result);

    // Should create function definition
    assert!(modified.contains("def greet():"), "Should define function: {}", modified);
    assert!(modified.contains("print(\"hello\")"), "Should include original code in function: {}", modified);

    // Should replace with function call
    assert!(modified.contains("greet()"), "Should call the function: {}", modified);
}

#[test]
fn test_extract_function_multiple_lines() {
    let source = r#"x = 1
y = 2
z = x + y
print(z)"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractFunction {
            name: "calculate_sum".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(6, 21),  // "y = 2\nz = x + y"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);

    // Should create function with both lines
    assert!(modified.contains("def calculate_sum():"));
    assert!(modified.contains("calculate_sum()"));
}

#[test]
fn test_extract_function_preserves_indentation() {
    let source = r#"def main():
    x = 1
    y = 2"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractFunction {
            name: "init_vars".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(16, 31),  // "x = 1\n    y = 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);

    // Function definition should be at module level (no indentation)
    assert!(modified.contains("def init_vars():"));

    // Call should preserve original indentation
    assert!(modified.contains("    init_vars()"));
}

#[test]
fn test_extract_function_single_expression() {
    let source = r#"result = 1 + 2 + 3"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractFunction {
            name: "compute".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(9, 18),  // "1 + 2 + 3"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("def compute():"));
    assert!(modified.contains("compute()"));
}

#[test]
fn test_extract_variable_with_method_chain() {
    let source = r#"name = user.get_name().strip()"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "raw_name".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(7, 30),  // "user.get_name().strip()" - fixed end position
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("raw_name = user.get_name().strip()"));
    assert!(modified.contains("name = raw_name"));
}

#[test]
fn test_extract_function_empty_selection() {
    let source = r#"x = 1"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractFunction {
            name: "empty_func".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(3, 3),  // Empty selection
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    // Empty selection should still work (extract empty code)
    assert!(!result.has_errors());
}

#[test]
fn test_extract_variable_number_literal() {
    let source = r#"total = price * 1.08"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "tax_rate".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(16, 20),  // "1.08"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("tax_rate = 1.08"));
    assert!(modified.contains("total = price * tax_rate"));
}

#[test]
fn test_extract_function_return_value() {
    let source = r#"def main():
    x = 5
    return x * 2"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractFunction {
            name: "double".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(28, 36),  // "x * 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("def double():"));
}

#[test]
fn test_extract_operations_on_typescript() {
    let source = r#"const x = 1 + 2;"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "sum".to_string(),
        },
        file: PathBuf::from("test.ts"),
        span: Span::new(10, 15),  // "1 + 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    // Should work with TypeScript too
    assert!(!result.has_errors());
    assert!(result.has_changes());
}

#[test]
fn test_extract_operations_on_rust() {
    let source = r#"fn main() {
    let x = 1 + 2;
}"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractVariable {
            name: "sum".to_string(),
        },
        file: PathBuf::from("test.rs"),
        span: Span::new(24, 29),  // "1 + 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    // Should work with Rust too
    assert!(!result.has_errors());
    assert!(result.has_changes());
}
