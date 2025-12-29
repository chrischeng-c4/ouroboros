# Upsert Implementation Task

## Overview
Implement INSERT ON CONFLICT UPDATE (upsert) operations for data-bridge-postgres.

## Reference
- Design Doc: `/Users/chrischeng/projects/data-bridge/docs/postgres_orm_design.md` Section 3
- Existing insert implementation: `/Users/chrischeng/projects/data-bridge/crates/data-bridge-postgres/src/row.rs` lines 178-332

## Tasks

### 1. Rust Layer - QueryBuilder (query.rs)

**File**: `/Users/chrischeng/projects/data-bridge/crates/data-bridge-postgres/src/query.rs`

Add `build_upsert` method to QueryBuilder after `build_update` (around line 392):

```rust
/// Builds an UPSERT SQL query (INSERT ON CONFLICT UPDATE).
///
/// Returns the SQL string with $1, $2, etc. placeholders and the parameter values.
pub fn build_upsert(
    &self,
    values: &[(String, ExtractedValue)],
    conflict_target: &[String],
    update_columns: Option<&[String]>,
) -> Result<(String, Vec<ExtractedValue>)> {
    // Validation
    if values.is_empty() {
        return Err(DataBridgeError::Query("Cannot upsert with no values".to_string()));
    }
    if conflict_target.is_empty() {
        return Err(DataBridgeError::Query("Conflict target cannot be empty".to_string()));
    }

    // Validate column names
    for (col, _) in values {
        Self::validate_identifier(col)?;
    }
    for col in conflict_target {
        Self::validate_identifier(col)?;
    }
    if let Some(cols) = update_columns {
        for col in cols {
            Self::validate_identifier(col)?;
        }
    }

    // Build INSERT clause
    let mut sql = format!("INSERT INTO {} (", self.table);
    let columns: Vec<&str> = values.iter().map(|(col, _)| col.as_str()).collect();
    sql.push_str(&columns.join(", "));
    sql.push_str(") VALUES (");

    let placeholders: Vec<String> = (1..=values.len()).map(|i| format!("${}", i)).collect();
    sql.push_str(&placeholders.join(", "));
    sql.push_str(")");

    // Build ON CONFLICT clause
    sql.push_str(" ON CONFLICT (");
    sql.push_str(&conflict_target.join(", "));
    sql.push_str(") DO UPDATE SET ");

    // Determine which columns to update
    let columns_to_update: Vec<&str> = if let Some(update_cols) = update_columns {
        update_cols.iter().map(|s| s.as_str()).collect()
    } else {
        // Update all columns except conflict target
        columns.iter()
            .filter(|col| !conflict_target.contains(&col.to_string()))
            .copied()
            .collect()
    };

    if columns_to_update.is_empty() {
        return Err(DataBridgeError::Query(
            "No columns to update after excluding conflict target".to_string()
        ));
    }

    // Build SET clause using EXCLUDED
    let set_parts: Vec<String> = columns_to_update
        .iter()
        .map(|col| format!("{} = EXCLUDED.{}", col, col))
        .collect();
    sql.push_str(&set_parts.join(", "));

    // Add RETURNING *
    sql.push_str(" RETURNING *");

    let params: Vec<ExtractedValue> = values.iter().map(|(_, val)| val.clone()).collect();

    Ok((sql, params))
}
```

Add tests at end of query.rs test module (after line 1308):

