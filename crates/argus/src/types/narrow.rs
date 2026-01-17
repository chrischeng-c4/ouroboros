//! Type narrowing based on control flow analysis
//!
//! This module handles type narrowing for:
//! - isinstance() checks
//! - None checks (is None, is not None)
//! - Truthiness checks
//! - Type guards

use std::collections::HashMap;

use super::ty::Type;

/// Represents a condition that can narrow types
#[derive(Debug, Clone)]
pub enum NarrowingCondition {
    /// isinstance(x, T) or isinstance(x, (T1, T2))
    IsInstance {
        var_name: String,
        types: Vec<Type>,
    },
    /// x is None
    IsNone { var_name: String },
    /// x is not None
    IsNotNone { var_name: String },
    /// x (truthiness check)
    Truthy { var_name: String },
    /// not x (falsiness check)
    Falsy { var_name: String },
    /// x == literal
    Equals {
        var_name: String,
        value: Type,
    },
    /// x != literal
    NotEquals {
        var_name: String,
        value: Type,
    },
    /// Compound: cond1 and cond2
    And(Box<NarrowingCondition>, Box<NarrowingCondition>),
    /// Compound: cond1 or cond2
    Or(Box<NarrowingCondition>, Box<NarrowingCondition>),
    /// Negation: not cond
    Not(Box<NarrowingCondition>),
    /// Unknown/unanalyzable condition
    Unknown,
}

/// Type narrower that tracks narrowed types in different branches
#[derive(Debug, Clone, Default)]
pub struct TypeNarrower {
    /// Stack of narrowing scopes (for nested if/else)
    scopes: Vec<NarrowingScope>,
}

/// A narrowing scope tracks type refinements in a specific branch
#[derive(Debug, Clone, Default)]
pub struct NarrowingScope {
    /// Narrowed types for variables in this scope
    narrowed: HashMap<String, Type>,
}

impl TypeNarrower {
    pub fn new() -> Self {
        Self { scopes: vec![] }
    }

    /// Push a new narrowing scope (entering an if/else branch)
    pub fn push_scope(&mut self) {
        self.scopes.push(NarrowingScope::default());
    }

    /// Pop the current narrowing scope (leaving a branch)
    pub fn pop_scope(&mut self) -> Option<NarrowingScope> {
        self.scopes.pop()
    }

    /// Apply a narrowing condition to the current scope
    pub fn apply_condition(&mut self, condition: &NarrowingCondition, original_types: &HashMap<String, Type>) {
        match condition {
            NarrowingCondition::IsInstance { var_name, types } => {
                // Narrow to the union of instance types
                let narrowed = if types.len() == 1 {
                    types[0].clone()
                } else {
                    Type::union(types.clone())
                };
                self.narrow_var(var_name, narrowed);
            }
            NarrowingCondition::IsNone { var_name } => {
                self.narrow_var(var_name, Type::None);
            }
            NarrowingCondition::IsNotNone { var_name } => {
                if let Some(original) = original_types.get(var_name) {
                    let narrowed = original.without_none();
                    self.narrow_var(var_name, narrowed);
                }
            }
            NarrowingCondition::Truthy { var_name } => {
                // Truthy narrows away None and False-like values
                if let Some(original) = original_types.get(var_name) {
                    let narrowed = original.without_none();
                    self.narrow_var(var_name, narrowed);
                }
            }
            NarrowingCondition::Falsy { var_name } => {
                // Falsy could mean None, False, 0, "", etc.
                // For Optional types, narrow to None
                if let Some(original) = original_types.get(var_name) {
                    if original.contains_none() {
                        self.narrow_var(var_name, Type::None);
                    }
                }
            }
            NarrowingCondition::Equals { var_name, value } => {
                self.narrow_var(var_name, value.clone());
            }
            NarrowingCondition::NotEquals { var_name, value } => {
                if let Some(original) = original_types.get(var_name) {
                    // If comparing against None, remove None from type
                    if matches!(value, Type::None) {
                        let narrowed = original.without_none();
                        self.narrow_var(var_name, narrowed);
                    }
                }
            }
            NarrowingCondition::And(left, right) => {
                self.apply_condition(left, original_types);
                self.apply_condition(right, original_types);
            }
            NarrowingCondition::Or(left, right) => {
                // For OR, we need to compute the union of both branches
                let mut left_narrower = self.clone();
                let mut right_narrower = self.clone();
                left_narrower.apply_condition(left, original_types);
                right_narrower.apply_condition(right, original_types);

                // Merge: take union of narrowed types
                // This is conservative - we keep the widest possible type
                // For now, just don't narrow on OR
            }
            NarrowingCondition::Not(inner) => {
                // Apply the negation of the condition
                let negated = Self::negate_condition(inner);
                self.apply_condition(&negated, original_types);
            }
            NarrowingCondition::Unknown => {}
        }
    }

