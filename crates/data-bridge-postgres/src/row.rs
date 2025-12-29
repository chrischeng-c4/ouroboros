//! PostgreSQL row representation.
//!
//! This module provides a row abstraction for query results,
//! similar to data-bridge-mongodb's document handling.
//!
//! # Examples
//!
//! ## Insert a row
//!
//! ```ignore
//! use data_bridge_postgres::{Connection, ExtractedValue, PoolConfig, Row};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let conn = Connection::new("postgresql://localhost/mydb", PoolConfig::default()).await?;
//! let pool = conn.pool();
//!
//! let values = vec![
//!     ("name".to_string(), ExtractedValue::String("Alice".to_string())),
//!     ("age".to_string(), ExtractedValue::Int(30)),
//! ];
//!
//! let row = Row::insert(pool, "users", &values).await?;
//! let id = row.get("id")?; // Auto-generated ID
//! # Ok(())
//! # }
//! ```
//!
//! ## Find rows
//!
//! ```ignore
//! use data_bridge_postgres::{QueryBuilder, Operator, ExtractedValue, Row};
//!
//! # async fn example(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
//! // Find by ID
//! let row = Row::find_by_id(pool, "users", 42).await?;
//!
//! // Find with filters
//! let query = QueryBuilder::new("users")?
//!     .where_clause("age", Operator::Gte, ExtractedValue::Int(18))?
//!     .limit(10);
//! let rows = Row::find_many(pool, "users", Some(&query)).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Batch insert
//!
//! ```ignore
//! use std::collections::HashMap;
//! use data_bridge_postgres::{Connection, ExtractedValue, PoolConfig, Row};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let conn = Connection::new("postgresql://localhost/mydb", PoolConfig::default()).await?;
//! let pool = conn.pool();
//!
//! let mut row1 = HashMap::new();
//! row1.insert("name".to_string(), ExtractedValue::String("Alice".to_string()));
//! row1.insert("age".to_string(), ExtractedValue::Int(30));
//!
//! let mut row2 = HashMap::new();
//! row2.insert("name".to_string(), ExtractedValue::String("Bob".to_string()));
//! row2.insert("age".to_string(), ExtractedValue::Int(25));
//!
//! // Batch insert is much faster than individual inserts
//! let rows = Row::insert_many(pool, "users", &[row1, row2]).await?;
//! assert_eq!(rows.len(), 2);
//! # Ok(())
//! # }
//! ```
//!
//! ## Update and delete
//!
//! ```ignore
//! use data_bridge_postgres::{ExtractedValue, Row};
//!
//! # async fn example(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
//! // Update
//! let updates = vec![
//!     ("name".to_string(), ExtractedValue::String("Bob".to_string())),
//! ];
//! Row::update(pool, "users", 42, &updates).await?;
//!
//! // Delete
//! Row::delete(pool, "users", 42).await?;
//! # Ok(())
//! # }
//! ```

use serde_json::Value as JsonValue;
use sqlx::postgres::{PgArguments, PgPool};
use sqlx::Row as SqlxRow;
use std::collections::HashMap;

use crate::{DataBridgeError, ExtractedValue, QueryBuilder, Result, row_to_extracted};
use crate::query::{JoinType, Operator, OrderDirection};

/// Relation configuration for eager loading
#[derive(Debug, Clone)]
pub struct RelationConfig {
    /// Name of the relation (used as key in result)
    pub name: String,
    /// Table to join
    pub table: String,
    /// Column in main table that references the foreign table
    pub foreign_key: String,
    /// Column in foreign table being referenced (usually "id")
    pub reference_column: String,
    /// Type of join (usually Left for optional relations)
    pub join_type: JoinType,
    /// Columns to select from the related table (None = all)
    pub select_columns: Option<Vec<String>>,
}

/// Represents a single row from a PostgreSQL query result.
///
/// This is the primary data structure returned from queries.
/// It wraps column names and values in a type-safe manner.
#[derive(Debug, Clone)]
pub struct Row {
    /// Column name to value mapping
    columns: HashMap<String, ExtractedValue>,
}

impl Row {
    /// Creates a new row from a column map.
    pub fn new(columns: HashMap<String, ExtractedValue>) -> Self {
        Self { columns }
    }

    /// Gets a value by column name.
    ///
    /// # Arguments
    ///
    /// * `column` - Column name
    ///
    /// # Errors
    ///
    /// Returns error if column doesn't exist.
    pub fn get(&self, column: &str) -> Result<&ExtractedValue> {
        self.columns
            .get(column)
            .ok_or_else(|| DataBridgeError::Query(format!("Column '{}' not found", column)))
    }

    /// Gets all column names.
    pub fn columns(&self) -> Vec<&str> {
        self.columns.keys().map(|s| s.as_str()).collect()
    }

    /// Gets a reference to the column map.
    pub fn columns_map(&self) -> &HashMap<String, ExtractedValue> {
        &self.columns
    }

    /// Converts row to a JSON object.
    pub fn to_json(&self) -> Result<JsonValue> {
        let mut map = serde_json::Map::new();
        for (key, value) in &self.columns {
            let json_value = extracted_value_to_json(value)?;
            map.insert(key.clone(), json_value);
        }
        Ok(JsonValue::Object(map))
    }

    /// Converts row to Python dict.
    pub fn to_python(/* &self, py: Python */) -> Result<()> {
        // TODO: Implement row to Python dict conversion
        // - Create Python dict
        // - For each column, convert ExtractedValue to Python object
        // - Set dict items
        // - Return PyDict
        todo!("Implement Row::to_python - requires PyO3 integration")
    }

    /// Converts from SQLx row.
    pub fn from_sqlx(row: &sqlx::postgres::PgRow) -> Result<Self> {
        let columns = row_to_extracted(row)?;
        Ok(Self { columns })
    }

