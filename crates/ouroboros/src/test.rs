//! Test framework PyO3 bindings
//!
//! Provides Python bindings for the ouroboros-test crate.

use ouroboros_test::{
    TestMeta, TestResult, TestRunner, TestStatus, TestType,
    TestReport, Reporter, ReportFormat, CoverageInfo, FileCoverage,
    runner::{RunnerConfig, TestSummary}, reporter::EnvironmentInfo,
    benchmark::{
        BenchmarkConfig, BenchmarkResult, BenchmarkStats,
        BenchmarkReport, BenchmarkReportGroup, BenchmarkEnvironment,
    },
    discovery::{
        DiscoveryConfig, FileType, FileInfo, TestRegistry, BenchmarkRegistry,
        DiscoveryStats, walk_files, filter_files,
    },
    fixtures::{FixtureMeta, FixtureRegistry, FixtureScope},
    parametrize::{Parameter, ParameterSet, ParameterValue, ParametrizedTest},
    // Profiler types are now re-exported from the top level (from performance module)
    ProfilePhase, PhaseTiming, PhaseBreakdown,
    GilTestConfig, GilContentionResult,
    MemorySnapshot, MemoryProfile,
    FlamegraphData, ProfileResult, ProfileConfig,
    generate_flamegraph_svg,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::{Arc, Mutex};

// =====================
// Enums
// =====================

/// Python TestType enum
#[pyclass(name = "TestType", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyTestType {
    Unit,
    Profile,
    Stress,
    Security,
}

impl From<PyTestType> for TestType {
    fn from(py_type: PyTestType) -> Self {
        match py_type {
            PyTestType::Unit => TestType::Unit,
            PyTestType::Profile => TestType::Profile,
            PyTestType::Stress => TestType::Stress,
            PyTestType::Security => TestType::Security,
        }
    }
}

impl From<TestType> for PyTestType {
    fn from(rust_type: TestType) -> Self {
        match rust_type {
            TestType::Unit => PyTestType::Unit,
            TestType::Profile => PyTestType::Profile,
            TestType::Stress => PyTestType::Stress,
            TestType::Security => PyTestType::Security,
        }
    }
}

#[pymethods]
impl PyTestType {
    fn __str__(&self) -> &'static str {
        match self {
            PyTestType::Unit => "unit",
            PyTestType::Profile => "profile",
            PyTestType::Stress => "stress",
            PyTestType::Security => "security",
        }
    }

    fn __repr__(&self) -> String {
        format!("TestType.{}", self.__str__().to_uppercase())
    }
}

/// Python TestStatus enum
#[pyclass(name = "TestStatus", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyTestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

impl From<TestStatus> for PyTestStatus {
    fn from(status: TestStatus) -> Self {
        match status {
            TestStatus::Passed => PyTestStatus::Passed,
            TestStatus::Failed => PyTestStatus::Failed,
            TestStatus::Skipped => PyTestStatus::Skipped,
            TestStatus::Error => PyTestStatus::Error,
        }
    }
}

impl From<PyTestStatus> for TestStatus {
    fn from(status: PyTestStatus) -> Self {
        match status {
            PyTestStatus::Passed => TestStatus::Passed,
            PyTestStatus::Failed => TestStatus::Failed,
            PyTestStatus::Skipped => TestStatus::Skipped,
            PyTestStatus::Error => TestStatus::Error,
        }
    }
}

#[pymethods]
impl PyTestStatus {
    fn __str__(&self) -> &'static str {
        match self {
            PyTestStatus::Passed => "PASSED",
            PyTestStatus::Failed => "FAILED",
            PyTestStatus::Skipped => "SKIPPED",
            PyTestStatus::Error => "ERROR",
        }
    }

    fn __repr__(&self) -> String {
        format!("TestStatus.{}", self.__str__())
    }
}

/// Python ReportFormat enum
#[pyclass(name = "ReportFormat", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyReportFormat {
    Markdown,
    Html,
    Json,
    Yaml,
    JUnit,
    Console,
}

impl From<PyReportFormat> for ReportFormat {
    fn from(fmt: PyReportFormat) -> Self {
        match fmt {
            PyReportFormat::Markdown => ReportFormat::Markdown,
            PyReportFormat::Html => ReportFormat::Html,
            PyReportFormat::Json => ReportFormat::Json,
            PyReportFormat::Yaml => ReportFormat::Yaml,
            PyReportFormat::JUnit => ReportFormat::JUnit,
            PyReportFormat::Console => ReportFormat::Console,
        }
    }
}

#[pymethods]
impl PyReportFormat {
    fn __str__(&self) -> &'static str {
        match self {
            PyReportFormat::Markdown => "markdown",
            PyReportFormat::Html => "html",
            PyReportFormat::Json => "json",
            PyReportFormat::Yaml => "yaml",
            PyReportFormat::JUnit => "junit",
            PyReportFormat::Console => "console",
        }
    }

    fn __repr__(&self) -> String {
        format!("ReportFormat.{}", self.__str__().to_uppercase())
    }
}

// =====================
// TestMeta
// =====================

/// Python TestMeta class - metadata for a test
#[pyclass(name = "TestMeta")]
#[derive(Clone)]
pub struct PyTestMeta {
    inner: TestMeta,
}

#[pymethods]
impl PyTestMeta {
    /// Create new test metadata
    #[new]
    #[pyo3(signature = (name, test_type = None, timeout = None, tags = None))]
    fn new(
        name: String,
        test_type: Option<PyTestType>,
        timeout: Option<f64>,
        tags: Option<Vec<String>>,
    ) -> Self {
        let mut meta = TestMeta::new(name);

        if let Some(tt) = test_type {
            meta = meta.with_type(tt.into());
        }
        if let Some(t) = timeout {
            meta = meta.with_timeout(t);
        }
        if let Some(t) = tags {
            meta = meta.with_tags(t);
        }

        Self { inner: meta }
    }

    /// Test name
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Full qualified name
    #[getter]
    fn full_name(&self) -> &str {
        &self.inner.full_name
    }

    /// Set full name
    #[setter]
    fn set_full_name(&mut self, full_name: String) {
        self.inner.full_name = full_name;
    }

    /// Test type
    #[getter]
    fn test_type(&self) -> PyTestType {
        self.inner.test_type.into()
    }

    /// Timeout in seconds
    #[getter]
    fn timeout(&self) -> Option<f64> {
        self.inner.timeout
    }

    /// Tags
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    /// Skip reason
    #[getter]
    fn skip_reason(&self) -> Option<&str> {
        self.inner.skip_reason.as_deref()
    }

    /// Check if skipped
    fn is_skipped(&self) -> bool {
        self.inner.is_skipped()
    }

    /// Check if has tag
    fn has_tag(&self, tag: &str) -> bool {
        self.inner.has_tag(tag)
    }

    /// Skip this test
    fn skip(&mut self, reason: String) {
        self.inner.skip_reason = Some(reason);
    }

    /// Set source file path
    fn set_file_path(&mut self, path: String) {
        self.inner.file_path = Some(path);
    }

    /// Set line number
    fn set_line_number(&mut self, line: u32) {
        self.inner.line_number = Some(line);
    }

    fn __repr__(&self) -> String {
        format!(
            "TestMeta(name='{}', type={}, timeout={:?})",
            self.inner.name, self.inner.test_type, self.inner.timeout
        )
    }
}

// =====================
// TestResult
// =====================

/// Python TestResult class
#[pyclass(name = "TestResult")]
#[derive(Clone)]
pub struct PyTestResult {
    inner: TestResult,
}

#[pymethods]
impl PyTestResult {
    /// Create a passed result
    #[staticmethod]
    fn passed(meta: &PyTestMeta, duration_ms: u64) -> Self {
        Self {
            inner: TestResult::passed(meta.inner.clone(), duration_ms),
        }
    }

    /// Create a failed result
    #[staticmethod]
    fn failed(meta: &PyTestMeta, duration_ms: u64, error: String) -> Self {
        Self {
            inner: TestResult::failed(meta.inner.clone(), duration_ms, error),
        }
    }

    /// Create a skipped result
    #[staticmethod]
    fn skipped(meta: &PyTestMeta, reason: String) -> Self {
        Self {
            inner: TestResult::skipped(meta.inner.clone(), reason),
        }
    }

    /// Create an error result
    #[staticmethod]
    fn error(meta: &PyTestMeta, duration_ms: u64, error: String) -> Self {
        Self {
            inner: TestResult::error(meta.inner.clone(), duration_ms, error),
        }
    }

    /// Test metadata
    #[getter]
    fn meta(&self) -> PyTestMeta {
        PyTestMeta {
            inner: self.inner.meta.clone(),
        }
    }

    /// Test status
    #[getter]
    fn status(&self) -> PyTestStatus {
        self.inner.status.into()
    }

    /// Duration in milliseconds
    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    /// Error message
    #[getter]
    fn error_message(&self) -> Option<&str> {
        self.inner.error.as_deref()
    }

    /// Stack trace
    #[getter]
    fn stack_trace(&self) -> Option<&str> {
        self.inner.stack_trace.as_deref()
    }

