//! Migration support functions.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;

use super::conversion::get_connection;

/// Initialize migration system (create _migrations table)
///
/// Returns:
///     Awaitable that resolves when migration table is created
///
/// Example:
///     await migration_init()
#[pyfunction]
pub(super) fn migration_init<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = ouroboros_postgres::MigrationRunner::new((*conn).clone(), None);
        runner.init()
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to initialize migrations: {}", e)))?;

        Python::with_gil(|py| Ok(py.None()))
    })
}

/// Get migration status (applied and pending migrations)
///
/// Args:
///     migrations_dir: Directory containing migration files
///
/// Returns:
///     Dictionary with 'applied' and 'pending' lists
///
/// Example:
///     status = await migration_status("migrations")
#[pyfunction]
#[pyo3(signature = (migrations_dir))]
pub(super) fn migration_status<'py>(
    py: Python<'py>,
    migrations_dir: String,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = ouroboros_postgres::MigrationRunner::new((*conn).clone(), None);

        let migrations = ouroboros_postgres::MigrationRunner::load_from_directory(std::path::Path::new(&migrations_dir))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to load migrations: {}", e)))?;

        let status = runner.status(&migrations)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get migration status: {}", e)))?;

        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("applied", status.applied)?;
            dict.set_item("pending", status.pending)?;
            Ok(dict.to_object(py))
        })
    })
}

/// Apply all pending migrations from a directory
///
/// Args:
///     migrations_dir: Directory containing migration files
///
/// Returns:
///     List of applied migration versions
///
/// Example:
///     applied = await migration_apply("migrations")
#[pyfunction]
#[pyo3(signature = (migrations_dir))]
pub(super) fn migration_apply<'py>(
    py: Python<'py>,
    migrations_dir: String,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = ouroboros_postgres::MigrationRunner::new((*conn).clone(), None);

        let migrations = ouroboros_postgres::MigrationRunner::load_from_directory(std::path::Path::new(&migrations_dir))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to load migrations: {}", e)))?;

        let applied = runner.migrate(&migrations)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to apply migrations: {}", e)))?;

        Python::with_gil(|py| Ok(applied.to_object(py)))
    })
}

/// Rollback last N migrations
///
/// Args:
///     migrations_dir: Directory containing migration files
///     steps: Number of migrations to rollback (default: 1)
///
/// Returns:
///     List of reverted migration versions
///
/// Example:
///     reverted = await migration_rollback("migrations", steps=2)
#[pyfunction]
#[pyo3(signature = (migrations_dir, steps=1))]
pub(super) fn migration_rollback<'py>(
    py: Python<'py>,
    migrations_dir: String,
    steps: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = ouroboros_postgres::MigrationRunner::new((*conn).clone(), None);

        let migrations = ouroboros_postgres::MigrationRunner::load_from_directory(std::path::Path::new(&migrations_dir))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to load migrations: {}", e)))?;

        let reverted = runner.rollback(&migrations, steps)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to rollback migrations: {}", e)))?;

        Python::with_gil(|py| Ok(reverted.to_object(py)))
    })
}

/// Create a new migration file
///
/// Args:
///     description: Migration description (e.g., "create_users_table")
///     migrations_dir: Directory to create migration file in (default: "migrations")
///
/// Returns:
///     Path to created migration file
///
/// Example:
///     filename = migration_create("create_users_table", "migrations")
#[pyfunction]
#[pyo3(signature = (description, migrations_dir="migrations"))]
pub(super) fn migration_create(
    _py: Python<'_>,
    description: String,
    migrations_dir: &str,
) -> PyResult<String> {
    use chrono::Utc;
    use std::fs;
    use std::path::Path;

    // Create migrations directory if it doesn't exist
    let dir_path = Path::new(migrations_dir);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create migrations directory: {}", e)))?;
    }

    // Generate timestamp-based version
    let now = Utc::now();
    let version = now.format("%Y%m%d_%H%M%S").to_string();

    // Clean description (replace spaces with underscores, lowercase)
    let clean_desc = description
        .replace(' ', "_")
        .to_lowercase();

    // Create filename
    let filename = format!("{}_{}.sql", version, clean_desc);
    let file_path = dir_path.join(&filename);

    // Create migration file template
    let template = format!(
        r#"-- Migration: {}_{}
-- Description: {}

-- UP
CREATE TABLE example (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- DOWN
DROP TABLE IF EXISTS example CASCADE;
"#,
        version, clean_desc, description
    );

    fs::write(&file_path, template)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create migration file: {}", e)))?;

    Ok(file_path.display().to_string())
}

