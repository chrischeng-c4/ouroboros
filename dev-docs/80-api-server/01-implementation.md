# API Server Implementation Details

## Rust/Python Boundary (PyO3)

The framework relies heavily on PyO3 for bindings. Special care is taken to handle panics and errors:
- **Panic Safety**: All FFI entry points are wrapped in `catch_unwind` to prevent Rust panics from crashing the Python interpreter.
- **Error Propagation**: Rust errors are mapped to Python exceptions (e.g., `ApiError` -> `HTTPException`).

## Serialization with sonic-rs

We utilize `sonic-rs` for JSON serialization, which offers significant performance improvements over standard libraries.
- **Speed**: 3-7x faster than `serde_json`.
- **Direct Bytes**: Writes directly to the HTTP response buffer, avoiding intermediate Python strings.

## Async Runtime Integration

The server runs on the **Tokio** runtime.
- Python `async def` handlers are awaited by the Tokio runtime via PyO3's `coroutine` support.
- Background tasks are spawned on the Tokio runtime, ensuring they don't block the main response path.

## Type Extraction

Type hints in Python (`str`, `int`, `MyModel`) are inspected at startup.
- Metadata is passed to Rust.
- Rust constructs an optimized "Extractor" chain for each route.
- During a request, Rust validates input against these extractors *before* acquiring the GIL to call the Python handler.
