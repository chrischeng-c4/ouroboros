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
