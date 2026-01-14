# TestServer: Auto-Start Python Applications

## Overview

The `TestServer` class in `data-bridge-test` has been enhanced to support automatic spawning and management of Python web applications (Flask, FastAPI, Django, etc.) as subprocesses for integration testing.

## Features

- **Subprocess Management**: Automatically spawn Python applications as child processes
- **Health Check Polling**: Wait for application readiness before running tests
- **Graceful Shutdown**: Properly terminate subprocesses (SIGTERM → SIGKILL)
- **HTTP Client Integration**: Built-in reqwest client for making requests
- **Flexible Configuration**: Configurable ports, timeouts, and health endpoints

## API

### TestServer.from_app()

Create a TestServer from a Python application:

```python
from data_bridge.test import TestServer

server = TestServer.from_app(
    app_module="myapp.server",      # Python module to import
    app_callable="app",             # Name of the Flask/FastAPI app
    port=18765,                     # Port to bind to
    startup_timeout=10.0,           # Max seconds to wait for startup
    health_endpoint="/health",      # Endpoint to poll for readiness
)

handle = await server.start()       # Spawns subprocess, waits for health
# ... use handle.url to make requests ...
handle.stop()                       # Graceful shutdown
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `app_module` | `str` | Required | Python module to import (e.g., "tests.fixtures.test_app") |
| `app_callable` | `str` | `"app"` | Name of the Flask/FastAPI application instance |
| `port` | `int` | `18765` | Port to bind the server to |
| `startup_timeout` | `float` | `10.0` | Maximum seconds to wait for server startup |
| `health_endpoint` | `str \| None` | `"/health"` | Endpoint to poll for health checks (None = TCP check only) |

### TestServerHandle

The `start()` method returns a `TestServerHandle` with:

- **`url`**: Base URL of the server (e.g., "http://127.0.0.1:18765")
- **`port`**: Actual port number
- **`client`**: HTTP client URL (for now, returns the base URL)
- **`stop()`**: Method to gracefully shutdown the server

## Example: Flask Application

### 1. Create Your Flask App

```python
# myapp/server.py
from flask import Flask, jsonify

app = Flask(__name__)

@app.route("/health")
def health():
    return jsonify({"status": "healthy"})

@app.route("/api/users")
def get_users():
    return jsonify({"users": [{"id": 1, "name": "Alice"}]})

if __name__ == "__main__":
    app.run(host="127.0.0.1", port=18765)
```

### 2. Write Tests

```python
# tests/test_api.py
import pytest
import httpx
from data_bridge.test import TestServer

@pytest.mark.asyncio
async def test_api_integration():
    """Test the Flask API end-to-end."""
    server = TestServer.from_app(
        app_module="myapp.server",
        app_callable="app",
        port=18765,
        health_endpoint="/health",
    )

    handle = await server.start()

    try:
        async with httpx.AsyncClient() as client:
            # Test health endpoint
            response = await client.get(f"{handle.url}/health")
            assert response.status_code == 200
            assert response.json()["status"] == "healthy"

            # Test API endpoint
            response = await client.get(f"{handle.url}/api/users")
            assert response.status_code == 200
            assert len(response.json()["users"]) == 1
    finally:
        handle.stop()
```

### 3. Use with Fixtures

```python
import pytest
import httpx
from data_bridge.test import TestServer

@pytest.fixture(scope="module")
async def api_server():
    """Fixture that starts the API server once for all tests."""
    server = TestServer.from_app(
        app_module="myapp.server",
        app_callable="app",
        port=18765,
        health_endpoint="/health",
    )
    handle = await server.start()
    yield handle
    handle.stop()

@pytest.mark.asyncio
async def test_health(api_server):
    """Test using the fixture."""
    async with httpx.AsyncClient() as client:
        response = await client.get(f"{api_server.url}/health")
        assert response.status_code == 200

@pytest.mark.asyncio
async def test_users(api_server):
    """Another test using the same fixture."""
    async with httpx.AsyncClient() as client:
        response = await client.get(f"{api_server.url}/api/users")
        assert response.status_code == 200
```

## Implementation Details

### Subprocess Spawning

The server spawns the Python application using:

```bash
python3 -c 'from {module} import {callable}; {callable}.run(host="127.0.0.1", port={port})'
```

This works with Flask's `app.run()`, FastAPI's `uvicorn.run(app)`, etc.

### Health Check Logic

1. **With `health_endpoint`**: Polls the HTTP endpoint until it returns 2xx
2. **Without `health_endpoint`**: Attempts TCP connection to the port

Health checks are performed every 100ms until success or timeout.

### Graceful Shutdown

When `stop()` is called:

1. Sends SIGTERM to the subprocess
2. Waits for process to exit
3. If still running after 5 seconds, sends SIGKILL

The `Drop` implementation ensures cleanup even if `stop()` isn't explicitly called.

## Comparison with Existing Functionality

The TestServer supports two modes:

### 1. Axum Static Routes (Original)

```python
server = TestServer()
server.get("/test", {"status": "ok"})
handle = await server.start()
```

**Use case**: Fast, simple mock servers for HTTP client testing.

### 2. Python App Subprocess (New)

```python
server = TestServer.from_app(
    app_module="myapp.server",
    app_callable="app",
    port=18765,
)
handle = await server.start()
```

**Use case**: Integration testing of real Python web applications.

## Running the Example

```bash
# Run the standalone example
uv run python examples/test_server_example.py

# Run the tests
uv run pytest tests/test_test_server.py -v
```

## Requirements

- **Flask** (for example app): `uv pip install flask`
- **httpx** (for async HTTP client): `uv pip install httpx`

## Limitations

- Currently only supports applications with a `.run()` method (Flask, custom apps)
- For FastAPI/Uvicorn, you may need to wrap in a script that calls `uvicorn.run(app)`
- The `client` property currently returns the URL string (future: may return HttpClient instance)

## Future Enhancements

- [ ] Support for FastAPI/Uvicorn directly
- [ ] Return actual `HttpClient` instance from `client` property
- [ ] Capture subprocess stdout/stderr for debugging
- [ ] Support for custom environment variables
- [ ] Support for HTTPS/TLS servers

## Architecture

```
Python Test Code
       ↓
TestServer.from_app(config)
       ↓
[Rust] TestServer::start_python_app()
       ↓
Spawns: python3 -c 'from module import app; app.run(...)'
       ↓
Health Check Loop (HTTP or TCP)
       ↓
Returns TestServerHandle
       ↓
Test makes HTTP requests
       ↓
TestServerHandle.stop() → SIGTERM → subprocess exits
```

## Related Files

- **Rust Implementation**: `crates/data-bridge-test/src/http_server.rs`
- **PyO3 Bindings**: `crates/data-bridge/src/test.rs`
- **Python API**: `python/data_bridge/test/__init__.py`
- **Test Fixture**: `tests/fixtures/test_app.py`
- **Tests**: `tests/test_test_server.py`
- **Example**: `examples/test_server_example.py`
