//! Discovery types and functions.

use ouroboros_qc::discovery::{
    DiscoveryConfig, FileInfo, TestRegistry, BenchmarkRegistry,
    DiscoveryStats, walk_files, filter_files,
};
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use super::enums::PyFileType;

// =====================
// FileInfo
// =====================

/// Python FileInfo wrapper
#[pyclass(name = "FileInfo")]
#[derive(Clone)]
pub struct PyFileInfo {
    pub(super) inner: FileInfo,
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
        let file_type_str = match self.inner.file_type {
            ouroboros_qc::discovery::FileType::Test => "FileType.TEST",
            ouroboros_qc::discovery::FileType::Benchmark => "FileType.BENCHMARK",
        };
        format!(
            "FileInfo(path='{}', module_name='{}', file_type={})",
            self.path(),
            self.module_name(),
            file_type_str
        )
    }
}

// =====================
// DiscoveryConfig
// =====================

/// Python DiscoveryConfig wrapper
#[pyclass(name = "DiscoveryConfig")]
#[derive(Clone)]
pub struct PyDiscoveryConfig {
    pub(super) inner: DiscoveryConfig,
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

// =====================
// TestRegistry
// =====================

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

// =====================
// BenchmarkRegistry
// =====================

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

// =====================
// DiscoveryStats
// =====================

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

// =====================
// Discovery functions
// =====================

/// Walk files and discover test/benchmark files
#[pyfunction]
pub fn discover_files(config: PyDiscoveryConfig) -> PyResult<Vec<PyFileInfo>> {
    let files = walk_files(&config.inner)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    Ok(files.into_iter().map(|f| PyFileInfo { inner: f }).collect())
}

/// Filter files by pattern
#[pyfunction]
pub fn filter_files_by_pattern(files: Vec<PyFileInfo>, pattern: String) -> Vec<PyFileInfo> {
    let rust_files: Vec<FileInfo> = files.into_iter().map(|f| f.inner).collect();
    let filtered = filter_files(rust_files, &pattern);
    filtered.into_iter().map(|f| PyFileInfo { inner: f }).collect()
}