    /// Set stack trace
    fn set_stack_trace(&mut self, trace: String) {
        self.inner.stack_trace = Some(trace);
    }

    /// Check if passed
    fn is_passed(&self) -> bool {
        self.inner.is_passed()
    }

    /// Check if failed
    fn is_failed(&self) -> bool {
        self.inner.is_failed()
    }

    fn __repr__(&self) -> String {
        format!(
            "TestResult(name='{}', status={}, duration={}ms)",
            self.inner.meta.name, self.inner.status, self.inner.duration_ms
        )
    }
}

// =====================
// TestSummary
// =====================

/// Python TestSummary class
#[pyclass(name = "TestSummary")]
#[derive(Clone)]
pub struct PyTestSummary {
    inner: TestSummary,
}

#[pymethods]
impl PyTestSummary {
    /// Total tests
    #[getter]
    fn total(&self) -> usize {
        self.inner.total
    }

    /// Passed tests
    #[getter]
    fn passed(&self) -> usize {
        self.inner.passed
    }

    /// Failed tests
    #[getter]
    fn failed(&self) -> usize {
        self.inner.failed
    }

    /// Skipped tests
    #[getter]
    fn skipped(&self) -> usize {
        self.inner.skipped
    }

    /// Error tests
    #[getter]
    fn errors(&self) -> usize {
        self.inner.errors
    }

    /// Total duration in ms
    #[getter]
    fn total_duration_ms(&self) -> u64 {
        self.inner.total_duration_ms
    }

    /// Check if all passed
    fn all_passed(&self) -> bool {
        self.inner.all_passed()
    }

    /// Get pass rate
    fn pass_rate(&self) -> f64 {
        self.inner.pass_rate()
    }

    fn __repr__(&self) -> String {
        format!(
            "TestSummary(total={}, passed={}, failed={}, skipped={})",
            self.inner.total, self.inner.passed, self.inner.failed, self.inner.skipped
        )
    }
}

// =====================
// TestRunner
// =====================

/// Python TestRunner class
#[pyclass(name = "TestRunner")]
pub struct PyTestRunner {
    inner: TestRunner,
}

#[pymethods]
impl PyTestRunner {
    /// Create a new test runner
    #[new]
    #[pyo3(signature = (
        test_type = None,
        tags = None,
        name_pattern = None,
        fail_fast = false,
        verbose = false,
        parallel = false,
        max_workers = 4
    ))]
    fn new(
        test_type: Option<PyTestType>,
        tags: Option<Vec<String>>,
        name_pattern: Option<String>,
        fail_fast: bool,
        verbose: bool,
        parallel: bool,
        max_workers: usize,
    ) -> Self {
        let config = RunnerConfig {
            test_type: test_type.map(Into::into),
            tags: tags.unwrap_or_default(),
            name_pattern,
            fail_fast,
            verbose,
            parallel,
            max_workers,
        };

        Self {
            inner: TestRunner::new(config),
        }
    }

    /// Start the test run
    fn start(&mut self) {
        self.inner.start();
    }

    /// Record a test result
    fn record(&mut self, result: &PyTestResult) {
        self.inner.record(result.inner.clone());
    }

    /// Get all results
    fn results(&self) -> Vec<PyTestResult> {
        self.inner
            .results()
            .iter()
            .map(|r| PyTestResult { inner: r.clone() })
            .collect()
    }

    /// Get summary
    fn summary(&self) -> PyTestSummary {
        PyTestSummary {
            inner: self.inner.summary(),
        }
    }

    /// Get total duration in seconds
    fn total_duration_secs(&self) -> f64 {
        self.inner.total_duration().as_secs_f64()
    }

    /// Check if test should run based on filters
    fn should_run(&self, meta: &PyTestMeta) -> bool {
        self.inner.should_run(&meta.inner)
    }

    /// Run tests in parallel using Tokio (returns Python awaitable)
    ///
    /// This is an async method that can be awaited from Python.
    /// It spawns Tokio tasks to execute tests concurrently.
    ///
    /// # Arguments
    /// * `suite_instance` - The TestSuite instance
    /// * `test_descriptors` - List of TestDescriptor objects
    fn run_parallel_async<'py>(
        &self,
        py: Python<'py>,
        suite_instance: PyObject,
        test_descriptors: Vec<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pyo3_async_runtimes::tokio::future_into_py;
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let config = self.inner.config().clone();

        // Clone Python references while we have the GIL
        let suite_refs: Vec<PyObject> = test_descriptors
            .iter()
            .map(|_| suite_instance.clone_ref(py))
            .collect();
        let test_refs: Vec<PyObject> = test_descriptors
            .iter()
            .map(|test| test.clone_ref(py))
            .collect();

        // Clone additional refs for error handling (2 extra per test)
        let test_refs_for_panic: Vec<PyObject> = test_descriptors
            .iter()
            .map(|test| test.clone_ref(py))
            .collect();
        let test_refs_for_timeout: Vec<PyObject> = test_descriptors
            .iter()
            .map(|test| test.clone_ref(py))
            .collect();

        // Clone suite reference for setup/teardown
        let suite_for_setup = suite_instance.clone_ref(py);
        let suite_for_teardown = suite_instance.clone_ref(py);

        // Return async future for Python to await
        future_into_py(py, async move {
            // Run suite setup (sequential, before parallel tests)
            let setup_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
                Python::with_gil(|py| {
                    let suite_bound = suite_for_setup.bind(py);
                    call_async_method(py, suite_bound, "setup_suite")
                        .map_err(|e| format!("{}", e))
                })
            })
            .await;

            // If setup_suite fails, skip test execution and return early
            // The Python layer will handle reporting setup failures
            if let Ok(Err(_setup_err)) = setup_result {
                // Just return empty results - suite.run() in Python will catch this
                return Ok(Vec::new());
            }

            // Create semaphore to limit concurrency
            let semaphore = Arc::new(Semaphore::new(config.max_workers));

            // Spawn tasks for each test
            let mut tasks = Vec::new();

            for ((suite_ref, test_ref), (test_ref_panic, test_ref_timeout)) in suite_refs
                .into_iter()
                .zip(test_refs.into_iter())
                .zip(test_refs_for_panic.into_iter().zip(test_refs_for_timeout.into_iter()))
            {
                let sem = semaphore.clone();
                let test_ref_for_panic = test_ref_panic;
                let test_ref_for_timeout = test_ref_timeout;

                // Spawn task that will acquire semaphore permit
                let task = tokio::spawn(async move {
                    // Acquire permit (blocks if at max_workers)
                    let _permit = sem.acquire().await.unwrap();

                    // Get timeout from test metadata
                    let timeout_duration = Python::with_gil(|py| {
                        let test_desc = test_ref.bind(py);
                        let meta_result = test_desc.call_method0("get_meta");

                        match meta_result {
                            Ok(meta_obj) => {
                                let meta: Result<PyTestMeta, _> = meta_obj.extract();
                                match meta {
                                    Ok(m) => {
                                        // Use test-specific timeout if configured, otherwise default to 60s
                                        m.inner.timeout
                                            .map(std::time::Duration::from_secs_f64)
                                            .unwrap_or(std::time::Duration::from_secs(60))
                                    }
                                    Err(_) => std::time::Duration::from_secs(60),
                                }
                            }
                            Err(_) => std::time::Duration::from_secs(60),
                        }
                    });

                    // Execute test in blocking task with panic recovery and timeout
                    let test_future = tokio::task::spawn_blocking(move || {
                        execute_single_test_with_gil(suite_ref, test_ref)
                    });

                    match tokio::time::timeout(timeout_duration, test_future).await {
                        Ok(Ok(result)) => result,
                        Ok(Err(join_err)) => {
                            // Task panicked - create error result
                            Python::with_gil(|py| {
                                let test_desc = test_ref_for_panic.bind(py);
                                let meta_obj = test_desc.call_method0("get_meta")?;
                                let meta: PyTestMeta = meta_obj.extract()?;

                                Ok(PyTestResult {
                                    inner: TestResult::error(
                                        meta.inner,
                                        0,
                                        format!("Test task panicked: {}", join_err)
                                    ),
                                })
                            })
                        }
                        Err(_timeout_err) => {
                            // Timeout occurred - create error result
                            Python::with_gil(|py| {
                                let test_desc = test_ref_for_timeout.bind(py);
                                let meta_obj = test_desc.call_method0("get_meta")?;
                                let meta: PyTestMeta = meta_obj.extract()?;

                                let timeout_secs = timeout_duration.as_secs_f64();
                                Ok(PyTestResult {
                                    inner: TestResult::error(
                                        meta.inner,
                                        (timeout_secs * 1000.0) as u64,
                                        format!("Test timed out after {:.1} seconds", timeout_secs)
                                    ),
                                })
                            })
                        }
                    }
                });

                tasks.push(task);
            }

            // Collect results with improved error handling
            let mut results = Vec::new();
            for task in tasks {
                match task.await {
                    Ok(Ok(result)) => {
                        // Test executed successfully (may have passed or failed)
                        let test_failed = !result.is_passed();
                        results.push(result);

                        // Fail-fast: stop collecting if a test failed and fail_fast is enabled
                        if test_failed && config.fail_fast {
                            eprintln!("Fail-fast enabled: stopping after first failure");
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        // Python error during test execution (timeout, panic, etc.)
                        // Create an error result with detailed information
                        Python::with_gil(|_py| {
                            let error_msg = format!("Test execution error: {}", e);
                            eprintln!("{}", error_msg);

                            // Note: We can't create a proper TestResult here without test metadata
                            // The error is logged but not added to results
                            // In practice, errors should be caught in execute_single_test_with_gil
                        });

                        // Fail-fast on error
                        if config.fail_fast {
                            eprintln!("Fail-fast enabled: stopping after error");
                            break;
                        }
                    }
                    Err(e) => {
                        // Task join error (should be rare - indicates tokio task failure)
                        eprintln!("Task join error: {:?}", e);

                        // Fail-fast on task error
                        if config.fail_fast {
                            break;
                        }
                    }
                }
            }

            // Run suite teardown (sequential, after all tests)
            let _ = tokio::task::spawn_blocking(move || -> Result<(), String> {
                Python::with_gil(|py| {
                    let suite_bound = suite_for_teardown.bind(py);
                    call_async_method(py, suite_bound, "teardown_suite")
                        .map_err(|e| format!("{}", e))
                })
            })
            .await;

            Ok(results)
        })
    }
}