    /// Negate a condition
    fn negate_condition(condition: &NarrowingCondition) -> NarrowingCondition {
        match condition {
            NarrowingCondition::IsNone { var_name } => {
                NarrowingCondition::IsNotNone { var_name: var_name.clone() }
            }
            NarrowingCondition::IsNotNone { var_name } => {
                NarrowingCondition::IsNone { var_name: var_name.clone() }
            }
            NarrowingCondition::Truthy { var_name } => {
                NarrowingCondition::Falsy { var_name: var_name.clone() }
            }
            NarrowingCondition::Falsy { var_name } => {
                NarrowingCondition::Truthy { var_name: var_name.clone() }
            }
            NarrowingCondition::And(left, right) => {
                // not (A and B) = (not A) or (not B)
                NarrowingCondition::Or(
                    Box::new(Self::negate_condition(left)),
                    Box::new(Self::negate_condition(right)),
                )
            }
            NarrowingCondition::Or(left, right) => {
                // not (A or B) = (not A) and (not B)
                NarrowingCondition::And(
                    Box::new(Self::negate_condition(left)),
                    Box::new(Self::negate_condition(right)),
                )
            }
            NarrowingCondition::Not(inner) => {
                // not (not x) = x
                (**inner).clone()
            }
            other => other.clone(),
        }
    }

    /// Set a narrowed type for a variable
    fn narrow_var(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.narrowed.insert(name.to_string(), ty);
        }
    }

    /// Get the narrowed type for a variable, if any
    pub fn get_narrowed(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.narrowed.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// Compute the intersection of a type with the narrowed type
    pub fn narrow_type(&self, name: &str, original: &Type) -> Type {
        if let Some(narrowed) = self.get_narrowed(name) {
            // Return the more specific type
            narrowed.clone()
        } else {
            original.clone()
        }
    }
}

/// Parse a condition expression into a NarrowingCondition
#[allow(dead_code)]
pub fn parse_condition(source: &str, node: &tree_sitter::Node) -> NarrowingCondition {
    let node_text = |n: &tree_sitter::Node| -> &str {
        n.utf8_text(source.as_bytes()).unwrap_or("")
    };

    match node.kind() {
        "comparison_operator" => parse_comparison(source, node),
        "boolean_operator" => parse_boolean_op(source, node),
        "not_operator" => {
            if let Some(arg) = node.child(1) {
                NarrowingCondition::Not(Box::new(parse_condition(source, &arg)))
            } else {
                NarrowingCondition::Unknown
            }
        }
        "call" => parse_call_condition(source, node),
        "identifier" => {
            // Bare identifier is a truthiness check
            NarrowingCondition::Truthy {
                var_name: node_text(node).to_string(),
            }
        }
        _ => NarrowingCondition::Unknown,
    }
}

#[allow(dead_code)]
fn parse_comparison(source: &str, node: &tree_sitter::Node) -> NarrowingCondition {
    let node_text = |n: &tree_sitter::Node| -> &str {
        n.utf8_text(source.as_bytes()).unwrap_or("")
    };

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() >= 3 {
        let left = &children[0];
        let op = node_text(&children[1]);
        let right = &children[2];

        let left_text = node_text(left);
        let right_text = node_text(right);

        match op {
            "is" => {
                if right_text == "None" && left.kind() == "identifier" {
                    return NarrowingCondition::IsNone {
                        var_name: left_text.to_string(),
                    };
                }
            }
            "is not" => {
                if right_text == "None" && left.kind() == "identifier" {
                    return NarrowingCondition::IsNotNone {
                        var_name: left_text.to_string(),
                    };
                }
            }
            "==" => {
                if left.kind() == "identifier" && right_text == "None" {
                    return NarrowingCondition::IsNone {
                        var_name: left_text.to_string(),
                    };
                }
            }
            "!=" => {
                if left.kind() == "identifier" && right_text == "None" {
                    return NarrowingCondition::IsNotNone {
                        var_name: left_text.to_string(),
                    };
                }
            }
            _ => {}
        }
    }

    NarrowingCondition::Unknown
}

#[allow(dead_code)]
fn parse_boolean_op(source: &str, node: &tree_sitter::Node) -> NarrowingCondition {
    let node_text = |n: &tree_sitter::Node| -> &str {
        n.utf8_text(source.as_bytes()).unwrap_or("")
    };

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() >= 3 {
        let left = &children[0];
        let op = node_text(&children[1]);
        let right = &children[2];

        let left_cond = parse_condition(source, left);
        let right_cond = parse_condition(source, right);

        match op {
            "and" => NarrowingCondition::And(Box::new(left_cond), Box::new(right_cond)),
            "or" => NarrowingCondition::Or(Box::new(left_cond), Box::new(right_cond)),
            _ => NarrowingCondition::Unknown,
        }
    } else {
        NarrowingCondition::Unknown
    }
}

