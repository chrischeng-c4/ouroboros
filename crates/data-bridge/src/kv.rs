//! PyO3 bindings for KV store client
//!
//! Exposes the TCP client for connecting to kv-server.

use data_bridge_kv_client::{ClientError, KvClient, KvValue};
use pyo3::exceptions::{PyConnectionError, PyKeyError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyFloat, PyInt, PyList, PyString};
use pyo3_async_runtimes::tokio::future_into_py;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Convert Python value to KvValue
fn py_to_kv_value(obj: &Bound<'_, PyAny>) -> PyResult<KvValue> {
    if obj.is_none() {
        return Ok(KvValue::Null);
    }

    if let Ok(v) = obj.downcast::<PyInt>() {
        let i: i64 = v.extract()?;
        return Ok(KvValue::Int(i));
    }

    if let Ok(v) = obj.downcast::<PyFloat>() {
        let f: f64 = v.extract()?;
        return Ok(KvValue::Float(f));
    }

    if let Ok(v) = obj.downcast::<PyString>() {
        let s: String = v.extract()?;
        return Ok(KvValue::String(s));
    }

    if let Ok(v) = obj.downcast::<PyBytes>() {
        let b: Vec<u8> = v.extract()?;
        return Ok(KvValue::Bytes(b));
    }

    if let Ok(v) = obj.downcast::<PyList>() {
        let mut items = Vec::with_capacity(v.len());
        for item in v.iter() {
            items.push(py_to_kv_value(&item)?);
        }
        return Ok(KvValue::List(items));
    }

    if let Ok(v) = obj.downcast::<PyDict>() {
        let mut map = HashMap::new();
        for (key, value) in v.iter() {
            let k: String = key.extract()?;
            let v = py_to_kv_value(&value)?;
            map.insert(k, v);
        }
        return Ok(KvValue::Map(map));
    }

    // Check for Decimal
    if let Ok(type_name) = obj.get_type().name() {
        if type_name == "Decimal" {
            let s: String = obj.str()?.extract()?;
            let d = Decimal::from_str_exact(&s)
                .map_err(|e| PyValueError::new_err(format!("Invalid Decimal: {}", e)))?;
            return Ok(KvValue::Decimal(d));
        }
    }

    let type_name = obj.get_type().name().map(|n| n.to_string()).unwrap_or_else(|_| "unknown".to_string());
    Err(PyTypeError::new_err(format!(
        "Unsupported type: {}",
        type_name
    )))
}

/// Convert KvValue to Python object
fn kv_value_to_py(py: Python<'_>, value: KvValue) -> PyResult<PyObject> {
    match value {
        KvValue::Null => Ok(py.None()),
        KvValue::Int(i) => Ok(i.into_pyobject(py)?.into_any().unbind()),
        KvValue::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
        KvValue::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        KvValue::Bytes(b) => Ok(PyBytes::new(py, &b).into_any().unbind()),
        KvValue::Decimal(d) => {
            let decimal_mod = py.import("decimal")?;
            let decimal_class = decimal_mod.getattr("Decimal")?;
            let py_decimal = decimal_class.call1((d.to_string(),))?;
            Ok(py_decimal.into_any().unbind())
        }
        KvValue::List(items) => {
            let py_list = PyList::empty(py);
            for item in items {
                py_list.append(kv_value_to_py(py, item)?)?;
            }
            Ok(py_list.into_any().unbind())
        }
        KvValue::Map(map) => {
            let py_dict = PyDict::new(py);
            for (k, v) in map {
                py_dict.set_item(k, kv_value_to_py(py, v)?)?;
            }
            Ok(py_dict.into_any().unbind())
        }
    }
}

/// Map ClientError to Python exception
fn client_error_to_py(e: ClientError) -> PyErr {
    match e {
        ClientError::Connection(e) => PyConnectionError::new_err(e.to_string()),
        ClientError::Protocol(e) => PyRuntimeError::new_err(format!("Protocol error: {}", e)),
        ClientError::Server(msg) => PyRuntimeError::new_err(format!("Server error: {}", msg)),
        ClientError::KeyNotFound => PyKeyError::new_err("Key not found"),
    }
}

