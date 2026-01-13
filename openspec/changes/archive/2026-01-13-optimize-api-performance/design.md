# Architecture Design: API Performance Optimization

## Context
The optimize-api-performance proposal has been extensively implemented with significant architectural changes beyond the original scope. The goal was to solve Python serialization bottlenecks using Rust and achieve performance parity or better with FastAPI.

## Architecture

### Current Architecture (PyLoop Phase 2)
```
Request → Axum/Hyper → Router → PythonHandler 
                                      ↓
                              PyLoop (Tokio runtime)
                                      ↓
                              poll_coroutine() loop
                                      ↓
                              Python coroutine (no asyncio)
                                      ↓
                              Response conversion
```

**Key Principles**:
- ✅ **Single PyLoop event loop**: Follows Python best practices.
- ✅ **Pure Rust async execution**: No Python asyncio dependency.
- ✅ **Native coroutine polling**: No event loop creation overhead.
- ✅ **Proper GIL management**: Acquire only when needed.
- ✅ **Zero Python byte handling**: All BSON/JSON in Rust.

## Evolution & Experiments

### Phase 1-4: Original Optimizations (Success)
- **Changes**: sonic-rs, Zero-copy Bytes, Hyper tuning, Thread-local loop, Lazy PyDict.
- **Result**: Near parity with FastAPI (0.96x - 1.03x).

### Phase 5: PyLoop Migration (Success - Architecture)
- **Changes**: Integrated PyLoop, removed thread-local `asyncio` loops.
- **Result**: Solved architectural violation of multiple event loops.

### Phase 6: Single Global Event Loop (Failed Experiment)
- **Concept**: Single global event loop to align with Python conventions.
- **Result**: -10% to -18% performance regression due to serialization bottleneck and channel overhead.
- **Status**: Reverted.

### Phase 7: pyo3-async-runtimes Integration (Failed Experiment)
- **Concept**: Official pyo3-async-runtimes integration.
- **Result**: -44% to -59% performance regression due to overhead of Future conversion and Tokio scheduling.
- **Status**: Reverted.

### Phase 2 (PyLoop): Native Coroutine Execution (Success - Performance)
- **Problem**: Phase 5 used `asyncio.new_event_loop()` workaround, causing performance regression.
- **Solution**: Implemented native Rust coroutine polling loop.
- **Result**: 1.03x - 1.61x faster than FastAPI.

## Performance Comparison

| Scenario | Phase 4 (Thread-Local) | Phase 2 (PyLoop Native) | vs FastAPI (Final) |
|----------|------------------------|-------------------------|--------------------|
| Plaintext | 999 ops/s | 488 ops/s | **1.03x** |
| JSON | 1,063 ops/s | 553 ops/s | **1.11x** |
| Path Params | 1,006 ops/s | 616 ops/s | **1.61x** |

*Note: Absolute ops/s numbers vary due to test environment differences between phases, but the Ratio vs FastAPI (run in the same environment) is the key metric.*

## Conclusion
The final architecture uses a custom Native Coroutine Polling mechanism within PyLoop, bypassing `asyncio` overhead entirely while maintaining Python compatibility. This achieved the goal of exceeding FastAPI performance (up to 1.61x).