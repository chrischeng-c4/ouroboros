//! Benchmark utilities for performance testing
//!
//! Provides structs and functions for running benchmarks with
//! timing statistics, similar to pytest-benchmark.

use std::time::Instant;
use serde::{Deserialize, Serialize};
use serde_yaml;

/// Statistics from a benchmark run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkStats {
    /// Number of iterations per round
    pub iterations: u32,
    /// Number of rounds
    pub rounds: u32,
    /// Number of warmup iterations
    pub warmup: u32,
    /// Total number of timed runs (iterations * rounds)
    pub total_runs: u32,

    // Timing stats (in milliseconds)
    /// Mean time per operation
    pub mean_ms: f64,
    /// Minimum time observed
    pub min_ms: f64,
    /// Maximum time observed
    pub max_ms: f64,
    /// Standard deviation
    pub stddev_ms: f64,
    /// Median time (P50)
    pub median_ms: f64,
    /// Total time for all runs
    pub total_ms: f64,

    // Percentiles (pytest-benchmark style)
    /// 25th percentile (Q1)
    pub p25_ms: f64,
    /// 75th percentile (Q3)
    pub p75_ms: f64,
    /// 95th percentile
    pub p95_ms: f64,
    /// 99th percentile
    pub p99_ms: f64,

    // Outlier detection (IQR-based)
    /// Interquartile range (Q3 - Q1)
    pub iqr_ms: f64,
    /// Total number of outliers
    pub outliers: u32,
    /// Outliers below Q1 - 1.5*IQR
    pub outliers_low: u32,
    /// Outliers above Q3 + 1.5*IQR
    pub outliers_high: u32,

    // Confidence interval (95%)
    /// Standard error (stddev / sqrt(n))
    pub std_error_ms: f64,
    /// 95% CI lower bound
    pub ci_lower_ms: f64,
    /// 95% CI upper bound
    pub ci_upper_ms: f64,

    /// All individual timings (in milliseconds)
    pub all_times_ms: Vec<f64>,
}

/// Calculate percentile from sorted data using linear interpolation
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len() as f64;
    let index = (p / 100.0) * (n - 1.0);
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let fraction = index - lower as f64;

    if upper >= sorted.len() {
        sorted[sorted.len() - 1]
    } else if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

/// Detect outliers using IQR method
/// Returns (total_outliers, low_outliers, high_outliers)
fn detect_outliers(sorted: &[f64], q1: f64, q3: f64) -> (u32, u32, u32) {
    let iqr = q3 - q1;
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;

    let mut low = 0u32;
    let mut high = 0u32;

    for &value in sorted {
        if value < lower_fence {
            low += 1;
        } else if value > upper_fence {
            high += 1;
        }
    }

    (low + high, low, high)
}

/// Calculate 95% confidence interval using t-distribution approximation
/// For large n (>30), t ≈ 1.96
fn confidence_interval(mean: f64, stddev: f64, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 0.0);
    }

    let std_error = stddev / (n as f64).sqrt();

    // t-value for 95% CI (approximation)
    // For n > 30, use 1.96; for smaller n, use larger values
    let t_value = if n > 120 {
        1.96
    } else if n > 60 {
        2.0
    } else if n > 30 {
        2.04
    } else if n > 20 {
        2.09
    } else if n > 10 {
        2.23
    } else {
        2.57 // n ≈ 5-10
    };

    let margin = t_value * std_error;
    (mean - margin, mean + margin)
}