// =====================
// Helper Functions for Parallel Execution
// =====================

/// Call a potentially async method on a Python object
///
/// This helper handles both sync and async methods by checking if the result
/// is a coroutine and awaiting it if necessary.
fn call_async_method(py: Python<'_>, obj: &Bound<'_, pyo3::PyAny>, method_name: &str) -> PyResult<()> {
    let result = obj.call_method0(method_name)?;

    // Check if result is a coroutine
    let inspect = py.import("inspect")?;
    let is_coro = inspect
        .call_method1("iscoroutine", (result.clone(),))?
        .extract::<bool>()?;

    if is_coro {
        // It's a coroutine - run it with asyncio.run()
        let asyncio = py.import("asyncio")?;
        asyncio.call_method1("run", (result,))?;
    }

    Ok(())
}

/// Execute a single test with GIL management
///
/// This function is called from a blocking task and handles:
/// 1. GIL acquisition
/// 2. Test lifecycle (setup → test → teardown)
/// 3. Error handling and result conversion
/// 4. Timeout enforcement (if configured)
fn execute_single_test_with_gil(
    suite_instance: PyObject,
    test_descriptor: PyObject,
) -> PyResult<PyTestResult> {
    Python::with_gil(|py| {
        use std::time::Instant;

        // Get test metadata
        let test_desc = test_descriptor.bind(py);
        let meta_obj = test_desc.call_method0("get_meta")?;
        let meta: PyTestMeta = meta_obj.extract()?;

        // Check if test is async
        let is_async = test_desc
            .getattr("is_async")
            .and_then(|attr| attr.extract::<bool>())
            .unwrap_or(false);

        // Run test setup
        let suite = suite_instance.bind(py);
        if let Err(e) = call_async_method(py, suite, "setup") {
            // Setup failed - return error result
            let result = PyTestResult {
                inner: TestResult::error(meta.inner, 0, format!("Test setup failed: {}", e)),
            };
            return Ok(result);
        }

        // Execute the test
        let start = Instant::now();
        let test_result = if is_async {
            // Async test - call to get coroutine, then await it
            // Note: Python async execution needs to happen in an async context
            // For now, we'll use a workaround: spawn the coroutine in Python's event loop
            // This is a simplified implementation - full async support would use pyo3-asyncio
            let coro_result = test_desc.call1((suite,));
            match coro_result {
                Ok(coro) => {
                    // We have a coroutine object
                    // We need to run it in Python's event loop
                    // For blocking context, we use asyncio.run()
                    let asyncio = py.import("asyncio")?;
                    asyncio.call_method1("run", (coro,))
                }
                Err(e) => Err(e),
            }
        } else {
            // Synchronous test - just call it
            test_desc.call1((suite,))
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        // Always run teardown, even if test failed
        let _ = call_async_method(py, suite, "teardown");

        // Convert result
        let result = match test_result {
            Ok(_) => PyTestResult {
                inner: TestResult::passed(meta.inner, duration_ms),
            },
            Err(e) => {
                // Check if it's an AssertionError (test failure) or other error
                let is_assertion = e.is_instance_of::<pyo3::exceptions::PyAssertionError>(py);

                let error_msg = format!("{}", e);
                let stack_trace = if let Some(traceback) = e.traceback(py) {
                    traceback.format().unwrap_or_default()
                } else {
                    String::new()
                };

                let mut result = if is_assertion {
                    TestResult::failed(meta.inner, duration_ms, error_msg)
                } else {
                    TestResult::error(meta.inner, duration_ms, error_msg)
                };

                if !stack_trace.is_empty() {
                    result = result.with_stack_trace(stack_trace);
                }

                PyTestResult { inner: result }
            }
        };

        Ok(result)
    })
}

// =====================
// Expectation (Assertions)
// =====================

/// Python Expectation class for assertions
#[pyclass(name = "Expectation")]
pub struct PyExpectation {
    value: PyObject,
    negated: bool,
}

#[pymethods]
impl PyExpectation {
    /// Create a new expectation
    #[new]
    fn new(value: PyObject) -> Self {
        Self {
            value,
            negated: false,
        }
    }

    /// Negate the expectation
    #[getter]
    fn not_(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            value: self.value.clone_ref(py),
            negated: !self.negated,
        })
    }

    /// Assert equality
    fn to_equal(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        let result = self.value.bind(py).eq(expected.bind(py))?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT equal {:?}", self.value, expected)
            } else {
                format!("Expected {:?} to equal {:?}", self.value, expected)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert truthiness
    fn to_be_true(&self, py: Python<'_>) -> PyResult<()> {
        let result = self.value.bind(py).is_truthy()?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to be falsy", self.value)
            } else {
                format!("Expected {:?} to be truthy", self.value)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert falsiness
    fn to_be_false(&self, py: Python<'_>) -> PyResult<()> {
        let result = !self.value.bind(py).is_truthy()?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to be truthy", self.value)
            } else {
                format!("Expected {:?} to be falsy", self.value)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert is None
    fn to_be_none(&self, py: Python<'_>) -> PyResult<()> {
        let result = self.value.bind(py).is_none();
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                "Expected value to NOT be None".to_string()
            } else {
                format!("Expected None, but got {:?}", self.value)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert greater than
    fn to_be_greater_than(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        let result = self.value.bind(py).gt(expected.bind(py))?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT be greater than {:?}", self.value, expected)
            } else {
                format!("Expected {:?} to be greater than {:?}", self.value, expected)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert less than
    fn to_be_less_than(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        let result = self.value.bind(py).lt(expected.bind(py))?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT be less than {:?}", self.value, expected)
            } else {
                format!("Expected {:?} to be less than {:?}", self.value, expected)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert contains
    fn to_contain(&self, py: Python<'_>, item: PyObject) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let bound_item = item.bind(py);
        let result = bound_value.contains(bound_item)?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT contain {:?}", self.value, item)
            } else {
                format!("Expected {:?} to contain {:?}", self.value, item)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert has key (for dicts)
    fn to_have_key(&self, py: Python<'_>, key: PyObject) -> PyResult<()> {
        let bound_value = self.value.bind(py);

        // Try to access as dict
        let result = if let Ok(dict) = bound_value.downcast::<PyDict>() {
            dict.contains(&key)?
        } else {
            // Try __contains__ method
            bound_value.contains(&key)?
        };

        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT have key {:?}", self.value, key)
            } else {
                format!("Expected {:?} to have key {:?}", self.value, key)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert length
    fn to_have_length(&self, py: Python<'_>, expected_len: usize) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let actual_len = bound_value.len()?;
        let result = actual_len == expected_len;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected length to NOT be {}, but got {}", expected_len, actual_len)
            } else {
                format!("Expected length {}, but got {}", expected_len, actual_len)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert empty
    fn to_be_empty(&self, py: Python<'_>) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let result = bound_value.len()? == 0;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                "Expected value to NOT be empty".to_string()
            } else {
                format!("Expected empty value, but got length {}", bound_value.len()?)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert starts with (for strings)
    fn to_start_with(&self, py: Python<'_>, prefix: &str) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let s: String = bound_value.extract()?;
        let result = s.starts_with(prefix);
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected '{}' to NOT start with '{}'", s, prefix)
            } else {
                format!("Expected '{}' to start with '{}'", s, prefix)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert ends with (for strings)
    fn to_end_with(&self, py: Python<'_>, suffix: &str) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let s: String = bound_value.extract()?;
        let result = s.ends_with(suffix);
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected '{}' to NOT end with '{}'", s, suffix)
            } else {
                format!("Expected '{}' to end with '{}'", s, suffix)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert matches regex
    fn to_match(&self, py: Python<'_>, pattern: &str) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let s: String = bound_value.extract()?;

        let regex = regex::Regex::new(pattern)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex: {}", e)))?;

        let result = regex.is_match(&s);
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected '{}' to NOT match pattern '{}'", s, pattern)
            } else {
                format!("Expected '{}' to match pattern '{}'", s, pattern)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert that calling a callable raises a specific exception type.
    ///
    /// Usage: `expect(lambda: some_func()).to_raise(ValueError)`
    ///
    /// The value should be a callable (typically a lambda) that when called
    /// should raise the specified exception type.
    fn to_raise(&self, py: Python<'_>, exception_type: PyObject) -> PyResult<()> {
        let bound_callable = self.value.bind(py);

        // Verify the value is callable
        if !bound_callable.is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "to_raise() expects a callable (e.g., lambda: func())",
            ));
        }

        // Call the callable and see what happens
        let call_result = bound_callable.call0();

        match call_result {
            Ok(_) => {
                // No exception was raised
                if self.negated {
                    // expect(...).not().to_raise(E) - we expected NO exception, and none was raised
                    Ok(())
                } else {
                    // expect(...).to_raise(E) - we expected an exception, but none was raised
                    let exc_name = exception_type
                        .bind(py)
                        .getattr("__name__")
                        .map(|n| n.to_string())
                        .unwrap_or_else(|_| format!("{:?}", exception_type));
                    Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(format!(
                        "Expected {} to be raised, but no exception was raised",
                        exc_name
                    )))
                }
            }
            Err(err) => {
                // An exception was raised - check if it's the right type
                let raised_type = err.get_type(py);
                let expected_type = exception_type.bind(py);

                // Check if the raised exception is an instance of the expected type
                // Using PyAny::is_instance to handle inheritance properly
                let is_expected_type = raised_type.is_subclass(expected_type).unwrap_or(false);

                if self.negated {
                    // expect(...).not().to_raise(E) - we expected NO exception of type E
                    if is_expected_type {
                        let exc_name = expected_type
                            .getattr("__name__")
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| format!("{:?}", exception_type));
                        Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(format!(
                            "Expected {} NOT to be raised, but it was: {}",
                            exc_name,
                            err
                        )))
                    } else {
                        // A different exception was raised, which is fine for negated case
                        // But we should re-raise it since it's unexpected
                        Err(err)
                    }
                } else {
                    // expect(...).to_raise(E) - we expected exception of type E
                    if is_expected_type {
                        Ok(())
                    } else {
                        let expected_name = expected_type
                            .getattr("__name__")
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| format!("{:?}", exception_type));
                        let raised_name = raised_type
                            .getattr("__name__")
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| "Unknown".to_string());
                        Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(format!(
                            "Expected {} to be raised, but got {}: {}",
                            expected_name, raised_name, err
                        )))
                    }
                }
            }
        }
    }
}

/// Create an expectation from a value
#[pyfunction]
fn expect(value: PyObject) -> PyExpectation {
    PyExpectation::new(value)
}

// =====================
// Reporter
// =====================

/// Python Reporter class
#[pyclass(name = "Reporter")]
pub struct PyReporter {
    inner: Reporter,
}

#[pymethods]
impl PyReporter {
    /// Create a new reporter
    #[new]
    #[pyo3(signature = (format = PyReportFormat::Markdown))]
    fn new(format: PyReportFormat) -> Self {
        Self {
            inner: Reporter::new(format.into()),
        }
    }

    /// Create markdown reporter
    #[staticmethod]
    fn markdown() -> Self {
        Self {
            inner: Reporter::markdown(),
        }
    }

    /// Create HTML reporter
    #[staticmethod]
    fn html() -> Self {
        Self {
            inner: Reporter::html(),
        }
    }

    /// Create JSON reporter
    #[staticmethod]
    fn json() -> Self {
        Self {
            inner: Reporter::json(),
        }
    }

    /// Create JUnit reporter
    #[staticmethod]
    fn junit() -> Self {
        Self {
            inner: Reporter::junit(),
        }
    }

    /// Generate report string
    fn generate(&self, report: &PyTestReport) -> String {
        self.inner.generate(&report.inner)
    }
}

// =====================
// TestReport
// =====================

/// Python TestReport class
#[pyclass(name = "TestReport")]
#[derive(Clone)]
pub struct PyTestReport {
    inner: TestReport,
}

#[pymethods]
impl PyTestReport {
    /// Create a new test report
    #[new]
    fn new(suite_name: String, results: Vec<PyTestResult>) -> Self {
        let rust_results: Vec<TestResult> = results.iter().map(|r| r.inner.clone()).collect();
        Self {
            inner: TestReport::new(suite_name, rust_results),
        }
    }

    /// Suite name
    #[getter]
    fn suite_name(&self) -> &str {
        &self.inner.suite_name
    }

    /// Generated timestamp
    #[getter]
    fn generated_at(&self) -> &str {
        &self.inner.generated_at
    }

    /// Duration in milliseconds
    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    /// Summary
    #[getter]
    fn summary(&self) -> PyTestSummary {
        PyTestSummary {
            inner: self.inner.summary.clone(),
        }
    }

    /// All results
    #[getter]
    fn results(&self) -> Vec<PyTestResult> {
        self.inner
            .results
            .iter()
            .map(|r| PyTestResult { inner: r.clone() })
            .collect()
    }

    /// Get results by test type
    fn results_by_type(&self, test_type: PyTestType) -> Vec<PyTestResult> {
        self.inner
            .results_by_type(test_type.into())
            .into_iter()
            .map(|r| PyTestResult { inner: r.clone() })
            .collect()
    }

    /// Get failed results
    fn failed_results(&self) -> Vec<PyTestResult> {
        self.inner
            .failed_results()
            .into_iter()
            .map(|r| PyTestResult { inner: r.clone() })
            .collect()
    }

    /// Set environment info
    fn set_environment(
        &mut self,
        python_version: Option<String>,
        rust_version: Option<String>,
        platform: Option<String>,
        hostname: Option<String>,
    ) {
        self.inner.environment = EnvironmentInfo {
            python_version,
            rust_version,
            platform,
            hostname,
        };
    }

    /// Set coverage info
    fn set_coverage(&mut self, coverage: &PyCoverageInfo) {
        self.inner.set_coverage(coverage.inner.clone());
    }

    /// Get coverage info
    #[getter]
    fn coverage(&self) -> Option<PyCoverageInfo> {
        self.inner.coverage.as_ref().map(|c| PyCoverageInfo { inner: c.clone() })
    }
}

// =====================
// Coverage Info
// =====================

/// Python FileCoverage class - coverage info for a single file
#[pyclass(name = "FileCoverage")]
#[derive(Clone)]
pub struct PyFileCoverage {
    inner: FileCoverage,
}

#[pymethods]
impl PyFileCoverage {
    #[new]
    fn new(
        path: String,
        statements: usize,
        covered: usize,
        missing_lines: Vec<usize>,
    ) -> Self {
        let coverage_percent = if statements > 0 {
            (covered as f64 / statements as f64) * 100.0
        } else {
            0.0
        };
        Self {
            inner: FileCoverage {
                path,
                statements,
                covered,
                missing_lines,
                coverage_percent,
            },
        }
    }

    #[getter]
    fn path(&self) -> &str {
        &self.inner.path
    }

    #[getter]
    fn statements(&self) -> usize {
        self.inner.statements
    }

    #[getter]
    fn covered(&self) -> usize {
        self.inner.covered
    }

    #[getter]
    fn missing_lines(&self) -> Vec<usize> {
        self.inner.missing_lines.clone()
    }

    #[getter]
    fn coverage_percent(&self) -> f64 {
        self.inner.coverage_percent
    }

    fn __repr__(&self) -> String {
        format!(
            "FileCoverage(path='{}', coverage={:.1}%, {}/{})",
            self.inner.path, self.inner.coverage_percent, self.inner.covered, self.inner.statements
        )
    }
}

/// Python CoverageInfo class - overall coverage summary
#[pyclass(name = "CoverageInfo")]
#[derive(Clone)]
pub struct PyCoverageInfo {
    inner: CoverageInfo,
}

#[pymethods]
impl PyCoverageInfo {
    #[new]
    #[pyo3(signature = (total_statements = 0, covered_statements = 0, files = None, uncovered_files = None))]
    fn new(
        total_statements: usize,
        covered_statements: usize,
        files: Option<Vec<PyFileCoverage>>,
        uncovered_files: Option<Vec<String>>,
    ) -> Self {
        let coverage_percent = if total_statements > 0 {
            (covered_statements as f64 / total_statements as f64) * 100.0
        } else {
            0.0
        };
        Self {
            inner: CoverageInfo {
                total_statements,
                covered_statements,
                coverage_percent,
                files: files.map(|f| f.into_iter().map(|fc| fc.inner).collect()).unwrap_or_default(),
                uncovered_files: uncovered_files.unwrap_or_default(),
            },
        }
    }

    #[getter]
    fn total_statements(&self) -> usize {
        self.inner.total_statements
    }

    #[getter]
    fn covered_statements(&self) -> usize {
        self.inner.covered_statements
    }

    #[getter]
    fn coverage_percent(&self) -> f64 {
        self.inner.coverage_percent
    }

    #[getter]
    fn files(&self) -> Vec<PyFileCoverage> {
        self.inner.files.iter().map(|f| PyFileCoverage { inner: f.clone() }).collect()
    }

    #[getter]
    fn uncovered_files(&self) -> Vec<String> {
        self.inner.uncovered_files.clone()
    }

    /// Add file coverage
    fn add_file(&mut self, file: &PyFileCoverage) {
        self.inner.files.push(file.inner.clone());
        // Recalculate totals
        self.inner.total_statements += file.inner.statements;
        self.inner.covered_statements += file.inner.covered;
        self.inner.coverage_percent = if self.inner.total_statements > 0 {
            (self.inner.covered_statements as f64 / self.inner.total_statements as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Add uncovered file
    fn add_uncovered_file(&mut self, path: String) {
        self.inner.uncovered_files.push(path);
    }

    fn __repr__(&self) -> String {
        format!(
            "CoverageInfo({:.1}%, {}/{} statements, {} files)",
            self.inner.coverage_percent,
            self.inner.covered_statements,
            self.inner.total_statements,
            self.inner.files.len()
        )
    }
}

// =====================
// Benchmark
// =====================

/// Python BenchmarkStats class
#[pyclass(name = "BenchmarkStats")]
#[derive(Clone)]
pub struct PyBenchmarkStats {
    inner: BenchmarkStats,
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

/// Python BenchmarkResult class
#[pyclass(name = "BenchmarkResult")]
#[derive(Clone)]
pub struct PyBenchmarkResult {
    inner: BenchmarkResult,
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

/// Compare multiple benchmark results and return formatted comparison
#[pyfunction]
#[pyo3(signature = (results, baseline_name = None))]
fn compare_benchmarks(results: Vec<PyBenchmarkResult>, baseline_name: Option<&str>) -> String {
    let rust_results: Vec<BenchmarkResult> = results.iter().map(|r| r.inner.clone()).collect();
    ouroboros_test::benchmark::compare_results(&rust_results, baseline_name)
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
fn print_comparison_table(results: Vec<PyBenchmarkResult>, baseline_name: Option<&str>) {
    let rust_results: Vec<BenchmarkResult> = results.iter().map(|r| r.inner.clone()).collect();
    ouroboros_test::benchmark::print_comparison_table(&rust_results, baseline_name);
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

// =====================
// Test Server
// =====================

use ouroboros_test::http_server::{TestServer, TestServerHandle, TestServerConfig};
use tokio::sync::Mutex as TokioMutex;

/// Python wrapper for TestServerHandle
#[pyclass(name = "TestServerHandle")]
pub struct PyTestServerHandle {
    url: String,
    port: u16,
    handle: Arc<TokioMutex<Option<TestServerHandle>>>,
}

#[pymethods]
impl PyTestServerHandle {
    /// Get the base URL for this server
    #[getter]
    fn url(&self) -> &str {
        &self.url
    }

    /// Get the port number
    #[getter]
    fn port(&self) -> u16 {
        self.port
    }

    /// Get an HTTP client for making requests (returns HttpClient from ouroboros-http)
    /// For now, this is a placeholder - users should use the server URL with their own client
    #[getter]
    fn client(&self) -> String {
        // Return the base URL for now
        // In the future, we could return an actual HttpClient instance
        self.url.clone()
    }

    /// Stop the server
    fn stop(&self) -> PyResult<()> {
        let handle = self.handle.clone();
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            let mut guard = handle.lock().await;
            if let Some(h) = guard.take() {
                h.stop();
            }
        });
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("TestServerHandle(url='{}', port={})", self.url, self.port)
    }
}

/// Python TestServer class for creating test HTTP servers
#[pyclass(name = "TestServer")]
pub struct PyTestServer {
    routes: std::collections::HashMap<String, serde_json::Value>,
    port: Option<u16>,
    /// Configuration for Python app mode
    app_config: Option<TestServerConfig>,
}

#[pymethods]
impl PyTestServer {
    /// Create a new test server
    #[new]
    fn new() -> Self {
        Self {
            routes: std::collections::HashMap::new(),
            port: None,
            app_config: None,
        }
    }

    /// Create a test server from a Python application
    #[staticmethod]
    #[pyo3(signature = (
        app_module,
        app_callable = "app".to_string(),
        port = 18765,
        startup_timeout = 10.0,
        health_endpoint = None
    ))]
    fn from_app(
        app_module: String,
        app_callable: String,
        port: u16,
        startup_timeout: f64,
        health_endpoint: Option<String>,
    ) -> Self {
        let config = TestServerConfig {
            app_module,
            app_callable,
            port,
            startup_timeout,
            health_endpoint,
        };
        Self {
            routes: std::collections::HashMap::new(),
            port: Some(port),
            app_config: Some(config),
        }
    }

    /// Set the port to listen on
    fn port(&mut self, port: u16) {
        self.port = Some(port);
    }

    /// Add a GET route with JSON response
    fn get(&mut self, path: &str, response: &Bound<'_, pyo3::types::PyAny>) -> PyResult<()> {
        let json_value = python_to_json(response)?;
        self.routes.insert(path.to_string(), json_value);
        Ok(())
    }

    /// Add multiple routes from a dict
    fn routes(&mut self, routes: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        for (key, value) in routes.iter() {
            let path: String = key.extract()?;
            let json_value = python_to_json(&value)?;
            self.routes.insert(path, json_value);
        }
        Ok(())
    }

    /// Start the server (async)
    fn start<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyAny>> {
        let routes = self.routes.clone();
        let port = self.port;
        let app_config = self.app_config.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let builder = if let Some(config) = app_config {
                // Create from Python app
                TestServer::from_app(config)
            } else {
                // Create Axum server with routes
                let mut builder = TestServer::new();

                if let Some(p) = port {
                    builder = builder.port(p);
                }

                for (path, response) in routes {
                    builder = builder.get(&path, response);
                }

                builder
            };

            let handle = builder.start().await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            let url = handle.url.clone();
            let port = handle.port;

            Ok(PyTestServerHandle {
                url,
                port,
                handle: Arc::new(TokioMutex::new(Some(handle))),
            })
        })
    }

    fn __repr__(&self) -> String {
        format!("TestServer(routes={}, port={:?})", self.routes.len(), self.port)
    }
}

/// Convert Python object to serde_json::Value
fn python_to_json(obj: &Bound<'_, pyo3::types::PyAny>) -> PyResult<serde_json::Value> {
    // Try to convert via JSON string (simple approach)
    let json_module = obj.py().import("json")?;
    let json_str: String = json_module.call_method1("dumps", (obj,))?.extract()?;
    serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

// =====================
// Discovery types
// =====================

/// Python FileType enum
#[pyclass(name = "FileType", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFileType {
    Test,
    Benchmark,
}

impl From<FileType> for PyFileType {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Test => PyFileType::Test,
            FileType::Benchmark => PyFileType::Benchmark,
        }
    }
}

impl From<PyFileType> for FileType {
    fn from(py_type: PyFileType) -> Self {
        match py_type {
            PyFileType::Test => FileType::Test,
            PyFileType::Benchmark => FileType::Benchmark,
        }
    }
}

#[pymethods]
impl PyFileType {
    fn __str__(&self) -> &'static str {
        match self {
            PyFileType::Test => "test",
            PyFileType::Benchmark => "benchmark",
        }
    }

    fn __repr__(&self) -> String {
        format!("FileType.{}", self.__str__().to_uppercase())
    }
}