```rust
#[test]
fn test_upsert_single_conflict() {
    let qb = QueryBuilder::new("users").unwrap();
    let values = vec![
        ("email".to_string(), ExtractedValue::String("alice@example.com".to_string())),
        ("name".to_string(), ExtractedValue::String("Alice".to_string())),
        ("age".to_string(), ExtractedValue::Int(30)),
    ];
    let conflict_target = vec!["email".to_string()];
    let (sql, params) = qb.build_upsert(&values, &conflict_target, None).unwrap();

    assert!(sql.contains("INSERT INTO users"));
    assert!(sql.contains("ON CONFLICT (email)"));
    assert!(sql.contains("DO UPDATE SET"));
    assert!(sql.contains("name = EXCLUDED.name"));
    assert!(sql.contains("age = EXCLUDED.age"));
    assert!(sql.contains("RETURNING *"));
    assert_eq!(params.len(), 3);
}

#[test]
fn test_upsert_selective_update() {
    let qb = QueryBuilder::new("users").unwrap();
    let values = vec![
        ("email".to_string(), ExtractedValue::String("alice@example.com".to_string())),
        ("name".to_string(), ExtractedValue::String("Alice".to_string())),
        ("age".to_string(), ExtractedValue::Int(30)),
    ];
    let conflict_target = vec!["email".to_string()];
    let update_columns = Some(vec!["name".to_string()].as_slice());
    let (sql, params) = qb.build_upsert(&values, &conflict_target, update_columns).unwrap();

    assert!(sql.contains("DO UPDATE SET name = EXCLUDED.name"));
    assert!(!sql.contains("age = EXCLUDED.age"));
    assert_eq!(params.len(), 3);
}

#[test]
fn test_upsert_composite_key() {
    let qb = QueryBuilder::new("users").unwrap();
    let values = vec![
        ("email".to_string(), ExtractedValue::String("alice@example.com".to_string())),
        ("department".to_string(), ExtractedValue::String("Engineering".to_string())),
        ("name".to_string(), ExtractedValue::String("Alice".to_string())),
    ];
    let conflict_target = vec!["email".to_string(), "department".to_string()];
    let (sql, params) = qb.build_upsert(&values, &conflict_target, None).unwrap();

    assert!(sql.contains("ON CONFLICT (email, department)"));
    assert!(sql.contains("DO UPDATE SET name = EXCLUDED.name"));
    assert_eq!(params.len(), 3);
}

#[test]
fn test_upsert_empty_conflict_target() {
    let qb = QueryBuilder::new("users").unwrap();
    let values = vec![
        ("email".to_string(), ExtractedValue::String("alice@example.com".to_string())),
    ];
    let conflict_target: Vec<String> = vec![];
    let result = qb.build_upsert(&values, &conflict_target, None);
    assert!(result.is_err());
}

#[test]
fn test_upsert_invalid_column_name() {
    let qb = QueryBuilder::new("users").unwrap();
    let values = vec![
        ("drop".to_string(), ExtractedValue::String("value".to_string())),
    ];
    let conflict_target = vec!["drop".to_string()];
    let result = qb.build_upsert(&values, &conflict_target, None);
    assert!(result.is_err());
}
```

### 2. Rust Layer - Row Operations (row.rs)

**File**: `/Users/chrischeng/projects/data-bridge/crates/data-bridge-postgres/src/row.rs`

Add after `insert_many` method (around line 332):

```rust
/// Upsert a single row (INSERT ON CONFLICT UPDATE).
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `table` - Table name
/// * `values` - Column values to insert/update
/// * `conflict_target` - Columns for ON CONFLICT clause (unique constraint)
/// * `update_columns` - Optional columns to update on conflict (None = all except conflict)
///
/// # Returns
///
/// Returns the inserted or updated row with all columns.
///
/// # Examples
///
/// ```ignore
/// use data_bridge_postgres::{Connection, ExtractedValue, PoolConfig, Row};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = Connection::new("postgresql://localhost/mydb", PoolConfig::default()).await?;
/// let pool = conn.pool();
///
/// let values = vec![
///     ("email".to_string(), ExtractedValue::String("alice@example.com".to_string())),
///     ("name".to_string(), ExtractedValue::String("Alice".to_string())),
///     ("age".to_string(), ExtractedValue::Int(30)),
/// ];
/// let conflict_target = vec!["email".to_string()];
///
/// // If email exists: UPDATE name and age
/// // If email new: INSERT all values
/// let row = Row::upsert(pool, "users", &values, &conflict_target, None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn upsert(
    pool: &PgPool,
    table: &str,
    values: &[(String, ExtractedValue)],
    conflict_target: &[String],
    update_columns: Option<&[String]>,
) -> Result<Self> {
    if values.is_empty() {
        return Err(DataBridgeError::Query("Cannot upsert with no values".to_string()));
    }

    // Build UPSERT query with RETURNING *
    let query_builder = QueryBuilder::new(table)?;
    let (sql, params) = query_builder.build_upsert(values, conflict_target, update_columns)?;

    // Bind parameters
    let mut args = PgArguments::default();
    for param in &params {
        param.bind_to_arguments(&mut args)?;
    }

    // Execute query with bound arguments
    let row = sqlx::query_with(&sql, args)
        .fetch_one(pool)
        .await
        .map_err(|e| DataBridgeError::Query(format!("Upsert failed: {}", e)))?;

    // Convert PgRow to Row
    Self::from_sqlx(&row)
}