impl BenchmarkStats {
    /// Create new stats from timing data
    pub fn from_times(times: Vec<f64>, iterations: u32, rounds: u32, warmup: u32) -> Self {
        let total_runs = iterations * rounds;

        if times.is_empty() {
            return Self {
                iterations,
                rounds,
                warmup,
                total_runs,
                ..Default::default()
            };
        }

        let n = times.len();
        let mean = times.iter().sum::<f64>() / n as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let total = times.iter().sum::<f64>();

        // Calculate standard deviation
        let variance = if n > 1 {
            times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        let stddev = variance.sqrt();

        // Sort for percentile calculations
        let mut sorted = times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate percentiles
        let p25 = percentile(&sorted, 25.0);
        let median = percentile(&sorted, 50.0);
        let p75 = percentile(&sorted, 75.0);
        let p95 = percentile(&sorted, 95.0);
        let p99 = percentile(&sorted, 99.0);

        // Calculate IQR and detect outliers
        let iqr = p75 - p25;
        let (outliers, outliers_low, outliers_high) = detect_outliers(&sorted, p25, p75);

        // Calculate confidence interval
        let std_error = stddev / (n as f64).sqrt();
        let (ci_lower, ci_upper) = confidence_interval(mean, stddev, n);

        Self {
            iterations,
            rounds,
            warmup,
            total_runs,
            mean_ms: mean,
            min_ms: min,
            max_ms: max,
            stddev_ms: stddev,
            median_ms: median,
            total_ms: total,
            // New percentile fields
            p25_ms: p25,
            p75_ms: p75,
            p95_ms: p95,
            p99_ms: p99,
            // Outlier detection
            iqr_ms: iqr,
            outliers,
            outliers_low,
            outliers_high,
            // Confidence interval
            std_error_ms: std_error,
            ci_lower_ms: ci_lower,
            ci_upper_ms: ci_upper,
            all_times_ms: times,
        }
    }

    /// Calculate operations per second based on mean time
    pub fn ops_per_second(&self) -> f64 {
        if self.mean_ms == 0.0 {
            0.0
        } else {
            1000.0 / self.mean_ms
        }
    }

    /// Format stats as a human-readable string
    pub fn format(&self) -> String {
        format!(
            "Mean:   {:>10.3}ms ± {:.3}ms (95% CI: {:.3}-{:.3}ms)\n\
             Min:    {:>10.3}ms\n\
             Max:    {:>10.3}ms\n\
             Stddev: {:>10.3}ms\n\
             Median: {:>10.3}ms\n\
             P25:    {:>10.3}ms  P75: {:.3}ms  P95: {:.3}ms  P99: {:.3}ms\n\
             IQR:    {:>10.3}ms  Outliers: {} ({} low, {} high)\n\
             Ops/s:  {:>10.1}\n\
             Runs:   {} ({}x{})",
            self.mean_ms, self.std_error_ms, self.ci_lower_ms, self.ci_upper_ms,
            self.min_ms,
            self.max_ms,
            self.stddev_ms,
            self.median_ms,
            self.p25_ms, self.p75_ms, self.p95_ms, self.p99_ms,
            self.iqr_ms, self.outliers, self.outliers_low, self.outliers_high,
            self.ops_per_second(),
            self.total_runs,
            self.iterations,
            self.rounds
        )
    }

    /// Format stats as a short single-line summary
    pub fn format_short(&self) -> String {
        format!(
            "{:.3}ms ± {:.3}ms (P50={:.3}ms, P95={:.3}ms)",
            self.mean_ms, self.stddev_ms, self.median_ms, self.p95_ms
        )
    }
}

/// Result of a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Name of this benchmark
    pub name: String,
    /// Timing statistics
    pub stats: BenchmarkStats,
    /// Whether benchmark completed successfully
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl BenchmarkResult {
    /// Create a successful benchmark result
    pub fn success(name: impl Into<String>, stats: BenchmarkStats) -> Self {
        Self {
            name: name.into(),
            stats,
            success: true,
            error: None,
        }
    }

    /// Create a failed benchmark result
    pub fn failure(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stats: BenchmarkStats::default(),
            success: false,
            error: Some(error.into()),
        }
    }

    /// Format result as a human-readable string
    pub fn format(&self) -> String {
        if self.success {
            format!("{}:\n{}", self.name, self.stats.format())
        } else {
            format!("{}: FAILED - {}", self.name, self.error.as_deref().unwrap_or("unknown error"))
        }
    }

    /// Print detailed statistics to stdout
    ///
    /// Formats output as:
    /// ```text
    ///   name:
    ///     Mean:    1.234ms ± 0.123ms (95% CI: 1.111-1.357ms)
    ///     Median:  1.200ms
    ///     P95:     1.500ms  P99: 1.800ms
    ///     IQR:     0.300ms  (P25=1.100, P75=1.400)
    ///     Outliers: 2 (0 low, 2 high)
    ///     Ops/s:   810.4
    ///     Runs:    100 (20x5)
    /// ```
    pub fn print_detailed(&self) {
        if !self.success {
            println!("  {}: FAILED - {}", self.name, self.error.as_deref().unwrap_or("unknown"));
            return;
        }

        let s = &self.stats;
        println!("  {}:", self.name);
        println!("    Mean:    {:>8.3}ms ± {:.3}ms (95% CI: {:.3}-{:.3}ms)",
            s.mean_ms, s.std_error_ms, s.ci_lower_ms, s.ci_upper_ms);
        println!("    Median:  {:>8.3}ms", s.median_ms);
        println!("    P95:     {:>8.3}ms  P99: {:.3}ms", s.p95_ms, s.p99_ms);
        println!("    IQR:     {:>8.3}ms  (P25={:.3}, P75={:.3})", s.iqr_ms, s.p25_ms, s.p75_ms);
        println!("    Outliers: {} ({} low, {} high)", s.outliers, s.outliers_low, s.outliers_high);
        println!("    Ops/s:   {:>8.1}", s.ops_per_second());
        println!("    Runs:    {} ({}x{})", s.total_runs, s.iterations, s.rounds);
    }
}

