//! Validation system example
//!
//! This example demonstrates how to use the validation system to validate
//! HTTP request parameters (path, query, header, body).

use ouroboros_api::validation::{
    FieldDescriptor, NumericConstraints, ParamLocation, ParamValidator, RequestValidator,
    StringConstraints, TypeDescriptor,
};
use ouroboros_api::request::SerializableValue;
use std::collections::HashMap;

fn main() {
    println!("=== Data Bridge API Validation Example ===\n");

    // Example 1: Simple path parameter validation
    example_path_params();
    println!();

    // Example 2: Query parameter validation with constraints
    example_query_params();
    println!();

    // Example 3: Body validation with nested objects
    example_body_validation();
    println!();

    // Example 4: Email and URL format validation
    example_format_validation();
    println!();

    // Example 5: Numeric constraints
    example_numeric_constraints();
}

fn example_path_params() {
    println!("--- Example 1: Path Parameter Validation ---");

    let mut validator = RequestValidator::new();
    validator.path_params.push(ParamValidator {
        name: "user_id".to_string(),
        location: ParamLocation::Path,
        type_desc: TypeDescriptor::String(Default::default()),
        required: true,
        default: None,
    });

    let mut path_params = HashMap::new();
    path_params.insert("user_id".to_string(), "user-123".to_string());

    match validator.validate(&path_params, &HashMap::new(), &HashMap::new(), None) {
        Ok(validated) => {
            println!("✓ Path params validated successfully");
            println!("  user_id: {:?}", validated.path_params.get("user_id"));
        }
        Err(errors) => {
            println!("✗ Validation failed:");
            for err in &errors.errors {
                println!("  - {}: {}", err.field, err.message);
            }
        }
    }
}

fn example_query_params() {
    println!("--- Example 2: Query Parameter Validation ---");

    let mut validator = RequestValidator::new();

    // Limit parameter: integer, 1-100
    validator.query_params.push(ParamValidator {
        name: "limit".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::Int64(NumericConstraints {
            minimum: Some(1),
            maximum: Some(100),
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        }),
        required: false,
        default: Some(SerializableValue::Int(10)),
    });

    // Search parameter: string, 3-50 characters
    validator.query_params.push(ParamValidator {
        name: "search".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::String(StringConstraints {
            min_length: Some(3),
            max_length: Some(50),
            pattern: None,
            format: None,
        }),
        required: false,
        default: None,
    });

    let mut query_params = HashMap::new();
    query_params.insert("limit".to_string(), SerializableValue::Int(25));
    query_params.insert("search".to_string(), SerializableValue::String("test query".to_string()));

    match validator.validate(&HashMap::new(), &query_params, &HashMap::new(), None) {
        Ok(validated) => {
            println!("✓ Query params validated successfully");
            println!("  limit: {:?}", validated.query_params.get("limit"));
            println!("  search: {:?}", validated.query_params.get("search"));
        }
        Err(errors) => {
            println!("✗ Validation failed:");
            for err in &errors.errors {
                println!("  - {}: {}", err.field, err.message);
            }
        }
    }
}

fn example_body_validation() {
    println!("--- Example 3: Body Validation (Nested Object) ---");

    let mut validator = RequestValidator::new();

    // Define user schema
    validator.body_validator = Some(TypeDescriptor::Object {
        fields: vec![
            FieldDescriptor {
                name: "name".to_string(),
                type_desc: TypeDescriptor::String(StringConstraints {
                    min_length: Some(2),
                    max_length: Some(100),
                    pattern: None,
                    format: None,
                }),
                required: true,
                default: None,
                description: Some("User's full name".to_string()),
            },
            FieldDescriptor {
                name: "email".to_string(),
                type_desc: TypeDescriptor::Email,
                required: true,
                default: None,
                description: Some("User's email address".to_string()),
            },
            FieldDescriptor {
                name: "age".to_string(),
                type_desc: TypeDescriptor::Int64(NumericConstraints {
                    minimum: Some(0),
                    maximum: Some(150),
                    exclusive_minimum: None,
                    exclusive_maximum: None,
                    multiple_of: None,
                }),
                required: false,
                default: None,
                description: Some("User's age".to_string()),
            },
        ],
        additional: None,
    });

    let body = SerializableValue::Object(vec![
        ("name".to_string(), SerializableValue::String("Alice Johnson".to_string())),
        ("email".to_string(), SerializableValue::String("alice@example.com".to_string())),
        ("age".to_string(), SerializableValue::Int(30)),
    ]);

    match validator.validate(&HashMap::new(), &HashMap::new(), &HashMap::new(), Some(&body)) {
        Ok(validated) => {
            println!("✓ Body validated successfully");
            if let Some(SerializableValue::Object(fields)) = &validated.body {
                for (key, value) in fields {
                    println!("  {}: {:?}", key, value);
                }
            }
        }
        Err(errors) => {
            println!("✗ Validation failed:");
            for err in &errors.errors {
                println!("  - {}: {}", err.field, err.message);
            }
        }
    }
}