/// Python FileInfo wrapper
#[pyclass(name = "FileInfo")]
#[derive(Clone)]
pub struct PyFileInfo {
    inner: FileInfo,
}

#[pymethods]
impl PyFileInfo {
    #[getter]
    fn path(&self) -> String {
        self.inner.path.display().to_string()
    }

    #[getter]
    fn module_name(&self) -> String {
        self.inner.module_name.clone()
    }

    #[getter]
    fn file_type(&self) -> PyFileType {
        self.inner.file_type.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "FileInfo(path='{}', module_name='{}', file_type={})",
            self.path(),
            self.module_name(),
            self.file_type().__repr__()
        )
    }
}

/// Python DiscoveryConfig wrapper
#[pyclass(name = "DiscoveryConfig")]
#[derive(Clone)]
pub struct PyDiscoveryConfig {
    inner: DiscoveryConfig,
}

#[pymethods]
impl PyDiscoveryConfig {
    #[new]
    #[pyo3(signature = (root_path="tests/".to_string(), patterns=None, exclusions=None, max_depth=10))]
    fn new(
        root_path: String,
        patterns: Option<Vec<String>>,
        exclusions: Option<Vec<String>>,
        max_depth: usize,
    ) -> Self {
        let mut config = DiscoveryConfig::default();
        config.root_path = root_path.into();
        if let Some(p) = patterns {
            config.patterns = p;
        }
        if let Some(e) = exclusions {
            config.exclusions = e;
        }
        config.max_depth = max_depth;
        Self { inner: config }
    }

