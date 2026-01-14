# Change: Complete API BaseModel

## Why
To provide a high-performance, standalone Python validation library that replaces Pydantic without requiring external dependencies (other than `ouroboros`). This allows developers to define schemas using standard Python typing (including `Annotated` and `Field`) and enforce them using the existing high-speed Rust validation engine.

## What Changes
- **New Module**: Create `ouroboros.validation` as the standalone home for `BaseModel`, `Field`, and validation logic.
- **Annotated Support**: Update `BaseModel` to support `name: Annotated[str, Field(...)]` syntax (Pydantic v2 style).
- **Rust Integration**: Expose a `validate_value` function from Rust via PyO3 and wire it into `BaseModel.__init__`.
- **Response Models**: Update `api.Router` decorators to accept `response_model` and use it for serialization/validation.
- **Nested Validation**: Ensure `TypeDescriptor` generation handles nested `BaseModel`s recursively.
- **Zero Dependencies**: Ensure no runtime imports of `pydantic` are required (though compatibility shim remains).

## Impact
- **Affected Specs**: `api-server`
- **Affected Code**: 
    - `python/ouroboros/api/models.py` (Refactor/Move)
    - `python/ouroboros/api/type_extraction.py` (Enhance)
    - `crates/data-bridge-api/src/lib.rs` (Expose validation)
    - `crates/data-bridge-api/src/validation.rs` (Enhance if needed)
