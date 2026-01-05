# KV Store Benchmark Tests

Comprehensive performance benchmarks for the data-bridge KV store.

## Overview

The benchmark suite measures and compares:

1. **Single Client Performance** - Raw throughput for individual operations
2. **Connection Pool Performance** - Concurrent access and pool overhead
3. **Redis Comparison** - Side-by-side performance vs Redis
4. **Latency Distribution** - P50/P95/P99 latency metrics

## Requirements

### Required
- **kv-server**: Must be built in release mode
  ```bash
  cargo build --release --bin kv-server
  ```

### Optional (for Redis comparison)
- **Redis**: Running on `localhost:6379`
- **redis-py**: Python Redis client
  ```bash
  uv pip install redis
  ```

If Redis is not available, comparison tests will be skipped automatically.

## Running Benchmarks

### Run all benchmarks
```bash
uv run pytest tests/kv/test_benchmark.py -v -s -m benchmark
```

### Run specific test classes
```bash
# Single client tests
uv run pytest tests/kv/test_benchmark.py::TestSingleClientPerformance -v -s -m benchmark

# Pool tests
uv run pytest tests/kv/test_benchmark.py::TestPoolPerformance -v -s -m benchmark

# Redis comparison
uv run pytest tests/kv/test_benchmark.py::TestRedisComparison -v -s -m benchmark

# Latency distribution
uv run pytest tests/kv/test_benchmark.py::TestLatencyDistribution -v -s -m benchmark
```

### Run specific tests
```bash
uv run pytest tests/kv/test_benchmark.py::TestSingleClientPerformance::test_set_throughput -v -s
```

### Skip benchmarks in regular test runs
```bash
uv run pytest tests/kv/ -v -m "not benchmark"
```

## Benchmark Results

### Single Client Performance (10,000 ops)

| Operation | Throughput | Notes |
|-----------|-----------|-------|
| SET | 7,500+ ops/sec | Raw write performance |
| GET | 7,400+ ops/sec | Raw read performance |
| Mixed | 6,700+ ops/sec | 50% GET, 50% SET |
| INCR | 7,600+ ops/sec | Atomic increment |

Target: >5,000 ops/sec for all operations

### Connection Pool Performance

| Test | Throughput | Configuration |
|------|-----------|---------------|
| Concurrent SET | 15,000+ ops/sec | 10 workers, 1,000 ops each |
| Concurrent Mixed | 14,600+ ops/sec | 10 workers, mixed workload |
| Pool Overhead | <20% | vs single client |

Target: >10,000 ops/sec with concurrent workers

### vs Redis Comparison (5,000-10,000 ops)

| Operation | data-bridge | Redis | Ratio |
|-----------|-------------|-------|-------|
| SET | 8,000+ ops/sec | 5,500+ ops/sec | **1.47x** |
| GET | 8,300+ ops/sec | 5,800+ ops/sec | **1.43x** |
| Mixed | 8,000+ ops/sec | 5,700+ ops/sec | **1.40x** |
| INCR | 8,300+ ops/sec | 5,000+ ops/sec | **1.67x** |

**data-bridge is consistently 1.4x-1.67x faster than Redis**

### Latency Distribution (1,000 ops)

#### SET Operations
| Metric | Latency |
|--------|---------|
| Min | 0.087 ms |
| Avg | 0.129 ms |
| P50 | 0.115 ms |
| P95 | 0.197 ms |
| P99 | 0.241 ms |
| Max | 0.633 ms |

#### GET Operations
| Metric | Latency |
|--------|---------|
| Min | 0.095 ms |
| Avg | 0.128 ms |
| P50 | 0.113 ms |
| P95 | 0.181 ms |
| P99 | 0.436 ms |
| Max | 1.367 ms |

Target: Avg <1ms, P99 <5ms

## Test Structure

### TestSingleClientPerformance
- `test_set_throughput` - Measures SET ops/sec with single client
- `test_get_throughput` - Measures GET ops/sec with pre-populated data
- `test_mixed_workload` - Measures 50/50 GET/SET mix
- `test_incr_throughput` - Measures atomic increment performance

