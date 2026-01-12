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

## Why Not Existing Solutions?

### Why Not uvloop?
uvloop is the current market leader (3-5x faster than asyncio), but has critical limitations for data-bridge:

1. **No Rust Integration**: uvloop is written in Cython/C, cannot access Tokio runtime
2. **Single-threaded**: Cannot leverage multiple CPU cores (Python GIL limitation)
3. **Not Composable**: Cannot mix with Rust-native async runtimes
4. **Still requires thread-local loops**: Doesn't solve our core architectural problem

**Benchmark comparison** (expected):
```
uvloop:            ~40,000 req/sec (libuv-based, single-threaded)
data-bridge-pyloop: ~60,000 req/sec (Tokio-based, multicore)
```

### Why Not pyo3-asyncio?
pyo3-asyncio bridges Tokio and asyncio, but uses a **dual event loop architecture**:

```
Python asyncio loop (main)
    ↓
    Spawns Tokio runtime as background thread
    ↓
    Marshaling overhead between loops
```

**Problems**:
1. Two separate event loops running concurrently
2. Marshaling overhead when converting between Python and Rust coroutines
3. Complex state management (two runtime states)
4. data-bridge already runs in Tokio - we need the reverse integration

**data-bridge-pyloop approach** (single event loop):
```
Tokio runtime (main)
    ↓
    Python coroutines execute as Tokio tasks
    ↓
    Zero-copy, single runtime state
```

### Performance Foundation
data-bridge MongoDB ORM already proves this architecture:
- **1.4-5.4x faster than Beanie** (pure Python ORM)
- Zero Python byte handling (all BSON in Rust)
- GIL minimization (released during processing)

Applying the same principles to the event loop should yield similar improvements.

## Risks
- **Coroutine Lifecycle**: Python coroutines expect to be polled in a specific way. Mismatch can cause leaks (seen in Phase 7).
- **Signal Handling**: Handling Ctrl+C correctly in a custom loop is tricky.
- **Blocking**: If a Python callback blocks (non-async), it blocks a Tokio worker thread. We must enforce "no blocking in async" strictly or use `spawn_blocking`.
