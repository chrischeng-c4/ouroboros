# KV Store Performance Benchmarks

Comprehensive performance analysis of the data-bridge KV storage engine

**Generated:** 2026-01-06 03:16:32  
**Total Duration:** 41.99s  

## Pure Engine Performance

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| set_single_thread | 0.000ms | 0.000ms | 0.060ms | 3091801.8 | (baseline) |
| get_single_thread | 0.000ms | 0.000ms | 0.000ms | 6775159.6 | **2.19x faster** |
| mixed_50_50 | 0.000ms | 0.000ms | 0.001ms | 5828524.8 | **1.89x faster** |
| incr_atomic | 0.000ms | 0.000ms | 0.000ms | 15465033.6 | **5.00x faster** |
| decr_atomic | 0.000ms | 0.000ms | 0.060ms | 6029981.1 | **1.95x faster** |
| cas_operations | 0.000ms | 0.000ms | 0.000ms | 18236195.2 | **5.90x faster** |
| setnx_operations | 0.000ms | 0.000ms | 0.016ms | 4471152.1 | **1.45x faster** |
| exists_checks | 0.000ms | 0.000ms | 0.000ms | 8409014.5 | **2.72x faster** |
| delete_operations | 0.000ms | 0.000ms | 0.002ms | 7742814.7 | **2.50x faster** |

## Concurrency - SET Operations

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| set_concurrent_2t | 0.136ms | 0.089ms | 2.776ms | 7373.4 | (baseline) |
| set_concurrent_4t | 0.139ms | 0.080ms | 0.770ms | 7205.7 | 1.02x slower |
| set_concurrent_8t | 0.190ms | 0.120ms | 0.359ms | 5261.3 | 1.40x slower |

## Concurrency - GET Operations

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| get_concurrent_2t | 0.078ms | 0.051ms | 0.238ms | 12852.9 | (baseline) |
| get_concurrent_4t | 0.087ms | 0.053ms | 0.174ms | 11534.7 | 1.11x slower |
| get_concurrent_8t | 0.130ms | 0.084ms | 0.318ms | 7699.6 | 1.67x slower |

## Lock Contention

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| lock_contention | 0.342ms | 0.208ms | 0.944ms | 2926.5 | (baseline) |

## Memory & TTL Management

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| memory_100k_entries | 34.189ms | 26.529ms | 49.115ms | 29.2 | (baseline) |
| ttl_cleanup | 14.715ms | 12.473ms | 28.527ms | 68.0 | **2.32x faster** |

## Insertion Scalability

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| insert_1k | 0.201ms | 0.184ms | 0.263ms | 4974.2 | (baseline) |
| insert_10k | 2.023ms | 1.956ms | 2.530ms | 494.2 | 10.06x slower |
| insert_100k | 30.574ms | 26.563ms | 44.767ms | 32.7 | 152.08x slower |
| insert_1m | 588.275ms | 588.275ms | 588.275ms | 1.7 | 2926.19x slower |

## Environment

- **Rust:** 
- **Platform:** macos
- **CPU:** 10 cores
