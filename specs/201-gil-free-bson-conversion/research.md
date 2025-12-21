# Research: GIL-Free BSON Conversion

**Feature**: 201-gil-free-bson-conversion
**Date**: 2025-12-20
**Status**: Complete

## Research Questions

### Q1: How to safely release GIL during BSON conversion?

**Decision**: Use two-phase conversion with intermediate representation

**Rationale**:
- PyO3's `py.allow_threads(||{})` releases GIL for closures that don't access Python objects
- BSON types (Document, Bson enum) are `Send + Sync` in Rust - safe for GIL-free processing
- Extract Python data to intermediate Rust types first (minimal GIL time)
- Convert intermediate → BSON with GIL released in async block

**Pattern**:
```rust
// Phase 1: Extract (GIL held, fast)
let items: Vec<(String, PyObject)> = dict.items()...;

// Phase 2: Convert (GIL released)
future_into_py(py, async move {
    let bson_doc = Python::with_gil(|py| {
        py.allow_threads(|| convert_items_to_bson(&items))
    })?;
    // ... MongoDB operation
})
```

**Alternatives Considered**:
- ❌ **Parallel processing with Rayon**: Adds complexity, not needed for single-document operations
- ❌ **Custom PyO3 extension types**: Requires maintaining Python object lifecycle, defeats purpose
- ❌ **Copy all data upfront**: Memory overhead, unnecessary for simple types

**References**:
- PyO3 docs: https://pyo3.rs/v0.24.0/parallelism.html
- Existing pattern in insert_many (Feature 103) for bulk operations

---

### Q2: How to verify GIL is actually released?

**Decision**: Use Python's `sys.getswitchinterval()` and threading to measure contention

**Rationale**:
- GIL release should allow other Python threads to run during conversion
- Measurable via concurrent execution: 100 threads should show <10% latency increase
- Can instrument with `gil-rs` crate for direct GIL state monitoring (development only)

**Testing Approach**:
```python
import threading
import time

def test_gil_release():
    def query_operation():
        start = time.perf_counter()
        User.find_one(User.age == 35)
        return time.perf_counter() - start

    # Sequential baseline
    sequential_time = [query_operation() for _ in range(100)]

    # Concurrent test
    threads = [threading.Thread(target=query_operation) for _ in range(100)]
    start = time.perf_counter()
    [t.start() for t in threads]
    [t.join() for t in threads]
    concurrent_time = time.perf_counter() - start

    # If GIL released, concurrent ~= sequential (ideal: 100x parallelism)
    # Accept <10% overhead due to thread scheduling
    assert concurrent_time < sequential_time * 1.1
```

**Alternatives Considered**:
- ❌ **Python profiling (cProfile)**: Doesn't show GIL hold time directly
- ❌ **Manual instrumentation**: Error-prone, hard to maintain
- ✅ **Concurrent load test**: Simple, verifies real-world behavior

**References**:
- Python threading docs: https://docs.python.org/3/library/threading.html
- gil-rs: https://github.com/PyO3/pyo3/discussions/2093

---

### Q3: How to maintain BSON conversion semantics exactly?

**Decision**: Property-based testing + comprehensive type matrix

**Rationale**:
- Must handle all BSON types: null, bool, int32/64, double, string, binary, array, document, ObjectId, datetime, regex, etc.
- Use existing `py_to_bson()` logic as reference, refactor to extract→convert pattern
- Property-based test: `old_conversion(data) == new_conversion(data)` for all types

**Type Coverage Matrix**:
| Python Type | BSON Type | Test Case | Edge Case |
|-------------|-----------|-----------|-----------|
| None | Null | Simple | - |
| bool | Boolean | True/False | Must check before int (bool subclass of int) |
| int | Int32/Int64 | Range checks | Max int32, max int64, overflow |
| float | Double | Simple | NaN, Inf, -Inf |
| str | String | Unicode | Empty, emoji, null bytes |
| bytes | Binary | Simple | Empty, large (16MB) |
| list | Array | Nested | Empty, mixed types, deep nesting |
| dict | Document | Nested | Empty, circular ref (should error) |
| datetime | DateTime | Timezone | UTC, local, microseconds |
| ObjectId | ObjectId | Valid/invalid | Hex string, bytes, existing ObjectId |

