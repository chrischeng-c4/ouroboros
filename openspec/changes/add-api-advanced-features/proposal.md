# Change: Add Advanced API Features

## Why
To make `data-bridge-api` a complete alternative to FastAPI/Uvicorn, it requires support for real-time communication (WebSocket) and server-sent events (SSE). Additionally, completing the type-safe extractors (`Path<T>`, `Query<T>`) is essential for developer ergonomics and parity with modern frameworks.

## What Changes

### 1. Extractors (`crates/data-bridge-api/src/extractors.rs`)
- Implement `Path<T>::from_request` using `serde` to deserialize from `req.path_params`.
- Implement `Query<T>::from_request` using `serde` to deserialize from `req.query_params`.
- Ensure proper error mapping to HTTP 400/422.

### 2. WebSocket Support
- **Rust (`crates/data-bridge-api/src/websocket.rs`)**:
  - Implement WebSocket handshake and connection handling.
  - Support `accept()`, `send_text()`, `receive_text()`, `send_json()`, `receive_json()`, `close()`.
  - Use `axum::extract::ws` or `tokio-tungstenite` (add dependency).
- **Python (`python/data_bridge/api/websocket.py`)**:
  - Add `WebSocket` class wrapping the Rust implementation.
  - Add `@app.websocket("/path")` decorator.

### 3. Server-Sent Events (SSE)
- **Rust (`crates/data-bridge-api/src/sse.rs`)**:
  - Implement `SseResponse` logic.
  - Support keep-alive and event formatting (`data:`, `event:`, `id:`).
- **Python (`python/data_bridge/api/response.py`)**:
  - Add `EventSourceResponse` class.
  - Support async generator iteration for event streaming.

### 4. Tests
- Add comprehensive integration tests for all new features.
- Complete pending P7-4 tests (Graceful shutdown, Health, Middleware, etc.).

## Impact
- **Affected Specs**: `api-server`
- **Affected Code**:
  - `crates/data-bridge-api/` (New modules: `websocket.rs`, `sse.rs`; Modified: `extractors.rs`, `server.rs`, `python_handler.rs`)
  - `python/data_bridge/api/` (New: `websocket.py`; Modified: `app.py`, `response.py`)
  - `tests/api/` (New integration tests)
