//! Tests for type inference

use std::collections::HashMap;

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
