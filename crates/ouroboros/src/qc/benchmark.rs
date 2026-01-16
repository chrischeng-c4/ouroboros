//! Benchmark types and functions.

use ouroboros_qc::benchmark::{
    BenchmarkConfig, BenchmarkResult, BenchmarkStats,
    BenchmarkReport, BenchmarkReportGroup, BenchmarkEnvironment,
};
use pyo3::prelude::*;

// =====================
// BenchmarkStats
// =====================

/// Python BenchmarkStats class
#[pyclass(name = "BenchmarkStats")]
#[derive(Clone)]
pub struct PyBenchmarkStats {
    pub(super) inner: BenchmarkStats,
}

#[pymethods]
impl PyBenchmarkStats {
    /// Number of iterations per round
    #[getter]
    fn iterations(&self) -> u32 {
        self.inner.iterations
    }

    /// Number of rounds
    #[getter]
    fn rounds(&self) -> u32 {
        self.inner.rounds
    }

    /// Number of warmup iterations
    #[getter]
    fn warmup(&self) -> u32 {
        self.inner.warmup
    }

    /// Total number of timed runs
    #[getter]
    fn total_runs(&self) -> u32 {
        self.inner.total_runs
    }

    /// Mean time per operation (ms)
    #[getter]
    fn mean_ms(&self) -> f64 {
        self.inner.mean_ms
    }

    /// Minimum time observed (ms)
    #[getter]
    fn min_ms(&self) -> f64 {
        self.inner.min_ms
    }

    /// Maximum time observed (ms)
    #[getter]
    fn max_ms(&self) -> f64 {
        self.inner.max_ms
    }

    /// Standard deviation (ms)
    #[getter]
    fn stddev_ms(&self) -> f64 {
        self.inner.stddev_ms
    }

    /// Median time (ms)
    #[getter]
    fn median_ms(&self) -> f64 {
        self.inner.median_ms
    }

    /// Total time for all runs (ms)
    #[getter]
    fn total_ms(&self) -> f64 {
        self.inner.total_ms
    }

    /// All individual timings (ms)
    #[getter]
    fn all_times_ms(&self) -> Vec<f64> {
        self.inner.all_times_ms.clone()
    }

    // Percentiles
    /// 25th percentile (Q1)
    #[getter]
    fn p25_ms(&self) -> f64 {
        self.inner.p25_ms
    }

    /// 75th percentile (Q3)
    #[getter]
    fn p75_ms(&self) -> f64 {
        self.inner.p75_ms
    }

    /// 95th percentile
    #[getter]
    fn p95_ms(&self) -> f64 {
        self.inner.p95_ms
    }

    /// 99th percentile
    #[getter]
    fn p99_ms(&self) -> f64 {
        self.inner.p99_ms
    }

    // Outlier detection
    /// Interquartile range (Q3 - Q1)
    #[getter]
    fn iqr_ms(&self) -> f64 {
        self.inner.iqr_ms
    }

    /// Total number of outliers
    #[getter]
    fn outliers(&self) -> u32 {
        self.inner.outliers
    }

    /// Outliers below Q1 - 1.5*IQR
    #[getter]
    fn outliers_low(&self) -> u32 {
        self.inner.outliers_low
    }

    /// Outliers above Q3 + 1.5*IQR
    #[getter]
    fn outliers_high(&self) -> u32 {
        self.inner.outliers_high
    }

    // Confidence interval
    /// Standard error (stddev / sqrt(n))
    #[getter]
    fn std_error_ms(&self) -> f64 {
        self.inner.std_error_ms
    }

    /// 95% CI lower bound
    #[getter]
    fn ci_lower_ms(&self) -> f64 {
        self.inner.ci_lower_ms
    }

    /// 95% CI upper bound
    #[getter]
    fn ci_upper_ms(&self) -> f64 {
        self.inner.ci_upper_ms
    }

    /// Calculate operations per second
    fn ops_per_second(&self) -> f64 {
        self.inner.ops_per_second()
    }

    /// Format stats as human-readable string
    fn format(&self) -> String {
        self.inner.format()
    }

    /// Format stats as short single-line summary
    fn format_short(&self) -> String {
        self.inner.format_short()
    }

    fn __repr__(&self) -> String {
        format!(
            "BenchmarkStats(mean={:.3}ms ± {:.3}ms, P50={:.3}ms, P95={:.3}ms, outliers={})",
            self.inner.mean_ms, self.inner.stddev_ms, self.inner.median_ms,
            self.inner.p95_ms, self.inner.outliers
        )
    }
}

// =====================
// BenchmarkResult
// =====================

/// Python BenchmarkResult class
#[pyclass(name = "BenchmarkResult")]
#[derive(Clone)]
pub struct PyBenchmarkResult {
    pub(super) inner: BenchmarkResult,
}

