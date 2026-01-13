//! Complete example of PyLoop + HTTP Server integration
//!
//! This example demonstrates:
//! 1. Creating a PyLoop instance
//! 2. Registering Python handlers (sync and async)
//! 3. Starting the HTTP server with PyLoop
//! 4. Making requests and seeing responses
//!
//! Run with: cargo run --example python_handler_example

use data_bridge_api::{
    handler::HandlerMeta, python_handler::PythonHandler, validation::RequestValidator, Router,
    Server, ServerConfig,
};
use data_bridge_pyloop::PyLoop;
use pyo3::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== PyLoop + HTTP Server Integration Example ===\n");

    // Step 1: Initialize PyLoop (shared across all handlers)
    println!("1. Creating PyLoop instance...");
    let pyloop = Arc::new(PyLoop::create()?);
    println!("   ✓ PyLoop created\n");

    // Step 2: Create Router
    println!("2. Creating Router...");
    let mut router = Router::new();
    println!("   ✓ Router created\n");

    // Step 3: Register Python handlers
    println!("3. Registering Python handlers...");

    // Example 1: Simple sync handler
    Python::with_gil(|py| -> Result<(), Box<dyn std::error::Error>> {
        #[allow(deprecated)]
        let sync_handler = py.eval_bound(
            c"
def sync_handler(request):
    return {
        'status': 200,
        'body': {
            'message': 'Hello from sync Python handler!',
            'method': request['method'],
            'path': request['path']
        }
    }
sync_handler
",
            None,
            None,
        )?;

        let handler = PythonHandler::new(sync_handler.into(), pyloop.clone());
        router.get_python(
            "/api/sync",
            handler,
            RequestValidator::new(),
            HandlerMeta::new("sync_handler".to_string()),
        )?;
        println!("   ✓ Registered sync handler: GET /api/sync");
        Ok(())
    })?;

    // Example 2: Async handler
    Python::with_gil(|py| -> PyResult<()> {
        let async_handler = py.eval(
            r#"
import asyncio

async def async_handler(request):
    # Simulate async work
    await asyncio.sleep(0.001)
    return {
        "status": 200,
        "body": {
            "message": "Hello from async Python handler!",
            "query_params": request["query_params"]
        }
    }
async_handler
"#,
            None,
            None,
        )?;

        let handler = PythonHandler::new(async_handler.into(), pyloop.clone());
        router.get_python(
            "/api/async",
            handler,
            RequestValidator::new(),
            HandlerMeta::new("async_handler".to_string()),
        )?;
        println!("   ✓ Registered async handler: GET /api/async");
        Ok(())
    })?;

    // Example 3: Handler with path parameters
    Python::with_gil(|py| -> PyResult<()> {
        let path_param_handler = py.eval(
            r#"
def path_param_handler(request):
    user_id = request["path_params"].get("user_id", "unknown")
    return {
        "status": 200,
        "body": {
            "user_id": user_id,
            "message": f"User {user_id} requested"
        }
    }
path_param_handler
"#,
            None,
            None,
        )?;

        let handler = PythonHandler::new(path_param_handler.into(), pyloop.clone());
        router.get_python(
            "/api/users/{user_id}",
            handler,
            RequestValidator::new(),
            HandlerMeta::new("get_user".to_string()),
        )?;
        println!("   ✓ Registered path param handler: GET /api/users/{{user_id}}");
        Ok(())
    })?;

    // Example 4: POST handler with JSON body
    Python::with_gil(|py| -> PyResult<()> {
        let post_handler = py.eval(
            r#"
def post_handler(request):
    body = request.get("body", {})
    return {
        "status": 201,
        "body": {
            "message": "Resource created",
            "received": body
        },
        "headers": {
            "X-Custom-Header": "Created"
        }
    }
post_handler
"#,
            None,
            None,
        )?;

        let handler = PythonHandler::new(post_handler.into(), pyloop.clone());
        router.post_python(
            "/api/resources",
            handler,
            RequestValidator::new(),
            HandlerMeta::new("create_resource".to_string()),
        )?;
        println!("   ✓ Registered POST handler: POST /api/resources");
        Ok(())
    })?;

    // Example 5: Handler returning tuple (status_code, body)
    Python::with_gil(|py| -> PyResult<()> {
        let tuple_handler = py.eval(
            r#"
def tuple_handler(request):
    return (404, {"error": "Not Found", "path": request["path"]})
tuple_handler
"#,
            None,
            None,
        )?;

        let handler = PythonHandler::new(tuple_handler.into(), pyloop.clone());
        router.get_python(
            "/api/notfound",
            handler,
            RequestValidator::new(),
            HandlerMeta::new("not_found_example".to_string()),
        )?;
        println!("   ✓ Registered tuple response handler: GET /api/notfound");
        Ok(())
    })?;

    // Example 6: Handler returning direct value (assumes 200 OK)
    Python::with_gil(|py| -> PyResult<()> {
        let direct_handler = py.eval(
            r#"
def direct_handler(request):
    return {"status": "ok", "timestamp": "2024-01-01T00:00:00Z"}
direct_handler
"#,
            None,
            None,
        )?;

        let handler = PythonHandler::new(direct_handler.into(), pyloop.clone());
        router.get_python(
            "/api/status",
            handler,
            RequestValidator::new(),
            HandlerMeta::new("status".to_string()),
        )?;
        println!("   ✓ Registered direct value handler: GET /api/status\n");
        Ok(())
    })?;

    // Step 4: Create Server with PyLoop
    println!("4. Creating Server with PyLoop...");
    let config = ServerConfig::new("127.0.0.1:8000")
        .max_body_size(10 * 1024 * 1024)
        .logging(true);

    let server = Server::new(router, config).with_pyloop(pyloop);
    println!("   ✓ Server created with PyLoop integration\n");

    // Step 5: Print usage instructions
    println!("=== Server Ready ===");
    println!("Listening on: http://127.0.0.1:8000");
    println!("\nTry these requests:");
    println!("  curl http://127.0.0.1:8000/api/sync");
    println!("  curl http://127.0.0.1:8000/api/async?name=Alice");
    println!("  curl http://127.0.0.1:8000/api/users/123");
    println!("  curl -X POST http://127.0.0.1:8000/api/resources -H 'Content-Type: application/json' -d '{{\"name\":\"test\"}}'");
    println!("  curl http://127.0.0.1:8000/api/notfound");
    println!("  curl http://127.0.0.1:8000/api/status");
    println!("\nPress Ctrl+C to stop\n");

    // Step 6: Run server
    server.run().await?;

    Ok(())
}