    #[getter]
    fn root_path(&self) -> String {
        self.inner.root_path.display().to_string()
    }

    #[getter]
    fn patterns(&self) -> Vec<String> {
        self.inner.patterns.clone()
    }

    #[getter]
    fn exclusions(&self) -> Vec<String> {
        self.inner.exclusions.clone()
    }

    #[getter]
    fn max_depth(&self) -> usize {
        self.inner.max_depth
    }

    fn __repr__(&self) -> String {
        format!(
            "DiscoveryConfig(root_path='{}', patterns={:?}, max_depth={})",
            self.root_path(),
            self.patterns(),
            self.max_depth()
        )
    }
}

/// Python TestRegistry wrapper
#[pyclass(name = "TestRegistry")]
pub struct PyTestRegistry {
    inner: Arc<Mutex<TestRegistry>>,
}

#[pymethods]
impl PyTestRegistry {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TestRegistry::new())),
        }
    }

    fn register(&self, file: PyFileInfo) -> PyResult<()> {
        self.inner
            .lock()
            .unwrap()
            .register(file.inner);
        Ok(())
    }

    fn get_all(&self) -> Vec<PyFileInfo> {
        self.inner
            .lock()
            .unwrap()
            .get_all()
            .iter()
            .cloned()
            .map(|f| PyFileInfo { inner: f })
            .collect()
    }

    fn filter_by_pattern(&self, pattern: String) -> PyResult<()> {
        self.inner
            .lock()
            .unwrap()
            .filter_by_pattern(&pattern);
        Ok(())
    }

    fn count(&self) -> usize {
        self.inner.lock().unwrap().count()
    }

    fn clear(&self) -> PyResult<()> {
        self.inner.lock().unwrap().clear();
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("TestRegistry(count={})", self.count())
    }
}

