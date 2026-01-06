# Adaptive Sampling Implementation

## Overview

Adaptive sampling has been implemented in `crates/data-bridge-test/src/benchmark.rs` to automatically determine the optimal number of benchmark iterations based on statistical convergence criteria.

## Implementation Summary

### New Components

#### 1. `AdaptiveBenchmarkConfig` Struct
Configuration for adaptive benchmarking with the following fields:
- `enable_adaptive`: Enable/disable adaptive sampling
- `target_cv_percent`: Target coefficient of variation (default: 5%)
- `target_ci_width_percent`: Target confidence interval width (default: 5%)
- `min_iterations`: Minimum iterations before early stopping (default: 10)
- `max_iterations`: Maximum iteration cap (default: 10,000)
- `warmup`: Warmup iterations (default: 3)
- `initial_sample_size`: Initial sample for variance estimation (default: 5)
- `timeout_ms`: Optional timeout in milliseconds

**Presets:**
- `AdaptiveBenchmarkConfig::default()`: Balanced configuration (5% CV/CI)
- `AdaptiveBenchmarkConfig::quick()`: Faster convergence (10% CV/CI, max 1,000 iterations)
- `AdaptiveBenchmarkConfig::thorough()`: Precise measurements (2% CV/CI, max 50,000 iterations)

#### 2. `BenchmarkStats` Extensions
Added adaptive metadata fields:
- `adaptive_stopped_early`: Whether early stopping occurred
- `adaptive_reason`: Reason for stopping (e.g., "converged: CV=3.2%, CI_width=4.1%" or "timeout")
- `adaptive_iterations_used`: Actual number of iterations executed

#### 3. Helper Functions
- `calculate_cv(mean, std_dev)`: Computes coefficient of variation as percentage
- `calculate_required_iterations(mean, std_dev, target_cv, z_score)`: Estimates required sample size

#### 4. `Benchmarker::run_adaptive()` Method
Main adaptive sampling implementation with 4 phases:

**Phase 1: Warmup**
- Runs warmup iterations (not timed)

**Phase 2: Initial Estimation**
- Collects initial sample (default: 5 iterations)
- Calculates preliminary mean and standard deviation
- Estimates required iterations using sample size formula

**Phase 3: Iteration Estimation**
- Computes required iterations: `n = ((σ * z) / (μ * CV_target))²`
- Clamps to [min_iterations, max_iterations]

**Phase 4: Adaptive Sampling with Early Stopping**
- Continues sampling up to target iterations
- Checks convergence every 10 iterations (after min_iterations)
- Stops early if:
  - CV ≤ target_cv_percent AND CI_width ≤ target_ci_width_percent
  - Timeout reached
- Otherwise runs until max_iterations

### Statistical Formulas

**Coefficient of Variation (CV):**
```
CV = (σ / μ) × 100%
```

**Required Sample Size:**
```
n = ((σ × z_score) / (μ × CV_target))²
```
Where z_score = 1.96 for 95% confidence

**Confidence Interval Width:**
```
CI_width = (2 × z_score × SE / μ) × 100%
where SE = σ / √n
```

## Usage Examples

### Basic Usage
```rust
use data_bridge_test::benchmark::{AdaptiveBenchmarkConfig, Benchmarker};

let benchmarker = Benchmarker::default_config();
let config = AdaptiveBenchmarkConfig::default();

let result = benchmarker.run_adaptive("my_benchmark", || {
    // Your code to benchmark
    expensive_operation()
}, config);

println!("Iterations: {}", result.stats.adaptive_iterations_used);
println!("Stopped early: {}", result.stats.adaptive_stopped_early);
```

### Custom Configuration
```rust
let config = AdaptiveBenchmarkConfig::new()
    .with_target_cv(3.0)           // 3% CV target
    .with_target_ci_width(4.0)     // 4% CI width target
    .with_min_iterations(20)       // At least 20 iterations
    .with_max_iterations(5000)     // At most 5000 iterations
    .with_timeout_ms(1000.0);      // 1 second timeout

let result = benchmarker.run_adaptive("custom_bench", || {
    my_operation()
}, config);
```

