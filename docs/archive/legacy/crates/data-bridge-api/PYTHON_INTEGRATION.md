# Python Handler Integration Guide

This guide explains how to integrate Python handlers with the Rust HTTP server using PyLoop.

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│              Python Handler                      │
│  (sync or async function)                       │
└──────────────────┬──────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────┐
│           PythonHandler Wrapper                  │
│  • Converts Request → Python dict              │
│  • Spawns on PyLoop                            │
│  • Converts Result → Response                  │
└──────────────────┬──────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────┐
│              PyLoop (Event Loop)                 │
│  • Rust-backed asyncio loop                     │
│  • Spawns Python on tokio::spawn_blocking       │
│  • Manages GIL efficiently                      │
└──────────────────┬──────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────┐
│            Tokio Runtime (HTTP I/O)              │
│  • Hyper 1.0 HTTP server                        │
│  • Request routing (GIL-free)                   │
│  • Response serialization                       │
└─────────────────────────────────────────────────┘
```

## Quick Start

### 1. Create PyLoop and Router

```rust
use data_bridge_api::{Router, Server, ServerConfig, PythonHandler};
use data_bridge_api::handler::HandlerMeta;
use data_bridge_api::validation::RequestValidator;
use data_bridge_pyloop::PyLoop;
use pyo3::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> PyResult<()> {
    // Create shared PyLoop instance
    let pyloop = Python::with_gil(|py| {
        PyLoop::new().map(Arc::new)
    })?;

    // Create router
    let mut router = Router::new();

    // ... register handlers ...

    // Create server with PyLoop
    let server = Server::with_router(router)
        .with_pyloop(pyloop);

    server.run().await?;
    Ok(())
}
```

### 2. Register Python Handlers

```rust
Python::with_gil(|py| -> PyResult<()> {
    // Define your Python handler
    let handler_fn = py.eval(r#"
def my_handler(request):
    return {
        "status": 200,
        "body": {"message": "Hello!"}
    }
my_handler
"#, None, None)?;

    // Wrap in PythonHandler
    let handler = PythonHandler::new(
        handler_fn.into(),
        pyloop.clone()
    );

    // Register with router
    router.get_python(
        "/api/hello",
        handler,
        RequestValidator::new(),
        HandlerMeta::new("my_handler".to_string()),
    )?;

    Ok(())
})
```

## Request Format

Python handlers receive a dict with the following structure:

```python
{
    "method": "GET",                    # HTTP method
    "path": "/api/users/123",           # Request path
    "url": "http://localhost:8000/api/users/123?page=1",  # Full URL
    "headers": {                        # Headers (lowercase keys)
        "content-type": "application/json",
        "user-agent": "curl/7.68.0"
    },
    "query_params": {                   # Query parameters
        "page": "1",
        "limit": "10"
    },
    "path_params": {                    # Path parameters from route
        "user_id": "123"
    },
    "body": {...},                      # Parsed JSON body (or None)
    "form_data": {                      # Form data (if multipart/form)
        "fields": {"name": "Alice"},
        "files": [...]
    }
}
```

## Response Formats

Python handlers can return responses in three ways:

### Format 1: Dict with status, body, headers

```python
def handler(request):
    return {
        "status": 201,
        "body": {"id": 123, "name": "Alice"},
        "headers": {
            "X-Custom-Header": "Value"
        }
    }
```

### Format 2: Tuple (status_code, body)

```python
def handler(request):
    return (404, {"error": "Not found"})
```

### Format 3: Direct value (assumes 200 OK)

```python
def handler(request):
    return {"data": "hello"}  # Automatically wrapped with status 200
```

## Handler Types

### Sync Handler

```python
def sync_handler(request):
    # Synchronous processing
    user_id = request["path_params"]["user_id"]
    return {"user_id": user_id}
```

### Async Handler

```python
import asyncio

async def async_handler(request):
    # Async processing
    await asyncio.sleep(0.1)
    data = await fetch_from_db(request["path_params"]["id"])
    return {"data": data}
```

Both types are automatically detected and handled appropriately by PyLoop.

## Registration Methods

The Router provides convenience methods for all HTTP methods:

```rust
router.get_python(path, handler, validator, metadata)?;
router.post_python(path, handler, validator, metadata)?;
router.put_python(path, handler, validator, metadata)?;
router.patch_python(path, handler, validator, metadata)?;
router.delete_python(path, handler, validator, metadata)?;
router.route_python(method, path, handler, validator, metadata)?;
```

## Complete Example

```rust
use data_bridge_api::{Router, Server, ServerConfig, PythonHandler};
use data_bridge_api::handler::HandlerMeta;
use data_bridge_api::validation::RequestValidator;
use data_bridge_pyloop::PyLoop;
use pyo3::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create PyLoop
    let pyloop = Python::with_gil(|py| PyLoop::new().map(Arc::new))?;

    // 2. Create Router
    let mut router = Router::new();

    // 3. Register handlers
    Python::with_gil(|py| -> PyResult<()> {
        // GET /api/users/{id}
        let get_user = py.eval(r#"
async def get_user(request):
    user_id = request["path_params"]["user_id"]
    # Fetch from database...
    return {
        "status": 200,
        "body": {"id": user_id, "name": "Alice"}
    }
get_user
"#, None, None)?;

        router.get_python(
            "/api/users/{user_id}",
            PythonHandler::new(get_user.into(), pyloop.clone()),
            RequestValidator::new(),
            HandlerMeta::new("get_user".to_string()),
        )?;

        // POST /api/users
        let create_user = py.eval(r#"
def create_user(request):
    body = request["body"]
    # Save to database...
    return (201, {"id": "new_id", "name": body.get("name")})
create_user
"#, None, None)?;

        router.post_python(
            "/api/users",
            PythonHandler::new(create_user.into(), pyloop.clone()),
            RequestValidator::new(),
            HandlerMeta::new("create_user".to_string()),
        )?;

        Ok(())
    })?;

    // 4. Create and run server
    let config = ServerConfig::new("127.0.0.1:8000");
    let server = Server::new(router, config).with_pyloop(pyloop);

    println!("Server listening on http://127.0.0.1:8000");
    server.run().await?;

    Ok(())
}
```

## Performance Characteristics

### Phase 1 Implementation (Current)

- **Sync handlers**: ~50µs overhead (spawn_blocking)
- **Async handlers**: ~50µs overhead (spawn_blocking + event loop creation)
- **GIL management**: Released during I/O, held only during Python execution
- **Throughput**: Expected 2x faster than uvicorn for simple handlers

### Phase 4 Future Enhancement

- **True async integration**: No spawn_blocking overhead
- **Coroutine awaiting**: Direct integration with PyLoop's event loop
- **Target**: <10µs overhead for async handlers

## Error Handling

Python exceptions are automatically converted to HTTP responses:

```python
def handler(request):
    if not authorized:
        raise Exception("Unauthorized")  # Returns 500
    return {"data": "ok"}
```

For custom status codes, use the response dict format:

```python
def handler(request):
    if not authorized:
        return {"status": 401, "body": {"error": "Unauthorized"}}
    return {"data": "ok"}
```

## Best Practices

1. **Share PyLoop Instance**: Create one PyLoop and share it (Arc) across all handlers
2. **Use Async for I/O**: Async handlers for database/network operations
3. **Keep Handlers Small**: Python execution holds the GIL - keep it brief
4. **Validate in Rust**: Use RequestValidator for schema validation (GIL-free)
5. **Return Structured Responses**: Use dict format for custom headers/status

## Troubleshooting

### Handler not executing

Check that:
- PyLoop is attached to the server: `server.with_pyloop(pyloop)`
- Handler is registered: `router.get_python(...)`
- Path matches: `/api/test` vs `/api/test/`

### GIL-related deadlocks

Ensure:
- PyLoop is created once and shared (Arc)
- No blocking operations in Python handlers
- Use async for long-running operations

### Performance issues

Profile:
- Use sync handlers for CPU-bound work
- Use async handlers for I/O-bound work
- Consider pure Rust handlers for critical paths

## See Also

- [Example: python_handler_example.rs](examples/python_handler_example.rs)
- [PyLoop Documentation](../data-bridge-pyloop/)