/// Autogenerate migration SQL from schema diff
///
/// Args:
///     current_tables: List of current table dicts (from introspection)
///     desired_tables: List of desired table dicts (from Python Table classes)
///
/// Returns:
///     Dictionary with 'up', 'down', and 'has_changes' keys
///
/// Example:
///     migration = autogenerate_migration(current_tables, desired_tables)
///     if migration['has_changes']:
///         print(migration['up'])
#[pyfunction]
#[pyo3(signature = (current_tables, desired_tables))]
pub(super) fn autogenerate_migration<'py>(
    py: Python<'py>,
    current_tables: Vec<Bound<'py, PyDict>>,
    desired_tables: Vec<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyDict>> {
    use ouroboros_postgres::schema::{SchemaDiff, TableInfo, ColumnInfo, ColumnType, IndexInfo, ForeignKeyInfo};
    use pyo3::types::PyList;

    // Convert Python dicts to Rust TableInfo
    fn dict_to_table_info(_py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<TableInfo> {
        let name: String = dict.get_item("name")?
            .ok_or_else(|| PyValueError::new_err("Missing 'name' in table"))?
            .extract()?;

        let schema: String = dict.get_item("schema")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or_else(|| "public".to_string());

        // Parse columns
        let columns_list = dict.get_item("columns")?
            .ok_or_else(|| PyValueError::new_err("Missing 'columns' in table"))?;
        let columns_list = columns_list.downcast::<PyList>()?;

        let mut columns = Vec::new();
        for col_item in columns_list.iter() {
            let col_dict = col_item.downcast::<PyDict>()?;
            columns.push(dict_to_column_info(col_dict)?);
        }

        // Parse indexes (optional)
        let indexes = match dict.get_item("indexes")? {
            Some(idx_list) => {
                let idx_list = idx_list.downcast::<PyList>()?;
                let mut indexes = Vec::new();
                for idx_item in idx_list.iter() {
                    let idx_dict = idx_item.downcast::<PyDict>()?;
                    indexes.push(dict_to_index_info(idx_dict)?);
                }
                indexes
            }
            None => Vec::new(),
        };

        // Parse foreign keys (optional)
        let foreign_keys = match dict.get_item("foreign_keys")? {
            Some(fk_list) => {
                let fk_list = fk_list.downcast::<PyList>()?;
                let mut fks = Vec::new();
                for fk_item in fk_list.iter() {
                    let fk_dict = fk_item.downcast::<PyDict>()?;
                    fks.push(dict_to_fk_info(fk_dict)?);
                }
                fks
            }
            None => Vec::new(),
        };

        Ok(TableInfo {
            name,
            schema,
            columns,
            indexes,
            foreign_keys,
        })
    }

    fn dict_to_column_info(dict: &Bound<'_, PyDict>) -> PyResult<ColumnInfo> {
        let name: String = dict.get_item("name")?
            .ok_or_else(|| PyValueError::new_err("Missing 'name' in column"))?
            .extract()?;

        let data_type_str: String = dict.get_item("data_type")?
            .ok_or_else(|| PyValueError::new_err("Missing 'data_type' in column"))?
            .extract()?;

        let data_type = ColumnType::parse(&data_type_str);

        let nullable: bool = dict.get_item("nullable")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or(true);

        let default: Option<String> = dict.get_item("default")?
            .and_then(|v| if v.is_none() { None } else { Some(v) })
            .map(|v| v.extract())
            .transpose()?;

        let is_primary_key: bool = dict.get_item("is_primary_key")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or(false);

        let is_unique: bool = dict.get_item("is_unique")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or(false);

        Ok(ColumnInfo {
            name,
            data_type,
            nullable,
            default,
            is_primary_key,
            is_unique,
        })
    }

    fn dict_to_index_info(dict: &Bound<'_, PyDict>) -> PyResult<IndexInfo> {
        let name: String = dict.get_item("name")?
            .ok_or_else(|| PyValueError::new_err("Missing 'name' in index"))?
            .extract()?;

        let columns: Vec<String> = dict.get_item("columns")?
            .ok_or_else(|| PyValueError::new_err("Missing 'columns' in index"))?
            .extract()?;

        let is_unique: bool = dict.get_item("is_unique")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or(false);

        let index_type: String = dict.get_item("index_type")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or_else(|| "btree".to_string());

        Ok(IndexInfo {
            name,
            columns,
            is_unique,
            index_type,
        })
    }

    fn dict_to_fk_info(dict: &Bound<'_, PyDict>) -> PyResult<ForeignKeyInfo> {
        let name: String = dict.get_item("name")?
            .ok_or_else(|| PyValueError::new_err("Missing 'name' in foreign_key"))?
            .extract()?;

        let columns: Vec<String> = dict.get_item("columns")?
            .ok_or_else(|| PyValueError::new_err("Missing 'columns' in foreign_key"))?
            .extract()?;

        let referenced_table: String = dict.get_item("referenced_table")?
            .ok_or_else(|| PyValueError::new_err("Missing 'referenced_table' in foreign_key"))?
            .extract()?;

        let referenced_columns: Vec<String> = dict.get_item("referenced_columns")?
            .ok_or_else(|| PyValueError::new_err("Missing 'referenced_columns' in foreign_key"))?
            .extract()?;

        let on_delete: String = dict.get_item("on_delete")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or_else(|| "NO ACTION".to_string());

        let on_update: String = dict.get_item("on_update")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or_else(|| "NO ACTION".to_string());

        Ok(ForeignKeyInfo {
            name,
            columns,
            referenced_table,
            referenced_columns,
            on_delete,
            on_update,
        })
    }

    // Convert Python dicts to Rust structs
    let mut current: Vec<TableInfo> = Vec::new();
    for table_dict in &current_tables {
        current.push(dict_to_table_info(py, table_dict)?);
    }

    let mut desired: Vec<TableInfo> = Vec::new();
    for table_dict in &desired_tables {
        desired.push(dict_to_table_info(py, table_dict)?);
    }

    // Generate diff
    let diff = SchemaDiff::compare(&current, &desired);

    // Generate SQL
    let up_sql = diff.generate_up_sql();
    let down_sql = diff.generate_down_sql();

    // Return as Python dict
    let result = PyDict::new(py);
    result.set_item("up", up_sql)?;
    result.set_item("down", down_sql)?;
    result.set_item("has_changes", !diff.is_empty())?;

    Ok(result.into())
}
