# Core MongoDB Implementation

> Part of [Core MongoDB Engine Documentation](./README.md)

This document provides details on the file structure, key data structures, and implementation patterns used in the core engine.

## File Structure

### `crates/data-bridge-mongodb/` (Pure Rust)
```
src/
├── lib.rs              # Crate root, exports
├── connection.rs       # Connection pooling logic
├── document.rs         # The `Document` trait and impls
├── query.rs            # `QueryBuilder` implementation
└── error.rs            # Crate-specific errors
```

### `crates/data-bridge/` (PyO3 Bindings)
```
src/
├── lib.rs              # Module definition
├── mongodb.rs          # Main binding file (HUGE)
│   ├── struct RustDocument
│   ├── fn extract_py_value
│   ├── fn extracted_to_bson
│   └── fn bson_to_py
├── validation.rs       # Type checking logic
└── config.rs           # Configuration structs
```

## Key Data Structures

### `ExtractedValue` (Enum)
This is the pivotal type that allows us to release the GIL. It mirrors the structure of a generic JSON/BSON document but owns its data in Rust.

```rust
#[derive(Debug, Clone)]
pub enum ExtractedValue {
    None,
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    DateTime(chrono::DateTime<chrono::Utc>),
    ObjectId(bson::oid::ObjectId),
    List(Vec<ExtractedValue>),
    Dict(std::collections::HashMap<String, ExtractedValue>),
    Binary(Vec<u8>),
}
```

**Design Decision**: Why not use `bson::Bson` directly?
1.  **Separation of Concerns**: `ExtractedValue` is purely for data coming *from Python*. `Bson` is for the DB.
2.  **Performance**: We might want to perform validations or transformations on `ExtractedValue` before BSON conversion.
3.  **Flexibility**: Allows us to handle Python-specific types that might not map 1:1 to BSON immediately (though currently they mostly do).

### `RustDocument` (Struct)
The state wrapper for Python instances.

```rust
#[pyclass]
pub struct RustDocument {
    pub inner: bson::Document,
    pub collection: String,
    pub id: Option<bson::oid::ObjectId>,
}
```

## Implementation Patterns

### 1. The "Extraction-Conversion" Pattern
To optimize GIL usage, we split operations into two distinct phases:

```rust
// 1. Extract (Fast, GIL held)
let extracted = extract_py_value(py_obj)?;

// 2. Process (Slow, GIL released)
let result = py.allow_threads(move || {
    let bson = extracted_to_bson(extracted)?;
    // ... DB operations ...
    Ok(result)
});
```

### 2. Builder Pattern for Queries
We use the Builder pattern to construct queries. This ensures valid state at compile time where possible and provides a clean API.

```rust
// Rust Usage
let query = QueryBuilder::new(collection)
    .filter(doc! { "age": { "$gt": 18 } })
    .sort(doc! { "name": 1 })
    .limit(10);
```

### 3. Error Mapping
We use `thiserror` for Rust errors and map them to PyO3 exceptions in `lib.rs` or at the boundary.

```rust
impl From<data_bridge_common::Error> for PyErr {
    fn from(err: data_bridge_common::Error) -> PyErr {
        match err {
            Error::Mongo(e) => PyMongoError::new_err(e.to_string()),
            Error::Validation(e) => PyValueError::new_err(e),
            // ...
        }
    }
}
```

## Future Improvements

1.  **Zero-Copy Deserialization**: Currently, `bson_to_py` creates new Python objects. We could explore using `PyBuffer` protocol for binary data to avoid copies.
2.  **Custom Type Registry**: Allow users to register custom Python<->BSON converters in Rust.
3.  **Connection Multiplexing**: Advanced pooling strategies for high-throughput microservices.
