//! Transaction support for PostgreSQL.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;

use ouroboros_postgres::{QueryBuilder, Operator, Row, Transaction, transaction::IsolationLevel};

use super::conversion::{
    get_connection, py_dict_to_extracted_values, py_value_to_extracted, extracted_to_py_value,
};
use super::wrappers::{RowWrapper, OptionalRowWrapper};

/// Python wrapper for PostgreSQL transaction
#[pyclass]
#[derive(Clone)]
pub(super) struct PyTransaction {
    tx: Arc<tokio::sync::Mutex<Option<Transaction>>>,
}

#[pymethods]
impl PyTransaction {
    /// Commit the transaction
    fn commit<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_opt = tx_mutex.lock().await;
            let tx = tx_opt.take()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            tx.commit().await
                .map_err(|e| PyRuntimeError::new_err(format!("Commit failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
        })
    }

    /// Rollback the transaction
    fn rollback<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_opt = tx_mutex.lock().await;
            let tx = tx_opt.take()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            tx.rollback().await
                .map_err(|e| PyRuntimeError::new_err(format!("Rollback failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
        })
    }

    /// Create a savepoint within this transaction
    fn savepoint<'py>(&mut self, py: Python<'py>, name: String) -> PyResult<Bound<'py, PyAny>> {
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            tx.savepoint(&name).await
                .map_err(|e| PyRuntimeError::new_err(format!("Savepoint creation failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
        })
    }

    /// Rollback to a savepoint within this transaction
    fn rollback_to_savepoint<'py>(&mut self, py: Python<'py>, name: String) -> PyResult<Bound<'py, PyAny>> {
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            tx.rollback_to(&name).await
                .map_err(|e| PyRuntimeError::new_err(format!("Rollback to savepoint failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
        })
    }

    /// Release a savepoint within this transaction
    fn release_savepoint<'py>(&mut self, py: Python<'py>, name: String) -> PyResult<Bound<'py, PyAny>> {
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            tx.release_savepoint(&name).await
                .map_err(|e| PyRuntimeError::new_err(format!("Release savepoint failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
        })
    }

    /// Insert a single row within this transaction
    fn insert_one<'py>(&mut self, py: Python<'py>, table: String, data: &Bound<'_, PyDict>) -> PyResult<Bound<'py, PyAny>> {
        let values = py_dict_to_extracted_values(py, data)?;
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            let row = Row::insert(&mut **tx.as_mut_transaction(), &table, &values)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Insert failed: {}", e)))?;

            RowWrapper::from_row(&row)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))
        })
    }

    /// Fetch a single row within this transaction
    fn fetch_one<'py>(&mut self, py: Python<'py>, table: String, filter: &Bound<'_, PyDict>) -> PyResult<Bound<'py, PyAny>> {
        let filter_values = py_dict_to_extracted_values(py, filter)?;
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            let mut query = QueryBuilder::new(&table)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

            for (field, value) in filter_values {
                query = query.where_clause(&field, Operator::Eq, value)
                    .map_err(|e| PyRuntimeError::new_err(format!("Invalid filter: {}", e)))?;
            }
            query = query.limit(1);

            let (sql, params) = query.build_select();
            let mut args = sqlx::postgres::PgArguments::default();
            for param in &params {
                param.bind_to_arguments(&mut args)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
            }

            let result = sqlx::query_with(&sql, args)
                .fetch_optional(&mut **tx.as_mut_transaction())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))?;

            let wrapper = if let Some(pg_row) = result {
                let row = Row::from_sqlx(&pg_row)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;
                OptionalRowWrapper(Some(RowWrapper::from_row(&row)?))
            } else {
                OptionalRowWrapper(None)
            };

            Ok(wrapper)
        })
    }

    /// Update a single row within this transaction
    fn update_one<'py>(&mut self, py: Python<'py>, table: String, pk_column: String, pk_value: &Bound<'_, PyAny>, update: &Bound<'_, PyDict>) -> PyResult<Bound<'py, PyAny>> {
        let pk_val = py_value_to_extracted(py, pk_value)?;
        let update_values = py_dict_to_extracted_values(py, update)?;
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            let mut query = QueryBuilder::new(&table)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

            query = query.where_clause(&pk_column, Operator::Eq, pk_val.clone())
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid primary key: {}", e)))?;

            let (sql, params) = query.build_update(&update_values)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to build UPDATE: {}", e)))?;

            let mut args = sqlx::postgres::PgArguments::default();
            for param in &params {
                param.bind_to_arguments(&mut args)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
            }

            let result = sqlx::query_with(&sql, args)
                .execute(&mut **tx.as_mut_transaction())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Update failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Python::with_gil(|py| extracted_to_py_value(py, &pk_val))
            } else {
                Err(PyRuntimeError::new_err("Update failed: no rows affected"))
            }
        })
    }

    /// Delete a single row within this transaction
    fn delete_one<'py>(&mut self, py: Python<'py>, table: String, pk_column: String, pk_value: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let pk_val = py_value_to_extracted(py, pk_value)?;
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            let mut query = QueryBuilder::new(&table)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

            query = query.where_clause(&pk_column, Operator::Eq, pk_val)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid primary key: {}", e)))?;

            let (sql, params) = query.build_delete();
            let mut args = sqlx::postgres::PgArguments::default();
            for param in &params {
                param.bind_to_arguments(&mut args)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
            }

            let result = sqlx::query_with(&sql, args)
                .execute(&mut **tx.as_mut_transaction())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))?;

            Ok(result.rows_affected() as i64)
        })
    }

    /// Execute raw SQL within this transaction
    fn execute<'py>(&mut self, py: Python<'py>, sql: String, params: Option<Vec<Bound<'py, PyAny>>>) -> PyResult<Bound<'py, PyAny>> {
        let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = if let Some(param_list) = params {
            param_list.iter().map(|p| py_value_to_extracted(py, p)).collect::<PyResult<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let tx_mutex = self.tx.clone();

        future_into_py(py, async move {
            let mut tx_lock = tx_mutex.lock().await;
            let tx = tx_lock.as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

            use sqlx::postgres::PgArguments;
            let mut args = PgArguments::default();
            for param in &extracted_params {
                param.bind_to_arguments(&mut args)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
            }

            let sql_upper = sql.trim().to_uppercase();
            let has_returning = sql_upper.contains("RETURNING");
            let is_select = sql_upper.starts_with("SELECT") || sql_upper.starts_with("WITH") || has_returning;
            let is_dml = (sql_upper.starts_with("INSERT") || sql_upper.starts_with("UPDATE") || sql_upper.starts_with("DELETE")) && !has_returning;

            if is_select {
                let rows = sqlx::query_with(&sql, args)
                    .fetch_all(&mut **tx.as_mut_transaction())
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Query execution failed: {}", e)))?;

                let result = Python::with_gil(|py| -> PyResult<PyObject> {
                    let py_list = PyList::empty(py);
                    for row in rows {
                        let columns = ouroboros_postgres::row_to_extracted(&row)
                            .map_err(|e| PyRuntimeError::new_err(format!("Failed to extract row: {}", e)))?;
                        let py_dict = PyDict::new(py);
                        for (column_name, value) in columns {
                            let py_value = extracted_to_py_value(py, &value)?;
                            py_dict.set_item(column_name, py_value)?;
                        }
                        py_list.append(py_dict)?;
                    }
                    Ok(py_list.to_object(py))
                })?;
                Ok(result)
            } else if is_dml {
                let result = sqlx::query_with(&sql, args)
                    .execute(&mut **tx.as_mut_transaction())
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Query execution failed: {}", e)))?;
                Python::with_gil(|py| Ok(result.rows_affected().to_object(py)))
            } else {
                sqlx::query_with(&sql, args)
                    .execute(&mut **tx.as_mut_transaction())
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Query execution failed: {}", e)))?;
                Python::with_gil(|py| Ok(py.None()))
            }
        })
    }
}

