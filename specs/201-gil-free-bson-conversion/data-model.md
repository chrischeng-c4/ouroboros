# Data Model: GIL-Free BSON Conversion

**Feature**: 201-gil-free-bson-conversion
**Date**: 2025-12-20

## Overview

This feature introduces an intermediate data representation for BSON conversion that enables Global Interpreter Lock (GIL) release. The model consists of Rust types that are `Send + Sync`, allowing safe processing in async contexts without holding the Python GIL.

## Core Entities

### 1. SerializablePyValue (Intermediate Representation)

**Purpose**: Bridge between Python objects and BSON, designed to be extracted from Python with minimal GIL hold time, then converted to BSON with GIL released.

**Type Definition**:
```rust
/// Intermediate representation of Python values extracted for BSON conversion.
/// All variants are Send + Sync, enabling GIL-free processing.
#[derive(Clone, Debug, PartialEq)]
pub enum SerializablePyValue {
    /// Python None → BSON Null
    Null,

    /// Python bool → BSON Boolean
    Bool(bool),

    /// Python int → BSON Int32 or Int64 (range-dependent)
    Int(i64),

    /// Python float → BSON Double
    Float(f64),

    /// Python str → BSON String (UTF-8)
    String(String),

    /// Python bytes → BSON Binary
    Bytes(Vec<u8>),

    /// Python list → BSON Array (recursive)
    List(Vec<SerializablePyValue>),

    /// Python dict → BSON Document (recursive)
    Dict(Vec<(String, SerializablePyValue)>),

    /// Python ObjectId/PydanticObjectId/str (24-char hex) → BSON ObjectId
    ObjectId(String),

    /// Python datetime → BSON DateTime (UTC timestamp in microseconds)
    DateTime(i64),

    /// Python Decimal → BSON Decimal128 (string representation)
    Decimal(String),

    /// Python UUID → BSON Binary (subtype 0x04)
    Uuid([u8; 16]),

    /// Python regex pattern → BSON Regex
    Regex { pattern: String, options: String },
}
```

**Invariants**:
- All String fields are valid UTF-8 (enforced by Rust String type)
- ObjectId strings are 24 hexadecimal characters (validated during extraction)
- DateTime timestamps are microseconds since Unix epoch
- List/Dict nesting depth ≤100 levels (MongoDB limit, enforced during extraction)
- Total serialized size ≤16MB (MongoDB document limit, checked before conversion)

**Lifecycle**:
1. **Extract**: Python object → SerializablePyValue (GIL held, <1ms)
2. **Transfer**: Move to async context (GIL released)
3. **Convert**: SerializablePyValue → BSON (GIL released, bulk of time)
4. **Send**: BSON → MongoDB (GIL released, network I/O)

---

### 2. ConversionContext

**Purpose**: Configuration and state for BSON conversion operations.

**Type Definition**:
```rust
/// Context for BSON conversion operations
#[derive(Clone, Debug)]
pub struct ConversionContext {
    /// Security configuration (collection name validation, etc.)
    pub security_config: SecurityConfig,

    /// Maximum nesting depth for recursive structures (default: 100)
    pub max_depth: usize,

    /// Maximum document size in bytes (default: 16MB)
    pub max_size: usize,

    /// Whether to perform strict type checking
    pub strict_types: bool,
}

impl Default for ConversionContext {
    fn default() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            max_depth: 100,
            max_size: 16 * 1024 * 1024,  // 16MB
            strict_types: true,
        }
    }
}
```

**Usage**: Passed through conversion pipeline, accessed without GIL.

---

### 3. ConversionError

**Purpose**: Error type for conversion failures, compatible with Python exception conversion.

**Type Definition**:
```rust
/// Errors that can occur during BSON conversion
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

impl From<ConversionError> for PyErr {
    fn from(err: ConversionError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}
```

**Error Handling Strategy**:
- Errors detected during extraction (GIL held) → immediate PyErr
- Errors detected during conversion (GIL released) → Result propagated to GIL re-acquisition point
- Same error messages as current implementation (FR-010 requirement)

