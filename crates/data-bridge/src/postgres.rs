//! PostgreSQL module for Python bindings
//!
//! This module provides Python bindings for PostgreSQL operations using PyO3.
//! All SQL serialization/deserialization happens in Rust for maximum performance.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use data_bridge_postgres::{Connection, PoolConfig, QueryBuilder, Operator, OrderDirection, Row};
use sqlx::{Column, Row as SqlxRow};

// For base64 encoding of binary data
use base64::{Engine as _, engine::general_purpose};

// Global connection pool using RwLock for close/reset support
use std::sync::RwLock as StdRwLock;
static PG_POOL: StdRwLock<Option<Arc<Connection>>> = StdRwLock::new(None);

// Thread-local cache for datetime module to avoid repeated imports
thread_local! {
    static DATETIME_MODULE: RefCell<Option<PyObject>> = RefCell::new(None);
}

/// Get cached datetime module or import it
fn get_datetime_module(py: Python<'_>) -> PyResult<PyObject> {
    DATETIME_MODULE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(py.import_bound("datetime")?.to_object(py));
        }
        Ok(cache.as_ref().unwrap().clone_ref(py))
    })
}

// ============================================================================
// Wrapper Types for PyO3 IntoPyObject
// ============================================================================

/// Wrapper for Row to implement IntoPyObject
#[derive(Debug, Clone)]
struct RowWrapper {
    columns: Vec<(String, data_bridge_postgres::ExtractedValue)>,
}

impl<'py> IntoPyObject<'py> for RowWrapper {
    type Target = PyDict;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        for (column, value) in self.columns {
            let py_value = extracted_to_py_value(py, &value)?;
            dict.set_item(column, py_value)?;
        }
        Ok(dict)
    }
}

impl RowWrapper {
    fn from_row(row: &Row) -> PyResult<Self> {
        let mut columns = Vec::new();
        for column in row.columns() {
            if let Ok(value) = row.get(column) {
                columns.push((column.to_string(), value.clone()));
            }
        }
        Ok(Self { columns })
    }

    /// Create RowWrapper using pre-computed column names to avoid repeated allocations
    fn from_row_with_columns(row: &Row, column_names: &[String]) -> PyResult<Self> {
        let mut columns = Vec::with_capacity(column_names.len());
        for column in column_names {
            if let Ok(value) = row.get(column) {
                columns.push((column.clone(), value.clone()));
            }
        }
        Ok(Self { columns })
    }

    /// Create RowWrapper by taking ownership of Row (zero-copy)
    fn from_row_owned(row: Row, column_names: &[String]) -> PyResult<Self> {
        let mut columns = Vec::with_capacity(column_names.len());
        // Extract values in the order specified by column_names
        for column in column_names {
            if let Ok(value) = row.get(column) {
                // Still need to clone here because we're iterating and can't take ownership
                // But we'll optimize this next
                columns.push((column.clone(), value.clone()));
            }
        }
        Ok(Self { columns })
    }
}

/// Wrapper for optional Row
#[derive(Debug, Clone)]
struct OptionalRowWrapper(Option<RowWrapper>);

impl<'py> IntoPyObject<'py> for OptionalRowWrapper {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.0 {
            Some(row) => {
                let dict = row.into_pyobject(py)?;
                Ok(dict.into_any())
            }
            None => Ok(py.None().into_bound(py)),
        }
    }
}

/// Wrapper for multiple rows
#[derive(Debug, Clone)]
struct RowsWrapper(Vec<RowWrapper>);