/// Upsert multiple rows with a single batch statement.
///
/// This generates: INSERT INTO table (cols) VALUES ($1,$2),($3,$4) ON CONFLICT (...) DO UPDATE ...
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `table` - Table name
/// * `rows` - Vector of rows (HashMaps of column -> value)
/// * `conflict_target` - Columns for ON CONFLICT clause
/// * `update_columns` - Optional columns to update on conflict
///
/// # Returns
///
/// Returns vector of inserted/updated rows.
///
/// # Examples
///
/// ```ignore
/// use std::collections::HashMap;
/// use data_bridge_postgres::{Connection, ExtractedValue, PoolConfig, Row};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = Connection::new("postgresql://localhost/mydb", PoolConfig::default()).await?;
/// let pool = conn.pool();
///
/// let mut row1 = HashMap::new();
/// row1.insert("email".to_string(), ExtractedValue::String("alice@example.com".to_string()));
/// row1.insert("name".to_string(), ExtractedValue::String("Alice".to_string()));
///
/// let mut row2 = HashMap::new();
/// row2.insert("email".to_string(), ExtractedValue::String("bob@example.com".to_string()));
/// row2.insert("name".to_string(), ExtractedValue::String("Bob".to_string()));
///
/// let conflict_target = vec!["email".to_string()];
/// let rows = Row::upsert_many(pool, "users", &[row1, row2], &conflict_target, None).await?;
/// assert_eq!(rows.len(), 2);
/// # Ok(())
/// # }
/// ```
pub async fn upsert_many(
    pool: &PgPool,
    table: &str,
    rows: &[HashMap<String, ExtractedValue>],
    conflict_target: &[String],
    update_columns: Option<&[String]>,
) -> Result<Vec<Self>> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    if conflict_target.is_empty() {
        return Err(DataBridgeError::Query("Conflict target cannot be empty".to_string()));
    }

    // Get column names from first row and validate
    let first_row = &rows[0];
    if first_row.is_empty() {
        return Err(DataBridgeError::Query("Cannot upsert with no columns".to_string()));
    }

    // Collect and sort column names for consistent ordering
    let mut column_names: Vec<&String> = first_row.keys().collect();
    column_names.sort();

    // Validate all rows have the same columns
    for (idx, row) in rows.iter().enumerate().skip(1) {
        if row.len() != first_row.len() {
            return Err(DataBridgeError::Query(format!(
                "Row {} has {} columns, expected {} columns",
                idx,
                row.len(),
                first_row.len()
            )));
        }

        for col in column_names.iter() {
            if !row.contains_key(*col) {
                return Err(DataBridgeError::Query(format!(
                    "Row {} is missing column: {}",
                    idx, col
                )));
            }
        }
    }

    // Validate table name and column names
    QueryBuilder::validate_identifier(table)?;
    for col in &column_names {
        QueryBuilder::validate_identifier(col)?;
    }
    for col in conflict_target {
        QueryBuilder::validate_identifier(col)?;
    }
    if let Some(cols) = update_columns {
        for col in cols {
            QueryBuilder::validate_identifier(col)?;
        }
    }

    // Build SQL with multiple VALUES clauses
    let mut sql = format!("INSERT INTO {} (", table);
    sql.push_str(&column_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    sql.push_str(") VALUES ");

    // Build VALUES placeholders for all rows
    let num_cols = column_names.len();
    let value_clauses: Vec<String> = (0..rows.len())
        .map(|row_idx| {
            let start = row_idx * num_cols + 1;
            let placeholders: Vec<String> = (start..start + num_cols)
                .map(|i| format!("${}", i))
                .collect();
            format!("({})", placeholders.join(", "))
        })
        .collect();
    sql.push_str(&value_clauses.join(", "));

    // Add ON CONFLICT clause
    sql.push_str(" ON CONFLICT (");
    sql.push_str(&conflict_target.join(", "));
    sql.push_str(") DO UPDATE SET ");

    // Determine which columns to update
    let columns_to_update: Vec<&&String> = if let Some(update_cols) = update_columns {
        column_names.iter()
            .filter(|col| update_cols.contains(&col.to_string()))
            .collect()
    } else {
        // Update all columns except conflict target
        column_names.iter()
            .filter(|col| !conflict_target.contains(&col.to_string()))
            .collect()
    };

    if columns_to_update.is_empty() {
        return Err(DataBridgeError::Query(
            "No columns to update after excluding conflict target".to_string()
        ));
    }

    // Build SET clause using EXCLUDED
    let set_parts: Vec<String> = columns_to_update
        .iter()
        .map(|col| format!("{} = EXCLUDED.{}", col, col))
        .collect();
    sql.push_str(&set_parts.join(", "));

    // Add RETURNING *
    sql.push_str(" RETURNING *");

    // Bind all values in row-major order
    let mut args = PgArguments::default();
    for row in rows {
        for col_name in &column_names {
            let value = row.get(*col_name).ok_or_else(|| {
                DataBridgeError::Query(format!("Missing column: {}", col_name))
            })?;
            value.bind_to_arguments(&mut args)?;
        }
    }

    // Execute query
    let pg_rows = sqlx::query_with(&sql, args)
        .fetch_all(pool)
        .await
        .map_err(|e| DataBridgeError::Query(format!("Batch upsert failed: {}", e)))?;

    // Convert all rows
    pg_rows.iter().map(Self::from_sqlx).collect()
}
```

### 3. PyO3 Bindings (postgres.rs)

**File**: `/Users/chrischeng/projects/data-bridge/crates/data-bridge/src/postgres.rs`

Add after `insert_many` function (around line 460):

```rust
/// Upsert a single row (INSERT ON CONFLICT UPDATE)
///
/// Args:
///     table: Table name
///     data: Dictionary of column values
///     conflict_target: Column(s) for ON CONFLICT clause (str or list)
///     update_columns: Optional list of columns to update on conflict
///
/// Returns:
///     Dictionary with upserted row data
///
/// Example:
///     result = await upsert_one(
///         "users",
///         {"email": "alice@example.com", "name": "Alice", "age": 30},
///         conflict_target="email"
///     )
#[pyfunction]
#[pyo3(signature = (table, data, conflict_target, update_columns=None))]
fn upsert_one<'py>(
    py: Python<'py>,
    table: String,
    data: &Bound<'_, PyDict>,
    conflict_target: &Bound<'_, PyAny>,
    update_columns: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract Python values (GIL held)
    let values = py_dict_to_extracted_values(py, data)?;

    // Convert conflict_target to Vec<String>
    let conflict_vec: Vec<String> = if let Ok(s) = conflict_target.extract::<String>() {
        vec![s]
    } else if let Ok(list) = conflict_target.extract::<Vec<String>>() {
        list
    } else {
        return Err(PyValueError::new_err(
            "conflict_target must be a string or list of strings"
        ));
    };

    // Phase 2: Execute SQL (GIL released via future_into_py)
    future_into_py(py, async move {
        let row = Row::upsert(
            conn.pool(),
            &table,
            &values,
            &conflict_vec,
            update_columns.as_deref(),
        )
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Upsert failed: {}", e)))?;

        // Phase 3: Convert result to Python (GIL acquired inside future_into_py)
        RowWrapper::from_row(&row)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert row: {}", e)))
    })
}

