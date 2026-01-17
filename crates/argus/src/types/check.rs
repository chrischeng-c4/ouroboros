//! Type checking - verifies type compatibility and generates diagnostics

use std::collections::HashMap;
use tree_sitter::Node;

use super::infer::TypeInferencer;
use super::ty::{ParamKind, Type};
use crate::diagnostic::{Diagnostic, DiagnosticCategory, DiagnosticSeverity, Range};
use crate::syntax::ParsedFile;

/// Type error information
#[derive(Debug, Clone)]
pub struct TypeError {
    pub range: Range,
    pub expected: Type,
    pub got: Type,
    pub message: String,
}

/// Function context for tracking return types
#[derive(Debug, Clone)]
struct FunctionContext {
    name: String,
    return_type: Type,
    has_return: bool,
}

/// Type checker that combines inference with compatibility checking
pub struct TypeChecker<'a> {
    /// Type inferencer
    inferencer: TypeInferencer<'a>,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
    /// Source code
    source: &'a str,
    /// Stack of function contexts (for nested functions)
    function_stack: Vec<FunctionContext>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inferencer: TypeInferencer::new(source),
            diagnostics: Vec::new(),
            source,
            function_stack: Vec::new(),
        }
    }

    /// Get current function context
    fn current_function(&self) -> Option<&FunctionContext> {
        self.function_stack.last()
    }

    /// Mark current function as having a return statement
    fn mark_has_return(&mut self) {
        if let Some(ctx) = self.function_stack.last_mut() {
            ctx.has_return = true;
        }
    }

    /// Check a file and return diagnostics
    pub fn check_file(&mut self, file: &ParsedFile) -> Vec<Diagnostic> {
        let root = file.tree.root_node();
        self.check_node(&root);
        std::mem::take(&mut self.diagnostics)
    }

    /// Recursively check a node
    fn check_node(&mut self, node: &Node) {
        match node.kind() {
            "class_definition" => {
                self.check_class(node);
                return; // check_class handles its own recursion
            }
            "function_definition" | "async_function_definition" => {
                self.check_function(node);
                return; // check_function handles its own recursion
            }
            "assignment" => {
                self.check_assignment(node);
            }
            "return_statement" => {
                self.check_return(node);
            }
            "call" => {
                self.check_call(node);
            }
            "attribute" => {
                self.check_attribute(node);
            }
            _ => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.check_node(&child);
        }
    }

    /// Check function definition
    fn check_function(&mut self, node: &Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n).to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Get return type annotation
        let return_type = node
            .child_by_field_name("return_type")
            .map(|rt| self.inferencer.parse_type_annotation(&rt))
            .unwrap_or(Type::Unknown);

        // Check for missing return type annotation (only for public functions)
        if node.child_by_field_name("return_type").is_none() && !name.starts_with('_') {
            self.diagnostics.push(Diagnostic::new(
                Range::from_node(node),
                DiagnosticSeverity::Hint,
                "TC002",
                DiagnosticCategory::Type,
                format!("Function '{}' is missing return type annotation", name),
            ));
        }

        // Analyze function and add to environment
        self.inferencer.analyze_function(node);

        // Push function context
        self.function_stack.push(FunctionContext {
            name: name.clone(),
            return_type: return_type.clone(),
            has_return: false,
        });

        // Check function body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                self.check_node(&child);
            }
        }

        // Pop function context and check for missing return
        if let Some(ctx) = self.function_stack.pop() {
            // If function has declared return type (not None/Unknown) but no return
            if !ctx.has_return
                && !matches!(ctx.return_type, Type::None | Type::Unknown | Type::Any)
            {
                self.diagnostics.push(Diagnostic::warning(
                    Range::from_node(node),
                    "TC003",
                    DiagnosticCategory::Type,
                    format!(
                        "Function '{}' declares return type '{}' but may not return a value",
                        ctx.name, ctx.return_type
                    ),
                ));
            }
        }
    }

    /// Check class definition
    fn check_class(&mut self, node: &Node) {
        // Analyze class and register in inferencer
        self.inferencer.analyze_class(node);

        // Check class body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                self.check_node(&child);
            }
        }
    }

    /// Check assignment for type consistency
    fn check_assignment(&mut self, node: &Node) {
        let left = node.child_by_field_name("left");
        let right = node.child_by_field_name("right");

        if let (Some(target), Some(value)) = (left, right) {
            let value_type = self.inferencer.infer_expr(&value);

            // Check for annotated assignment
            if let Some(type_node) = node.child_by_field_name("type") {
                let expected = self.inferencer.parse_type_annotation(&type_node);

                if !self.is_assignable(&expected, &value_type) {
                    self.diagnostics.push(Diagnostic::error(
                        Range::from_node(&value),
                        "TC001",
                        DiagnosticCategory::Type,
                        format!(
                            "Type mismatch: expected '{}', got '{}'",
                            expected, value_type
                        ),
                    ));
                }
            }

            // Bind the variable
            self.inferencer.bind_assignment(&target, value_type);
        }
    }

    /// Check return statement
    fn check_return(&mut self, node: &Node) {
        // Mark that we have a return in this function
        self.mark_has_return();

        // Get expected return type from function context
        let expected_return = self
            .current_function()
            .map(|ctx| ctx.return_type.clone())
            .unwrap_or(Type::Unknown);

        // Get actual return value type
        let actual_return = if let Some(value) = node.child(1) {
            self.inferencer.infer_expr(&value)
        } else {
            Type::None // bare "return" returns None
        };

        // Check compatibility
        if !expected_return.is_unknown() && !expected_return.is_any() {
            if !self.is_assignable(&expected_return, &actual_return) {
                self.diagnostics.push(Diagnostic::error(
                    Range::from_node(node),
                    "TC003",
                    DiagnosticCategory::Type,
                    format!(
                        "Incompatible return type: expected '{}', got '{}'",
                        expected_return, actual_return
                    ),
                ));
            }
        }
    }

    /// Check function call
    fn check_call(&mut self, node: &Node) {
        let func = match node.child_by_field_name("function") {
            Some(f) => f,
            None => return,
        };

        let func_type = self.inferencer.infer_expr(&func);

        match &func_type {
            Type::Callable { params, .. } => {
                self.check_call_arguments(node, params);
            }
            Type::Unknown | Type::Any => {
                // Can't check unknown/any types
            }
            _ => {
                // Not callable
                self.diagnostics.push(Diagnostic::error(
                    Range::from_node(&func),
                    "TC004",
                    DiagnosticCategory::Type,
                    format!("Type '{}' is not callable", func_type),
                ));
            }
        }
    }

    /// Check call arguments
    fn check_call_arguments(&mut self, call: &Node, params: &[super::ty::Param]) {
        let args = match call.child_by_field_name("arguments") {
            Some(a) => a,
            None => return,
        };

        let mut positional_args: Vec<(Range, Type)> = Vec::new();
        let mut keyword_args: HashMap<String, (Range, Type)> = HashMap::new();
        let mut cursor = args.walk();

        for child in args.children(&mut cursor) {
            match child.kind() {
                "(" | ")" | "," => continue,
                "keyword_argument" => {
                    // Parse keyword argument: name=value
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = self.node_text(&name_node).to_string();
                        if let Some(value_node) = child.child_by_field_name("value") {
                            let value_type = self.inferencer.infer_expr(&value_node);
                            keyword_args.insert(name, (Range::from_node(&child), value_type));
                        }
                    }
                }
                _ => {
                    positional_args.push((
                        Range::from_node(&child),
                        self.inferencer.infer_expr(&child),
                    ));
                }
            }
        }

        // Build param lookup by name
        let param_by_name: HashMap<&str, &super::ty::Param> =
            params.iter().map(|p| (p.name.as_str(), p)).collect();

        // Check for too many positional arguments
        let max_positional = params
            .iter()
            .filter(|p| !matches!(p.kind, ParamKind::VarPositional | ParamKind::VarKeyword))
            .count();

        if positional_args.len() > max_positional
            && !params.iter().any(|p| matches!(p.kind, ParamKind::VarPositional))
        {
            self.diagnostics.push(Diagnostic::error(
                Range::from_node(&args),
                "TC006",
                DiagnosticCategory::Type,
                format!(
                    "Too many arguments: expected at most {}, got {}",
                    max_positional,
                    positional_args.len()
                ),
            ));
        }

        // Check positional argument types
        for (i, (range, arg_ty)) in positional_args.iter().enumerate() {
            if let Some(param) = params.get(i) {
                if !param.ty.is_unknown() && !param.ty.is_any() {
                    if !self.is_assignable(&param.ty, arg_ty) {
                        self.diagnostics.push(Diagnostic::error(
                            range.clone(),
                            "TC005",
                            DiagnosticCategory::Type,
                            format!(
                                "Argument '{}' type mismatch: expected '{}', got '{}'",
                                param.name, param.ty, arg_ty
                            ),
                        ));
                    }
                }
            }
        }

        // Check keyword argument types
        for (name, (range, arg_ty)) in &keyword_args {
            if let Some(param) = param_by_name.get(name.as_str()) {
                if !param.ty.is_unknown() && !param.ty.is_any() {
                    if !self.is_assignable(&param.ty, arg_ty) {
                        self.diagnostics.push(Diagnostic::error(
                            range.clone(),
                            "TC005",
                            DiagnosticCategory::Type,
                            format!(
                                "Argument '{}' type mismatch: expected '{}', got '{}'",
                                name, param.ty, arg_ty
                            ),
                        ));
                    }
                }
            } else if !params.iter().any(|p| matches!(p.kind, ParamKind::VarKeyword)) {
                // Unknown keyword argument
                self.diagnostics.push(Diagnostic::error(
                    range.clone(),
                    "TC008",
                    DiagnosticCategory::Type,
                    format!("Unknown keyword argument: '{}'", name),
                ));
            }
        }

        // Check for missing required arguments
        let provided_count = positional_args.len();
        for (i, param) in params.iter().enumerate() {
            if !param.has_default
                && !matches!(param.kind, ParamKind::VarPositional | ParamKind::VarKeyword)
            {
                let provided_positionally = i < provided_count;
                let provided_by_keyword = keyword_args.contains_key(&param.name);

                if !provided_positionally && !provided_by_keyword {
                    self.diagnostics.push(Diagnostic::error(
                        Range::from_node(&args),
                        "TC007",
                        DiagnosticCategory::Type,
                        format!("Missing required argument: '{}'", param.name),
                    ));
                }
            }
        }
    }

    /// Check attribute access
    fn check_attribute(&mut self, node: &Node) {
        let object = match node.child_by_field_name("object") {
            Some(o) => o,
            None => return,
        };

        let object_type = self.inferencer.infer_expr(&object);

        // Check for None attribute access
        if object_type.contains_none() {
            let attr = node
                .child_by_field_name("attribute")
                .map(|a| self.node_text(&a))
                .unwrap_or("?");

            self.diagnostics.push(Diagnostic::warning(
                Range::from_node(node),
                "TC009",
                DiagnosticCategory::Type,
                format!(
                    "Accessing '{}' on potentially None value (type: {})",
                    attr, object_type
                ),
            ));
        }
    }

    /// Check if source type is assignable to target type
    fn is_assignable(&self, target: &Type, source: &Type) -> bool {
        // Any accepts anything
        if target.is_any() || source.is_any() {
            return true;
        }

        // Unknown is compatible with anything (not yet inferred)
        if target.is_unknown() || source.is_unknown() {
            return true;
        }

        // Error type is compatible with anything (avoid cascading errors)
        if target.is_error() || source.is_error() {
            return true;
        }

        // Same type
        if target == source {
            return true;
        }

        match (target, source) {
            // Optional accepts None
            (Type::Optional(_), Type::None) => true,
            (Type::Optional(inner), other) => self.is_assignable(inner, other),

            // Union accepts any member
            (Type::Union(types), src) => types.iter().any(|t| self.is_assignable(t, src)),
            (target, Type::Union(types)) => types.iter().all(|t| self.is_assignable(target, t)),

            // Float accepts Int
            (Type::Float, Type::Int) => true,

            // List covariance
            (Type::List(a), Type::List(b)) => self.is_assignable(a, b),

            // Dict invariance (simplified - should be covariant in value)
            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                self.is_assignable(k1, k2) && self.is_assignable(v1, v2)
            }

            // Tuple structural compatibility
            (Type::Tuple(a), Type::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(t1, t2)| self.is_assignable(t1, t2))
            }

            // Class subtyping (simplified)
            (
                Type::Instance { name: n1, .. },
                Type::Instance { name: n2, .. },
            ) => n1 == n2, // TODO: check inheritance

            _ => false,
        }
    }

    /// Get text of a node
    fn node_text(&self, node: &Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Get collected diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::MultiParser;

    fn check_code(code: &str) -> Vec<Diagnostic> {
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut checker = TypeChecker::new(code);
        checker.check_file(&parsed)
    }

    #[test]
    fn test_type_mismatch() {
        let diagnostics = check_code(
            r#"
x: int = "hello"
"#,
        );

        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "TC001" && d.message.contains("Type mismatch")));
    }

    #[test]
    fn test_compatible_assignment() {
        let diagnostics = check_code(
            r#"
x: int = 42
y: float = 3.14
z: float = 42  # int is assignable to float
"#,
        );

        // Should not have type mismatch errors (may have TC002 for missing return types)
        assert!(!diagnostics.iter().any(|d| d.code == "TC001"));
    }

    #[test]
    fn test_no_type_error_for_correct_types() {
        // Simple cases that should not produce type errors
        let diagnostics = check_code(
            r#"
x: int = 42
y: str = "hello"
z: float = 3.14
"#,
        );

        // Should not have type mismatch errors
        assert!(!diagnostics.iter().any(|d| d.code == "TC001"));
    }

    #[test]
    fn test_return_type_mismatch() {
        let diagnostics = check_code(
            r#"
def get_number() -> int:
    return "hello"
"#,
        );

        assert!(diagnostics
            .iter()
            .any(|d| d.code == "TC003" && d.message.contains("Incompatible return type")));
    }

    #[test]
    fn test_return_type_correct() {
        let diagnostics = check_code(
            r#"
def get_number() -> int:
    return 42
"#,
        );

        // Should not have return type errors
        assert!(!diagnostics.iter().any(|d| d.code == "TC003"));
    }

    #[test]
    fn test_function_missing_return() {
        let diagnostics = check_code(
            r#"
def get_number() -> int:
    x = 42
"#,
        );

        // Should warn about missing return
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "TC003" && d.message.contains("may not return")));
    }

    #[test]
    fn test_class_method_return_type() {
        let diagnostics = check_code(
            r#"
class Calculator:
    def add(self, x: int, y: int) -> int:
        return x + y
"#,
        );

        // Should not have return type errors - add returns int
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == "TC003" && d.message.contains("Incompatible return type")));
    }

    #[test]
    fn test_class_method_wrong_return() {
        let diagnostics = check_code(
            r#"
class Greeter:
    def greet(self) -> str:
        return 42
"#,
        );

        // Should have return type error - returns int instead of str
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "TC003" && d.message.contains("Incompatible return type")));
    }

    #[test]
    fn test_class_type_checking() {
        let diagnostics = check_code(
            r#"
class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y

p = Point(1, 2)
"#,
        );

        // Basic class definition should not have errors
        // (might have TC002 for missing __init__ return type hint but that's fine)
        assert!(!diagnostics.iter().any(|d| d.code == "TC001"));
    }
}