---

## Data Flow

### Flow 1: Python Dict → BSON Document (Write Path)

```text
┌─────────────────┐
│  Python Dict    │  (GIL held)
│  {"a": 1, ...}  │
└────────┬────────┘
         │ extract_dict_items()
         │ <1ms, GIL held
         ▼
┌─────────────────────────────┐
│  Vec<(String, Serializable  │  (Send + Sync)
│  PyValue)>                  │
│  [("a", Int(1)), ...]       │
└────────┬────────────────────┘
         │ Move to async block
         │ GIL released
         ▼
┌─────────────────┐
│  py.allow_      │  (GIL released)
│  threads(||{    │
│    items_to_    │  Bulk of time here
│    bson()       │
│  })             │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  BSON Document  │  (Send to MongoDB)
│  {a: 1, ...}    │
└─────────────────┘
```

**Performance**:
- Extract phase: <1ms (simple field access)
- Convert phase: 90%+ of time (GIL released)
- Total: Similar to current, but concurrent operations don't block

---

### Flow 2: BSON Document → Python Dict (Read Path)

```text
┌─────────────────┐
│  BSON Document  │  (from MongoDB)
│  {a: 1, ...}    │
└────────┬────────┘
         │ bson_to_serializable()
         │ GIL released
         ▼
┌─────────────────────────────┐
│  Vec<(String, Serializable  │  (Send + Sync)
│  PyValue)>                  │
└────────┬────────────────────┘
         │ Move to GIL context
         ▼
┌─────────────────┐
│  Python::with_  │  (GIL held, <1ms)
│  gil(|py| {     │
│    serializable_│
│    to_py_dict() │
│  })             │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Python Dict    │  (returned to user)
│  {"a": 1, ...}  │
└─────────────────┘
```

**Performance**:
- BSON → Serializable: 90%+ of time (GIL released)
- Serializable → PyDict: <1ms (GIL held)
- Eliminates current `RustDocument.to_dict()` overhead

---

## State Transitions

### Conversion State Machine

```text
[Python Object]
      │
      │ extract (GIL held)
      ▼
[SerializablePyValue] ←── Validation errors → [ConversionError]
      │                                              │
      │ transfer (GIL released)                      │
      ▼                                              │
[Async Context]                                      │
      │                                              │
      │ convert (GIL released)                       │
      ▼                                              │
[BSON] ─── Conversion errors ───────────────────────┘
      │
      │ MongoDB I/O
      ▼
[Database]
```

**States**:
1. **Extracted**: Python data in Rust intermediate form (Send + Sync)
2. **Transferred**: Moved to async context (no GIL)
3. **Converted**: BSON ready for MongoDB
4. **Error**: Conversion failed, error propagated to Python

---

## Validation Rules

### Type-Specific Validation (During Extraction)

| Python Type | Validation | Error Condition |
|-------------|------------|-----------------|
| ObjectId str | Regex `^[0-9a-fA-F]{24}$` | InvalidObjectId |
| int | -2^63 ≤ value < 2^63 | IntegerOverflow |
| datetime | Has UTC timezone or naive | N/A (convert to UTC) |
| dict | Keys are strings | TypeMismatch |
| Nested structures | Depth ≤ 100 | DepthLimitExceeded |
| Document | Total size ≤ 16MB | DocumentTooLarge |

### Security Validation (Preserved from Current)

| Validation | Location | Error |
|------------|----------|-------|
| Collection name | Before conversion | PyValueError |
| Field names (no `$`) | During extraction | PyValueError |
| Query operators | After conversion | PyValueError |

---

## Memory Management

### Memory Overhead Analysis

**Worst Case** (large document with all complex types):
- Original Python dict: ~8MB (example)
- Extracted SerializablePyValue: ~8MB (strings/bytes copied)
- Converted BSON: ~8MB (final representation)
- **Peak**: 16MB (intermediate + final), meets SC-007 constraint (≤2x)