/// Configuration for benchmark runs
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of iterations per round
    pub iterations: u32,
    /// Number of rounds
    pub rounds: u32,
    /// Number of warmup iterations (not timed)
    pub warmup: u32,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 20,
            rounds: 3,
            warmup: 3,
        }
    }
}

impl BenchmarkConfig {
    /// Create a new benchmark configuration
    pub fn new(iterations: u32, rounds: u32, warmup: u32) -> Self {
        Self {
            iterations,
            rounds,
            warmup,
        }
    }

    /// Quick benchmark (fewer iterations)
    pub fn quick() -> Self {
        Self {
            iterations: 5,
            rounds: 2,
            warmup: 1,
        }
    }

    /// Thorough benchmark (more iterations)
    pub fn thorough() -> Self {
        Self {
            iterations: 100,
            rounds: 5,
            warmup: 10,
        }
    }

    /// Calculate optimal iterations based on a sample timing
    ///
    /// Given a sample time in milliseconds, calculates the number of
    /// iterations needed to reach the target total time.
    ///
    /// # Arguments
    /// * `sample_time_ms` - Time for a single operation in milliseconds
    /// * `target_time_ms` - Target total time for all iterations (default: 100ms)
    ///
    /// # Returns
    /// A `BenchmarkConfig` with calibrated iteration count
    pub fn calibrated(sample_time_ms: f64, target_time_ms: f64) -> Self {
        const MIN_ITERATIONS: u32 = 10;
        const MAX_ITERATIONS: u32 = 10_000;
        const DEFAULT_ROUNDS: u32 = 5;
        const DEFAULT_WARMUP: u32 = 3;

        if sample_time_ms <= 0.0 {
            return Self::default();
        }

        // Calculate iterations to reach target time
        let estimated_iters = (target_time_ms / sample_time_ms).ceil() as u32;
        let iterations = estimated_iters.clamp(MIN_ITERATIONS, MAX_ITERATIONS);

        Self {
            iterations,
            rounds: DEFAULT_ROUNDS,
            warmup: DEFAULT_WARMUP,
        }
    }
}

/// Compare multiple benchmark results
pub fn compare_results(results: &[BenchmarkResult], baseline_name: Option<&str>) -> String {
    if results.is_empty() {
        return "No results to compare".to_string();
    }

    // Find baseline (default to first successful result)
    let baseline = if let Some(name) = baseline_name {
        results.iter().find(|r| r.name == name && r.success)
    } else {
        results.iter().find(|r| r.success)
    };

    let baseline_mean = baseline.map(|b| b.stats.mean_ms).unwrap_or(1.0);

    let mut lines = vec![
        "=".repeat(75),
        format!(
            "{:<30} {:>12} {:>12} {:>15}",
            "Benchmark", "Mean (ms)", "Ops/s", "vs Baseline"
        ),
        "-".repeat(75),
    ];

    for result in results {
        if !result.success {
            lines.push(format!("{:<30} FAILED: {}", result.name, result.error.as_deref().unwrap_or("?")));
            continue;
        }

        let ratio = result.stats.mean_ms / baseline_mean;
        let vs_baseline = if baseline.map(|b| &b.name) == Some(&result.name) {
            "(baseline)".to_string()
        } else if ratio < 1.0 {
            format!("{:.2}x faster", 1.0 / ratio)
        } else {
            format!("{:.2}x slower", ratio)
        };

        lines.push(format!(
            "{:<30} {:>12.3} {:>12.1} {:>15}",
            result.name,
            result.stats.mean_ms,
            result.stats.ops_per_second(),
            vs_baseline
        ));
    }

    lines.push("=".repeat(75));
    lines.join("\n")
}

