//! PostgreSQL module for Python bindings
//!
//! This module provides Python bindings for PostgreSQL operations using PyO3.
//! All SQL serialization/deserialization happens in Rust for maximum performance.

use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use std::sync::Arc;

use data_bridge_postgres::{Connection, PoolConfig, QueryBuilder, Operator, OrderDirection, Row, Transaction, transaction::IsolationLevel, SchemaInspector};
use sqlx::Row as SqlxRow;

// For base64 encoding of binary data
use base64::{Engine as _, engine::general_purpose};

// Global connection pool using RwLock for close/reset support
use std::sync::RwLock as StdRwLock;
static PG_POOL: StdRwLock<Option<Arc<Connection>>> = StdRwLock::new(None);

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
///
/// Optimized to check Python type name first, avoiding sequential type extractions.
/// This reduces overhead by jumping directly to the correct type extraction.
fn py_value_to_extracted(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
) -> PyResult<data_bridge_postgres::ExtractedValue> {
    use data_bridge_postgres::ExtractedValue;

    // Fast path: check None first (very common)
    if value.is_none() {
        return Ok(ExtractedValue::Null);
    }

    // Get Python type name once - much faster than multiple failed extractions
    let type_name = value.get_type().name()?;

    match type_name.to_cow()?.as_ref() {
        // Most common types first
        "int" => {
            // Try i64 first (most common), then i32, then i16
            if let Ok(i) = value.extract::<i64>() {
                Ok(ExtractedValue::BigInt(i))
            } else if let Ok(i) = value.extract::<i32>() {
                Ok(ExtractedValue::Int(i))
            } else if let Ok(i) = value.extract::<i16>() {
                Ok(ExtractedValue::SmallInt(i))
            } else {
                Err(PyTypeError::new_err("Integer out of range"))
            }
        }
        "str" => {
            let s = value.extract::<String>()?;
            Ok(ExtractedValue::String(s))
        }
        "bool" => {
            let b = value.extract::<bool>()?;
            Ok(ExtractedValue::Bool(b))
        }
        "float" => {
            // Try f64 first (more common), then f32
            if let Ok(f) = value.extract::<f64>() {
                Ok(ExtractedValue::Double(f))
            } else {
                let f = value.extract::<f32>()?;
                Ok(ExtractedValue::Float(f))
            }
        }
        "bytes" | "bytearray" => {
            let bytes = value.extract::<Vec<u8>>()?;
            Ok(ExtractedValue::Bytes(bytes))
        }
        "list" | "tuple" => {
            // Handle both lists and tuples by converting to list
            let list = if let Ok(l) = value.downcast::<PyList>() {
                l.clone()
            } else if let Ok(t) = value.downcast::<PyTuple>() {
                t.to_list()
            } else {
                return Err(PyTypeError::new_err("Expected list or tuple"));
            };

            let mut vec = Vec::with_capacity(list.len());
            for item in list.iter() {
                vec.push(py_value_to_extracted(py, &item)?);
            }
            Ok(ExtractedValue::Array(vec))
        }
        "dict" => {
            let dict = value.downcast::<PyDict>()?;
            let values = py_dict_to_extracted_values(py, dict)?;
            Ok(ExtractedValue::Json(serde_json::json!(
                values.into_iter()
                    .map(|(k, v)| (k, extracted_to_json(&v)))
                    .collect::<serde_json::Map<String, serde_json::Value>>()
            )))
        }
        "NoneType" => Ok(ExtractedValue::Null),
        "datetime" => {
            // Handle datetime by converting to string representation
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        "date" => {
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        "UUID" | "uuid" => {
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        "Decimal" => {
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        _ => {
            // Fallback: try common extractions for custom types
            if let Ok(s) = value.extract::<String>() {
                Ok(ExtractedValue::String(s))
            } else if let Ok(i) = value.extract::<i64>() {
                Ok(ExtractedValue::BigInt(i))
            } else if let Ok(f) = value.extract::<f64>() {
                Ok(ExtractedValue::Double(f))
            } else {
                // Last resort: convert to string representation
                let s = value.str()?.to_string();
                Ok(ExtractedValue::String(s))
            }
        }
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


/// Static regex for adjusting placeholders (compiled once)
static PLACEHOLDER_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"\$(\d+)").expect("placeholder regex is valid")
});

/// Adjusts parameter placeholders in SQL to account for offset
/// Example: "age > $1 AND status = $2" with offset 3 becomes "age > $4 AND status = $5"
/// Returns error if the SQL contains malformed placeholders
fn adjust_placeholders(sql: &str, offset: usize) -> Result<String, String> {
    let mut last_error = None;
    let result = PLACEHOLDER_RE.replace_all(sql, |caps: &regex::Captures| {
        match caps[1].parse::<usize>() {
            Ok(num) => format!("${}", num + offset),
            Err(e) => {
                last_error = Some(format!("Invalid placeholder number '{}': {}", &caps[1], e));
                caps[0].to_string() // Return original on error
            }
        }
    }).to_string();

    if let Some(err) = last_error {
        Err(err)
    } else {
        Ok(result)
    }
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
            let datetime = py.import("datetime")?;
            let date = datetime.getattr("date")?;
            date.call_method1("fromisoformat", (d.to_string(),))?.to_object(py)
        }
        ExtractedValue::Time(t) => {
            let datetime = py.import("datetime")?;
            let time = datetime.getattr("time")?;
            time.call_method1("fromisoformat", (t.to_string(),))?.to_object(py)
        }
        ExtractedValue::Timestamp(ts) => {
            // Convert NaiveDateTime to Python datetime (no timezone)
            let datetime = py.import("datetime")?;
            let dt = datetime.getattr("datetime")?;
            dt.call_method1("fromisoformat", (ts.to_string(),))?.to_object(py)
        }
        ExtractedValue::TimestampTz(ts) => {
            // Convert to Python datetime with timezone
            let datetime = py.import("datetime")?;
            let dt = datetime.getattr("datetime")?;
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

/// Upsert a single row (INSERT ON CONFLICT UPDATE)
///
/// Args:
///     table: Table name
///     data: Dictionary of column values
///     conflict_target: List of column(s) for ON CONFLICT
///     update_columns: Optional list of columns to update (None = all except conflict_target)
///
/// Returns:
///     Dictionary with upserted row data
///
/// Example:
///     result = await upsert_one("users",
///         {"email": "alice@example.com", "name": "Alice", "age": 30},
///         ["email"],
///         update_columns=["name", "age"]
///     )
#[pyfunction]
#[pyo3(signature = (table, data, conflict_target, update_columns=None))]
fn upsert_one<'py>(
    py: Python<'py>,
    table: String,
    data: &Bound<'_, PyDict>,
    conflict_target: Vec<String>,
    update_columns: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let values = py_dict_to_extracted_values(py, data)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let row = Row::upsert(
            conn.pool(),
            &table,
            &values,
            &conflict_target,
            update_columns.as_deref()
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Upsert failed: {}", e)))?;

        // Phase 3: Convert result to Python (GIL acquired inside future_into_py)
        RowWrapper::from_row(&row)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))
    })
}

