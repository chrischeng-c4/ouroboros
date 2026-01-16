//! Test server types.

use ouroboros_qc::http_server::{TestServer, TestServerHandle, TestServerConfig};
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// =====================
// TestServerHandle
// =====================

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

// =====================
// TestServer
// =====================

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
pub(super) fn python_to_json(obj: &Bound<'_, pyo3::types::PyAny>) -> PyResult<serde_json::Value> {
    // Try to convert via JSON string (simple approach)
    let json_module = obj.py().import("json")?;
    let json_str: String = json_module.call_method1("dumps", (obj,))?.extract()?;
    serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}
