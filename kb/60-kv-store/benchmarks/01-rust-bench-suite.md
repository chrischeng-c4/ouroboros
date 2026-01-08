# KV Store Benchmark Suite

Comprehensive performance benchmarks for the data-bridge KV storage engine.

## Running Benchmarks

### Run all benchmarks

```bash
cargo bench --bench kv_benchmark -p data-bridge-kv
```

### Run in release mode (recommended)

```bash
cargo bench --bench kv_benchmark -p data-bridge-kv --release
```

## Benchmark Categories

### 1. Pure Engine Performance (No Network)

Tests the raw performance of the KV engine operations:

- **SET throughput** - Single-threaded write performance
- **GET throughput** - Single-threaded read performance
- **Mixed workload** - 50% GET, 50% SET operations
- **INCR operations** - Atomic increment performance
- **DECR operations** - Atomic decrement performance
- **CAS operations** - Compare-And-Swap performance
- **SETNX operations** - Set-if-not-exists performance
- **EXISTS checks** - Key existence checking performance
- **DELETE operations** - Key deletion performance

**Expected Results:**
- Single operations: 5-16M ops/sec
- Atomic operations (INCR/DECR/CAS): 12-16M ops/sec
- GET faster than SET by ~1.4x

### 2. Concurrency - SET Operations

Tests multi-threaded write performance:

- **2 threads** - Baseline concurrent writes
- **4 threads** - Medium concurrency
- **8 threads** - High concurrency

**Expected Results:**
- 2 threads: ~9K composite ops/sec
- 4 threads: ~8K composite ops/sec (1.1x slower due to contention)
- 8 threads: ~6K composite ops/sec (1.5x slower due to contention)

### 3. Concurrency - GET Operations

Tests multi-threaded read performance:

- **2 threads** - Baseline concurrent reads
- **4 threads** - Medium concurrency
- **8 threads** - High concurrency

**Expected Results:**
- 2 threads: ~13K composite ops/sec
- 4 threads: ~12K composite ops/sec
- 8 threads: ~8K composite ops/sec
- Reads scale better than writes due to RwLock

### 4. Lock Contention

Tests distributed lock performance with 4 threads competing for a single lock.

**Expected Results:**
- ~3K lock/unlock cycles per second
- This tests worst-case contention scenario

### 5. Memory & TTL Management

Tests memory usage and TTL cleanup:

- **Memory usage** - Insert 100K entries
- **TTL cleanup** - Clean up 10K expired entries

**Expected Results:**
- 100K insertions: ~34ms
- TTL cleanup: ~15ms (2.3x faster than insertion)

### 6. Insertion Scalability

Tests how the engine scales with dataset size:

- **1K entries** - Baseline
- **10K entries** - 10x scale
- **100K entries** - 100x scale
- **1M entries** - 1000x scale

**Expected Results:**
- Linear scaling: 10x entries = ~10x time
- 1K: 0.2ms
- 10K: 2ms
- 100K: 34ms
- 1M: 611ms

## Generated Reports

The benchmark generates three report formats:

1. **JSON** (`kv_benchmark_report.json`) - Machine-readable data
2. **HTML** (`kv_benchmark_report.html`) - Interactive charts and tables
3. **Markdown** (`kv_benchmark_report.md`) - Human-readable tables

### Viewing Reports

```bash
# View Markdown report
cat kv_benchmark_report.md

# Open HTML report in browser (macOS)
open kv_benchmark_report.html

# Open HTML report in browser (Linux)
xdg-open kv_benchmark_report.html
```

## Benchmark Configuration

The benchmark uses `BenchmarkConfig::thorough()` which provides:

- **Iterations**: 100 per round
- **Rounds**: 5
- **Warmup**: 10 iterations
- **Total runs**: 500 per benchmark

For the 1M insertion test, a lighter configuration is used (1 iteration, 1 round) due to time constraints.

## Performance Metrics

Each benchmark reports:

- **Mean time** - Average time per operation
- **Min/Max time** - Fastest and slowest runs
- **P50 (Median)** - 50th percentile
- **P95** - 95th percentile (outlier detection)
- **P99** - 99th percentile (extreme outliers)
- **Standard deviation** - Measure of variance
- **Ops/sec** - Operations per second
- **95% Confidence Interval** - Statistical confidence bounds
- **IQR** - Interquartile range (outlier detection)
- **Outliers** - Count of statistical outliers (high/low)

## Understanding Results

### Single-threaded Performance

The pure engine benchmarks show the theoretical maximum throughput:

```
incr_atomic: 16M ops/sec
```

This means the engine can process 16 million atomic increments per second on a single thread.

### Concurrent Performance

The concurrent benchmarks show real-world multi-threaded performance:

```
set_concurrent_2t: 9K composite ops/sec
```

This means 2 threads together can perform 9,000 complete insertion cycles per second. The benchmark measures the total time for all threads to complete, so this is the effective throughput.

### Scalability Analysis

The insertion scalability benchmarks show linear scaling:

```
1K entries:   0.2ms  (5K ops/sec)
10K entries:  2ms    (500 ops/sec)
100K entries: 34ms   (30 ops/sec)
1M entries:   611ms  (2 ops/sec)
```

This demonstrates O(1) per-operation complexity - the time per entry remains constant as the dataset grows.

## Comparing Results

To compare different configurations or optimizations:

1. Run benchmark and save reports:
   ```bash
   cargo bench --bench kv_benchmark -p data-bridge-kv
   mv kv_benchmark_report.md baseline.md
   ```

2. Make changes to the code

3. Run benchmark again:
   ```bash
   cargo bench --bench kv_benchmark -p data-bridge-kv
   ```

4. Compare reports:
   ```bash
   diff baseline.md kv_benchmark_report.md
   ```

## CI/CD Integration

To run benchmarks in CI and fail on regressions:

```bash
cargo bench --bench kv_benchmark -p data-bridge-kv -- --save-baseline main

# After changes
cargo bench --bench kv_benchmark -p data-bridge-kv -- --baseline main
```

## Troubleshooting

### Benchmark takes too long

The 1M insertion test takes ~611ms. For quick testing, you can:

1. Skip the scalability group
2. Reduce the thorough config iterations
3. Run specific benchmarks only

### High variance in results

If you see high standard deviation:

1. Close other applications
2. Disable CPU frequency scaling
3. Run on a dedicated benchmark machine
4. Increase warmup iterations

### Memory issues

The 1M insertion test allocates significant memory. If you encounter OOM:

1. Reduce the insertion count
2. Run on a machine with more RAM
3. Monitor with `cargo bench --bench kv_benchmark -- --profile-time`

## Future Improvements

Potential benchmark additions:

1. **Network latency simulation** - Test with simulated I/O delays
2. **Different value sizes** - Small (10B), medium (1KB), large (10KB)
3. **Key distribution patterns** - Uniform vs. skewed access
4. **Persistence overhead** - Once storage backend is added
5. **Cross-shard operations** - Test shard balancing
6. **Replication lag** - Once replication is implemented

## Architecture Notes

The benchmarks use the `data-bridge-test` crate's `Benchmarker` which provides:

- Statistical analysis (mean, median, percentiles)
- Outlier detection (IQR method)
- Confidence intervals (95% CI)
- Multiple output formats (JSON, HTML, Markdown)
- Comparison baseline support
- Warmup runs to stabilize caches

This is similar to pytest-benchmark but in pure Rust.
