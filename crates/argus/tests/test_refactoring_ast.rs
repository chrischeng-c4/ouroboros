//! Integration tests for refactoring engine AST integration (Phase 3 M3.1)

use argus::types::RefactoringEngine;
use std::path::PathBuf;

#[test]
fn test_ast_cache_population() {
    let code = r#"
def func1():
    pass

def func2():
    return 42
"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    // Populate AST cache
    let result = engine.populate_ast_cache(&file, code);
    assert!(result.is_ok(), "Failed to populate AST cache: {:?}", result.err());

    // Verify AST is in cache
    let ast_result = engine.get_ast(&file, code);
    assert!(ast_result.is_ok(), "Failed to get AST from cache");

    let ast = ast_result.unwrap();
    assert_eq!(ast.root().kind, "module", "Root node should be module");
    assert!(ast.root().children.len() > 0, "Module should have children");
}

#[test]
fn test_convert_node_structure() {
    let code = r#"def add(a, b):
    return a + b"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    engine.populate_ast_cache(&file, code).unwrap();
    let ast = engine.get_ast(&file, code).unwrap();

    // Verify root structure
    let root = ast.root();
    assert_eq!(root.kind, "module");

    // Find function definition
    let functions = ast.find_by_kind("function_definition");
    assert_eq!(functions.len(), 1, "Should find exactly one function");

    let func = functions[0];
    assert_eq!(func.kind, "function_definition");

    // Verify function has a name
    let identifiers = ast.find_by_kind("identifier");
    assert!(identifiers.len() > 0, "Should have identifier nodes");

    // Check that we have leaf nodes with values
    let leaf_nodes: Vec<_> = identifiers.iter()
        .filter(|n| n.value.is_some())
        .collect();
    assert!(leaf_nodes.len() > 0, "Should have leaf nodes with text values");
}

#[test]
fn test_find_at_span() {
    let code = r#"x = 1 + 2"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    engine.populate_ast_cache(&file, code).unwrap();
    let ast = engine.get_ast(&file, code).unwrap();

    // Find all number nodes
    let numbers = ast.find_by_kind("integer");
    assert!(numbers.len() >= 2, "Should find at least 2 numbers");

    // Try to find a node by its exact span
    let first_num = numbers[0];
    let found = ast.find_at_span(&first_num.span);
    assert!(found.is_some(), "Should find node at exact span");

    let found_node = found.unwrap();
    assert_eq!(found_node.span.start, first_num.span.start);
    assert_eq!(found_node.span.end, first_num.span.end);
}

#[test]
fn test_find_by_kind_multiple_results() {
    let code = r#"
def func1():
    pass

def func2():
    pass

class MyClass:
    def method1(self):
        pass
"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    engine.populate_ast_cache(&file, code).unwrap();
    let ast = engine.get_ast(&file, code).unwrap();

    // Find all function definitions (should include methods)
    let functions = ast.find_by_kind("function_definition");
    assert_eq!(functions.len(), 3, "Should find 3 functions (func1, func2, method1)");

    // Find class definition
    let classes = ast.find_by_kind("class_definition");
    assert_eq!(classes.len(), 1, "Should find 1 class");
}

#[test]
fn test_find_at_position() {
    let code = r#"def func():
    x = 1
    return x"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    engine.populate_ast_cache(&file, code).unwrap();
    let ast = engine.get_ast(&file, code).unwrap();

    // Find node at position (line 1, col 4) - should be inside "func"
    // Note: tree-sitter uses 0-based line/col
    let found = ast.find_at_position(0, 4);
    assert!(found.is_some(), "Should find node at position");

    // Find node at position (line 1, col 8) - should be inside assignment
    let found_stmt = ast.find_at_position(1, 8);
    assert!(found_stmt.is_some(), "Should find statement node");
}

#[test]
fn test_find_at_position_returns_innermost() {
    let code = r#"result = 1 + 2 + 3"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    engine.populate_ast_cache(&file, code).unwrap();
    let ast = engine.get_ast(&file, code).unwrap();

    // Find node at position of "1" (approximately col 9)
    // Should return the integer node, not the binary_operator or module
    let found = ast.find_at_position(0, 9);
    assert!(found.is_some(), "Should find node at position");

    let node = found.unwrap();
    // The innermost node should be more specific than "module"
    assert_ne!(node.kind, "module", "Should find innermost node, not root");
}

#[test]
fn test_span_tracking() {
    let code = r#"def hello():
    print("world")"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    engine.populate_ast_cache(&file, code).unwrap();
    let ast = engine.get_ast(&file, code).unwrap();

    // Find function definition
    let functions = ast.find_by_kind("function_definition");
    assert_eq!(functions.len(), 1);

    let func = functions[0];

    // Verify span has both byte positions and line/col positions
    assert!(func.span.start < func.span.end, "Start should be before end");
    assert!(func.span.start_line <= func.span.end_line, "Start line should be <= end line");

    // Function should start at line 0 (0-based)
    assert_eq!(func.span.start_line, 0, "Function should start at line 0");
}

#[test]
fn test_lazy_loading() {
    let code = r#"x = 1"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    // get_ast should populate cache if not present
    let ast1 = engine.get_ast(&file, code);
    assert!(ast1.is_ok(), "First get_ast should succeed");

    // Verify the first AST
    let root_kind1 = ast1.unwrap().root().kind.clone();

    // Second call should return cached AST
    let ast2 = engine.get_ast(&file, code);
    assert!(ast2.is_ok(), "Second get_ast should return cached AST");

    // Both should have the same structure
    let root_kind2 = ast2.unwrap().root().kind.clone();
    assert_eq!(root_kind1, root_kind2);
}

#[test]
fn test_get_ast_mut() {
    let code = r#"y = 2"#;

    let mut engine = RefactoringEngine::new();
    let file = PathBuf::from("test.py");

    // get_ast_mut should populate cache and return mutable reference
    let ast_mut = engine.get_ast_mut(&file, code);
    assert!(ast_mut.is_ok(), "get_ast_mut should succeed");

    // Verify we can access the AST
    let ast = ast_mut.unwrap();
    assert_eq!(ast.root().kind, "module");
}

#[test]
fn test_detect_language_from_file_extension() {
    let python_file = PathBuf::from("test.py");
    let typescript_file = PathBuf::from("test.ts");
    let rust_file = PathBuf::from("test.rs");

    let mut engine = RefactoringEngine::new();

    // Python should work
    let result = engine.populate_ast_cache(&python_file, "x = 1");
    assert!(result.is_ok(), "Should parse Python file");

    // TypeScript should work
    let result = engine.populate_ast_cache(&typescript_file, "const x = 1;");
    assert!(result.is_ok(), "Should parse TypeScript file");

    // Rust should work
    let result = engine.populate_ast_cache(&rust_file, "fn main() {}");
    assert!(result.is_ok(), "Should parse Rust file");
}

#[test]
fn test_invalid_file_extension() {
    let invalid_file = PathBuf::from("test.xyz");
    let mut engine = RefactoringEngine::new();

    let result = engine.populate_ast_cache(&invalid_file, "x = 1");
    assert!(result.is_err(), "Should fail for unknown file extension");

    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Failed to detect language"),
            "Error should mention language detection: {}", err_msg);
}