    /// Insert row into database, return generated ID.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `values` - Column name -> value mapping
    ///
    /// # Errors
    ///
    /// Returns error if insert fails or table is invalid.
    ///
    /// # Returns
    ///
    /// Returns the inserted row with all columns (including generated ID).
    pub async fn insert(
        pool: &PgPool,
        table: &str,
        values: &[(String, ExtractedValue)],
    ) -> Result<Self> {
        if values.is_empty() {
            return Err(DataBridgeError::Query("Cannot insert with no values".to_string()));
        }

        // Build INSERT query with RETURNING *
        let query_builder = QueryBuilder::new(table)?;
        let (sql, params) = query_builder.build_insert(values)?;

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query with bound arguments
        let row = sqlx::query_with(&sql, args)
            .fetch_one(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Insert failed: {}", e)))?;

        // Convert PgRow to Row
        Self::from_sqlx(&row)
    }

    /// Insert multiple rows with a single batch INSERT statement.
    ///
    /// This is much faster than individual inserts for large batches because
    /// it generates a single INSERT with multiple VALUES clauses:
    /// `INSERT INTO table (col1, col2) VALUES ($1, $2), ($3, $4), ... RETURNING *`
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `rows` - Vector of rows, where each row is a HashMap of column -> value
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Any row is empty
    /// - Rows have different columns
    /// - Insert fails
    /// - Table is invalid
    ///
    /// # Returns
    ///
    /// Returns vector of inserted rows with all columns (including generated IDs).
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
    /// row1.insert("name".to_string(), ExtractedValue::String("Alice".to_string()));
    /// row1.insert("age".to_string(), ExtractedValue::Int(30));
    ///
    /// let mut row2 = HashMap::new();
    /// row2.insert("name".to_string(), ExtractedValue::String("Bob".to_string()));
    /// row2.insert("age".to_string(), ExtractedValue::Int(25));
    ///
    /// let rows = Row::insert_many(pool, "users", &[row1, row2]).await?;
    /// assert_eq!(rows.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert_many(
        pool: &PgPool,
        table: &str,
        rows: &[HashMap<String, ExtractedValue>],
    ) -> Result<Vec<Self>> {
        if rows.is_empty() {
            return Ok(vec![]);
        }

        // Get column names from first row and validate
        let first_row = &rows[0];
        if first_row.is_empty() {
            return Err(DataBridgeError::Query("Cannot insert with no columns".to_string()));
        }

        // Collect and sort column names for consistent ordering
        let mut column_names: Vec<&String> = first_row.keys().collect();
        column_names.sort();

        // Validate all rows have the same columns
        for (idx, row) in rows.iter().enumerate().skip(1) {
            let mut row_columns: Vec<&String> = row.keys().collect();
            row_columns.sort();
            if row_columns != column_names {
                return Err(DataBridgeError::Query(format!(
                    "Row {} has different columns than first row. Expected: {:?}, Got: {:?}",
                    idx, column_names, row_columns
                )));
            }
        }

        // Validate table name
        let _query_builder = QueryBuilder::new(table)?;

        // Validate column names
        for col in &column_names {
            QueryBuilder::validate_identifier(col)?;
        }

        // Build batch INSERT SQL
        // INSERT INTO table (col1, col2) VALUES ($1, $2), ($3, $4), ... RETURNING *
        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ",
            table,
            column_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        );

        let mut param_num = 1;
        let mut values_clauses = Vec::with_capacity(rows.len());
        let mut params = Vec::with_capacity(rows.len() * column_names.len());

        for row in rows {
            let placeholders: Vec<String> = (0..column_names.len())
                .map(|_| {
                    let p = format!("${}", param_num);
                    param_num += 1;
                    p
                })
                .collect();
            values_clauses.push(format!("({})", placeholders.join(", ")));

            // Collect parameter values in the same order as column names
            for col in &column_names {
                params.push(row.get(*col).unwrap().clone());
            }
        }

        sql.push_str(&values_clauses.join(", "));
        sql.push_str(" RETURNING *");

        // Bind all parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query and fetch all returned rows
        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Batch insert failed: {}", e)))?;

