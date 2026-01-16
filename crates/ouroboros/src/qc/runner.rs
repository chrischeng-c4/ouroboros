//! Test runner and helper functions.

use ouroboros_qc::{TestRunner, TestResult, runner::RunnerConfig};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::core::{PyTestMeta, PyTestResult, PyTestSummary};
use super::enums::PyTestType;

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
pub(super) fn call_async_method(py: Python<'_>, obj: &Bound<'_, pyo3::PyAny>, method_name: &str) -> PyResult<()> {
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
