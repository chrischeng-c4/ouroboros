# ouroboros-api

High-performance API framework with Rust backend, designed as a FastAPI replacement.

## Overview

`ouroboros-api` provides a complete HTTP API framework that combines Rust's performance with Python's ease of use. It follows the ouroboros architecture principle: **Rust handles the heavy lifting (validation, routing, serialization) while Python defines contracts via type hints**.

### FastAPI Comparison

| Feature | FastAPI | ouroboros-api |
|---------|---------|---------------|
| Routing | Starlette | matchit (radix tree) |
| Validation | Pydantic | ouroboros-validation |
| JSON | orjson | sonic-rs (3-7x faster) |
| Concurrency | asyncio | Two-phase GIL pattern |
| WebSocket | Starlette | tokio-tungstenite |
| SSE | Custom | async-stream |

## Quick Start

```rust
use ouroboros_api::{Router, Server, ServerConfig, Response};
use ouroboros_api::request::SerializableValue;

#[tokio::main]
async fn main() {
    // Create router
    let mut router = Router::new();

    // Add routes
    router.get("/", |_req| async {
        Response::json(SerializableValue::String("Hello, World!".to_string()))
    });

    router.get("/users/{id}", |req| async move {
        let id = req.path_param("id").unwrap_or("unknown");
        Response::json(SerializableValue::Object(vec![
            ("id".to_string(), SerializableValue::String(id.to_string())),
        ]))
    });

    // Start server
    let config = ServerConfig::default();
    Server::new(config).serve(router).await.unwrap();
}
```

## Architecture

### Two-Phase GIL Pattern

The framework uses a two-phase approach to maximize concurrency:

1. **Phase 1 (No GIL)**: Rust handles request parsing, validation, routing, and response serialization
2. **Phase 2 (With GIL)**: Python handler executes business logic when needed

```text
Request → [Rust: Parse/Validate] → [Python: Handler] → [Rust: Serialize] → Response
              No GIL needed           GIL acquired        No GIL needed
```

This pattern achieves near-native performance for I/O-bound workloads while preserving Python's flexibility.

## Features

### Core Features

- **Router**: High-performance radix tree routing via `matchit`
- **Handlers**: Support for sync/async handlers with automatic parameter extraction
- **Request/Response**: Type-safe request handling with automatic content negotiation
- **Middleware**: Composable middleware chain with Tower integration

### Validation

- Powered by `ouroboros-validation` for Pydantic-compatible validation
- Pre-compiled validators for zero-overhead request validation
- Comprehensive type support: primitives, nested objects, arrays, unions

### WebSocket & SSE

```rust
use ouroboros_api::{SseEvent, SseStream};

// Server-Sent Events
async fn events() -> SseStream {
    SseStream::new(async_stream::stream! {
        loop {
            yield SseEvent::data("heartbeat");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}
```

### Security

- **JWT**: Full JWT authentication with multiple algorithms (HS256, RS256, ES256)
- **OAuth2**: Password bearer flow support
- **API Keys**: Header, query, and cookie-based API key authentication
- **Cookies**: Secure cookie handling with HMAC signing

### Rate Limiting

- Token bucket and sliding window algorithms
- Tiered rate limiting for different user levels
- Distributed rate limiting support

### Compression

- Automatic response compression (gzip, deflate)
- Configurable compression levels and thresholds

### Content Negotiation

- Accept header parsing and media type matching
- Language negotiation support

### Static Files & Templates

- Static file serving with caching headers
- Template rendering support

### OpenAPI

- Automatic OpenAPI schema generation via `utoipa`

### Observability (Optional)

Enable with the `observability` feature:

```toml
[dependencies]
ouroboros-api = { path = "...", features = ["observability"] }
```

- OpenTelemetry distributed tracing
- Trace context propagation
- OTLP export

## Module Overview

| Module | Description |
|--------|-------------|
| `router` | Radix tree routing with path parameters |
| `handler` | Handler traits and metadata |
| `request` | Request abstraction with parameter extraction |
| `response` | Response builder with multiple formats |
| `middleware` | Middleware chain and common middlewares |
| `validation` | Request validation using ouroboros-validation |
| `extractors` | Type-safe request data extraction |
| `error` | Error types and HTTP status mapping |
| `security` | JWT, OAuth2, and API key authentication |
| `rate_limit` | Rate limiting algorithms |
| `lifecycle` | Startup/shutdown hooks |
| `websocket` | WebSocket connection handling |
| `sse` | Server-Sent Events support |
| `compression` | Response compression |
| `content_negotiation` | Accept header handling |
| `cookies` | Cookie management with signing |
| `static_files` | Static file serving |
| `templates` | Template rendering |
| `upload` | File upload handling |
| `openapi` | OpenAPI schema generation |

## Configuration

### ServerConfig

```rust
use ouroboros_api::ServerConfig;

let config = ServerConfig {
    host: "0.0.0.0".to_string(),
    port: 8000,
    workers: num_cpus::get(),
    keep_alive: Duration::from_secs(75),
    ..Default::default()
};
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `observability` | OpenTelemetry tracing support | off |
| `bson` | MongoDB BSON type support | off |

## Examples

See the `examples/` directory for complete examples:

- `simple_server.rs` - Basic HTTP server
- `validation_example.rs` - Request validation
- `sse_example.rs` - Server-Sent Events
- `form_upload_example.rs` - File uploads
- `openapi_example.rs` - OpenAPI schema generation
- `python_handler_example.rs` - Python integration

## License

MIT OR Apache-2.0
