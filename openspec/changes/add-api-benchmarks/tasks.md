## 1. Benchmark Implementation
- [x] 1.1 Create benchmark harness in `tests/api/benchmarks/` (following `tests/postgres/benchmarks` pattern).
- [x] 1.2 Implement Throughput scenarios:
    - `GET /plaintext` (Minimal overhead)
    - `GET /items/{id}` (Path extraction)
    - JSON response testing
- [x] 1.3 Implement Latency scenarios (P50, P99) under load (100, 1000, 5000 concurrent).
- [x] 1.4 Implement Serialization benchmarks (Payload sizes: 1KB, 10KB, 100KB, 1MB).
- [x] 1.5 Implement GIL verification test (ensure Rust router doesn't block Python threads).
- [x] 1.6 Create benchmark structure using `BenchmarkGroup` API (matches project pattern).
- [x] 1.7 Create README documentation for running benchmarks.

## 2. Validation & Reporting
- [x] 2.1 Benchmarks ready to run against `data-bridge-api` (see README for commands).
- [x] 2.2 Benchmarks ready to run against FastAPI + Uvicorn baseline (automated in tests).
- [x] 2.3 Comparison report generated automatically by `test_summary` function.