/// KV Store client for connecting to kv-server
#[pyclass(name = "KvClient")]
pub struct PyKvClient {
    client: Arc<Mutex<KvClient>>,
}

#[pymethods]
impl PyKvClient {
    /// Connect to a KV server
    ///
    /// Args:
    ///     addr: Server address (e.g., "127.0.0.1:6380")
    ///
    /// Returns:
    ///     Connected KvClient instance
    #[staticmethod]
    fn connect(py: Python<'_>, addr: String) -> PyResult<Bound<'_, PyAny>> {
        future_into_py(py, async move {
            let client = KvClient::connect(&addr).await.map_err(client_error_to_py)?;
            Ok(PyKvClient {
                client: Arc::new(Mutex::new(client)),
            })
        })
    }

    /// Ping the server
    ///
    /// Returns:
    ///     "PONG" if server is responsive
    fn ping<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.ping().await.map_err(client_error_to_py)
        })
    }

    /// Get a value by key
    ///
    /// Args:
    ///     key: The key to look up
    ///
    /// Returns:
    ///     The value, or None if not found
    fn get<'py>(&self, py: Python<'py>, key: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            let result = guard.get(&key).await.map_err(client_error_to_py)?;
            Python::with_gil(|py| match result {
                Some(value) => kv_value_to_py(py, value),
                None => Ok(py.None()),
            })
        })
    }

    /// Set a value
    ///
    /// Args:
    ///     key: The key to set
    ///     value: The value to store
    ///     ttl: Optional time-to-live in seconds
    #[pyo3(signature = (key, value, ttl = None))]
    fn set<'py>(
        &self,
        py: Python<'py>,
        key: String,
        value: &Bound<'py, PyAny>,
        ttl: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kv_value = py_to_kv_value(value)?;
        let duration = ttl.map(Duration::from_secs_f64);
        let client = self.client.clone();

        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.set(&key, kv_value, duration).await.map_err(client_error_to_py)?;
            Ok(())
        })
    }

    /// Delete a key
    ///
    /// Args:
    ///     key: The key to delete
    ///
    /// Returns:
    ///     True if the key existed, False otherwise
    fn delete<'py>(&self, py: Python<'py>, key: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.delete(&key).await.map_err(client_error_to_py)
        })
    }

    /// Check if a key exists
    ///
    /// Args:
    ///     key: The key to check
    ///
    /// Returns:
    ///     True if the key exists, False otherwise
    fn exists<'py>(&self, py: Python<'py>, key: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.exists(&key).await.map_err(client_error_to_py)
        })
    }

    /// Atomically increment an integer value
    ///
    /// Args:
    ///     key: The key to increment
    ///     delta: Amount to add (default: 1)
    ///
    /// Returns:
    ///     The new value after incrementing
    #[pyo3(signature = (key, delta = 1))]
    fn incr<'py>(&self, py: Python<'py>, key: String, delta: i64) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.incr(&key, delta).await.map_err(client_error_to_py)
        })
    }

    /// Atomically decrement an integer value
    ///
    /// Args:
    ///     key: The key to decrement
    ///     delta: Amount to subtract (default: 1)
    ///
    /// Returns:
    ///     The new value after decrementing
    #[pyo3(signature = (key, delta = 1))]
    fn decr<'py>(&self, py: Python<'py>, key: String, delta: i64) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.decr(&key, delta).await.map_err(client_error_to_py)
        })
    }

    /// Get server info
    ///
    /// Returns:
    ///     JSON string with server statistics
    fn info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let mut guard = client.lock().await;
            guard.info().await.map_err(client_error_to_py)
        })
    }

    fn __repr__(&self) -> String {
        "KvClient(connected)".to_string()
    }
}

/// Register the KV module
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyKvClient>()?;
    m.add("__doc__", "KV store client for connecting to kv-server via TCP")?;
    Ok(())
}