/// Python BenchmarkRegistry wrapper
#[pyclass(name = "BenchmarkRegistry")]
pub struct PyBenchmarkRegistry {
    inner: Arc<Mutex<BenchmarkRegistry>>,
}

#[pymethods]
impl PyBenchmarkRegistry {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BenchmarkRegistry::new())),
        }
    }

    fn register(&self, file: PyFileInfo) -> PyResult<()> {
        self.inner
            .lock()
            .unwrap()
            .register(file.inner);
        Ok(())
    }

    fn get_all(&self) -> Vec<PyFileInfo> {
        self.inner
            .lock()
            .unwrap()
            .get_all()
            .iter()
            .cloned()
            .map(|f| PyFileInfo { inner: f })
            .collect()
    }

    fn filter_by_pattern(&self, pattern: String) -> PyResult<()> {
        self.inner
            .lock()
            .unwrap()
            .filter_by_pattern(&pattern);
        Ok(())
    }

    fn count(&self) -> usize {
        self.inner.lock().unwrap().count()
    }

    fn clear(&self) -> PyResult<()> {
        self.inner.lock().unwrap().clear();
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("BenchmarkRegistry(count={})", self.count())
    }
}

/// Python DiscoveryStats wrapper
#[pyclass(name = "DiscoveryStats")]
pub struct PyDiscoveryStats {
    inner: DiscoveryStats,
}

#[pymethods]
impl PyDiscoveryStats {
    #[getter]
    fn files_found(&self) -> usize {
        self.inner.files_found
    }

    #[getter]
    fn filtered_count(&self) -> usize {
        self.inner.filtered_count
    }

    #[getter]
    fn discovery_time_ms(&self) -> u64 {
        self.inner.discovery_time_ms
    }

    fn __repr__(&self) -> String {
        format!(
            "DiscoveryStats(files_found={}, filtered_count={}, discovery_time_ms={}ms)",
            self.files_found(),
            self.filtered_count(),
            self.discovery_time_ms()
        )
    }
}

/// Walk files and discover test/benchmark files
#[pyfunction]
fn discover_files(config: PyDiscoveryConfig) -> PyResult<Vec<PyFileInfo>> {
    let files = walk_files(&config.inner)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    Ok(files.into_iter().map(|f| PyFileInfo { inner: f }).collect())
}

/// Filter files by pattern
#[pyfunction]
fn filter_files_by_pattern(files: Vec<PyFileInfo>, pattern: String) -> Vec<PyFileInfo> {
    let rust_files: Vec<FileInfo> = files.into_iter().map(|f| f.inner).collect();
    let filtered = filter_files(rust_files, &pattern);
    filtered.into_iter().map(|f| PyFileInfo { inner: f }).collect()
}

// =====================
// Profiler types
// =====================

/// Python ProfilePhase enum
#[pyclass(name = "ProfilePhase", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyProfilePhase {
    PythonExtract,
    RustConvert,
    NetworkIO,
    PyO3Boundary,
    Total,
}

impl From<ProfilePhase> for PyProfilePhase {
    fn from(phase: ProfilePhase) -> Self {
        match phase {
            ProfilePhase::PythonExtract => PyProfilePhase::PythonExtract,
            ProfilePhase::RustConvert => PyProfilePhase::RustConvert,
            ProfilePhase::NetworkIO => PyProfilePhase::NetworkIO,
            ProfilePhase::PyO3Boundary => PyProfilePhase::PyO3Boundary,
            ProfilePhase::Total => PyProfilePhase::Total,
        }
    }
}

impl From<PyProfilePhase> for ProfilePhase {
    fn from(phase: PyProfilePhase) -> Self {
        match phase {
            PyProfilePhase::PythonExtract => ProfilePhase::PythonExtract,
            PyProfilePhase::RustConvert => ProfilePhase::RustConvert,
            PyProfilePhase::NetworkIO => ProfilePhase::NetworkIO,
            PyProfilePhase::PyO3Boundary => ProfilePhase::PyO3Boundary,
            PyProfilePhase::Total => ProfilePhase::Total,
        }
    }
}

#[pymethods]
impl PyProfilePhase {
    fn __str__(&self) -> &'static str {
        match self {
            PyProfilePhase::PythonExtract => "PythonExtract",
            PyProfilePhase::RustConvert => "RustConvert",
            PyProfilePhase::NetworkIO => "NetworkIO",
            PyProfilePhase::PyO3Boundary => "PyO3Boundary",
            PyProfilePhase::Total => "Total",
        }
    }

    fn __repr__(&self) -> String {
        format!("ProfilePhase.{}", self.__str__())
    }
}

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

/// Python GilTestConfig class
#[pyclass(name = "GilTestConfig")]
#[derive(Clone)]
pub struct PyGilTestConfig {
    inner: GilTestConfig,
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

/// Generate flamegraph SVG from folded stacks
#[pyfunction]
fn generate_flamegraph(folded_stacks: Vec<String>, title: &str, output_path: &str) -> PyResult<()> {
    generate_flamegraph_svg(&folded_stacks, title, output_path)
        .map_err(PyErr::new::<pyo3::exceptions::PyIOError, _>)
}

// =====================
// Parametrize
// =====================

/// Python ParameterValue class
#[pyclass(name = "ParameterValue")]
#[derive(Clone)]
pub struct PyParameterValue {
    inner: ParameterValue,
}

#[pymethods]
impl PyParameterValue {
    /// Create an integer parameter value
    #[staticmethod]
    fn int(value: i64) -> Self {
        Self {
            inner: ParameterValue::Int(value),
        }
    }

    /// Create a float parameter value
    #[staticmethod]
    fn float(value: f64) -> Self {
        Self {
            inner: ParameterValue::Float(value),
        }
    }

    /// Create a string parameter value
    #[staticmethod]
    fn string(value: String) -> Self {
        Self {
            inner: ParameterValue::String(value),
        }
    }

    /// Create a boolean parameter value
    #[staticmethod]
    fn bool(value: bool) -> Self {
        Self {
            inner: ParameterValue::Bool(value),
        }
    }

    /// Create a None parameter value
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: ParameterValue::None,
        }
    }

    /// Create from Python object (auto-conversion)
    #[staticmethod]
    fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(v) = obj.extract::<i64>() {
            Ok(Self::int(v))
        } else if let Ok(v) = obj.extract::<f64>() {
            Ok(Self::float(v))
        } else if let Ok(v) = obj.extract::<String>() {
            Ok(Self::string(v))
        } else if let Ok(v) = obj.extract::<bool>() {
            Ok(Self::bool(v))
        } else if obj.is_none() {
            Ok(Self::none())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                format!("Unsupported parameter type: {}", obj.get_type())
            ))
        }
    }

    /// Format for test name
    fn format_for_name(&self) -> String {
        self.inner.format_for_name()
    }

    /// Convert to Python object
    fn to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.inner {
            ParameterValue::Int(v) => Ok(v.to_object(py)),
            ParameterValue::Float(v) => Ok(v.to_object(py)),
            ParameterValue::String(v) => Ok(v.to_object(py)),
            ParameterValue::Bool(v) => Ok(v.to_object(py)),
            ParameterValue::None => Ok(py.None()),
            ParameterValue::List(_) => Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                "List parameter values not yet supported for Python conversion"
            )),
            ParameterValue::Dict(_) => Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                "Dict parameter values not yet supported for Python conversion"
            )),
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ParameterValue({})", self.inner)
    }
}

/// Python ParameterSet class
#[pyclass(name = "ParameterSet")]
#[derive(Clone)]
pub struct PyParameterSet {
    inner: ParameterSet,
}

#[pymethods]
impl PyParameterSet {
    #[new]
    fn new() -> Self {
        Self {
            inner: ParameterSet::new(),
        }
    }

