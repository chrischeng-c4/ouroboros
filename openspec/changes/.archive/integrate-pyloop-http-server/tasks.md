## 1. Core Integration (Tokio Bridge)
- [ ] 1.1 Implement `PyLoop::spawn_python_handler(callable, args) -> impl Future<PyResult<PyObject>>` in `data-bridge-pyloop`.
- [ ] 1.2 Add sync/async detection: check `hasattr("__await__")` for coroutines.
- [ ] 1.3 Implement coroutine execution using `spawn_blocking` (Phase 1 workaround).
- [ ] 1.4 Add oneshot channel for Python → Rust result communication.
- [ ] 1.5 Modify `data-bridge-api::Server` to accept a `PyLoop` handle or access the global singleton.
- [ ] 1.6 Implement `PythonHandler` struct in `data-bridge-api` that wraps a PyObject (function) and implements the `Handler` trait.
- [ ] 1.7 Implement handler dispatch logic: Convert `Request` -> `PyObject`, spawn task, await result, convert `PyObject` -> `Response`.

## 2. API & Entry Point
- [ ] 2.1 Create `data_bridge.serve(app)` Python entry point.
- [ ] 2.2 Implement `App` builder in Rust (exposed to Python) to collect routes.
- [ ] 2.3 Implement `@app.get`, `@app.post` decorators in Python that register handlers in the Rust `App`.

## 3. Declarative DSL
- [ ] 3.1 Implement `CrudHandler<T>` generic in Rust (where T is a Model wrapper).
- [ ] 3.2 Create `register_crud` PyO3 function in `data-bridge/src/api.rs`.
- [ ] 3.3 Implement Schema Introspection: Extract collection name and fields from Pydantic model.
- [ ] 3.4 Handle generic types: `List[T]`, `Dict[K, V]`, `Optional[T]`.
- [ ] 3.5 Wire up the 5 CRUD operations to MongoDB ORM:
  - [ ] 3.5.1 GET /resource - List with query params (limit, skip, filter)
  - [ ] 3.5.2 GET /resource/{id} - Find by ID
  - [ ] 3.5.3 POST /resource - Create with validation
  - [ ] 3.5.4 PUT /resource/{id} - Update with validation
  - [ ] 3.5.5 DELETE /resource/{id} - Delete by ID
- [ ] 3.6 Add MongoDB connection pool integration for declarative handlers.

## 4. Error Handling & Resilience
- [ ] 4.1 Implement `convert_python_error(PyErr) -> Response` with HTTP status mapping.
- [ ] 4.2 Handle common exceptions: HTTPException, ValidationError, DatabaseError.
- [ ] 4.3 Add request timeout support with configurable per-route timeout.
- [ ] 4.4 Implement timeout mechanism using `tokio::time::timeout`.
- [ ] 4.5 Add proper error logging (don't expose stack traces to clients).

## 5. Middleware & Production Features
- [ ] 5.1 Integrate Tower middleware: CORS, Compression, Request Logging.
- [ ] 5.2 Implement Python middleware wrapper (optional, for compatibility).
- [ ] 5.3 Add graceful shutdown coordination between PyLoop and Hyper.
- [ ] 5.4 Implement shutdown sequence: stop accepting → drain connections → stop PyLoop → cleanup.
- [ ] 5.5 Add observability hooks: metrics (request count, latency), tracing integration.

## 6. Testing & Verification
- [ ] 6.1 Unit Tests:
  - [ ] 6.1.1 Rust spawning Python sync function
  - [ ] 6.1.2 Rust spawning Python async coroutine
  - [ ] 6.1.3 oneshot channel communication
  - [ ] 6.1.4 Error propagation from Python to Rust
- [ ] 6.2 Integration Tests:
  - [ ] 6.2.1 Full HTTP request hitting a Python handler
  - [ ] 6.2.2 Declarative CRUD endpoints (all 5 operations)
  - [ ] 6.2.3 Request timeout behavior
  - [ ] 6.2.4 Graceful shutdown
- [ ] 6.3 Benchmarks:
  - [ ] 6.3.1 Declarative CRUD vs FastAPI (target: 30x faster)
  - [ ] 6.3.2 Python handler vs uvicorn (target: 2x faster)
  - [ ] 6.3.3 Memory usage under load (10k concurrent connections)
  - [ ] 6.3.4 Latency distribution (p50, p99, p999)
- [ ] 6.4 Load Testing:
  - [ ] 6.4.1 Sustained load test (1 hour, 1k req/sec)
  - [ ] 6.4.2 Spike test (burst to 10k req/sec)
  - [ ] 6.4.3 Verify no memory leaks
