//! Many-to-Many relationship functions and raw SQL execution.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;

use ouroboros_postgres::schema::ManyToManyConfig;

use super::conversion::{get_connection, py_value_to_extracted, extracted_to_py_value};

/// Create a join table for many-to-many relationship
///
/// Args:
///     join_table: Name of the join table to create
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     source_table: Name of the source table
///     target_table: Name of the target table
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Example:
///     await m2m_create_join_table("user_roles", "user_id", "role_id", "users", "roles")
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, source_table, target_table, source_reference="id", target_reference="id"))]
pub(super) fn m2m_create_join_table<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    source_table: String,
    target_table: String,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        ouroboros_postgres::row::Row::create_join_table(conn.pool(), &config, &source_table)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(Python::with_gil(|py| py.None()))
    })
}

/// Add a relation between source and target in the join table
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     target_id: ID of the target record
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Example:
///     await m2m_add_relation("user_roles", "user_id", "role_id", "roles", 1, 2)
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, target_id, source_reference="id", target_reference="id"))]
pub(super) fn m2m_add_relation<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    target_id: i64,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        ouroboros_postgres::row::Row::add_m2m_relation(conn.pool(), &config, source_id, target_id)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(Python::with_gil(|py| py.None()))
    })
}

/// Remove a relation between source and target from the join table
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     target_id: ID of the target record
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Returns:
///     int: Number of relations removed
///
/// Example:
///     count = await m2m_remove_relation("user_roles", "user_id", "role_id", "roles", 1, 2)
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, target_id, source_reference="id", target_reference="id"))]
pub(super) fn m2m_remove_relation<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    target_id: i64,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        let affected = ouroboros_postgres::row::Row::remove_m2m_relation(conn.pool(), &config, source_id, target_id)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(affected)
    })
}

/// Clear all relations for a source from the join table
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Returns:
///     int: Number of relations cleared
///
/// Example:
///     count = await m2m_clear_relations("user_roles", "user_id", "role_id", "roles", 1)
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, source_reference="id", target_reference="id"))]
pub(super) fn m2m_clear_relations<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        let affected = ouroboros_postgres::row::Row::clear_m2m_relations(conn.pool(), &config, source_id)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(affected)
    })
}

/// Fetch all related target records for a source through the join table
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     select_columns: Optional list of columns to select from target table
///     order_by: Optional list of (column, direction) tuples for ordering
///     limit: Optional maximum number of records to return
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Returns:
///     List[Dict]: List of related target records
///
/// Example:
///     roles = await m2m_fetch_related("user_roles", "user_id", "role_id", "roles", 1,
///                                      select_columns=["id", "name"],
///                                      order_by=[("name", "ASC")],
///                                      limit=10)
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, select_columns=None, order_by=None, limit=None, source_reference="id", target_reference="id"))]
pub(super) fn m2m_fetch_related<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    select_columns: Option<Vec<String>>,
    order_by: Option<Vec<(String, String)>>,
    limit: Option<i64>,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        // Convert select_columns to references
        let cols_refs: Option<Vec<&str>> = select_columns.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect());

        // Convert order_by to references
        let order_refs: Option<Vec<(&str, &str)>> = order_by.as_ref().map(|v| {
            v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
        });

        let results = ouroboros_postgres::row::Row::fetch_m2m_related(
            conn.pool(),
            &config,
            source_id,
            cols_refs.as_deref(),
            order_refs.as_deref(),
            limit,
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        // Convert to Python dicts
        Python::with_gil(|py| {
            let py_results: Vec<PyObject> = results
                .into_iter()
                .map(|row| {
                    let dict = PyDict::new(py);
                    for (column_name, value) in row {
                        let py_value = extracted_to_py_value(py, &value)?;
                        dict.set_item(column_name, py_value)?;
                    }
                    Ok(dict.to_object(py))
                })
                .collect::<PyResult<Vec<_>>>()?;
            Ok(py_results.into_pyobject(py)?.into_any().unbind())
        })
    })
}

/// Count the number of related target records for a source
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Returns:
///     int: Number of related records
///
/// Example:
///     count = await m2m_count_related("user_roles", "user_id", "role_id", "roles", 1)
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, source_reference="id", target_reference="id"))]
pub(super) fn m2m_count_related<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        let count = ouroboros_postgres::row::Row::count_m2m_related(conn.pool(), &config, source_id)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(count)
    })
}

