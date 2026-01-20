//! Integration tests for framework support in type inferencer

use argus::syntax::{Language, MultiParser};
use argus::types::{
    DjangoField, DjangoFieldType, DjangoModel, DjangoTypeProvider,
    Type, TypeInferencer,
};
use std::collections::HashMap;

/// Helper to setup a type inferencer with Django models
fn setup_inferencer_with_django(source: &str, models: Vec<DjangoModel>) -> TypeInferencer {
    let mut inferencer = TypeInferencer::new(source);

    // Register Django type provider
    let mut django_provider = DjangoTypeProvider::new();
    for model in models {
        django_provider.register_model(model);
    }

    inferencer.framework_registry_mut().register(Box::new(django_provider));
    inferencer
}

#[test]
fn test_django_model_field_inference() {
    let source = r#"user.name"#;

    // Create User model
    let mut fields = HashMap::new();
    fields.insert(
        "name".to_string(),
        DjangoField {
            name: "name".to_string(),
            field_type: DjangoFieldType::CharField,
            null: false,
            has_default: false,
        },
    );

    let user_model = DjangoModel {
        name: "User".to_string(),
        fields,
        relations: Vec::new(),
    };

    let mut inferencer = setup_inferencer_with_django(source, vec![user_model]);

    // Bind `user` to User type in the environment
    inferencer.bind_type("user".to_string(), Type::Instance {
        name: "User".to_string(),
        module: None,
        type_args: vec![],
    });

    // Parse the code
    let mut parser = MultiParser::new().expect("Failed to create parser");
    let parsed = parser.parse(source, Language::Python).expect("Failed to parse");

    // The root should be an expression_statement containing an attribute
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();

    // Find the attribute node
    for child in root.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let expr = child.child(0).unwrap();
            if expr.kind() == "attribute" {
                let inferred_type = inferencer.infer_expr(&expr);
                assert!(matches!(inferred_type, Type::Str),
                    "user.name should be inferred as str, got {:?}", inferred_type);
                return;
            }
        }
    }

    panic!("Could not find attribute node");
}

#[test]
fn test_django_queryset_filter_method() {
    let source = r#"users.filter()"#;

    // Create User model
    let user_model = DjangoModel {
        name: "User".to_string(),
        fields: HashMap::new(),
        relations: Vec::new(),
    };

    let mut inferencer = setup_inferencer_with_django(source, vec![user_model]);

    // Bind `users` to UserQuerySet type in the environment
    inferencer.bind_type("users".to_string(), Type::Instance {
        name: "UserQuerySet".to_string(),
        module: None,
        type_args: vec![],
    });

    // Parse the code
    let mut parser = MultiParser::new().expect("Failed to create parser");
    let parsed = parser.parse(source, Language::Python).expect("Failed to parse");

    // Find the call expression
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let expr = child.child(0).unwrap();
            if expr.kind() == "call" {
                let inferred_type = inferencer.infer_expr(&expr);

                if let Type::Instance { name, .. } = inferred_type {
                    assert_eq!(name, "UserQuerySet", "users.filter() should return UserQuerySet");
                } else {
                    panic!("users.filter() should return Instance type, got {:?}", inferred_type);
                }
                return;
            }
        }
    }

    panic!("Could not find call node");
}

#[test]
fn test_django_queryset_get_method() {
    let source = r#"products.get()"#;

    // Create Product model
    let product_model = DjangoModel {
        name: "Product".to_string(),
        fields: HashMap::new(),
        relations: Vec::new(),
    };

    let mut inferencer = setup_inferencer_with_django(source, vec![product_model]);

    // Bind `products` to ProductQuerySet type
    inferencer.bind_type("products".to_string(), Type::Instance {
        name: "ProductQuerySet".to_string(),
        module: None,
        type_args: vec![],
    });

    // Parse the code
    let mut parser = MultiParser::new().expect("Failed to create parser");
    let parsed = parser.parse(source, Language::Python).expect("Failed to parse");

    // Find the call expression
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let expr = child.child(0).unwrap();
            if expr.kind() == "call" {
                let inferred_type = inferencer.infer_expr(&expr);

                if let Type::Instance { name, .. } = inferred_type {
                    assert_eq!(name, "Product", "products.get() should return Product");
                } else {
                    panic!("products.get() should return Instance type, got {:?}", inferred_type);
                }
                return;
            }
        }
    }

    panic!("Could not find call node");
}

#[test]
fn test_django_queryset_count_method() {
    let source = r#"articles.count()"#;

    // Create Article model
    let article_model = DjangoModel {
        name: "Article".to_string(),
        fields: HashMap::new(),
        relations: Vec::new(),
    };

    let mut inferencer = setup_inferencer_with_django(source, vec![article_model]);

    // Bind `articles` to ArticleQuerySet type
    inferencer.bind_type("articles".to_string(), Type::Instance {
        name: "ArticleQuerySet".to_string(),
        module: None,
        type_args: vec![],
    });

    // Parse the code
    let mut parser = MultiParser::new().expect("Failed to create parser");
    let parsed = parser.parse(source, Language::Python).expect("Failed to parse");

    // Find the call expression
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let expr = child.child(0).unwrap();
            if expr.kind() == "call" {
                let inferred_type = inferencer.infer_expr(&expr);
                assert!(matches!(inferred_type, Type::Int),
                    "articles.count() should return int, got {:?}", inferred_type);
                return;
            }
        }
    }

    panic!("Could not find call node");
}