    /// Add a parameter
    fn add(&mut self, name: String, value: PyParameterValue) {
        self.inner.add(name, value.inner);
    }

    /// Get a parameter value
    fn get(&self, name: &str) -> Option<PyParameterValue> {
        self.inner.get(name).map(|v| PyParameterValue { inner: v.clone() })
    }

    /// Format for test name
    fn format_for_name(&self) -> String {
        self.inner.format_for_name()
    }

    /// Convert to Python dict
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.params {
            let py_val = PyParameterValue { inner: v.clone() }.to_py(py)?;
            dict.set_item(k, py_val)?;
        }
        Ok(dict.to_object(py))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("ParameterSet({})", self.inner.format_for_name())
    }
}

/// Python Parameter class
#[pyclass(name = "Parameter")]
#[derive(Clone)]
pub struct PyParameter {
    inner: Parameter,
}

#[pymethods]
impl PyParameter {
    #[new]
    fn new(name: String, values: Vec<PyParameterValue>) -> Self {
        let values = values.into_iter().map(|v| v.inner).collect();
        Self {
            inner: Parameter::new(name, values),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn values(&self) -> Vec<PyParameterValue> {
        self.inner.values.iter().map(|v| PyParameterValue { inner: v.clone() }).collect()
    }

    /// Validate the parameter
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e)
        })
    }

    fn __repr__(&self) -> String {
        format!("Parameter(name='{}', values={} items)", self.inner.name, self.inner.values.len())
    }
}

/// Python ParametrizedTest class
#[pyclass(name = "ParametrizedTest")]
#[derive(Clone)]
pub struct PyParametrizedTest {
    inner: ParametrizedTest,
}

#[pymethods]
impl PyParametrizedTest {
    #[new]
    fn new(base_name: String) -> Self {
        Self {
            inner: ParametrizedTest::new(base_name),
        }
    }

    /// Add a parameter
    fn add_parameter(&mut self, param: PyParameter) -> PyResult<()> {
        self.inner.add_parameter(param.inner).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e)
        })
    }

    /// Expand into test instances
    fn expand(&self) -> Vec<(String, PyParameterSet)> {
        self.inner.expand().into_iter().map(|(name, set)| {
            (name, PyParameterSet { inner: set })
        }).collect()
    }

    /// Count total instances
    fn count_instances(&self) -> usize {
        self.inner.count_instances()
    }

    #[getter]
    fn base_name(&self) -> &str {
        &self.inner.base_name
    }

    fn __repr__(&self) -> String {
        format!(
            "ParametrizedTest(base_name='{}', parameters={}, instances={})",
            self.inner.base_name,
            self.inner.parameters.len(),
            self.inner.count_instances()
        )
    }
}

// =====================
// Fixtures
// =====================

/// Python FixtureScope enum
#[pyclass(name = "FixtureScope", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFixtureScope {
    Function,
    Class,
    Module,
    Session,
}

impl From<PyFixtureScope> for FixtureScope {
    fn from(py_scope: PyFixtureScope) -> Self {
        match py_scope {
            PyFixtureScope::Function => FixtureScope::Function,
            PyFixtureScope::Class => FixtureScope::Class,
            PyFixtureScope::Module => FixtureScope::Module,
            PyFixtureScope::Session => FixtureScope::Session,
        }
    }
}

impl From<FixtureScope> for PyFixtureScope {
    fn from(scope: FixtureScope) -> Self {
        match scope {
            FixtureScope::Function => PyFixtureScope::Function,
            FixtureScope::Class => PyFixtureScope::Class,
            FixtureScope::Module => PyFixtureScope::Module,
            FixtureScope::Session => PyFixtureScope::Session,
        }
    }
}

impl std::fmt::Display for PyFixtureScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PyFixtureScope::Function => write!(f, "function"),
            PyFixtureScope::Class => write!(f, "class"),
            PyFixtureScope::Module => write!(f, "module"),
            PyFixtureScope::Session => write!(f, "session"),
        }
    }
}

#[pymethods]
impl PyFixtureScope {
    fn __str__(&self) -> &'static str {
        match self {
            PyFixtureScope::Function => "function",
            PyFixtureScope::Class => "class",
            PyFixtureScope::Module => "module",
            PyFixtureScope::Session => "session",
        }
    }

    fn __repr__(&self) -> String {
        format!("FixtureScope.{}", self.__str__().to_uppercase())
    }

    #[staticmethod]
    fn from_string(s: &str) -> PyResult<Self> {
        s.parse::<FixtureScope>()
            .map(PyFixtureScope::from)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
    }
}

/// Python FixtureRegistry wrapper
#[pyclass(name = "FixtureRegistry")]
pub struct PyFixtureRegistry {
    inner: Arc<Mutex<FixtureRegistry>>,
}

#[pymethods]
impl PyFixtureRegistry {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(FixtureRegistry::new())),
        }
    }

    /// Register a fixture from Python
    fn register(
        &self,
        name: &str,
        scope: PyFixtureScope,
        autouse: bool,
        dependencies: Vec<String>,
        has_teardown: bool,
    ) -> PyResult<()> {
        let mut registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        // Create fixture metadata
        let meta = FixtureMeta::new(name, scope.into(), autouse)
            .with_dependencies(dependencies)
            .with_teardown(has_teardown);

        registry.register(meta);
        Ok(())
    }

    /// Get fixture metadata by name
    fn get_meta(&self, name: &str) -> PyResult<Option<PyFixtureMeta>> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        Ok(registry.get_meta(name).map(|m| PyFixtureMeta {
            name: m.name.clone(),
            scope: m.scope.into(),
            autouse: m.autouse,
            dependencies: m.dependencies.clone(),
            has_teardown: m.has_teardown,
        }))
    }

    /// Get all fixture names
    fn get_all_names(&self) -> PyResult<Vec<String>> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        Ok(registry.get_all_names())
    }

    /// Get autouse fixtures for a scope
    fn get_autouse_fixtures(&self, scope: PyFixtureScope) -> PyResult<Vec<String>> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        let fixtures = registry.get_autouse_fixtures(scope.into());
        Ok(fixtures.iter().map(|f| f.name.clone()).collect())
    }

    /// Resolve fixture dependency order
    fn resolve_order(&self, fixture_names: Vec<String>) -> PyResult<Vec<String>> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        registry.resolve_order(&fixture_names)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
    }

    /// Detect circular dependencies
    fn detect_circular_deps(&self) -> PyResult<()> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        registry.detect_circular_deps().map_err(|cycle| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Circular fixture dependency detected: {}", cycle.join(" -> "))
            )
        })
    }

    /// Check if fixture exists
    fn has_fixture(&self, name: &str) -> PyResult<bool> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        Ok(registry.has_fixture(name))
    }

    /// Get number of registered fixtures
    fn __len__(&self) -> PyResult<usize> {
        let registry = self.inner.lock().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to lock registry: {}", e)
            )
        })?;

        Ok(registry.len())
    }
}

/// Python wrapper for FixtureMeta
#[pyclass(name = "FixtureMeta")]
#[derive(Clone)]
pub struct PyFixtureMeta {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    scope: PyFixtureScope,
    #[pyo3(get)]
    autouse: bool,
    #[pyo3(get)]
    dependencies: Vec<String>,
    #[pyo3(get)]
    has_teardown: bool,
}

#[pymethods]
impl PyFixtureMeta {
    fn __repr__(&self) -> String {
        format!(
            "FixtureMeta(name='{}', scope={}, autouse={}, dependencies={:?}, has_teardown={})",
            self.name, self.scope, self.autouse, self.dependencies, self.has_teardown
        )
    }
}

// =====================
// Hooks
// =====================

use ouroboros_test::HookType;

/// Python HookType enum
#[pyclass(name = "HookType", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyHookType {
    SetupClass,
    TeardownClass,
    SetupModule,
    TeardownModule,
    SetupMethod,
    TeardownMethod,
}

impl From<PyHookType> for HookType {
    fn from(py_type: PyHookType) -> Self {
        match py_type {
            PyHookType::SetupClass => HookType::SetupClass,
            PyHookType::TeardownClass => HookType::TeardownClass,
            PyHookType::SetupModule => HookType::SetupModule,
            PyHookType::TeardownModule => HookType::TeardownModule,
            PyHookType::SetupMethod => HookType::SetupMethod,
            PyHookType::TeardownMethod => HookType::TeardownMethod,
        }
    }
}

impl From<HookType> for PyHookType {
    fn from(hook_type: HookType) -> Self {
        match hook_type {
            HookType::SetupClass => PyHookType::SetupClass,
            HookType::TeardownClass => PyHookType::TeardownClass,
            HookType::SetupModule => PyHookType::SetupModule,
            HookType::TeardownModule => PyHookType::TeardownModule,
            HookType::SetupMethod => PyHookType::SetupMethod,
            HookType::TeardownMethod => PyHookType::TeardownMethod,
        }
    }
}

