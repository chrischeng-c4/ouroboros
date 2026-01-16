//! Advanced query functions (find_many, aggregates, CTEs).

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use ouroboros_postgres::{QueryBuilder, Operator, OrderDirection, Row};
use ouroboros_postgres::query::{AggregateFunction, WindowFunction, WindowSpec};

use super::conversion::{get_connection, py_value_to_extracted};
use super::wrappers::{RowWrapper, OptionalRowWrapper, RowsWrapper};

/// Find a single row by foreign key value
#[pyfunction]
pub(super) fn find_by_foreign_key<'py>(
    py: Python<'py>,
    table: String,
    foreign_key_column: String,
    foreign_key_value: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;
    let fk_val = py_value_to_extracted(py, foreign_key_value)?;

    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        query = query.where_clause(&foreign_key_column, Operator::Eq, fk_val)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid foreign key: {}", e)))?;

        query = query.limit(1);

        let (sql, params) = query.build_select();

        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        let result = sqlx::query_with(&sql, args)
            .fetch_optional(conn.pool())
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

/// Find multiple rows with advanced query options
#[pyfunction]
#[pyo3(signature = (table, where_clause, params, order_by=None, offset=None, limit=None, select_cols=None, distinct=None, distinct_on=None))]
pub(super) fn find_many<'py>(
    py: Python<'py>,
    table: String,
    where_clause: String,
    params: Vec<Bound<'py, PyAny>>,
    order_by: Option<Vec<(String, String)>>,
    offset: Option<i64>,
    limit: Option<i64>,
    select_cols: Option<Vec<String>>,
    distinct: Option<bool>,
    distinct_on: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params
        .iter()
        .map(|param| py_value_to_extracted(py, param))
        .collect::<Result<Vec<_>, _>>()?;

    future_into_py(py, async move {
        let mut sql = if let Some(cols) = &select_cols {
            let cols_quoted: Vec<String> = cols.iter().map(|c| format!("\"{}\"", c)).collect();
            format!("SELECT {} FROM {}", cols_quoted.join(", "), table)
        } else {
            format!("SELECT * FROM {}", table)
        };

        if distinct.unwrap_or(false) {
            sql = sql.replace("SELECT ", "SELECT DISTINCT ");
        }

        if let Some(cols) = &distinct_on {
            let cols_quoted: Vec<String> = cols.iter().map(|c| format!("\"{}\"", c)).collect();
            sql = sql.replace("SELECT ", &format!("SELECT DISTINCT ON ({}) ", cols_quoted.join(", ")));
        }

        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        if let Some(order_specs) = order_by {
            let order_parts: Vec<String> = order_specs
                .iter()
                .map(|(col, dir)| format!("\"{}\" {}", col, if dir.to_lowercase() == "desc" { "DESC" } else { "ASC" }))
                .collect();
            if !order_parts.is_empty() {
                sql.push_str(&format!(" ORDER BY {}", order_parts.join(", ")));
            }
        }

        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {}", l));
        }

        if let Some(o) = offset {
            sql.push_str(&format!(" OFFSET {}", o));
        }

        let mut args = sqlx::postgres::PgArguments::default();
        for param in &extracted_params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))?;

        let mut wrappers = Vec::with_capacity(pg_rows.len());
        for pg_row in &pg_rows {
            let row = Row::from_sqlx(pg_row)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;
            wrappers.push(RowWrapper::from_row(&row)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}

/// Helper function to parse operator string to Operator enum
pub(super) fn parse_operator(op_str: &str) -> PyResult<Operator> {
    match op_str.to_lowercase().as_str() {
        "eq" | "=" => Ok(Operator::Eq),
        "ne" | "!=" | "<>" => Ok(Operator::Ne),
        "gt" | ">" => Ok(Operator::Gt),
        "gte" | ">=" => Ok(Operator::Gte),
        "lt" | "<" => Ok(Operator::Lt),
        "lte" | "<=" => Ok(Operator::Lte),
        "like" => Ok(Operator::Like),
        "ilike" => Ok(Operator::ILike),
        "in" => Ok(Operator::In),
        "is_null" => Ok(Operator::IsNull),
        "is_not_null" => Ok(Operator::IsNotNull),
        "json_contains" => Ok(Operator::JsonContains),
        "json_contained_by" => Ok(Operator::JsonContainedBy),
        "json_key_exists" => Ok(Operator::JsonKeyExists),
        "json_any_key_exists" => Ok(Operator::JsonAnyKeyExists),
        "json_all_keys_exist" => Ok(Operator::JsonAllKeysExist),
        _ => Err(PyValueError::new_err(format!("Unknown operator: {}", op_str))),
    }
}

/// Execute an aggregate query with GROUP BY support
#[pyfunction]
#[pyo3(signature = (table, aggregates, group_by=None, having=None, where_conditions=None, order_by=None, limit=None, distinct=None, distinct_on=None, ctes=None, subqueries=None, windows=None, set_operations=None))]
pub(super) fn query_aggregate<'py>(
    py: Python<'py>,
    table: String,
    aggregates: Vec<(String, Option<String>, Option<String>)>,
    group_by: Option<Vec<String>>,
    having: Option<Vec<(String, Option<String>, String, Bound<'py, PyAny>)>>,
    where_conditions: Option<Vec<(String, String, Bound<'py, PyAny>)>>,
    order_by: Option<Vec<(String, String)>>,
    limit: Option<i64>,
    distinct: Option<bool>,
    distinct_on: Option<Vec<String>>,
    ctes: Option<Vec<(String, String, Vec<Bound<'py, PyAny>>)>>,
    subqueries: Option<Vec<(String, Option<String>, String, Vec<Bound<'py, PyAny>>)>>,
    windows: Option<Vec<(String, Option<String>, Option<i32>, Option<Bound<'py, PyAny>>, Vec<String>, Vec<(String, String)>, String)>>,
    set_operations: Option<Vec<(String, String, Vec<Bound<'py, PyAny>>)>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Parse aggregates
    let mut agg_funcs = Vec::new();
    for (func_type, column, alias) in aggregates {
        let agg_func = match func_type.to_lowercase().as_str() {
            "count" => AggregateFunction::Count,
            "count_column" => AggregateFunction::CountColumn(column.ok_or_else(|| PyValueError::new_err("count_column requires a column name"))?),
            "count_distinct" => AggregateFunction::CountDistinct(column.ok_or_else(|| PyValueError::new_err("count_distinct requires a column name"))?),
            "sum" => AggregateFunction::Sum(column.ok_or_else(|| PyValueError::new_err("sum requires a column name"))?),
            "avg" => AggregateFunction::Avg(column.ok_or_else(|| PyValueError::new_err("avg requires a column name"))?),
            "min" => AggregateFunction::Min(column.ok_or_else(|| PyValueError::new_err("min requires a column name"))?),
            "max" => AggregateFunction::Max(column.ok_or_else(|| PyValueError::new_err("max requires a column name"))?),
            _ => return Err(PyValueError::new_err(format!("Unknown aggregate function: {}", func_type))),
        };
        agg_funcs.push((agg_func, alias));
    }

    // Extract WHERE conditions
    let where_params: Vec<(String, Operator, ouroboros_postgres::ExtractedValue)> = if let Some(conditions) = where_conditions {
        conditions.into_iter().map(|(field, op_str, value)| {
            let operator = parse_operator(&op_str)?;
            let extracted_value = py_value_to_extracted(py, &value)?;
            Ok((field, operator, extracted_value))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    // Extract HAVING conditions
    let having_params: Vec<(AggregateFunction, Operator, ouroboros_postgres::ExtractedValue)> = if let Some(conditions) = having {
        conditions.into_iter().map(|(func_type, column, op_str, value)| {
            let agg_func = match func_type.to_lowercase().as_str() {
                "count" => AggregateFunction::Count,
                "count_column" => AggregateFunction::CountColumn(column.clone().ok_or_else(|| PyValueError::new_err("count_column requires a column name"))?),
                "count_distinct" => AggregateFunction::CountDistinct(column.clone().ok_or_else(|| PyValueError::new_err("count_distinct requires a column name"))?),
                "sum" => AggregateFunction::Sum(column.clone().ok_or_else(|| PyValueError::new_err("sum requires a column name"))?),
                "avg" => AggregateFunction::Avg(column.clone().ok_or_else(|| PyValueError::new_err("avg requires a column name"))?),
                "min" => AggregateFunction::Min(column.clone().ok_or_else(|| PyValueError::new_err("min requires a column name"))?),
                "max" => AggregateFunction::Max(column.clone().ok_or_else(|| PyValueError::new_err("max requires a column name"))?),
                _ => return Err(PyValueError::new_err(format!("Unknown aggregate function: {}", func_type))),
            };
            let operator = parse_operator(&op_str)?;
            let extracted_value = py_value_to_extracted(py, &value)?;
            Ok((agg_func, operator, extracted_value))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    // Extract CTEs
    let cte_params: Vec<(String, String, Vec<ouroboros_postgres::ExtractedValue>)> = if let Some(cte_list) = ctes {
        cte_list.into_iter().map(|(name, sql, params)| {
            let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params.iter().map(|p| py_value_to_extracted(py, p)).collect::<PyResult<Vec<_>>>()?;
            Ok((name, sql, extracted_params))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    // Extract subqueries
    let subquery_params: Vec<(String, Option<String>, String, Vec<ouroboros_postgres::ExtractedValue>)> = if let Some(sq_list) = subqueries {
        sq_list.into_iter().map(|(sq_type, field, sql, params)| {
            let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params.iter().map(|p| py_value_to_extracted(py, p)).collect::<PyResult<Vec<_>>>()?;
            Ok((sq_type, field, sql, extracted_params))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    // Extract window functions
    let window_params: Vec<(WindowFunction, Vec<String>, Vec<(String, OrderDirection)>, String)> = if let Some(window_list) = windows {
        window_list.into_iter().map(|(func_type, column, offset, default_val, partition_by, order_by_spec, alias)| {
            let func = match func_type.to_lowercase().as_str() {
                "row_number" => WindowFunction::RowNumber,
                "rank" => WindowFunction::Rank,
                "dense_rank" => WindowFunction::DenseRank,
                "ntile" => WindowFunction::Ntile(offset.ok_or_else(|| PyValueError::new_err("ntile requires an offset parameter"))?),
                "sum" => WindowFunction::Sum(column.clone().ok_or_else(|| PyValueError::new_err("sum requires a column name"))?),
                "avg" => WindowFunction::Avg(column.clone().ok_or_else(|| PyValueError::new_err("avg requires a column name"))?),
                "count" => WindowFunction::Count,
                "count_column" => WindowFunction::CountColumn(column.clone().ok_or_else(|| PyValueError::new_err("count_column requires a column name"))?),
                "min" => WindowFunction::Min(column.clone().ok_or_else(|| PyValueError::new_err("min requires a column name"))?),
                "max" => WindowFunction::Max(column.clone().ok_or_else(|| PyValueError::new_err("max requires a column name"))?),
                "lag" => {
                    let col = column.clone().ok_or_else(|| PyValueError::new_err("lag requires a column name"))?;
                    let default_extracted = if let Some(default_py) = default_val { Some(py_value_to_extracted(py, &default_py)?) } else { None };
                    WindowFunction::Lag(col, offset, default_extracted)
                }
                "lead" => {
                    let col = column.clone().ok_or_else(|| PyValueError::new_err("lead requires a column name"))?;
                    let default_extracted = if let Some(default_py) = default_val { Some(py_value_to_extracted(py, &default_py)?) } else { None };
                    WindowFunction::Lead(col, offset, default_extracted)
                }
                "first_value" => WindowFunction::FirstValue(column.clone().ok_or_else(|| PyValueError::new_err("first_value requires a column name"))?),
                "last_value" => WindowFunction::LastValue(column.clone().ok_or_else(|| PyValueError::new_err("last_value requires a column name"))?),
                _ => return Err(PyValueError::new_err(format!("Unknown window function: {}", func_type))),
            };
            let order_specs: Vec<(String, OrderDirection)> = order_by_spec.into_iter().map(|(col, dir)| {
                let direction = if dir.to_lowercase() == "desc" { OrderDirection::Desc } else { OrderDirection::Asc };
                (col, direction)
            }).collect();
            Ok((func, partition_by, order_specs, alias))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    // Extract set operations
    let set_op_params: Vec<(String, String, Vec<ouroboros_postgres::ExtractedValue>)> = if let Some(ops) = set_operations {
        ops.into_iter().map(|(op_type, sql, params)| {
            let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params.iter().map(|p| py_value_to_extracted(py, p)).collect::<Result<Vec<_>, _>>()?;
            Ok((op_type, sql, extracted_params))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Apply CTEs
        for (name, sql, params) in cte_params {
            query = query.with_cte_raw(&name, &sql, params)
                .map_err(|e| PyValueError::new_err(format!("Invalid CTE: {}", e)))?;
        }

        // Apply DISTINCT
        if distinct.unwrap_or(false) {
            query = query.distinct();
        }

        if let Some(cols) = distinct_on {
            let col_refs: Vec<&str> = cols.iter().map(|s| s.as_str()).collect();
            query = query.distinct_on(&col_refs)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid distinct_on: {}", e)))?;
        }

        // Add aggregates
        for (agg_func, alias) in agg_funcs {
            let alias_ref: Option<&str> = alias.as_deref();
            query = query.aggregate(agg_func, alias_ref)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid aggregate: {}", e)))?;
        }

        // Add GROUP BY
        if let Some(group_cols) = group_by {
            let group_refs: Vec<&str> = group_cols.iter().map(|s| s.as_str()).collect();
            query = query.group_by(&group_refs)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid group_by: {}", e)))?;
        }

        // Add HAVING conditions
        for (agg_func, operator, value) in having_params {
            query = query.having(agg_func, operator, value)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid having clause: {}", e)))?;
        }

        // Add WHERE conditions
        for (field, operator, value) in where_params {
            query = query.where_clause(&field, operator, value)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid filter: {}", e)))?;
        }

        // Add subquery conditions
        for (sq_type, field, sql, params) in subquery_params {
            match sq_type.to_lowercase().as_str() {
                "in" => {
                    let field_str = field.ok_or_else(|| PyValueError::new_err("IN subquery requires a field name"))?;
                    query = query.where_in_raw_sql(&field_str, &sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid IN subquery: {}", e)))?;
                }
                "not_in" => {
                    let field_str = field.ok_or_else(|| PyValueError::new_err("NOT IN subquery requires a field name"))?;
                    query = query.where_not_in_raw_sql(&field_str, &sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid NOT IN subquery: {}", e)))?;
                }
                "exists" => {
                    query = query.where_exists_raw_sql(&sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid EXISTS subquery: {}", e)))?;
                }
                "not_exists" => {
                    query = query.where_not_exists_raw_sql(&sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid NOT EXISTS subquery: {}", e)))?;
                }
                _ => return Err(PyValueError::new_err(format!("Unknown subquery type: {}", sq_type))),
            }
        }

        // Add window functions
        for (func, partition_by, order_specs, alias) in window_params {
            let mut spec = WindowSpec::new();
            if !partition_by.is_empty() {
                let cols: Vec<&str> = partition_by.iter().map(|s| s.as_str()).collect();
                spec = spec.partition_by(&cols);
            }
            for (col, direction) in order_specs {
                spec = spec.order_by(&col, direction);
            }
            query = query.window(func, spec, &alias)
                .map_err(|e| PyValueError::new_err(format!("Invalid window function: {}", e)))?;
        }

        // Add ORDER BY
        if let Some(order_specs) = order_by {
            for (field, direction) in order_specs {
                let dir = if direction.to_lowercase() == "desc" { OrderDirection::Desc } else { OrderDirection::Asc };
                query = query.order_by(&field, dir)
                    .map_err(|e| PyRuntimeError::new_err(format!("Invalid order_by: {}", e)))?;
            }
        }

        // Add LIMIT
        if let Some(l) = limit {
            query = query.limit(l);
        }

        // Apply set operations
        for (op_type, sql, params) in set_op_params {
            query = match op_type.to_lowercase().as_str() {
                "union" => query.union_raw(sql, params),
                "union_all" => query.union_all_raw(sql, params),
                "intersect" => query.intersect_raw(sql, params),
                "intersect_all" => query.intersect_all_raw(sql, params),
                "except" => query.except_raw(sql, params),
                "except_all" => query.except_all_raw(sql, params),
                _ => return Err(PyValueError::new_err(format!("Unknown set operation: {}", op_type))),
            };
        }

        let (sql, params) = query.build_select();

        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Aggregate query failed: {}", e)))?;

        let mut wrappers = Vec::with_capacity(pg_rows.len());
        for pg_row in &pg_rows {
            let row = Row::from_sqlx(pg_row)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;
            wrappers.push(RowWrapper::from_row(&row)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}

/// Execute a query with Common Table Expressions (CTEs)
#[pyfunction]
#[pyo3(signature = (main_table, ctes, select_columns=None, where_conditions=None, order_by=None, limit=None, subqueries=None))]
pub(super) fn query_with_cte<'py>(
    py: Python<'py>,
    main_table: String,
    ctes: Vec<(String, String, Vec<Bound<'py, PyAny>>)>,
    select_columns: Option<Vec<String>>,
    where_conditions: Option<Vec<(String, String, Bound<'py, PyAny>)>>,
    order_by: Option<Vec<(String, String)>>,
    limit: Option<i64>,
    subqueries: Option<Vec<(String, Option<String>, String, Vec<Bound<'py, PyAny>>)>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Extract CTE parameters
    let cte_params: Vec<(String, String, Vec<ouroboros_postgres::ExtractedValue>)> = ctes
        .into_iter()
        .map(|(name, sql, params)| {
            let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params.iter().map(|p| py_value_to_extracted(py, p)).collect::<PyResult<Vec<_>>>()?;
            Ok((name, sql, extracted_params))
        })
        .collect::<PyResult<Vec<_>>>()?;

    // Extract WHERE conditions
    let where_params: Vec<(String, Operator, ouroboros_postgres::ExtractedValue)> = if let Some(conditions) = where_conditions {
        conditions.into_iter().map(|(field, op_str, value)| {
            let operator = parse_operator(&op_str)?;
            let extracted_value = py_value_to_extracted(py, &value)?;
            Ok((field, operator, extracted_value))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    // Extract subqueries
    let subquery_params: Vec<(String, Option<String>, String, Vec<ouroboros_postgres::ExtractedValue>)> = if let Some(sq_list) = subqueries {
        sq_list.into_iter().map(|(sq_type, field, sql, params)| {
            let extracted_params: Vec<ouroboros_postgres::ExtractedValue> = params.iter().map(|p| py_value_to_extracted(py, p)).collect::<PyResult<Vec<_>>>()?;
            Ok((sq_type, field, sql, extracted_params))
        }).collect::<PyResult<Vec<_>>>()?
    } else {
        Vec::new()
    };

    future_into_py(py, async move {
        let mut query = QueryBuilder::new(&main_table)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid table name: {}", e)))?;

        // Apply CTEs
        for (name, sql, params) in cte_params {
            query = query.with_cte_raw(&name, &sql, params)
                .map_err(|e| PyValueError::new_err(format!("Invalid CTE: {}", e)))?;
        }

        // Apply SELECT columns
        if let Some(cols) = select_columns {
            query = query.select(cols)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid select columns: {}", e)))?;
        }

        // Add WHERE conditions
        for (field, operator, value) in where_params {
            query = query.where_clause(&field, operator, value)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid filter: {}", e)))?;
        }

        // Add subquery conditions
        for (sq_type, field, sql, params) in subquery_params {
            match sq_type.to_lowercase().as_str() {
                "in" => {
                    let field_str = field.ok_or_else(|| PyValueError::new_err("IN subquery requires a field name"))?;
                    query = query.where_in_raw_sql(&field_str, &sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid IN subquery: {}", e)))?;
                }
                "not_in" => {
                    let field_str = field.ok_or_else(|| PyValueError::new_err("NOT IN subquery requires a field name"))?;
                    query = query.where_not_in_raw_sql(&field_str, &sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid NOT IN subquery: {}", e)))?;
                }
                "exists" => {
                    query = query.where_exists_raw_sql(&sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid EXISTS subquery: {}", e)))?;
                }
                "not_exists" => {
                    query = query.where_not_exists_raw_sql(&sql, params)
                        .map_err(|e| PyRuntimeError::new_err(format!("Invalid NOT EXISTS subquery: {}", e)))?;
                }
                _ => return Err(PyValueError::new_err(format!("Unknown subquery type: {}", sq_type))),
            }
        }

        // Add ORDER BY
        if let Some(order_specs) = order_by {
            for (field, direction) in order_specs {
                let dir = if direction.to_lowercase() == "desc" { OrderDirection::Desc } else { OrderDirection::Asc };
                query = query.order_by(&field, dir)
                    .map_err(|e| PyRuntimeError::new_err(format!("Invalid order_by: {}", e)))?;
            }
        }

        // Add LIMIT
        if let Some(l) = limit {
            query = query.limit(l);
        }

        let (sql, params) = query.build_select();

        let mut args = sqlx::postgres::PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to bind parameter: {}", e)))?;
        }

        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(conn.pool())
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("CTE query failed: {}", e)))?;

        let mut wrappers = Vec::with_capacity(pg_rows.len());
        for pg_row in &pg_rows {
            let row = Row::from_sqlx(pg_row)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))?;
            wrappers.push(RowWrapper::from_row(&row)?);
        }

        Ok(RowsWrapper(wrappers))
    })
}
