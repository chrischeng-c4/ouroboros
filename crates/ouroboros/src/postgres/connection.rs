//! Connection management functions.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;
use ouroboros_postgres::{Connection, PoolConfig, RetryConfig};

use super::PG_POOL;

/// Initialize PostgreSQL connection pool
///
/// Args:
///     connection_string: PostgreSQL connection URI (e.g., "postgresql://user:password@localhost/db")
///     min_connections: Minimum number of connections in pool (default: 1)
///     max_connections: Maximum number of connections in pool (default: 10)
///     connect_timeout: Connection timeout in seconds (default: 30)
///     max_lifetime: Maximum lifetime of a connection in seconds (default: 1800 = 30 minutes, None to disable)
///     idle_timeout: Idle timeout in seconds (default: 600 = 10 minutes, None to disable)
///     max_retries: Maximum number of retry attempts for connection (default: 3, 0 to disable)
///     initial_retry_delay_ms: Initial delay between retries in milliseconds (default: 100)
///     max_retry_delay_ms: Maximum delay between retries in milliseconds (default: 5000)
///     statement_cache_capacity: Number of prepared statements to cache per connection (default: 100, 0 to disable)
///
/// Returns:
///     Awaitable that resolves when connection is established
///
/// Example:
///     # Basic usage
///     await init("postgresql://localhost/mydb")
///
///     # With custom pool settings
///     await init("postgresql://localhost/mydb", max_connections=20, connect_timeout=60)
///
///     # With retry configuration
///     await init("postgresql://localhost/mydb", max_retries=5, initial_retry_delay_ms=200)
///
///     # Production configuration
///     await init(
///         "postgresql://localhost/mydb",
///         min_connections=5,
///         max_connections=50,
///         connect_timeout=10,
///         max_lifetime=3600,  # 1 hour
///         idle_timeout=300,   # 5 minutes
///         max_retries=3,
///         statement_cache_capacity=200,  # More caching for high-query workloads
///     )
#[pyfunction]
#[pyo3(signature = (
    connection_string,
    min_connections=1,
    max_connections=10,
    connect_timeout=30,
    max_lifetime=1800,
    idle_timeout=600,
    max_retries=3,
    initial_retry_delay_ms=100,
    max_retry_delay_ms=5000,
    statement_cache_capacity=100
))]
#[allow(clippy::too_many_arguments)]
pub(super) fn init<'py>(
    py: Python<'py>,
    connection_string: String,
    min_connections: u32,
    max_connections: u32,
    connect_timeout: u64,
    max_lifetime: Option<u64>,
    idle_timeout: Option<u64>,
    max_retries: u32,
    initial_retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    statement_cache_capacity: usize,
) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let retry_config = RetryConfig {
            max_retries,
            initial_delay_ms: initial_retry_delay_ms,
            max_delay_ms: max_retry_delay_ms,
            backoff_multiplier: 2.0,
        };

        let config = PoolConfig {
            min_connections,
            max_connections,
            connect_timeout,
            max_lifetime,
            idle_timeout,
            retry: retry_config,
            statement_cache_capacity,
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
