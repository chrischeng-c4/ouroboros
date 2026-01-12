# Change: Integrate PyLoop with HTTP Server

## Why
Currently, the `data-bridge` project uses a standard ASGI stack (uvloop + uvicorn + FastAPI) or a separate Tokio runtime for the Rust API server. This creates unnecessary overhead in context switching and data serialization (BSON -> Python -> JSON).

We have successfully implemented `data-bridge-pyloop` (Phase 3), which provides a Rust-native Python event loop on top of Tokio. By integrating this directly with our Rust-based Hyper/Axum HTTP server, we can achieve:
1.  **Unified Runtime**: A single Tokio runtime handling both HTTP I/O and Python tasks.
2.  **Zero-Copy Routing**: Request parsing and routing happen entirely in Rust (no GIL).
3.  **Hybrid Dispatch**: Pure Rust handlers avoid the GIL entirely; Python handlers are spawned efficiently on the event loop.

## What Changes
- **Architecture**: Replace the Uvicorn/ASGI entry point with a custom Rust entry point (`data_bridge.serve()`) that initializes the shared Tokio runtime and spawns the Hyper server.
- **Dispatch Logic**: Update `data-bridge-api` to support two handler types:
    - **Native Rust**: Executed directly on the I/O thread (async).
    - **Python Bridge**: Wraps Python coroutines in a `PyLoop::spawn` call, awaiting the result as a Rust Future.
- **New Feature**: Add a **Declarative DSL** for routes. Instead of writing boilerplate handlers, users can define a Pydantic model, and Rust will automatically generate high-performance CRUD endpoints.

## Impact
- **Affected Specs**: `api-server`
- **Affected Code**:
    - `crates/data-bridge-api/` (Server logic, Handler traits)
    - `crates/data-bridge-pyloop/` (Expose `spawn` API for Rust)
    - `python/data_bridge/` (New entry point and decorators)

## Risks & Mitigation

### High Risk 🔴

**Risk 1: PyLoop Bridge API Not Implemented**
- **Issue**: `spawn_python_handler()` returning Rust Future doesn't exist yet
- **Impact**: Core integration blocked without this API
- **Mitigation**:
  - Implement as Priority 1 task (1-2 days)
  - Use oneshot channel pattern (proven in MongoDB ORM)
  - Fallback: Use `spawn_blocking` if needed (adds ~50µs overhead)

**Risk 2: Coroutine Polling Blocks Threads**
- **Issue**: Current `task.rs` poll loop uses `std::thread::sleep()`, blocks HTTP workers
- **Impact**: Could limit concurrent request handling
- **Mitigation**:
  - Phase 1: Use `spawn_blocking` (workaround, acceptable overhead)
  - Phase 4: Implement proper awaitable protocol (future enhancement)
  - Benchmark early to verify acceptable performance

### Medium Risk 🟡

**Risk 3: Pydantic Type Extraction Complexity**
- **Issue**: Generic types (`List[T]`, `Optional[Dict[K,V]]`) are complex to parse
- **Impact**: Declarative DSL may not support all Pydantic features initially
- **Mitigation**:
  - Start with simple types (str, int, float, bool)
  - Incrementally add complex types based on usage patterns
  - Document unsupported types clearly

**Risk 4: Python Handler Performance Target**
- **Issue**: Target 2x FastAPI performance assumes <1ms Python execution
- **Impact**: May not achieve 2x improvement for slow handlers
- **Mitigation**:
  - Focus optimization on Rust-side (routing, validation, serialization)
  - Set realistic expectations: Fast routing + Python overhead ≈ 1.5-2x improvement
  - Declarative DSL (pure Rust) will achieve 30x+ improvement regardless

**Risk 5: Production Stability (Shutdown, Timeout, Errors)**
- **Issue**: Graceful shutdown coordination and error handling are complex
- **Impact**: Potential resource leaks or crashes under edge cases
- **Mitigation**:
  - Comprehensive testing (Phase 6)
  - Use proven patterns from MongoDB ORM
  - Load testing before production deployment

### Low Risk 🟢

**Risk 6: MongoDB Connection Pool Integration**
- **Issue**: Declarative handlers need to share connection pool
- **Impact**: Minor - connection pool already exists
- **Mitigation**: Use existing `data-bridge-mongodb` connection pool (no new implementation needed)

**Risk 7: Middleware Compatibility**
- **Issue**: Python middleware may not integrate smoothly with Tower middleware
- **Impact**: Limited - can prioritize Rust middleware
- **Mitigation**: Rust middleware for production, Python middleware optional (dev/debug only)

## Success Criteria

**Phase 1 (Core Integration) - Week 1-2**:
- ✅ `spawn_python_handler()` API implemented and tested
- ✅ Simple Python sync handler works end-to-end
- ✅ Simple Python async handler works end-to-end
- ✅ Error propagation from Python to HTTP response

**Phase 2 (API Surface) - Week 2-3**:
- ✅ `@app.get`, `@app.post` decorators work
- ✅ FastAPI-compatible API surface
- ✅ Request/Response objects accessible in Python

**Phase 3 (Declarative DSL) - Week 3-4**:
- ✅ `@app.crud(Model)` generates 5 CRUD endpoints
- ✅ Pure Rust execution (zero Python overhead)
- ✅ Benchmark: 30x faster than FastAPI for CRUD operations

**Phase 4-6 (Production Hardening) - Week 5-6**:
- ✅ Error handling covers all edge cases
- ✅ Request timeout works correctly
- ✅ Graceful shutdown without resource leaks
- ✅ Load test: 1 hour sustained load, no memory leaks
- ✅ Benchmark: 2x faster than uvicorn for Python handlers

## Performance Targets

**Declarative CRUD (Pure Rust)**:
- Target: **60,000 req/sec** (simple list operation)
- Baseline: FastAPI + Beanie ≈ 2,000 req/sec
- Expected: **30x improvement**

**Python Handler (Hybrid)**:
- Target: **3,000 req/sec** (simple JSON echo)
- Baseline: FastAPI + uvicorn ≈ 1,500 req/sec
- Expected: **2x improvement**

**Latency (p99)**:
- Declarative CRUD: <5ms
- Python handler: <10ms
- Timeout: <50ms (for long operations)