/// Upsert multiple rows with a single batch statement
///
/// Args:
///     table: Table name
///     rows: List of dictionaries with column values
///     conflict_target: List of column(s) for ON CONFLICT
///     update_columns: Optional list of columns to update (None = all except conflict_target)
///
/// Returns:
///     List of dictionaries with upserted row data
///
/// Example:
///     results = await upsert_many("users", [
///         {"email": "alice@example.com", "name": "Alice"},
///         {"email": "bob@example.com", "name": "Bob"}
///     ], ["email"])
#[pyfunction]
#[pyo3(signature = (table, rows, conflict_target, update_columns=None))]
fn upsert_many<'py>(
    py: Python<'py>,
    table: String,
    rows: Vec<Bound<'py, PyDict>>,
    conflict_target: Vec<String>,
    update_columns: Option<Vec<String>>,
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

    // Phase 2: Execute batch UPSERT (GIL released via future_into_py)
    future_into_py(py, async move {
        // Use Row::upsert_many() batch method for better performance
        let batch_results = Row::upsert_many(
            conn.pool(),
            &table,
            &extracted_rows,
            &conflict_target,
            update_columns.as_deref()
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Batch upsert failed: {}", e)))?;

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
        let mut wrappers = Vec::with_capacity(pg_rows.len());
        for pg_row in &pg_rows {
            let row = Row::from_sqlx(pg_row)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;
            wrappers.push(RowWrapper::from_row(&row)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}

/// Fetch a single row with related data using JOINs (eager loading)
///
/// Args:
///     table: Table name
///     id: Primary key value
///     relations: List of relation configurations (dictionaries)
///
/// Returns:
///     Dictionary with row data including nested relations, or None if not found
///
/// Example:
///     user = await fetch_one_with_relations("users", 1, [
///         {
///             "name": "posts",
///             "table": "posts",
///             "foreign_key": "user_id",
///             "reference_column": "id",
///             "join_type": "left",
///             "select_columns": None
///         }
///     ])
#[pyfunction]
#[pyo3(signature = (table, id, relations))]
fn fetch_one_with_relations<'py>(
    py: Python<'py>,
    table: String,
    id: i64,
    relations: Vec<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    use data_bridge_postgres::row::{RelationConfig, Row};
    use data_bridge_postgres::query::JoinType;

    let conn = get_connection()?;

    // Parse relations from Python dicts
    let mut relation_configs: Vec<RelationConfig> = Vec::new();
    for rel_dict in &relations {
        let name: String = rel_dict.get_item("name")?
            .ok_or_else(|| PyValueError::new_err("Missing 'name' in relation"))?
            .extract()?;

        let rel_table: String = rel_dict.get_item("table")?
            .ok_or_else(|| PyValueError::new_err("Missing 'table' in relation"))?
            .extract()?;

        let foreign_key: String = rel_dict.get_item("foreign_key")?
            .ok_or_else(|| PyValueError::new_err("Missing 'foreign_key' in relation"))?
            .extract()?;

        let reference_column: String = rel_dict.get_item("reference_column")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or_else(|| "id".to_string());

        let join_type_str: String = rel_dict.get_item("join_type")?
            .map(|v| v.extract())
            .transpose()?
            .unwrap_or_else(|| "left".to_string());

        let join_type = match join_type_str.to_lowercase().as_str() {
            "inner" => JoinType::Inner,
            "left" => JoinType::Left,
            "right" => JoinType::Right,
            "full" => JoinType::Full,
            _ => JoinType::Left,
        };

        let select_columns: Option<Vec<String>> = rel_dict.get_item("select_columns")?
            .and_then(|v| if v.is_none() { None } else { Some(v) })
            .map(|v| v.extract())
            .transpose()?;

        relation_configs.push(RelationConfig {
            name,
            table: rel_table,
            foreign_key,
            reference_column,
            join_type,
            select_columns,
        });
    }

    future_into_py(py, async move {
        let result = Row::find_with_relations(
            conn.pool(),
            &table,
            id,
            &relation_configs,
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Fetch with relations failed: {}", e)))?;

        match result {
            Some(row) => {
                RowWrapper::from_row(&row)
                    .map(|r| Some(r))
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))
            }
            None => Ok(None),
        }
    })
}

/// Simple eager loading - fetch one row with related data
///
/// Args:
///     table: Table name
///     id: Primary key value
///     joins: List of (relation_name, fk_column, ref_table) tuples
///
/// Returns:
///     Dictionary with row data including nested relations, or None if not found
///
/// Example:
///     user = await fetch_one_eager("users", 1, [
///         ("posts", "user_id", "posts"),
///         ("profile", "user_id", "profiles")
///     ])
#[pyfunction]
#[pyo3(signature = (table, id, joins))]
fn fetch_one_eager<'py>(
    py: Python<'py>,
    table: String,
    id: i64,
    joins: Vec<(String, String, String)>,
) -> PyResult<Bound<'py, PyAny>> {
    use data_bridge_postgres::row::Row;

    let conn = get_connection()?;

    future_into_py(py, async move {
        // Convert tuples to borrowed slices inside the async block
        let join_refs: Vec<(&str, &str, &str)> = joins
            .iter()
            .map(|(name, fk, ref_table)| (name.as_str(), fk.as_str(), ref_table.as_str()))
            .collect();

        let result = Row::find_one_eager(
            conn.pool(),
            &table,
            id,
            &join_refs,
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Eager fetch failed: {}", e)))?;

        match result {
            Some(row) => {
                RowWrapper::from_row(&row)
                    .map(|r| Some(r))
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))
            }
            None => Ok(None),
        }
    })
}