/// Print a comparison table to stdout with enhanced statistics
///
/// Output format:
/// ```text
/// Benchmark        Mean       P50       P95       P99   Outliers     vs Base
/// --------------------------------------------------------------------------------
/// data-bridge    1.234ms   1.200ms   1.500ms   1.800ms          2   (baseline)
/// httpx          2.345ms   2.300ms   2.800ms   3.100ms          1   1.90x slower
/// ```
pub fn print_comparison_table(results: &[BenchmarkResult], baseline_name: Option<&str>) {
    if results.is_empty() {
        println!("No results to compare");
        return;
    }

    // Find baseline
    let baseline = if let Some(name) = baseline_name {
        results.iter().find(|r| r.name == name && r.success)
    } else {
        results.iter().find(|r| r.success)
    };
    let baseline_mean = baseline.map(|b| b.stats.mean_ms).unwrap_or(1.0);

    // Print header
    println!();
    println!("{:<15} {:>10} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "Benchmark", "Mean", "P50", "P95", "P99", "Outliers", "vs Base");
    println!("{}", "-".repeat(82));

    // Print rows
    for result in results {
        if !result.success {
            println!("{:<15} FAILED: {}", result.name, result.error.as_deref().unwrap_or("?"));
            continue;
        }

        let s = &result.stats;
        let ratio = s.mean_ms / baseline_mean;

        let vs_base = if baseline.map(|b| &b.name) == Some(&result.name) {
            "(baseline)".to_string()
        } else if ratio < 1.0 {
            format!("{:.2}x faster", 1.0 / ratio)
        } else {
            format!("{:.2}x slower", ratio)
        };

        println!("{:<15} {:>9.3}ms {:>9.3}ms {:>9.3}ms {:>9.3}ms {:>10} {:>12}",
            result.name, s.mean_ms, s.median_ms, s.p95_ms, s.p99_ms, s.outliers, vs_base);
    }
    println!();
}

/// Synchronous benchmarker for timing operations
///
/// Note: For async operations, use the Python-side async benchmark function
/// which wraps this for timing collection.
pub struct Benchmarker {
    config: BenchmarkConfig,
}

impl Benchmarker {
    /// Create a new benchmarker with the given configuration
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(BenchmarkConfig::default())
    }

    /// Run a synchronous benchmark
    pub fn run<F, R>(&self, name: impl Into<String>, mut func: F) -> BenchmarkResult
    where
        F: FnMut() -> R,
    {
        let name = name.into();

        // Warmup
        for _ in 0..self.config.warmup {
            let _ = func();
        }

        // Timed runs
        let mut times = Vec::with_capacity((self.config.iterations * self.config.rounds) as usize);

        for _ in 0..self.config.rounds {
            for _ in 0..self.config.iterations {
                let start = Instant::now();
                let _ = func();
                let elapsed = start.elapsed();
                times.push(elapsed.as_secs_f64() * 1000.0); // Convert to ms
            }
        }

        let stats = BenchmarkStats::from_times(
            times,
            self.config.iterations,
            self.config.rounds,
            self.config.warmup,
        );

        BenchmarkResult::success(name, stats)
    }

    /// Get the configuration
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }
}

/// Benchmark report for generating HTML/JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Report title
    pub title: String,
    /// Report description
    pub description: Option<String>,
    /// When the report was generated
    pub generated_at: String,
    /// Total duration of all benchmarks (ms)
    pub total_duration_ms: f64,
    /// Benchmark groups
    pub groups: Vec<BenchmarkReportGroup>,
    /// Environment info
    pub environment: BenchmarkEnvironment,
}

/// A group of related benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReportGroup {
    /// Group name
    pub name: String,
    /// Baseline benchmark name
    pub baseline: Option<String>,
    /// Results in this group
    pub results: Vec<BenchmarkResult>,
}

/// Environment information for the benchmark
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkEnvironment {
    pub python_version: Option<String>,
    pub rust_version: Option<String>,
    pub platform: Option<String>,
    pub cpu: Option<String>,
    pub hostname: Option<String>,
}

impl BenchmarkReport {
    /// Create a new benchmark report
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            generated_at: chrono::Utc::now().to_rfc3339(),
            total_duration_ms: 0.0,
            groups: Vec::new(),
            environment: BenchmarkEnvironment::default(),
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a benchmark group
    pub fn add_group(&mut self, group: BenchmarkReportGroup) {
        self.total_duration_ms += group.results.iter()
            .map(|r| r.stats.total_ms)
            .sum::<f64>();
        self.groups.push(group);
    }

    /// Set environment info
    pub fn set_environment(&mut self, env: BenchmarkEnvironment) {
        self.environment = env;
    }