**Testing Strategy**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_conversion_equivalence() {
        let test_cases = generate_all_bson_types();
        for (py_value, expected_bson) in test_cases {
            let old_result = py_dict_to_bson_current(&py_value);
            let new_result = extract_and_convert_nogil(&py_value);
            assert_eq!(old_result, new_result);
        }
    }
}
```

**Alternatives Considered**:
- ❌ **Manual regression testing**: Doesn't scale, easy to miss edge cases
- ❌ **Snapshot testing**: Fragile, hard to reason about failures
- ✅ **Type matrix + property tests**: Comprehensive, maintainable

**References**:
- BSON spec: http://bsonspec.org/spec.html
- MongoDB BSON types: https://www.mongodb.com/docs/manual/reference/bson-types/

---

### Q4: What intermediate representation for extracted data?

**Decision**: `Vec<(String, SerializablePyValue)>` enum

**Rationale**:
- Need to extract Python objects to Rust before GIL release
- Must be `Send + Sync` (no Python references)
- Keep it simple: serialize primitives immediately, defer complex types

**Design**:
```rust
#[derive(Clone, Debug)]
enum SerializablePyValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<SerializablePyValue>),
    Dict(Vec<(String, SerializablePyValue)>),
    ObjectId(String),  // Hex representation
    DateTime(i64),     // Unix timestamp microseconds
}
```

**Extraction** (GIL held, minimal work):
```rust
fn extract_py_value(py: Python, value: &Bound<PyAny>) -> PyResult<SerializablePyValue> {
    if value.is_none() { return Ok(SerializablePyValue::Null); }
    if let Ok(b) = value.extract::<bool>() { return Ok(SerializablePyValue::Bool(b)); }
    // ... etc for all types
}
```

**Conversion** (GIL released):
```rust
fn serializable_to_bson(value: &SerializablePyValue) -> Bson {
    match value {
        SerializablePyValue::Null => Bson::Null,
        SerializablePyValue::Bool(b) => Bson::Boolean(*b),
        // ... etc - no Python objects, pure Rust
    }
}
```

**Alternatives Considered**:
- ❌ **PyObject references**: Not Send/Sync, can't release GIL
- ❌ **serde_json::Value**: Wrong abstraction, loses BSON type info
- ❌ **Direct BSON construction while holding GIL**: Current approach, defeats purpose
- ✅ **Custom enum**: Explicit, type-safe, minimal overhead

**Memory Overhead**: 2x document size worst case (extracted + converted), acceptable per spec constraint SC-007.

---

### Q5: How to handle errors during GIL-released conversion?

**Decision**: Result propagation with Python exception conversion

**Rationale**:
- Rust Result type carries errors through GIL-released section
- Convert to PyErr when re-acquiring GIL
- Same error messages as current implementation (FR-010 requirement)

**Pattern**:
```rust
future_into_py(py, async move {
    let bson_doc = Python::with_gil(|py| {
        py.allow_threads(|| {
            convert_items_to_bson(&items)
                .map_err(|e| ConversionError::from(e))  // Rust error
        })
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;  // Python exception

    // ... MongoDB operation
})
```

**Error Categories**:
1. **Type errors**: Unsupported Python type → PyValueError
2. **Value errors**: Invalid ObjectId, out of range int → PyValueError
3. **Size errors**: Document >16MB → PyValueError (pre-check before conversion)

**Alternatives Considered**:
- ❌ **Panic on error**: Crashes Python process, unacceptable
- ❌ **Silent fallback**: Violates semantic preservation requirement
- ✅ **Result propagation**: Idiomatic Rust, clear error handling

---

### Q6: Performance measurement methodology

**Decision**: Criterion-rs benchmarks + Python pytest-benchmark

**Rationale**:
- Need before/after comparison for 8 operations
- Criterion provides statistical rigor (outlier detection, regression analysis)
- pytest-benchmark integrates with existing Python test suite

**Benchmark Suite**:
```text
benchmarks/
├── rust_benches/           # Criterion benchmarks
│   └── conversion.rs       # Micro-benchmarks for extract/convert
│
└── python_benches/         # pytest-benchmark
    ├── bench_find_one.py   # Existing (baseline comparison)
    ├── bench_update.py     # Existing (baseline comparison)
    └── bench_gil_contention.py  # NEW (concurrent load)
```

**Acceptance Criteria** (from spec):
- find_one: ≤3.5ms (FR-005)
- update_many: ≤150ms (FR-006)
- Concurrent: <10% overhead at 100 threads (SC-003)

**Alternatives Considered**:
- ❌ **Manual timing**: Not statistically rigorous
- ❌ **Python timeit**: Good for micro-benchmarks, not full operation tests
- ✅ **Criterion + pytest-benchmark**: Industry standard, statistically sound

---

## Implementation Phases

Based on research, recommend 3-phase rollout (aligned with spec):

### Phase 1: Core Conversion Utilities (P1)
- Implement `SerializablePyValue` enum
- Write `extract_py_value()` and `serializable_to_bson()`
- Apply to `find_one` and `update_many` (highest impact operations)
- Validate: benchmarks meet targets, all tests pass

### Phase 2: Comprehensive Rollout (P2)
- Apply pattern to remaining 6 operations (find, insert_one, insert_many, update_one, delete_one, delete_many)
- Eliminate `RustDocument.to_dict()` intermediate step
- Validate: no performance regression on any operation

### Phase 3: Cleanup & Optimization (P3)
- Remove operator check in update operations (lines 1616-1620)
- Simplify code paths (single conversion flow)
- Documentation updates

**Risk Mitigation**: Phased approach allows early validation of performance gains and semantic correctness before full rollout.

---

## Open Questions (None)

All research questions resolved. No blockers for implementation.

**Ready for Phase 1: Design & Contracts**
