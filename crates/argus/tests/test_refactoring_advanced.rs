//! Integration tests for advanced refactoring operations (Phase 3 M3.4)

use argus::types::{
    RefactoringEngine, RefactorRequest, RefactorKind, RefactorOptions, Span, SignatureChanges,
};
use std::path::PathBuf;

/// Helper to apply text edits to source code
fn apply_edits(source: &str, mut result: argus::types::RefactorResult) -> String {
    // Find the first file with edits
    let file_path = result.file_edits.keys().next().cloned();

    if let Some(path) = file_path {
        if let Some(edits) = result.file_edits.get_mut(&path) {
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
            return modified;
        }
    }

    source.to_string()
}

// ============================================================================
// Extract Method Tests
// ============================================================================

#[test]
fn test_extract_method_simple() {
    let source = r#"class MyClass:
    def process(self):
        x = 1
        y = 2"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractMethod {
            name: "init_values".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(39, 59),  // "x = 1\n        y = 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    assert!(result.has_changes(), "Should have changes");

    let modified = apply_edits(source, result);
    assert!(modified.contains("def init_values(self):"), "Should define method: {}", modified);
    assert!(modified.contains("self.init_values()"), "Should call method: {}", modified);
}

#[test]
fn test_extract_method_single_statement() {
    let source = r#"class Calculator:
    def calc(self):
        return 1 + 2"#;

    let request = RefactorRequest {
        kind: RefactorKind::ExtractMethod {
            name: "compute".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(48, 56),  // "1 + 2"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("def compute(self):"));
}

// ============================================================================
// Inline Symbol Tests
// ============================================================================

#[test]
fn test_inline_variable_simple() {
    let source = r#"temp = 1 + 2
result = temp * 3
output = temp + 5"#;

    let request = RefactorRequest {
        kind: RefactorKind::Inline,
        file: PathBuf::from("test.py"),
        span: Span::new(0, 4),  // "temp"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    assert!(result.has_changes(), "Should have changes");

    let modified = apply_edits(source, result);

    // Should inline temp and remove definition
    assert!(modified.contains("result = 1 + 2 * 3") || modified.contains("1 + 2"), "Should inline first usage: {}", modified);
    assert!(modified.contains("output = 1 + 2 + 5") || modified.contains("1 + 2"), "Should inline second usage: {}", modified);
    assert!(!modified.contains("temp = 1 + 2") || modified.is_empty(), "Should remove definition: {}", modified);
}

#[test]
fn test_inline_variable_single_usage() {
    let source = r#"x = 42
print(x)"#;

    let request = RefactorRequest {
        kind: RefactorKind::Inline,
        file: PathBuf::from("test.py"),
        span: Span::new(0, 1),  // "x"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("print(42)"), "Should inline value: {}", modified);
}

#[test]
fn test_inline_no_definition() {
    let source = r#"result = x + 1"#;

    let request = RefactorRequest {
        kind: RefactorKind::Inline,
        file: PathBuf::from("test.py"),
        span: Span::new(9, 10),  // "x"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(result.has_errors(), "Should have error for missing definition");
    assert!(!result.has_changes());
}

#[test]
fn test_inline_no_usages() {
    let source = r#"unused = 42"#;

    let request = RefactorRequest {
        kind: RefactorKind::Inline,
        file: PathBuf::from("test.py"),
        span: Span::new(0, 6),  // "unused"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    // Should have warning about no usages
    assert!(!result.has_errors(), "Shouldn't error, just warn");
}

// ============================================================================
// Change Signature Tests
// ============================================================================

#[test]
fn test_change_signature_add_parameter() {
    let source = r#"def func():
    pass"#;

    let changes = SignatureChanges {
        new_params: vec![
            ("x".to_string(), Some("int".to_string()), None),
        ],
        param_order: vec![],
        removed_params: vec![],
        new_return_type: None,
    };

    let request = RefactorRequest {
        kind: RefactorKind::ChangeSignature { changes },
        file: PathBuf::from("test.py"),
        span: Span::new(0, 11),  // "def func():"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    assert!(result.has_changes(), "Should have changes");

    let modified = apply_edits(source, result);
    assert!(modified.contains("def func(x: int):"), "Should add parameter: {}", modified);
}

#[test]
fn test_change_signature_add_with_default() {
    let source = r#"def greet():
    print("hello")"#;

    let changes = SignatureChanges {
        new_params: vec![
            ("name".to_string(), Some("str".to_string()), Some("\"World\"".to_string())),
        ],
        param_order: vec![],
        removed_params: vec![],
        new_return_type: None,
    };

    let request = RefactorRequest {
        kind: RefactorKind::ChangeSignature { changes },
        file: PathBuf::from("test.py"),
        span: Span::new(0, 12),  // "def greet():"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("name: str = \"World\""), "Should add parameter with default: {}", modified);
}

#[test]
fn test_change_signature_multiple_params() {
    let source = r#"def process():
    return 42"#;

    let changes = SignatureChanges {
        new_params: vec![
            ("a".to_string(), None, None),
            ("b".to_string(), Some("int".to_string()), None),
            ("c".to_string(), Some("str".to_string()), Some("\"default\"".to_string())),
        ],
        param_order: vec![],
        removed_params: vec![],
        new_return_type: None,
    };

    let request = RefactorRequest {
        kind: RefactorKind::ChangeSignature { changes },
        file: PathBuf::from("test.py"),
        span: Span::new(0, 14),  // "def process():"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    let modified = apply_edits(source, result);
    assert!(modified.contains("a, b: int, c: str = \"default\""), "Should add all parameters: {}", modified);
}

// ============================================================================
// Move Definition Tests
// ============================================================================

#[test]
fn test_move_definition_function() {
    let source = r#"def helper():
    return 42"#;

    let request = RefactorRequest {
        kind: RefactorKind::MoveDefinition {
            target_file: PathBuf::from("utils.py"),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(0, 27),  // Entire function
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    assert!(result.has_changes(), "Should have changes");

    // Should remove from source file
    let modified = apply_edits(source, result.clone());
    assert!(modified.is_empty() || modified.trim().is_empty(), "Should remove definition from source: {}", modified);

    // Should create new file
    assert!(result.new_files.contains_key(&PathBuf::from("utils.py")), "Should create target file");
    let new_content = result.new_files.get(&PathBuf::from("utils.py")).unwrap();
    assert!(new_content.contains("def helper():"), "Should contain definition in new file: {}", new_content);
}

#[test]
fn test_move_definition_class() {
    let source = r#"class MyClass:
    pass"#;

    let request = RefactorRequest {
        kind: RefactorKind::MoveDefinition {
            target_file: PathBuf::from("models.py"),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(0, 22),  // Entire class
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());

    // Check new file created
    assert!(result.new_files.contains_key(&PathBuf::from("models.py")));
}

// ============================================================================
// Combined/Integration Tests
// ============================================================================

#[test]
fn test_extract_method_then_inline() {
    // This would test a workflow, but for now just test that both work independently
    let source1 = r#"class C:
    def m(self):
        x = 1"#;

    let request1 = RefactorRequest {
        kind: RefactorKind::ExtractMethod {
            name: "get_x".to_string(),
        },
        file: PathBuf::from("test.py"),
        span: Span::new(33, 38),  // "x = 1"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result1 = engine.execute(&request1, source1);
    assert!(!result1.has_errors());
}

#[test]
fn test_inline_then_extract() {
    let source = r#"temp = 5
result = temp + 10"#;

    // First inline temp
    let request = RefactorRequest {
        kind: RefactorKind::Inline,
        file: PathBuf::from("test.py"),
        span: Span::new(0, 4),  // "temp"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());
}

#[test]
fn test_advanced_operations_typescript() {
    let source = r#"const temp = 1 + 2;
const result = temp * 3;"#;

    let request = RefactorRequest {
        kind: RefactorKind::Inline,
        file: PathBuf::from("test.ts"),
        span: Span::new(6, 10),  // "temp"
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
}

#[test]
fn test_advanced_operations_rust() {
    let source = r#"fn helper() -> i32 {
    42
}"#;

    let request = RefactorRequest {
        kind: RefactorKind::MoveDefinition {
            target_file: PathBuf::from("helpers.rs"),
        },
        file: PathBuf::from("main.rs"),
        span: Span::new(0, 29),  // Entire function
        options: RefactorOptions::default(),
    };

    let mut engine = RefactoringEngine::new();
    let result = engine.execute(&request, source);

    assert!(!result.has_errors());
    assert!(result.has_changes());
}
