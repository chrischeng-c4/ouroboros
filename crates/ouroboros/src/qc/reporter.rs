//! Reporter and TestReport types.

use ouroboros_qc::{Reporter, TestReport, TestResult, reporter::EnvironmentInfo};
use pyo3::prelude::*;

use super::core::{PyTestResult, PyTestSummary};
use super::enums::{PyReportFormat, PyTestType};
use super::coverage::PyCoverageInfo;

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
    pub(super) inner: TestReport,
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