    /// Generate JSON report
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate HTML report with charts
    pub fn to_html(&self) -> String {
        let mut html = String::new();

        // HTML header with Chart.js
        html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>"#);
        html.push_str(&self.title);
        html.push_str(r#"</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0d1117; color: #c9d1d9; padding: 2rem;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { color: #58a6ff; margin-bottom: 0.5rem; }
        h2 { color: #8b949e; margin: 2rem 0 1rem; border-bottom: 1px solid #30363d; padding-bottom: 0.5rem; }
        .description { color: #8b949e; margin-bottom: 2rem; }
        .meta { display: flex; gap: 2rem; margin-bottom: 2rem; flex-wrap: wrap; }
        .meta-item { background: #161b22; padding: 1rem; border-radius: 6px; border: 1px solid #30363d; }
        .meta-label { color: #8b949e; font-size: 0.85rem; }
        .meta-value { color: #c9d1d9; font-size: 1.1rem; font-weight: 600; }
        .group { background: #161b22; border-radius: 8px; padding: 1.5rem; margin-bottom: 2rem; border: 1px solid #30363d; }
        .group-title { color: #58a6ff; margin-bottom: 1rem; }
        .chart-container { height: 300px; margin-bottom: 1.5rem; }
        table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
        th, td { padding: 0.75rem; text-align: left; border-bottom: 1px solid #30363d; }
        th { color: #8b949e; font-weight: 500; }
        td { color: #c9d1d9; }
        .faster { color: #3fb950; }
        .slower { color: #f85149; }
        .baseline { color: #8b949e; }
        .number { font-family: 'SF Mono', Monaco, monospace; }
        .env { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-top: 1rem; }
        .env-item { background: #0d1117; padding: 0.75rem; border-radius: 4px; }
        footer { margin-top: 3rem; text-align: center; color: #8b949e; font-size: 0.85rem; }
    </style>
</head>
<body>
    <div class="container">
        <h1>"#);
        html.push_str(&self.title);
        html.push_str("</h1>\n");

        if let Some(ref desc) = self.description {
            html.push_str(&format!("        <p class=\"description\">{}</p>\n", desc));
        }

        // Meta info
        html.push_str("        <div class=\"meta\">\n");
        html.push_str(&format!(
            "            <div class=\"meta-item\"><div class=\"meta-label\">Generated</div><div class=\"meta-value\">{}</div></div>\n",
            &self.generated_at[..19].replace('T', " ")
        ));
        html.push_str(&format!(
            "            <div class=\"meta-item\"><div class=\"meta-label\">Total Duration</div><div class=\"meta-value\">{:.2}s</div></div>\n",
            self.total_duration_ms / 1000.0
        ));
        html.push_str(&format!(
            "            <div class=\"meta-item\"><div class=\"meta-label\">Groups</div><div class=\"meta-value\">{}</div></div>\n",
            self.groups.len()
        ));
        let total_benchmarks: usize = self.groups.iter().map(|g| g.results.len()).sum();
        html.push_str(&format!(
            "            <div class=\"meta-item\"><div class=\"meta-label\">Benchmarks</div><div class=\"meta-value\">{}</div></div>\n",
            total_benchmarks
        ));
        html.push_str("        </div>\n\n");

        // Benchmark groups
        for (i, group) in self.groups.iter().enumerate() {
            html.push_str("        <div class=\"group\">\n");
            html.push_str(&format!("            <h2 class=\"group-title\">{}</h2>\n", group.name));
            html.push_str(&format!("            <div class=\"chart-container\"><canvas id=\"chart{}\"></canvas></div>\n", i));

            // Results table
            html.push_str("            <table>\n");
            html.push_str("                <tr><th>Benchmark</th><th>Mean</th><th>Min</th><th>Max</th><th>Stddev</th><th>Ops/s</th><th>vs Baseline</th></tr>\n");

            let baseline_mean = group.baseline.as_ref()
                .and_then(|b| group.results.iter().find(|r| &r.name == b))
                .map(|r| r.stats.mean_ms)
                .or_else(|| group.results.first().map(|r| r.stats.mean_ms))
                .unwrap_or(1.0);

            for result in &group.results {
                let ratio = result.stats.mean_ms / baseline_mean;
                let vs_baseline = if Some(&result.name) == group.baseline.as_ref() ||
                    (group.baseline.is_none() && group.results.first().map(|r| &r.name) == Some(&result.name)) {
                    "<span class=\"baseline\">(baseline)</span>".to_string()
                } else if ratio < 1.0 {
                    format!("<span class=\"faster\">{:.2}x faster</span>", 1.0 / ratio)
                } else {
                    format!("<span class=\"slower\">{:.2}x slower</span>", ratio)
                };

                html.push_str(&format!(
                    "                <tr><td>{}</td><td class=\"number\">{:.3}ms</td><td class=\"number\">{:.3}ms</td><td class=\"number\">{:.3}ms</td><td class=\"number\">{:.3}ms</td><td class=\"number\">{:.1}</td><td>{}</td></tr>\n",
                    result.name, result.stats.mean_ms, result.stats.min_ms, result.stats.max_ms,
                    result.stats.stddev_ms, result.stats.ops_per_second(), vs_baseline
                ));
            }
            html.push_str("            </table>\n");
            html.push_str("        </div>\n\n");
        }

        // Environment section
        html.push_str("        <h2>Environment</h2>\n");
        html.push_str("        <div class=\"env\">\n");
        if let Some(ref v) = self.environment.python_version {
            html.push_str(&format!("            <div class=\"env-item\"><strong>Python:</strong> {}</div>\n", v));
        }
        if let Some(ref v) = self.environment.rust_version {
            html.push_str(&format!("            <div class=\"env-item\"><strong>Rust:</strong> {}</div>\n", v));
        }
        if let Some(ref v) = self.environment.platform {
            html.push_str(&format!("            <div class=\"env-item\"><strong>Platform:</strong> {}</div>\n", v));
        }
        if let Some(ref v) = self.environment.cpu {
            html.push_str(&format!("            <div class=\"env-item\"><strong>CPU:</strong> {}</div>\n", v));
        }
        if let Some(ref v) = self.environment.hostname {
            html.push_str(&format!("            <div class=\"env-item\"><strong>Host:</strong> {}</div>\n", v));
        }
        html.push_str("        </div>\n\n");

        // Footer
        html.push_str("        <footer>Generated by data-bridge-test benchmark framework</footer>\n");

        // Chart.js scripts
        html.push_str("    </div>\n\n    <script>\n");
        for (i, group) in self.groups.iter().enumerate() {
            let labels: Vec<_> = group.results.iter().map(|r| format!("'{}'", r.name)).collect();
            let means: Vec<_> = group.results.iter().map(|r| format!("{:.3}", r.stats.mean_ms)).collect();
            let mins: Vec<_> = group.results.iter().map(|r| format!("{:.3}", r.stats.min_ms)).collect();
            let maxs: Vec<_> = group.results.iter().map(|r| format!("{:.3}", r.stats.max_ms)).collect();

            html.push_str(&format!(r#"
        new Chart(document.getElementById('chart{}'), {{
            type: 'bar',
            data: {{
                labels: [{}],
                datasets: [
                    {{ label: 'Mean (ms)', data: [{}], backgroundColor: 'rgba(88, 166, 255, 0.8)' }},
                    {{ label: 'Min (ms)', data: [{}], backgroundColor: 'rgba(63, 185, 80, 0.6)' }},
                    {{ label: 'Max (ms)', data: [{}], backgroundColor: 'rgba(248, 81, 73, 0.6)' }}
                ]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{ legend: {{ labels: {{ color: '#c9d1d9' }} }} }},
                scales: {{
                    x: {{ ticks: {{ color: '#8b949e' }}, grid: {{ color: '#30363d' }} }},
                    y: {{ ticks: {{ color: '#8b949e' }}, grid: {{ color: '#30363d' }}, title: {{ display: true, text: 'Time (ms)', color: '#8b949e' }} }}
                }}
            }}
        }});
"#, i, labels.join(", "), means.join(", "), mins.join(", "), maxs.join(", ")));
        }
        html.push_str("    </script>\n</body>\n</html>");

        html
    }

    /// Generate Markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", self.title));

        if let Some(ref desc) = self.description {
            md.push_str(&format!("{}\n\n", desc));
        }

        md.push_str(&format!("**Generated:** {}  \n", &self.generated_at[..19].replace('T', " ")));
        md.push_str(&format!("**Total Duration:** {:.2}s  \n\n", self.total_duration_ms / 1000.0));

        for group in &self.groups {
            md.push_str(&format!("## {}\n\n", group.name));
            md.push_str("| Benchmark | Mean | Min | Max | Ops/s | vs Baseline |\n");
            md.push_str("|-----------|------|-----|-----|-------|-------------|\n");

            let baseline_mean = group.baseline.as_ref()
                .and_then(|b| group.results.iter().find(|r| &r.name == b))
                .map(|r| r.stats.mean_ms)
                .or_else(|| group.results.first().map(|r| r.stats.mean_ms))
                .unwrap_or(1.0);

            for result in &group.results {
                let ratio = result.stats.mean_ms / baseline_mean;
                let vs_baseline = if Some(&result.name) == group.baseline.as_ref() ||
                    (group.baseline.is_none() && group.results.first().map(|r| &r.name) == Some(&result.name)) {
                    "(baseline)".to_string()
                } else if ratio < 1.0 {
                    format!("**{:.2}x faster**", 1.0 / ratio)
                } else {
                    format!("{:.2}x slower", ratio)
                };

                md.push_str(&format!(
                    "| {} | {:.3}ms | {:.3}ms | {:.3}ms | {:.1} | {} |\n",
                    result.name, result.stats.mean_ms, result.stats.min_ms,
                    result.stats.max_ms, result.stats.ops_per_second(), vs_baseline
                ));
            }
            md.push('\n');
        }

        // Environment
        md.push_str("## Environment\n\n");
        if let Some(ref v) = self.environment.python_version {
            md.push_str(&format!("- **Python:** {}\n", v));
        }
        if let Some(ref v) = self.environment.rust_version {
            md.push_str(&format!("- **Rust:** {}\n", v));
        }
        if let Some(ref v) = self.environment.platform {
            md.push_str(&format!("- **Platform:** {}\n", v));
        }
        if let Some(ref v) = self.environment.cpu {
            md.push_str(&format!("- **CPU:** {}\n", v));
        }

        md
    }

    /// Generate YAML report
    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(self).unwrap_or_else(|_| "# Error generating YAML".to_string())
    }

    /// Generate console output with ANSI colors
    pub fn to_console(&self) -> String {
        const RESET: &str = "\x1b[0m";
        const BOLD: &str = "\x1b[1m";
        const GREEN: &str = "\x1b[32m";
        const RED: &str = "\x1b[31m";
        const YELLOW: &str = "\x1b[33m";
        const CYAN: &str = "\x1b[36m";
        const DIM: &str = "\x1b[2m";

        let mut out = String::new();

        // Header
        out.push_str(&format!("\n{}{}═══ {} ═══{}\n\n", BOLD, CYAN, self.title, RESET));

        if let Some(ref desc) = self.description {
            out.push_str(&format!("{}{}{}\n\n", DIM, desc, RESET));
        }

        out.push_str(&format!("{}Generated:{} {}\n", BOLD, RESET, &self.generated_at[..19].replace('T', " ")));
        out.push_str(&format!("{}Total Duration:{} {:.2}s\n\n", BOLD, RESET, self.total_duration_ms / 1000.0));

        for group in &self.groups {
            out.push_str(&format!("{}{}── {} ──{}\n\n", BOLD, YELLOW, group.name, RESET));

            let baseline_mean = group.baseline.as_ref()
                .and_then(|b| group.results.iter().find(|r| &r.name == b))
                .map(|r| r.stats.mean_ms)
                .or_else(|| group.results.first().map(|r| r.stats.mean_ms))
                .unwrap_or(1.0);

            // Header row
            out.push_str(&format!("  {}{:<30} {:>10} {:>10} {:>10} {:>12} {:>15}{}\n",
                DIM, "Benchmark", "Mean", "Min", "Max", "Ops/s", "vs Baseline", RESET));
            out.push_str(&format!("  {}{}{}\n", DIM, "─".repeat(95), RESET));

            for result in &group.results {
                let ratio = result.stats.mean_ms / baseline_mean;
                let (vs_baseline, color) = if Some(&result.name) == group.baseline.as_ref() ||
                    (group.baseline.is_none() && group.results.first().map(|r| &r.name) == Some(&result.name)) {
                    ("(baseline)".to_string(), DIM)
                } else if ratio < 1.0 {
                    (format!("{:.2}x faster", 1.0 / ratio), GREEN)
                } else {
                    (format!("{:.2}x slower", ratio), RED)
                };

                out.push_str(&format!("  {:<30} {:>9.3}ms {:>9.3}ms {:>9.3}ms {:>12.1} {}{:>15}{}\n",
                    result.name, result.stats.mean_ms, result.stats.min_ms,
                    result.stats.max_ms, result.stats.ops_per_second(),
                    color, vs_baseline, RESET));
            }
            out.push('\n');
        }

        // Environment
        out.push_str(&format!("{}{}Environment{}\n", BOLD, CYAN, RESET));
        if let Some(ref v) = self.environment.python_version {
            out.push_str(&format!("  Python:   {}\n", v));
        }
        if let Some(ref v) = self.environment.rust_version {
            out.push_str(&format!("  Rust:     {}\n", v));
        }
        if let Some(ref v) = self.environment.platform {
            out.push_str(&format!("  Platform: {}\n", v));
        }
        if let Some(ref v) = self.environment.cpu {
            out.push_str(&format!("  CPU:      {}\n", v));
        }

        out
    }
}

impl BenchmarkReportGroup {
    /// Create a new benchmark group
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            baseline: None,
            results: Vec::new(),
        }
    }

    /// Set the baseline benchmark name
    pub fn with_baseline(mut self, baseline: impl Into<String>) -> Self {
        self.baseline = Some(baseline.into());
        self
    }

    /// Add a result to this group
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_stats_calculation() {
        let times = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = BenchmarkStats::from_times(times, 5, 1, 0);

        assert!((stats.mean_ms - 3.0).abs() < 0.001);
        assert!((stats.min_ms - 1.0).abs() < 0.001);
        assert!((stats.max_ms - 5.0).abs() < 0.001);
        assert!((stats.median_ms - 3.0).abs() < 0.001);
        assert!((stats.total_ms - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_percentile_calculation() {
        // Test with 100 values (0-99)
        let times: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let stats = BenchmarkStats::from_times(times, 100, 1, 0);

        // P25 should be ~24.75, P50 should be ~49.5, P75 should be ~74.25
        assert!((stats.p25_ms - 24.75).abs() < 0.1, "P25 was {}", stats.p25_ms);
        assert!((stats.median_ms - 49.5).abs() < 0.1, "Median was {}", stats.median_ms);
        assert!((stats.p75_ms - 74.25).abs() < 0.1, "P75 was {}", stats.p75_ms);
        assert!((stats.p95_ms - 94.05).abs() < 0.1, "P95 was {}", stats.p95_ms);
        assert!((stats.p99_ms - 98.01).abs() < 0.1, "P99 was {}", stats.p99_ms);
    }

    #[test]
    fn test_iqr_and_outliers() {
        // Create data with outliers: [1, 2, 3, 4, 5, 6, 7, 8, 9, 100]
        // Using linear interpolation:
        // Q1 (index 2.25): between 3 and 4 = 3.25
        // Q3 (index 6.75): between 7 and 8 = 7.75
        // IQR = 7.75 - 3.25 = 4.5
        // Lower fence = 3.25 - 1.5*4.5 = -3.5
        // Upper fence = 7.75 + 1.5*4.5 = 14.5
        // So 100 is an outlier (high)
        let times = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0];
        let stats = BenchmarkStats::from_times(times, 10, 1, 0);

        assert!((stats.iqr_ms - 4.5).abs() < 0.1, "IQR was {}", stats.iqr_ms);
        assert_eq!(stats.outliers, 1, "Expected 1 outlier, got {}", stats.outliers);
        assert_eq!(stats.outliers_high, 1, "Expected 1 high outlier");
        assert_eq!(stats.outliers_low, 0, "Expected 0 low outliers");
    }

    #[test]
    fn test_confidence_interval() {
        // With stddev=1, n=100, std_error = 0.1
        // 95% CI with t≈2.0 should be mean ± 0.2
        let times: Vec<f64> = (0..100).map(|_| 10.0).collect(); // All same value
        let stats = BenchmarkStats::from_times(times, 100, 1, 0);

        assert!((stats.mean_ms - 10.0).abs() < 0.001);
        assert!((stats.stddev_ms - 0.0).abs() < 0.001);
        assert!((stats.ci_lower_ms - 10.0).abs() < 0.001);
        assert!((stats.ci_upper_ms - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_confidence_interval_with_variance() {
        // Mean = 50, stddev ≈ 29.15
        let times: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let stats = BenchmarkStats::from_times(times, 100, 1, 0);

        // std_error = 29.15 / 10 = 2.915
        // CI should be roughly 50 ± 5.7
        assert!(stats.ci_lower_ms < stats.mean_ms);
        assert!(stats.ci_upper_ms > stats.mean_ms);
        assert!((stats.ci_upper_ms - stats.ci_lower_ms) > 5.0);
    }

    #[test]
    fn test_percentile_edge_cases() {
        // Single value
        let stats = BenchmarkStats::from_times(vec![5.0], 1, 1, 0);
        assert!((stats.p25_ms - 5.0).abs() < 0.001);
        assert!((stats.p75_ms - 5.0).abs() < 0.001);

        // Two values
        let stats = BenchmarkStats::from_times(vec![1.0, 10.0], 2, 1, 0);
        assert!((stats.p25_ms - 3.25).abs() < 0.01);
        assert!((stats.median_ms - 5.5).abs() < 0.01);
        assert!((stats.p75_ms - 7.75).abs() < 0.01);
    }

    #[test]
    fn test_benchmarker_sync() {
        let benchmarker = Benchmarker::new(BenchmarkConfig::new(3, 2, 1));

        let result = benchmarker.run("test_add", || {
            let mut sum = 0;
            for i in 0..1000 {
                sum += i;
            }
            sum
        });

        assert!(result.success);
        assert_eq!(result.stats.iterations, 3);
        assert_eq!(result.stats.rounds, 2);
        assert_eq!(result.stats.total_runs, 6);
        assert!(result.stats.mean_ms > 0.0);
        // Verify new fields are populated
        assert!(result.stats.p95_ms >= result.stats.median_ms);
    }

    #[test]
    fn test_compare_results() {
        let stats1 = BenchmarkStats::from_times(vec![1.0, 1.1, 0.9], 3, 1, 0);
        let stats2 = BenchmarkStats::from_times(vec![2.0, 2.1, 1.9], 3, 1, 0);

        let results = vec![
            BenchmarkResult::success("fast", stats1),
            BenchmarkResult::success("slow", stats2),
        ];

        let comparison = compare_results(&results, Some("fast"));
        assert!(comparison.contains("fast"));
        assert!(comparison.contains("slow"));
        assert!(comparison.contains("baseline"));
        assert!(comparison.contains("slower"));
    }

    #[test]
    fn test_format_stats() {
        let times = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = BenchmarkStats::from_times(times, 5, 1, 0);

        let formatted = stats.format();
        assert!(formatted.contains("Mean:"));
        assert!(formatted.contains("P25:"));
        assert!(formatted.contains("P75:"));
        assert!(formatted.contains("P95:"));
        assert!(formatted.contains("IQR:"));
        assert!(formatted.contains("Outliers:"));

        let short = stats.format_short();
        assert!(short.contains("P50="));
        assert!(short.contains("P95="));
    }
}