fn example_format_validation() {
    println!("--- Example 4: Format Validation (Email, URL, UUID) ---");

    let mut validator = RequestValidator::new();

    validator.query_params.push(ParamValidator {
        name: "email".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::Email,
        required: true,
        default: None,
    });

    validator.query_params.push(ParamValidator {
        name: "website".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::Url,
        required: false,
        default: None,
    });

    validator.query_params.push(ParamValidator {
        name: "request_id".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::Uuid,
        required: false,
        default: None,
    });

    let mut query_params = HashMap::new();
    query_params.insert(
        "email".to_string(),
        SerializableValue::String("user@example.com".to_string()),
    );
    query_params.insert(
        "website".to_string(),
        SerializableValue::String("https://example.com".to_string()),
    );
    query_params.insert(
        "request_id".to_string(),
        SerializableValue::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
    );

    match validator.validate(&HashMap::new(), &query_params, &HashMap::new(), None) {
        Ok(validated) => {
            println!("✓ Format validation successful");
            println!("  email: {:?}", validated.query_params.get("email"));
            println!("  website: {:?}", validated.query_params.get("website"));
            println!("  request_id: {:?}", validated.query_params.get("request_id"));
        }
        Err(errors) => {
            println!("✗ Validation failed:");
            for err in &errors.errors {
                println!("  - {}: {}", err.field, err.message);
            }
        }
    }

    // Now try with invalid values
    println!("\n  Testing invalid values:");

    let mut invalid_params = HashMap::new();
    invalid_params.insert(
        "email".to_string(),
        SerializableValue::String("not-an-email".to_string()),
    );

    match validator.validate(&HashMap::new(), &invalid_params, &HashMap::new(), None) {
        Ok(_) => println!("  ✗ Should have failed validation"),
        Err(errors) => {
            println!("  ✓ Correctly rejected invalid email:");
            for err in &errors.errors {
                println!("    - {}: {}", err.field, err.message);
            }
        }
    }
}

fn example_numeric_constraints() {
    println!("--- Example 5: Numeric Constraints ---");

    let mut validator = RequestValidator::new();

    // Price: must be positive, max 2 decimal places (simulated with multiple_of)
    validator.query_params.push(ParamValidator {
        name: "price".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::Float64(NumericConstraints {
            minimum: Some(0.0),
            maximum: Some(999999.99),
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: Some(0.01), // 2 decimal places
        }),
        required: true,
        default: None,
    });

    // Quantity: positive integer
    validator.query_params.push(ParamValidator {
        name: "quantity".to_string(),
        location: ParamLocation::Query,
        type_desc: TypeDescriptor::Int64(NumericConstraints {
            minimum: Some(1),
            maximum: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        }),
        required: true,
        default: None,
    });

    let mut query_params = HashMap::new();
    query_params.insert("price".to_string(), SerializableValue::Float(29.99));
    query_params.insert("quantity".to_string(), SerializableValue::Int(5));

    match validator.validate(&HashMap::new(), &query_params, &HashMap::new(), None) {
        Ok(validated) => {
            println!("✓ Numeric validation successful");
            println!("  price: {:?}", validated.query_params.get("price"));
            println!("  quantity: {:?}", validated.query_params.get("quantity"));
        }
        Err(errors) => {
            println!("✗ Validation failed:");
            for err in &errors.errors {
                println!("  - {}: {}", err.field, err.message);
            }
        }
    }

    // Try invalid values
    println!("\n  Testing invalid numeric values:");

    let mut invalid_params = HashMap::new();
    invalid_params.insert("price".to_string(), SerializableValue::Float(-10.0)); // negative
    invalid_params.insert("quantity".to_string(), SerializableValue::Int(0)); // below minimum

    match validator.validate(&HashMap::new(), &invalid_params, &HashMap::new(), None) {
        Ok(_) => println!("  ✗ Should have failed validation"),
        Err(errors) => {
            println!("  ✓ Correctly rejected invalid values:");
            for err in &errors.errors {
                println!("    - {}: {}", err.field, err.message);
            }
        }
    }
}
