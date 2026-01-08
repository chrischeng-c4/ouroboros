# KV Store Concurrent Performance (Fixed)

Corrected concurrent benchmarks measuring actual throughput without thread spawning overhead

**Generated:** 2026-01-06 09:45:10  
**Total Duration:** 2.95s  

## Concurrent SET Operations

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| set_2_threads | 45.959ms | 39.357ms | 52.832ms | 21.8 | (baseline) |
| set_4_threads | 61.233ms | 52.840ms | 72.716ms | 16.3 | 1.33x slower |
| set_8_threads | 82.703ms | 73.533ms | 88.727ms | 12.1 | 1.80x slower |

## Concurrent GET Operations

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| get_2_threads | 28.046ms | 23.525ms | 33.458ms | 35.7 | (baseline) |
| get_4_threads | 34.852ms | 31.855ms | 38.713ms | 28.7 | 1.24x slower |
| get_8_threads | 37.203ms | 29.175ms | 40.290ms | 26.9 | 1.33x slower |

## Concurrent INCR Operations (Atomic)

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| incr_2_threads | 12.776ms | 10.746ms | 15.572ms | 78.3 | (baseline) |
| incr_4_threads | 33.058ms | 29.195ms | 42.522ms | 30.3 | 2.59x slower |
| incr_8_threads | 62.966ms | 54.383ms | 72.808ms | 15.9 | 4.93x slower |

## Concurrent Mixed Workload (50% GET, 50% SET)

| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |
|-----------|------|-----|-----|-------|-------------|
| mixed_2_threads | 26.997ms | 24.099ms | 34.999ms | 37.0 | (baseline) |
| mixed_4_threads | 75.159ms | 50.412ms | 115.873ms | 13.3 | 2.78x slower |
| mixed_8_threads | 88.084ms | 80.108ms | 113.411ms | 11.4 | 3.26x slower |

## Environment

- **Rust:** 
- **Platform:** macos
- **CPU:** 10 cores
