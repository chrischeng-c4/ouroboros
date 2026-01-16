//! Connection management functions.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;
use ouroboros_postgres::{Connection, PoolConfig};

use super::PG_POOL;

/// Initialize PostgreSQL connection pool
///
/// Args:
///     connection_string: PostgreSQL connection URI (e.g., "postgresql://user:password@localhost/db")
///     min_connections: Minimum number of connections in pool (default: 1)
///     max_connections: Maximum number of connections in pool (default: 10)
///     connect_timeout: Connection timeout in seconds (default: 30)
///
/// Returns:
///     Awaitable that resolves when connection is established
///
/// Example:
///     await init("postgresql://localhost/mydb", max_connections=20)
#[pyfunction]
#[pyo3(signature = (connection_string, min_connections=1, max_connections=10, connect_timeout=30))]
pub(super) fn init<'py>(
    py: Python<'py>,
    connection_string: String,
    min_connections: u32,
    max_connections: u32,
    connect_timeout: u64,
) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let config = PoolConfig {
            min_connections,
            max_connections,
            connect_timeout,
            max_lifetime: Some(1800), // 30 minutes
            idle_timeout: Some(600),   // 10 minutes
        };

        let connection = Connection::new(&connection_string, config)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to initialize PostgreSQL: {}", e)))?;

        let mut pool = PG_POOL
            .write()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to acquire pool lock: {}", e)))?;

        *pool = Some(Arc::new(connection));

        Ok(())
    })
}

/// Close the PostgreSQL connection pool
///
/// Returns:
///     Awaitable that resolves when pool is closed
///
/// Example:
///     await close()
#[pyfunction]
pub(super) fn close<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let pool = PG_POOL
            .write()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to acquire pool lock: {}", e)))?
            .take();

        if let Some(conn) = pool {
            conn.close()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to close pool: {}", e)))?;
        }

        Ok(())
    })
}

/// Check if PostgreSQL connection is initialized
///
/// Returns:
///     bool: True if connected, False otherwise
///
/// Example:
///     if is_connected():
///         print("Connected to PostgreSQL")
#[pyfunction]
pub(super) fn is_connected(_py: Python<'_>) -> PyResult<bool> {
    let pool = PG_POOL
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to acquire pool lock: {}", e)))?;

    Ok(pool.is_some())
}
