## 1. Core Optimizations
- [x] 1.1 Replace `serde_json` with `sonic-rs` in `crates/data-bridge-api/src/server.rs` for request body parsing.
- [x] 1.2 Refactor `SerializableRequest` in `crates/data-bridge-api/src/request.rs` to use `bytes::Bytes` instead of `Vec<u8>` for the body.
- [x] 1.3 Update `collect_body` in `server.rs` to return `bytes::Bytes`.
- [x] 1.4 Configure Hyper server in `server.rs` with performance tuning options (`keep_alive`, `pipeline_flush`, `TCP_NODELAY`).

## 2. Integration & Cleanup
- [x] 2.1 Verify `SerializableRequest` to Python conversion still works (or update it if it relied on `Vec<u8>`).
- [x] 2.2 Ensure `sonic-rs` is correctly configured (e.g., using `LazyValue` if appropriate for delayed parsing, though full parsing is likely needed for validation).

## 3. Verification
- [x] 3.1 Run `cargo test -p data-bridge-api` to ensure no regressions. ✅ 106 tests passed
- [x] 3.2 Run `pytest benchmarks/` to verify performance improvements against the baseline.

## Benchmark Results

| Scenario | Before | After | Change |
|----------|--------|-------|--------|
| Plaintext | 835 ops/s | 916 ops/s | **+9.7%** |
| Serialize 10KB | 712 ops/s | 807 ops/s | **+13.3%** |
| Serialize 100KB | 310 ops/s | 338 ops/s | **+9.0%** |
| Serialize 1MB | 47 ops/s | 48 ops/s | +2.1% |

**Note**: Still slower than FastAPI. Further optimization needed in Phase 2 (Python handler invocation, GIL management).
