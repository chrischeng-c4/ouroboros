# Design: data-bridge-pyloop

## Context
The `data-bridge` platform requires high-performance async execution for mixed Python/Rust workloads. Existing solutions like `uvloop` are optimized for pure Python networking but incur overhead when bridging to Rust's Tokio runtime. Our current "thread-local loop" solution fails to scale and violates Python's single-loop assumption.

## Goals
- **Unified Runtime**: Single `tokio::Runtime` driving both Rust tasks and Python coroutines.
- **Standards Compliance**: Fully implement `asyncio.AbstractEventLoop` interface via PyO3.
- **Zero-Copy**: Pass tasks between Python and Rust without serialization/pickling overhead.
- **GIL Minimization**: Ensure the loop itself (reactor) runs in Rust without holding the GIL, acquiring it only to tick Python coroutines.

## Architecture

### 1. The `PyLoop` Struct (Rust)
A PyO3 class that implements the Python `asyncio.AbstractEventLoop` interface.

```rust
#[pyclass]
struct PyLoop {
    // Handle to the underlying Tokio runtime
    runtime: tokio::runtime::Handle,
    // Task sender for scheduling Python coroutines onto Tokio
    sender: tokio::sync::mpsc::UnboundedSender<PyTask>,
}
```

### 2. Execution Model
The "loop" is actually just a bridge to Tokio.
1.  **`call_soon(callback, *args)`**:
    - Wraps `(callback, args)` into a Rust struct.
    - Spawns a Tokio task that acquires the GIL and executes the callback.
2.  **`create_task(coro)`**:
    - Wraps the Python coroutine in a `RustFuture`.
    - Spawns a Tokio task that polls the coroutine.
    - **Crucial**: When the Python coroutine `awaits` a Rust future, the Rust future releases the GIL, allowing other Tokio tasks to run.

### 3. Integration with `data-bridge-api`
Currently, `api.rs` manages its own loops. This will be replaced:

**Old:**
```rust
thread_local! { static EVENT_LOOP: RefCell<Option<Py<PyAny>>> = ... }
```

**New:**
```rust
// On startup
data_bridge_pyloop::install(); // Sets asyncio event loop policy

// In handlers
// Just use the global loop implicitly via asyncio.get_event_loop()
```

## Migration Plan
1.  **Phase 1**: Build `data-bridge-pyloop` crate with basic `call_soon` and `create_task` support.
2.  **Phase 2**: Create `PyEventLoopPolicy` and register it in Python.
3.  **Phase 3**: Verify compliance with `pytest-asyncio`.
4.  **Phase 4**: Refactor `data-bridge-api` to remove thread-local loops and rely on `pyloop`.

## Risks
- **Coroutine Lifecycle**: Python coroutines expect to be polled in a specific way. Mismatch can cause leaks (seen in Phase 7).
- **Signal Handling**: Handling Ctrl+C correctly in a custom loop is tricky.
- **Blocking**: If a Python callback blocks (non-async), it blocks a Tokio worker thread. We must enforce "no blocking in async" strictly or use `spawn_blocking`.