impl<'py> IntoPyObject<'py> for RowsWrapper {
    type Target = PyList;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let list = PyList::empty(py);
        for row in self.0 {
            let dict = row.into_pyobject(py)?;
            list.append(dict)?;
        }
        Ok(list)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Gets the PostgreSQL connection pool or returns an error if not initialized.
fn get_connection() -> PyResult<Arc<Connection>> {
    PG_POOL
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to acquire pool lock: {}", e)))?
        .clone()
        .ok_or_else(|| PyRuntimeError::new_err("PostgreSQL connection not initialized. Call init() first."))
}

/// Converts Python dict to ExtractedValue for query parameters
fn py_dict_to_extracted_values(
    py: Python<'_>,
    dict: &Bound<'_, PyDict>,
) -> PyResult<Vec<(String, data_bridge_postgres::ExtractedValue)>> {
    let mut result = Vec::new();

    for (key, value) in dict.iter() {
        let key_str = key.extract::<String>()?;
        let extracted_value = py_value_to_extracted(py, &value)?;
        result.push((key_str, extracted_value));
    }

    Ok(result)
}

/// Converts Python value to ExtractedValue
fn py_value_to_extracted(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
) -> PyResult<data_bridge_postgres::ExtractedValue> {
    use data_bridge_postgres::ExtractedValue;

    if value.is_none() {
        Ok(ExtractedValue::Null)
    } else if let Ok(b) = value.extract::<bool>() {
        Ok(ExtractedValue::Bool(b))
    } else if let Ok(i) = value.extract::<i64>() {
        Ok(ExtractedValue::BigInt(i))
    } else if let Ok(i) = value.extract::<i32>() {
        Ok(ExtractedValue::Int(i))
    } else if let Ok(i) = value.extract::<i16>() {
        Ok(ExtractedValue::SmallInt(i))
    } else if let Ok(f) = value.extract::<f64>() {
        Ok(ExtractedValue::Double(f))
    } else if let Ok(f) = value.extract::<f32>() {
        Ok(ExtractedValue::Float(f))
    } else if let Ok(s) = value.extract::<String>() {
        Ok(ExtractedValue::String(s))
    } else if let Ok(bytes) = value.extract::<Vec<u8>>() {
        Ok(ExtractedValue::Bytes(bytes))
    } else if let Ok(list) = value.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(py_value_to_extracted(py, &item)?);
        }
        Ok(ExtractedValue::Array(vec))
    } else if let Ok(dict) = value.downcast::<PyDict>() {
        let values = py_dict_to_extracted_values(py, dict)?;
        Ok(ExtractedValue::Json(serde_json::json!(
            values.into_iter()
                .map(|(k, v)| (k, extracted_to_json(&v)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
        )))
    } else {
        Err(PyValueError::new_err(format!(
            "Unsupported Python type for PostgreSQL: {}",
            value.get_type().name()?
        )))
    }
}

/// Helper to convert ExtractedValue to JSON for nested structures
fn extracted_to_json(value: &data_bridge_postgres::ExtractedValue) -> serde_json::Value {
    use data_bridge_postgres::ExtractedValue;

    match value {
        ExtractedValue::Null => serde_json::Value::Null,
        ExtractedValue::Bool(b) => serde_json::Value::Bool(*b),
        ExtractedValue::SmallInt(i) => serde_json::Value::Number((*i).into()),
        ExtractedValue::Int(i) => serde_json::Value::Number((*i).into()),
        ExtractedValue::BigInt(i) => serde_json::Value::Number((*i).into()),
        ExtractedValue::Float(f) => serde_json::Number::from_f64(*f as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ExtractedValue::Double(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ExtractedValue::String(s) => serde_json::Value::String(s.clone()),
        ExtractedValue::Bytes(b) => serde_json::Value::String(general_purpose::STANDARD.encode(b)),
        ExtractedValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(extracted_to_json).collect())
        }
        ExtractedValue::Json(j) => j.clone(),
        ExtractedValue::Uuid(u) => serde_json::Value::String(u.to_string()),
        ExtractedValue::Date(d) => serde_json::Value::String(d.to_string()),
        ExtractedValue::Time(t) => serde_json::Value::String(t.to_string()),
        ExtractedValue::Timestamp(ts) => serde_json::Value::String(ts.to_string()),
        ExtractedValue::TimestampTz(ts) => serde_json::Value::String(ts.to_rfc3339()),
        ExtractedValue::Decimal(d) => serde_json::Value::String(d.clone()),
    }
}


/// Adjusts parameter placeholders in SQL to account for offset
/// Example: "age > $1 AND status = $2" with offset 3 becomes "age > $4 AND status = $5"
fn adjust_placeholders(sql: &str, offset: usize) -> String {
    use regex::Regex;
    let re = Regex::new(r"\$(\d+)").unwrap();
    re.replace_all(sql, |caps: &regex::Captures| {
        let num: usize = caps[1].parse().unwrap();
        format!("${}", num + offset)
    }).to_string()
}

/// Converts ExtractedValue back to Python object
fn extracted_to_py_value(py: Python<'_>, value: &data_bridge_postgres::ExtractedValue) -> PyResult<PyObject> {
    use data_bridge_postgres::ExtractedValue;

    Ok(match value {
        ExtractedValue::Null => py.None(),
        ExtractedValue::Bool(b) => b.to_object(py),
        ExtractedValue::SmallInt(i) => i.to_object(py),
        ExtractedValue::Int(i) => i.to_object(py),
        ExtractedValue::BigInt(i) => i.to_object(py),
        ExtractedValue::Float(f) => f.to_object(py),
        ExtractedValue::Double(f) => f.to_object(py),
        ExtractedValue::String(s) => s.to_object(py),
        ExtractedValue::Bytes(b) => b.to_object(py),
        ExtractedValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(extracted_to_py_value(py, item)?)?;
            }
            list.to_object(py)
        }
        ExtractedValue::Json(j) => pythonize::pythonize(py, j)?.into(),
        ExtractedValue::Uuid(u) => u.to_string().to_object(py),
        ExtractedValue::Date(d) => {
            let datetime = get_datetime_module(py)?;
            let date = datetime.bind(py).getattr("date")?;
            date.call_method1("fromisoformat", (d.to_string(),))?.to_object(py)
        }
        ExtractedValue::Time(t) => {
            let datetime = get_datetime_module(py)?;
            let time = datetime.bind(py).getattr("time")?;
            time.call_method1("fromisoformat", (t.to_string(),))?.to_object(py)
        }
        ExtractedValue::Timestamp(ts) => {
            // Convert NaiveDateTime to Python datetime (no timezone)
            let datetime = get_datetime_module(py)?;
            let dt = datetime.bind(py).getattr("datetime")?;
            dt.call_method1("fromisoformat", (ts.to_string(),))?.to_object(py)
        }
        ExtractedValue::TimestampTz(ts) => {
            // Convert to Python datetime with timezone
            let datetime = get_datetime_module(py)?;
            let dt = datetime.bind(py).getattr("datetime")?;
            dt.call_method1("fromisoformat", (ts.to_rfc3339(),))?.to_object(py)
        }
        ExtractedValue::Decimal(d) => {
            // Convert to Python Decimal
            let decimal_mod = py.import("decimal")?;
            let decimal_cls = decimal_mod.getattr("Decimal")?;
            decimal_cls.call1((d,))?.to_object(py)
        }
    })
}

