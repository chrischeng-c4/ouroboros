## 1. Rust Server Implementation
- [x] 1.1 Create `crates/data-bridge-api/src/server.rs` implementing a Hyper service for `Router`.
- [x] 1.2 Implement graceful shutdown and signal handling in Rust server.
- [x] 1.3 Expose `serve()` function in `crates/data-bridge-api/src/lib.rs`.

## 2. Python Handler Invocation (The Bridge)
- [x] 2.1 Implement `py_to_rust_request` conversion (headers, body, query).
- [x] 2.2 Implement `rust_to_py_response` conversion.
- [x] 2.3 Implement the TODO in `crates/data-bridge/src/api.rs`: Acquire GIL, call handler, await coroutine, convert response.
- [x] 2.4 Handle Python exceptions and convert them to 500 or appropriate HTTP error responses.

## 3. Python API & Entry Point
- [x] 3.1 Update `python/data_bridge/api/app.py` to use `_api.ApiApp.serve()` in `run()`.
- [x] 3.2 Implement `__call__` in `App` for ASGI compatibility (adapter to Rust router or pure Python fallback?).

## 4. Testing & Benchmarks
- [ ] 4.1 Add integration test: Python handler returning JSON.
- [ ] 4.2 Add integration test: Path parameters and validation.
- [ ] 4.3 Add benchmark comparing `data-bridge` vs `fastapi+uvicorn`.

---

## Current Status (2026-01-09)

### ✅ Completed
1. **Rust Server** (`crates/data-bridge-api/src/server.rs`):
   - 665 lines, full Hyper 1.0 implementation
   - Graceful shutdown (SIGINT/SIGTERM)
   - Request logging with tracing
   - 12 comprehensive tests

2. **Python Handler Bridge** (`crates/data-bridge/src/api.rs`):
   - Request conversion: Rust → Python dict
   - Response conversion: Python (dict/list/str/bytes) → Rust
   - Handler invocation with PyO3

3. **Python Entry Point** (`python/data_bridge/api/app.py`):
   - Fixed Rust module import: `from data_bridge.data_bridge import api as _api`
   - `run()` method with `use_rust_server=True` (default)
   - `__call__()` for ASGI compatibility (uvicorn fallback)
   - Fixed metadata dict to exclude None values

### 🔧 Current Issue
**Async Handler Support**: Rust handler invocation assumes all handlers are async (coroutines), but needs to support both:
- **Sync handlers**: `def health(request): return {...}`
- **Async handlers**: `async def health(request): return {...}`

**Error encountered**:
```
RuntimeError: no running event loop
```

**Root cause**: `pyo3_async_runtimes::tokio::into_future()` requires Python's asyncio event loop, but we're in a pure Tokio context.

**Solution needed** (`crates/data-bridge/src/api.rs:291-309`):
```rust
// Current: Always treats result as coroutine
let coro_bound = handler_bound.call1((py_args,))?;
let result = pyo3_async_runtimes::tokio::into_future(coro_bound.clone())?.await?;

// Needed: Check if coroutine before awaiting
let result = handler_bound.call1((py_args,))?;
if is_coroutine(&result) {
    // Await async handler
    pyo3_async_runtimes::tokio::into_future(result.clone())?.await?
} else {
    // Return sync handler result directly
    result
}
```

**Files to modify**:
- `crates/data-bridge/src/api.rs` (handler invocation logic)

### 📋 Next Steps
1. Implement sync/async handler detection in `api.rs`
2. Test with both sync and async handlers
3. Complete integration tests (4.1, 4.2)
4. Add FastAPI benchmark (4.3)

### 📁 Modified Files
```
M  Cargo.lock
M  crates/data-bridge-api/Cargo.toml         # Added hyper-util, http-body-util
M  crates/data-bridge-api/src/lib.rs         # Added server module
M  crates/data-bridge-api/src/router.rs      # Router ownership via Arc
M  crates/data-bridge/src/api.rs             # Handler invocation bridge
M  pyproject.toml                            # Already had api feature
M  python/data_bridge/api/app.py             # Fixed import, run(), __call__

A  crates/data-bridge-api/src/server.rs      # NEW: 665 lines
A  crates/data-bridge-api/examples/simple_server.rs
A  openspec/changes/complete-api-python-integration/
A  dev-docs/80-api-server/
```
