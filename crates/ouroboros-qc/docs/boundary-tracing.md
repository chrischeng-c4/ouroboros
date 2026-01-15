# PyO3 Boundary Tracing

## Overview

The boundary tracing system provides detailed performance insights into data movement across the Python/Rust boundary in data-bridge. This infrastructure enables developers to identify GIL contention, optimize BSON conversion, and ensure efficient PyO3 operations.

## Architecture

### Four-Phase Model

Every PyO3 operation in data-bridge follows a four-phase pattern:

1. **Extract** (GIL held): Extract Python objects to intermediate representation
   - Minimal Python object access
   - Prepare data for conversion
   - Target: <1ms for 1000 documents

2. **Convert** (GIL released): Convert intermediate to BSON/native Rust types
   - All BSON serialization happens here
   - Uses Rayon for parallelization (≥50 docs)
   - Target: <10ms for 1000 documents

3. **Network** (GIL released): Async I/O operations with MongoDB
   - MongoDB driver operations
   - Fully asynchronous with Tokio
   - Latency depends on network and MongoDB

4. **Materialize** (GIL held): Create Python objects from Rust data
   - Construct Python objects from BSON results
   - Minimal processing, fast conversion
   - Target: <2ms for 1000 documents

## Usage

### Basic Tracing

```rust
use data_bridge_test::BoundaryTracer;

let mut tracer = BoundaryTracer::new("insert_many");

// Phase 1: Extract Python data
tracer.start_extract();
// ... extract Python objects to intermediate Vec
tracer.end_extract();

// Phase 2: Convert to BSON
tracer.start_convert();
tracer.record_gil_release();  // Mark GIL release
// ... convert to BSON in parallel
tracer.end_convert();

// Phase 3: Network I/O
tracer.start_network();
// ... MongoDB operations
tracer.end_network();

// Phase 4: Materialize results
tracer.start_materialize();
// ... create Python objects
tracer.end_materialize();

tracer.set_doc_count(1000);
tracer.set_parallel(true);  // Used Rayon parallelization
let timing = tracer.finish();

println!("{}", timing.format());
```

Output:
```
insert_many (1000 docs, 8.32µs/doc):
  Extract: 823µs (GIL held)
  Convert: 4521µs (GIL released, parallel)
  Network: 2103µs (GIL released)
  Materialize: 875µs (GIL held)
  Total: 8322µs (GIL held 20.4%)
```

### Global Metrics Collection

For aggregating metrics across multiple operations:

```rust
use data_bridge_test::BoundaryMetrics;
use std::sync::Arc;

let metrics = Arc::new(BoundaryMetrics::new());

// In each PyO3 function
{
    let mut tracer = BoundaryTracer::new("find");
    // ... perform operation
    let timing = tracer.finish();
    metrics.record(&timing);
}

// Later, get aggregate statistics
println!("Operations: {}", metrics.operation_count());
println!("Total documents: {}", metrics.doc_count());
println!("GIL releases: {}", metrics.gil_release_count());
println!("Avg extract: {:.2}µs", metrics.avg_extract_us());
println!("Avg convert: {:.2}µs", metrics.avg_convert_us());

// Get complete snapshot
let snapshot = metrics.snapshot();
for (key, value) in snapshot {
    println!("{}: {}", key, value);
}

// Reset metrics
metrics.reset();
```

### Instrumenting PyO3 Functions

Example integration with a typical PyO3 function:

```rust
use pyo3::prelude::*;
use data_bridge_test::BoundaryTracer;

#[pyfunction]
fn insert_many(py: Python, docs: Vec<PyObject>) -> PyResult<Vec<String>> {
    let mut tracer = BoundaryTracer::new("insert_many");

    // Phase 1: Extract (GIL held)
    tracer.start_extract();
    let doc_count = docs.len();
    let py_data = docs.iter()
        .map(|obj| extract_python_data(py, obj))
        .collect::<PyResult<Vec<_>>>()?;
    tracer.end_extract();

    // Phase 2: Convert (GIL released)
    tracer.start_convert();
    tracer.record_gil_release();
    let bson_docs = py.allow_threads(|| {
        convert_to_bson_parallel(&py_data)
    })?;
    tracer.end_convert();

    // Phase 3: Network (GIL released)
    tracer.start_network();
    let insert_result = py.allow_threads(|| {
        runtime().block_on(collection.insert_many(bson_docs))
    })?;
    tracer.end_network();

    // Phase 4: Materialize (GIL held)
    tracer.start_materialize();
    let ids = insert_result.inserted_ids.values()
        .map(|id| id.to_string())
        .collect();
    tracer.end_materialize();

    tracer.set_doc_count(doc_count);
    tracer.set_parallel(doc_count >= 50);  // Rayon threshold

    let timing = tracer.finish();
    eprintln!("{}", timing.format());  // Log for debugging

    Ok(ids)
}
```