#[pymethods]
impl PyBenchmarkResult {
    /// Create a new benchmark result from collected times
    #[staticmethod]
    #[pyo3(signature = (name, times_ms, iterations = 20, rounds = 3, warmup = 3))]
    fn from_times(
        name: String,
        times_ms: Vec<f64>,
        iterations: u32,
        rounds: u32,
        warmup: u32,
    ) -> Self {
        let stats = BenchmarkStats::from_times(times_ms, iterations, rounds, warmup);
        Self {
            inner: BenchmarkResult::success(name, stats),
        }
    }

    /// Create a failed benchmark result
    #[staticmethod]
    fn failure(name: String, error: String) -> Self {
        Self {
            inner: BenchmarkResult::failure(name, error),
        }
    }

    /// Name of this benchmark
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Timing statistics
    #[getter]
    fn stats(&self) -> PyBenchmarkStats {
        PyBenchmarkStats {
            inner: self.inner.stats.clone(),
        }
    }

    /// Whether benchmark completed successfully
    #[getter]
    fn success(&self) -> bool {
        self.inner.success
    }

    /// Error message if failed
    #[getter]
    fn error(&self) -> Option<&str> {
        self.inner.error.as_deref()
    }

    /// Format result as human-readable string
    fn format(&self) -> String {
        self.inner.format()
    }

    /// Print detailed statistics to stdout
    ///
    /// Formats output with Mean ± SE, Median, P95/P99, IQR, Outliers, Ops/s
    fn print_detailed(&self) {
        self.inner.print_detailed();
    }

    fn __repr__(&self) -> String {
        if self.inner.success {
            format!(
                "BenchmarkResult(name='{}', mean={:.3}ms, ops/s={:.1})",
                self.inner.name,
                self.inner.stats.mean_ms,
                self.inner.stats.ops_per_second()
            )
        } else {
            format!(
                "BenchmarkResult(name='{}', FAILED: {})",
                self.inner.name,
                self.inner.error.as_deref().unwrap_or("unknown")
            )
        }
    }
}

// =====================
// BenchmarkConfig
// =====================

/// Python BenchmarkConfig class
#[pyclass(name = "BenchmarkConfig")]
#[derive(Clone)]
pub struct PyBenchmarkConfig {
    inner: BenchmarkConfig,
}

#[pymethods]
impl PyBenchmarkConfig {
    /// Create a new benchmark configuration
    #[new]
    #[pyo3(signature = (iterations = 20, rounds = 3, warmup = 3))]
    fn new(iterations: u32, rounds: u32, warmup: u32) -> Self {
        Self {
            inner: BenchmarkConfig::new(iterations, rounds, warmup),
        }
    }

    /// Create a quick benchmark configuration
    #[staticmethod]
    fn quick() -> Self {
        Self {
            inner: BenchmarkConfig::quick(),
        }
    }

    /// Create a thorough benchmark configuration
    #[staticmethod]
    fn thorough() -> Self {
        Self {
            inner: BenchmarkConfig::thorough(),
        }
    }

    /// Create a calibrated benchmark configuration
    ///
    /// Automatically determines optimal iterations based on sample timing.
    ///
    /// # Arguments
    /// * `sample_time_ms` - Time for a single operation in milliseconds
    /// * `target_time_ms` - Target total time (default: 100ms)
    #[staticmethod]
    #[pyo3(signature = (sample_time_ms, target_time_ms = 100.0))]
    fn calibrated(sample_time_ms: f64, target_time_ms: f64) -> Self {
        Self {
            inner: BenchmarkConfig::calibrated(sample_time_ms, target_time_ms),
        }
    }

    /// Number of iterations per round
    #[getter]
    fn iterations(&self) -> u32 {
        self.inner.iterations
    }

    /// Number of rounds
    #[getter]
    fn rounds(&self) -> u32 {
        self.inner.rounds
    }

    /// Number of warmup iterations
    #[getter]
    fn warmup(&self) -> u32 {
        self.inner.warmup
    }

    fn __repr__(&self) -> String {
        format!(
            "BenchmarkConfig(iterations={}, rounds={}, warmup={})",
            self.inner.iterations, self.inner.rounds, self.inner.warmup
        )
    }
}

// =====================
// Benchmark functions
// =====================

/// Compare multiple benchmark results and return formatted comparison
#[pyfunction]
#[pyo3(signature = (results, baseline_name = None))]
pub fn compare_benchmarks(results: Vec<PyBenchmarkResult>, baseline_name: Option<&str>) -> String {
    let rust_results: Vec<BenchmarkResult> = results.iter().map(|r| r.inner.clone()).collect();
    ouroboros_qc::benchmark::compare_results(&rust_results, baseline_name)
}

/// Print a comparison table to stdout with enhanced statistics
///
/// Output format:
/// ```text
/// Benchmark        Mean       P50       P95       P99   Outliers     vs Base
/// --------------------------------------------------------------------------------
/// ouroboros    1.234ms   1.200ms   1.500ms   1.800ms          2   (baseline)
/// httpx          2.345ms   2.300ms   2.800ms   3.100ms          1   1.90x slower
/// ```
#[pyfunction]
#[pyo3(signature = (results, baseline_name = None))]
pub fn print_comparison_table(results: Vec<PyBenchmarkResult>, baseline_name: Option<&str>) {
    let rust_results: Vec<BenchmarkResult> = results.iter().map(|r| r.inner.clone()).collect();
    ouroboros_qc::benchmark::print_comparison_table(&rust_results, baseline_name);
}

