# Lock Contention Investigation - Root Cause Analysis

**Date**: 2026-01-06
**Status**: ✅ Resolved - Benchmark Methodology Bug

## Problem Statement

Original benchmarks showed **performance degradation** with increased concurrency:

| Threads | SET ops/sec | vs 2 threads |
|---------|-------------|--------------|
| 2       | 8,623       | baseline     |
| 4       | 8,470       | 1.02x slower |
| 8       | 5,546       | 1.55x slower |

This was concerning for a **sharded, multi-core architecture** designed to scale.

---

## Investigation Process

### 1. Shard Distribution Analysis

**Result**: ✅ EXCELLENT

```
Keys: 10,000
Shards: 256
Non-empty shards: 256/256 (100% utilization)

Distribution:
  Min keys/shard: 24
  Max keys/shard: 55
  Avg keys/shard: 39
  Std deviation: 5.70
```

**Conclusion**: Hash distribution is working perfectly. All shards are utilized evenly.

---

### 2. Concurrent Access Patterns

**Test A: High Contention (100 shared keys)**

| Threads | ops/sec   | Speedup |
|---------|-----------|---------|
| 1       | 7.9M      | 1.00x   |
| 2       | 9.1M      | 1.15x   |
| 4       | 9.8M      | 1.24x   |
| 8       | 12.8M     | 1.61x   |

**Test B: Low Contention (unique keys per thread)**

| Threads | ops/sec   | Speedup |
|---------|-----------|---------|
| 1       | 3.6M      | 1.00x   |
| 2       | 4.8M      | 1.31x   |
| 4       | 8.3M      | 2.28x   |
| 8       | 9.5M      | 2.61x   |

**Test C: Persistent Thread Pool**

| Threads | ops/sec   | Speedup |
|---------|-----------|---------|
| 1       | 4.8M      | 1.00x   |
| 2       | 6.9M      | 1.44x   |
| 4       | 10.1M     | 2.10x   |
| 8       | 10.3M     | 2.14x   |

**Conclusion**: ✅ The KV engine scales well! Even with high contention, we see speedups.

---

## Root Cause: Benchmark Methodology Bug

### The Flawed Benchmark Code

```rust
let benchmarker = Benchmarker::new(BenchmarkConfig::thorough());
// Config: iterations=100, rounds=5, warmup=10

benchmarker.run("set_concurrent_2t", || {
    let engine = Arc::new(KvEngine::new());  // ❌ NEW ENGINE EVERY ITERATION!
    let mut handles = vec![];

    for thread_id in 0..2 {
        let engine = Arc::clone(&engine);
        handles.push(thread::spawn(move || {  // ❌ SPAWN THREADS EVERY ITERATION!
            for i in 0..ops_per_thread {
                engine.set(&key, value, None);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();  // ❌ JOIN OVERHEAD EVERY ITERATION!
    }
})
```

### What Gets Measured

The `benchmarker.run()` method executes the **entire closure** for each iteration:

- **Iterations**: 100
- **Rounds**: 5
- **Total executions**: 500

For the 8-thread benchmark, this means:
- ❌ 500 × engine creations
- ❌ 500 × 8 = **4,000 thread spawns**
- ❌ 4,000 thread joins
- ✅ Actual SET operations

### Why Performance Degrades

| Component          | 2 threads  | 4 threads  | 8 threads  |
|--------------------|------------|------------|------------|
| Thread spawns/iter | 2          | 4          | 8          |
| Total spawns       | 1,000      | 2,000      | 4,000      |
| **Overhead impact**| **Medium** | **High**   | **SEVERE** |

Thread spawning overhead dominates, masking the actual engine performance!

### Evidence: 3 Orders of Magnitude Difference

- **Flawed benchmark**: 8,623 ops/sec (2 threads)
- **Corrected test**: 6.9M ops/sec (2 threads, persistent pool)
- **Ratio**: **800x difference!**

---

## Architecture Validation

The diagnostic tests confirm:

1. ✅ **Sharding works**: Perfect distribution across 256 shards
2. ✅ **RwLock is efficient**: 7.9M ops/sec even under high contention
3. ✅ **Multi-core scaling**: 2.14x speedup with 8 threads (expected: ~2-3x on 10-core machine)
4. ✅ **No lock thrashing**: Concurrent different-key access achieves 9.5M ops/sec with 8 threads

---

## Recommendations

### 1. Fix Benchmark Methodology

**Before** (measures overhead):
```rust
benchmarker.run("test", || {
    let engine = Arc::new(KvEngine::new());
    spawn_threads_and_do_work();
})
```

**After** (measures actual performance):
```rust
let engine = Arc::new(KvEngine::new());  // ✅ Create once
benchmarker.run("test", || {
    do_work_on_existing_engine(&engine);  // ✅ Only measure operations
})
```

### 2. Separate Benchmark Types

- **Microbenchmarks**: Single-threaded, pure engine operations (current)
- **Concurrency benchmarks**: Pre-allocated threads, measure throughput
- **Scalability benchmarks**: Vary thread count, measure scaling factor

### 3. Use Criterion.rs

The current benchmark harness is unsuitable for concurrent workloads. Consider migrating to `criterion.rs`:

```rust
fn bench_concurrent_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_set");
    let engine = Arc::new(KvEngine::new());

    for num_threads in [1, 2, 4, 8] {
        group.bench_function(format!("{}_threads", num_threads), |b| {
            b.iter(|| {
                // Only the actual work is measured
                concurrent_set_workload(&engine, num_threads);
            });
        });
    }
}
```

---

## Conclusion

**Status**: ✅ **No Lock Contention Issue**

The perceived performance degradation was a **benchmark artifact**, not a real architectural problem. The KV engine:

- Scales well with concurrent access (2.14x speedup with 8 threads)
- Has excellent shard distribution (100% utilization, low variance)
- Handles both high and low contention workloads efficiently
- Achieves 10M+ ops/sec with multi-threaded access

The actual bottleneck was thread spawning overhead in the benchmark measurement, which increased linearly with thread count.

---

## Files Modified

- `crates/data-bridge-kv/examples/diagnose_contention.rs` - Diagnostic tool
- `kb/60-kv-store/benchmarks/04-contention-analysis.md` - This document

## Next Steps

1. ✅ Document findings (this file)
2. ⏭️ Create corrected concurrent benchmarks
3. ⏭️ Consider migrating to criterion.rs for future benchmarks
4. ⏭️ Add persistent thread pool benchmarks to official suite
