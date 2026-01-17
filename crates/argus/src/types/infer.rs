//! Type inference engine for Python

use std::collections::HashMap;
use tree_sitter::Node;

use super::imports::{parse_import, ImportResolver};
use super::stubs::StubLoader;
use super::ty::{Param, ParamKind, Type, TypeVarId};

/// Information about a class definition
#[derive(Debug, Clone, Default)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Base classes
    pub bases: Vec<String>,
    /// Instance attributes (name -> type)
    pub attributes: HashMap<String, Type>,
    /// Methods (name -> callable type)
    pub methods: HashMap<String, Type>,
    /// Class variables (name -> type)
    pub class_vars: HashMap<String, Type>,
}

impl ClassInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bases: Vec::new(),
            attributes: HashMap::new(),
            methods: HashMap::new(),
            class_vars: HashMap::new(),
        }
    }

    /// Get attribute type (checks instance attrs, then methods, then class vars)
    pub fn get_attribute(&self, name: &str) -> Option<&Type> {
        self.attributes
            .get(name)
            .or_else(|| self.methods.get(name))
            .or_else(|| self.class_vars.get(name))
    }
}

/// Type environment mapping names to types
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    /// Stack of scopes, innermost last
    scopes: Vec<HashMap<String, Type>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the innermost scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Bind a name to a type in the current scope
    pub fn bind(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// Look up a name, searching from innermost to outermost scope
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }
}

/// Type inferencer for Python code
pub struct TypeInferencer<'a> {
    /// Source code
    source: &'a str,
    /// Type environment
    env: TypeEnv,
    /// Class registry (class name -> class info)
    classes: HashMap<String, ClassInfo>,
    /// TypeVar registry (name -> type)
    type_vars: HashMap<String, Type>,
    /// Counter for generating fresh type variables
    next_type_var: usize,
    /// Type overrides from narrowing (checked before env)
    type_overrides: Option<HashMap<String, Type>>,
    /// Stub loader for builtin/typing/collections stubs
    #[allow(dead_code)]
    stubs: StubLoader,
    /// Import resolver for module resolution
    resolver: ImportResolver,
}

