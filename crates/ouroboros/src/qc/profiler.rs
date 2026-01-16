//! Profiler types.

use ouroboros_qc::{
    PhaseTiming, PhaseBreakdown,
    GilTestConfig, GilContentionResult,
    MemorySnapshot, MemoryProfile,
    FlamegraphData, ProfileResult, ProfileConfig,
    generate_flamegraph_svg,
};
use pyo3::prelude::*;

// =====================
// PhaseTiming
// =====================

/// Python PhaseTiming class
#[pyclass(name = "PhaseTiming")]
#[derive(Clone)]
pub struct PyPhaseTiming {
    inner: PhaseTiming,
}

#[pymethods]
impl PyPhaseTiming {
    /// Total time in nanoseconds
    #[getter]
    fn total_ns(&self) -> u64 {
        self.inner.total_ns
    }

    /// Number of samples
    #[getter]
    fn count(&self) -> u64 {
        self.inner.count
    }

    /// Minimum time in nanoseconds
    #[getter]
    fn min_ns(&self) -> u64 {
        self.inner.min_ns
    }

    /// Maximum time in nanoseconds
    #[getter]
    fn max_ns(&self) -> u64 {
        self.inner.max_ns
    }

    /// Average time in nanoseconds
    #[getter]
    fn avg_ns(&self) -> f64 {
        self.inner.avg_ns()
    }

    /// Total time in milliseconds
    #[getter]
    fn total_ms(&self) -> f64 {
        self.inner.total_ms()
    }

    /// Average time in milliseconds
    #[getter]
    fn avg_ms(&self) -> f64 {
        self.inner.avg_ms()
    }

    fn __repr__(&self) -> String {
        format!(
            "PhaseTiming(total={:.3}ms, count={}, avg={:.3}ms)",
            self.total_ms(),
            self.inner.count,
            self.avg_ms()
        )
    }
}

// =====================
// PhaseBreakdown
// =====================

/// Python PhaseBreakdown class
#[pyclass(name = "PhaseBreakdown")]
#[derive(Clone)]
pub struct PyPhaseBreakdown {
    inner: PhaseBreakdown,
}

#[pymethods]
impl PyPhaseBreakdown {
    /// Get timing for a specific phase
    fn get_phase(&self, phase_name: &str) -> Option<PyPhaseTiming> {
        self.inner
            .get_phase(phase_name)
            .map(|t| PyPhaseTiming { inner: t.clone() })
    }

    /// Get all phase names
    fn phase_names(&self) -> Vec<String> {
        self.inner.phase_names()
    }

    /// Get operation count
    #[getter]
    fn operation_count(&self) -> u64 {
        self.inner.operation_count
    }

    /// Get total duration in milliseconds
    #[getter]
    fn total_duration_ms(&self) -> f64 {
        self.inner.total_duration_ms()
    }

    /// Get percentage breakdown
    fn percentage_breakdown(&self) -> std::collections::HashMap<String, f64> {
        self.inner.percentage_breakdown()
    }

    /// Format as human-readable string
    fn format(&self) -> String {
        self.inner.format()
    }

    fn __repr__(&self) -> String {
        format!(
            "PhaseBreakdown(operations={}, duration={:.2}ms, phases={})",
            self.inner.operation_count,
            self.total_duration_ms(),
            self.inner.phases.len()
        )
    }
}

// =====================
// GilTestConfig
// =====================

/// Python GilTestConfig class
#[pyclass(name = "GilTestConfig")]
#[derive(Clone)]
pub struct PyGilTestConfig {
    pub(super) inner: GilTestConfig,
}

#[pymethods]
impl PyGilTestConfig {
    #[new]
    #[pyo3(signature = (concurrent_workers=4, duration_secs=10.0, operations_per_worker=100, warmup_iterations=3))]
    fn new(
        concurrent_workers: usize,
        duration_secs: f64,
        operations_per_worker: u64,
        warmup_iterations: u32,
    ) -> Self {
        Self {
            inner: GilTestConfig {
                concurrent_workers,
                duration_secs,
                operations_per_worker,
                warmup_iterations,
            },
        }
    }

    #[getter]
    fn concurrent_workers(&self) -> usize {
        self.inner.concurrent_workers
    }

    #[getter]
    fn duration_secs(&self) -> f64 {
        self.inner.duration_secs
    }

    #[getter]
    fn operations_per_worker(&self) -> u64 {
        self.inner.operations_per_worker
    }

    #[getter]
    fn warmup_iterations(&self) -> u32 {
        self.inner.warmup_iterations
    }

    fn __repr__(&self) -> String {
        format!(
            "GilTestConfig(workers={}, ops_per_worker={})",
            self.inner.concurrent_workers, self.inner.operations_per_worker
        )
    }
}

// =====================
// GilContentionResult
// =====================

/// Python GilContentionResult class
#[pyclass(name = "GilContentionResult")]
#[derive(Clone)]
pub struct PyGilContentionResult {
    inner: GilContentionResult,
}

