# HTTP Server Module

The `server` module provides a production-ready HTTP server built on Hyper 1.0 that wraps the data-bridge-api Router.

## Features

- **Hyper 1.0 Integration**: Built on the latest Hyper async HTTP library
- **Tokio Runtime**: Full async request processing with Tokio
- **GIL-Free Processing**: Follows the two-phase GIL pattern for optimal performance
- **Request Validation**: Integrated with the existing validation layer
- **Graceful Shutdown**: Responds to SIGINT (Ctrl+C) and SIGTERM signals
- **Request Logging**: Built-in request/response logging with tracing
- **Configurable**: Customizable bind address, body size limits, and logging

## Quick Start

```rust
use data_bridge_api::{Router, Server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and configure router
    let mut router = Router::new();
    // ... register routes ...

    // Create server with configuration
    let config = ServerConfig::new("127.0.0.1:8000")
        .max_body_size(10 * 1024 * 1024)  // 10MB
        .logging(true);

    let server = Server::new(router, config);

    // Run until shutdown signal
    server.run().await?;

    Ok(())
}
```

## Configuration

### ServerConfig

The `ServerConfig` struct allows you to customize server behavior:

```rust
let config = ServerConfig::new("0.0.0.0:3000")
    .max_body_size(5 * 1024 * 1024)  // 5MB max body size
    .logging(false);                  // Disable request logging
```

**Options:**

- `bind_addr`: Socket address to bind to (e.g., `"127.0.0.1:8000"`)
- `max_body_size`: Maximum request body size in bytes (default: 10MB)
- `enable_logging`: Enable/disable request logging (default: true)

## Architecture

### Request Flow

1. **Accept Connection**: Hyper accepts TCP connection
2. **Extract Phase** (GIL held in PyO3 context):
   - Parse HTTP request into `SerializableRequest`
   - Extract headers, query params, body
   - Convert to GIL-free representation
3. **Process Phase** (GIL released):
   - Route matching via Router
   - Request validation
   - Handler execution
4. **Response Conversion**:
   - Convert `Response` to Hyper response
   - Serialize body (using sonic-rs for JSON)

### Two-Phase GIL Pattern

The server follows the same pattern as the rest of data-bridge:

```
Python/GIL Context  → Extract (fast) → Process (GIL-free) → Build (fast)
HTTP Context        → Parse (fast)   → Handle (async)     → Serialize (fast)
```

This ensures minimal GIL contention and maximum throughput.

## Content Type Handling

The server automatically processes different content types:

### JSON (`application/json`)

```rust
// Automatically parsed from request body
let body_json = req.body_json().unwrap();
let name = body_json["name"].as_str();
```

### URL-encoded forms (`application/x-www-form-urlencoded`)

```rust
// Parsed into form_data
let form_data = req.form_data().unwrap();
let username = form_data.fields.get("username");
```

### Multipart forms (`multipart/form-data`)

```rust
// Supports both text fields and file uploads
let form_data = req.form_data().unwrap();
for file in &form_data.files {
    println!("Uploaded: {} ({} bytes)", file.filename, file.data.len());
}
```

### Raw bytes

```rust
// For other content types
let body = req.body().unwrap();
```

## Graceful Shutdown

The server handles shutdown signals gracefully:

- **SIGINT (Ctrl+C)**: Typical terminal interrupt
- **SIGTERM**: Standard termination signal (Unix)

When a signal is received:
1. Server stops accepting new connections
2. Existing connections are allowed to complete
3. Server exits cleanly

## Error Handling

The server provides consistent error responses:

```rust
// Route not found → 404
// Validation error → 422
// Handler error → appropriate status code
// Internal error → 500
```

All errors follow the same JSON structure:

```json
{
  "detail": "Error message here"
}
```

## Logging

When logging is enabled, the server logs:

- Request method and path
- Remote client address
- Handler errors
- Server lifecycle events

Example output:

```
Server listening on http://127.0.0.1:8000
GET /users/123 - from 127.0.0.1:54321
POST /users - from 127.0.0.1:54322
Shutdown signal received, stopping server
```

## Example: Complete Server

See `examples/simple_server.rs` for a complete example with:

- Multiple route types (GET, POST)
- Path parameters
- JSON request/response
- Query parameters
- Error handling

Run with:

```bash
cargo run --example simple_server -p data-bridge-api
```

Test with:

```bash
# Root endpoint
curl http://localhost:8000/

# Path parameter
curl http://localhost:8000/hello/World

# JSON POST
curl -X POST http://localhost:8000/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","age":30}'

# Query parameters
curl http://localhost:8000/echo?foo=bar&baz=qux
```

## Performance Considerations

1. **GIL-Free Processing**: All CPU-intensive work happens without holding the GIL
2. **Sonic-rs JSON**: Uses sonic-rs for 3-7x faster JSON serialization
3. **Zero-copy Where Possible**: Minimizes data copying during request/response handling
4. **Async I/O**: Non-blocking I/O throughout the request pipeline
5. **Body Size Limits**: Prevents memory exhaustion from large requests

## Testing

The server module includes comprehensive tests:

```bash
# Run all server tests
cargo test -p data-bridge-api server::tests

# Run specific test
cargo test -p data-bridge-api test_server_creation
```

## Integration with Python

While the server is written in Rust, it's designed to integrate with Python handlers via PyO3:

1. Python function defines request/response schema
2. Rust validates and routes requests
3. Python handler executes business logic
4. Rust serializes and sends response

This provides Python's ease of use with Rust's performance.

## Future Enhancements

Planned features:

- [ ] HTTP/2 support (via hyper's http2 feature)
- [ ] WebSocket support
- [ ] Middleware system (CORS, compression, etc.)
- [ ] Request/response streaming
- [ ] Connection pooling
- [ ] Rate limiting
- [ ] TLS/HTTPS support

## Dependencies

- `hyper 1.0`: Core HTTP implementation
- `hyper-util`: Utilities for Hyper (TokioIo, etc.)
- `http-body-util`: Body utilities
- `tokio`: Async runtime
- `tracing`: Structured logging

## License

MIT OR Apache-2.0
