## 1. Core Integration (Tokio Bridge)
- [ ] 1.1 Expose `PyLoop::spawn_from_rust(coroutine) -> PyResult<PyFuture>` in `data-bridge-pyloop`.
- [ ] 1.2 Modify `data-bridge-api::Server` to accept a `PyLoop` handle or access the global one.
- [ ] 1.3 Implement `PythonHandler` struct in `data-bridge-api` that wraps a PyObject (function) and implements the `Handler` trait.
- [ ] 1.4 Implement `call` logic: Convert `Request` -> `PyObject`, spawn task, await result, convert `PyObject` -> `Response`.

## 2. API & Entry Point
- [ ] 2.1 Create `data_bridge.serve(app)` Python entry point.
- [ ] 2.2 Implement `App` builder in Rust (exposed to Python) to collect routes.
- [ ] 2.3 Implement `@app.get`, `@app.post` decorators in Python that register handlers in the Rust `App`.

## 3. Declarative DSL
- [ ] 3.1 Implement `CrudHandler<T>` generic in Rust (where T is a Model wrapper).
- [ ] 3.2 Create `register_crud` PyO3 function.
- [ ] 3.3 Implement Schema Introspection: Extract collection name and fields from Pydantic model.
- [ ] 3.4 Wire up the 5 CRUD operations (List, Get, Create, Update, Delete) to the generic Rust handlers.

## 4. Testing & Verification
- [ ] 4.1 Unit Test: Rust spawning Python task and getting result.
- [ ] 4.2 Integration Test: Full HTTP request hitting a Python handler.
- [ ] 4.3 Benchmark: Compare `uvicorn` vs `data-bridge` (aim for 2x throughput on simple JSON echo).
- [ ] 4.4 Verification: Test CRUD generation with a sample model.
