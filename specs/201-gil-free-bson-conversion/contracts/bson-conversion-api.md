# API Contract: BSON Conversion

**Feature**: 201-gil-free-bson-conversion
**Version**: 1.0.0
**Status**: Draft

## Overview

This document defines the internal Rust API contract for GIL-free BSON conversion. This is an internal interface between PyO3 bindings (`mongodb.rs`) and conversion utilities (`conversion.rs`). The Python-facing API remains unchanged (100% backward compatible).

## Module: `conversion.rs`

### Public Types

#### `SerializablePyValue`

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum SerializablePyValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<SerializablePyValue>),
    Dict(Vec<(String, SerializablePyValue)>),
    ObjectId(String),
    DateTime(i64),
    Decimal(String),
    Uuid([u8; 16]),
    Regex { pattern: String, options: String },
}
```

**Contract**:
- All variants are `Send + Sync`
- ObjectId strings are exactly 24 hexadecimal characters
- DateTime is microseconds since Unix epoch (UTC)
- List and Dict support arbitrary nesting up to depth 100

---

#### `ConversionContext`

```rust
#[derive(Clone, Debug)]
pub struct ConversionContext {
    pub security_config: SecurityConfig,
    pub max_depth: usize,
    pub max_size: usize,
    pub strict_types: bool,
}
```

**Default Values**:
- `max_depth`: 100 (MongoDB limit)
- `max_size`: 16777216 (16MB, MongoDB document limit)
- `strict_types`: true
- `security_config`: Default security configuration

---

#### `ConversionError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Unsupported Python type: {0}")]
    UnsupportedType(String),

    #[error("Invalid ObjectId: {0}")]
    InvalidObjectId(String),

    #[error("Integer out of range: {0}")]
    IntegerOverflow(i64),

    #[error("Document exceeds maximum size: {0} bytes (max: {1})")]
    DocumentTooLarge(usize, usize),

    #[error("Nesting depth exceeds maximum: {0} levels (max: {1})")]
    DepthLimitExceeded(usize, usize),

    #[error("Invalid UTF-8 in string: {0}")]
    InvalidUtf8(String),

    #[error("Circular reference detected at depth {0}")]
    CircularReference(usize),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
}

