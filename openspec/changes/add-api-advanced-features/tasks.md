## 1. Dependencies & Configuration
- [ ] 1.1 Add `tokio-tungstenite` (or verify `axum` re-exports) to `crates/data-bridge-api/Cargo.toml`.
- [ ] 1.2 Add `async-stream` or similar if needed for SSE iterators.

## 2. Extractors Implementation
- [ ] 2.1 Implement `Path<T>::from_request` in `crates/data-bridge-api/src/extractors.rs`.
- [ ] 2.2 Implement `Query<T>::from_request` in `crates/data-bridge-api/src/extractors.rs`.
- [ ] 2.3 Add unit tests for extractors in `crates/data-bridge-api/src/extractors.rs`.
- [ ] 2.4 Verify integration with `python_handler.rs` (ensure params are passed correctly).

## 3. WebSocket Implementation
- [ ] 3.1 Create `crates/data-bridge-api/src/websocket.rs` with `WebSocketHandler` struct.
- [ ] 3.2 Implement upgrade logic in `crates/data-bridge-api/src/server.rs` (detect `Upgrade: websocket`).
- [ ] 3.3 Create `python/data_bridge/api/websocket.py` with `WebSocket` class.
- [ ] 3.4 Implement `accept`, `receive_text`, `send_text`, `close` methods.
- [ ] 3.5 Add `@app.websocket` decorator logic in `python/data_bridge/api/app.py`.

## 4. SSE Implementation
- [ ] 4.1 Create `crates/data-bridge-api/src/sse.rs` handling `text/event-stream`.
- [ ] 4.2 Create `python/data_bridge/api/response.py` `EventSourceResponse` class.
- [ ] 4.3 Implement async iterator support for streaming events from Python to Rust.

## 5. Testing & Validation
- [ ] 5.1 Add `tests/api/test_websocket.py` (Connect, Send/Receive, Close).
- [ ] 5.2 Add `tests/api/test_sse.py` (Stream events, Keep-alive).
- [ ] 5.3 Implement pending P7-4 tests:
    - [ ] Graceful shutdown tests
    - [ ] Health endpoint tests
    - [ ] Middleware tests
    - [ ] Background tasks tests
    - [ ] File upload tests
- [ ] 5.4 Run full test suite and ensure >80% coverage.
