//! Integration tests for deep type inference (M5.3)
//!
//! Tests:
//! - Dynamic protocol conformance checking
//! - Cross-file type propagation
//! - Generic type inference
//! - Framework integration

use argus::types::{
    DeepTypeInferencer, TypeBinding, ImportInfo, TypeContext, ProtocolDef, MethodSignature,
    ClassInfo, Type, Param, ParamKind,
};
use std::path::PathBuf;
use std::collections::HashMap;

// ============================================================================
// Protocol Conformance Tests
// ============================================================================

#[test]
fn test_protocol_conformance_basic() {
    let mut context = TypeContext::new();

    // Define a simple protocol: Drawable
    let mut protocol = ProtocolDef {
        name: "Drawable".to_string(),
        methods: HashMap::new(),
        attributes: HashMap::new(),
        parents: Vec::new(),
    };

    // Required method: draw() -> None
    protocol.methods.insert(
        "draw".to_string(),
        MethodSignature {
            name: "draw".to_string(),
            params: vec![],
            return_type: Type::None,
            is_async: false,
        },
    );

    context.add_protocol("Drawable".to_string(), protocol);

    // Define a class that implements Drawable
    let mut class_info = ClassInfo::new("Circle".to_string());
    class_info.methods.insert(
        "draw".to_string(),
        Type::Callable {
            params: vec![],
            ret: Box::new(Type::None),
        },
    );

    context.add_class_info("Circle".to_string(), class_info);

    // Test conformance
    let circle_instance = Type::Instance {
        name: "Circle".to_string(),
        module: None,
        type_args: vec![],
    };

    assert!(
        context.satisfies_protocol(&circle_instance, "Drawable"),
        "Circle should implement Drawable protocol"
    );
}

#[test]
fn test_protocol_conformance_missing_method() {
    let mut context = TypeContext::new();

    // Define protocol with required method
    let mut protocol = ProtocolDef {
        name: "Clickable".to_string(),
        methods: HashMap::new(),
        attributes: HashMap::new(),
        parents: Vec::new(),
    };

    protocol.methods.insert(
        "on_click".to_string(),
        MethodSignature {
            name: "on_click".to_string(),
            params: vec![],
            return_type: Type::None,
            is_async: false,
        },
    );

    context.add_protocol("Clickable".to_string(), protocol);

    // Define a class without the required method
    let class_info = ClassInfo::new("Button".to_string());
    // No methods added
    context.add_class_info("Button".to_string(), class_info);

    // Test conformance
    let button_instance = Type::Instance {
        name: "Button".to_string(),
        module: None,
        type_args: vec![],
    };

    assert!(
        !context.satisfies_protocol(&button_instance, "Clickable"),
        "Button should NOT implement Clickable (missing on_click method)"
    );
}

#[test]
fn test_protocol_conformance_with_attributes() {
    let mut context = TypeContext::new();

    // Define protocol with attributes
    let mut protocol = ProtocolDef {
        name: "Named".to_string(),
        methods: HashMap::new(),
        attributes: HashMap::new(),
        parents: Vec::new(),
    };

    protocol.attributes.insert("name".to_string(), Type::Str);

    context.add_protocol("Named".to_string(), protocol);

    // Define class with attribute
    let mut class_info = ClassInfo::new("Person".to_string());
    class_info.attributes.insert("name".to_string(), Type::Str);
    context.add_class_info("Person".to_string(), class_info);

    // Test conformance
    let person_instance = Type::Instance {
        name: "Person".to_string(),
        module: None,
        type_args: vec![],
    };

    assert!(
        context.satisfies_protocol(&person_instance, "Named"),
        "Person should implement Named protocol"
    );
}

#[test]
fn test_protocol_conformance_parent_protocols() {
    let mut context = TypeContext::new();

    // Define parent protocol: Readable
    let mut readable = ProtocolDef {
        name: "Readable".to_string(),
        methods: HashMap::new(),
        attributes: HashMap::new(),
        parents: Vec::new(),
    };
    readable.methods.insert(
        "read".to_string(),
        MethodSignature {
            name: "read".to_string(),
            params: vec![],
            return_type: Type::Str,
            is_async: false,
        },
    );
    context.add_protocol("Readable".to_string(), readable);

    // Define child protocol: Writable extends Readable
    let mut writable = ProtocolDef {
        name: "Writable".to_string(),
        methods: HashMap::new(),
        attributes: HashMap::new(),
        parents: vec!["Readable".to_string()],
    };
    writable.methods.insert(
        "write".to_string(),
        MethodSignature {
            name: "write".to_string(),
            params: vec![("data".to_string(), Type::Str)],
            return_type: Type::None,
            is_async: false,
        },
    );
    context.add_protocol("Writable".to_string(), writable);

    // Define class implementing both methods
    let mut class_info = ClassInfo::new("File".to_string());
    class_info.methods.insert(
        "read".to_string(),
        Type::Callable {
            params: vec![],
            ret: Box::new(Type::Str),
        },
    );
    class_info.methods.insert(
        "write".to_string(),
        Type::Callable {
            params: vec![Param {
                name: "data".to_string(),
                ty: Type::Str,
                has_default: false,
                kind: ParamKind::Positional,
            }],
            ret: Box::new(Type::None),
        },
    );
    context.add_class_info("File".to_string(), class_info);

    // Test conformance
    let file_instance = Type::Instance {
        name: "File".to_string(),
        module: None,
        type_args: vec![],
    };

    assert!(
        context.satisfies_protocol(&file_instance, "Writable"),
        "File should implement Writable protocol (which extends Readable)"
    );
}

