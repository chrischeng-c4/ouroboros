//! Simple HTTP server example
//!
//! This example demonstrates how to create a basic HTTP server using ouroboros-api.
//!
//! Run with:
//! ```bash
//! cargo run --example simple_server -p ouroboros-api
//! ```
//!
//! Test with:
//! ```bash
//! curl http://localhost:8000/
//! curl http://localhost:8000/hello/World
//! curl -X POST http://localhost:8000/users -H "Content-Type: application/json" -d '{"name":"Alice","age":30}'
//! ```

use ouroboros_api::{
    handler::HandlerMeta,
    request::SerializableValue,
    validation::RequestValidator,
    Router, Server, ServerConfig,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create router
    let mut router = Router::new();

    // Register routes
    register_routes(&mut router)?;

    // Create server
    let config = ServerConfig::new("127.0.0.1:8000")
        .max_body_size(10 * 1024 * 1024) // 10MB
        .logging(true);

    let server = Server::new(router, config);

    // Run server
    println!("Server running on http://127.0.0.1:8000");
    println!("Try:");
    println!("  curl http://localhost:8000/");
    println!("  curl http://localhost:8000/hello/World");
    println!("  curl -X POST http://localhost:8000/users -H 'Content-Type: application/json' -d '{{\"name\":\"Alice\",\"age\":30}}'");

    server.run().await?;

    Ok(())
}

fn register_routes(router: &mut Router) -> Result<(), Box<dyn std::error::Error>> {
    use ouroboros_api::request::HttpMethod;

    // GET / - Root endpoint
    router.route(
        HttpMethod::Get,
        "/",
        Arc::new(|_req, _validated| {
            Box::pin(async move {
                use ouroboros_api::Response;
                Ok(Response::json(SerializableValue::Object(vec![
                    (
                        "message".to_string(),
                        SerializableValue::String("Welcome to ouroboros-api!".to_string()),
                    ),
                    (
                        "version".to_string(),
                        SerializableValue::String("0.1.0".to_string()),
                    ),
                ])))
            })
        }),
        RequestValidator::new(),
        {
            let mut meta = HandlerMeta::new("root".to_string());
            meta.summary = Some("Root endpoint".to_string());
            meta.description = Some("Returns a welcome message".to_string());
            meta
        },
    )?;

    // GET /hello/:name - Greeting with path parameter
    router.route(
        HttpMethod::Get,
        "/hello/:name",
        Arc::new(|req, _validated| {
            Box::pin(async move {
                use ouroboros_api::Response;

                let name = req
                    .path_param("name")
                    .unwrap_or("Anonymous");

                Ok(Response::json(SerializableValue::Object(vec![(
                    "greeting".to_string(),
                    SerializableValue::String(format!("Hello, {}!", name)),
                )])))
            })
        }),
        RequestValidator::new(),
        {
            let mut meta = HandlerMeta::new("greet".to_string());
            meta.summary = Some("Greet a user".to_string());
            meta.description = Some("Returns a personalized greeting".to_string());
            meta
        },
    )?;

    // POST /users - Create user (with JSON body)
    router.route(
        HttpMethod::Post,
        "/users",
        Arc::new(|req, _validated| {
            Box::pin(async move {
                use ouroboros_api::Response;

                if let Some(body_json) = req.body_json() {
                    // Extract user data
                    let name = body_json["name"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string();
                    let age = body_json["age"].as_i64().unwrap_or(0);

                    Ok(Response::json(SerializableValue::Object(vec![
                        (
                            "message".to_string(),
                            SerializableValue::String("User created".to_string()),
                        ),
                        ("name".to_string(), SerializableValue::String(name)),
                        ("age".to_string(), SerializableValue::Int(age)),
                    ]))
                    .status(201))
                } else {
                    Ok(Response::bad_request("Missing JSON body"))
                }
            })
        }),
        RequestValidator::new(),
        {
            let mut meta = HandlerMeta::new("create_user".to_string());
            meta.summary = Some("Create a new user".to_string());
            meta.description = Some("Creates a new user from JSON body".to_string());
            meta
        },
    )?;

    // GET /echo - Echo query parameters
    router.route(
        HttpMethod::Get,
        "/echo",
        Arc::new(|req, _validated| {
            Box::pin(async move {
                use ouroboros_api::Response;

                // Convert query params to JSON-like structure
                let params: Vec<(String, SerializableValue)> = req
                    .inner
                    .query_params
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                Ok(Response::json(SerializableValue::Object(vec![(
                    "query_params".to_string(),
                    SerializableValue::Object(params),
                )])))
            })
        }),
        RequestValidator::new(),
        HandlerMeta::new("echo".to_string()),
    )?;

    Ok(())
}
