//! Type checking - verifies type compatibility and generates diagnostics

use std::collections::HashMap;
use tree_sitter::Node;

use super::annotation::parse_type_annotation;
use super::infer::TypeInferencer;
use super::narrow::{self, TypeNarrower};
use super::ty::{LiteralValue, ParamKind, Type};
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
    /// Type narrower for control flow analysis
    narrower: TypeNarrower,
}

impl<'a> TypeChecker<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inferencer: TypeInferencer::new(source),
            diagnostics: Vec::new(),
            source,
            function_stack: Vec::new(),
            narrower: TypeNarrower::new(),
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
            "if_statement" => {
                self.check_if_statement(node);
                return; // check_if_statement handles its own recursion
            }
            "while_statement" => {
                self.check_while_statement(node);
                return; // handles its own recursion
            }
            "for_statement" => {
                self.check_for_statement(node);
                return; // handles its own recursion
            }
            "try_statement" => {
                self.check_try_statement(node);
                return; // handles its own recursion
            }
            "import_statement" | "import_from_statement" => {
                self.inferencer.analyze_import(node);
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
            .map(|rt| parse_type_annotation(self.source, &rt))
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
                let expected = parse_type_annotation(self.source, &type_node);

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

    /// Check if statement with type narrowing
    fn check_if_statement(&mut self, node: &Node) {
        // Get the condition
        let condition = match node.child_by_field_name("condition") {
            Some(c) => c,
            None => return,
        };

        // Parse the condition into a narrowing condition
        let narrowing_cond = narrow::parse_condition(self.source, &condition);

        // Collect original types from environment for narrowing
        let original_types = self.inferencer.get_env_types();

        // Handle the consequence (if branch)
        if let Some(consequence) = node.child_by_field_name("consequence") {
            self.narrower.push_scope();
            self.narrower.apply_condition(&narrowing_cond, &original_types);

            // Set narrowed types as overrides in the inferencer
            let narrowed_types = self.collect_narrowed_types();
            self.inferencer.set_type_overrides(narrowed_types);

            // Check the body
            let mut cursor = consequence.walk();
            for child in consequence.children(&mut cursor) {
                self.check_node(&child);
            }

            self.inferencer.clear_type_overrides();
            self.narrower.pop_scope();
        }

        // Handle else/elif branches
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "else_clause" => {
                    self.narrower.push_scope();
                    // Apply negated condition for else branch
                    let negated = narrow::negate_condition(&narrowing_cond);
                    self.narrower.apply_condition(&negated, &original_types);

                    // Set narrowed types as overrides in the inferencer
                    let narrowed_types = self.collect_narrowed_types();
                    self.inferencer.set_type_overrides(narrowed_types);

                    // Check the else body
                    if let Some(body) = child.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for body_child in body.children(&mut body_cursor) {
                            self.check_node(&body_child);
                        }
                    }

                    self.inferencer.clear_type_overrides();
                    self.narrower.pop_scope();
                }
                "elif_clause" => {
                    // Recursively handle elif as another if
                    self.check_if_statement(&child);
                }
                _ => {}
            }
        }
    }

    /// Collect all narrowed types from the narrower
    fn collect_narrowed_types(&self) -> HashMap<String, Type> {
        let mut types = HashMap::new();
        // Get all narrowed types from the narrower scopes
        for name in self.inferencer.get_env_types().keys() {
            if let Some(ty) = self.narrower.get_narrowed(name) {
                types.insert(name.clone(), ty.clone());
            }
        }
        types
    }

    /// Check while statement with type narrowing
    fn check_while_statement(&mut self, node: &Node) {
        // Get the condition
        let condition = match node.child_by_field_name("condition") {
            Some(c) => c,
            None => return,
        };

        // Parse the condition for narrowing
        let narrowing_cond = narrow::parse_condition(self.source, &condition);
        let original_types = self.inferencer.get_env_types();

        // Handle the body (condition is true inside loop)
        if let Some(body) = node.child_by_field_name("body") {
            self.narrower.push_scope();
            self.narrower.apply_condition(&narrowing_cond, &original_types);

            let narrowed_types = self.collect_narrowed_types();
            self.inferencer.set_type_overrides(narrowed_types);

            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                self.check_node(&child);
            }

            self.inferencer.clear_type_overrides();
            self.narrower.pop_scope();
        }

        // Handle else clause (executed when condition becomes false)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "else_clause" {
                if let Some(body) = child.child_by_field_name("body") {
                    let mut body_cursor = body.walk();
                    for body_child in body.children(&mut body_cursor) {
                        self.check_node(&body_child);
                    }
                }
            }
        }
    }

    /// Check for statement - binds loop variable
    fn check_for_statement(&mut self, node: &Node) {
        // Get the iterable and infer its element type
        let iterable = node.child_by_field_name("right");
        let element_type = if let Some(iter_node) = iterable {
            let iter_type = self.inferencer.infer_expr(&iter_node);
            match iter_type {
                Type::List(elem) => (*elem).clone(),
                Type::Set(elem) => (*elem).clone(),
                Type::Dict(key, _) => (*key).clone(), // iterating dict gives keys
                Type::Tuple(elems) => {
                    if elems.is_empty() {
                        Type::Unknown
                    } else {
                        Type::union(elems)
                    }
                }
                Type::Str => Type::Str, // iterating str gives chars (single char strings)
                _ => Type::Unknown,
            }
        } else {
            Type::Unknown
        };

        // Bind the loop variable
        if let Some(target) = node.child_by_field_name("left") {
            self.inferencer.bind_assignment(&target, element_type);
        }

        // Check the body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                self.check_node(&child);
            }
        }

        // Handle else clause
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "else_clause" {
                if let Some(body) = child.child_by_field_name("body") {
                    let mut body_cursor = body.walk();
                    for body_child in body.children(&mut body_cursor) {
                        self.check_node(&body_child);
                    }
                }
            }
        }
    }

    /// Check try statement
    fn check_try_statement(&mut self, node: &Node) {
        // Check the try body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                self.check_node(&child);
            }
        }

        // Check exception handlers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "except_clause" => {
                    // Bind exception variable if present
                    if let Some(name) = child.child_by_field_name("name") {
                        // Exception type - default to BaseException if not specified
                        let exc_type = if let Some(type_node) = child.child_by_field_name("type") {
                            parse_type_annotation(self.source, &type_node)
                        } else {
                            Type::Instance {
                                name: "BaseException".to_string(),
                                module: Some("builtins".to_string()),
                                type_args: vec![],
                            }
                        };
                        self.inferencer.bind_assignment(&name, exc_type);
                    }

                    // Check except body
                    if let Some(body) = child.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for body_child in body.children(&mut body_cursor) {
                            self.check_node(&body_child);
                        }
                    }
                }
                "finally_clause" => {
                    if let Some(body) = child.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for body_child in body.children(&mut body_cursor) {
                            self.check_node(&body_child);
                        }
                    }
                }
                "else_clause" => {
                    if let Some(body) = child.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for body_child in body.children(&mut body_cursor) {
                            self.check_node(&body_child);
                        }
                    }
                }
                _ => {}
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

            // Class subtyping with inheritance
            (
                Type::Instance { name: n1, .. },
                Type::Instance { name: n2, .. },
            ) => n1 == n2 || self.inferencer.is_subclass(n2, n1),

            // Protocol structural subtyping
            // A type is assignable to a Protocol if it has all required members
            (Type::Protocol { members, .. }, Type::Instance { name, .. }) => {
                // Check that the source type has all required protocol members
                members.iter().all(|(member_name, member_ty)| {
                    if let Some(attr_ty) = self.inferencer.get_attribute_recursive(name, member_name) {
                        self.is_assignable(member_ty, &attr_ty)
                    } else {
                        false
                    }
                })
            }

            // Callable to Protocol with __call__
            (Type::Protocol { members, .. }, Type::Callable { .. }) => {
                // A Callable can match a Protocol if the Protocol only requires __call__
                members.len() == 1 && members.iter().any(|(name, _)| name == "__call__")
            }

            // Literal types are assignable to their base types
            (Type::Int, Type::Literal(LiteralValue::Int(_))) => true,
            (Type::Float, Type::Literal(LiteralValue::Float(_))) => true,
            (Type::Float, Type::Literal(LiteralValue::Int(_))) => true, // int literal -> float
            (Type::Str, Type::Literal(LiteralValue::Str(_))) => true,
            (Type::Bool, Type::Literal(LiteralValue::Bool(_))) => true,
            (Type::None, Type::Literal(LiteralValue::None)) => true,

            // Literal to Literal (same value)
            (Type::Literal(a), Type::Literal(b)) => a == b,

            // TypedDict structural subtyping
            // A dict is assignable to TypedDict if it has all required keys
            (Type::TypedDict { fields, .. }, Type::Dict(key_ty, val_ty)) => {
                // For a generic dict to be assignable to TypedDict,
                // key must be str and value must be compatible with all field types
                if !self.is_assignable(&Type::Str, key_ty) {
                    return false;
                }
                fields.iter().all(|(_, field_ty, _)| {
                    self.is_assignable(field_ty, val_ty)
                })
            }

            // TypedDict to TypedDict: all required fields must match
            (
                Type::TypedDict { fields: target_fields, .. },
                Type::TypedDict { fields: source_fields, .. },
            ) => {
                // Check all required target fields exist in source with compatible types
                target_fields.iter().all(|(name, ty, required)| {
                    if let Some((_, src_ty, _)) = source_fields.iter().find(|(n, _, _)| n == name) {
                        self.is_assignable(ty, src_ty)
                    } else {
                        !required // Missing field is ok only if not required
                    }
                })
            }

            // TypedDict is a subtype of Dict[str, Union[field_types]]
            (Type::Dict(key_ty, _), Type::TypedDict { .. }) => {
                self.is_assignable(key_ty, &Type::Str)
            }

            // PEP 591: Final[T] is assignable to T
            (target, Type::Final(inner)) => self.is_assignable(target, inner),
            (Type::Final(inner), source) => self.is_assignable(inner, source),

            // PEP 593: Annotated[T, ...] is assignable to T
            (target, Type::Annotated { inner, .. }) => self.is_assignable(target, inner),
            (Type::Annotated { inner, .. }, source) => self.is_assignable(inner, source),

            // PEP 675: LiteralString is assignable to str
            (Type::Str, Type::LiteralString) => true,
            // String literals are assignable to LiteralString
            (Type::LiteralString, Type::Literal(LiteralValue::Str(_))) => true,
            // LiteralString is assignable to LiteralString
            (Type::LiteralString, Type::LiteralString) => true,

            // PEP 673: Self type - compare class names
            (Type::SelfType { class_name: Some(n1) }, Type::SelfType { class_name: Some(n2) }) => n1 == n2,
            (Type::SelfType { class_name: Some(name) }, Type::Instance { name: inst_name, .. }) => name == inst_name,
            (Type::Instance { name: inst_name, .. }, Type::SelfType { class_name: Some(name) }) => name == inst_name,
            // Unresolved Self is compatible with any Self
            (Type::SelfType { class_name: None }, Type::SelfType { .. }) => true,
            (Type::SelfType { .. }, Type::SelfType { class_name: None }) => true,

            // Overloaded functions - source can be any signature
            (Type::Callable { .. }, Type::Overloaded { signatures }) => {
                signatures.iter().any(|sig| self.is_assignable(target, sig))
            }
            // Overloaded to Callable - at least one signature must match
            (Type::Overloaded { signatures }, source) => {
                signatures.iter().any(|sig| self.is_assignable(sig, source))
            }

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
#[path = "check_tests.rs"]
mod tests;