impl From<ConversionError> for PyErr
```

**Contract**:
- All errors are convertible to `PyErr` (PyValueError)
- Error messages match current implementation exactly
- Thread-safe (Send + Sync)

---

### Public Functions

#### `extract_py_value`

```rust
pub fn extract_py_value(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    context: &ConversionContext,
) -> PyResult<SerializablePyValue>
```

**Purpose**: Extract Python value to intermediate representation.

**Preconditions**:
- GIL is held (py: Python<'_> parameter ensures this)
- value is a valid Python object
- context is initialized

**Postconditions**:
- Returns Ok(SerializablePyValue) on success
- SerializablePyValue is Send + Sync (can be moved to async context)
- Returns Err(PyErr) if type unsupported or validation fails

**Performance**:
- Target: <1ms for typical documents (<100 fields)
- Memory: Allocates copy of strings/bytes (intermediate representation)

**Error Conditions**:
- UnsupportedType: Python type has no BSON equivalent
- InvalidObjectId: String is not 24 hex characters
- DepthLimitExceeded: Nested structure exceeds context.max_depth
- TypeMismatch: Dict key is not a string

**Type Mapping**:
| Python Type | SerializablePyValue | Notes |
|-------------|---------------------|-------|
| None | Null | |
| bool | Bool | Check before int (bool subclass of int) |
| int | Int | Range check: -2^63 to 2^63-1 |
| float | Float | NaN, Inf handled |
| str | String | UTF-8 validated |
| bytes | Bytes | Copy to Vec<u8> |
| list | List | Recursive |
| dict | Dict | Keys must be strings, preserve order |
| datetime | DateTime | Convert to UTC microseconds |
| ObjectId/PydanticObjectId | ObjectId | Hex string validated |
| Decimal | Decimal | String representation |
| UUID | Uuid | 16-byte array |

---

#### `extract_dict_items`

```rust
pub fn extract_dict_items(
    py: Python<'_>,
    dict: &Bound<'_, PyDict>,
    context: &ConversionContext,
) -> PyResult<Vec<(String, SerializablePyValue)>>
```

**Purpose**: Extract Python dict to key-value pairs (convenience wrapper for `extract_py_value` on dicts).

**Preconditions**:
- GIL is held
- dict is a valid Python dictionary

**Postconditions**:
- Returns Ok(Vec<(String, SerializablePyValue)>) with insertion order preserved
- Returned value is Send + Sync

**Error Conditions**:
- Same as `extract_py_value`
- Additionally: DocumentTooLarge if estimated size exceeds context.max_size

**Performance**:
- O(n) where n is number of dict items
- Target: <1ms for n<100

---

#### `serializable_to_bson`

```rust
pub fn serializable_to_bson(
    value: &SerializablePyValue,
) -> Result<Bson, ConversionError>
```

**Purpose**: Convert intermediate representation to BSON. **Can be called without GIL** (no Python parameters).

**Preconditions**:
- value is a valid SerializablePyValue

**Postconditions**:
- Returns Ok(Bson) on success
- No Python objects referenced (GIL-free)

**Error Conditions**:
- InvalidObjectId: ObjectId string cannot be parsed (should not happen if extract_py_value validated correctly)
- InvalidUtf8: String contains invalid UTF-8 (should not happen, Rust String is UTF-8)

**Performance**:
- O(n) where n is size of value (fields, array elements, nesting)
- This is the bulk of conversion time (90%+)
- Target: ~1μs per field

**Type Mapping**:
| SerializablePyValue | BSON Type | Notes |
|---------------------|-----------|-------|
| Null | Bson::Null | |
| Bool(b) | Bson::Boolean(b) | |
| Int(i) | Bson::Int32 or Bson::Int64 | Range-dependent |
| Float(f) | Bson::Double(f) | |
| String(s) | Bson::String(s) | |
| Bytes(b) | Bson::Binary(Generic, b) | |
| List(items) | Bson::Array | Recursive |
| Dict(pairs) | Bson::Document | Recursive, ordered |
| ObjectId(hex) | Bson::ObjectId | Parse hex string |
| DateTime(micros) | Bson::DateTime | UTC timestamp |
| Decimal(s) | Bson::Decimal128 | Parse string |
| Uuid(bytes) | Bson::Binary(Uuid, bytes) | Subtype 0x04 |
| Regex{pattern, opts} | Bson::RegularExpression | |

---

#### `items_to_bson_document`

```rust
pub fn items_to_bson_document(
    items: &[(String, SerializablePyValue)],
) -> Result<BsonDocument, ConversionError>
```

**Purpose**: Convert key-value pairs to BSON document (convenience wrapper for `serializable_to_bson` on dicts).

**Preconditions**:
- items is a non-empty vector
- All keys are unique (not enforced, last value wins - MongoDB behavior)

**Postconditions**:
- Returns Ok(BsonDocument) with insertion order preserved
- No GIL required

**Performance**:
- O(n) where n is number of items
- Target: ~1μs per field

---

#### `bson_to_serializable`

```rust
pub fn bson_to_serializable(
    bson: &Bson,
) -> SerializablePyValue
```

**Purpose**: Convert BSON to intermediate representation. **Can be called without GIL**.

**Preconditions**:
- bson is a valid BSON value

**Postconditions**:
- Returns SerializablePyValue (infallible conversion)
- No Python objects referenced

**Performance**:
- O(n) where n is size of BSON value
- Target: ~1μs per field
- This is the bulk of read-path conversion time

**Type Mapping** (inverse of serializable_to_bson):
| BSON Type | SerializablePyValue | Notes |
|-----------|---------------------|-------|
| Bson::Null | Null | |
| Bson::Boolean(b) | Bool(b) | |
| Bson::Int32(i) | Int(i as i64) | |
| Bson::Int64(i) | Int(i) | |
| Bson::Double(f) | Float(f) | |
| Bson::String(s) | String(s) | |
| Bson::Binary(_, b) | Bytes(b) or Uuid (if subtype 0x04) | |
| Bson::Array(items) | List | Recursive |
| Bson::Document(doc) | Dict | Recursive, ordered |
| Bson::ObjectId(oid) | ObjectId(oid.to_hex()) | |
| Bson::DateTime(dt) | DateTime(micros) | UTC timestamp |
| Bson::Decimal128(d) | Decimal(d.to_string()) | |
| Bson::RegularExpression{pattern, opts} | Regex{pattern, options} | |

---

#### `serializable_to_py_dict`

```rust
pub fn serializable_to_py_dict<'py>(
    py: Python<'py>,
    value: &SerializablePyValue,
) -> PyResult<Bound<'py, PyDict>>
```

**Purpose**: Convert intermediate representation to Python dictionary. **Requires GIL**.

**Preconditions**:
- GIL is held
- value is a SerializablePyValue::Dict variant (others will error)

**Postconditions**:
- Returns Ok(PyDict) on success
- Dictionary contains Python objects matching value

**Error Conditions**:
- TypeMismatch: value is not Dict variant
- Python allocation failures (rare, OOM)

**Performance**:
- O(n) where n is number of dict items
- Target: <1ms for n<100
- Allocates Python objects (GIL held, fast)

---

## Usage Patterns

### Pattern 1: Write Path (Python → MongoDB)

```rust
// In mongodb.rs operation functions (e.g., update_many)

