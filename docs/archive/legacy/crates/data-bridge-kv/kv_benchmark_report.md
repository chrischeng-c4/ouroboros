# KV Store Performance Benchmarks

Comprehensive performance analysis of the data-bridge KV storage engine

**Generated:** 2026-01-06 07:06:07  
**Total Duration:** 42.84s  

## Pure Engine Performance

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| set_single_thread | 0.000ms | 0.000ms | 0.001ms | 5239881.8 | (baseline) |
| get_single_thread | 0.000ms | 0.000ms | 0.000ms | 7712478.8 | **1.47x faster** |
| mixed_50_50 | 0.000ms | 0.000ms | 0.001ms | 5976500.4 | **1.14x faster** |
| incr_atomic | 0.000ms | 0.000ms | 0.000ms | 18319715.7 | **3.50x faster** |
| decr_atomic | 0.000ms | 0.000ms | 0.000ms | 18894305.3 | **3.61x faster** |
| cas_operations | 0.000ms | 0.000ms | 0.006ms | 10291454.0 | **1.96x faster** |
| setnx_operations | 0.000ms | 0.000ms | 0.001ms | 5050607.1 | 1.04x slower |
| exists_checks | 0.000ms | 0.000ms | 0.000ms | 8413684.0 | **1.61x faster** |
| delete_operations | 0.000ms | 0.000ms | 0.000ms | 8179423.8 | **1.56x faster** |

## Concurrency - SET Operations

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| set_concurrent_2t | 0.116ms | 0.087ms | 0.237ms | 8623.0 | (baseline) |
| set_concurrent_4t | 0.118ms | 0.084ms | 0.235ms | 8470.0 | 1.02x slower |
| set_concurrent_8t | 0.180ms | 0.115ms | 0.378ms | 5546.2 | 1.55x slower |

## Concurrency - GET Operations

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| get_concurrent_2t | 0.075ms | 0.047ms | 0.166ms | 13420.9 | (baseline) |
| get_concurrent_4t | 0.084ms | 0.049ms | 0.138ms | 11956.3 | 1.12x slower |
| get_concurrent_8t | 0.124ms | 0.083ms | 0.287ms | 8076.5 | 1.66x slower |

## Lock Contention

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| lock_contention | 0.325ms | 0.211ms | 0.772ms | 3079.4 | (baseline) |

## Memory & TTL Management

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| memory_100k_entries | 34.509ms | 27.383ms | 70.631ms | 29.0 | (baseline) |
| ttl_cleanup | 14.863ms | 12.416ms | 25.670ms | 67.3 | **2.32x faster** |

## Insertion Scalability

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| insert_1k | 0.199ms | 0.184ms | 0.311ms | 5025.7 | (baseline) |
| insert_10k | 2.086ms | 1.979ms | 3.306ms | 479.4 | 10.48x slower |
| insert_100k | 31.643ms | 27.113ms | 44.949ms | 31.6 | 159.03x slower |
| insert_1m | 682.868ms | 682.868ms | 682.868ms | 1.5 | 3431.90x slower |

## Environment

- **Rust:** 
- **Platform:** macos
- **CPU:** 10 cores
