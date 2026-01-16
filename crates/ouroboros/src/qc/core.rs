//! Core test types: TestMeta, TestResult, TestSummary.

use ouroboros_qc::{TestMeta, TestResult, runner::TestSummary};
use pyo3::prelude::*;

use super::enums::{PyTestType, PyTestStatus};

// =====================
// TestMeta
// =====================

/// Python TestMeta class - metadata for a test
#[pyclass(name = "TestMeta")]
#[derive(Clone)]
pub struct PyTestMeta {
    pub(super) inner: TestMeta,
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
    pub(super) inner: TestResult,
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
    pub fn is_passed(&self) -> bool {
        self.inner.is_passed()
    }

    /// Check if failed
    pub fn is_failed(&self) -> bool {
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
    pub(super) inner: TestSummary,
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