// =====================
// Benchmark Report
// =====================

/// Python BenchmarkEnvironment class
#[pyclass(name = "BenchmarkEnvironment")]
#[derive(Clone)]
pub struct PyBenchmarkEnvironment {
    inner: BenchmarkEnvironment,
}

#[pymethods]
impl PyBenchmarkEnvironment {
    #[new]
    #[pyo3(signature = (python_version = None, rust_version = None, platform = None, cpu = None, hostname = None))]
    fn new(
        python_version: Option<String>,
        rust_version: Option<String>,
        platform: Option<String>,
        cpu: Option<String>,
        hostname: Option<String>,
    ) -> Self {
        Self {
            inner: BenchmarkEnvironment {
                python_version,
                rust_version,
                platform,
                cpu,
                hostname,
            },
        }
    }

    #[getter]
    fn python_version(&self) -> Option<&str> { self.inner.python_version.as_deref() }
    #[getter]
    fn rust_version(&self) -> Option<&str> { self.inner.rust_version.as_deref() }
    #[getter]
    fn platform(&self) -> Option<&str> { self.inner.platform.as_deref() }
    #[getter]
    fn cpu(&self) -> Option<&str> { self.inner.cpu.as_deref() }
    #[getter]
    fn hostname(&self) -> Option<&str> { self.inner.hostname.as_deref() }
}

/// Python BenchmarkReportGroup class
#[pyclass(name = "BenchmarkReportGroup")]
#[derive(Clone)]
pub struct PyBenchmarkReportGroup {
    inner: BenchmarkReportGroup,
}

#[pymethods]
impl PyBenchmarkReportGroup {
    #[new]
    #[pyo3(signature = (name, baseline = None))]
    fn new(name: String, baseline: Option<String>) -> Self {
        let mut group = BenchmarkReportGroup::new(name);
        if let Some(b) = baseline {
            group = group.with_baseline(b);
        }
        Self { inner: group }
    }

    #[getter]
    fn name(&self) -> &str { &self.inner.name }

    #[getter]
    fn baseline(&self) -> Option<&str> { self.inner.baseline.as_deref() }

    #[getter]
    fn results(&self) -> Vec<PyBenchmarkResult> {
        self.inner.results.iter().map(|r| PyBenchmarkResult { inner: r.clone() }).collect()
    }

    /// Add a result to this group
    fn add_result(&mut self, result: &PyBenchmarkResult) {
        self.inner.add_result(result.inner.clone());
    }
}

/// Python BenchmarkReport class
#[pyclass(name = "BenchmarkReport")]
#[derive(Clone)]
pub struct PyBenchmarkReport {
    inner: BenchmarkReport,
}

#[pymethods]
impl PyBenchmarkReport {
    #[new]
    #[pyo3(signature = (title, description = None))]
    fn new(title: String, description: Option<String>) -> Self {
        let mut report = BenchmarkReport::new(title);
        if let Some(desc) = description {
            report = report.with_description(desc);
        }
        Self { inner: report }
    }

    #[getter]
    fn title(&self) -> &str { &self.inner.title }

    #[getter]
    fn description(&self) -> Option<&str> { self.inner.description.as_deref() }

    #[getter]
    fn generated_at(&self) -> &str { &self.inner.generated_at }

    #[getter]
    fn total_duration_ms(&self) -> f64 { self.inner.total_duration_ms }

    #[getter]
    fn groups(&self) -> Vec<PyBenchmarkReportGroup> {
        self.inner.groups.iter().map(|g| PyBenchmarkReportGroup { inner: g.clone() }).collect()
    }

    /// Add a benchmark group
    fn add_group(&mut self, group: &PyBenchmarkReportGroup) {
        self.inner.add_group(group.inner.clone());
    }

    /// Set environment info
    fn set_environment(&mut self, env: &PyBenchmarkEnvironment) {
        self.inner.set_environment(env.inner.clone());
    }

    /// Generate JSON report
    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    /// Generate HTML report with charts
    fn to_html(&self) -> String {
        self.inner.to_html()
    }

    /// Generate Markdown report
    fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    /// Generate YAML report
    fn to_yaml(&self) -> String {
        self.inner.to_yaml()
    }

    /// Generate console output with ANSI colors
    fn to_console(&self) -> String {
        self.inner.to_console()
    }

    /// Save report to file
    fn save(&self, path: &str, format: &str) -> PyResult<()> {
        let content = match format {
            "html" => self.inner.to_html(),
            "json" => self.inner.to_json(),
            "yaml" | "yml" => self.inner.to_yaml(),
            "markdown" | "md" => self.inner.to_markdown(),
            "console" => self.inner.to_console(),
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Unknown format: {}. Use 'html', 'json', 'yaml', 'markdown', or 'console'", format)
            )),
        };

        std::fs::write(path, content)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
    }
}