#[test]
fn test_protocol_conformance_incompatible_signature() {
    let mut context = TypeContext::new();

    // Define protocol
    let mut protocol = ProtocolDef {
        name: "Calculator".to_string(),
        methods: HashMap::new(),
        attributes: HashMap::new(),
        parents: Vec::new(),
    };

    // Required: add(x: int, y: int) -> int
    protocol.methods.insert(
        "add".to_string(),
        MethodSignature {
            name: "add".to_string(),
            params: vec![
                ("x".to_string(), Type::Int),
                ("y".to_string(), Type::Int),
            ],
            return_type: Type::Int,
            is_async: false,
        },
    );

    context.add_protocol("Calculator".to_string(), protocol);

    // Define class with wrong return type
    let mut class_info = ClassInfo::new("BadCalc".to_string());
    class_info.methods.insert(
        "add".to_string(),
        Type::Callable {
            params: vec![
                Param {
                    name: "x".to_string(),
                    ty: Type::Int,
                    has_default: false,
                    kind: ParamKind::Positional,
                },
                Param {
                    name: "y".to_string(),
                    ty: Type::Int,
                    has_default: false,
                    kind: ParamKind::Positional,
                },
            ],
            ret: Box::new(Type::Str), // Wrong! Should be Int
        },
    );
    context.add_class_info("BadCalc".to_string(), class_info);

    // Test conformance
    let bad_calc_instance = Type::Instance {
        name: "BadCalc".to_string(),
        module: None,
        type_args: vec![],
    };

    assert!(
        !context.satisfies_protocol(&bad_calc_instance, "Calculator"),
        "BadCalc should NOT implement Calculator (wrong return type)"
    );
}

// ============================================================================
// Cross-File Type Propagation Tests
// ============================================================================

#[test]
fn test_cross_file_type_propagation_basic() {
    let mut inferencer = DeepTypeInferencer::new();

    // Setup source file (utils.py)
    let source_file = PathBuf::from("utils.py");
    inferencer.add_file(source_file.clone());

    // Add a symbol to source file
    let binding = TypeBinding {
        ty: Type::Callable {
            params: vec![Param {
                name: "x".to_string(),
                ty: Type::Int,
                has_default: false,
                kind: ParamKind::Positional,
            }],
            ret: Box::new(Type::Int),
        },
        source_file: source_file.clone(),
        symbol: "double".to_string(),
        line: 1,
        is_exported: true,
        dependencies: vec![],
    };

    inferencer.context_mut().add_binding(source_file.clone(), binding.clone());

    // Add symbol to file analysis
    inferencer.add_file_symbol(&source_file, "double".to_string(), binding);

    // Setup target file (main.py)
    let target_file = PathBuf::from("main.py");
    inferencer.add_file(target_file.clone());

    // Propagate type from source to target
    inferencer.propagate_types(&source_file, &target_file, Some(&["double".to_string()]));

    // Verify symbol exists in target file
    let target_symbols = inferencer.get_file_symbols(&target_file);
    assert!(
        target_symbols.contains_key("double"),
        "double should be imported in main.py"
    );

    // Verify type is correct
    match target_symbols.get("double") {
        Some(Type::Callable { params, ret }) => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].ty, Type::Int);
            assert_eq!(**ret, Type::Int);
        }
        _ => panic!("double should be a Callable type"),
    }
}

