# Change: Complete API Python Integration

## Why
Currently, `data-bridge-api` has a high-performance Rust HTTP framework but lacks the critical "bridge" to Python. It cannot execute Python handlers from Rust, lacks a standalone server entry point, and doesn't expose a full ASGI interface. To compete with FastAPI/Uvicorn, we must complete this integration to unlock the 10-20x performance gains of the Rust-based architecture.

## What Changes
- **Python Handler Invocation**: Implement the missing logic in `crates/data-bridge/src/api.rs` to call Python handlers from the Rust router, managing GIL and type conversions.
- **Standalone Rust Server**: Implement a Hyper/Axum-based HTTP server in `crates/data-bridge-api` and expose it to Python via `App.run()`.
- **ASGI Compatibility**: Implement `__call__` on the Python `App` class to support standard ASGI runners (like Uvicorn) as a fallback/compatibility mode.
- **Request/Response Lifecycle**: Complete the end-to-end flow including signal handling and graceful shutdown.

## Impact
- **New Capability**: `api-server` (High-performance Python API Server).
- **Affected Code**:
    - `crates/data-bridge/src/api.rs` (PyO3 bindings)
    - `crates/data-bridge-api/src/server.rs` (New server module)
    - `crates/data-bridge-api/src/lib.rs` (Export server)
    - `python/data_bridge/api/app.py` (App entry point)
