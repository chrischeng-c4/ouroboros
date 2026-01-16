//! Fixture types.

use ouroboros_qc::fixtures::{FixtureMeta, FixtureRegistry};
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use super::enums::PyFixtureScope;

// =====================
// FixtureRegistry
// =====================

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

// =====================
// FixtureMeta
// =====================

/// Python wrapper for FixtureMeta
#[pyclass(name = "FixtureMeta")]
#[derive(Clone)]
pub struct PyFixtureMeta {
    #[pyo3(get)]
    pub(super) name: String,
    #[pyo3(get)]
    pub(super) scope: PyFixtureScope,
    #[pyo3(get)]
    pub(super) autouse: bool,
    #[pyo3(get)]
    pub(super) dependencies: Vec<String>,
    #[pyo3(get)]
    pub(super) has_teardown: bool,
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