/// Check if a relation exists between source and target
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     target_id: ID of the target record
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Returns:
///     bool: True if relation exists, False otherwise
///
/// Example:
///     exists = await m2m_has_relation("user_roles", "user_id", "role_id", "roles", 1, 2)
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, target_id, source_reference="id", target_reference="id"))]
pub(super) fn m2m_has_relation<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    target_id: i64,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        let exists = ouroboros_postgres::row::Row::has_m2m_relation(conn.pool(), &config, source_id, target_id)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(exists)
    })
}

/// Set the exact list of related targets (remove old, add new)
///
/// Args:
///     join_table: Name of the join table
///     source_key: Column name for the source foreign key
///     target_key: Column name for the target foreign key
///     target_table: Name of the target table
///     source_id: ID of the source record
///     target_ids: List of target IDs to set as relations
///     source_reference: Column in source table being referenced (default: "id")
///     target_reference: Column in target table being referenced (default: "id")
///
/// Example:
///     await m2m_set_relations("user_roles", "user_id", "role_id", "roles", 1, [2, 3, 4])
#[pyfunction]
#[pyo3(signature = (join_table, source_key, target_key, target_table, source_id, target_ids, source_reference="id", target_reference="id"))]
pub(super) fn m2m_set_relations<'py>(
    py: Python<'py>,
    join_table: String,
    source_key: String,
    target_key: String,
    target_table: String,
    source_id: i64,
    target_ids: Vec<i64>,
    source_reference: &str,
    target_reference: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let source_ref = source_reference.to_string();
    let target_ref = target_reference.to_string();

    future_into_py(py, async move {
        let config = ManyToManyConfig::new(
            join_table, source_key, target_key, target_table
        )
        .with_source_reference(source_ref)
        .with_target_reference(target_ref);

        ouroboros_postgres::row::Row::set_m2m_relations(conn.pool(), &config, source_id, &target_ids)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(Python::with_gil(|py| py.None()))
    })
}

/// Execute raw SQL query
///
/// Args:
///     sql: SQL query string with $1, $2, etc. placeholders
///     params: Optional list of parameters to bind
///
/// Returns:
///     - List[Dict] for SELECT queries
///     - int (rows affected) for INSERT/UPDATE/DELETE
///     - None for DDL commands (CREATE, ALTER, DROP)
///
/// Example:
///     # SELECT query
///     results = await execute("SELECT * FROM users WHERE age > $1", [25])
///
///     # INSERT query
///     count = await execute("INSERT INTO users (name, age) VALUES ($1, $2)", ["Alice", 30])
///
///     # DDL command
///     await execute("CREATE INDEX idx_users_age ON users(age)")
#[pyfunction]
#[pyo3(signature = (sql, params=None))]
pub(super) fn execute<'py>(
    py: Python<'py>,
    sql: String,
    params: Option<Vec<Bound<'py, PyAny>>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Extract parameters from Python to Rust (while holding GIL)
    let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = if let Some(param_list) = params {
        param_list
            .iter()
            .map(|p| py_value_to_extracted(py, p))
            .collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    future_into_py(py, async move {
        use sqlx::postgres::PgArguments;

        let pool = conn.pool();

        // Determine query type by examining the SQL (case-insensitive)
        let sql_upper = sql.trim().to_uppercase();
        let has_returning = sql_upper.contains("RETURNING");
        let is_select = sql_upper.starts_with("SELECT") || sql_upper.starts_with("WITH") || has_returning;
        let is_dml = (sql_upper.starts_with("INSERT")
            || sql_upper.starts_with("UPDATE")
            || sql_upper.starts_with("DELETE")) && !has_returning;

        // Bind parameters to query
        let mut args = PgArguments::default();
        for param in &extracted_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        if is_select {
            // Execute SELECT query and return rows
            let rows = sqlx::query_with(&sql, args)
                .fetch_all(pool)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Query execution failed: {}", e)))?;

            // Convert rows to Python dicts
            let result = Python::with_gil(|py| -> PyResult<PyObject> {
                let py_list = PyList::empty(py);

                for row in rows {
                    // Convert row to ExtractedValue map
                    let columns = ouroboros_postgres::row_to_extracted(&row)
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to extract row: {}", e)))?;

                    // Create Python dict
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
            // Execute DML query and return affected row count
            let result = sqlx::query_with(&sql, args)
                .execute(pool)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Query execution failed: {}", e)))?;

            Python::with_gil(|py| {
                Ok(result.rows_affected().to_object(py))
            })
        } else {
            // Execute DDL or other commands (no return value)
            sqlx::query_with(&sql, args)
                .execute(pool)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Query execution failed: {}", e)))?;

            Python::with_gil(|py| {
                Ok(py.None())
            })
        }
    })
}
