//! MongoDB connection management functions.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use ouroboros_mongodb::Connection;

use super::validation::{ValidatedCollectionName, py_validated_collection_name, py_validate_query};
use crate::config::get_config;
use crate::error_handling::sanitize_error;

// Global connection instance (Week 10: Changed to RwLock for close/reset support)
pub(super) static CONNECTION: StdRwLock<Option<Arc<Connection>>> = StdRwLock::new(None);

/// Minimum batch size to enable parallel processing
/// Below this threshold, sequential processing is faster due to parallelization overhead
pub(super) const PARALLEL_THRESHOLD: usize = 50;

/// Get the global connection, returning an error if not initialized
pub(super) fn get_connection() -> PyResult<Arc<Connection>> {
    CONNECTION
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?
        .clone()
        .ok_or_else(|| PyRuntimeError::new_err("MongoDB not initialized. Call init() first."))
}

/// Validate collection name for security
///
/// Prevents NoSQL injection via collection names
pub(super) fn validate_collection_name(name: &str) -> PyResult<ValidatedCollectionName> {
    py_validated_collection_name(name)
}

/// Validate MongoDB query for dangerous operators if validation is enabled
///
/// Checks the security config and validates the query if validate_queries is true.
/// Blocks dangerous operators like $where, $function, $accumulator.
///
/// # Arguments
/// * `query` - The query document to validate
///
/// # Errors
/// Returns PyValueError if dangerous operators are detected and validation is enabled
pub(super) fn validate_query_if_enabled(query: &bson::Document) -> PyResult<()> {
    let config = get_config();
    if config.validate_queries {
        py_validate_query(&bson::Bson::Document(query.clone()))?;
    }
    Ok(())
}

/// Initialize MongoDB connection
///
/// Args:
///     connection_string: MongoDB connection URI (e.g., "mongodb://localhost:27017/mydb")
///
/// Returns:
///     None
///
/// Raises:
///     RuntimeError: If already initialized or connection fails
#[pyfunction]
pub fn init<'py>(py: Python<'py>, connection_string: String) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let conn = Connection::new(&connection_string)
            .await
            .map_err(|e| {
                let config = get_config();
                let error_msg = e.to_string();
                let sanitized = sanitize_error(&error_msg, !config.sanitize_errors);
                PyRuntimeError::new_err(sanitized)
            })?;

        // Check if already initialized
        {
            let read_lock = CONNECTION.read()
                .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;
            if read_lock.is_some() {
                return Err(PyRuntimeError::new_err("MongoDB already initialized. Call close() first to reinitialize."));
            }
        }

        // Set connection
        let mut write_lock = CONNECTION.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;
        *write_lock = Some(Arc::new(conn));

        Ok(())
    })
}

/// Get connection status
#[pyfunction]
pub fn is_connected() -> bool {
    CONNECTION.read()
        .ok()
        .and_then(|lock| lock.as_ref().map(|_| true))
        .unwrap_or(false)
}

/// Close the MongoDB connection (Week 10: Connection Lifecycle)
///
/// Closes and releases the current connection. After calling this,
/// init() can be called again to establish a new connection.
///
/// This is useful for:
/// - Clean shutdown
/// - Testing (reset between tests)
/// - Connection refresh/reconnection
///
/// Example:
///     >>> await init("mongodb://localhost:27017/db1")
///     >>> # ... use database ...
///     >>> await close()
///     >>> await init("mongodb://localhost:27017/db2")  # Different database
#[pyfunction]
pub fn close<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let mut write_lock = CONNECTION.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;

        if write_lock.is_none() {
            return Err(PyRuntimeError::new_err("No active connection to close"));
        }

        // Drop the connection (Arc will be dropped when no references remain)
        *write_lock = None;

        Ok(())
    })
}

/// Reset the connection (Week 10: Connection Lifecycle)
///
/// Clear the connection without async operation. Useful for testing.
///
/// Note: This is a synchronous operation for convenience in tests.
/// For production use, prefer close() which is async.
///
/// Example:
///     >>> reset()  # Synchronous, for testing
///     >>> await init("mongodb://localhost:27017/test")
#[pyfunction]
pub fn reset() -> PyResult<()> {
    let mut write_lock = CONNECTION.write()
        .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;
    *write_lock = None;
    Ok(())
}

/// Available features in this build
#[pyfunction]
pub fn available_features() -> Vec<String> {
    vec!["mongodb".to_string()]
}