**Typical Case** (small documents):
- Python dict: ~1KB
- SerializablePyValue: ~1KB
- BSON: ~1KB
- **Peak**: 2KB (negligible overhead)

**Optimization Opportunities** (Future Work):
- Zero-copy for bytes/strings when possible (Feature 204)
- Arena allocation for intermediate representation
- Streaming conversion for large arrays

---

## Relationships

### Entity Dependencies

```text
ConversionContext ──uses──> SecurityConfig (from validation.rs)
                 │
                 └──configures──> extract_py_value()
                                  serializable_to_bson()

SerializablePyValue ──recursive──> SerializablePyValue (List, Dict)
                   │
                   └──converts──> BSON

ConversionError ──implements──> std::error::Error
               └──converts──> PyErr
```

### Module Relationships

```text
crates/data-bridge/src/
  ├── conversion.rs (NEW)
  │   ├── SerializablePyValue
  │   ├── ConversionContext
  │   ├── ConversionError
  │   ├── extract_py_value()
  │   ├── items_to_bson()
  │   └── bson_to_serializable()
  │
  ├── mongodb.rs (MODIFIED)
  │   └── Uses conversion.rs for all 8 operations
  │
  └── validation.rs (UNCHANGED)
      └── Provides SecurityConfig to ConversionContext
```

---

## Design Decisions

### Why Enum vs Trait Objects?

**Decision**: Use enum `SerializablePyValue`

**Rationale**:
- Exhaustive pattern matching catches all types
- No vtable overhead (performance critical)
- Clear memory layout (easier to reason about size)
- Simpler error handling (no Box<dyn Error>)

**Trade-off**: Enum is larger in memory (size of largest variant), but acceptable for intermediate representation.

---

### Why Vec<(String, SerializablePyValue)> vs HashMap?

**Decision**: Use `Vec<(String, SerializablePyValue)>` for Dict representation

**Rationale**:
- Preserves insertion order (Python dicts are ordered since 3.7)
- Faster for small documents (<100 fields) - most MongoDB documents
- No hash computation overhead
- Simpler to serialize to BSON (MongoDB documents are ordered)

**Trade-off**: O(n) lookup vs O(1) for HashMap, but n is typically small.

---

### Why String for ObjectId vs [u8; 12]?

**Decision**: Store as `String` (24 hex chars)

**Rationale**:
- Python ObjectId is typically string representation
- Avoids conversion during extraction (fast path)
- Conversion to BSON ObjectId happens once during BSON serialization (GIL released)

**Trade-off**: 24 bytes vs 12 bytes storage, but intermediate representation is short-lived.

---

## Testing Strategy

### Unit Tests (Rust)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_all_types() {
        // Test SerializablePyValue::Null through Regex
    }

    #[test]
    fn test_nested_dict_depth_limit() {
        // Create dict nested 101 levels deep
        // Assert DepthLimitExceeded error
    }

    #[test]
    fn test_document_size_limit() {
        // Create document >16MB
        // Assert DocumentTooLarge error
    }

    #[test]
    fn test_conversion_roundtrip() {
        // Python → Serializable → BSON → Serializable → Python
        // Assert equality
    }
}
```

### Integration Tests (Python)

```python
def test_semantic_equivalence():
    """Old conversion == New conversion for all BSON types"""
    test_cases = [
        {"null": None},
        {"bool": True},
        {"int": 42},
        {"nested": {"a": [1, 2, {"b": 3}]}},
        # ... all type combinations
    ]
    for doc in test_cases:
        # Insert with new implementation, read back
        result = collection.find_one(doc)
        assert result == doc
```

---

## Future Enhancements (Out of Scope)

- **Feature 204: Zero-copy deserialization** - Use Cow<str> instead of String for SerializablePyValue
- **Streaming conversion** - Process large arrays without full materialization
- **Custom allocator** - Arena allocation for intermediate representation to reduce allocation overhead

---

**Status**: ✅ Design complete, ready for contract definition