### TestPoolPerformance
- `test_pool_concurrent_set` - 10 workers doing concurrent SETs
- `test_pool_concurrent_mixed` - 10 workers with mixed operations
- `test_pool_vs_single_client` - Measures pool overhead for sequential ops

### TestRedisComparison
- `test_set_comparison` - SET: data-bridge vs Redis
- `test_get_comparison` - GET: data-bridge vs Redis
- `test_mixed_comparison` - Mixed workload comparison
- `test_incr_comparison` - INCR: data-bridge vs Redis

All comparison tests automatically skip if Redis is unavailable.

### TestLatencyDistribution
- `test_set_latency_distribution` - P50/P95/P99 for SET operations
- `test_get_latency_distribution` - P50/P95/P99 for GET operations

## Performance Targets

| Category | Metric | Target | Current |
|----------|--------|--------|---------|
| Single Client | Throughput | >5,000 ops/sec | 6,700-7,600 ops/sec ✅ |
| Pool | Throughput | >10,000 ops/sec | 14,600-15,000 ops/sec ✅ |
| Pool | Overhead | <20% | -4% (faster!) ✅ |
| Latency | Average | <1ms | 0.128-0.129ms ✅ |
| Latency | P99 | <5ms | 0.241-0.436ms ✅ |
| vs Redis | Ratio | >1.0x | 1.40-1.67x ✅ |

All targets met! ✅

## Implementation Notes

### Warm-up
Each test includes a warm-up phase (typically 100 operations) to:
- Establish TCP connection
- Fill OS-level buffers
- Stabilize timing measurements

### Cleanup
Tests clean up their own keys to avoid:
- Memory bloat in kv-server
- Test interference
- False performance metrics

### Assertions
Each test includes performance assertions to catch regressions:
- Single client: Must exceed 5,000 ops/sec
- Pool: Must exceed 10,000 ops/sec
- Pool overhead: Must be <20%
- Latency: Avg <1ms, P99 <5ms

## Interpreting Results

### High Throughput (Good)
```
[SET Throughput] 8,000 ops/sec (1.250s for 10,000 ops)
```
- Fast operations, efficient implementation
- Server handles load well

### Low Throughput (Investigation Needed)
```
[SET Throughput] 3,000 ops/sec (3.333s for 10,000 ops)
```
- Check server load
- Check network latency
- Review Rust implementation

### Pool Overhead
```
Pool vs Single Client:
  Single:  8,000 ops/sec
  Pool:    7,500 ops/sec
  Overhead: 6.3%
```
- Normal: 5-15% overhead for connection pooling
- Acceptable: <20%
- Concerning: >30%

### Redis Comparison
```
Ratio: 1.47x
```
- 1.0x = Same performance as Redis
- >1.0x = Faster than Redis ✅
- <1.0x = Slower than Redis (needs investigation)

## Continuous Integration

To integrate benchmarks into CI:

```bash
# Run benchmarks and fail if targets not met
uv run pytest tests/kv/test_benchmark.py -v -m benchmark

# Generate performance report
uv run pytest tests/kv/test_benchmark.py -v -s -m benchmark > benchmark_results.txt
```

## Troubleshooting

### kv-server not found
```
RuntimeError: KV server failed to start
```
**Solution**: Build kv-server in release mode:
```bash
cargo build --release --bin kv-server
```

### Tests run too slowly
Check if running in debug mode. Always use release mode for benchmarks:
```bash
cargo build --release --bin kv-server
```

### Redis comparison tests skipped
```
SKIPPED [1] Redis not available: Connection refused
```
This is expected if Redis is not running. Start Redis:
```bash
redis-server --port 6379
```

### Intermittent failures
Network or system load can affect results. Ensure:
- System is not under heavy load
- kv-server has sufficient resources
- No other tests running concurrently

## Future Enhancements

- [ ] Pipeline operations benchmark
- [ ] Bulk operations (MGET, MSET)
- [ ] TTL expiration overhead
- [ ] Lock contention benchmarks
- [ ] Network latency simulation
- [ ] Memory usage tracking
- [ ] CPU profiling integration