// ============================================================================
// PyO3 Functions
// ============================================================================

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
fn init<'py>(
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
fn close<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
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
fn is_connected(_py: Python<'_>) -> PyResult<bool> {
    let pool = PG_POOL
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to acquire pool lock: {}", e)))?;

    Ok(pool.is_some())
}

/// Insert a single row into a table
///
/// Args:
///     table: Table name
///     data: Dictionary of column values
///     returning: Optional list of columns to return
///
/// Returns:
///     Dictionary with inserted row data (if returning is specified)
///
/// Example:
///     result = await insert_one("users", {"name": "Alice", "age": 30}, returning=["id"])
#[pyfunction]
#[pyo3(signature = (table, data))]
fn insert_one<'py>(
    py: Python<'py>,
    table: String,
    data: &Bound<'_, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let values = py_dict_to_extracted_values(py, data)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let row = Row::insert(conn.pool(), &table, &values)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Insert failed: {}", e)))?;

        // Phase 3: Convert result to Python (GIL acquired inside future_into_py)
        RowWrapper::from_row(&row)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))
    })
}

/// Insert multiple rows into a table
///
/// Args:
///     table: Table name
///     rows: List of dictionaries with column values
///
/// Returns:
///     List of dictionaries with inserted row data
///
/// Example:
///     results = await insert_many("users", [
///         {"name": "Alice", "age": 30},
///         {"name": "Bob", "age": 25}
///     ])
#[pyfunction]
#[pyo3(signature = (table, rows))]
fn insert_many<'py>(
    py: Python<'py>,
    table: String,
    rows: Vec<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract all rows (GIL held)
    let mut extracted_rows: Vec<HashMap<String, data_bridge_postgres::ExtractedValue>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let values = py_dict_to_extracted_values(py, row)?;
        // Convert Vec<(String, ExtractedValue)> to HashMap
        let map: HashMap<String, data_bridge_postgres::ExtractedValue> = values.into_iter().collect();
        extracted_rows.push(map);
    }

    // Phase 2: Execute batch INSERT (GIL released via future_into_py)
    future_into_py(py, async move {
        // Use Row::insert_many() batch method for better performance
        let batch_results = Row::insert_many(conn.pool(), &table, &extracted_rows)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Batch insert failed: {}", e)))?;

        // Phase 3: Convert results to Python (GIL acquired inside future_into_py)
        let result_rows: Vec<RowWrapper> = batch_results
            .iter()
            .map(RowWrapper::from_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RowsWrapper(result_rows))
    })
}