impl<'a> TypeInferencer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut env = TypeEnv::new();
        // Add builtins
        Self::add_builtins(&mut env);

        // Initialize stubs and resolver
        let mut stubs = StubLoader::new();
        stubs.load_builtins();

        let mut resolver = ImportResolver::new();
        for (path, info) in stubs.modules() {
            resolver.register_module(path, info.clone());
        }

        Self {
            source,
            env,
            classes: HashMap::new(),
            type_vars: HashMap::new(),
            next_type_var: 0,
            type_overrides: None,
            stubs,
            resolver,
        }
    }

    /// Set type overrides from narrowing (used for control flow analysis)
    pub fn set_type_overrides(&mut self, overrides: HashMap<String, Type>) {
        self.type_overrides = Some(overrides);
    }

    /// Clear type overrides
    pub fn clear_type_overrides(&mut self) {
        self.type_overrides = None;
    }

    /// Register a TypeVar definition
    pub fn register_type_var(&mut self, name: &str, bound: Option<Type>, constraints: Vec<Type>) {
        let id = TypeVarId(self.next_type_var);
        self.next_type_var += 1;
        let tv = Type::TypeVar {
            id,
            name: name.to_string(),
            bound: bound.map(Box::new),
            constraints,
        };
        self.type_vars.insert(name.to_string(), tv.clone());
        self.env.bind(name.to_string(), tv);
    }

    /// Look up a TypeVar by name
    pub fn get_type_var(&self, name: &str) -> Option<&Type> {
        self.type_vars.get(name)
    }

    /// Instantiate a generic type with concrete type arguments
    pub fn instantiate_generic(&self, generic_type: &Type, type_args: &[Type]) -> Type {
        let type_var_ids = generic_type.type_vars();
        if type_var_ids.len() != type_args.len() {
            return Type::Error; // Wrong number of type args
        }

        let substitutions: HashMap<TypeVarId, Type> = type_var_ids
            .into_iter()
            .zip(type_args.iter().cloned())
            .collect();

        generic_type.substitute(&substitutions)
    }

    /// Get class info by name
    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes.get(name)
    }

    /// Get attribute type with inheritance support
    /// Walks up the inheritance chain to find the attribute
    pub fn get_attribute_recursive(&self, class_name: &str, attr_name: &str) -> Option<Type> {
        if let Some(class_info) = self.classes.get(class_name) {
            // First check the current class
            if let Some(ty) = class_info.get_attribute(attr_name) {
                return Some(ty.clone());
            }
            // Then check base classes (in order)
            for base_name in &class_info.bases {
                if let Some(ty) = self.get_attribute_recursive(base_name, attr_name) {
                    return Some(ty);
                }
            }
        }
        None
    }

    /// Get class-level attribute (class vars, methods) with inheritance support
    fn get_class_attribute_recursive(&self, class_name: &str, attr_name: &str) -> Option<Type> {
        if let Some(class_info) = self.classes.get(class_name) {
            // First check class variables
            if let Some(ty) = class_info.class_vars.get(attr_name) {
                return Some(ty.clone());
            }
            // Then check methods
            if let Some(ty) = class_info.methods.get(attr_name) {
                return Some(ty.clone());
            }
            // Then check base classes
            for base_name in &class_info.bases {
                if let Some(ty) = self.get_class_attribute_recursive(base_name, attr_name) {
                    return Some(ty);
                }
            }
        }
        None
    }

    /// Check if a class is a subclass of another (including self)
    pub fn is_subclass(&self, child: &str, parent: &str) -> bool {
        if child == parent {
            return true;
        }
        if let Some(class_info) = self.classes.get(child) {
            for base_name in &class_info.bases {
                if self.is_subclass(base_name, parent) {
                    return true;
                }
            }
        }
        false
    }

    /// Add builtin types to the environment
    fn add_builtins(env: &mut TypeEnv) {
        // Builtin functions
        env.bind(
            "len".to_string(),
            Type::callable(vec![Type::Any], Type::Int),
        );
        env.bind(
            "str".to_string(),
            Type::callable(vec![Type::Any], Type::Str),
        );
        env.bind(
            "int".to_string(),
            Type::callable(vec![Type::Any], Type::Int),
        );
        env.bind(
            "float".to_string(),
            Type::callable(vec![Type::Any], Type::Float),
        );
        env.bind(
            "bool".to_string(),
            Type::callable(vec![Type::Any], Type::Bool),
        );
        env.bind(
            "list".to_string(),
            Type::callable(vec![], Type::list(Type::Unknown)),
        );
        env.bind(
            "dict".to_string(),
            Type::callable(vec![], Type::dict(Type::Unknown, Type::Unknown)),
        );
        env.bind(
            "set".to_string(),
            Type::callable(vec![], Type::Set(Box::new(Type::Unknown))),
        );
        env.bind(
            "print".to_string(),
            Type::Callable {
                params: vec![Param {
                    name: "values".to_string(),
                    ty: Type::Any,
                    has_default: false,
                    kind: ParamKind::VarPositional,
                }],
                ret: Box::new(Type::None),
            },
        );
        env.bind(
            "range".to_string(),
            Type::callable(vec![Type::Int], Type::list(Type::Int)),
        );
        env.bind(
            "enumerate".to_string(),
            Type::callable(
                vec![Type::list(Type::Unknown)],
                Type::list(Type::Tuple(vec![Type::Int, Type::Unknown])),
            ),
        );
        env.bind(
            "zip".to_string(),
            Type::callable(
                vec![Type::list(Type::Unknown), Type::list(Type::Unknown)],
                Type::list(Type::Tuple(vec![Type::Unknown, Type::Unknown])),
            ),
        );
        env.bind(
            "isinstance".to_string(),
            Type::callable(vec![Type::Any, Type::Any], Type::Bool),
        );
        env.bind(
            "hasattr".to_string(),
            Type::callable(vec![Type::Any, Type::Str], Type::Bool),
        );
        env.bind(
            "getattr".to_string(),
            Type::callable(vec![Type::Any, Type::Str], Type::Any),
        );
    }

    /// Generate a fresh type variable
    #[allow(dead_code)]
    fn fresh_type_var(&mut self, name: &str) -> Type {
        let id = TypeVarId(self.next_type_var);
        self.next_type_var += 1;
        Type::TypeVar {
            id,
            name: name.to_string(),
            bound: None,
            constraints: vec![],
        }
    }

    /// Get the text of a node
    fn node_text(&self, node: &Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Infer the type of an expression
    pub fn infer_expr(&mut self, node: &Node) -> Type {
        match node.kind() {
            // Literals
            "integer" => Type::Int,
            "float" => Type::Float,
            "string" => Type::Str,
            "true" | "false" => Type::Bool,
            "none" => Type::None,
            "ellipsis" => Type::Any, // ... is typically used as placeholder

            // Identifier lookup
            "identifier" => {
                let name = self.node_text(node);
                // Check type overrides first (from narrowing)
                if let Some(ref overrides) = self.type_overrides {
                    if let Some(ty) = overrides.get(name) {
                        return ty.clone();
                    }
                }
                self.env.lookup(name).cloned().unwrap_or(Type::Unknown)
            }

            // Binary operators
            "binary_operator" => self.infer_binary_op(node),

            // Unary operators
            "unary_operator" => self.infer_unary_op(node),

            // Comparison operators
            "comparison_operator" => Type::Bool,
            "boolean_operator" => Type::Bool,
            "not_operator" => Type::Bool,

            // Container literals
            "list" => self.infer_list_literal(node),
            "dictionary" => self.infer_dict_literal(node),
            "set" => self.infer_set_literal(node),
            "tuple" => self.infer_tuple_literal(node),

            // List comprehension
            "list_comprehension" => self.infer_list_comprehension(node),

            // Call expression
            "call" => self.infer_call(node),

            // Attribute access
            "attribute" => self.infer_attribute(node),

            // Subscript
            "subscript" => self.infer_subscript(node),

            // Conditional expression (ternary)
            "conditional_expression" => self.infer_conditional(node),

            // Lambda
            "lambda" => self.infer_lambda(node),

            // Await expression
            "await" => {
                if let Some(arg) = node.child(1) {
                    // Unwrap Awaitable[T] -> T
                    let inner = self.infer_expr(&arg);
                    // For now, just return the inner type
                    inner
                } else {
                    Type::Unknown
                }
            }

            // Parenthesized expression
            "parenthesized_expression" => {
                if let Some(inner) = node.child(1) {
                    self.infer_expr(&inner)
                } else {
                    Type::Unknown
                }
            }

            _ => Type::Unknown,
        }
    }

    /// Infer binary operator result type
    fn infer_binary_op(&mut self, node: &Node) -> Type {
        let left = node.child_by_field_name("left");
        let right = node.child_by_field_name("right");
        let op = node.child_by_field_name("operator");

        let (left_ty, right_ty) = match (left, right) {
            (Some(l), Some(r)) => (self.infer_expr(&l), self.infer_expr(&r)),
            _ => return Type::Unknown,
        };

        let op_text = op.map(|o| self.node_text(&o)).unwrap_or("");

        match op_text {
            // Arithmetic operators
            "+" => match (&left_ty, &right_ty) {
                (Type::Str, Type::Str) => Type::Str,
                (Type::List(a), Type::List(b)) if a == b => Type::list((**a).clone()),
                (Type::Int, Type::Int) => Type::Int,
                (Type::Float, _) | (_, Type::Float) => Type::Float,
                _ => Type::Unknown,
            },
            "-" | "*" => match (&left_ty, &right_ty) {
                (Type::Str, Type::Int) if op_text == "*" => Type::Str, // "a" * 3
                (Type::Int, Type::Str) if op_text == "*" => Type::Str, // 3 * "a"
                (Type::Int, Type::Int) => Type::Int,
                (Type::Float, _) | (_, Type::Float) => Type::Float,
                _ => Type::Unknown,
            },
            "/" => Type::Float, // Python 3 true division
            "//" => Type::Int,  // Floor division
            "%" => match (&left_ty, &right_ty) {
                (Type::Str, _) => Type::Str, // String formatting
                (Type::Int, Type::Int) => Type::Int,
                _ => Type::Unknown,
            },
            "**" => match (&left_ty, &right_ty) {
                (Type::Int, Type::Int) => Type::Int,
                _ => Type::Float,
            },

            // Bitwise operators
            "&" | "|" | "^" | "<<" | ">>" => Type::Int,

            // Membership/identity
            "in" | "not in" | "is" | "is not" => Type::Bool,

            _ => Type::Unknown,
        }
    }

    /// Infer unary operator result type
    fn infer_unary_op(&mut self, node: &Node) -> Type {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        let op = children.first().map(|n| self.node_text(n)).unwrap_or("");
        let operand = children.get(1);

        match op {
            "-" | "+" => {
                if let Some(arg) = operand {
                    let arg_ty = self.infer_expr(arg);
                    match arg_ty {
                        Type::Int => Type::Int,
                        Type::Float => Type::Float,
                        _ => Type::Unknown,
                    }
                } else {
                    Type::Unknown
                }
            }
            "~" => Type::Int, // Bitwise NOT
            "not" => Type::Bool,
            _ => Type::Unknown,
        }
    }

    /// Infer list literal type
    fn infer_list_literal(&mut self, node: &Node) -> Type {
        let mut cursor = node.walk();
        let mut element_types = Vec::new();

        for child in node.children(&mut cursor) {
            if child.kind() != "[" && child.kind() != "]" && child.kind() != "," {
                element_types.push(self.infer_expr(&child));
            }
        }

        if element_types.is_empty() {
            Type::list(Type::Unknown)
        } else {
            // Use the first element's type (simplified)
            // A full implementation would compute LUB (least upper bound)
            Type::list(element_types[0].clone())
        }
    }

    /// Infer dict literal type
    fn infer_dict_literal(&mut self, node: &Node) -> Type {
        let mut cursor = node.walk();
        let mut key_type = Type::Unknown;
        let mut value_type = Type::Unknown;

        for child in node.children(&mut cursor) {
            if child.kind() == "pair" {
                if let Some(key) = child.child_by_field_name("key") {
                    key_type = self.infer_expr(&key);
                }
                if let Some(value) = child.child_by_field_name("value") {
                    value_type = self.infer_expr(&value);
                }
                break; // Just use first pair for type
            }
        }

        Type::dict(key_type, value_type)
    }

    /// Infer set literal type
    fn infer_set_literal(&mut self, node: &Node) -> Type {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() != "{" && child.kind() != "}" && child.kind() != "," {
                let elem_ty = self.infer_expr(&child);
                return Type::Set(Box::new(elem_ty));
            }
        }

        Type::Set(Box::new(Type::Unknown))
    }

    /// Infer tuple literal type
    fn infer_tuple_literal(&mut self, node: &Node) -> Type {
        let mut cursor = node.walk();
        let mut element_types = Vec::new();

        for child in node.children(&mut cursor) {
            if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                element_types.push(self.infer_expr(&child));
            }
        }

        Type::Tuple(element_types)
    }

    /// Infer list comprehension type
    fn infer_list_comprehension(&mut self, node: &Node) -> Type {
        // [expr for x in iter] -> list[type of expr]
        if let Some(body) = node.child(1) {
            // This is simplified - should handle the iteration binding
            let elem_ty = self.infer_expr(&body);
            Type::list(elem_ty)
        } else {
            Type::list(Type::Unknown)
        }
    }

    /// Infer function call result type with generic type inference
    fn infer_call(&mut self, node: &Node) -> Type {
        let func = match node.child_by_field_name("function") {
            Some(f) => f,
            None => return Type::Unknown,
        };

        let func_ty = self.infer_expr(&func);

        match &func_ty {
            Type::Callable { params, ret } => {
                // Collect argument types
                let arg_types = self.collect_call_arguments(node);

                // Check if return type has type variables
                let type_vars = ret.type_vars();
                if type_vars.is_empty() {
                    // No generics, just return the declared return type
                    return (**ret).clone();
                }

                // Unify parameter types with argument types to infer TypeVars
                let mut substitutions = HashMap::new();
                for (param, arg_ty) in params.iter().zip(arg_types.iter()) {
                    param.ty.unify(arg_ty, &mut substitutions);
                }

                // Apply substitutions to the return type
                ret.substitute(&substitutions)
            }
            Type::Instance { name, .. } => {
                // Constructor call returns instance
                Type::Instance {
                    name: name.clone(),
                    module: None,
                    type_args: vec![],
                }
            }
            Type::ClassType { name, .. } => {
                // type[T]() returns T
                Type::Instance {
                    name: name.clone(),
                    module: None,
                    type_args: vec![],
                }
            }
            _ => Type::Unknown,
        }
    }

    /// Collect argument types from a call expression
    fn collect_call_arguments(&mut self, node: &Node) -> Vec<Type> {
        let mut arg_types = Vec::new();

        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                match child.kind() {
                    "(" | ")" | "," => continue,
                    "keyword_argument" => {
                        // Skip keyword for now, just get the value
                        if let Some(value) = child.child_by_field_name("value") {
                            arg_types.push(self.infer_expr(&value));
                        }
                    }
                    _ => {
                        arg_types.push(self.infer_expr(&child));
                    }
                }
            }
        }

        arg_types
    }

    /// Infer attribute access type
    fn infer_attribute(&mut self, node: &Node) -> Type {
        let object = match node.child_by_field_name("object") {
            Some(o) => o,
            None => return Type::Unknown,
        };
        let attribute = match node.child_by_field_name("attribute") {
            Some(a) => a,
            None => return Type::Unknown,
        };

        let object_type = self.infer_expr(&object);
        let attr_name = self.node_text(&attribute);

        match &object_type {
            Type::Instance { name, .. } => {
                // Look up attribute with inheritance support
                self.get_attribute_recursive(name, attr_name)
                    .unwrap_or(Type::Unknown)
            }
            Type::ClassType { name, .. } => {
                // Class attribute access (static methods, class vars) with inheritance
                self.get_class_attribute_recursive(name, attr_name)
                    .unwrap_or(Type::Unknown)
            }
            Type::Optional(inner) => {
                // For Optional[T].attr, return the attribute type from T with inheritance
                if let Type::Instance { name, .. } = inner.as_ref() {
                    self.get_attribute_recursive(name, attr_name)
                        .unwrap_or(Type::Unknown)
                } else {
                    Type::Unknown
                }
            }
            _ => Type::Unknown,
        }
    }

    /// Infer subscript type
    fn infer_subscript(&mut self, node: &Node) -> Type {
        let value = match node.child_by_field_name("value") {
            Some(v) => v,
            None => return Type::Unknown,
        };

        let value_ty = self.infer_expr(&value);

        match &value_ty {
            Type::List(elem) => (**elem).clone(),
            Type::Dict(_, val) => (**val).clone(),
            Type::Tuple(elems) => {
                // For tuple, try to get specific index
                if let Some(subscript) = node.child_by_field_name("subscript") {
                    if subscript.kind() == "integer" {
                        if let Ok(idx) = self.node_text(&subscript).parse::<usize>() {
                            if let Some(ty) = elems.get(idx) {
                                return ty.clone();
                            }
                        }
                    }
                }
                // Unknown index, return union of all element types
                Type::union(elems.clone())
            }
            Type::Str => Type::Str, // str[n] -> str
            _ => Type::Unknown,
        }
    }

    /// Infer conditional expression type
    fn infer_conditional(&mut self, node: &Node) -> Type {
        // Python: true_val if condition else false_val
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // [true_val, "if", condition, "else", false_val]
        if children.len() >= 5 {
            let true_ty = self.infer_expr(&children[0]);
            let false_ty = self.infer_expr(&children[4]);

            if true_ty == false_ty {
                true_ty
            } else {
                Type::union(vec![true_ty, false_ty])
            }
        } else {
            Type::Unknown
        }
    }

    /// Infer lambda type
    fn infer_lambda(&mut self, node: &Node) -> Type {
        // lambda params: body
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    params.push(Param {
                        name: self.node_text(&child).to_string(),
                        ty: Type::Unknown,
                        has_default: false,
                        kind: ParamKind::Positional,
                    });
                }
            }
        }

        let ret = if let Some(body) = node.child_by_field_name("body") {
            self.infer_expr(&body)
        } else {
            Type::Unknown
        };

        Type::Callable {
            params,
            ret: Box::new(ret),
        }
    }

    /// Analyze a function definition and add it to the environment
    pub fn analyze_function(&mut self, node: &Node) -> Type {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n).to_string())
            .unwrap_or_default();

        let mut params = Vec::new();
        let mut return_type = Type::Unknown;

        // Parse parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            params = self.parse_parameters(&params_node);
        }

        // Parse return type annotation
        if let Some(return_node) = node.child_by_field_name("return_type") {
            return_type = self.parse_type_annotation(&return_node);
        }

        let func_type = Type::Callable {
            params,
            ret: Box::new(return_type),
        };

        self.env.bind(name, func_type.clone());
        func_type
    }

    /// Analyze a class definition and add it to the registry
    pub fn analyze_class(&mut self, node: &Node) -> ClassInfo {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n).to_string())
            .unwrap_or_default();

        let mut class_info = ClassInfo::new(name.clone());

        // Parse base classes
        if let Some(bases) = node.child_by_field_name("superclasses") {
            let mut cursor = bases.walk();
            for child in bases.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "attribute" {
                    class_info.bases.push(self.node_text(&child).to_string());
                }
            }
        }

        // Parse class body
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "function_definition" | "async_function_definition" => {
                        self.parse_class_method(&child, &mut class_info);
                    }
                    "expression_statement" => {
                        self.parse_class_attribute(&child, &mut class_info);
                    }
                    _ => {}
                }
            }
        }

        // Add class type to environment
        let class_type = Type::ClassType {
            name: name.clone(),
            module: None,
        };
        self.env.bind(name.clone(), class_type);

        // Store class info
        self.classes.insert(name, class_info.clone());

        class_info
    }

    /// Parse a method in a class definition
    fn parse_class_method(&mut self, node: &Node, class_info: &mut ClassInfo) {
        let method_name = node
            .child_by_field_name("name")
            .map(|n| self.node_text(&n).to_string())
            .unwrap_or_default();

        let mut params = Vec::new();
        let mut return_type = Type::Unknown;

        // Parse parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            params = self.parse_parameters(&params_node);

            // Check for self parameter and extract attribute assignments
            if let Some(first_param) = params.first() {
                if first_param.name == "self" {
                    // This is an instance method
                    // Parse body for self.attr = ... assignments
                    if method_name == "__init__" {
                        if let Some(body) = node.child_by_field_name("body") {
                            self.parse_init_assignments(&body, class_info);
                        }
                    }
                }
            }
        }

        // Parse return type
        if let Some(return_node) = node.child_by_field_name("return_type") {
            return_type = self.parse_type_annotation(&return_node);
        }

        // Handle special return type for __init__
        if method_name == "__init__" {
            return_type = Type::None;
        }

        let method_type = Type::Callable {
            params,
            ret: Box::new(return_type),
        };

        class_info.methods.insert(method_name, method_type);
    }

    /// Parse attribute assignments in __init__
    fn parse_init_assignments(&mut self, body: &Node, class_info: &mut ClassInfo) {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                if let Some(expr) = child.child(0) {
                    if expr.kind() == "assignment" {
                        self.parse_self_assignment(&expr, class_info);
                    }
                }
            }
        }
    }

    /// Parse self.attr = value assignments
    fn parse_self_assignment(&mut self, node: &Node, class_info: &mut ClassInfo) {
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "attribute" {
                if let Some(obj) = left.child_by_field_name("object") {
                    if self.node_text(&obj) == "self" {
                        if let Some(attr) = left.child_by_field_name("attribute") {
                            let attr_name = self.node_text(&attr).to_string();

                            // Get type from annotation or infer from value
                            let attr_type = if let Some(type_node) = node.child_by_field_name("type") {
                                self.parse_type_annotation(&type_node)
                            } else if let Some(value) = node.child_by_field_name("right") {
                                self.infer_expr(&value)
                            } else {
                                Type::Unknown
                            };

                            class_info.attributes.insert(attr_name, attr_type);
                        }
                    }
                }
            }
        }
    }

    /// Parse class-level attribute (class variable or annotated attribute)
    fn parse_class_attribute(&mut self, node: &Node, class_info: &mut ClassInfo) {
        if let Some(expr) = node.child(0) {
            match expr.kind() {
                "assignment" => {
                    // name = value or name: type = value
                    if let Some(left) = expr.child_by_field_name("left") {
                        if left.kind() == "identifier" {
                            let attr_name = self.node_text(&left).to_string();
                            let attr_type = if let Some(type_node) = expr.child_by_field_name("type") {
                                self.parse_type_annotation(&type_node)
                            } else if let Some(value) = expr.child_by_field_name("right") {
                                self.infer_expr(&value)
                            } else {
                                Type::Unknown
                            };
                            class_info.class_vars.insert(attr_name, attr_type);
                        }
                    }
                }
                // Annotated attribute without assignment: name: type
                _ => {}
            }
        }
    }

    /// Parse function parameters
    fn parse_parameters(&mut self, node: &Node) -> Vec<Param> {
        let mut params = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    params.push(Param {
                        name: self.node_text(&child).to_string(),
                        ty: Type::Unknown,
                        has_default: false,
                        kind: ParamKind::Positional,
                    });
                }
                "typed_parameter" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(&n).to_string())
                        .unwrap_or_default();
                    let ty = child
                        .child_by_field_name("type")
                        .map(|t| self.parse_type_annotation(&t))
                        .unwrap_or(Type::Unknown);
                    params.push(Param {
                        name,
                        ty,
                        has_default: false,
                        kind: ParamKind::Positional,
                    });
                }
                "default_parameter" | "typed_default_parameter" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| self.node_text(&n).to_string())
                        .unwrap_or_default();
                    let ty = child
                        .child_by_field_name("type")
                        .map(|t| self.parse_type_annotation(&t))
                        .unwrap_or(Type::Unknown);
                    params.push(Param {
                        name,
                        ty,
                        has_default: true,
                        kind: ParamKind::Positional,
                    });
                }
                "list_splat_pattern" => {
                    if let Some(name_node) = child.child(1) {
                        params.push(Param {
                            name: self.node_text(&name_node).to_string(),
                            ty: Type::Tuple(vec![Type::Unknown]),
                            has_default: false,
                            kind: ParamKind::VarPositional,
                        });
                    }
                }
                "dictionary_splat_pattern" => {
                    if let Some(name_node) = child.child(1) {
                        params.push(Param {
                            name: self.node_text(&name_node).to_string(),
                            ty: Type::dict(Type::Str, Type::Unknown),
                            has_default: false,
                            kind: ParamKind::VarKeyword,
                        });
                    }
                }
                _ => {}
            }
        }

        params
    }

    /// Parse a type annotation
    pub fn parse_type_annotation(&self, node: &Node) -> Type {
        let text = self.node_text(node);

        match node.kind() {
            "identifier" | "type" => self.parse_simple_type(text),
            "subscript" => self.parse_generic_type(node),
            "binary_operator" => {
                // Union type: X | Y
                let left = node.child_by_field_name("left");
                let right = node.child_by_field_name("right");
                match (left, right) {
                    (Some(l), Some(r)) => {
                        let left_ty = self.parse_type_annotation(&l);
                        let right_ty = self.parse_type_annotation(&r);
                        Type::union(vec![left_ty, right_ty])
                    }
                    _ => Type::Unknown,
                }
            }
            "none" => Type::None,
            _ => self.parse_simple_type(text),
        }
    }

    /// Parse a simple type name
    fn parse_simple_type(&self, name: &str) -> Type {
        match name {
            "int" => Type::Int,
            "float" => Type::Float,
            "str" => Type::Str,
            "bool" => Type::Bool,
            "bytes" => Type::Bytes,
            "None" => Type::None,
            "Any" => Type::Any,
            "object" => Type::Any,
            _ => Type::Instance {
                name: name.to_string(),
                module: None,
                type_args: vec![],
            },
        }
    }

    /// Parse a generic type like list[int], dict[str, int]
    fn parse_generic_type(&self, node: &Node) -> Type {
        let base = node
            .child_by_field_name("value")
            .map(|n| self.node_text(&n))
            .unwrap_or("");

        let args = self.parse_type_args(node);

        match base {
            "list" | "List" => {
                Type::list(args.first().cloned().unwrap_or(Type::Unknown))
            }
            "dict" | "Dict" => {
                let key = args.first().cloned().unwrap_or(Type::Unknown);
                let val = args.get(1).cloned().unwrap_or(Type::Unknown);
                Type::dict(key, val)
            }
            "set" | "Set" => {
                Type::Set(Box::new(args.first().cloned().unwrap_or(Type::Unknown)))
            }
            "tuple" | "Tuple" => Type::Tuple(args),
            "Optional" => {
                let inner = args.first().cloned().unwrap_or(Type::Unknown);
                Type::optional(inner)
            }
            "Union" => Type::union(args),
            "Callable" => {
                // Callable[[arg_types], return_type]
                if args.len() >= 2 {
                    let ret = args.last().cloned().unwrap_or(Type::Unknown);
                    let params: Vec<_> = args[..args.len() - 1]
                        .iter()
                        .enumerate()
                        .map(|(i, ty)| Param {
                            name: format!("_{}", i),
                            ty: ty.clone(),
                            has_default: false,
                            kind: ParamKind::Positional,
                        })
                        .collect();
                    Type::Callable {
                        params,
                        ret: Box::new(ret),
                    }
                } else {
                    Type::Unknown
                }
            }
            "type" => {
                let inner = args.first().cloned().unwrap_or(Type::Unknown);
                if let Type::Instance { name, module, .. } = inner {
                    Type::ClassType { name, module }
                } else {
                    Type::Unknown
                }
            }
            _ => Type::Instance {
                name: base.to_string(),
                module: None,
                type_args: args,
            },
        }
    }

    /// Parse type arguments from a subscript node
    fn parse_type_args(&self, node: &Node) -> Vec<Type> {
        let mut args = Vec::new();

        if let Some(subscript) = node.child_by_field_name("subscript") {
            match subscript.kind() {
                "tuple" | "expression_list" => {
                    let mut cursor = subscript.walk();
                    for child in subscript.children(&mut cursor) {
                        if child.kind() != "," && child.kind() != "(" && child.kind() != ")" {
                            args.push(self.parse_type_annotation(&child));
                        }
                    }
                }
                _ => {
                    args.push(self.parse_type_annotation(&subscript));
                }
            }
        }

        args
    }

    /// Bind a variable to a type based on assignment
    pub fn bind_assignment(&mut self, target: &Node, value_type: Type) {
        if target.kind() == "identifier" {
            let name = self.node_text(target).to_string();
            self.env.bind(name, value_type);
        }
    }

    /// Get the current environment (for testing)
    pub fn env(&self) -> &TypeEnv {
        &self.env
    }

    /// Get all variable types from the current environment
    pub fn get_env_types(&self) -> HashMap<String, Type> {
        let mut types = HashMap::new();
        for scope in &self.env.scopes {
            for (name, ty) in scope {
                types.insert(name.clone(), ty.clone());
            }
        }
        types
    }

    /// Analyze an import statement and add imported names to the environment
    pub fn analyze_import(&mut self, node: &Node) {
        if let Some(import) = parse_import(self.source, node) {
            let resolved = self.resolver.resolve_import(&import);
            for (name, ty) in resolved {
                self.env.bind(name, ty);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::MultiParser;

    fn infer_type(code: &str) -> Type {
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        // Find first expression
        let root = parsed.tree.root_node();
        if let Some(stmt) = root.child(0) {
            if stmt.kind() == "expression_statement" {
                if let Some(expr) = stmt.child(0) {
                    return inferencer.infer_expr(&expr);
                }
            }
        }
        Type::Unknown
    }

    #[test]
    fn test_infer_literals() {
        assert_eq!(infer_type("42"), Type::Int);
        assert_eq!(infer_type("3.14"), Type::Float);
        assert_eq!(infer_type("\"hello\""), Type::Str);
        assert_eq!(infer_type("True"), Type::Bool);
        assert_eq!(infer_type("None"), Type::None);
    }

    #[test]
    fn test_infer_binary_ops() {
        assert_eq!(infer_type("1 + 2"), Type::Int);
        assert_eq!(infer_type("1.0 + 2"), Type::Float);
        assert_eq!(infer_type("\"a\" + \"b\""), Type::Str);
        assert_eq!(infer_type("10 / 3"), Type::Float);
        assert_eq!(infer_type("10 // 3"), Type::Int);
    }

    #[test]
    fn test_infer_containers() {
        assert_eq!(infer_type("[1, 2, 3]"), Type::list(Type::Int));
        assert_eq!(
            infer_type("{\"a\": 1}"),
            Type::dict(Type::Str, Type::Int)
        );
        assert_eq!(
            infer_type("(1, \"a\")"),
            Type::Tuple(vec![Type::Int, Type::Str])
        );
    }

    #[test]
    fn test_class_analysis() {
        let code = r#"
class Person:
    name: str
    age: int = 0

    def __init__(self, name: str, age: int) -> None:
        self.name = name
        self.age = age

    def greet(self) -> str:
        return "Hello, " + self.name
"#;
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        // Find class definition
        let root = parsed.tree.root_node();
        if let Some(class_node) = root.child(0) {
            if class_node.kind() == "class_definition" {
                let class_info = inferencer.analyze_class(&class_node);

                assert_eq!(class_info.name, "Person");

                // Check class variables
                assert!(class_info.class_vars.contains_key("name"));
                assert!(class_info.class_vars.contains_key("age"));

                // Check methods
                assert!(class_info.methods.contains_key("__init__"));
                assert!(class_info.methods.contains_key("greet"));

                // Check __init__ sets instance attributes
                assert!(class_info.attributes.contains_key("name"));
                assert!(class_info.attributes.contains_key("age"));
            }
        }
    }

    #[test]
    fn test_class_attribute_inference() {
        let code = r#"
class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y

p = Point(1, 2)
p.x
"#;
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        // Walk through the code to analyze class and assignments
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "class_definition" {
                inferencer.analyze_class(&child);
            }
        }

        // Check that Point class was registered
        let class_info = inferencer.get_class("Point");
        assert!(class_info.is_some());
        let class_info = class_info.unwrap();
        assert!(class_info.attributes.contains_key("x"));
        assert!(class_info.attributes.contains_key("y"));
    }

    #[test]
    fn test_typing_import_integration() {
        let code = "from typing import List, Optional";
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        let root = parsed.tree.root_node();
        if let Some(import_node) = root.child(0) {
            inferencer.analyze_import(&import_node);
        }

        // Verify List and Optional are now in env
        assert!(inferencer.env().lookup("List").is_some());
        assert!(inferencer.env().lookup("Optional").is_some());
    }

    #[test]
    fn test_collections_import_integration() {
        let code = "from collections import deque, Counter";
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        let root = parsed.tree.root_node();
        if let Some(import_node) = root.child(0) {
            inferencer.analyze_import(&import_node);
        }

        assert!(inferencer.env().lookup("deque").is_some());
        assert!(inferencer.env().lookup("Counter").is_some());
    }

    #[test]
    fn test_import_with_alias() {
        let code = "from typing import List as L, Dict as D";
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        let root = parsed.tree.root_node();
        if let Some(import_node) = root.child(0) {
            inferencer.analyze_import(&import_node);
        }

        // Should be available under aliases
        assert!(inferencer.env().lookup("L").is_some());
        assert!(inferencer.env().lookup("D").is_some());
        // Original names should not be bound
        assert!(inferencer.env().lookup("List").is_none());
        assert!(inferencer.env().lookup("Dict").is_none());
    }

    #[test]
    fn test_inheritance_attribute_lookup() {
        let code = r#"
class Animal:
    species: str = "unknown"

    def speak(self) -> str:
        return "sound"

class Dog(Animal):
    def bark(self) -> str:
        return "woof"
"#;
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        // Analyze all classes
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "class_definition" {
                inferencer.analyze_class(&child);
            }
        }

        // Dog should have its own method
        let bark = inferencer.get_attribute_recursive("Dog", "bark");
        assert!(bark.is_some());

        // Dog should inherit speak from Animal
        let speak = inferencer.get_attribute_recursive("Dog", "speak");
        assert!(speak.is_some());

        // Dog should inherit class var from Animal
        let species = inferencer.get_attribute_recursive("Dog", "species");
        assert!(species.is_some());

        // Animal should not have bark
        let animal_bark = inferencer.get_attribute_recursive("Animal", "bark");
        assert!(animal_bark.is_none());
    }

    #[test]
    fn test_is_subclass() {
        let code = r#"
class Animal:
    pass

class Dog(Animal):
    pass

class Labrador(Dog):
    pass
"#;
        let mut parser = MultiParser::new().unwrap();
        let parsed = parser
            .parse(code, crate::syntax::Language::Python)
            .unwrap();
        let mut inferencer = TypeInferencer::new(code);

        // Analyze all classes
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "class_definition" {
                inferencer.analyze_class(&child);
            }
        }

        // Self is a subclass of self
        assert!(inferencer.is_subclass("Dog", "Dog"));

        // Dog is a subclass of Animal
        assert!(inferencer.is_subclass("Dog", "Animal"));

        // Labrador is a subclass of Dog and Animal (transitive)
        assert!(inferencer.is_subclass("Labrador", "Dog"));
        assert!(inferencer.is_subclass("Labrador", "Animal"));

        // Animal is NOT a subclass of Dog
        assert!(!inferencer.is_subclass("Animal", "Dog"));
    }

    #[test]
    fn test_generic_call_inference() {
        use crate::types::ty::{Param, ParamKind};

        // Test that calling a generic function infers type arguments
        // We'll manually create a generic function and test the inference

        // Create a generic identity function: def identity(x: T) -> T
        let t = Type::type_var(0, "T");
        let identity_fn = Type::Callable {
            params: vec![Param {
                name: "x".to_string(),
                ty: t.clone(),
                has_default: false,
                kind: ParamKind::Positional,
            }],
            ret: Box::new(t),
        };

        // Simulate unifying with Int argument
        let mut subs = HashMap::new();
        let param_ty = &identity_fn;
        if let Type::Callable { params, ret } = param_ty {
            // Unify parameter T with Int
            params[0].ty.unify(&Type::Int, &mut subs);

            // Apply substitution to return type
            let inferred_ret = ret.substitute(&subs);
            assert_eq!(inferred_ret, Type::Int);
        }
    }

    #[test]
    fn test_generic_list_inference() {
        use crate::types::ty::TypeVarId;

        // Test inferring element type from list[T] -> list[str]
        let t = Type::type_var(0, "T");
        let list_t = Type::list(t);

        let mut subs = HashMap::new();
        list_t.unify(&Type::list(Type::Str), &mut subs);

        // T should be inferred as Str
        assert_eq!(subs.get(&TypeVarId(0)), Some(&Type::Str));
    }
}
