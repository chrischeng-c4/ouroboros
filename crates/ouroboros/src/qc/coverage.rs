//! Coverage types: FileCoverage and CoverageInfo.

use ouroboros_qc::{CoverageInfo, FileCoverage};
use pyo3::prelude::*;

// =====================
// FileCoverage
// =====================

/// Python FileCoverage class - coverage info for a single file
#[pyclass(name = "FileCoverage")]
#[derive(Clone)]
pub struct PyFileCoverage {
    pub(super) inner: FileCoverage,
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

// =====================
// CoverageInfo
// =====================

/// Python CoverageInfo class - overall coverage summary
#[pyclass(name = "CoverageInfo")]
#[derive(Clone)]
pub struct PyCoverageInfo {
    pub(super) inner: CoverageInfo,
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