/// Fetch a single row from a table
///
/// Args:
///     table: Table name
///     filter: Dictionary of WHERE conditions
///     columns: Optional list of columns to select
///
/// Returns:
///     Dictionary with row data or None if not found
///
/// Example:
///     user = await fetch_one("users", {"id": 1}, columns=["name", "email"])
#[pyfunction]
#[pyo3(signature = (table, filter))]
fn fetch_one<'py>(
    py: Python<'py>,
    table: String,
    filter: &Bound<'_, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let filter_values = py_dict_to_extracted_values(py, filter)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Add WHERE conditions
        for (field, value) in filter_values {
            query = query.where_clause(&field, Operator::Eq, value)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid filter: {}", e)))?;
        }

        query = query.limit(1);

        // Build SQL and parameters
        let (sql, params) = query.build_select();

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .fetch_optional(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))?;

        // Phase 3: Convert result to Python (GIL acquired inside future_into_py)
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

/// Fetch multiple rows from a table
///
/// Args:
///     table: Table name
///     filter: Dictionary of WHERE conditions
///     columns: Optional list of columns to select
///     limit: Optional maximum number of rows to return
///     offset: Optional number of rows to skip
///     order_by: Optional list of (column, direction) tuples
///
/// Returns:
///     List of dictionaries with row data
///
/// Example:
///     users = await fetch_all("users", {"age": 30}, limit=10, order_by=[("name", "asc")])
#[pyfunction]
#[pyo3(signature = (table, filter, limit=None, offset=None, order_by=None))]
fn fetch_all<'py>(
    py: Python<'py>,
    table: String,
    filter: &Bound<'_, PyDict>,
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<Vec<(String, String)>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let filter_values = py_dict_to_extracted_values(py, filter)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Add WHERE conditions
        for (field, value) in filter_values {
            query = query.where_clause(&field, Operator::Eq, value)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid filter: {}", e)))?;
        }

        // Add ORDER BY
        if let Some(order_specs) = order_by {
            for (field, direction) in order_specs {
                let dir = if direction.to_lowercase() == "desc" {
                    OrderDirection::Desc
                } else {
                    OrderDirection::Asc
                };
                query = query.order_by(&field, dir)
                    .map_err(|e| PyRuntimeError::new_err(format!("Invalid order_by: {}", e)))?;
            }
        }

        // Add LIMIT
        if let Some(l) = limit {
            query = query.limit(l);
        }

        // Add OFFSET
        if let Some(o) = offset {
            query = query.offset(o);
        }

        // Build SQL and parameters
        let (sql, params) = query.build_select();

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))?;

        // Phase 3: Convert results to Python (GIL acquired inside future_into_py)
        if pg_rows.is_empty() {
            return Ok(RowsWrapper(Vec::new()));
        }

        // Convert to owned Rows first
        let rows: Vec<Row> = pg_rows.iter()
            .map(|pg_row| Row::from_sqlx(pg_row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;

        // Extract column names once from first row to avoid repeated allocations
        let column_names: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|col| col.to_string())
            .collect();

        let mut wrappers = Vec::with_capacity(rows.len());
        for row in rows {
            wrappers.push(RowWrapper::from_row_with_columns(&row, &column_names)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}

/// Update a single row in a table by primary key
///
/// Args:
///     table: Table name
///     pk_column: Primary key column name
///     pk_value: Primary key value
///     update: Dictionary of column values to update
///
/// Returns:
///     Primary key value of the updated row
///
/// Example:
///     result = await update_one("users", "id", 1, {"name": "Bob", "age": 35})
#[pyfunction]
#[pyo3(signature = (table, pk_column, pk_value, update))]
fn update_one<'py>(
    py: Python<'py>,
    table: String,
    pk_column: String,
    pk_value: &Bound<'_, PyAny>,
    update: &Bound<'_, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let pk_val = py_value_to_extracted(py, pk_value)?;
    let update_values = py_dict_to_extracted_values(py, update)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Add WHERE condition for primary key
        query = query.where_clause(&pk_column, Operator::Eq, pk_val.clone())
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid primary key: {}", e)))?;

        // Build UPDATE SQL
        let (sql, params) = query.build_update(&update_values)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to build UPDATE: {}", e)))?;

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .execute(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Update failed: {}", e)))?;

        // Phase 3: Return primary key value if update succeeded (GIL acquired inside future_into_py)
        if result.rows_affected() > 0 {
            Python::with_gil(|py| extracted_to_py_value(py, &pk_val))
        } else {
            Err(PyRuntimeError::new_err("Update failed: no rows affected"))
        }
    })
}

