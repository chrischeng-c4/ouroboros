# Quick Start: GIL-Free BSON Conversion

**Feature**: 201-gil-free-bson-conversion
**Audience**: Developers implementing this feature
**Time to Read**: 10 minutes

## What This Feature Does

Optimizes BSON conversion in data-bridge by releasing the Python Global Interpreter Lock (GIL) during conversion operations. This enables:
- **2.5x faster** single document queries (find_one)
- **5.4x faster** bulk updates (update_many)
- **True concurrency** - multiple threads don't block each other during database operations

## Key Innovation

**Before** (current implementation):
```rust
fn find_one(py: Python, filter: &PyDict) -> PyResult<...> {
    let filter_doc = py_dict_to_bson(py, filter)?;  // ← GIL held during conversion
    future_into_py(py, async move {
        collection.find_one(filter_doc).await  // ← GIL released here, but too late
    })
}
```

**After** (this feature):
```rust
fn find_one(py: Python, filter: &PyDict) -> PyResult<...> {
    let filter_items = extract_dict_items(py, filter)?;  // ← GIL held <1ms
    future_into_py(py, async move {
        let filter_doc = py.allow_threads(|| {  // ← GIL released during conversion!
            items_to_bson_document(&filter_items)
        })?;
        collection.find_one(filter_doc).await
    })
}
```

## Architecture Overview

```text
┌─────────────────────┐
│   Python Layer      │
│   (GIL held)        │
│   User calls        │
│   User.find_one()   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   PyO3 Boundary     │  Step 1: Extract Python data
│   mongodb.rs        │  (GIL held, <1ms)
│   extract_dict_     │  → Vec<(String, SerializablePyValue)>
│   items()           │
└──────────┬──────────┘
           │ Move to async context
           ▼
┌─────────────────────┐
│   Async Block       │  Step 2: Convert to BSON
│   (GIL released!)   │  (GIL released, ~90% of time)
│   py.allow_threads  │  SerializablePyValue → BSON
│   (|| convert())    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   MongoDB I/O       │  Step 3: Database operation
│   (GIL released)    │  (network I/O)
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   Result to Python  │  Step 4: Convert result
│   (GIL held <1ms)   │  BSON → Python dict
└─────────────────────┘
```

## Implementation Phases

### Phase 1: Core Utilities (Week 1)

**Goal**: Implement conversion utilities and apply to 2 highest-impact operations.

**Tasks**:
1. Create `crates/data-bridge/src/conversion.rs`:
   - `SerializablePyValue` enum (all BSON types)
   - `extract_py_value()` - Python → Intermediate
   - `serializable_to_bson()` - Intermediate → BSON
   - `bson_to_serializable()` - BSON → Intermediate
   - `serializable_to_py_dict()` - Intermediate → Python

2. Refactor `mongodb.rs::find_one()` to use new pattern
3. Refactor `mongodb.rs::update_many()` to use new pattern

4. Write tests:
   - `tests/unit/test_gil_release.py` - GIL verification
   - `tests/integration/test_conversion_semantics.py` - Semantic equivalence
   - Rust unit tests in `conversion.rs`

5. Run benchmarks:
   - `bench_find_one.py` - Should show ≤3.5ms
   - `bench_update.py` - Should show ≤150ms

**Definition of Done**:
- ✅ find_one meets performance target (≤3.5ms)
- ✅ update_many meets performance target (≤150ms)
- ✅ All existing tests pass (313+)
- ✅ GIL release verified via concurrent load test

---

### Phase 2: Comprehensive Rollout (Week 2)

**Goal**: Apply pattern to all 6 remaining operations, eliminate to_dict() overhead.

**Tasks**:
1. Refactor remaining operations:
   - `find()` - Return PyDict directly
   - `insert_one()`
   - `insert_many()`
   - `update_one()`
   - `delete_one()`
   - `delete_many()`

2. Modify `python/data_bridge/_engine.py`:
   - Remove `result.to_dict()` calls
   - Operations return PyDict directly from Rust

3. Add concurrent load test:
   - `tests/mongo/benchmarks/bench_gil_contention.py`
   - Verify <10% overhead at 100 concurrent operations

4. Run full benchmark suite - verify no regressions

**Definition of Done**:
- ✅ All 8 operations use GIL-free conversion
- ✅ No performance regression on any operation
- ✅ Concurrent scalability verified (<10% overhead)

---

### Phase 3: Cleanup & Optimization (Week 3)

**Goal**: Remove technical debt, simplify code paths.

**Tasks**:
1. Remove operator check in update operations:
   - Delete lines 1616-1620 in `mongodb.rs::update_many()`
   - Trust MongoDB to validate operators

2. Eliminate "fast-path" branching:
   - Single code path for all operations
   - Remove validate=True/False flags (always use GIL-free path)

3. Documentation:
   - Update CHANGELOG.md
   - Add performance comparison to README.md
   - Document GIL release pattern in CONTRIBUTING.md

