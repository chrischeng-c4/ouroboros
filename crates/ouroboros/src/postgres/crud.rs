//! CRUD operations for PostgreSQL.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use sqlx::Row as SqlxRow;

use ouroboros_postgres::{QueryBuilder, Operator, OrderDirection, Row, SchemaInspector};

use super::conversion::{
    get_connection, py_dict_to_extracted_values, py_value_to_extracted,
    extracted_to_py_value, adjust_placeholders,
};
use super::wrappers::{RowWrapper, OptionalRowWrapper, RowsWrapper};

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
pub(super) fn insert_one<'py>(
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
pub(super) fn insert_many<'py>(
    py: Python<'py>,
    table: String,
    rows: Vec<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract all rows (GIL held)
    let mut extracted_rows: Vec<HashMap<String, ouroboros_postgres::ExtractedValue>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let values = py_dict_to_extracted_values(py, row)?;
        // Convert Vec<(String, ExtractedValue)> to HashMap
        let map: HashMap<String, ouroboros_postgres::ExtractedValue> = values.into_iter().collect();
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
pub(super) fn upsert_one<'py>(
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
pub(super) fn upsert_many<'py>(
    py: Python<'py>,
    table: String,
    rows: Vec<Bound<'py, PyDict>>,
    conflict_target: Vec<String>,
    update_columns: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract all rows (GIL held)
    let mut extracted_rows: Vec<HashMap<String, ouroboros_postgres::ExtractedValue>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let values = py_dict_to_extracted_values(py, row)?;
        // Convert Vec<(String, ExtractedValue)> to HashMap
        let map: HashMap<String, ouroboros_postgres::ExtractedValue> = values.into_iter().collect();
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
pub(super) fn fetch_one<'py>(
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
#[pyo3(signature = (table, filter, limit=None, offset=None, order_by=None, distinct=None, distinct_on=None))]
pub(super) fn fetch_all<'py>(
    py: Python<'py>,
    table: String,
    filter: &Bound<'_, PyDict>,
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<Vec<(String, String)>>,
    distinct: Option<bool>,
    distinct_on: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    // Phase 1: Extract Python values (GIL held)
    let filter_values = py_dict_to_extracted_values(py, filter)?;

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Apply DISTINCT settings
        if distinct.unwrap_or(false) {
            query = query.distinct();
        }

        if let Some(cols) = distinct_on {
            let col_refs: Vec<&str> = cols.iter().map(|s| s.as_str()).collect();
            query = query.distinct_on(&col_refs)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid distinct_on: {}", e)))?;
        }

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
pub(super) fn fetch_one_with_relations<'py>(
    py: Python<'py>,
    table: String,
    id: i64,
    relations: Vec<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    use ouroboros_postgres::row::RelationConfig;
    use ouroboros_postgres::query::JoinType;

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
pub(super) fn fetch_one_eager<'py>(
    py: Python<'py>,
    table: String,
    id: i64,
    joins: Vec<(String, String, String)>,
) -> PyResult<Bound<'py, PyAny>> {
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
pub(super) fn fetch_many_with_relations<'py>(
    py: Python<'py>,
    table: String,
    relations: Vec<Bound<'py, PyDict>>,
    filter: Option<Bound<'py, PyDict>>,
    order_by: Option<(String, String)>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> PyResult<Bound<'py, PyAny>> {
    use ouroboros_postgres::row::RelationConfig;
    use ouroboros_postgres::query::{JoinType, OrderDirection};
    use ouroboros_postgres::ExtractedValue;

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
pub(super) fn update_one<'py>(
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

/// Update multiple rows matching WHERE clause
///
/// Args:
///     table: Table name
///     updates: Dictionary of column values to update
///     where_clause: SQL WHERE clause string (without "WHERE" keyword)
///     params: List of parameter values for WHERE clause
///     returning: Optional list of column names to return (default: None)
///
/// Returns:
///     If returning is None: Number of rows updated (int)
///     If returning is Some: List of dicts with returned column values
///
/// Example:
///     updated = await update_many("users", {"status": "active"}, "age > $1", [25])
///     results = await update_many("users", {"status": "active"}, "age > $1", [25], ["id", "name"])
#[pyfunction]
#[pyo3(signature = (table, updates, where_clause, params, returning=None))]
pub(super) fn update_many<'py>(
    py: Python<'py>,
    table: String,
    updates: &Bound<'_, PyDict>,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
    returning: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python values (GIL held)
    let update_values = py_dict_to_extracted_values(py, updates)?;
    let where_params: Vec<ouroboros_postgres::ExtractedValue> = params
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

        // Add RETURNING clause if specified
        if let Some(ref cols) = returning {
            if !cols.is_empty() {
                let returning_clause = if cols.len() == 1 && cols[0] == "*" {
                    "*".to_string()
                } else {
                    cols.iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                sql.push_str(&format!(" RETURNING {}", returning_clause));
            }
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
        if returning.is_some() {
            // With RETURNING clause, fetch rows
            let rows = sqlx::query_with(&sql, args)
                .fetch_all(conn.pool())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Update failed: {}", e)))?;

            // Convert rows to Python dicts
            Python::with_gil(|py| {
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
            })
        } else {
            // Without RETURNING, return row count
            let result = sqlx::query_with(&sql, args)
                .execute(conn.pool())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Update failed: {}", e)))?;

            // Phase 3: Return result (GIL acquired inside future_into_py)
            Python::with_gil(|py| {
                Ok((result.rows_affected() as i64).to_object(py))
            })
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
pub(super) fn delete_one<'py>(
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

/// Delete multiple rows matching WHERE clause
///
/// Args:
///     table: Table name
///     where_clause: SQL WHERE clause string (without "WHERE" keyword)
///     params: List of parameter values
///     returning: Optional list of column names to return (default: None)
///
/// Returns:
///     If returning is None: Number of rows deleted (int)
///     If returning is Some: List of dicts with returned column values
///
/// Example:
///     deleted = await delete_many("users", "age < $1", [18])
///     results = await delete_many("users", "age < $1", [18], ["id", "name"])
#[pyfunction]
#[pyo3(signature = (table, where_clause, params, returning=None))]
pub(super) fn delete_many<'py>(
    py: Python<'py>,
    table: String,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
    returning: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python parameter values (GIL held)
    let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params
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

        // Add RETURNING clause if specified
        if let Some(ref cols) = returning {
            if !cols.is_empty() {
                let returning_clause = if cols.len() == 1 && cols[0] == "*" {
                    "*".to_string()
                } else {
                    cols.iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                sql.push_str(&format!(" RETURNING {}", returning_clause));
            }
        }

        // Bind parameters
        let mut args = sqlx::postgres::PgArguments::default();
        for param in &extracted_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        // Execute query
        if returning.is_some() {
            // With RETURNING clause, fetch rows
            let rows = sqlx::query_with(&sql, args)
                .fetch_all(conn.pool())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))?;

            // Convert rows to Python dicts
            Python::with_gil(|py| {
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
            })
        } else {
            // Without RETURNING, return row count
            let result = sqlx::query_with(&sql, args)
                .execute(conn.pool())
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))?;

            // Phase 3: Return result (GIL acquired inside future_into_py)
            Python::with_gil(|py| {
                Ok((result.rows_affected() as i64).to_object(py))
            })
        }
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
pub(super) fn delete_with_cascade<'py>(
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
pub(super) fn delete_checked<'py>(
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
pub(super) fn get_backreferences<'py>(
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
pub(super) fn count<'py>(
    py: Python<'py>,
    table: String,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python parameter values (GIL held)
    let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params
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