#[pymethods]
impl PyHookType {
    fn __str__(&self) -> &'static str {
        match self {
            PyHookType::SetupClass => "setup_class",
            PyHookType::TeardownClass => "teardown_class",
            PyHookType::SetupModule => "setup_module",
            PyHookType::TeardownModule => "teardown_module",
            PyHookType::SetupMethod => "setup_method",
            PyHookType::TeardownMethod => "teardown_method",
        }
    }

    fn __repr__(&self) -> String {
        format!("HookType.{}", self.__str__().to_uppercase())
    }
}

/// Python HookRegistry class
#[pyclass(name = "HookRegistry")]
pub struct PyHookRegistry {
    hooks: Arc<Mutex<HashMap<HookType, Vec<PyObject>>>>,
}

#[pymethods]
impl PyHookRegistry {
    /// Create a new hook registry
    #[new]
    fn new() -> Self {
        Self {
            hooks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a hook function
    fn register_hook(&self, hook_type: PyHookType, hook_fn: PyObject) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks
            .entry(hook_type.into())
            .or_insert_with(Vec::new)
            .push(hook_fn);
    }

    /// Clear all hooks of a specific type
    fn clear_hooks(&self, hook_type: PyHookType) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.remove(&hook_type.into());
    }

    /// Clear all hooks
    fn clear_all(&self) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.clear();
    }

    /// Get the number of registered hooks for a specific type
    fn hook_count(&self, hook_type: PyHookType) -> usize {
        let hooks = self.hooks.lock().unwrap();
        hooks
            .get(&hook_type.into())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Run hooks of a specific type (async method)
    fn run_hooks<'py>(
        &self,
        py: Python<'py>,
        hook_type: PyHookType,
        suite_instance: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pyo3_async_runtimes::tokio::future_into_py;

        let hook_type_rust: HookType = hook_type.into();

        // Clone hooks while holding the lock by explicitly cloning each PyObject
        let hooks_to_run: Vec<PyObject> = {
            let hooks = self.hooks.lock().unwrap();
            hooks
                .get(&hook_type_rust)
                .map(|v| v.iter().map(|obj| obj.clone_ref(py)).collect())
                .unwrap_or_default()
        };

        future_into_py(py, async move {
            Python::with_gil(|py| {
                run_hooks_impl(py, hook_type_rust, &hooks_to_run, suite_instance.as_ref())
            })
        })
    }

    fn __repr__(&self) -> String {
        let hooks = self.hooks.lock().unwrap();
        format!(
            "HookRegistry(setup_class={}, teardown_class={}, setup_method={}, teardown_method={}, setup_module={}, teardown_module={})",
            hooks.get(&HookType::SetupClass).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::TeardownClass).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::SetupMethod).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::TeardownMethod).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::SetupModule).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::TeardownModule).map(|v| v.len()).unwrap_or(0),
        )
    }
}

use std::collections::HashMap;

/// Helper function to run hooks (synchronous implementation)
fn run_hooks_impl(
    py: Python<'_>,
    hook_type: HookType,
    hooks: &[PyObject],
    suite_instance: Option<&PyObject>,
) -> PyResult<PyObject> {
    if hooks.is_empty() {
        return Ok(py.None());
    }

    let mut errors = Vec::new();
    let is_teardown = hook_type.is_teardown();

    for (idx, hook_fn) in hooks.iter().enumerate() {
        let hook_name = format!("{}[{}]", hook_type, idx);

        // Check if hook is async
        let asyncio = py.import_bound("asyncio")?;
        let is_coroutine_fn = asyncio.getattr("iscoroutinefunction")?;
        let is_async: bool = is_coroutine_fn.call1((hook_fn,))?.extract()?;

        // Call the hook
        let result = if is_async {
            // Async hook - need to await it
            run_async_hook_sync(py, hook_fn, suite_instance)
        } else {
            // Sync hook - call directly
            run_sync_hook(py, hook_fn, suite_instance)
        };

        // Handle errors
        if let Err(e) = result {
            let error_msg = format!("{} failed: {}", hook_name, e);
            errors.push(error_msg);

            // For setup hooks, fail fast
            if !is_teardown {
                return Err(e);
            }
            // For teardown hooks, collect error but continue
        }
    }

    // Return collected errors (if any)
    if errors.is_empty() {
        Ok(py.None())
    } else {
        Ok(errors.join("; ").into_py(py))
    }
}

/// Run a synchronous hook
fn run_sync_hook(
    py: Python<'_>,
    hook_fn: &PyObject,
    suite_instance: Option<&PyObject>,
) -> PyResult<()> {
    if let Some(instance) = suite_instance {
        // Call as instance method: hook_fn(self)
        hook_fn.call1(py, (instance,))?;
    } else {
        // Call as standalone function: hook_fn()
        hook_fn.call0(py)?;
    }
    Ok(())
}

/// Run an asynchronous hook synchronously
/// Uses get_running_loop().run_until_complete() if inside async context,
/// otherwise falls back to asyncio.run()
fn run_async_hook_sync(
    py: Python<'_>,
    hook_fn: &PyObject,
    suite_instance: Option<&PyObject>,
) -> PyResult<()> {
    // Get the coroutine
    let coro = if let Some(instance) = suite_instance {
        // Call as instance method: await hook_fn(self)
        hook_fn.call1(py, (instance,))?
    } else {
        // Call as standalone function: await hook_fn()
        hook_fn.call0(py)?
    };

    // Check if it's actually a coroutine
    let asyncio = py.import_bound("asyncio")?;
    let is_coro = asyncio.getattr("iscoroutine")?;
    let is_coroutine: bool = is_coro.call1((coro.clone_ref(py),))?.extract()?;

    if !is_coroutine {
        // Not a coroutine, just return
        return Ok(());
    }

    // Try to get the running event loop first
    let get_running_loop = asyncio.getattr("get_running_loop")?;
    match get_running_loop.call0() {
        Ok(loop_obj) => {
            // We're inside an async context - use run_until_complete
            let run_until_complete = loop_obj.getattr("run_until_complete")?;
            run_until_complete.call1((coro,))?;
        }
        Err(_) => {
            // No running loop - use asyncio.run
            let run_fn = asyncio.getattr("run")?;
            run_fn.call1((coro,))?;
        }
    }

    Ok(())
}

// =====================
// Module registration
// =====================

/// Register test module classes and functions
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Enums
    m.add_class::<PyTestType>()?;
    m.add_class::<PyTestStatus>()?;
    m.add_class::<PyReportFormat>()?;
    m.add_class::<PyFileType>()?;

    // Core types
    m.add_class::<PyTestMeta>()?;
    m.add_class::<PyTestResult>()?;
    m.add_class::<PyTestSummary>()?;
    m.add_class::<PyTestRunner>()?;

    // Assertions
    m.add_class::<PyExpectation>()?;
    m.add_function(wrap_pyfunction!(expect, m)?)?;

    // Reporter
    m.add_class::<PyReporter>()?;
    m.add_class::<PyTestReport>()?;

    // Coverage
    m.add_class::<PyFileCoverage>()?;
    m.add_class::<PyCoverageInfo>()?;

    // Benchmark
    m.add_class::<PyBenchmarkStats>()?;
    m.add_class::<PyBenchmarkResult>()?;
    m.add_class::<PyBenchmarkConfig>()?;
    m.add_function(wrap_pyfunction!(compare_benchmarks, m)?)?;
    m.add_function(wrap_pyfunction!(print_comparison_table, m)?)?;

    // Benchmark Report
    m.add_class::<PyBenchmarkEnvironment>()?;
    m.add_class::<PyBenchmarkReportGroup>()?;
    m.add_class::<PyBenchmarkReport>()?;

    // Test Server
    m.add_class::<PyTestServer>()?;
    m.add_class::<PyTestServerHandle>()?;

    // Discovery
    m.add_class::<PyFileInfo>()?;
    m.add_class::<PyDiscoveryConfig>()?;
    m.add_class::<PyTestRegistry>()?;
    m.add_class::<PyBenchmarkRegistry>()?;
    m.add_class::<PyDiscoveryStats>()?;
    m.add_function(wrap_pyfunction!(discover_files, m)?)?;
    m.add_function(wrap_pyfunction!(filter_files_by_pattern, m)?)?;

    // Profiler
    m.add_class::<PyProfilePhase>()?;
    m.add_class::<PyPhaseTiming>()?;
    m.add_class::<PyPhaseBreakdown>()?;
    m.add_class::<PyGilTestConfig>()?;
    m.add_class::<PyGilContentionResult>()?;
    m.add_class::<PyMemorySnapshot>()?;
    m.add_class::<PyMemoryProfile>()?;
    m.add_class::<PyFlamegraphData>()?;
    m.add_class::<PyProfileResult>()?;
    m.add_class::<PyProfileConfig>()?;
    m.add_function(wrap_pyfunction!(generate_flamegraph, m)?)?;

    // Fixtures
    m.add_class::<PyFixtureScope>()?;
    m.add_class::<PyFixtureMeta>()?;
    m.add_class::<PyFixtureRegistry>()?;

    // Parametrize
    m.add_class::<PyParameterValue>()?;
    m.add_class::<PyParameterSet>()?;
    m.add_class::<PyParameter>()?;
    m.add_class::<PyParametrizedTest>()?;

    // Hooks
    m.add_class::<PyHookType>()?;
    m.add_class::<PyHookRegistry>()?;

    Ok(())
}