#[pymethods]
impl PyGilContentionResult {
    #[getter]
    fn sequential_baseline_ms(&self) -> f64 {
        self.inner.sequential_baseline_ms
    }

    #[getter]
    fn concurrent_total_ms(&self) -> f64 {
        self.inner.concurrent_total_ms
    }

    #[getter]
    fn worker_times_ms(&self) -> Vec<f64> {
        self.inner.worker_times_ms.clone()
    }

    #[getter]
    fn overhead_percent(&self) -> f64 {
        self.inner.overhead_percent
    }

    #[getter]
    fn gil_release_effective(&self) -> bool {
        self.inner.gil_release_effective
    }

    #[getter]
    fn theoretical_speedup(&self) -> f64 {
        self.inner.theoretical_speedup
    }

    #[getter]
    fn actual_speedup(&self) -> f64 {
        self.inner.actual_speedup
    }

    #[getter]
    fn efficiency_percent(&self) -> f64 {
        self.inner.efficiency_percent
    }

    /// Format as human-readable string
    fn format(&self) -> String {
        self.inner.format()
    }

    fn __repr__(&self) -> String {
        format!(
            "GilContentionResult(effective={}, speedup={:.2}x, efficiency={:.1}%)",
            self.inner.gil_release_effective,
            self.inner.actual_speedup,
            self.inner.efficiency_percent
        )
    }
}

// =====================
// MemorySnapshot
// =====================

/// Python MemorySnapshot class
#[pyclass(name = "MemorySnapshot")]
#[derive(Clone)]
pub struct PyMemorySnapshot {
    inner: MemorySnapshot,
}

#[pymethods]
impl PyMemorySnapshot {
    #[getter]
    fn rss_bytes(&self) -> u64 {
        self.inner.rss_bytes
    }

    #[getter]
    fn peak_rss_bytes(&self) -> u64 {
        self.inner.peak_rss_bytes
    }

    #[getter]
    fn rss_mb(&self) -> f64 {
        self.inner.rss_mb()
    }

    #[getter]
    fn peak_rss_mb(&self) -> f64 {
        self.inner.peak_rss_mb()
    }

    fn __repr__(&self) -> String {
        format!("MemorySnapshot(rss={:.2}MB)", self.rss_mb())
    }
}

// =====================
// MemoryProfile
// =====================

/// Python MemoryProfile class
#[pyclass(name = "MemoryProfile")]
#[derive(Clone)]
pub struct PyMemoryProfile {
    inner: MemoryProfile,
}

#[pymethods]
impl PyMemoryProfile {
    #[getter]
    fn before(&self) -> PyMemorySnapshot {
        PyMemorySnapshot {
            inner: self.inner.before.clone(),
        }
    }

    #[getter]
    fn after(&self) -> PyMemorySnapshot {
        PyMemorySnapshot {
            inner: self.inner.after.clone(),
        }
    }

    #[getter]
    fn peak(&self) -> PyMemorySnapshot {
        PyMemorySnapshot {
            inner: self.inner.peak.clone(),
        }
    }

    #[getter]
    fn delta_bytes(&self) -> i64 {
        self.inner.delta_bytes
    }

    #[getter]
    fn delta_mb(&self) -> f64 {
        self.inner.delta_mb()
    }

    #[getter]
    fn peak_rss_mb(&self) -> f64 {
        self.inner.peak_rss_mb()
    }

    #[getter]
    fn iterations(&self) -> u64 {
        self.inner.iterations
    }

    /// Format as human-readable string
    fn format(&self) -> String {
        self.inner.format()
    }

    fn __repr__(&self) -> String {
        format!(
            "MemoryProfile(delta={:+.2}MB, peak={:.2}MB)",
            self.delta_mb(),
            self.peak_rss_mb()
        )
    }
}

// =====================
// FlamegraphData
// =====================

/// Python FlamegraphData class
#[pyclass(name = "FlamegraphData")]
#[derive(Clone)]
pub struct PyFlamegraphData {
    inner: FlamegraphData,
}

#[pymethods]
impl PyFlamegraphData {
    #[new]
    fn new() -> Self {
        Self {
            inner: FlamegraphData::new(),
        }
    }

    /// Add a folded stack sample
    fn add_stack(&mut self, stack: String) {
        self.inner.add_stack(stack);
    }

    #[getter]
    fn folded_stacks(&self) -> Vec<String> {
        self.inner.folded_stacks.clone()
    }

    #[getter]
    fn sample_count(&self) -> u64 {
        self.inner.sample_count
    }

    /// Check if there's data
    fn has_data(&self) -> bool {
        self.inner.has_data()
    }

    fn __repr__(&self) -> String {
        format!("FlamegraphData(samples={})", self.inner.sample_count)
    }
}

// =====================
// ProfileResult
// =====================

/// Python ProfileResult class
#[pyclass(name = "ProfileResult")]
#[derive(Clone)]
pub struct PyProfileResult {
    inner: ProfileResult,
}

#[pymethods]
impl PyProfileResult {
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn started_at(&self) -> &str {
        &self.inner.started_at
    }

