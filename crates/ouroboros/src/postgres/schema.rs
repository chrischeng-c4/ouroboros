//! Schema introspection functions.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;

use super::conversion::get_connection;

/// List all tables in a schema
///
/// Args:
///     schema: Schema name (default: "public")
///
/// Returns:
///     List of table names
///
/// Example:
///     tables = await list_tables("public")
#[pyfunction]
#[pyo3(signature = (schema=None))]
pub(super) fn list_tables<'py>(
    py: Python<'py>,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = ouroboros_postgres::SchemaInspector::new((*conn).clone());
        let tables = inspector.list_tables(schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to list tables: {}", e)))?;

        Python::with_gil(|py| {
            Ok(tables.to_object(py))
        })
    })
}

/// Check if a table exists
///
/// Args:
///     table: Table name
///     schema: Schema name (default: "public")
///
/// Returns:
///     True if table exists, False otherwise
///
/// Example:
///     exists = await table_exists("users", "public")
#[pyfunction]
#[pyo3(signature = (table, schema=None))]
pub(super) fn table_exists<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = ouroboros_postgres::SchemaInspector::new((*conn).clone());
        let exists = inspector.table_exists(&table, schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to check table existence: {}", e)))?;

        Python::with_gil(|py| {
            Ok(exists.to_object(py))
        })
    })
}

/// Get column information for a table
///
/// Args:
///     table: Table name
///     schema: Schema name (default: "public")
///
/// Returns:
///     List of dictionaries with column information
///
/// Example:
///     columns = await get_columns("users", "public")
#[pyfunction]
#[pyo3(signature = (table, schema=None))]
pub(super) fn get_columns<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = ouroboros_postgres::SchemaInspector::new((*conn).clone());
        let columns = inspector.get_columns(&table, schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get columns: {}", e)))?;

        Python::with_gil(|py| {
            // Convert columns to Python list of dicts
            let py_list = PyList::empty(py);
            for col in columns {
                let py_dict = PyDict::new(py);
                py_dict.set_item("name", col.name)?;
                py_dict.set_item("data_type", format!("{:?}", col.data_type))?;
                py_dict.set_item("nullable", col.nullable)?;
                py_dict.set_item("default", col.default)?;
                py_dict.set_item("is_primary_key", col.is_primary_key)?;
                py_dict.set_item("is_unique", col.is_unique)?;
                py_list.append(py_dict)?;
            }
            Ok(py_list.to_object(py))
        })
    })
}

/// Get index information for a table
///
/// Args:
///     table: Table name
///     schema: Schema name (default: "public")
///
/// Returns:
///     List of dictionaries with index information
///
/// Example:
///     indexes = await get_indexes("users", "public")
#[pyfunction]
#[pyo3(signature = (table, schema=None))]
pub(super) fn get_indexes<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = ouroboros_postgres::SchemaInspector::new((*conn).clone());
        let indexes = inspector.get_indexes(&table, schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get indexes: {}", e)))?;

        Python::with_gil(|py| {
            // Convert indexes to Python list of dicts
            let py_list = PyList::empty(py);
            for idx in indexes {
                let py_dict = PyDict::new(py);
                py_dict.set_item("name", idx.name)?;
                py_dict.set_item("columns", idx.columns)?;
                py_dict.set_item("is_unique", idx.is_unique)?;
                py_dict.set_item("index_type", idx.index_type)?;
                py_list.append(py_dict)?;
            }
            Ok(py_list.to_object(py))
        })
    })
}

/// Get foreign key information for a table
///
/// Args:
///     table: Table name
///     schema: Schema name (default: "public")
///
/// Returns:
///     List of dictionaries with foreign key information
///
/// Example:
///     foreign_keys = await get_foreign_keys("posts", "public")
#[pyfunction]
#[pyo3(signature = (table, schema=None))]
pub(super) fn get_foreign_keys<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = ouroboros_postgres::SchemaInspector::new((*conn).clone());
        let foreign_keys = inspector.get_foreign_keys(&table, schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get foreign keys: {}", e)))?;

        Python::with_gil(|py| {
            // Convert foreign keys to Python list of dicts
            let py_list = PyList::empty(py);
            for fk in foreign_keys {
                let py_dict = PyDict::new(py);
                py_dict.set_item("name", fk.name)?;
                py_dict.set_item("columns", fk.columns)?;
                py_dict.set_item("referenced_table", fk.referenced_table)?;
                py_dict.set_item("referenced_columns", fk.referenced_columns)?;
                py_dict.set_item("on_delete", fk.on_delete)?;
                py_dict.set_item("on_update", fk.on_update)?;
                py_list.append(py_dict)?;
            }
            Ok(py_list.to_object(py))
        })
    })
}

/// Get complete table information
///
/// Args:
///     table: Table name
///     schema: Schema name (default: "public")
///
/// Returns:
///     Dictionary with table information including columns and indexes
///
/// Example:
///     info = await inspect_table("users", "public")
#[pyfunction]
#[pyo3(signature = (table, schema=None))]
pub(super) fn inspect_table<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = ouroboros_postgres::SchemaInspector::new((*conn).clone());
        let table_info = inspector.inspect_table(&table, schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to inspect table: {}", e)))?;

        Python::with_gil(|py| {
            let py_dict = PyDict::new(py);
            py_dict.set_item("name", table_info.name)?;
            py_dict.set_item("schema", table_info.schema)?;

            // Convert columns
            let columns_list = PyList::empty(py);
            for col in table_info.columns {
                let col_dict = PyDict::new(py);
                col_dict.set_item("name", col.name)?;
                col_dict.set_item("data_type", format!("{:?}", col.data_type))?;
                col_dict.set_item("nullable", col.nullable)?;
                col_dict.set_item("default", col.default)?;
                col_dict.set_item("is_primary_key", col.is_primary_key)?;
                col_dict.set_item("is_unique", col.is_unique)?;
                columns_list.append(col_dict)?;
            }
            py_dict.set_item("columns", columns_list)?;

            // Convert indexes
            let indexes_list = PyList::empty(py);
            for idx in table_info.indexes {
                let idx_dict = PyDict::new(py);
                idx_dict.set_item("name", idx.name)?;
                idx_dict.set_item("columns", idx.columns)?;
                idx_dict.set_item("is_unique", idx.is_unique)?;
                idx_dict.set_item("index_type", idx.index_type)?;
                indexes_list.append(idx_dict)?;
            }
            py_dict.set_item("indexes", indexes_list)?;

            // Convert foreign keys
            let foreign_keys_list = PyList::empty(py);
            for fk in table_info.foreign_keys {
                let fk_dict = PyDict::new(py);
                fk_dict.set_item("name", fk.name)?;
                fk_dict.set_item("columns", fk.columns)?;
                fk_dict.set_item("referenced_table", fk.referenced_table)?;
                fk_dict.set_item("referenced_columns", fk.referenced_columns)?;
                fk_dict.set_item("on_delete", fk.on_delete)?;
                fk_dict.set_item("on_update", fk.on_update)?;
                foreign_keys_list.append(fk_dict)?;
            }
            py_dict.set_item("foreign_keys", foreign_keys_list)?;

            Ok(py_dict.to_object(py))
        })
    })
}