### Using Presets
```rust
// Quick benchmark (less precise, faster)
let quick_config = AdaptiveBenchmarkConfig::quick();
let result = benchmarker.run_adaptive("quick", fast_op, quick_config);

// Thorough benchmark (more precise, slower)
let thorough_config = AdaptiveBenchmarkConfig::thorough();
let result = benchmarker.run_adaptive("thorough", critical_op, thorough_config);
```

## Benefits

### 1. Efficiency
- **Fast operations**: Converge quickly with minimal iterations
- **Slow operations**: Use appropriate sample size without over-sampling
- **Variable operations**: Automatically collect more samples for accuracy

### 2. Precision
- Guarantees statistical confidence based on CV and CI criteria
- Adapts to operation variance automatically

### 3. Resource Control
- Timeout prevents runaway benchmarks
- Max iterations cap prevents excessive runtime
- Min iterations ensures statistical validity

### 4. Transparency
- Reports actual iterations used
- Provides stop reason (converged/timeout/max_iterations)
- Maintains full statistics (mean, median, percentiles, etc.)

## Test Coverage

Added 8 new tests covering:
- ✅ Configuration defaults and builders
- ✅ Quick and thorough presets
- ✅ CV calculation (including edge cases)
- ✅ Required iterations calculation
- ✅ Basic adaptive benchmarking
- ✅ Convergence detection
- ✅ Timeout handling
- ✅ Max iterations ceiling

**Total tests in benchmark.rs**: 19 tests (all passing)
**Total tests in crate**: 116 tests (all passing)

## Example Output

Run the demo:
```bash
cargo run -p data-bridge-test --example adaptive_benchmark_demo
```

Sample output:
```
1. Fast operation (adaptive with default config):
   Iterations used: 10
   Stopped early: false
   Mean: 0.001ms ± 0.000ms

3. Slow operation (adaptive with quick config):
   Iterations used: 10
   Stopped early: false
   Mean: 0.057ms ± 0.001ms

4. Variable operation (adaptive with thorough config):
   Iterations used: 20
   Stopped early: false
   Mean: 0.006ms ± 0.000ms
   CV: 1.28%
```

## Files Modified

### `/crates/data-bridge-test/src/benchmark.rs`
- Added imports: `Duration`, `black_box`
- Added `AdaptiveBenchmarkConfig` struct (93 lines)
- Extended `BenchmarkStats` with 3 adaptive fields
- Added helper functions: `calculate_cv()`, `calculate_required_iterations()`
- Implemented `Benchmarker::run_adaptive()` method (102 lines)
- Added 8 comprehensive tests
- Total additions: ~200 lines

### `/crates/data-bridge-test/examples/adaptive_benchmark_demo.rs`
- New example demonstrating all adaptive features (163 lines)

## Performance Characteristics

**Time Complexity:**
- Initial sample: O(initial_sample_size)
- Convergence checks: O(n) where n is iterations used
- Total: O(iterations_used) - linear in actual iterations

**Space Complexity:**
- O(iterations_used) for storing timings
- Pre-allocated vector to avoid reallocation

**Convergence Speed:**
- Fast, consistent operations: ~10-20 iterations (vs fixed 100+)
- Variable operations: Adapts based on variance (20-1000 iterations)
- Slow operations with timeout: Stops early to prevent excessive runtime

## Future Enhancements

Potential improvements:
1. Bayesian adaptive sampling for faster convergence
2. Outlier detection during sampling to improve accuracy
3. Multi-phase warmup for operations with JIT compilation
4. Parallel adaptive sampling for multi-threaded benchmarks
5. Auto-tuning of CV/CI targets based on operation characteristics

## References

- Statistical sample size estimation: https://en.wikipedia.org/wiki/Sample_size_determination
- Coefficient of variation: https://en.wikipedia.org/wiki/Coefficient_of_variation
- Adaptive sampling in benchmarking: https://criterion.rs/ (similar approach)
