## 1. Rust Implementation
- [x] 1.1 Expose `validate_python_value` function in `data-bridge-api` via PyO3.
    - Input: `(value: PyObject, type_descriptor: PyObject)`
    - Output: `Result<PyObject, PyErr>` (returns coerced value or raises ValidationError)
- [x] 1.2 Implement `PythonTypeDescriptor` conversion in Rust (or ensure JSON compatibility).
- [x] 1.3 Add `validate_value` to the `ouroboros.api` submodule in `crates/data-bridge/src/lib.rs`.

## 2. Python Implementation
- [x] 2.1 Create `python/ouroboros/validation/` package.
- [x] 2.2 Move `BaseModel` and `Field` from `api/models.py` to `validation/models.py`.
- [x] 2.3 Implement `Annotated[T, Field(...)]` parsing in `_extract_schema`.
- [x] 2.4 Wire up `BaseModel.__init__` to call `ouroboros.api.validate_value`.
- [x] 2.5 Implement recursive `TypeDescriptor` generation for nested models.
- [x] 2.6 Create compatibility shim in `ouroboros.api.models` (re-export from `validation`).

## 3. API Server Integration
- [x] 3.1 Update `extract_handler_meta` in `type_extraction.py` to support new `BaseModel`.
- [x] 3.2 Add `response_model` parameter to route decorators (`@app.get()`, etc.).
- [x] 3.3 Implement response validation/serialization using the specified `response_model`.

## 4. Testing
- [x] 4.1 Create tests for `Annotated` syntax support.
- [x] 4.2 Create tests for Nested Model validation (Python -> Rust -> Python).
- [x] 4.3 Create tests for `response_model` enforcement.
- [x] 4.4 Verify zero `pydantic` dependency (run tests in env without pydantic).