#[test]
fn test_django_queryset_first_method() {
    let source = r#"comments.first()"#;

    // Create Comment model
    let comment_model = DjangoModel {
        name: "Comment".to_string(),
        fields: HashMap::new(),
        relations: Vec::new(),
    };

    let mut inferencer = setup_inferencer_with_django(source, vec![comment_model]);

    // Bind `comments` to CommentQuerySet type
    inferencer.bind_type("comments".to_string(), Type::Instance {
        name: "CommentQuerySet".to_string(),
        module: None,
        type_args: vec![],
    });

    // Parse the code
    let mut parser = MultiParser::new().expect("Failed to create parser");
    let parsed = parser.parse(source, Language::Python).expect("Failed to parse");

    // Find the call expression
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let expr = child.child(0).unwrap();
            if expr.kind() == "call" {
                let inferred_type = inferencer.infer_expr(&expr);

                if let Type::Optional(inner) = inferred_type {
                    if let Type::Instance { name, .. } = inner.as_ref() {
                        assert_eq!(name, "Comment", "comments.first() should return Optional[Comment]");
                    } else {
                        panic!("Inner type should be Instance, got {:?}", inner);
                    }
                } else {
                    panic!("comments.first() should return Optional type, got {:?}", inferred_type);
                }
                return;
            }
        }
    }

    panic!("Could not find call node");
}

#[test]
fn test_django_model_save_method() {
    let source = r#"task.save()"#;

    // Create Task model
    let task_model = DjangoModel {
        name: "Task".to_string(),
        fields: HashMap::new(),
        relations: Vec::new(),
    };

    let mut inferencer = setup_inferencer_with_django(source, vec![task_model]);

    // Bind `task` to Task type
    inferencer.bind_type("task".to_string(), Type::Instance {
        name: "Task".to_string(),
        module: None,
        type_args: vec![],
    });

    // Parse the code
    let mut parser = MultiParser::new().expect("Failed to create parser");
    let parsed = parser.parse(source, Language::Python).expect("Failed to parse");

    // Find the call expression
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let expr = child.child(0).unwrap();
            if expr.kind() == "call" {
                let inferred_type = inferencer.infer_expr(&expr);
                assert!(matches!(inferred_type, Type::None),
                    "task.save() should return None, got {:?}", inferred_type);
                return;
            }
        }
    }

    panic!("Could not find call node");
}

#[test]
fn test_django_field_types() {
    let test_cases = vec![
        ("name", DjangoFieldType::CharField, Type::Str),
        ("age", DjangoFieldType::IntegerField, Type::Int),
        ("score", DjangoFieldType::FloatField, Type::Float),
        ("active", DjangoFieldType::BooleanField, Type::Bool),
    ];

    for (field_name, field_type, expected_type) in test_cases {
        let source = format!("user.{}", field_name);

        let mut fields = HashMap::new();
        fields.insert(
            field_name.to_string(),
            DjangoField {
                name: field_name.to_string(),
                field_type,
                null: false,
                has_default: false,
            },
        );

        let user_model = DjangoModel {
            name: "User".to_string(),
            fields,
            relations: Vec::new(),
        };

        let mut inferencer = setup_inferencer_with_django(&source, vec![user_model]);

        // Bind `user` to User type
        inferencer.bind_type("user".to_string(), Type::Instance {
            name: "User".to_string(),
            module: None,
            type_args: vec![],
        });

        // Parse the code
        let mut parser = MultiParser::new().expect("Failed to create parser");
        let parsed = parser.parse(&source, Language::Python).expect("Failed to parse");

        // Find the attribute access node
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                let expr = child.child(0).unwrap();
                if expr.kind() == "attribute" {
                    let inferred_type = inferencer.infer_expr(&expr);
                    assert!(std::mem::discriminant(&inferred_type) == std::mem::discriminant(&expected_type),
                            "{} should be inferred as {:?}, got {:?}", source, expected_type, inferred_type);
                    break;
                }
            }
        }
    }
}

#[test]
fn test_django_queryset_chaining_methods() {
    let methods = vec!["filter", "exclude", "order_by", "select_related", "prefetch_related"];

    for method in methods {
        let source = format!("orders.{}()", method);

        // Create Order model
        let order_model = DjangoModel {
            name: "Order".to_string(),
            fields: HashMap::new(),
            relations: Vec::new(),
        };

        let mut inferencer = setup_inferencer_with_django(&source, vec![order_model]);

        // Bind `orders` to OrderQuerySet type
        inferencer.bind_type("orders".to_string(), Type::Instance {
            name: "OrderQuerySet".to_string(),
            module: None,
            type_args: vec![],
        });

        // Parse the code
        let mut parser = MultiParser::new().expect("Failed to create parser");
        let parsed = parser.parse(&source, Language::Python).expect("Failed to parse");

        // Find the call expression
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                let expr = child.child(0).unwrap();
                if expr.kind() == "call" {
                    let inferred_type = inferencer.infer_expr(&expr);

                    if let Type::Instance { name, .. } = inferred_type {
                        assert_eq!(name, "OrderQuerySet", "orders.{}() should return OrderQuerySet", method);
                    } else {
                        panic!("orders.{}() should return Instance type, got {:?}", method, inferred_type);
                    }
                    break;
                }
            }
        }
    }
}