#[test]
fn test_cross_file_type_propagation_all_exported() {
    let mut inferencer = DeepTypeInferencer::new();

    // Setup source file with multiple symbols
    let source_file = PathBuf::from("lib.py");
    inferencer.add_file(source_file.clone());

    // Add exported symbols
    let symbols = vec![
        ("func1", Type::Int, true),
        ("func2", Type::Str, true),
        ("_internal", Type::Bool, false), // Not exported
    ];

    for (name, ty, is_exported) in symbols {
        let binding = TypeBinding {
            ty,
            source_file: source_file.clone(),
            symbol: name.to_string(),
            line: 1,
            is_exported,
            dependencies: vec![],
        };

        inferencer.context_mut().add_binding(source_file.clone(), binding.clone());
        inferencer.add_file_symbol(&source_file, name.to_string(), binding);
    }

    // Setup target file
    let target_file = PathBuf::from("app.py");
    inferencer.add_file(target_file.clone());

    // Propagate all exported symbols (None = import all)
    inferencer.propagate_types(&source_file, &target_file, None);

    // Verify only exported symbols are imported
    let target_symbols = inferencer.get_file_symbols(&target_file);
    assert!(target_symbols.contains_key("func1"));
    assert!(target_symbols.contains_key("func2"));
    assert!(!target_symbols.contains_key("_internal"), "_internal should not be imported");
}

#[test]
fn test_symbol_type_update_propagation() {
    let mut inferencer = DeepTypeInferencer::new();

    // Setup source file
    let source_file = PathBuf::from("models.py");
    inferencer.add_file(source_file.clone());

    let binding = TypeBinding {
        ty: Type::Int,
        source_file: source_file.clone(),
        symbol: "counter".to_string(),
        line: 1,
        is_exported: true,
        dependencies: vec![],
    };

    inferencer.context_mut().add_binding(source_file.clone(), binding.clone());
    inferencer.add_file_symbol(&source_file, "counter".to_string(), binding);

    // Setup importing file
    let target_file = PathBuf::from("views.py");
    inferencer.add_file(target_file.clone());
    inferencer.propagate_types(&source_file, &target_file, Some(&["counter".to_string()]));

    // Verify initial type
    let symbols_before = inferencer.get_file_symbols(&target_file);
    assert_eq!(symbols_before.get("counter"), Some(&Type::Int));

    // Update type in source file
    inferencer.update_symbol_type(&source_file, "counter", Type::Float);

    // Verify propagation to target file
    let symbols_after = inferencer.get_file_symbols(&target_file);
    assert_eq!(
        symbols_after.get("counter"),
        Some(&Type::Float),
        "Type change should propagate to importing file"
    );
}

#[test]
fn test_cascading_type_propagation() {
    let mut inferencer = DeepTypeInferencer::new();

    // Setup chain: base.py -> middle.py -> top.py
    let base_file = PathBuf::from("base.py");
    let middle_file = PathBuf::from("middle.py");
    let top_file = PathBuf::from("top.py");

    // Add files
    for file in &[&base_file, &middle_file, &top_file] {
        inferencer.add_file((*file).clone());
    }

    // Add symbol to base
    let binding = TypeBinding {
        ty: Type::Str,
        source_file: base_file.clone(),
        symbol: "message".to_string(),
        line: 1,
        is_exported: true,
        dependencies: vec![],
    };

    inferencer.context_mut().add_binding(base_file.clone(), binding.clone());
    inferencer.add_file_symbol(&base_file, "message".to_string(), binding);

    // Propagate base -> middle
    inferencer.propagate_types(&base_file, &middle_file, Some(&["message".to_string()]));

    // Mark as exported in middle
    inferencer.set_symbol_exported(&middle_file, "message", true);

    // Propagate middle -> top
    inferencer.propagate_types(&middle_file, &top_file, Some(&["message".to_string()]));

    // Verify symbol in top file
    let top_symbols = inferencer.get_file_symbols(&top_file);
    assert_eq!(
        top_symbols.get("message"),
        Some(&Type::Str),
        "Type should propagate through chain: base -> middle -> top"
    );

    // Update type in base
    inferencer.update_symbol_type(&base_file, "message", Type::Int);

    // Verify propagation to top (through middle)
    let top_symbols_after = inferencer.get_file_symbols(&top_file);
    assert_eq!(
        top_symbols_after.get("message"),
        Some(&Type::Int),
        "Type update should cascade through entire import chain"
    );
}

#[test]
fn test_import_info_tracking() {
    let mut inferencer = DeepTypeInferencer::new();

    let file = PathBuf::from("test.py");
    inferencer.add_file(file.clone());

    // Add import
    let import = ImportInfo {
        module: "os".to_string(),
        names: Some(vec!["path".to_string()]),
        alias: None,
    };

    inferencer.add_import(&file, import);

    // Verify import is recorded
    if let Some(analysis) = inferencer.get_file_analysis(&file) {
        assert_eq!(analysis.imports.len(), 1);
        assert_eq!(analysis.imports[0].module, "os");
        assert_eq!(
            analysis.imports[0].names.as_ref().unwrap()[0],
            "path"
        );
    } else {
        panic!("File should exist");
    }
}