## Performance Targets

Based on data-bridge's architecture principles:

### Extract Phase
- **Target**: <1ms for 1000 documents
- **Optimization**: Minimize Python object access
- **GIL**: Held (unavoidable)

### Convert Phase
- **Target**: <10ms for 1000 documents
- **Optimization**: Rayon parallelization for ≥50 docs
- **GIL**: Released (critical!)

### Network Phase
- **Target**: Variable (depends on MongoDB)
- **Optimization**: Async I/O, connection pooling
- **GIL**: Released (critical!)

### Materialize Phase
- **Target**: <2ms for 1000 documents
- **Optimization**: Fast Python object construction
- **GIL**: Held (unavoidable)

### Overall GIL Strategy
- **GIL held time**: <30% of total operation time
- **GIL releases**: ≥2 per operation (convert + network)
- **Parallel threshold**: ≥50 documents

## Interpretation Guide

### Healthy Profile (insert_many, 1000 docs)

```
insert_many (1000 docs, 17.76µs/doc):
  Extract: 823µs (GIL held)         ← Good: <1ms
  Convert: 4521µs (GIL released, parallel)  ← Good: <10ms, parallel
  Network: 10231µs (GIL released)   ← Variable (network dependent)
  Materialize: 1185µs (GIL held)    ← Good: <2ms
  Total: 17760µs (GIL held 11.3%)   ← Excellent: <30%
```

**Analysis**: Ideal profile showing:
- Fast extraction (<1ms)
- Efficient parallel conversion
- Minimal GIL contention (11.3%)
- Network is the bottleneck (expected)

### Problematic Profile (needs optimization)

```
insert_many (1000 docs, 45.32µs/doc):
  Extract: 5231µs (GIL held)        ← BAD: >1ms, too slow
  Convert: 18523µs (GIL released)   ← BAD: >10ms, not parallel?
  Network: 10241µs (GIL released)   ← OK
  Materialize: 11325µs (GIL held)   ← BAD: >2ms, too slow
  Total: 45320µs (GIL held 36.5%)   ← BAD: >30%
```

**Issues**:
1. Extract too slow → Reduce Python object complexity
2. Convert too slow → Missing Rayon parallelization?
3. Materialize too slow → Optimize Python object creation
4. GIL held 36.5% → Too much contention

### Red Flags

- **No GIL releases** (`gil_release_count: 0`): Missing `py.allow_threads()`
- **GIL held >50%**: Serious contention issue
- **Extract >5ms**: Python objects too complex
- **Convert not parallel**: Missing Rayon for large batches
- **Materialize >5ms**: Python object creation bottleneck

## Testing

### Unit Tests

```bash
# Run boundary tracing tests
cargo test -p data-bridge-test performance::boundary

# Run all performance tests
cargo test -p data-bridge-test performance::
```

### Integration Testing

Create a test fixture that instruments all PyO3 operations:

```rust
use data_bridge_test::{BoundaryTracer, BoundaryMetrics};
use std::sync::Arc;

#[test]
fn test_insert_many_boundary_performance() {
    let metrics = Arc::new(BoundaryMetrics::new());

    // Run operation
    let result = insert_many_with_tracing(&metrics, test_docs());

    // Verify performance
    assert_eq!(metrics.operation_count(), 1);
    assert!(metrics.avg_extract_us() < 1000.0, "Extract phase too slow");
    assert!(metrics.avg_convert_us() < 10000.0, "Convert phase too slow");
    assert!(metrics.gil_release_count() >= 2, "Not enough GIL releases");
}
```

## Best Practices

1. **Always trace critical paths**: Insert, update, find operations
2. **Log timing in debug builds**: Use `eprintln!()` for development
3. **Aggregate in production**: Use `BoundaryMetrics` for live monitoring
4. **Set doc_count accurately**: Enables per-document analysis
5. **Record GIL releases**: Critical for identifying contention
6. **Mark parallel operations**: Helps verify Rayon usage

## Future Enhancements

- [ ] PyO3 Python bindings for runtime profiling
- [ ] Automatic alerting for performance regressions
- [ ] Flamegraph integration for phase visualization
- [ ] Historical trend analysis
- [ ] Per-operation type breakdown (insert vs find vs update)

## Related Documentation

- `CLAUDE.md`: Architecture principles and GIL release strategy
- `crates/data-bridge/src/mongodb.rs`: PyO3 boundary implementation
- `benchmarks/bench_comparison.py`: Performance benchmarks

---

**Last Updated**: 2026-01-06
**Maintainer**: data-bridge development team