**Definition of Done**:
- ✅ Code simplified (single conversion path)
- ✅ Documentation updated
- ✅ Ready for PR review

---

## Code Locations

### New Files

```
crates/data-bridge/src/
└── conversion.rs                    # ~500 lines, NEW

crates/data-bridge-test/src/
└── gil_monitor.rs                   # ~100 lines, NEW (optional dev tool)

tests/unit/
└── test_gil_release.py              # ~150 lines, NEW

tests/integration/
└── test_conversion_semantics.py    # ~200 lines, NEW

tests/mongo/benchmarks/
└── bench_gil_contention.py          # ~100 lines, NEW
```

### Modified Files

```
crates/data-bridge/src/
├── lib.rs                           # +1 line (mod conversion)
└── mongodb.rs                       # ~200 lines modified (8 operations)

python/data_bridge/
└── _engine.py                       # ~30 lines modified (remove .to_dict() calls)
```

### Testing Files

All existing test files unchanged - they should pass without modification (FR-007).

---

## Development Workflow

### Step 1: Set Up Development Environment

```bash
# Clone and build
git checkout 201-gil-free-bson-conversion
maturin develop  # Build Rust extension

# Run tests to verify baseline
cargo test  # Rust tests
SKIP_INTEGRATION=true uv run pytest  # Python unit tests (no MongoDB)
```

### Step 2: Implement conversion.rs

```bash
# Create new file
touch crates/data-bridge/src/conversion.rs

# Add module to lib.rs
echo "pub mod conversion;" >> crates/data-bridge/src/lib.rs

# Implement types and functions (see contracts/bson-conversion-api.md)
# Start with SerializablePyValue enum
# Then extract_py_value()
# Then serializable_to_bson()
# Test incrementally with cargo test
```

### Step 3: Refactor find_one (Proof of Concept)

```bash
# Modify crates/data-bridge/src/mongodb.rs
# Change find_one to use new pattern (see Usage Pattern 2 in contracts)

# Build and test
maturin develop
uv run pytest tests/unit/test_find.py -v

# Benchmark
MONGODB_URI="mongodb://localhost:27017/bench" \
  uv run python tests/mongo/benchmarks/bench_find_one.py
# Should show ≤3.5ms
```

### Step 4: Refactor update_many

```bash
# Modify update_many similarly
# Build and test
maturin develop
uv run pytest tests/integration/test_update.py -v

# Benchmark
MONGODB_URI="mongodb://localhost:27017/bench" \
  uv run python tests/mongo/benchmarks/bench_update.py
# Should show ≤150ms
```

### Step 5: GIL Verification Test

```python
# tests/unit/test_gil_release.py

import threading
import time
from data_bridge import Document, init

class User(Document):
    name: str
    age: int

def test_concurrent_find_one_no_gil_blocking():
    """Verify GIL is released during find_one"""
    def find_user():
        start = time.perf_counter()
        User.find_one(User.age == 35)
        return time.perf_counter() - start

    # Sequential baseline
    sequential = [find_user() for _ in range(100)]
    sequential_total = sum(sequential)

    # Concurrent test
    times = []
    def worker():
        times.append(find_user())

    threads = [threading.Thread(target=worker) for _ in range(100)]
    start = time.perf_counter()
    [t.start() for t in threads]
    [t.join() for t in threads]
    concurrent_total = time.perf_counter() - start

    # If GIL released, concurrent should be ~similar to sequential
    # Allow 10% overhead for thread scheduling
    assert concurrent_total < sequential_total * 1.1, \
        f"GIL blocking detected: {concurrent_total:.2f}s vs {sequential_total:.2f}s"
```

### Step 6: Semantic Equivalence Test

```python
# tests/integration/test_conversion_semantics.py

import pytest
from data_bridge import Document
from datetime import datetime
from bson import ObjectId, Binary
from decimal import Decimal

class TestDoc(Document):
    pass

@pytest.mark.parametrize("test_value", [
    {"null": None},
    {"bool": True},
    {"int": 42},
    {"float": 3.14},
    {"string": "hello"},
    {"bytes": b"binary"},
    {"list": [1, 2, 3]},
    {"nested": {"a": {"b": {"c": 1}}}},
    {"objectid": ObjectId()},
    {"datetime": datetime.utcnow()},
    {"decimal": Decimal("123.45")},
])
def test_conversion_preserves_semantics(test_value):
    """Verify new conversion produces identical results"""
    doc = TestDoc.insert_one(test_value)
    result = TestDoc.find_one(TestDoc.id == doc.id)
    assert result.to_dict() == test_value
```

---

## Performance Validation

### Benchmark Baseline (2025-12-19)

**Before** (current implementation):
- find_one: 8.904ms
- update_many: 805ms
- Concurrent 100 threads: ~linear slowdown (GIL contention)

**After** (target):
- find_one: ≤3.5ms (2.5x improvement)
- update_many: ≤150ms (5.4x improvement)
- Concurrent 100 threads: <10% overhead (near-linear scaling)

