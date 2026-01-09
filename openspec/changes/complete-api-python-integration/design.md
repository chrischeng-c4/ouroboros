# Design: API Server Integration

## Goals
- **Zero-Copy (where possible)**: minimize overhead when passing data between Rust and Python.
- **Safety**: Robust GIL management. Python code must never block the Rust async runtime.
- **DX**: `app.run()` should just work and use the fast Rust server.

## Architecture

### 1. The Server
We use `hyper` directly or via `axum` (likely `hyper` low-level for max control or `axum` if we want easy middlewares). Given the `Router` is custom (matchit), we likely implement a `hyper::service::Service` or use `axum::Router` if we can bridge it.
Since `data-bridge-api` already has `Router` using `matchit`, we should wrap this `Router` in a Hyper Service.

- **Runtime**: Tokio (started by PyO3 or `pyo3-asyncio`).
- **Request Flow**:
    1.  Hyper receives HTTP request.
    2.  Rust Router matches path.
    3.  Validator validates input (Rust-only, fast).
    4.  If valid, `Handler` is called.
    5.  For Python handlers:
        - `pyo3_asyncio` used to await Python coroutine.
        - Request data converted to Python types (Dict/Pydantic-like).
        - GIL held *only* during conversion and Python execution.
        - GIL released during I/O (if Python awaits I/O that releases GIL).

### 2. Python Handler Invocation
The critical section in `crates/data-bridge/src/api.rs`:

```rust
async fn handle_request(...) {
    // 1. Prepare args (GIL required)
    let args = Python::with_gil(|py| { ... });
    
    // 2. Call (GIL required)
    let coro = Python::with_gil(|py| handler.call1(args));
    
    // 3. Await (GIL released during wait, re-acquired for result)
    let result = pyo3_asyncio::tokio::into_future(coro).await?;
    
    // 4. Process result (GIL required)
    let response = Python::with_gil(|py| { ... });
}
```

### 3. ASGI Compatibility
To support `uvicorn app:app`:
- Implement `async def __call__(self, scope, receive, send)` in `App`.
- **Strategy**: 
    - Convert ASGI Scope/Receive to `SerializableRequest`.
    - Call Rust Router (via FFI? or just implement a Python-side router fallback?).
    - Calling Rust Router from Python ASGI is: Python -> Rust -> Python (Handler) -> Rust -> Python (ASGI Send). This is circular and complex.
    - **Simplified Strategy**: For this iteration, if run via ASGI (Uvicorn), `App` uses the `_routes` list stored in Python to route requests purely in Python (using the same handler functions). This won't use the Rust Router or Validation (slower), but provides compatibility.
    - **Preferred**: Users should use `app.run()` which uses the Rust Server.

## Decisions
- **Threading**: Use `pyo3-asyncio` to bridge Tokio and asyncio.
- **Validation**: Rust validation happens *before* Python invocation.