/// Delete a single row from a table by primary key
///
/// Args:
///     table: Table name
///     pk_column: Primary key column name
///     pk_value: Primary key value
///
/// Returns:
///     Number of rows deleted (0 or 1)
///
/// Example:
///     deleted = await delete_one("users", "id", 1)
#[pyfunction]
#[pyo3(signature = (table, pk_column, pk_value))]
fn delete_one<'py>(
    py: Python<'py>,
    table: String,
    pk_column: String,
    pk_value: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let pk_val = py_value_to_extracted(py, pk_value)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Add WHERE condition for primary key
        query = query.where_clause(&pk_column, Operator::Eq, pk_val)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid primary key: {}", e)))?;

        // Build DELETE SQL
        let (sql, params) = query.build_delete();

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .execute(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))?;

        // Phase 3: Return number of rows deleted (GIL acquired inside future_into_py)
        Ok(result.rows_affected() as i64)
    })
}

/// Update multiple rows matching WHERE clause
///
/// Args:
///     table: Table name
///     updates: Dictionary of column values to update
///     where_clause: SQL WHERE clause string (without "WHERE" keyword)
///     params: List of parameter values for WHERE clause
///
/// Returns:
///     Number of rows updated
///
/// Example:
///     updated = await update_many("users", {"status": "active"}, "age > $1", [25])
#[pyfunction]
#[pyo3(signature = (table, updates, where_clause, params))]
fn update_many<'py>(
    py: Python<'py>,
    table: String,
    updates: &Bound<'_, PyDict>,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python values (GIL held)
    let update_values = py_dict_to_extracted_values(py, updates)?;
    let where_params: Vec<data_bridge_postgres::ExtractedValue> = params
        .iter()
        .map(|param| py_value_to_extracted(py, param))
        .collect::<Result<Vec<_>, _>>()?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        // Build SET clause
        let set_clause: Vec<String> = update_values
            .iter()
            .enumerate()
            .map(|(i, (col, _))| format!("{} = ${}", col, i + 1))
            .collect();

        // Build UPDATE SQL
        let mut sql = format!("UPDATE {} SET {}", table, set_clause.join(", "));

        // Add WHERE clause if provided
        if !where_clause.is_empty() {
            // Adjust parameter placeholders in WHERE clause
            let placeholder_offset = update_values.len();
            let adjusted_where = adjust_placeholders(&where_clause, placeholder_offset);
            sql.push_str(&format!(" WHERE {}", adjusted_where));
        }

        // Bind parameters (updates first, then where params)
        let mut args = sqlx::postgres::PgArguments::default();
        for (_, value) in &update_values {
            value.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind update parameter: {}", e)))?;
        }
        for param in &where_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind where parameter: {}", e)))?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .execute(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Update failed: {}", e)))?;

        // Phase 3: Return result (GIL acquired inside future_into_py)
        Ok(result.rows_affected() as i64)
    })
}