### Running Benchmarks

```bash
# Ensure MongoDB is running on localhost:27017
mongod --port 27017

# Run find_one benchmark
MONGODB_URI="mongodb://localhost:27017/bench" \
  uv run python tests/mongo/benchmarks/bench_find_one.py

# Run update_many benchmark
MONGODB_URI="mongodb://localhost:27017/bench" \
  uv run python tests/mongo/benchmarks/bench_update.py

# Run GIL contention benchmark
MONGODB_URI="mongodb://localhost:27017/bench" \
  uv run python tests/mongo/benchmarks/bench_gil_contention.py
```

### Interpreting Results

```
Find One Benchmark:
  data-bridge: 3.2ms ± 0.5ms  ← Should be ≤3.5ms ✓
  Beanie: 5.4ms ± 0.3ms       ← We should be faster

Update Many Benchmark:
  data-bridge: 145ms ± 10ms   ← Should be ≤150ms ✓
  Beanie: 253ms ± 15ms        ← We should be faster

GIL Contention Benchmark:
  Sequential (100 ops): 320ms
  Concurrent (100 threads): 340ms  ← Should be <352ms (10% overhead) ✓
```

---

## Common Pitfalls

### Pitfall 1: Holding GIL in allow_threads closure

```rust
// ❌ WRONG - py is a Python reference, can't use in allow_threads
let result = py.allow_threads(|| {
    let dict = PyDict::new(py);  // ← ERROR: py not Send
    // ...
});

// ✅ CORRECT - Extract data first, then release GIL
let data = extract_py_value(py, value)?;
let result = py.allow_threads(|| {
    serializable_to_bson(&data)  // ← No Python objects
});
```

### Pitfall 2: Forgetting to propagate errors

```rust
// ❌ WRONG - unwrap() panics, crashes Python
let bson_doc = py.allow_threads(|| {
    items_to_bson_document(&items).unwrap()  // ← Panic!
});

// ✅ CORRECT - Propagate Result
let bson_doc = py.allow_threads(|| {
    items_to_bson_document(&items)
}).map_err(|e| PyValueError::new_err(e.to_string()))?;
```

### Pitfall 3: Not validating during extraction

```rust
// ❌ WRONG - Validation happens during BSON conversion (GIL released)
let items = extract_dict_items(py, dict)?;  // No depth check
let bson_doc = py.allow_threads(|| {
    items_to_bson_document(&items)  // ← Can exceed depth limit here
});

// ✅ CORRECT - Validate during extraction (GIL held, clear errors)
let context = ConversionContext::default();  // max_depth = 100
let items = extract_dict_items(py, dict, &context)?;  // ← Depth checked
let bson_doc = py.allow_threads(|| {
    items_to_bson_document(&items)  // ← Already validated
});
```

---

## Debugging Tips

### Verify GIL is Released

```python
# Run with Python's GIL monitoring
import sys
sys.setswitchinterval(0.001)  # Force frequent GIL checks

# If GIL not released, you'll see threads blocking
# Use threading + time.perf_counter() to measure
```

### Profile Conversion Time

```rust
// Add timing instrumentation (remove before commit)
let start = std::time::Instant::now();
let bson_doc = items_to_bson_document(&items)?;
eprintln!("Conversion took: {:?}", start.elapsed());
```

### Check Memory Usage

```bash
# Use valgrind or heaptrack
heaptrack python -m pytest tests/integration/test_conversion_semantics.py
heaptrack --analyze heaptrack.*.gz

# Should show ≤2x document size peak memory
```

---

## Success Criteria Checklist

Before submitting PR, verify:

- [ ] **FR-005**: find_one completes in ≤3.5ms (run bench_find_one.py)
- [ ] **FR-006**: update_many completes in ≤150ms (run bench_update.py)
- [ ] **FR-007**: All 313+ Python tests pass (run pytest)
- [ ] **FR-008**: GIL released during conversion (run test_gil_release.py)
- [ ] **FR-009**: All BSON types convert correctly (run test_conversion_semantics.py)
- [ ] **FR-010**: Error messages unchanged (manual verification)
- [ ] **FR-011**: Handles 16MB documents (add test case)
- [ ] **FR-012**: No API changes (pytest passes without modifications)
- [ ] **SC-003**: Concurrent operations <10% overhead (run bench_gil_contention.py)
- [ ] **SC-007**: Memory ≤2x document size (heaptrack verification)
- [ ] **cargo test**: All Rust tests pass
- [ ] **cargo clippy**: No warnings

---

## Next Steps After Implementation

1. **Run `/speckit.tasks`** - Generate detailed task breakdown
2. **Follow TDD** - Write tests before implementation (CLAUDE.md principle)
3. **Benchmark early** - Verify performance gains in Phase 1 before Phase 2
4. **Code review** - Use checklist above for self-review before PR

---

**Ready to implement!** Start with Phase 1, Task 1: Create `conversion.rs` module.