#[allow(dead_code)]
fn parse_call_condition(source: &str, node: &tree_sitter::Node) -> NarrowingCondition {
    let node_text = |n: &tree_sitter::Node| -> &str {
        n.utf8_text(source.as_bytes()).unwrap_or("")
    };

    let func = match node.child_by_field_name("function") {
        Some(f) => f,
        None => return NarrowingCondition::Unknown,
    };

    let func_name = node_text(&func);

    if func_name == "isinstance" {
        let args = match node.child_by_field_name("arguments") {
            Some(a) => a,
            None => return NarrowingCondition::Unknown,
        };

        let mut cursor = args.walk();
        let arg_nodes: Vec<_> = args
            .children(&mut cursor)
            .filter(|n| n.kind() != "(" && n.kind() != ")" && n.kind() != ",")
            .collect();

        if arg_nodes.len() >= 2 {
            let var_node = &arg_nodes[0];
            let type_node = &arg_nodes[1];

            if var_node.kind() == "identifier" {
                let var_name = node_text(var_node).to_string();
                let types = parse_isinstance_types(source, type_node);
                return NarrowingCondition::IsInstance { var_name, types };
            }
        }
    }

    NarrowingCondition::Unknown
}

#[allow(dead_code)]
fn parse_isinstance_types(source: &str, node: &tree_sitter::Node) -> Vec<Type> {
    let node_text = |n: &tree_sitter::Node| -> &str {
        n.utf8_text(source.as_bytes()).unwrap_or("")
    };

    match node.kind() {
        "identifier" => {
            let name = node_text(node);
            vec![parse_simple_type_name(name)]
        }
        "tuple" => {
            let mut types = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    types.push(parse_simple_type_name(node_text(&child)));
                }
            }
            types
        }
        _ => vec![Type::Unknown],
    }
}

#[allow(dead_code)]
fn parse_simple_type_name(name: &str) -> Type {
    match name {
        "int" => Type::Int,
        "float" => Type::Float,
        "str" => Type::Str,
        "bool" => Type::Bool,
        "bytes" => Type::Bytes,
        "list" => Type::list(Type::Unknown),
        "dict" => Type::dict(Type::Unknown, Type::Unknown),
        "set" => Type::Set(Box::new(Type::Unknown)),
        "tuple" => Type::Tuple(vec![]),
        _ => Type::Instance {
            name: name.to_string(),
            module: None,
            type_args: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_none_narrowing() {
        let mut narrower = TypeNarrower::new();
        narrower.push_scope();

        let original_types: HashMap<String, Type> = [("x".to_string(), Type::optional(Type::Str))]
            .into_iter()
            .collect();

        let condition = NarrowingCondition::IsNone {
            var_name: "x".to_string(),
        };

        narrower.apply_condition(&condition, &original_types);

        assert_eq!(narrower.get_narrowed("x"), Some(&Type::None));
    }

    #[test]
    fn test_is_not_none_narrowing() {
        let mut narrower = TypeNarrower::new();
        narrower.push_scope();

        let original_types: HashMap<String, Type> = [("x".to_string(), Type::optional(Type::Str))]
            .into_iter()
            .collect();

        let condition = NarrowingCondition::IsNotNone {
            var_name: "x".to_string(),
        };

        narrower.apply_condition(&condition, &original_types);

        assert_eq!(narrower.get_narrowed("x"), Some(&Type::Str));
    }

    #[test]
    fn test_isinstance_narrowing() {
        let mut narrower = TypeNarrower::new();
        narrower.push_scope();

        let original_types: HashMap<String, Type> =
            [("x".to_string(), Type::union(vec![Type::Int, Type::Str]))]
                .into_iter()
                .collect();

        let condition = NarrowingCondition::IsInstance {
            var_name: "x".to_string(),
            types: vec![Type::Int],
        };

        narrower.apply_condition(&condition, &original_types);

        assert_eq!(narrower.get_narrowed("x"), Some(&Type::Int));
    }

    #[test]
    fn test_scope_stack() {
        let mut narrower = TypeNarrower::new();

        narrower.push_scope();
        narrower.narrow_var("x", Type::Int);
        assert_eq!(narrower.get_narrowed("x"), Some(&Type::Int));

        narrower.push_scope();
        narrower.narrow_var("x", Type::Str);
        assert_eq!(narrower.get_narrowed("x"), Some(&Type::Str));

        narrower.pop_scope();
        assert_eq!(narrower.get_narrowed("x"), Some(&Type::Int));

        narrower.pop_scope();
        assert_eq!(narrower.get_narrowed("x"), None);
    }
}