/// Delete multiple rows matching WHERE clause
///
/// Args:
///     table: Table name
///     where_clause: SQL WHERE clause string (without "WHERE" keyword)
///     params: List of parameter values
///
/// Returns:
///     Number of rows deleted
///
/// Example:
///     deleted = await delete_many("users", "age < $1", [18])
#[pyfunction]
#[pyo3(signature = (table, where_clause, params))]
fn delete_many<'py>(
    py: Python<'py>,
    table: String,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python parameter values (GIL held)
    let extracted_params: Vec<data_bridge_postgres::ExtractedValue> = params
        .iter()
        .map(|param| py_value_to_extracted(py, param))
        .collect::<Result<Vec<_>, _>>()?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        // Build DELETE query
        let mut sql = format!("DELETE FROM {}", table);

        // Add WHERE clause if provided
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &extracted_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .execute(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))?;

        // Phase 3: Return result (GIL acquired inside future_into_py)
        Ok(result.rows_affected() as i64)
    })
}

/// Count rows matching WHERE clause
///
/// Args:
///     table: Table name
///     where_clause: SQL WHERE clause string (without "WHERE" keyword)
///     params: List of parameter values
///
/// Returns:
///     int: Number of matching rows
///
/// Example:
///     count = await count("users", "age > $1", [25])
#[pyfunction]
#[pyo3(signature = (table, where_clause, params))]
fn count<'py>(
    py: Python<'py>,
    table: String,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python parameter values (GIL held)
    let extracted_params: Vec<data_bridge_postgres::ExtractedValue> = params
        .iter()
        .map(|param| py_value_to_extracted(py, param))
        .collect::<Result<Vec<_>, _>>()?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        // Build COUNT(*) query
        let mut sql = format!("SELECT COUNT(*) FROM {}", table);

        // Add WHERE clause if provided
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &extracted_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let row = sqlx::query_with(&sql, args)
            .fetch_one(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Count query failed: {}", e)))?;

        // Extract count value
        let count: i64 = row.try_get(0)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to extract count: {}", e)))?;

        // Phase 3: Return result (GIL acquired inside future_into_py)
        Ok(count)
    })
}

/// Execute raw SQL query
///
/// Args:
///     sql: SQL query string
///     params: Optional dictionary of named parameters
///
/// Returns:
///     List of dictionaries with row data
///
/// Example:
///     results = await execute("SELECT * FROM users WHERE age > :age", {"age": 25})
#[pyfunction]
#[pyo3(signature = (_sql, _params=None))]
fn execute<'py>(
    py: Python<'py>,
    _sql: String,
    _params: Option<&Bound<'_, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let _conn = get_connection()?;

    future_into_py(py, async move {
        // TODO: Implement raw SQL execution
        // This requires extending the QueryBuilder or Connection to support raw SQL
        Python::with_gil(|_py| {
            Err::<PyObject, PyErr>(PyRuntimeError::new_err("Raw SQL execution not yet implemented"))
        })
    })
}

