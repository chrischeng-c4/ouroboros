//! Example of using the OpenAPI schema generator
//!
//! Run with: cargo run -p ouroboros-api --example openapi_example

use ouroboros_api::openapi::*;
use ouroboros_api::request::HttpMethod;
use ouroboros_api::validation::{TypeDescriptor, StringConstraints, NumericConstraints, FieldDescriptor};

fn main() {
    // Create a new OpenAPI spec
    let mut spec = OpenApiSpec::new("My API", "1.0.0")
        .description("A sample API built with ouroboros-api")
        .server("https://api.example.com", Some("Production server".to_string()))
        .server("http://localhost:8000", Some("Development server".to_string()))
        .tag("users", Some("User management endpoints".to_string()))
        .tag("posts", Some("Blog post endpoints".to_string()));

    // Add a GET /users/{id} endpoint
    spec.add_operation(
        HttpMethod::Get,
        "/users/{id}",
        Operation::new()
            .operation_id("getUser")
            .summary("Get a user by ID")
            .description("Returns detailed information about a specific user")
            .tag("users")
            .parameter(Parameter {
                name: "id".to_string(),
                location: ParameterLocation::Path,
                description: Some("User ID".to_string()),
                required: true,
                deprecated: None,
                schema: Some(Schema::string().format("uuid")),
                example: None,
            })
            .response("200", Response {
                description: "User found".to_string(),
                content: Some({
                    let mut content = std::collections::HashMap::new();
                    content.insert("application/json".to_string(), MediaType {
                        schema: Some(Schema::reference("#/components/schemas/User")),
                        example: None,
                    });
                    content
                }),
            })
            .response("404", Response {
                description: "User not found".to_string(),
                content: None,
            }),
    );

    // Add a POST /users endpoint
    spec.add_operation(
        HttpMethod::Post,
        "/users",
        Operation::new()
            .operation_id("createUser")
            .summary("Create a new user")
            .tag("users")
            .request_body(RequestBody {
                description: Some("User data".to_string()),
                content: {
                    let mut content = std::collections::HashMap::new();
                    content.insert("application/json".to_string(), MediaType {
                        schema: Some(Schema::reference("#/components/schemas/CreateUserRequest")),
                        example: None,
                    });
                    content
                },
                required: true,
            })
            .response("201", Response {
                description: "User created successfully".to_string(),
                content: Some({
                    let mut content = std::collections::HashMap::new();
                    content.insert("application/json".to_string(), MediaType {
                        schema: Some(Schema::reference("#/components/schemas/User")),
                        example: None,
                    });
                    content
                }),
            }),
    );

    // Define schema for User object using TypeDescriptor
    let user_type = TypeDescriptor::Object {
        fields: vec![
            FieldDescriptor {
                name: "id".to_string(),
                type_desc: TypeDescriptor::Uuid,
                required: true,
                default: None,
                description: Some("User unique identifier".to_string()),
            },
            FieldDescriptor {
                name: "email".to_string(),
                type_desc: TypeDescriptor::Email,
                required: true,
                default: None,
                description: Some("User email address".to_string()),
            },
            FieldDescriptor {
                name: "name".to_string(),
                type_desc: TypeDescriptor::String(StringConstraints {
                    min_length: Some(1),
                    max_length: Some(100),
                    pattern: None,
                    format: None,
                }),
                required: true,
                default: None,
                description: Some("User full name".to_string()),
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
                description: Some("User age".to_string()),
            },
            FieldDescriptor {
                name: "created_at".to_string(),
                type_desc: TypeDescriptor::DateTime,
                required: true,
                default: None,
                description: Some("Account creation timestamp".to_string()),
            },
        ],
        additional: None,
    };

    // Convert TypeDescriptor to OpenAPI Schema
    let user_schema = type_descriptor_to_schema(&user_type);
    spec.add_schema("User", user_schema);

    // Define schema for CreateUserRequest
    let create_user_type = TypeDescriptor::Object {
        fields: vec![
            FieldDescriptor {
                name: "email".to_string(),
                type_desc: TypeDescriptor::Email,
                required: true,
                default: None,
                description: Some("User email address".to_string()),
            },
            FieldDescriptor {
                name: "name".to_string(),
                type_desc: TypeDescriptor::String(StringConstraints {
                    min_length: Some(1),
                    max_length: Some(100),
                    pattern: None,
                    format: None,
                }),
                required: true,
                default: None,
                description: Some("User full name".to_string()),
            },
            FieldDescriptor {
                name: "age".to_string(),
                type_desc: TypeDescriptor::Optional(Box::new(TypeDescriptor::Int64(NumericConstraints {
                    minimum: Some(0),
                    maximum: Some(150),
                    exclusive_minimum: None,
                    exclusive_maximum: None,
                    multiple_of: None,
                }))),
                required: false,
                default: None,
                description: Some("User age".to_string()),
            },
        ],
        additional: None,
    };

    let create_user_schema = type_descriptor_to_schema(&create_user_type);
    spec.add_schema("CreateUserRequest", create_user_schema);

    // Output as JSON
    println!("=== OpenAPI JSON ===");
    println!("{}", spec.to_json().unwrap());

    println!("\n\n=== OpenAPI YAML ===");
    println!("{}", spec.to_yaml().unwrap());
}