        // Convert all PgRows to Rows
        pg_rows.iter()
            .map(Self::from_sqlx)
            .collect()
    }

    /// Upsert a single row (INSERT ON CONFLICT UPDATE).
    ///
    /// This performs an "upsert" operation: if the row conflicts with an existing row
    /// (based on the conflict_target constraint), it will update; otherwise it inserts.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `values` - Column values to insert/update
    /// * `conflict_target` - Columns for ON CONFLICT clause (must match unique constraint)
    /// * `update_columns` - Optional columns to update on conflict (None = all except conflict_target)
    ///
    /// # Returns
    ///
    /// Returns the inserted or updated row with all columns (including generated values).
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
    /// This generates a multi-row INSERT with ON CONFLICT for efficient batch upserts:
    /// `INSERT INTO table (cols) VALUES ($1,$2),($3,$4),... ON CONFLICT (...) DO UPDATE ... RETURNING *`
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `rows` - Vector of rows (HashMaps of column -> value)
    /// * `conflict_target` - Columns for ON CONFLICT clause
    /// * `update_columns` - Optional columns to update on conflict (None = all except conflict_target)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Any row is empty
    /// - Rows have different columns
    /// - Conflict target is empty
    /// - Upsert fails
    /// - Table is invalid
    ///
    /// # Returns
    ///
    /// Returns vector of inserted/updated rows with all columns.
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

        // Execute query and fetch all returned rows
        let pg_rows = sqlx::query_with(&sql, args)
            .fetch_all(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Batch upsert failed: {}", e)))?;

        // Convert all PgRows to Rows
        pg_rows.iter()
            .map(Self::from_sqlx)
            .collect()
    }

    /// Find single row by primary key.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `id` - Primary key value
    ///
    /// # Errors
    ///
    /// Returns error if query fails.
    ///
    /// # Returns
    ///
    /// Returns Some(Row) if found, None if not found.
    pub async fn find_by_id(pool: &PgPool, table: &str, id: i64) -> Result<Option<Self>> {
        // Build SELECT * WHERE id = $1 query
        let query_builder = QueryBuilder::new(table)?
            .where_clause("id", crate::Operator::Eq, ExtractedValue::BigInt(id))?;
        let (sql, params) = query_builder.build_select();

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .fetch_optional(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Find by ID failed: {}", e)))?;

        match result {
            Some(row) => Ok(Some(Self::from_sqlx(&row)?)),
            None => Ok(None),
        }
    }

    /// Find multiple rows with filters.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `query` - Query builder with filters (optional)
    ///
    /// # Errors
    ///
    /// Returns error if query fails.
    ///
    /// # Returns
    ///
    /// Returns vector of matching rows.
    pub async fn find_many(
        pool: &PgPool,
        table: &str,
        query: Option<&QueryBuilder>,
    ) -> Result<Vec<Self>> {
        let (sql, params) = if let Some(qb) = query {
            qb.build_select()
        } else {
            let qb = QueryBuilder::new(table)?;
            qb.build_select()
        };

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let rows = sqlx::query_with(&sql, args)
            .fetch_all(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Find many failed: {}", e)))?;

        // Convert all PgRows to Rows
        rows.iter()
            .map(Self::from_sqlx)
            .collect()
    }

    /// Update row in database.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `id` - Primary key value
    /// * `values` - Column name -> value mapping for updates
    ///
    /// # Errors
    ///
    /// Returns error if update fails.
    ///
    /// # Returns
    ///
    /// Returns true if row was updated, false if not found.
    pub async fn update(
        pool: &PgPool,
        table: &str,
        id: i64,
        values: &[(String, ExtractedValue)],
    ) -> Result<bool> {
        if values.is_empty() {
            return Err(DataBridgeError::Query("Cannot update with no values".to_string()));
        }

        // Build UPDATE SET ... WHERE id = $N query
        let query_builder = QueryBuilder::new(table)?
            .where_clause("id", crate::Operator::Eq, ExtractedValue::BigInt(id))?;
        let (sql, params) = query_builder.build_update(values)?;

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .execute(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Update failed: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete row from database.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `id` - Primary key value
    ///
    /// # Errors
    ///
    /// Returns error if delete fails.
    ///
    /// # Returns
    ///
    /// Returns true if row was deleted, false if not found.
    pub async fn delete(pool: &PgPool, table: &str, id: i64) -> Result<bool> {
        // Build DELETE WHERE id = $1 query
        let query_builder = QueryBuilder::new(table)?
            .where_clause("id", crate::Operator::Eq, ExtractedValue::BigInt(id))?;
        let (sql, params) = query_builder.build_delete();

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let result = sqlx::query_with(&sql, args)
            .execute(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Delete failed: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    /// Count rows matching query.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `query` - Query builder with filters (optional)
    ///
    /// # Errors
    ///
    /// Returns error if query fails.
    ///
    /// # Returns
    ///
    /// Returns count of matching rows.
    pub async fn count(
        pool: &PgPool,
        table: &str,
        query: Option<&QueryBuilder>,
    ) -> Result<i64> {
        // Build SELECT COUNT(*) query
        let mut sql = format!("SELECT COUNT(*) FROM {}", table);
        let mut params = Vec::new();

        if let Some(qb) = query {
            // Extract WHERE clause from the SELECT query
            let (select_sql, select_params) = qb.build_select();
            params = select_params;

            // Find WHERE clause in the generated SQL
            if let Some(where_pos) = select_sql.find(" WHERE ") {
                let where_clause = &select_sql[where_pos..];
                // Find the end of WHERE clause (before ORDER BY, LIMIT, etc.)
                let end_pos = where_clause
                    .find(" ORDER BY ")
                    .or_else(|| where_clause.find(" LIMIT "))
                    .or_else(|| where_clause.find(" OFFSET "))
                    .unwrap_or(where_clause.len());
                sql.push_str(&where_clause[..end_pos]);
            }
        }

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let row = sqlx::query_with(&sql, args)
            .fetch_one(pool)
            .await
            .map_err(|e| DataBridgeError::Query(format!("Count failed: {}", e)))?;

        let count: i64 = row.try_get(0)
            .map_err(|e| DataBridgeError::Query(format!("Failed to extract count: {}", e)))?;

        Ok(count)
    }

    /// Fetch a single row with related data using JOINs.
    ///
    /// Returns a Row where related data is nested under the relation name.
    /// This eliminates N+1 queries by fetching everything in one query.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Main table name
    /// * `id` - Primary key value
    /// * `relations` - Configuration for relations to eager load
    ///
    /// # Errors
    ///
    /// Returns error if query fails or table is invalid.
    ///
    /// # Returns
    ///
    /// Returns Some(Row) with nested relation data if found, None if not found.
    pub async fn find_with_relations(
        pool: &PgPool,
        table: &str,
        id: i64,
        relations: &[RelationConfig],
    ) -> Result<Option<Self>> {
        // Build SELECT columns: main table + relations
        let mut select_cols = vec![format!("\"{}\".*", table)];

        for (idx, rel) in relations.iter().enumerate() {
            let alias = format!("_rel{}", idx);
            match &rel.select_columns {
                Some(cols) => {
                    for col in cols {
                        select_cols.push(format!("\"{}\".\"{}\" AS \"{}__{}\"", alias, col, rel.name, col));
                    }
                }
                None => {
                    // Select all columns with prefix using row_to_json
                    select_cols.push(format!("row_to_json(\"{}\") AS \"{}__data\"", alias, rel.name));
                }
            }
        }

        // Build query with JOINs
        let mut qb = QueryBuilder::new(table)?;

        for (idx, rel) in relations.iter().enumerate() {
            let alias = format!("_rel{}", idx);
            let on_condition = format!(
                "\"{}\".\"{}\" = \"{}\".\"{}\"",
                table, rel.foreign_key,
                alias, rel.reference_column
            );

            qb = qb.join(rel.join_type.clone(), &rel.table, Some(&alias), &on_condition)?;
        }

        // Qualify the id column with the table name to avoid ambiguity with JOINs
        let qualified_id_col = format!("{}.id", table);
        qb = qb.where_clause(&qualified_id_col, Operator::Eq, ExtractedValue::BigInt(id))?;

        let (sql, params) = qb.build_select();

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let row = sqlx::query_with(&sql, args)
            .fetch_optional(pool)
            .await
            .map_err(|e| DataBridgeError::Database(e.to_string()))?;

        match row {
            Some(pg_row) => {
                let mut result = Self::from_sqlx(&pg_row)?;

                // Extract relation data from prefixed columns
                for rel in relations {
                    let prefix = format!("{}__", rel.name);
                    let mut rel_data = serde_json::Map::new();

                    // Get columns that match this relation's prefix
                    let keys_to_process: Vec<String> = result.columns
                        .keys()
                        .filter(|k| k.starts_with(&prefix))
                        .cloned()
                        .collect();

                    for key in keys_to_process {
                        if let Some(value) = result.columns.remove(&key) {
                            let rel_key = key.strip_prefix(&prefix).unwrap().to_string();
                            let json_value = extracted_value_to_json(&value)?;
                            rel_data.insert(rel_key, json_value);
                        }
                    }

                    if !rel_data.is_empty() {
                        // Store as JSON object
                        result.columns.insert(rel.name.clone(), ExtractedValue::Json(JsonValue::Object(rel_data)));
                    }
                }

                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    /// Fetch multiple rows with related data using JOINs.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Main table name
    /// * `relations` - Configuration for relations to eager load
    /// * `where_clause` - Optional WHERE condition
    /// * `order_by` - Optional ORDER BY clause
    /// * `limit` - Optional LIMIT
    /// * `offset` - Optional OFFSET
    ///
    /// # Errors
    ///
    /// Returns error if query fails or table is invalid.
    ///
    /// # Returns
    ///
    /// Returns vector of rows with nested relation data.
    pub async fn find_many_with_relations(
        pool: &PgPool,
        table: &str,
        relations: &[RelationConfig],
        where_clause: Option<(&str, Operator, ExtractedValue)>,
        order_by: Option<(&str, OrderDirection)>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Self>> {
        let mut qb = QueryBuilder::new(table)?;

        // Add JOINs
        for (idx, rel) in relations.iter().enumerate() {
            let alias = format!("_rel{}", idx);
            let on_condition = format!(
                "\"{}\".\"{}\" = \"{}\".\"{}\"",
                table, rel.foreign_key,
                alias, rel.reference_column
            );

            qb = qb.join(rel.join_type.clone(), &rel.table, Some(&alias), &on_condition)?;
        }

        // Add WHERE if provided - qualify column with table name to avoid ambiguity
        if let Some((col, op, val)) = where_clause {
            // If column doesn't already contain a dot (not already qualified), qualify it
            let qualified_col = if col.contains('.') {
                col.to_string()
            } else {
                format!("{}.{}", table, col)
            };
            qb = qb.where_clause(&qualified_col, op, val)?;
        }

        // Add ORDER BY if provided - qualify column with table name to avoid ambiguity
        if let Some((col, dir)) = order_by {
            // If column doesn't already contain a dot (not already qualified), qualify it
            let qualified_col = if col.contains('.') {
                col.to_string()
            } else {
                format!("{}.{}", table, col)
            };
            qb = qb.order_by(&qualified_col, dir)?;
        }

        // Add LIMIT/OFFSET
        if let Some(l) = limit {
            qb = qb.limit(l);
        }
        if let Some(o) = offset {
            qb = qb.offset(o);
        }

        let (sql, params) = qb.build_select();

        // Bind parameters
        let mut args = PgArguments::default();
        for param in &params {
            param.bind_to_arguments(&mut args)?;
        }

        // Execute query
        let rows = sqlx::query_with(&sql, args)
            .fetch_all(pool)
            .await
            .map_err(|e| DataBridgeError::Database(e.to_string()))?;

        let mut results = Vec::with_capacity(rows.len());
        for pg_row in rows {
            results.push(Self::from_sqlx(&pg_row)?);
        }

        Ok(results)
    }

    /// Simple eager loading helper - fetches with LEFT JOINs.
    ///
    /// This is a convenience wrapper around `find_with_relations` for common cases.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Main table name
    /// * `id` - Primary key value
    /// * `joins` - Tuples of (relation_name, fk_column, ref_table)
    ///
    /// # Errors
    ///
    /// Returns error if query fails or table is invalid.
    ///
    /// # Returns
    ///
    /// Returns Some(Row) with nested relation data if found, None if not found.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Load user with their posts and comments
    /// let user = Row::find_one_eager(
    ///     pool,
    ///     "users",
    ///     42,
    ///     &[
    ///         ("posts", "user_id", "posts"),
    ///         ("comments", "user_id", "comments"),
    ///     ]
    /// ).await?;
    /// ```
    pub async fn find_one_eager(
        pool: &PgPool,
        table: &str,
        id: i64,
        joins: &[(&str, &str, &str)],  // (relation_name, fk_column, ref_table)
    ) -> Result<Option<Self>> {
        let relations: Vec<RelationConfig> = joins
            .iter()
            .map(|(name, fk, ref_table)| RelationConfig {
                name: name.to_string(),
                table: ref_table.to_string(),
                foreign_key: fk.to_string(),
                reference_column: "id".to_string(),
                join_type: JoinType::Left,
                select_columns: None,
            })
            .collect();

        Self::find_with_relations(pool, table, id, &relations).await
    }

    /// Delete a row with cascade handling based on foreign key rules.
    /// This method respects ON DELETE rules from child tables.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `id` - Primary key value
    /// * `id_column` - Name of the primary key column (default: "id")
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Delete fails
    /// - RESTRICT constraint prevents deletion
    /// - Table or column names are invalid
    ///
    /// # Returns
    ///
    /// Returns total number of rows deleted (including cascaded deletions).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Delete a user and cascade to related records
    /// let deleted_count = Row::delete_with_cascade(pool, "users", 42, "id").await?;
    /// println!("Deleted {} rows total", deleted_count);
    /// ```
    pub async fn delete_with_cascade(
        pool: &PgPool,
        table: &str,
        id: i64,
        id_column: &str,
    ) -> Result<u64> {
        use crate::schema::CascadeRule;

        // Validate identifiers
        QueryBuilder::validate_identifier(table)?;
        QueryBuilder::validate_identifier(id_column)?;

        // Start a transaction
        let mut tx = pool.begin().await.map_err(|e| DataBridgeError::Database(e.to_string()))?;

        // Get all back-references (tables that reference this table)
        let backrefs = Self::get_backreferences_internal(&mut *tx, table).await?;

        let mut total_deleted = 0u64;

        for backref in &backrefs {
            match backref.on_delete {
                CascadeRule::Restrict | CascadeRule::NoAction => {
                    // Check if any child rows exist
                    let check_query = format!(
                        "SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE \"{}\" = $1) as has_children",
                        backref.source_table, backref.source_column
                    );
                    let row: (bool,) = sqlx::query_as(&check_query)
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(|e| DataBridgeError::Database(e.to_string()))?;

                    if row.0 {
                        tx.rollback().await.map_err(|e| DataBridgeError::Database(e.to_string()))?;
                        return Err(DataBridgeError::Validation(format!(
                            "Cannot delete from '{}': referenced by '{}' ({})",
                            table, backref.source_table, backref.constraint_name
                        )));
                    }
                }
                CascadeRule::Cascade => {
                    // Recursively delete child rows
                    let delete_children = format!(
                        "DELETE FROM \"{}\" WHERE \"{}\" = $1",
                        backref.source_table, backref.source_column
                    );
                    let result = sqlx::query(&delete_children)
                        .bind(id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| DataBridgeError::Database(e.to_string()))?;
                    total_deleted += result.rows_affected();
                }
                CascadeRule::SetNull => {
                    // Set foreign key to NULL
                    let update_query = format!(
                        "UPDATE \"{}\" SET \"{}\" = NULL WHERE \"{}\" = $1",
                        backref.source_table, backref.source_column, backref.source_column
                    );
                    sqlx::query(&update_query)
                        .bind(id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| DataBridgeError::Database(e.to_string()))?;
                }
                CascadeRule::SetDefault => {
                    // Set foreign key to DEFAULT
                    let update_query = format!(
                        "UPDATE \"{}\" SET \"{}\" = DEFAULT WHERE \"{}\" = $1",
                        backref.source_table, backref.source_column, backref.source_column
                    );
                    sqlx::query(&update_query)
                        .bind(id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| DataBridgeError::Database(e.to_string()))?;
                }
            }
        }

        // Delete the parent row
        let delete_query = format!(
            "DELETE FROM \"{}\" WHERE \"{}\" = $1",
            table, id_column
        );
        let result = sqlx::query(&delete_query)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DataBridgeError::Database(e.to_string()))?;
        total_deleted += result.rows_affected();

        // Commit transaction
        tx.commit().await.map_err(|e| DataBridgeError::Database(e.to_string()))?;

        Ok(total_deleted)
    }

    /// Delete a row, checking for RESTRICT constraints.
    /// For CASCADE, relies on database-level ON DELETE CASCADE.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `table` - Table name
    /// * `id` - Primary key value
    /// * `id_column` - Name of the primary key column (default: "id")
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - RESTRICT constraint prevents deletion
    /// - Delete fails
    /// - Table or column names are invalid
    ///
    /// # Returns
    ///
    /// Returns number of rows deleted (database handles cascades).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Delete a user, checking RESTRICT but letting DB handle CASCADE
    /// let deleted = Row::delete_checked(pool, "users", 42, "id").await?;
    /// ```
    pub async fn delete_checked(
        pool: &PgPool,
        table: &str,
        id: i64,
        id_column: &str,
    ) -> Result<u64> {
        use crate::schema::CascadeRule;

        QueryBuilder::validate_identifier(table)?;
        QueryBuilder::validate_identifier(id_column)?;

        // Get back-references with RESTRICT rule
        let backrefs = Self::get_backreferences_internal(pool, table).await?;

        for backref in &backrefs {
            if matches!(backref.on_delete, CascadeRule::Restrict | CascadeRule::NoAction) {
                // Check if any child rows exist
                let check_query = format!(
                    "SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE \"{}\" = $1) as has_children",
                    backref.source_table, backref.source_column
                );
                let row: (bool,) = sqlx::query_as(&check_query)
                    .bind(id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?;

                if row.0 {
                    return Err(DataBridgeError::Validation(format!(
                        "Cannot delete from '{}': referenced by '{}' via column '{}' (constraint: {})",
                        table, backref.source_table, backref.source_column, backref.constraint_name
                    )));
                }
            }
        }

        // Perform the actual delete (database handles CASCADE)
        let query = format!(
            "DELETE FROM \"{}\" WHERE \"{}\" = $1",
            table, id_column
        );

        let result = sqlx::query(&query)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| DataBridgeError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Internal helper to get back-references without requiring a SchemaInspector instance.
    /// This queries the database directly for foreign key relationships.
    async fn get_backreferences_internal<'a, E>(
        executor: E,
        table: &str,
    ) -> Result<Vec<crate::schema::BackRef>>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        use crate::schema::{BackRef, CascadeRule};

        let query = r#"
            SELECT
                tc.table_name as source_table,
                kcu.column_name as source_column,
                ccu.table_name as target_table,
                ccu.column_name as target_column,
                tc.constraint_name,
                rc.delete_rule,
                rc.update_rule
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            JOIN information_schema.constraint_column_usage ccu
                ON ccu.constraint_name = tc.constraint_name
                AND ccu.table_schema = tc.table_schema
            JOIN information_schema.referential_constraints rc
                ON rc.constraint_name = tc.constraint_name
                AND rc.constraint_schema = tc.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
                AND ccu.table_name = $1
                AND tc.table_schema = $2
        "#;

        let rows = sqlx::query(query)
            .bind(table)
            .bind("public")
            .fetch_all(executor)
            .await
            .map_err(|e| DataBridgeError::Database(e.to_string()))?;

        let mut backrefs = Vec::new();
        for row in rows {
            backrefs.push(BackRef {
                source_table: row.try_get("source_table")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?,
                source_column: row.try_get("source_column")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?,
                target_table: row.try_get("target_table")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?,
                target_column: row.try_get("target_column")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?,
                constraint_name: row.try_get("constraint_name")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?,
                on_delete: CascadeRule::from_sql(&row.try_get::<String, _>("delete_rule")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?),
                on_update: CascadeRule::from_sql(&row.try_get::<String, _>("update_rule")
                    .map_err(|e| DataBridgeError::Database(e.to_string()))?),
            });
        }

        Ok(backrefs)
    }
}

/// Helper function to convert ExtractedValue to JSON.
fn extracted_value_to_json(value: &ExtractedValue) -> Result<JsonValue> {
    Ok(match value {
        ExtractedValue::Null => JsonValue::Null,
        ExtractedValue::Bool(v) => JsonValue::Bool(*v),
        ExtractedValue::SmallInt(v) => JsonValue::Number((*v).into()),
        ExtractedValue::Int(v) => JsonValue::Number((*v).into()),
        ExtractedValue::BigInt(v) => JsonValue::Number((*v).into()),
        ExtractedValue::Float(v) => {
            serde_json::Number::from_f64(*v as f64)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }
        ExtractedValue::Double(v) => {
            serde_json::Number::from_f64(*v)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }
        ExtractedValue::String(v) => JsonValue::String(v.clone()),
        ExtractedValue::Bytes(v) => {
            // Encode bytes as hex string
            let hex_string = v.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            JsonValue::String(hex_string)
        }
        ExtractedValue::Uuid(v) => JsonValue::String(v.to_string()),
        ExtractedValue::Date(v) => JsonValue::String(v.to_string()),
        ExtractedValue::Time(v) => JsonValue::String(v.to_string()),
        ExtractedValue::Timestamp(v) => JsonValue::String(v.to_string()),
        ExtractedValue::TimestampTz(v) => JsonValue::String(v.to_rfc3339()),
        ExtractedValue::Json(v) => v.clone(),
        ExtractedValue::Array(values) => {
            let json_values: Vec<JsonValue> = values
                .iter()
                .map(extracted_value_to_json)
                .collect::<Result<Vec<_>>>()?;
            JsonValue::Array(json_values)
        }
        ExtractedValue::Decimal(v) => JsonValue::String(v.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_row_data_creation() {
        // Test creating Row with various field types
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(42));
        columns.insert("name".to_string(), ExtractedValue::String("Alice".to_string()));
        columns.insert("age".to_string(), ExtractedValue::Int(30));
        columns.insert("active".to_string(), ExtractedValue::Bool(true));
        columns.insert("score".to_string(), ExtractedValue::Double(98.5));

        let row = Row::new(columns.clone());

        // Verify all fields are accessible
        assert!(matches!(row.get("id"), Ok(ExtractedValue::BigInt(42))));
        assert!(matches!(row.get("name"), Ok(ExtractedValue::String(s)) if s == "Alice"));
        assert!(matches!(row.get("age"), Ok(ExtractedValue::Int(30))));
        assert!(matches!(row.get("active"), Ok(ExtractedValue::Bool(true))));
        assert!(matches!(row.get("score"), Ok(ExtractedValue::Double(s)) if (s - 98.5).abs() < 0.001));

        // Verify columns map
        assert_eq!(row.columns_map(), &columns);
    }

    #[test]
    fn test_row_data_with_null_fields() {
        // Test Row with NULL values
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(1));
        columns.insert("name".to_string(), ExtractedValue::String("Bob".to_string()));
        columns.insert("email".to_string(), ExtractedValue::Null);
        columns.insert("phone".to_string(), ExtractedValue::Null);
        columns.insert("age".to_string(), ExtractedValue::Int(25));

        let row = Row::new(columns);

        // Verify NULL fields
        assert!(matches!(row.get("email"), Ok(ExtractedValue::Null)));
        assert!(matches!(row.get("phone"), Ok(ExtractedValue::Null)));

        // Verify non-NULL fields still work
        assert!(matches!(row.get("id"), Ok(ExtractedValue::BigInt(1))));
        assert!(matches!(row.get("name"), Ok(ExtractedValue::String(s)) if s == "Bob"));
        assert!(matches!(row.get("age"), Ok(ExtractedValue::Int(25))));
    }

    #[test]
    fn test_row_data_field_access() {
        // Test accessing fields by name
        let mut columns = HashMap::new();
        columns.insert("user_id".to_string(), ExtractedValue::BigInt(100));
        columns.insert("username".to_string(), ExtractedValue::String("charlie".to_string()));
        columns.insert("balance".to_string(), ExtractedValue::Decimal("1234.56".to_string()));

        let row = Row::new(columns);

        // Test successful field access
        let user_id = row.get("user_id");
        assert!(user_id.is_ok());
        assert!(matches!(user_id.unwrap(), ExtractedValue::BigInt(100)));

        let username = row.get("username");
        assert!(username.is_ok());
        assert!(matches!(username.unwrap(), ExtractedValue::String(s) if s == "charlie"));

        // Test accessing non-existent field
        let result = row.get("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Column 'nonexistent' not found"));

        // Test columns() method
        let column_names = row.columns();
        assert_eq!(column_names.len(), 3);
        assert!(column_names.contains(&"user_id"));
        assert!(column_names.contains(&"username"));
        assert!(column_names.contains(&"balance"));
    }

    #[test]
    fn test_row_data_clone() {
        // Test Clone trait
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(42));
        columns.insert("data".to_string(), ExtractedValue::String("original".to_string()));

        let row1 = Row::new(columns);
        let row2 = row1.clone();

        // Verify both rows have the same data
        assert!(matches!(row1.get("id"), Ok(ExtractedValue::BigInt(42))));
        assert!(matches!(row2.get("id"), Ok(ExtractedValue::BigInt(42))));

        assert!(matches!(row1.get("data"), Ok(ExtractedValue::String(s)) if s == "original"));
        assert!(matches!(row2.get("data"), Ok(ExtractedValue::String(s)) if s == "original"));

        // Verify they are independent (modifying one shouldn't affect the other)
        assert_eq!(row1.columns_map().len(), row2.columns_map().len());
    }

    #[test]
    fn test_row_data_debug() {
        // Test Debug trait
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(1));
        columns.insert("name".to_string(), ExtractedValue::String("test".to_string()));

        let row = Row::new(columns);
        let debug_str = format!("{:?}", row);

        // Verify debug output contains expected information
        assert!(debug_str.contains("Row"));
        assert!(debug_str.contains("columns"));
    }

    #[test]
    fn test_insert_params_extraction() {
        // Test extracting insert parameters from values
        let values = vec![
            ("name".to_string(), ExtractedValue::String("Alice".to_string())),
            ("age".to_string(), ExtractedValue::Int(30)),
            ("active".to_string(), ExtractedValue::Bool(true)),
        ];

        // Verify values are properly structured for insertion
        assert_eq!(values.len(), 3);

        // Verify each value tuple
        assert_eq!(values[0].0, "name");
        assert!(matches!(&values[0].1, ExtractedValue::String(s) if s == "Alice"));

        assert_eq!(values[1].0, "age");
        assert!(matches!(&values[1].1, ExtractedValue::Int(30)));

        assert_eq!(values[2].0, "active");
        assert!(matches!(&values[2].1, ExtractedValue::Bool(true)));
    }

    #[test]
    fn test_update_params_extraction() {
        // Test extracting update parameters
        let updates = vec![
            ("name".to_string(), ExtractedValue::String("Bob".to_string())),
            ("age".to_string(), ExtractedValue::Int(31)),
        ];

        // Verify updates are properly structured
        assert_eq!(updates.len(), 2);

        assert_eq!(updates[0].0, "name");
        assert!(matches!(&updates[0].1, ExtractedValue::String(s) if s == "Bob"));

        assert_eq!(updates[1].0, "age");
        assert!(matches!(&updates[1].1, ExtractedValue::Int(31)));
    }

    #[test]
    fn test_primary_key_handling() {
        // Test primary key field detection and handling
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(42));
        columns.insert("name".to_string(), ExtractedValue::String("test".to_string()));

        let row = Row::new(columns);

        // Verify ID field is accessible
        let id = row.get("id");
        assert!(id.is_ok());
        assert!(matches!(id.unwrap(), ExtractedValue::BigInt(42)));

        // Test with different primary key name
        let mut columns2 = HashMap::new();
        columns2.insert("user_id".to_string(), ExtractedValue::BigInt(100));
        columns2.insert("username".to_string(), ExtractedValue::String("alice".to_string()));

        let row2 = Row::new(columns2);

        // Verify custom primary key is accessible
        let user_id = row2.get("user_id");
        assert!(user_id.is_ok());
        assert!(matches!(user_id.unwrap(), ExtractedValue::BigInt(100)));
    }

    #[test]
    fn test_column_name_mapping() {
        // Test field to column name mapping
        let mut columns = HashMap::new();
        columns.insert("first_name".to_string(), ExtractedValue::String("John".to_string()));
        columns.insert("last_name".to_string(), ExtractedValue::String("Doe".to_string()));
        columns.insert("email_address".to_string(), ExtractedValue::String("john@example.com".to_string()));

        let row = Row::new(columns);

        // Verify all column names are preserved correctly
        let column_names = row.columns();
        assert_eq!(column_names.len(), 3);
        assert!(column_names.contains(&"first_name"));
        assert!(column_names.contains(&"last_name"));
        assert!(column_names.contains(&"email_address"));

        // Verify values are accessible by their exact column names
        assert!(matches!(row.get("first_name"), Ok(ExtractedValue::String(s)) if s == "John"));
        assert!(matches!(row.get("last_name"), Ok(ExtractedValue::String(s)) if s == "Doe"));
        assert!(matches!(row.get("email_address"), Ok(ExtractedValue::String(s)) if s == "john@example.com"));
    }

    #[test]
    fn test_empty_row_data() {
        // Test empty/minimal Row
        let columns = HashMap::new();
        let row = Row::new(columns);

        // Verify empty row behavior
        assert_eq!(row.columns().len(), 0);
        assert!(row.columns_map().is_empty());

        // Accessing any field should fail
        let result = row.get("any_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Column 'any_field' not found"));
    }

    #[test]
    fn test_row_to_json_conversion() {
        // Test converting Row to JSON
        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(42));
        columns.insert("name".to_string(), ExtractedValue::String("Alice".to_string()));
        columns.insert("active".to_string(), ExtractedValue::Bool(true));
        columns.insert("score".to_string(), ExtractedValue::Double(98.5));
        columns.insert("tags".to_string(), ExtractedValue::Null);

        let row = Row::new(columns);
        let json = row.to_json().unwrap();

        // Verify JSON object
        assert!(json.is_object());
        let obj = json.as_object().unwrap();

        // Verify field conversions
        assert_eq!(obj.get("id").unwrap().as_i64().unwrap(), 42);
        assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "Alice");
        assert_eq!(obj.get("active").unwrap().as_bool().unwrap(), true);
        assert!((obj.get("score").unwrap().as_f64().unwrap() - 98.5).abs() < 0.001);
        assert!(obj.get("tags").unwrap().is_null());
    }

    #[test]
    fn test_row_with_complex_types() {
        // Test Row with UUID, Date, Time, Timestamp
        let uuid = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 12, 26).unwrap();
        let time = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
        let timestamp = NaiveDateTime::new(date, time);

        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::Uuid(uuid));
        columns.insert("birth_date".to_string(), ExtractedValue::Date(date));
        columns.insert("meeting_time".to_string(), ExtractedValue::Time(time));
        columns.insert("created_at".to_string(), ExtractedValue::Timestamp(timestamp));
        columns.insert("metadata".to_string(), ExtractedValue::Json(serde_json::json!({"key": "value"})));

        let row = Row::new(columns);

        // Verify complex types are stored correctly
        assert!(matches!(row.get("id"), Ok(ExtractedValue::Uuid(_))));
        assert!(matches!(row.get("birth_date"), Ok(ExtractedValue::Date(_))));
        assert!(matches!(row.get("meeting_time"), Ok(ExtractedValue::Time(_))));
        assert!(matches!(row.get("created_at"), Ok(ExtractedValue::Timestamp(_))));
        assert!(matches!(row.get("metadata"), Ok(ExtractedValue::Json(_))));

        // Test JSON conversion includes complex types
        let json = row.to_json().unwrap();
        assert!(json.is_object());
        let obj = json.as_object().unwrap();

        // UUID should be converted to string
        assert!(obj.get("id").unwrap().is_string());

        // JSON should be preserved
        assert!(obj.get("metadata").unwrap().is_object());
        assert_eq!(
            obj.get("metadata").unwrap().get("key").unwrap().as_str().unwrap(),
            "value"
        );
    }

    #[test]
    fn test_row_with_array_values() {
        // Test Row with array ExtractedValue
        let array_values = vec![
            ExtractedValue::Int(1),
            ExtractedValue::Int(2),
            ExtractedValue::Int(3),
        ];

        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(1));
        columns.insert("numbers".to_string(), ExtractedValue::Array(array_values));

        let row = Row::new(columns);

        // Verify array is stored correctly
        if let Ok(ExtractedValue::Array(arr)) = row.get("numbers") {
            assert_eq!(arr.len(), 3);
            assert!(matches!(arr[0], ExtractedValue::Int(1)));
            assert!(matches!(arr[1], ExtractedValue::Int(2)));
            assert!(matches!(arr[2], ExtractedValue::Int(3)));
        } else {
            panic!("Expected Array ExtractedValue");
        }

        // Test JSON conversion of array
        let json = row.to_json().unwrap();
        let obj = json.as_object().unwrap();
        let json_array = obj.get("numbers").unwrap().as_array().unwrap();
        assert_eq!(json_array.len(), 3);
        assert_eq!(json_array[0].as_i64().unwrap(), 1);
        assert_eq!(json_array[1].as_i64().unwrap(), 2);
        assert_eq!(json_array[2].as_i64().unwrap(), 3);
    }

    #[test]
    fn test_row_with_bytes() {
        // Test Row with byte data
        let bytes = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello" in hex

        let mut columns = HashMap::new();
        columns.insert("id".to_string(), ExtractedValue::BigInt(1));
        columns.insert("data".to_string(), ExtractedValue::Bytes(bytes.clone()));

        let row = Row::new(columns);

        // Verify bytes are stored correctly
        if let Ok(ExtractedValue::Bytes(stored_bytes)) = row.get("data") {
            assert_eq!(stored_bytes, &bytes);
        } else {
            panic!("Expected Bytes ExtractedValue");
        }

        // Test JSON conversion - bytes should be hex encoded
        let json = row.to_json().unwrap();
        let obj = json.as_object().unwrap();
        let hex_string = obj.get("data").unwrap().as_str().unwrap();
        assert_eq!(hex_string, "48656c6c6f");
    }

    #[test]
    fn test_extracted_value_to_json_all_types() {
        // Test JSON conversion for all ExtractedValue types
        assert_eq!(extracted_value_to_json(&ExtractedValue::Null).unwrap(), JsonValue::Null);
        assert_eq!(extracted_value_to_json(&ExtractedValue::Bool(true)).unwrap(), JsonValue::Bool(true));
        assert_eq!(extracted_value_to_json(&ExtractedValue::SmallInt(42)).unwrap(), serde_json::json!(42));
        assert_eq!(extracted_value_to_json(&ExtractedValue::Int(100)).unwrap(), serde_json::json!(100));
        assert_eq!(extracted_value_to_json(&ExtractedValue::BigInt(1000)).unwrap(), serde_json::json!(1000));

        let float_json = extracted_value_to_json(&ExtractedValue::Float(3.14)).unwrap();
        assert!((float_json.as_f64().unwrap() - 3.14).abs() < 0.01);

        let double_json = extracted_value_to_json(&ExtractedValue::Double(2.71828)).unwrap();
        assert!((double_json.as_f64().unwrap() - 2.71828).abs() < 0.00001);

        assert_eq!(
            extracted_value_to_json(&ExtractedValue::String("test".to_string())).unwrap(),
            JsonValue::String("test".to_string())
        );

        assert_eq!(
            extracted_value_to_json(&ExtractedValue::Decimal("123.45".to_string())).unwrap(),
            JsonValue::String("123.45".to_string())
        );

        // Test array conversion
        let array = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::String("two".to_string()),
            ExtractedValue::Null,
        ]);
        let array_json = extracted_value_to_json(&array).unwrap();
        assert!(array_json.is_array());
        let arr = array_json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64().unwrap(), 1);
        assert_eq!(arr[1].as_str().unwrap(), "two");
        assert!(arr[2].is_null());
    }

    #[test]
    fn test_multiple_rows_batch_structure() {
        // Test structure for batch insert (simulating insert_many preparation)
        let mut row1 = HashMap::new();
        row1.insert("name".to_string(), ExtractedValue::String("Alice".to_string()));
        row1.insert("age".to_string(), ExtractedValue::Int(30));

        let mut row2 = HashMap::new();
        row2.insert("name".to_string(), ExtractedValue::String("Bob".to_string()));
        row2.insert("age".to_string(), ExtractedValue::Int(25));

        let mut row3 = HashMap::new();
        row3.insert("name".to_string(), ExtractedValue::String("Charlie".to_string()));
        row3.insert("age".to_string(), ExtractedValue::Int(35));

        let rows = vec![row1, row2, row3];

        // Verify batch structure
        assert_eq!(rows.len(), 3);

        // Verify all rows have same columns
        let first_keys: Vec<&String> = rows[0].keys().collect();
        for row in &rows {
            let row_keys: Vec<&String> = row.keys().collect();
            assert_eq!(row_keys.len(), first_keys.len());
        }

        // Verify individual row data
        assert!(matches!(rows[0].get("name"), Some(ExtractedValue::String(s)) if s == "Alice"));
        assert!(matches!(rows[1].get("name"), Some(ExtractedValue::String(s)) if s == "Bob"));
        assert!(matches!(rows[2].get("name"), Some(ExtractedValue::String(s)) if s == "Charlie"));
    }
}