/// Begin a transaction
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
#[pyfunction]
fn begin_transaction<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let _conn = get_connection()?;

    future_into_py(py, async move {
        // TODO: Implement transaction support
        // This requires wrapping the Transaction type from data-bridge-postgres
        Python::with_gil(|_py| {
            Err::<PyObject, PyErr>(PyRuntimeError::new_err("Transactions not yet implemented"))
        })
    })
}

/// Find multiple rows with advanced query options (alias for fetch_all with WHERE clause support)
///
/// Args:
///     table: Table name
///     where_clause: SQL WHERE clause string (without "WHERE" keyword)
///     params: List of parameter values
///     order_by: Optional list of (column, direction) tuples
///     offset: Optional number of rows to skip
///     limit: Optional maximum number of rows to return
///     select_cols: Optional list of column names to select
///
/// Returns:
///     List of dictionaries with row data
///
/// Example:
///     rows = await find_many(
///         "users",
///         "age > $1 AND status = $2",
///         [25, "active"],
///         order_by=[("name", "ASC")],
///         limit=10
///     )
#[pyfunction]
#[pyo3(signature = (table, where_clause, params, order_by=None, offset=None, limit=None, select_cols=None))]
fn find_many<'py>(
    py: Python<'py>,
    table: String,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
    order_by: Option<Vec<(String, String)>>,
    offset: Option<i64>,
    limit: Option<i64>,
    select_cols: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python parameter values (GIL held)
    let extracted_params: Vec<data_bridge_postgres::ExtractedValue> = params
        .iter()
        .map(|param| py_value_to_extracted(py, param))
        .collect::<Result<Vec<_>, _>>()?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        // Build SELECT clause
        let select_clause = if let Some(cols) = select_cols {
            cols.join(", ")
        } else {
            "*".to_string()
        };

        // Start building SQL
        let mut sql = format!("SELECT {} FROM {}", select_clause, table);

        // Add WHERE clause if provided
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        // Add ORDER BY
        if let Some(order_specs) = order_by {
            if !order_specs.is_empty() {
                sql.push_str(" ORDER BY ");
                let order_clauses: Vec<String> = order_specs
                    .iter()
                    .map(|(col, dir)| format!("{} {}", col, dir))
                    .collect();
                sql.push_str(&order_clauses.join(", "));
            }
        }

        // Add LIMIT
        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {}", l));
        }

        // Add OFFSET
        if let Some(o) = offset {
            sql.push_str(&format!(" OFFSET {}", o));
        }

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &extracted_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))?;

        // Phase 3: Convert results to Python (GIL acquired inside future_into_py)
        if pg_rows.is_empty() {
            return Ok(RowsWrapper(Vec::new()));
        }

        // Convert to owned Rows first
        let rows: Vec<Row> = pg_rows.iter()
            .map(|pg_row| Row::from_sqlx(pg_row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;

        // Extract column names once from first row to avoid repeated allocations
        let column_names: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|col| col.to_string())
            .collect();

        let mut wrappers = Vec::with_capacity(rows.len());
        for row in rows {
            wrappers.push(RowWrapper::from_row_with_columns(&row, &column_names)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register PostgreSQL module functions with Python
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(close, m)?)?;
    m.add_function(wrap_pyfunction!(is_connected, m)?)?;
    m.add_function(wrap_pyfunction!(insert_one, m)?)?;
    m.add_function(wrap_pyfunction!(insert_many, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_one, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_all, m)?)?;
    m.add_function(wrap_pyfunction!(find_many, m)?)?;
    m.add_function(wrap_pyfunction!(update_one, m)?)?;
    m.add_function(wrap_pyfunction!(update_many, m)?)?;
    m.add_function(wrap_pyfunction!(delete_one, m)?)?;
    m.add_function(wrap_pyfunction!(delete_many, m)?)?;
    m.add_function(wrap_pyfunction!(count, m)?)?;
    m.add_function(wrap_pyfunction!(execute, m)?)?;
    m.add_function(wrap_pyfunction!(begin_transaction, m)?)?;

    // Add module docstring
    m.add("__doc__", "PostgreSQL ORM module with async support")?;

    Ok(())
}