fn update_many<'py>(
    py: Python<'py>,
    collection_name: String,
    filter: &Bound<'_, PyDict>,
    update: &Bound<'_, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let validated_name = validate_collection_name(&collection_name)?;
    let conn = get_connection()?;
    let context = ConversionContext::default();

    // Phase 1: Extract (GIL held, <1ms)
    let filter_items = extract_dict_items(py, filter, &context)?;
    let update_items = extract_dict_items(py, update, &context)?;

    // Phase 2: Convert and execute (GIL released)
    future_into_py(py, async move {
        let filter_doc = Python::with_gil(|py| {
            py.allow_threads(|| {
                items_to_bson_document(&filter_items)
                    .map_err(|e| ConversionError::from(e))
            })
        }).map_err(|e: ConversionError| PyValueError::new_err(e.to_string()))?;

        let update_doc = Python::with_gil(|py| {
            py.allow_threads(|| {
                items_to_bson_document(&update_items)
                    .map_err(|e| ConversionError::from(e))
            })
        }).map_err(|e: ConversionError| PyValueError::new_err(e.to_string()))?;

        // Existing security validation
        validate_query_if_enabled(&filter_doc)?;

        // MongoDB operation
        let db = conn.database();
        let collection = db.collection::<BsonDocument>(&validated_name);
        let result = collection.update_many(filter_doc, update_doc).await?;

        Ok(result.modified_count)
    })
}
```

---

### Pattern 2: Read Path (MongoDB → Python)

```rust
// In mongodb.rs operation functions (e.g., find_one)