/// Fetch multiple rows with related data using JOINs
///
/// Args:
///     table: Table name
///     relations: List of relation configurations (dictionaries)
///     filter: Optional dictionary of WHERE conditions
///     order_by: Optional (column, direction) tuple
///     limit: Optional maximum number of rows to return
///     offset: Optional number of rows to skip
///
/// Returns:
///     List of dictionaries with row data including nested relations
///
/// Example:
///     users = await fetch_many_with_relations("users", [
///         {
///             "name": "posts",
///             "table": "posts",
///             "foreign_key": "user_id",
///             "reference_column": "id",
///             "join_type": "left"
///         }
///     ], filter={"age": 30}, limit=10)
#[pyfunction]
#[pyo3(signature = (table, relations, filter=None, order_by=None, limit=None, offset=None))]
fn fetch_many_with_relations<'py>(
    py: Python<'py>,
    table: String,
    relations: Vec<Bound<'py, PyDict>>,
    filter: Option<Bound<'py, PyDict>>,
    order_by: Option<(String, String)>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> PyResult<Bound<'py, PyAny>> {
    use data_bridge_postgres::row::{RelationConfig, Row};
    use data_bridge_postgres::query::{JoinType, Operator, OrderDirection};
    use data_bridge_postgres::ExtractedValue;

    let conn = get_connection()?;

    // Parse relations
    let mut relation_configs: Vec<RelationConfig> = Vec::new();
    for rel_dict in &relations {
        let name: String = rel_dict.get_item("name")?
            .ok_or_else(|| PyValueError::new_err("Missing 'name'"))?
            .extract()?;
        let rel_table: String = rel_dict.get_item("table")?
            .ok_or_else(|| PyValueError::new_err("Missing 'table'"))?
            .extract()?;
        let foreign_key: String = rel_dict.get_item("foreign_key")?
            .ok_or_else(|| PyValueError::new_err("Missing 'foreign_key'"))?
            .extract()?;
        let reference_column: String = rel_dict.get_item("reference_column")?
            .map(|v| v.extract()).transpose()?.unwrap_or_else(|| "id".to_string());
        let join_type_str: String = rel_dict.get_item("join_type")?
            .map(|v| v.extract()).transpose()?.unwrap_or_else(|| "left".to_string());
        let join_type = match join_type_str.to_lowercase().as_str() {
            "inner" => JoinType::Inner,
            "right" => JoinType::Right,
            "full" => JoinType::Full,
            _ => JoinType::Left,
        };

        relation_configs.push(RelationConfig {
            name,
            table: rel_table,
            foreign_key,
            reference_column,
            join_type,
            select_columns: None,
        });
    }

    // Parse filter if provided
    let where_clause: Option<(String, Operator, ExtractedValue)> = match filter {
        Some(ref f) => {
            if f.len() > 0 {
                let items: Vec<_> = f.iter().collect();
                if let Some((key, value)) = items.first() {
                    let col: String = key.extract()?;
                    let val = py_value_to_extracted(py, value)?;
                    Some((col, Operator::Eq, val))
                } else {
                    None
                }
            } else {
                None
            }
        }
        None => None,
    };

    // Parse order_by
    let order: Option<(String, OrderDirection)> = order_by.map(|(col, dir)| {
        let direction = match dir.to_lowercase().as_str() {
            "desc" => OrderDirection::Desc,
            _ => OrderDirection::Asc,
        };
        (col, direction)
    });

    future_into_py(py, async move {
        let results = Row::find_many_with_relations(
            conn.pool(),
            &table,
            &relation_configs,
            where_clause.as_ref().map(|(c, o, v)| (c.as_str(), o.clone(), v.clone())),
            order.as_ref().map(|(c, d)| (c.as_str(), d.clone())),
            limit,
            offset,
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Fetch many with relations failed: {}", e)))?;

        let rows: Vec<RowWrapper> = results
            .iter()
            .map(RowWrapper::from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert rows: {}", e)))?;

        Ok(RowsWrapper(rows))
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
            let adjusted_where = adjust_placeholders(&where_clause, placeholder_offset)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid WHERE clause placeholders: {}", e)))?;
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

/// Delete a row with cascade handling based on foreign key rules
///
/// This manually handles ON DELETE rules:
/// - CASCADE: Deletes child rows first (recursively)
/// - RESTRICT: Returns error if children exist
/// - SET NULL: Sets FK to NULL before delete
/// - SET DEFAULT: Sets FK to DEFAULT before delete
///
/// Args:
///     table: Table name to delete from
///     id: Primary key value of row to delete
///     id_column: Name of primary key column (default: "id")
///
/// Returns:
///     Total number of rows deleted (including cascaded children)
///
/// Example:
///     deleted = await delete_with_cascade("users", 1, "id")
#[pyfunction]
#[pyo3(signature = (table, id, id_column=None))]
fn delete_with_cascade<'py>(
    py: Python<'py>,
    table: String,
    id: i64,
    id_column: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let id_col = id_column.unwrap_or_else(|| "id".to_string());

    future_into_py(py, async move {
        let deleted = Row::delete_with_cascade(conn.pool(), &table, id, &id_col)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(deleted)
    })
}

/// Delete a row after checking RESTRICT constraints
///
/// Checks for RESTRICT/NO ACTION constraints and returns an error
/// if children exist. For CASCADE, relies on database-level handling.
///
/// Args:
///     table: Table name to delete from
///     id: Primary key value of row to delete
///     id_column: Name of primary key column (default: "id")
///
/// Returns:
///     Number of rows deleted (1 if success, 0 if not found)
///
/// Example:
///     deleted = await delete_checked("users", 1, "id")
#[pyfunction]
#[pyo3(signature = (table, id, id_column=None))]
fn delete_checked<'py>(
    py: Python<'py>,
    table: String,
    id: i64,
    id_column: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let id_col = id_column.unwrap_or_else(|| "id".to_string());

    future_into_py(py, async move {
        let deleted = Row::delete_checked(conn.pool(), &table, id, &id_col)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(deleted)
    })
}

/// Get all tables that reference a given table (back-references)
///
/// Useful for understanding relationships before delete operations.
///
/// Args:
///     table: Table name to find references to
///     schema: Schema name (default: "public")
///
/// Returns:
///     List of dicts with keys:
///     - source_table: Table that references this table
///     - source_column: FK column in source table
///     - target_table: This table
///     - target_column: Referenced column (usually "id")
///     - constraint_name: Name of FK constraint
///     - on_delete: DELETE rule ("CASCADE", "RESTRICT", etc.)
///     - on_update: UPDATE rule
///
/// Example:
///     backrefs = await get_backreferences("users", "public")
#[pyfunction]
#[pyo3(signature = (table, schema=None))]
fn get_backreferences<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = SchemaInspector::new((*conn).clone());
        let backrefs = inspector.get_backreferences(&table, schema.as_deref())
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        // Convert to Python list of dicts
        Python::with_gil(|py| {
            let result = PyList::empty(py);

            for br in backrefs {
                let dict = PyDict::new(py);
                dict.set_item("source_table", br.source_table)?;
                dict.set_item("source_column", br.source_column)?;
                dict.set_item("target_table", br.target_table)?;
                dict.set_item("target_column", br.target_column)?;
                dict.set_item("constraint_name", br.constraint_name)?;
                dict.set_item("on_delete", br.on_delete.to_sql())?;
                dict.set_item("on_update", br.on_update.to_sql())?;
                result.append(dict)?;
            }

            Ok(result.into_any().unbind())
        })
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
fn execute<'py>(
    py: Python<'py>,
    sql: String,
    params: Option<Vec<Bound<'py, PyAny>>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Extract parameters from Python to Rust (while holding GIL)
    let extracted_params: Vec<data_bridge_postgres::ExtractedValue> = if let Some(param_list) = params {
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
        let is_select = sql_upper.starts_with("SELECT") || sql_upper.starts_with("WITH");
        let is_dml = sql_upper.starts_with("INSERT")
            || sql_upper.starts_with("UPDATE")
            || sql_upper.starts_with("DELETE");

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
                    let columns = data_bridge_postgres::row_to_extracted(&row)
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

// ============================================================================
// Transaction Support
// ============================================================================

/// Python wrapper for PostgreSQL transaction
#[pyclass]
struct PyTransaction {
    tx: Option<Transaction>,
}

#[pymethods]
impl PyTransaction {
    /// Commit the transaction
    ///
    /// Returns:
    ///     None
    ///
    /// Example:
    ///     await tx.commit()
    fn commit<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let tx = self.tx.take()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

        future_into_py(py, async move {
            tx.commit().await
                .map_err(|e| PyRuntimeError::new_err(format!("Commit failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
        })
    }

    /// Rollback the transaction
    ///
    /// Returns:
    ///     None
    ///
    /// Example:
    ///     await tx.rollback()
    fn rollback<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let tx = self.tx.take()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already completed"))?;

        future_into_py(py, async move {
            tx.rollback().await
                .map_err(|e| PyRuntimeError::new_err(format!("Rollback failed: {}", e)))?;
            Python::with_gil(|py| Ok(py.None()))
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
fn begin_transaction<'py>(
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
            Ok(PyTransaction { tx: Some(tx) }.into_py(py))
        })
    })
}

// ============================================================================
// Schema Introspection
// ============================================================================

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
fn list_tables<'py>(
    py: Python<'py>,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = data_bridge_postgres::SchemaInspector::new((*conn).clone());
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
fn table_exists<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = data_bridge_postgres::SchemaInspector::new((*conn).clone());
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
fn get_columns<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = data_bridge_postgres::SchemaInspector::new((*conn).clone());
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
fn get_indexes<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = data_bridge_postgres::SchemaInspector::new((*conn).clone());
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
fn get_foreign_keys<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = data_bridge_postgres::SchemaInspector::new((*conn).clone());
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
fn inspect_table<'py>(
    py: Python<'py>,
    table: String,
    schema: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let inspector = data_bridge_postgres::SchemaInspector::new((*conn).clone());
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

// ============================================================================
// Query Builder
// ============================================================================

/// Find a single row by foreign key value
///
/// Args:
///     table: Table name
///     foreign_key_column: Column name to query
///     foreign_key_value: Value to match
///
/// Returns:
///     Dictionary with row data or None if not found
///
/// Example:
///     user = await find_by_foreign_key("users", "id", 123)
#[pyfunction]
fn find_by_foreign_key<'py>(
    py: Python<'py>,
    table: String,
    foreign_key_column: String,
    foreign_key_value: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python value (GIL held)
    let fk_val = py_value_to_extracted(py, foreign_key_value)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Add WHERE condition for foreign key
        query = query.where_clause(&foreign_key_column, Operator::Eq, fk_val)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid foreign key: {}", e)))?;

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
        let mut wrappers = Vec::with_capacity(pg_rows.len());
        for pg_row in &pg_rows {
            let row = Row::from_sqlx(pg_row)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;
            wrappers.push(RowWrapper::from_row(&row)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}

// ============================================================================
// Migration Support
// ============================================================================

/// Initialize migration system (create _migrations table)
///
/// Returns:
///     Awaitable that resolves when migration table is created
///
/// Example:
///     await migration_init()
#[pyfunction]
fn migration_init<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = data_bridge_postgres::MigrationRunner::new((*conn).clone(), None);
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
fn migration_status<'py>(
    py: Python<'py>,
    migrations_dir: String,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = data_bridge_postgres::MigrationRunner::new((*conn).clone(), None);

        let migrations = data_bridge_postgres::MigrationRunner::load_from_directory(std::path::Path::new(&migrations_dir))
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
fn migration_apply<'py>(
    py: Python<'py>,
    migrations_dir: String,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = data_bridge_postgres::MigrationRunner::new((*conn).clone(), None);

        let migrations = data_bridge_postgres::MigrationRunner::load_from_directory(std::path::Path::new(&migrations_dir))
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
fn migration_rollback<'py>(
    py: Python<'py>,
    migrations_dir: String,
    steps: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    future_into_py(py, async move {
        let runner = data_bridge_postgres::MigrationRunner::new((*conn).clone(), None);

        let migrations = data_bridge_postgres::MigrationRunner::load_from_directory(std::path::Path::new(&migrations_dir))
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
fn migration_create(
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
fn autogenerate_migration<'py>(
    py: Python<'py>,
    current_tables: Vec<Bound<'py, PyDict>>,
    desired_tables: Vec<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyDict>> {
    use data_bridge_postgres::schema::{SchemaDiff, TableInfo, ColumnInfo, ColumnType, IndexInfo, ForeignKeyInfo};

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
        let columns_list = columns_list.downcast::<pyo3::types::PyList>()?;

        let mut columns = Vec::new();
        for col_item in columns_list.iter() {
            let col_dict = col_item.downcast::<PyDict>()?;
            columns.push(dict_to_column_info(col_dict)?);
        }

        // Parse indexes (optional)
        let indexes = match dict.get_item("indexes")? {
            Some(idx_list) => {
                let idx_list = idx_list.downcast::<pyo3::types::PyList>()?;
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
                let fk_list = fk_list.downcast::<pyo3::types::PyList>()?;
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

// ============================================================================
// Module Registration
// ============================================================================

/// Register PostgreSQL module functions with Python
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register classes
    m.add_class::<PyTransaction>()?;

    // Register functions
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(close, m)?)?;
    m.add_function(wrap_pyfunction!(is_connected, m)?)?;
    m.add_function(wrap_pyfunction!(insert_one, m)?)?;
    m.add_function(wrap_pyfunction!(insert_many, m)?)?;
    m.add_function(wrap_pyfunction!(upsert_one, m)?)?;
    m.add_function(wrap_pyfunction!(upsert_many, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_one, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_all, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_one_with_relations, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_one_eager, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_many_with_relations, m)?)?;
    m.add_function(wrap_pyfunction!(find_many, m)?)?;
    m.add_function(wrap_pyfunction!(update_one, m)?)?;
    m.add_function(wrap_pyfunction!(update_many, m)?)?;
    m.add_function(wrap_pyfunction!(delete_one, m)?)?;
    m.add_function(wrap_pyfunction!(delete_many, m)?)?;
    m.add_function(wrap_pyfunction!(delete_with_cascade, m)?)?;
    m.add_function(wrap_pyfunction!(delete_checked, m)?)?;
    m.add_function(wrap_pyfunction!(count, m)?)?;
    m.add_function(wrap_pyfunction!(execute, m)?)?;
    m.add_function(wrap_pyfunction!(begin_transaction, m)?)?;

    // Schema introspection functions
    m.add_function(wrap_pyfunction!(list_tables, m)?)?;
    m.add_function(wrap_pyfunction!(table_exists, m)?)?;
    m.add_function(wrap_pyfunction!(get_columns, m)?)?;
    m.add_function(wrap_pyfunction!(get_indexes, m)?)?;
    m.add_function(wrap_pyfunction!(get_foreign_keys, m)?)?;
    m.add_function(wrap_pyfunction!(get_backreferences, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_table, m)?)?;
    m.add_function(wrap_pyfunction!(find_by_foreign_key, m)?)?;

    // Migration functions
    m.add_function(wrap_pyfunction!(migration_init, m)?)?;
    m.add_function(wrap_pyfunction!(migration_status, m)?)?;
    m.add_function(wrap_pyfunction!(migration_apply, m)?)?;
    m.add_function(wrap_pyfunction!(migration_rollback, m)?)?;
    m.add_function(wrap_pyfunction!(migration_create, m)?)?;
    m.add_function(wrap_pyfunction!(autogenerate_migration, m)?)?;

    // Add module docstring
    m.add("__doc__", "PostgreSQL ORM module with async support")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_placeholders_valid() {
        // Test basic placeholder adjustment
        let sql = "age > $1 AND status = $2";
        let result = adjust_placeholders(sql, 3);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "age > $4 AND status = $5");
    }

    #[test]
    fn test_adjust_placeholders_no_placeholders() {
        // Test SQL with no placeholders
        let sql = "SELECT * FROM users";
        let result = adjust_placeholders(sql, 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "SELECT * FROM users");
    }

    #[test]
    fn test_adjust_placeholders_zero_offset() {
        // Test with zero offset (no adjustment)
        let sql = "price < $1";
        let result = adjust_placeholders(sql, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "price < $1");
    }

    #[test]
    fn test_adjust_placeholders_multiple_digits() {
        // Test with multi-digit placeholder numbers
        let sql = "col1 = $10 AND col2 = $15";
        let result = adjust_placeholders(sql, 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "col1 = $15 AND col2 = $20");
    }

    #[test]
    fn test_adjust_placeholders_invalid_number() {
        // Test with invalid placeholder number (too large for usize on some systems)
        // This is theoretical but good to test error handling
        let sql = "value = $99999999999999999999999999999";
        let result = adjust_placeholders(sql, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid placeholder number"));
    }
}