/// Begin a transaction
///
/// Args:
///     isolation_level: Transaction isolation level (optional)
///         - "read_uncommitted"
///         - "read_committed" (default)
///         - "repeatable_read"
///         - "serializable"
///
/// Returns:
///     Transaction handle
///
/// Example:
///     tx = await begin_transaction()
///     try:
///         # ... perform operations
///         await tx.commit()
///     except:
///         await tx.rollback()
///
///     # With isolation level
///     tx = await begin_transaction("serializable")
#[pyfunction]
#[pyo3(signature = (isolation_level=None))]
pub(super) fn begin_transaction<'py>(
    py: Python<'py>,
    isolation_level: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Convert isolation level to enum before moving into async block
    let level = match isolation_level {
        Some("read_uncommitted") => IsolationLevel::ReadUncommitted,
        Some("read_committed") => IsolationLevel::ReadCommitted,
        Some("repeatable_read") => IsolationLevel::RepeatableRead,
        Some("serializable") => IsolationLevel::Serializable,
        None => IsolationLevel::ReadCommitted, // PostgreSQL default
        Some(other) => {
            return Err(PyValueError::new_err(format!("Invalid isolation level: {}", other)));
        }
    };

    future_into_py(py, async move {
        let tx = Transaction::begin(&conn, level).await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to begin transaction: {}", e)))?;

        Python::with_gil(|py| {
            Ok(PyTransaction { tx: Arc::new(tokio::sync::Mutex::new(Some(tx))) }.into_py(py))
        })
    })
}