/// Upsert multiple rows (batch INSERT ON CONFLICT UPDATE)
///
/// Args:
///     table: Table name
///     rows: List of dictionaries with column values
///     conflict_target: Column(s) for ON CONFLICT clause
///     update_columns: Optional list of columns to update on conflict
///
/// Returns:
///     List of dictionaries with upserted row data
///
/// Example:
///     results = await upsert_many(
///         "users",
///         [
///             {"email": "alice@example.com", "name": "Alice"},
///             {"email": "bob@example.com", "name": "Bob"}
///         ],
///         conflict_target="email"
///     )
#[pyfunction]
#[pyo3(signature = (table, rows, conflict_target, update_columns=None))]
fn upsert_many<'py>(
    py: Python<'py>,
    table: String,
    rows: Vec<Bound<'py, PyDict>>,
    conflict_target: &Bound<'_, PyAny>,
    update_columns: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyAny>> {
    let conn = get_connection()?;

    // Phase 1: Extract all rows (GIL held)
    let mut extracted_rows: Vec<HashMap<String, data_bridge_postgres::ExtractedValue>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let values = py_dict_to_extracted_values(py, row)?;
        let map: HashMap<String, data_bridge_postgres::ExtractedValue> = values.into_iter().collect();
        extracted_rows.push(map);
    }

    // Convert conflict_target to Vec<String>
    let conflict_vec: Vec<String> = if let Ok(s) = conflict_target.extract::<String>() {
        vec![s]
    } else if let Ok(list) = conflict_target.extract::<Vec<String>>() {
        list
    } else {
        return Err(PyValueError::new_err(
            "conflict_target must be a string or list of strings"
        ));
    };

    // Phase 2: Execute batch UPSERT (GIL released via future_into_py)
    future_into_py(py, async move {
        let batch_results = Row::upsert_many(
            conn.pool(),
            &table,
            &extracted_rows,
            &conflict_vec,
            update_columns.as_deref(),
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
```

Add to module registration at end of file (around line 1815):

```rust
m.add_function(wrap_pyfunction!(upsert_one, m)?)?;
m.add_function(wrap_pyfunction!(upsert_many, m)?)?;
```

### 4. Python Layer (connection.py)

**File**: `/Users/chrischeng/projects/data-bridge/python/data_bridge/postgres/connection.py`

Add after migration functions (around line 533):

```python
# ============================================================================
# UPSERT OPERATIONS
# ============================================================================

async def upsert_one(
    table: str,
    document: Dict[str, Any],
    conflict_target: Union[str, List[str]],
    update_columns: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """
    Insert or update a document using ON CONFLICT.

    Args:
        table: Table name
        document: Document data as dictionary
        conflict_target: Column(s) for ON CONFLICT clause (unique constraint)
        update_columns: Columns to update on conflict (None = all except conflict_target)

    Returns:
        Inserted or updated document with all columns

    Example:
        >>> # Upsert user by email (unique)
        >>> user = await upsert_one(
        ...     "users",
        ...     {"email": "alice@example.com", "name": "Alice Updated"},
        ...     conflict_target="email"
        ... )
        >>> # If email exists: UPDATE name
        >>> # If email new: INSERT both email and name
        >>>
        >>> # Upsert with composite key
        >>> post = await upsert_one(
        ...     "posts",
        ...     {"user_id": 1, "slug": "my-post", "title": "Updated Title"},
        ...     conflict_target=["user_id", "slug"],
        ...     update_columns=["title"]  # Only update title on conflict
        ... )

    Raises:
        RuntimeError: If PostgreSQL engine is not available or upsert fails
    """
    if _engine is None:
        raise RuntimeError("PostgreSQL engine not available.")

    # Normalize conflict_target to list for consistency
    if isinstance(conflict_target, str):
        conflict_target = [conflict_target]

    return await _engine.upsert_one(table, document, conflict_target, update_columns)


async def upsert_many(
    table: str,
    documents: List[Dict[str, Any]],
    conflict_target: Union[str, List[str]],
    update_columns: Optional[List[str]] = None,
) -> List[Dict[str, Any]]:
    """
    Batch insert or update documents using ON CONFLICT.

    Args:
        table: Table name
        documents: List of document dictionaries
        conflict_target: Column(s) for ON CONFLICT clause
        update_columns: Columns to update on conflict

    Returns:
        List of inserted or updated documents

    Example:
        >>> users = await upsert_many(
        ...     "users",
        ...     [
        ...         {"email": "alice@example.com", "name": "Alice"},
        ...         {"email": "bob@example.com", "name": "Bob"},
        ...     ],
        ...     conflict_target="email"
        ... )
        >>> print(len(users))  # 2

    Raises:
        RuntimeError: If PostgreSQL engine is not available or upsert fails
    """
    if _engine is None:
        raise RuntimeError("PostgreSQL engine not available.")

    if isinstance(conflict_target, str):
        conflict_target = [conflict_target]

    return await _engine.upsert_many(table, documents, conflict_target, update_columns)
```

### 5. Python Exports (__init__.py)

**File**: `/Users/chrischeng/projects/data-bridge/python/data_bridge/postgres/__init__.py`

Add upsert imports and exports (check current exports first, add to existing __all__):

```python
from .connection import (
    # ... existing imports ...
    upsert_one,
    upsert_many,
)

__all__ = [
    # ... existing exports ...
    "upsert_one",
    "upsert_many",
]
```

### 6. Integration Tests

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/integration/test_upsert.py` (NEW FILE)

```python
"""Integration tests for PostgreSQL upsert operations."""

import pytest
from data_bridge.postgres import connection as postgres


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_insert_new(pg_connection):
    """Test upsert inserts when no conflict."""
    # Create table with unique constraint
    await postgres.execute("""
        CREATE TABLE test_upsert_users (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Upsert new user
    user = await postgres.upsert_one(
        "test_upsert_users",
        {"email": "alice@example.com", "name": "Alice"},
        conflict_target="email"
    )

    assert user["email"] == "alice@example.com"
    assert user["name"] == "Alice"
    assert user["id"] is not None

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_users")


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_update_existing(pg_connection):
    """Test upsert updates when conflict."""
    # Create table and insert initial data
    await postgres.execute("""
        CREATE TABLE test_upsert_update (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL,
            age INTEGER
        )
    """)

    from data_bridge.postgres.connection import execute, insert_one

    initial = await insert_one("test_upsert_update", {
        "email": "alice@example.com",
        "name": "Alice",
        "age": 25
    })

    # Upsert with same email (should update)
    updated = await postgres.upsert_one(
        "test_upsert_update",
        {"email": "alice@example.com", "name": "Alice Updated", "age": 26},
        conflict_target="email"
    )

    assert updated["id"] == initial["id"]  # Same ID
    assert updated["name"] == "Alice Updated"  # Name updated
    assert updated["age"] == 26  # Age updated

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_update")


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_selective_update(pg_connection):
    """Test upsert only updates specified columns."""
    await postgres.execute("""
        CREATE TABLE test_upsert_selective (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL,
            age INTEGER,
            status VARCHAR(50)
        )
    """)

    from data_bridge.postgres.connection import insert_one

    initial = await insert_one("test_upsert_selective", {
        "email": "alice@example.com",
        "name": "Alice",
        "age": 25,
        "status": "active"
    })

    # Upsert with selective update (only name)
    updated = await postgres.upsert_one(
        "test_upsert_selective",
        {"email": "alice@example.com", "name": "Alice Updated", "age": 30, "status": "inactive"},
        conflict_target="email",
        update_columns=["name"]  # Only update name
    )

    assert updated["id"] == initial["id"]
    assert updated["name"] == "Alice Updated"  # Updated
    assert updated["age"] == 25  # NOT updated (original value)
    assert updated["status"] == "active"  # NOT updated (original value)

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_selective")


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_composite_key(pg_connection):
    """Test upsert with composite conflict target."""
    await postgres.execute("""
        CREATE TABLE test_upsert_composite (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            slug VARCHAR(255) NOT NULL,
            title VARCHAR(255) NOT NULL,
            UNIQUE (user_id, slug)
        )
    """)

    # Insert initial
    from data_bridge.postgres.connection import insert_one
    initial = await insert_one("test_upsert_composite", {
        "user_id": 1,
        "slug": "my-post",
        "title": "Original Title"
    })

    # Upsert with composite key
    updated = await postgres.upsert_one(
        "test_upsert_composite",
        {"user_id": 1, "slug": "my-post", "title": "Updated Title"},
        conflict_target=["user_id", "slug"]
    )

    assert updated["id"] == initial["id"]
    assert updated["title"] == "Updated Title"

    # Insert different slug (should insert new)
    new_post = await postgres.upsert_one(
        "test_upsert_composite",
        {"user_id": 1, "slug": "another-post", "title": "Another Post"},
        conflict_target=["user_id", "slug"]
    )

    assert new_post["id"] != initial["id"]  # Different ID (new row)

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_composite")


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_many(pg_connection):
    """Test batch upsert."""
    await postgres.execute("""
        CREATE TABLE test_upsert_batch (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL,
            age INTEGER
        )
    """)

    # Insert some initial data
    from data_bridge.postgres.connection import insert_many
    await insert_many("test_upsert_batch", [
        {"email": "alice@example.com", "name": "Alice", "age": 25},
        {"email": "charlie@example.com", "name": "Charlie", "age": 35},
    ])

    # Batch upsert (mix of new and existing)
    results = await postgres.upsert_many(
        "test_upsert_batch",
        [
            {"email": "alice@example.com", "name": "Alice Updated", "age": 30},  # Update
            {"email": "bob@example.com", "name": "Bob", "age": 28},  # Insert
            {"email": "charlie@example.com", "name": "Charlie Updated", "age": 40},  # Update
        ],
        conflict_target="email"
    )

    assert len(results) == 3

    # Verify results
    emails = {r["email"]: r for r in results}
    assert emails["alice@example.com"]["name"] == "Alice Updated"
    assert emails["alice@example.com"]["age"] == 30
    assert emails["bob@example.com"]["name"] == "Bob"
    assert emails["charlie@example.com"]["age"] == 40

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_batch")


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_returns_all_columns(pg_connection):
    """Test that upsert returns all columns including auto-generated."""
    await postgres.execute("""
        CREATE TABLE test_upsert_returning (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
    """)

    # Upsert
    user = await postgres.upsert_one(
        "test_upsert_returning",
        {"email": "alice@example.com", "name": "Alice"},
        conflict_target="email"
    )

    assert "id" in user
    assert "email" in user
    assert "name" in user
    assert "created_at" in user  # Auto-generated column

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_returning")


@pytest.mark.asyncio
@pytest.mark.integration
async def test_upsert_error_handling(pg_connection):
    """Test upsert error handling."""
    await postgres.execute("""
        CREATE TABLE test_upsert_errors (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Test with empty conflict target
    with pytest.raises(Exception):  # Should raise error
        await postgres.upsert_one(
            "test_upsert_errors",
            {"email": "alice@example.com", "name": "Alice"},
            conflict_target=[]
        )

    # Test with invalid conflict target (non-existent column)
    # This will fail at database level - PostgreSQL will report error
    with pytest.raises(Exception):
        await postgres.upsert_one(
            "test_upsert_errors",
            {"email": "alice@example.com", "name": "Alice"},
            conflict_target="nonexistent_column"
        )

    # Cleanup
    await postgres.execute("DROP TABLE test_upsert_errors")
```

### Test Fixture

Add to `/Users/chrischeng/projects/data-bridge/tests/postgres/conftest.py` (or create if doesn't exist):

```python
import pytest
from data_bridge.postgres import connection as postgres


@pytest.fixture
async def pg_connection():
    """Fixture to ensure PostgreSQL connection for integration tests."""
    if not postgres.is_connected():
        await postgres.init(
            connection_string="postgresql://localhost:5432/test_db",
            max_connections=5
        )

    yield

    # Cleanup is handled by individual tests
```

## Implementation Order

1. Start with QueryBuilder (`query.rs`) - build_upsert method and tests
2. Add Row operations (`row.rs`) - upsert and upsert_many
3. Add PyO3 bindings (`postgres.rs`) - Python function bindings
4. Add Python wrappers (`connection.py`) - async functions
5. Update exports (`__init__.py`)
6. Create integration tests (`test_upsert.py`)
7. Run all tests and verify

## Testing

After implementation:

```bash
# Build
cd /Users/chrischeng/projects/data-bridge
maturin develop

# Rust tests
cargo test -p data-bridge-postgres test_upsert

# Python integration tests (requires PostgreSQL running)
uv run pytest tests/postgres/integration/test_upsert.py -v

# Clippy
cargo clippy --all-targets
```

## Expected Outcome

- All Rust unit tests pass (6 tests in query.rs)
- All Python integration tests pass (7 tests in test_upsert.py)
- No clippy warnings
- Functions callable from Python:
  - `await postgres.upsert_one(...)`
  - `await postgres.upsert_many(...)`
