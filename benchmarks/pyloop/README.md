# PyLoop Performance Benchmarks

This directory contains comprehensive performance benchmarks comparing **data-bridge-pyloop** (Rust/Tokio-backed event loop) against Python's standard **asyncio** implementation.

## Quick Start

### Run All Benchmarks

```bash
# From repository root
python benchmarks/pyloop/bench_event_loop.py
```

### Expected Output

```
======================================================================
PyLoop vs Asyncio Performance Benchmark Suite
======================================================================

Warming up...
✓ PyLoop warmup complete
✓ Asyncio warmup complete

Running Callback Scheduling (50k iterations)...
[Results with speedup comparison]

Running Timer Scheduling (5k timers)...
[Results with speedup comparison]

Running Event Loop Overhead (10k iterations)...
[Results with speedup comparison]

✓ All Benchmarks Complete!
```

## Benchmark Suite

### 1. Callback Scheduling Throughput
**File**: `bench_event_loop.py::bench_pyloop_call_soon()`

Measures how many callbacks can be scheduled and executed per second using `call_soon()`.

**Metrics**:
- Operations per second
- Microseconds per callback
- Total duration

**Why it matters**: Core event loop operation that affects all async applications.

### 2. Timer Scheduling Performance
**File**: `bench_event_loop.py::bench_pyloop_timers()`

Measures timer scheduling performance with multiple concurrent timers using `call_later()`.

**Metrics**:
- Timers per second
- Average scheduling time
- Total duration

**Why it matters**: Critical for applications using `asyncio.sleep()` or delayed operations.

### 3. Event Loop Overhead
**File**: `bench_event_loop.py::bench_pyloop_empty_iterations()`

Measures the baseline cost per event loop iteration (empty callback).

**Metrics**:
- Microseconds per iteration
- Total duration

**Why it matters**: Lower overhead means more CPU available for actual work.

## Results Summary

See [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) for detailed analysis.

### Quick Summary (Phase 1-2.5)

| Benchmark | PyLoop | Asyncio | Speedup |
|-----------|--------|---------|---------|
| Callback Scheduling | 684k ops/sec | 70k ops/sec | **9.78x** |
| Timer Scheduling | 202k timers/sec | 339k timers/sec | 0.60x ⚠️ |
| Event Loop Overhead | 1.39 µs | 13.78 µs | **9.95x** |

**Key Findings**:
- 🏆 PyLoop is **9.78x faster** for callback scheduling
- ⚠️ PyLoop is **40% slower** for timer scheduling (optimization needed)
- 🏆 PyLoop has **90% lower** event loop overhead

## Interpreting Results

### Speedup Values

- **>1.0x**: PyLoop is faster than asyncio
- **<1.0x**: PyLoop is slower than asyncio
- **~1.0x**: Equivalent performance

### Performance Metrics

**Throughput** (higher is better):
- Operations per second
- Timers per second

**Latency** (lower is better):
- Microseconds per operation
- Microseconds per iteration

## Benchmark Methodology

### Test Parameters

```python
# Callback scheduling
ITERATIONS = 50000  # Number of callbacks

# Timer scheduling
NUM_TIMERS = 5000  # Number of timers
TIMER_DELAYS = 0-9ms  # Varying delays

# Event loop overhead
ITERATIONS = 10000  # Empty iterations
```

### Warmup Phase

Both implementations run 1,000 warmup iterations to:
- Stabilize JIT compilation
- Populate caches
- Allocate system resources
- Ensure consistent measurements

### Timing

All benchmarks use `time.perf_counter()` for high-precision timing (nanosecond accuracy).

## Adding New Benchmarks

### Benchmark Template

```python
def bench_pyloop_operation(params: int) -> Dict[str, Any]:
    """
    Benchmark PyLoop operation.

    Args:
        params: Benchmark parameters

    Returns:
        Dict with metrics (duration, throughput, latency, etc.)
    """
    from data_bridge.pyloop import PyLoop

    loop = PyLoop()
    # ... benchmark implementation ...
    return results


def bench_asyncio_operation(params: int) -> Dict[str, Any]:
    """Baseline asyncio benchmark."""
    import asyncio

    loop = asyncio.new_event_loop()
    # ... benchmark implementation ...
    loop.close()
    return results
```