    #[getter]
    fn ended_at(&self) -> &str {
        &self.inner.ended_at
    }

    #[getter]
    fn duration_ms(&self) -> f64 {
        self.inner.duration_ms
    }

    #[getter]
    fn success(&self) -> bool {
        self.inner.success
    }

    #[getter]
    fn error(&self) -> Option<&str> {
        self.inner.error.as_deref()
    }

    #[getter]
    fn phase_breakdown(&self) -> Option<PyPhaseBreakdown> {
        self.inner
            .phase_breakdown
            .as_ref()
            .map(|pb| PyPhaseBreakdown { inner: pb.clone() })
    }

    #[getter]
    fn gil_analysis(&self) -> Option<PyGilContentionResult> {
        self.inner
            .gil_analysis
            .as_ref()
            .map(|ga| PyGilContentionResult { inner: ga.clone() })
    }

    #[getter]
    fn memory_profile(&self) -> Option<PyMemoryProfile> {
        self.inner
            .memory_profile
            .as_ref()
            .map(|mp| PyMemoryProfile { inner: mp.clone() })
    }

    #[getter]
    fn flamegraph(&self) -> Option<PyFlamegraphData> {
        self.inner
            .flamegraph
            .as_ref()
            .map(|fg| PyFlamegraphData { inner: fg.clone() })
    }

    /// Format as human-readable string
    fn format(&self) -> String {
        self.inner.format()
    }

    /// Export to JSON
    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProfileResult(name='{}', duration={:.2}ms, success={})",
            self.inner.name, self.inner.duration_ms, self.inner.success
        )
    }
}

// =====================
// ProfileConfig
// =====================

/// Python ProfileConfig class
#[pyclass(name = "ProfileConfig")]
#[derive(Clone)]
pub struct PyProfileConfig {
    inner: ProfileConfig,
}

#[pymethods]
impl PyProfileConfig {
    #[new]
    #[pyo3(signature = (
        enable_phase_breakdown=true,
        enable_gil_analysis=false,
        enable_memory_profile=false,
        enable_flamegraph=false,
        iterations=100,
        warmup=10,
        output_dir=None
    ))]
    fn new(
        enable_phase_breakdown: bool,
        enable_gil_analysis: bool,
        enable_memory_profile: bool,
        enable_flamegraph: bool,
        iterations: u32,
        warmup: u32,
        output_dir: Option<String>,
    ) -> Self {
        Self {
            inner: ProfileConfig {
                enable_phase_breakdown,
                enable_gil_analysis,
                enable_memory_profile,
                enable_flamegraph,
                iterations,
                warmup,
                gil_config: GilTestConfig::default(),
                output_dir,
            },
        }
    }

    /// Create full profiling config
    #[staticmethod]
    fn full() -> Self {
        Self {
            inner: ProfileConfig::full(),
        }
    }

    /// Create quick profiling config
    #[staticmethod]
    fn quick() -> Self {
        Self {
            inner: ProfileConfig::quick(),
        }
    }

    #[getter]
    fn enable_phase_breakdown(&self) -> bool {
        self.inner.enable_phase_breakdown
    }

    #[getter]
    fn enable_gil_analysis(&self) -> bool {
        self.inner.enable_gil_analysis
    }

    #[getter]
    fn enable_memory_profile(&self) -> bool {
        self.inner.enable_memory_profile
    }

    #[getter]
    fn enable_flamegraph(&self) -> bool {
        self.inner.enable_flamegraph
    }

    #[getter]
    fn iterations(&self) -> u32 {
        self.inner.iterations
    }

    #[getter]
    fn warmup(&self) -> u32 {
        self.inner.warmup
    }

    #[getter]
    fn output_dir(&self) -> Option<&str> {
        self.inner.output_dir.as_deref()
    }

    #[getter]
    fn gil_config(&self) -> PyGilTestConfig {
        PyGilTestConfig {
            inner: self.inner.gil_config.clone(),
        }
    }

    /// Set GIL test configuration
    fn with_gil_config(&self, config: &PyGilTestConfig) -> Self {
        Self {
            inner: self.inner.clone().with_gil_config(config.inner.clone()),
        }
    }

    /// Set output directory
    fn with_output_dir(&self, dir: String) -> Self {
        Self {
            inner: self.inner.clone().with_output_dir(dir),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ProfileConfig(phases={}, gil={}, memory={}, flamegraph={}, iterations={})",
            self.inner.enable_phase_breakdown,
            self.inner.enable_gil_analysis,
            self.inner.enable_memory_profile,
            self.inner.enable_flamegraph,
            self.inner.iterations
        )
    }
}

// =====================
// Flamegraph function
// =====================

/// Generate flamegraph SVG from folded stacks
#[pyfunction]
pub fn generate_flamegraph(folded_stacks: Vec<String>, title: &str, output_path: &str) -> PyResult<()> {
    generate_flamegraph_svg(&folded_stacks, title, output_path)
        .map_err(PyErr::new::<pyo3::exceptions::PyIOError, _>)
}
