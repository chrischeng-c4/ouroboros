//! Custom Middleware Example
//!
//! This example demonstrates how to create and use custom middleware
//! in ouroboros-api for cross-cutting concerns like logging, timing,
//! and request modification.
//!
//! Run with:
//! ```bash
//! cargo run --example middleware_example -p ouroboros-api
//! ```

use async_trait::async_trait;
use ouroboros_api::{
    error::ApiResult,
    handler::HandlerMeta,
    middleware::{Middleware, MiddlewareChain},
    request::{HttpMethod, Request, SerializableValue},
    response::Response,
    validation::RequestValidator,
    Router, Server, ServerConfig,
};
use std::sync::Arc;

// ============================================================================
// Custom Middlewares
// ============================================================================

/// Logging middleware - logs request/response details
pub struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn before_request(&self, req: &mut Request) -> ApiResult<()> {
        println!(
            "[LOG] --> {} {}",
            req.inner.method.as_str(),
            req.inner.path
        );
        Ok(())
    }

    async fn after_response(&self, req: &Request, res: &mut Response) -> ApiResult<()> {
        println!(
            "[LOG] <-- {} {} => {}",
            req.inner.method.as_str(),
            req.inner.path,
            res.get_status()
        );
        Ok(())
    }
}

/// Timing middleware - measures request processing time
pub struct TimingMiddleware;

#[async_trait]
impl Middleware for TimingMiddleware {
    async fn before_request(&self, req: &mut Request) -> ApiResult<()> {
        // Store start time in request headers (for demonstration)
        req.inner.headers.insert(
            "x-request-start".to_string(),
            format!("{}", std::time::Instant::now().elapsed().as_nanos()),
        );
        Ok(())
    }

    async fn after_response(&self, _req: &Request, _res: &mut Response) -> ApiResult<()> {
        // In a real implementation, you would calculate the elapsed time
        // and add it to the response headers
        Ok(())
    }
}

/// Request ID middleware - adds unique ID to each request
pub struct RequestIdMiddleware;

#[async_trait]
impl Middleware for RequestIdMiddleware {
    async fn before_request(&self, req: &mut Request) -> ApiResult<()> {
        // Generate simple request ID
        let request_id = format!("req_{}", rand_id());
        req.inner.headers.insert("x-request-id".to_string(), request_id);
        Ok(())
    }

    async fn after_response(&self, _req: &Request, _res: &mut Response) -> ApiResult<()> {
        // Response header modification would be done via builder pattern
        // in the actual response creation
        Ok(())
    }
}

/// Simple random ID generator (for demo purposes)
fn rand_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

// ============================================================================
// Middleware Chain Demo
// ============================================================================

fn demonstrate_middleware_chain() {
    println!("Middleware Chain Demonstration");
    println!("===============================\n");

    // Create middleware chain
    let mut chain = MiddlewareChain::new();
    chain.add(Box::new(RequestIdMiddleware));
    chain.add(Box::new(LoggingMiddleware));
    chain.add(Box::new(TimingMiddleware));

    println!("Middleware chain created with:");
    println!("  1. RequestIdMiddleware - Adds unique request ID");
    println!("  2. LoggingMiddleware - Logs requests/responses");
    println!("  3. TimingMiddleware - Tracks request timing");
    println!();
    println!("Execution order:");
    println!("  Request:  1 -> 2 -> 3 -> Handler");
    println!("  Response: Handler -> 3 -> 2 -> 1");
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Demonstrate middleware concepts
    demonstrate_middleware_chain();
    println!();

    // Create router
    let mut router = Router::new();

    // Register routes
    router.route(
        HttpMethod::Get,
        "/",
        Arc::new(|_req, _validated| {
            Box::pin(async move {
                Ok(Response::json(SerializableValue::Object(vec![
                    ("message".to_string(), SerializableValue::String("Hello from middleware example!".to_string())),
                ])))
            })
        }),
        RequestValidator::new(),
        HandlerMeta::new("root".to_string()),
    )?;

    router.route(
        HttpMethod::Get,
        "/slow",
        Arc::new(|_req, _validated| {
            Box::pin(async move {
                // Simulate slow operation
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                Ok(Response::json(SerializableValue::Object(vec![
                    ("message".to_string(), SerializableValue::String("Slow response".to_string())),
                ])))
            })
        }),
        RequestValidator::new(),
        HandlerMeta::new("slow".to_string()),
    )?;

    // Create server
    let config = ServerConfig::new("127.0.0.1:8000").logging(true);
    let server = Server::new(router, config);

    println!("Middleware Example Server running on http://127.0.0.1:8000");
    println!();
    println!("Try:");
    println!("  curl -v http://localhost:8000/");
    println!("  curl -v http://localhost:8000/slow");

    server.run().await?;
    Ok(())
}