### Adding to Main Suite

```python
# In main()
run_benchmark(
    "Operation Name",
    lambda: bench_pyloop_operation(params),
    lambda: bench_asyncio_operation(params)
)
```

## Comparison with Other Implementations

### uvloop

**uvloop** is a popular asyncio replacement using libuv:
- 2-4x faster than asyncio overall
- Excellent I/O performance
- Production-proven

**PyLoop vs uvloop** (estimated):
- Better callback scheduling (9x vs 2-3x)
- Needs timer optimization (target: 1.5-2x)
- Target: 3-5x better overall (after Phase 3)

### Other Event Loops

- **trio**: Different async model (structured concurrency)
- **curio**: Minimal async implementation
- **gevent**: Greenlet-based (not compatible)

PyLoop aims to be a **drop-in asyncio replacement** with better performance.

## Known Limitations

### Phase 1-2.5 Implementation

✅ **Working**:
- Basic event loop operations
- Callback scheduling
- Timer scheduling (functional but not optimized)
- Task creation

⚠️ **Needs Optimization**:
- Timer scheduling (currently 40% slower)
- Coroutine execution (uses placeholder sleep)
- Awaitable integration

❌ **Not Yet Implemented**:
- Full asyncio API compatibility
- Signal handling
- Subprocess support
- Thread pool executor

See [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) for detailed analysis.

## Optimization Roadmap

### Phase 3 (High Priority)
- [ ] Timer optimization (Tokio timer wheel)
- [ ] Coroutine execution optimization
- [ ] Task scheduling improvements
- **Target**: 1.5-2x overall speedup vs asyncio

### Phase 4 (Medium Priority)
- [ ] I/O integration benchmarks
- [ ] Exception handling optimization
- [ ] Memory profiling

### Phase 5+ (Low Priority)
- [ ] Signal handling
- [ ] Subprocess support
- [ ] Advanced features

## CI Integration

### Running in CI

```bash
# Add to CI pipeline
python benchmarks/pyloop/bench_event_loop.py || exit 1
```

### Performance Regression Detection

```bash
# Compare with baseline
python benchmarks/pyloop/bench_event_loop.py > current.txt
diff baseline.txt current.txt || echo "Performance regression detected"
```

## Troubleshooting

### Import Errors

```
ImportError: Failed to import PyLoop from data_bridge native module
```

**Solution**: Build the extension module
```bash
maturin develop --release
```

### Benchmark Failures

```
✗ PyLoop warmup failed: [error]
```

**Solutions**:
1. Check PyLoop is properly installed
2. Verify Tokio runtime initialization
3. Check Python version (3.12+ required)

### Inconsistent Results

**Causes**:
- System load (other processes)
- CPU frequency scaling
- Thermal throttling
- Background tasks

**Solutions**:
1. Close unnecessary applications
2. Run multiple times and average
3. Use dedicated benchmark machine
4. Disable CPU frequency scaling

## Contributing

### Adding Benchmarks

1. Follow the benchmark template above
2. Add both PyLoop and asyncio versions
3. Include comprehensive documentation
4. Update BENCHMARK_RESULTS.md

### Submitting Results

Please include:
- Platform information (OS, CPU, Python version)
- Full benchmark output
- Any anomalies or issues observed
- Hardware specifications

## References

- **PyLoop Source**: `crates/data-bridge-pyloop/src/`
- **Design Doc**: `openspec/changes/implement-data-bridge-pyloop/design.md`
- **Phase Summary**: `PYLOOP_PHASE1_SUMMARY.md`
- **Tokio**: https://tokio.rs/
- **Python asyncio**: https://docs.python.org/3/library/asyncio.html
- **uvloop**: https://github.com/MagicStack/uvloop

## License

Same as data-bridge project.

---

**Last Updated**: 2026-01-12
**PyLoop Version**: 0.1.0 (Phase 1-2.5)
**Benchmark Suite**: v1.0