fn find_one<'py>(
    py: Python<'py>,
    collection_name: String,
    filter: Option<&Bound<'_, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let validated_name = validate_collection_name(&collection_name)?;
    let conn = get_connection()?;
    let context = ConversionContext::default();

    // Extract filter if provided
    let filter_items = match filter {
        Some(f) => Some(extract_dict_items(py, f, &context)?),
        None => None,
    };

    future_into_py(py, async move {
        // Convert filter (GIL released)
        let filter_doc = if let Some(items) = filter_items {
            Some(Python::with_gil(|py| {
                py.allow_threads(|| items_to_bson_document(&items))
            })?)
        } else {
            None
        };

        validate_query_if_enabled(filter_doc.as_ref().unwrap_or(&doc!{}))?;

        // MongoDB operation
        let db = conn.database();
        let collection = db.collection::<BsonDocument>(&validated_name);
        let bson_doc = collection.find_one(filter_doc).await?;

        match bson_doc {
            Some(doc) => {
                // Convert BSON → Serializable (GIL released)
                let serializable = bson_to_serializable(&Bson::Document(doc));

                // Convert Serializable → PyDict (GIL held, <1ms)
                Python::with_gil(|py| {
                    serializable_to_py_dict(py, &serializable)
                        .map(|d| d.into_py(py))
                })
            }
            None => Ok(Python::with_gil(|py| py.None())),
        }
    })
}
```

---

## Performance Guarantees

### Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| extract_py_value | O(n) | n = number of fields + nested elements |
| serializable_to_bson | O(n) | n = size of value |
| bson_to_serializable | O(n) | n = size of BSON |
| serializable_to_py_dict | O(n) | n = number of dict items |

### Memory Complexity

| Operation | Memory | Notes |
|-----------|--------|-------|
| extract_py_value | O(n) | Allocates intermediate representation |
| serializable_to_bson | O(1) | In-place conversion (borrows input) |
| bson_to_serializable | O(n) | Allocates intermediate representation |
| serializable_to_py_dict | O(n) | Allocates Python objects |

**Peak Memory**: ≤2x document size (intermediate + final), per spec constraint SC-007.

---

## Thread Safety

All types and functions are thread-safe (`Send + Sync`) **except**:
- Functions taking `Python<'_>` parameter (require GIL)
- Functions returning PyO3 types (require GIL)

**GIL-free functions** (can be called in `py.allow_threads(||{})`)
- `serializable_to_bson`
- `items_to_bson_document`
- `bson_to_serializable`

**GIL-required functions**:
- `extract_py_value`
- `extract_dict_items`
- `serializable_to_py_dict`

---

## Versioning

**Version**: 1.0.0 (initial implementation)

**Compatibility Promise**:
- Internal API (not exposed to Python users)
- Can change between minor versions if needed
- Breaking changes documented in CHANGELOG

**Stability**:
- Type definitions: Stable (may add variants, won't remove)
- Function signatures: Stable for 1.x series
- Error types: Stable (may add variants)

---

## Testing Contract

### Unit Tests (Rust)

```rust
// In conversion.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_all_bson_types() {
        // Verify extract_py_value handles all Python types
    }

    #[test]
    fn test_roundtrip_equivalence() {
        // Python → Serializable → BSON → Serializable → Python
        // Assert equality
    }

    #[test]
    fn test_depth_limit() {
        // Nested dict 101 levels deep
        // Assert DepthLimitExceeded
    }

    #[test]
    fn test_size_limit() {
        // Document >16MB
        // Assert DocumentTooLarge
    }

    #[test]
    fn test_error_messages() {
        // Verify error messages match current implementation
    }
}
```

### Integration Tests (Python)

```python
# In tests/integration/test_conversion_semantics.py

def test_semantic_equivalence_find_one():
    """New find_one produces same results as old"""
    # Insert document, read with new implementation
    # Assert data integrity

def test_semantic_equivalence_update_many():
    """New update_many produces same results as old"""
    # Update documents, verify final state

def test_all_bson_types():
    """All BSON types convert correctly"""
    # Test matrix of all types
```

---

## Migration Notes

### Breaking Changes from Current Implementation

**None** - This is an internal refactoring. Python API unchanged.

### Performance Changes

**Expected**:
- find_one: 2.5x faster (8.9ms → 3.5ms)
- update_many: 5.4x faster (805ms → 150ms)
- Concurrent operations: Linear scaling (<10% overhead at 100 threads)

**Monitored via**:
- Benchmark suite (bench_find_one.py, bench_update.py)
- GIL contention tests (bench_gil_contention.py)

---

**Status**: ✅ Contract complete, ready for implementation
