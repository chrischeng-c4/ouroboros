//! PostgreSQL query builder.
//!
//! This module provides a type-safe query builder for constructing SQL queries,
//! similar to data-bridge-mongodb's query builder but for SQL.
//!
//! # Examples
//!
//! ## SELECT Query
//!
//! ```ignore
//! use data_bridge_postgres::{QueryBuilder, Operator, OrderDirection, ExtractedValue};
//!
//! let qb = QueryBuilder::new("users")?
//!     .select(vec!["id".to_string(), "name".to_string()])?
//!     .where_clause("age", Operator::Gte, ExtractedValue::Int(18))?
//!     .where_clause("active", Operator::Eq, ExtractedValue::Bool(true))?
//!     .order_by("name", OrderDirection::Asc)?
//!     .limit(10)
//!     .offset(20);
//!
//! let (sql, params) = qb.build();
//! // Result: "SELECT id, name FROM users WHERE age >= $1 AND active = $2 ORDER BY name ASC LIMIT $3 OFFSET $4"
//! ```
//!
//! ## INSERT Query
//!
//! ```ignore
//! use data_bridge_postgres::{QueryBuilder, ExtractedValue};
//!
//! let qb = QueryBuilder::new("users")?;
//! let values = vec![
//!     ("name".to_string(), ExtractedValue::String("Alice".to_string())),
//!     ("age".to_string(), ExtractedValue::Int(30)),
//! ];
//! let (sql, params) = qb.build_insert(&values)?;
//! // Result: "INSERT INTO users (name, age) VALUES ($1, $2) RETURNING *"
//! ```
//!
//! ## UPDATE Query
//!
//! ```ignore
//! use data_bridge_postgres::{QueryBuilder, Operator, ExtractedValue};
//!
//! let qb = QueryBuilder::new("users")?
//!     .where_clause("id", Operator::Eq, ExtractedValue::Int(42))?;
//! let values = vec![
//!     ("name".to_string(), ExtractedValue::String("Bob".to_string())),
//!     ("age".to_string(), ExtractedValue::Int(35)),
//! ];
//! let (sql, params) = qb.build_update(&values)?;
//! // Result: "UPDATE users SET name = $1, age = $2 WHERE id = $3"
//! ```
//!
//! ## DELETE Query
//!
//! ```ignore
//! use data_bridge_postgres::{QueryBuilder, Operator, ExtractedValue};
//!
//! let qb = QueryBuilder::new("users")?
//!     .where_clause("id", Operator::Eq, ExtractedValue::Int(42))?;
//! let (sql, params) = qb.build_delete();
//! // Result: "DELETE FROM users WHERE id = $1"
//! ```

use crate::{DataBridgeError, ExtractedValue, Result};
use unicode_normalization::UnicodeNormalization;

/// Represents a Common Table Expression (CTE) for WITH clause
#[derive(Debug, Clone)]
pub struct CommonTableExpression {
    /// The name of the CTE (used to reference it in the main query)
    pub name: String,
    /// The SQL query for this CTE (will be built from a QueryBuilder)
    pub sql: String,
    /// Parameters for this CTE's query
    pub params: Vec<ExtractedValue>,
}

/// Represents a subquery for use in WHERE clauses
#[derive(Debug, Clone)]
pub struct Subquery {
    /// The SQL of the subquery
    pub sql: String,
    /// Parameters for the subquery
    pub params: Vec<ExtractedValue>,
}

/// Query comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    /// Equal (=)
    Eq,
    /// Not equal (!=)
    Ne,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    Gte,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    Lte,
    /// IN clause
    In,
    /// NOT IN clause
    NotIn,
    /// LIKE pattern matching
    Like,
    /// ILIKE case-insensitive pattern matching
    ILike,
    /// IS NULL
    IsNull,
    /// IS NOT NULL
    IsNotNull,
    /// Column value is in subquery results
    InSubquery,
    /// Column value is not in subquery results
    NotInSubquery,
    /// Subquery returns at least one row
    Exists,
    /// Subquery returns no rows
    NotExists,
    /// JSONB contains @>
    JsonContains,
    /// JSONB contained by <@
    JsonContainedBy,
    /// JSONB key exists ?
    JsonKeyExists,
    /// JSONB any key exists ?|
    JsonAnyKeyExists,
    /// JSONB all keys exist ?&
    JsonAllKeysExist,
}

impl Operator {
    /// Returns the SQL operator string.
    pub fn to_sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "!=",
            Operator::Gt => ">",
            Operator::Gte => ">=",
            Operator::Lt => "<",
            Operator::Lte => "<=",
            Operator::In => "IN",
            Operator::NotIn => "NOT IN",
            Operator::Like => "LIKE",
            Operator::ILike => "ILIKE",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
            Operator::InSubquery => "IN",
            Operator::NotInSubquery => "NOT IN",
            Operator::Exists => "EXISTS",
            Operator::NotExists => "NOT EXISTS",
            Operator::JsonContains => "@>",
            Operator::JsonContainedBy => "<@",
            Operator::JsonKeyExists => "?",
            Operator::JsonAnyKeyExists => "?|",
            Operator::JsonAllKeysExist => "?&",
        }
    }
}

/// SQL aggregate functions.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunction {
    /// COUNT(*) - count all rows
    Count,
    /// COUNT(column) - count non-null values in column
    CountColumn(String),
    /// COUNT(DISTINCT column) - count distinct values
    CountDistinct(String),
    /// SUM(column) - sum of values
    Sum(String),
    /// AVG(column) - average of values
    Avg(String),
    /// MIN(column) - minimum value
    Min(String),
    /// MAX(column) - maximum value
    Max(String),
}

/// Represents a HAVING clause condition for aggregate queries
#[derive(Debug, Clone)]
pub struct HavingCondition {
    /// The aggregate expression (e.g., "COUNT(*)", "SUM(amount)")
    pub aggregate: AggregateFunction,
    /// The comparison operator
    pub operator: Operator,
    /// The value to compare against
    pub value: ExtractedValue,
}

/// Sort order direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

impl OrderDirection {
    /// Returns the SQL order direction string.
    pub fn to_sql(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

/// Type of SQL JOIN
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    /// INNER JOIN
    Inner,
    /// LEFT JOIN
    Left,
    /// RIGHT JOIN
    Right,
    /// FULL OUTER JOIN
    Full,
}

impl JoinType {
    /// Returns the SQL JOIN type string.
    fn to_sql(&self) -> &'static str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL OUTER JOIN",
        }
    }
}

/// Set operation types for combining query results
#[derive(Debug, Clone, PartialEq)]
pub enum SetOperation {
    /// UNION - combines results, removes duplicates
    Union,
    /// UNION ALL - combines results, keeps duplicates
    UnionAll,
    /// INTERSECT - returns only rows in both queries
    Intersect,
    /// INTERSECT ALL - keeps duplicates
    IntersectAll,
    /// EXCEPT - returns rows in first query but not second
    Except,
    /// EXCEPT ALL - keeps duplicates
    ExceptAll,
}

impl SetOperation {
    /// Returns the SQL set operation string.
    fn to_sql(&self) -> &'static str {
        match self {
            SetOperation::Union => " UNION ",
            SetOperation::UnionAll => " UNION ALL ",
            SetOperation::Intersect => " INTERSECT ",
            SetOperation::IntersectAll => " INTERSECT ALL ",
            SetOperation::Except => " EXCEPT ",
            SetOperation::ExceptAll => " EXCEPT ALL ",
        }
    }
}

/// A combined query with set operation
#[derive(Debug, Clone)]
pub struct SetQuery {
    /// The operation to perform
    pub operation: SetOperation,
    /// The SQL of the other query
    pub sql: String,
    /// Parameters for the other query
    pub params: Vec<ExtractedValue>,
}

/// Structured JOIN condition to prevent SQL injection
/// Only allows safe table.column = table.column patterns
#[derive(Debug, Clone)]
pub struct JoinCondition {
    /// Column from the left (main) table
    pub left_column: String,
    /// Table/alias on the right side of the join
    pub right_table: String,
    /// Column from the right table
    pub right_column: String,
}

impl JoinCondition {
    /// Create a new JOIN condition with validated identifiers
    pub fn new(left_column: &str, right_table: &str, right_column: &str) -> Result<Self> {
        QueryBuilder::validate_identifier(left_column)?;
        QueryBuilder::validate_identifier(right_table)?;
        QueryBuilder::validate_identifier(right_column)?;
        Ok(Self {
            left_column: left_column.to_string(),
            right_table: right_table.to_string(),
            right_column: right_column.to_string(),
        })
    }

    /// Generate SQL for the ON clause
    pub fn to_sql(&self, main_table: &str) -> String {
        format!(
            "\"{}\".\"{}\" = \"{}\".\"{}\"",
            main_table,
            self.left_column,
            self.right_table,
            self.right_column
        )
    }
}

/// Represents a JOIN clause
#[derive(Debug, Clone)]
pub struct JoinClause {
    /// Type of JOIN (INNER, LEFT, RIGHT, FULL)
    pub join_type: JoinType,
    /// Table to join
    pub table: String,
    /// Optional alias for the joined table
    pub alias: Option<String>,
    /// ON condition for the join
    pub on_condition: JoinCondition,
}

/// Window function types
#[derive(Debug, Clone, PartialEq)]
pub enum WindowFunction {
    /// ROW_NUMBER() - assigns sequential numbers
    RowNumber,
    /// RANK() - assigns rank with gaps
    Rank,
    /// DENSE_RANK() - assigns rank without gaps
    DenseRank,
    /// NTILE(n) - divides rows into n groups
    Ntile(i32),
    /// LAG(column, offset, default) - access previous row
    Lag(String, Option<i32>, Option<ExtractedValue>),
    /// LEAD(column, offset, default) - access next row
    Lead(String, Option<i32>, Option<ExtractedValue>),
    /// FIRST_VALUE(column) - first value in window
    FirstValue(String),
    /// LAST_VALUE(column) - last value in window
    LastValue(String),
    /// SUM(column) as window function
    Sum(String),
    /// AVG(column) as window function
    Avg(String),
    /// COUNT(*) as window function
    Count,
    /// COUNT(column) as window function
    CountColumn(String),
    /// MIN(column) as window function
    Min(String),
    /// MAX(column) as window function
    Max(String),
}

/// Window specification (PARTITION BY, ORDER BY)
#[derive(Debug, Clone, Default)]
pub struct WindowSpec {
    /// PARTITION BY columns
    pub partition_by: Vec<String>,
    /// ORDER BY columns with direction
    pub order_by: Vec<(String, OrderDirection)>,
}

impl WindowSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn partition_by(mut self, columns: &[&str]) -> Self {
        self.partition_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.order_by.push((column.to_string(), direction));
        self
    }
}

/// A window function expression with alias
#[derive(Debug, Clone)]
pub struct WindowExpression {
    /// The window function
    pub function: WindowFunction,
    /// The window specification
    pub spec: WindowSpec,
    /// Alias for the result column
    pub alias: String,
}

/// Type-safe SQL query builder.
///
/// Provides a fluent API for constructing SELECT, INSERT, UPDATE, and DELETE queries
/// with parameter binding and security validation.
#[derive(Debug)]
pub struct QueryBuilder {
    table: String,
    /// SELECT columns (empty means SELECT *)
    select_columns: Vec<String>,
    /// JOIN clauses
    joins: Vec<JoinClause>,
    /// WHERE conditions (field, operator, value)
    where_conditions: Vec<WhereCondition>,
    /// ORDER BY clauses (field, direction)
    order_by_clauses: Vec<(String, OrderDirection)>,
    /// LIMIT clause
    limit_value: Option<i64>,
    /// OFFSET clause
    offset_value: Option<i64>,
    /// Aggregate functions with optional aliases
    aggregates: Vec<(AggregateFunction, Option<String>)>,
    /// GROUP BY columns
    group_by_columns: Vec<String>,
    /// HAVING conditions for filtering aggregate results
    having_conditions: Vec<HavingCondition>,
    /// Whether to use DISTINCT
    distinct: bool,
    /// Columns for DISTINCT ON (PostgreSQL-specific)
    distinct_on_columns: Vec<String>,
    /// Common Table Expressions (WITH clause)
    ctes: Vec<CommonTableExpression>,
    /// Window function expressions
    windows: Vec<WindowExpression>,
    /// Set operations (UNION, INTERSECT, EXCEPT)
    set_operations: Vec<SetQuery>,
    /// Columns to return from UPDATE/DELETE (RETURNING clause)
    returning: Vec<String>,
}

/// Represents a WHERE condition.
#[derive(Debug, Clone)]
struct WhereCondition {
    field: String,
    operator: Operator,
    value: Option<ExtractedValue>, // None for IS NULL / IS NOT NULL
    subquery: Option<Subquery>,    // Some for subquery conditions
}

impl QueryBuilder {
    /// Creates a new query builder for a table.
    ///
    /// # Arguments
    ///
    /// * `table` - Table name (validated for SQL injection)
    ///
    /// # Errors
    ///
    /// Returns error if table name is invalid.
    pub fn new(table: &str) -> Result<Self> {
        Self::validate_identifier(table)?;
        Ok(Self {
            table: table.to_string(),
            select_columns: Vec::new(),
            joins: Vec::new(),
            where_conditions: Vec::new(),
            order_by_clauses: Vec::new(),
            limit_value: None,
            offset_value: None,
            aggregates: Vec::new(),
            group_by_columns: Vec::new(),
            having_conditions: Vec::new(),
            distinct: false,
            distinct_on_columns: Vec::new(),
            ctes: Vec::new(),
            windows: Vec::new(),
            set_operations: Vec::new(),
            returning: Vec::new(),
        })
    }

    /// Specifies which columns to SELECT.
    ///
    /// # Arguments
    ///
    /// * `columns` - Column names to select
    pub fn select(mut self, columns: Vec<String>) -> Result<Self> {
        for col in &columns {
            Self::validate_identifier(col)?;
        }
        self.select_columns = columns;
        Ok(self)
    }

    /// Adds a WHERE condition.
    ///
    /// # Arguments
    ///
    /// * `field` - Column name
    /// * `operator` - Comparison operator
    /// * `value` - Value to compare against
    pub fn where_clause(mut self, field: &str, operator: Operator, value: ExtractedValue) -> Result<Self> {
        Self::validate_identifier(field)?;

        // For IS NULL and IS NOT NULL, we don't need a value
        let condition_value = match operator {
            Operator::IsNull | Operator::IsNotNull => None,
            _ => Some(value),
        };

        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator,
            value: condition_value,
            subquery: None,
        });
        Ok(self)
    }

    /// Adds a WHERE condition for IS NULL.
    pub fn where_null(self, field: &str) -> Result<Self> {
        self.where_clause(field, Operator::IsNull, ExtractedValue::Null)
    }

    /// Adds a WHERE condition for IS NOT NULL.
    pub fn where_not_null(self, field: &str) -> Result<Self> {
        self.where_clause(field, Operator::IsNotNull, ExtractedValue::Null)
    }

    /// Add a WHERE column IN (subquery) condition
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE id IN (SELECT user_id FROM orders WHERE total > 1000)
    /// let subquery = QueryBuilder::new("orders")?
    ///     .select(vec!["user_id".to_string()])?
    ///     .where_clause("total", Operator::Gt, ExtractedValue::Float(1000.0))?;
    ///
    /// let main = QueryBuilder::new("users")?
    ///     .where_in_subquery("id", subquery)?;
    /// ```
    pub fn where_in_subquery(mut self, field: &str, subquery: QueryBuilder) -> Result<Self> {
        Self::validate_identifier(field)?;
        let (sql, params) = subquery.build_select();
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::InSubquery,
            value: None,
            subquery: Some(Subquery { sql, params }),
        });
        Ok(self)
    }

    /// Add a WHERE column NOT IN (subquery) condition
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE id NOT IN (SELECT user_id FROM inactive_users)
    /// let subquery = QueryBuilder::new("inactive_users")?
    ///     .select(vec!["user_id".to_string()])?;
    ///
    /// let main = QueryBuilder::new("users")?
    ///     .where_not_in_subquery("id", subquery)?;
    /// ```
    pub fn where_not_in_subquery(mut self, field: &str, subquery: QueryBuilder) -> Result<Self> {
        Self::validate_identifier(field)?;
        let (sql, params) = subquery.build_select();
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::NotInSubquery,
            value: None,
            subquery: Some(Subquery { sql, params }),
        });
        Ok(self)
    }

    /// Add a WHERE EXISTS (subquery) condition
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id)
    /// let subquery = QueryBuilder::new("orders")?
    ///     .select(vec!["1".to_string()])?
    ///     .where_clause("user_id", Operator::Eq, ExtractedValue::Int(1))?;
    ///
    /// let main = QueryBuilder::new("users")?
    ///     .where_exists(subquery)?;
    /// ```
    pub fn where_exists(mut self, subquery: QueryBuilder) -> Result<Self> {
        let (sql, params) = subquery.build_select();
        self.where_conditions.push(WhereCondition {
            field: String::new(),  // Not used for EXISTS
            operator: Operator::Exists,
            value: None,
            subquery: Some(Subquery { sql, params }),
        });
        Ok(self)
    }

    /// Add a WHERE NOT EXISTS (subquery) condition
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE NOT EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id)
    /// let subquery = QueryBuilder::new("orders")?
    ///     .select(vec!["1".to_string()])?
    ///     .where_clause("user_id", Operator::Eq, ExtractedValue::Int(1))?;
    ///
    /// let main = QueryBuilder::new("users")?
    ///     .where_not_exists(subquery)?;
    /// ```
    pub fn where_not_exists(mut self, subquery: QueryBuilder) -> Result<Self> {
        let (sql, params) = subquery.build_select();
        self.where_conditions.push(WhereCondition {
            field: String::new(),  // Not used for NOT EXISTS
            operator: Operator::NotExists,
            value: None,
            subquery: Some(Subquery { sql, params }),
        });
        Ok(self)
    }

    /// Add a WHERE column IN (raw SQL subquery) condition
    ///
    /// This method allows using raw SQL for subqueries, which is useful when building
    /// subqueries from Python or other sources where you already have the SQL string.
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE id IN (SELECT user_id FROM orders WHERE total > $1)
    /// let main = QueryBuilder::new("users")?
    ///     .where_in_raw_sql("id", "SELECT user_id FROM orders WHERE total > $1", vec![ExtractedValue::Float(1000.0)])?;
    /// ```
    pub fn where_in_raw_sql(mut self, field: &str, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::InSubquery,
            value: None,
            subquery: Some(Subquery { sql: sql.to_string(), params }),
        });
        Ok(self)
    }

    /// Add a WHERE column NOT IN (raw SQL subquery) condition
    ///
    /// This method allows using raw SQL for subqueries, which is useful when building
    /// subqueries from Python or other sources where you already have the SQL string.
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE id NOT IN (SELECT user_id FROM inactive_users)
    /// let main = QueryBuilder::new("users")?
    ///     .where_not_in_raw_sql("id", "SELECT user_id FROM inactive_users", vec![])?;
    /// ```
    pub fn where_not_in_raw_sql(mut self, field: &str, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::NotInSubquery,
            value: None,
            subquery: Some(Subquery { sql: sql.to_string(), params }),
        });
        Ok(self)
    }

    /// Add a WHERE EXISTS (raw SQL subquery) condition
    ///
    /// This method allows using raw SQL for subqueries, which is useful when building
    /// subqueries from Python or other sources where you already have the SQL string.
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id AND total > $1)
    /// let main = QueryBuilder::new("users")?
    ///     .where_exists_raw_sql("SELECT 1 FROM orders WHERE orders.user_id = users.id AND total > $1", vec![ExtractedValue::Float(1000.0)])?;
    /// ```
    pub fn where_exists_raw_sql(mut self, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        self.where_conditions.push(WhereCondition {
            field: String::new(),  // Not used for EXISTS
            operator: Operator::Exists,
            value: None,
            subquery: Some(Subquery { sql: sql.to_string(), params }),
        });
        Ok(self)
    }

    /// Add a WHERE NOT EXISTS (raw SQL subquery) condition
    ///
    /// This method allows using raw SQL for subqueries, which is useful when building
    /// subqueries from Python or other sources where you already have the SQL string.
    ///
    /// # Example
    /// ```ignore
    /// // SELECT * FROM users WHERE NOT EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id)
    /// let main = QueryBuilder::new("users")?
    ///     .where_not_exists_raw_sql("SELECT 1 FROM orders WHERE orders.user_id = users.id", vec![])?;
    /// ```
    pub fn where_not_exists_raw_sql(mut self, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        self.where_conditions.push(WhereCondition {
            field: String::new(),  // Not used for NOT EXISTS
            operator: Operator::NotExists,
            value: None,
            subquery: Some(Subquery { sql: sql.to_string(), params }),
        });
        Ok(self)
    }

    /// Filter where JSONB column contains the given JSON
    ///
    /// # Example
    /// ```ignore
    /// // WHERE metadata @> '{"role": "admin"}'
    /// let qb = QueryBuilder::new("users")?
    ///     .where_json_contains("metadata", r#"{"role": "admin"}"#)?;
    /// ```
    pub fn where_json_contains(mut self, field: &str, json: &str) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::JsonContains,
            value: Some(ExtractedValue::String(json.to_string())),
            subquery: None,
        });
        Ok(self)
    }

    /// Filter where column is contained by the given JSON
    ///
    /// # Example
    /// ```ignore
    /// // WHERE metadata <@ '{"premium": true}'
    /// let qb = QueryBuilder::new("users")?
    ///     .where_json_contained_by("metadata", r#"{"premium": true}"#)?;
    /// ```
    pub fn where_json_contained_by(mut self, field: &str, json: &str) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::JsonContainedBy,
            value: Some(ExtractedValue::String(json.to_string())),
            subquery: None,
        });
        Ok(self)
    }

    /// Filter where JSONB column has the specified key
    ///
    /// # Example
    /// ```ignore
    /// // WHERE metadata ? 'email'
    /// let qb = QueryBuilder::new("users")?
    ///     .where_json_key_exists("metadata", "email")?;
    /// ```
    pub fn where_json_key_exists(mut self, field: &str, key: &str) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::JsonKeyExists,
            value: Some(ExtractedValue::String(key.to_string())),
            subquery: None,
        });
        Ok(self)
    }

    /// Filter where JSONB column has any of the specified keys
    ///
    /// # Example
    /// ```ignore
    /// // WHERE metadata ?| ARRAY['email', 'phone']
    /// let qb = QueryBuilder::new("users")?
    ///     .where_json_any_key_exists("metadata", &["email", "phone"])?;
    /// ```
    pub fn where_json_any_key_exists(mut self, field: &str, keys: &[&str]) -> Result<Self> {
        Self::validate_identifier(field)?;
        // Store as JSON array string for the query
        let keys_array = format!("ARRAY[{}]", keys.iter().map(|k| format!("'{}'", k)).collect::<Vec<_>>().join(", "));
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::JsonAnyKeyExists,
            value: Some(ExtractedValue::String(keys_array)),
            subquery: None,
        });
        Ok(self)
    }

    /// Filter where JSONB column has all of the specified keys
    ///
    /// # Example
    /// ```ignore
    /// // WHERE metadata ?& ARRAY['email', 'name']
    /// let qb = QueryBuilder::new("users")?
    ///     .where_json_all_keys_exist("metadata", &["email", "name"])?;
    /// ```
    pub fn where_json_all_keys_exist(mut self, field: &str, keys: &[&str]) -> Result<Self> {
        Self::validate_identifier(field)?;
        let keys_array = format!("ARRAY[{}]", keys.iter().map(|k| format!("'{}'", k)).collect::<Vec<_>>().join(", "));
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::JsonAllKeysExist,
            value: Some(ExtractedValue::String(keys_array)),
            subquery: None,
        });
        Ok(self)
    }

    /// Adds an ORDER BY clause.
    pub fn order_by(mut self, field: &str, direction: OrderDirection) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.order_by_clauses.push((field.to_string(), direction));
        Ok(self)
    }

    /// Sets LIMIT.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Sets OFFSET.
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Enable DISTINCT to remove duplicate rows.
    ///
    /// # Example
    /// ```
    /// use data_bridge_postgres::QueryBuilder;
    /// # use data_bridge_postgres::Result;
    /// # fn main() -> Result<()> {
    /// // SELECT DISTINCT email FROM users
    /// let qb = QueryBuilder::new("users")?
    ///     .select(vec!["email".to_string()])?
    ///     .distinct();
    /// # Ok(())
    /// # }
    /// ```
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Use DISTINCT ON to get first row for each unique combination (PostgreSQL-specific).
    ///
    /// Note: When using DISTINCT ON, ORDER BY should typically start with the DISTINCT ON columns.
    ///
    /// # Example
    /// ```
    /// use data_bridge_postgres::{QueryBuilder, OrderDirection};
    /// # use data_bridge_postgres::Result;
    /// # fn main() -> Result<()> {
    /// // SELECT DISTINCT ON (user_id) * FROM orders ORDER BY user_id, created_at DESC
    /// let qb = QueryBuilder::new("orders")?
    ///     .distinct_on(&["user_id"])?
    ///     .order_by("user_id", OrderDirection::Asc)?
    ///     .order_by("created_at", OrderDirection::Desc)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn distinct_on(mut self, columns: &[&str]) -> Result<Self> {
        for col in columns {
            Self::validate_identifier(col)?;
            self.distinct_on_columns.push(col.to_string());
        }
        Ok(self)
    }

    /// Clear DISTINCT settings.
    pub fn clear_distinct(mut self) -> Self {
        self.distinct = false;
        self.distinct_on_columns.clear();
        self
    }

    /// Add a Common Table Expression (CTE) to the query
    ///
    /// CTEs are defined in the WITH clause and can be referenced in the main query.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to use for this CTE
    /// * `query` - A QueryBuilder that defines the CTE's query
    ///
    /// # Example
    /// ```ignore
    /// // WITH high_value AS (SELECT user_id, SUM(amount) as total FROM orders GROUP BY user_id)
    /// // SELECT * FROM high_value WHERE total > 1000
    /// let cte_query = QueryBuilder::new("orders")?
    ///     .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total"))?
    ///     .group_by(&["user_id"])?;
    ///
    /// let main_query = QueryBuilder::new("high_value")?  // Reference the CTE name
    ///     .with_cte("high_value", cte_query)?
    ///     .where_clause("total", Operator::Gt, ExtractedValue::Float(1000.0))?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if the CTE name is invalid.
    pub fn with_cte(mut self, name: &str, query: QueryBuilder) -> Result<Self> {
        Self::validate_identifier(name)?;
        let (sql, params) = query.build_select();
        self.ctes.push(CommonTableExpression {
            name: name.to_string(),
            sql,
            params,
        });
        Ok(self)
    }

    /// Add a raw SQL CTE (for complex queries that can't be built with QueryBuilder)
    ///
    /// # Arguments
    ///
    /// * `name` - The name to use for this CTE
    /// * `sql` - The SQL query for this CTE
    /// * `params` - Parameters for the CTE query
    ///
    /// # Errors
    ///
    /// Returns error if the CTE name is invalid.
    ///
    /// # Security Note
    ///
    /// The CTE name is validated, but the SQL is not. Use this with caution
    /// and only with trusted SQL strings.
    pub fn with_cte_raw(mut self, name: &str, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        Self::validate_identifier(name)?;
        self.ctes.push(CommonTableExpression {
            name: name.to_string(),
            sql: sql.to_string(),
            params,
        });
        Ok(self)
    }

    /// Clear all CTEs
    pub fn clear_ctes(mut self) -> Self {
        self.ctes.clear();
        self
    }

    /// Add a window function to the query
    ///
    /// # Example
    /// ```
    /// use data_bridge_postgres::{QueryBuilder, WindowFunction, WindowSpec, OrderDirection};
    ///
    /// // SELECT *, ROW_NUMBER() OVER (ORDER BY amount DESC) as rank FROM orders
    /// let qb = QueryBuilder::new("orders").unwrap()
    ///     .window(
    ///         WindowFunction::RowNumber,
    ///         WindowSpec::new().order_by("amount", OrderDirection::Desc),
    ///         "rank"
    ///     ).unwrap();
    /// ```
    pub fn window(
        mut self,
        function: WindowFunction,
        spec: WindowSpec,
        alias: &str,
    ) -> Result<Self> {
        // Validate all column names
        Self::validate_identifier(alias)?;
        for col in &spec.partition_by {
            Self::validate_identifier(col)?;
        }
        for (col, _) in &spec.order_by {
            Self::validate_identifier(col)?;
        }
        // Validate columns in function
        match &function {
            WindowFunction::Lag(col, _, _)
            | WindowFunction::Lead(col, _, _)
            | WindowFunction::FirstValue(col)
            | WindowFunction::LastValue(col)
            | WindowFunction::Sum(col)
            | WindowFunction::Avg(col)
            | WindowFunction::CountColumn(col)
            | WindowFunction::Min(col)
            | WindowFunction::Max(col) => {
                Self::validate_identifier(col)?;
            }
            _ => {}
        }

        self.windows.push(WindowExpression {
            function,
            spec,
            alias: alias.to_string(),
        });
        Ok(self)
    }

    /// Add a JOIN clause.
    ///
    /// # Arguments
    ///
    /// * `join_type` - Type of JOIN (INNER, LEFT, RIGHT, FULL)
    /// * `table` - Table to join
    /// * `alias` - Optional alias for the joined table
    /// * `condition` - Structured JOIN condition for security
    ///
    /// # Errors
    ///
    /// Returns error if table or alias name is invalid.
    pub fn join(mut self, join_type: JoinType, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        Self::validate_identifier(table)?;
        if let Some(a) = alias {
            Self::validate_identifier(a)?;
        }

        self.joins.push(JoinClause {
            join_type,
            table: table.to_string(),
            alias: alias.map(|s| s.to_string()),
            on_condition: condition,
        });
        Ok(self)
    }

    /// Add an INNER JOIN.
    ///
    /// # Arguments
    ///
    /// * `table` - Table to join
    /// * `alias` - Optional alias for the joined table
    /// * `condition` - Structured JOIN condition for security
    pub fn inner_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Inner, table, alias, condition)
    }

    /// Add a LEFT JOIN.
    ///
    /// # Arguments
    ///
    /// * `table` - Table to join
    /// * `alias` - Optional alias for the joined table
    /// * `condition` - Structured JOIN condition for security
    pub fn left_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Left, table, alias, condition)
    }

    /// Add a RIGHT JOIN.
    ///
    /// # Arguments
    ///
    /// * `table` - Table to join
    /// * `alias` - Optional alias for the joined table
    /// * `condition` - Structured JOIN condition for security
    pub fn right_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Right, table, alias, condition)
    }

    /// Add a FULL OUTER JOIN.
    ///
    /// # Arguments
    ///
    /// * `table` - Table to join
    /// * `alias` - Optional alias for the joined table
    /// * `condition` - Structured JOIN condition for security
    pub fn full_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Full, table, alias, condition)
    }

    /// Add an aggregate function to the query.
    ///
    /// # Arguments
    ///
    /// * `func` - Aggregate function to apply
    /// * `alias` - Optional alias for the aggregate result
    ///
    /// # Errors
    ///
    /// Returns error if column names in the function or alias are invalid.
    pub fn aggregate(mut self, func: AggregateFunction, alias: Option<&str>) -> Result<Self> {
        // Validate column names in the aggregate function
        match &func {
            AggregateFunction::CountColumn(col) |
            AggregateFunction::CountDistinct(col) |
            AggregateFunction::Sum(col) |
            AggregateFunction::Avg(col) |
            AggregateFunction::Min(col) |
            AggregateFunction::Max(col) => {
                Self::validate_identifier(col)?;
            }
            AggregateFunction::Count => {}
        }
        if let Some(alias_str) = alias {
            Self::validate_identifier(alias_str)?;
        }
        self.aggregates.push((func, alias.map(String::from)));
        Ok(self)
    }

    /// Add GROUP BY columns.
    ///
    /// # Arguments
    ///
    /// * `columns` - Column names to group by
    ///
    /// # Errors
    ///
    /// Returns error if any column name is invalid.
    pub fn group_by(mut self, columns: &[&str]) -> Result<Self> {
        for col in columns {
            Self::validate_identifier(col)?;
            self.group_by_columns.push(col.to_string());
        }
        Ok(self)
    }

    /// Clear all aggregates and GROUP BY columns.
    pub fn clear_aggregates(mut self) -> Self {
        self.aggregates.clear();
        self.group_by_columns.clear();
        self
    }

    /// Add a HAVING condition to filter aggregate results
    ///
    /// # Example
    /// ```
    /// use data_bridge_postgres::{QueryBuilder, AggregateFunction, Operator, ExtractedValue};
    ///
    /// // SELECT COUNT(*) FROM orders GROUP BY status HAVING COUNT(*) > 10
    /// let qb = QueryBuilder::new("orders").unwrap()
    ///     .aggregate(AggregateFunction::Count, Some("order_count")).unwrap()
    ///     .group_by(&["status"]).unwrap()
    ///     .having(AggregateFunction::Count, Operator::Gt, ExtractedValue::Int(10)).unwrap();
    /// ```
    pub fn having(
        mut self,
        aggregate: AggregateFunction,
        operator: Operator,
        value: ExtractedValue,
    ) -> Result<Self> {
        // Validate column names in the aggregate function
        match &aggregate {
            AggregateFunction::CountColumn(col) |
            AggregateFunction::CountDistinct(col) |
            AggregateFunction::Sum(col) |
            AggregateFunction::Avg(col) |
            AggregateFunction::Min(col) |
            AggregateFunction::Max(col) => {
                Self::validate_identifier(col)?;
            }
            AggregateFunction::Count => {}
        }
        self.having_conditions.push(HavingCondition {
            aggregate,
            operator,
            value,
        });
        Ok(self)
    }

    /// Clear all HAVING conditions
    pub fn clear_having(mut self) -> Self {
        self.having_conditions.clear();
        self
    }

    /// Combine this query with another using UNION
    ///
    /// # Arguments
    ///
    /// * `other` - The query to combine with
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use data_bridge_postgres::QueryBuilder;
    /// let q1 = QueryBuilder::new("active_users").unwrap();
    /// let q2 = QueryBuilder::new("archived_users").unwrap();
    /// let combined = q1.union(q2);
    /// ```
    pub fn union(mut self, other: QueryBuilder) -> Self {
        let (sql, params) = other.build_select();
        self.set_operations.push(SetQuery {
            operation: SetOperation::Union,
            sql,
            params,
        });
        self
    }

    /// Combine with UNION ALL (keeps duplicates)
    pub fn union_all(mut self, other: QueryBuilder) -> Self {
        let (sql, params) = other.build_select();
        self.set_operations.push(SetQuery {
            operation: SetOperation::UnionAll,
            sql,
            params,
        });
        self
    }

    /// Combine with INTERSECT
    pub fn intersect(mut self, other: QueryBuilder) -> Self {
        let (sql, params) = other.build_select();
        self.set_operations.push(SetQuery {
            operation: SetOperation::Intersect,
            sql,
            params,
        });
        self
    }

    /// Combine with INTERSECT ALL
    pub fn intersect_all(mut self, other: QueryBuilder) -> Self {
        let (sql, params) = other.build_select();
        self.set_operations.push(SetQuery {
            operation: SetOperation::IntersectAll,
            sql,
            params,
        });
        self
    }

    /// Combine with EXCEPT
    pub fn except(mut self, other: QueryBuilder) -> Self {
        let (sql, params) = other.build_select();
        self.set_operations.push(SetQuery {
            operation: SetOperation::Except,
            sql,
            params,
        });
        self
    }

    /// Combine with EXCEPT ALL
    pub fn except_all(mut self, other: QueryBuilder) -> Self {
        let (sql, params) = other.build_select();
        self.set_operations.push(SetQuery {
            operation: SetOperation::ExceptAll,
            sql,
            params,
        });
        self
    }

    /// Combine with UNION using raw SQL
    pub fn union_raw(mut self, sql: String, params: Vec<ExtractedValue>) -> Self {
        self.set_operations.push(SetQuery {
            operation: SetOperation::Union,
            sql,
            params,
        });
        self
    }

    /// Combine with UNION ALL using raw SQL
    pub fn union_all_raw(mut self, sql: String, params: Vec<ExtractedValue>) -> Self {
        self.set_operations.push(SetQuery {
            operation: SetOperation::UnionAll,
            sql,
            params,
        });
        self
    }

    /// Combine with INTERSECT using raw SQL
    pub fn intersect_raw(mut self, sql: String, params: Vec<ExtractedValue>) -> Self {
        self.set_operations.push(SetQuery {
            operation: SetOperation::Intersect,
            sql,
            params,
        });
        self
    }

    /// Combine with INTERSECT ALL using raw SQL
    pub fn intersect_all_raw(mut self, sql: String, params: Vec<ExtractedValue>) -> Self {
        self.set_operations.push(SetQuery {
            operation: SetOperation::IntersectAll,
            sql,
            params,
        });
        self
    }

    /// Combine with EXCEPT using raw SQL
    pub fn except_raw(mut self, sql: String, params: Vec<ExtractedValue>) -> Self {
        self.set_operations.push(SetQuery {
            operation: SetOperation::Except,
            sql,
            params,
        });
        self
    }

    /// Combine with EXCEPT ALL using raw SQL
    pub fn except_all_raw(mut self, sql: String, params: Vec<ExtractedValue>) -> Self {
        self.set_operations.push(SetQuery {
            operation: SetOperation::ExceptAll,
            sql,
            params,
        });
        self
    }

    /// Add columns to the RETURNING clause for UPDATE/DELETE queries
    ///
    /// # Example
    /// ```ignore
    /// // UPDATE users SET status = 'inactive' WHERE id = 1 RETURNING id, name
    /// let qb = QueryBuilder::new("users")?
    ///     .where_clause("id", Operator::Eq, ExtractedValue::Int(1))?
    ///     .returning(&["id", "name"])?;
    /// ```
    pub fn returning(mut self, columns: &[&str]) -> Result<Self> {
        for col in columns {
            if *col != "*" {
                Self::validate_identifier(col)?;
            }
            self.returning.push(col.to_string());
        }
        Ok(self)
    }

    /// Return all columns from UPDATE/DELETE
    pub fn returning_all(mut self) -> Self {
        self.returning.push("*".to_string());
        self
    }

    /// Clear RETURNING clause
    pub fn clear_returning(mut self) -> Self {
        self.returning.clear();
        self
    }

    /// Builds a SELECT SQL query string with parameter placeholders.
    ///
    /// Returns the SQL string with $1, $2, etc. placeholders.
    pub fn build_select(&self) -> (String, Vec<ExtractedValue>) {
        let mut sql = String::new();
        let mut params = Vec::new();

        // Build WITH clause first if CTEs exist
        if !self.ctes.is_empty() {
            sql.push_str("WITH ");
            let cte_parts: Vec<String> = self.ctes
                .iter()
                .map(|cte| {
                    // Collect CTE params first
                    let cte_param_offset = params.len();
                    params.extend(cte.params.clone());

                    // Adjust parameter indices in CTE SQL
                    let adjusted_sql = Self::adjust_param_indices(&cte.sql, cte_param_offset);

                    format!("{} AS ({})", Self::quote_identifier(&cte.name), adjusted_sql)
                })
                .collect();
            sql.push_str(&cte_parts.join(", "));
            sql.push(' ');
        }

        // Continue with SELECT clause
        sql.push_str("SELECT ");

        // Add DISTINCT ON if specified (takes precedence)
        if !self.distinct_on_columns.is_empty() {
            let cols: Vec<String> = self.distinct_on_columns
                .iter()
                .map(|c| Self::quote_identifier(c))
                .collect();
            sql.push_str(&format!("DISTINCT ON ({}) ", cols.join(", ")));
        } else if self.distinct {
            sql.push_str("DISTINCT ");
        }

        // SELECT columns or aggregates or *
        let mut select_parts = Vec::new();

        if !self.aggregates.is_empty() {
            // Add GROUP BY columns first (they must appear in SELECT when using GROUP BY)
            for col in &self.group_by_columns {
                select_parts.push(Self::quote_identifier(col));
            }

            // Build aggregate expressions
            let agg_parts: Vec<String> = self.aggregates.iter()
                .map(|(func, alias)| {
                    let agg_sql = self.build_aggregate_sql(func);
                    if let Some(alias_str) = alias {
                        format!("{} AS {}", agg_sql, Self::quote_identifier(alias_str))
                    } else {
                        agg_sql
                    }
                })
                .collect();
            select_parts.extend(agg_parts);
        } else if !self.select_columns.is_empty() {
            let quoted_cols: Vec<String> = self.select_columns.iter()
                .map(|c| Self::quote_identifier(c))
                .collect();
            select_parts.extend(quoted_cols);
        }

        // Add window functions to SELECT (they can be combined with regular columns or aggregates)
        for expr in &self.windows {
            select_parts.push(self.build_window_sql(expr));
        }

        // If no explicit columns, aggregates, or windows, select *
        if select_parts.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&select_parts.join(", "));
        }

        sql.push_str(" FROM ");
        sql.push_str(&Self::quote_identifier(&self.table));

        // JOIN clauses
        for join in &self.joins {
            let table_ref = match &join.alias {
                Some(alias) => format!("{} AS \"{}\"", Self::quote_identifier(&join.table), alias),
                None => Self::quote_identifier(&join.table),
            };
            sql.push_str(&format!(
                " {} {} ON {}",
                join.join_type.to_sql(),
                table_ref,
                join.on_condition.to_sql(&self.table)
            ));
        }

        // WHERE clause
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let mut where_parts: Vec<String> = Vec::new();

            for cond in &self.where_conditions {
                let part = match cond.operator {
                    Operator::InSubquery => {
                        if let Some(ref sq) = cond.subquery {
                            // Adjust parameter indices in subquery SQL
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("{} IN ({})", Self::quote_identifier(&cond.field), adjusted_sql)
                        } else {
                            format!("{} IN (NULL)", Self::quote_identifier(&cond.field))
                        }
                    }
                    Operator::NotInSubquery => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("{} NOT IN ({})", Self::quote_identifier(&cond.field), adjusted_sql)
                        } else {
                            format!("{} NOT IN (NULL)", Self::quote_identifier(&cond.field))
                        }
                    }
                    Operator::Exists => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("EXISTS ({})", adjusted_sql)
                        } else {
                            "EXISTS (NULL)".to_string()
                        }
                    }
                    Operator::NotExists => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("NOT EXISTS ({})", adjusted_sql)
                        } else {
                            "NOT EXISTS (NULL)".to_string()
                        }
                    }
                    Operator::IsNull | Operator::IsNotNull => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        format!("{} {}", quoted_field, cond.operator.to_sql())
                    }
                    Operator::In | Operator::NotIn => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} (${})", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} (NULL)", quoted_field, cond.operator.to_sql())
                        }
                    }
                    Operator::JsonContains | Operator::JsonContainedBy => {
                        // JSONB contains uses the JSON value directly (cast to jsonb)
                        if let Some(ExtractedValue::String(json)) = &cond.value {
                            format!("{} {} '{}'::jsonb",
                                Self::quote_identifier(&cond.field),
                                cond.operator.to_sql(),
                                json.replace("'", "''")  // Escape single quotes
                            )
                        } else {
                            format!("{} {} NULL", Self::quote_identifier(&cond.field), cond.operator.to_sql())
                        }
                    }
                    Operator::JsonKeyExists => {
                        // Use parameterized query for key
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                        }
                    }
                    Operator::JsonAnyKeyExists | Operator::JsonAllKeysExist => {
                        // Array operators use literal array
                        if let Some(ExtractedValue::String(arr)) = &cond.value {
                            format!("{} {} {}",
                                Self::quote_identifier(&cond.field),
                                cond.operator.to_sql(),
                                arr
                            )
                        } else {
                            format!("{} {} NULL", Self::quote_identifier(&cond.field), cond.operator.to_sql())
                        }
                    }
                    _ => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                        }
                    }
                };
                where_parts.push(part);
            }

            sql.push_str(&where_parts.join(" AND "));
        }

        // GROUP BY clause
        if !self.group_by_columns.is_empty() {
            sql.push_str(" GROUP BY ");
            let group_parts: Vec<String> = self.group_by_columns.iter()
                .map(|col| Self::quote_identifier(col))
                .collect();
            sql.push_str(&group_parts.join(", "));
        }

        // HAVING clause
        if !self.having_conditions.is_empty() {
            sql.push_str(" HAVING ");
            let having_parts: Vec<String> = self.having_conditions
                .iter()
                .map(|cond| {
                    let agg_sql = self.build_aggregate_sql(&cond.aggregate);
                    params.push(cond.value.clone());
                    format!("{} {} ${}", agg_sql, cond.operator.to_sql(), params.len())
                })
                .collect();
            sql.push_str(&having_parts.join(" AND "));
        }

        // ORDER BY clause
        if !self.order_by_clauses.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_parts: Vec<String> = self.order_by_clauses.iter()
                .map(|(field, dir)| format!("{} {}", Self::quote_identifier(field), dir.to_sql()))
                .collect();
            sql.push_str(&order_parts.join(", "));
        }

        // LIMIT clause
        if let Some(limit) = self.limit_value {
            params.push(ExtractedValue::BigInt(limit));
            sql.push_str(&format!(" LIMIT ${}", params.len()));
        }

        // OFFSET clause
        if let Some(offset) = self.offset_value {
            params.push(ExtractedValue::BigInt(offset));
            sql.push_str(&format!(" OFFSET ${}", params.len()));
        }

        // Add set operations (UNION, INTERSECT, EXCEPT)
        for set_op in &self.set_operations {
            sql.push_str(set_op.operation.to_sql());

            // Adjust parameter indices in the set query
            let adjusted_sql = Self::adjust_param_indices(&set_op.sql, params.len());
            sql.push_str(&adjusted_sql);
            params.extend(set_op.params.clone());
        }

        (sql, params)
    }

    /// Builds an INSERT SQL query string with parameter placeholders.
    ///
    /// Returns the SQL string with $1, $2, etc. placeholders and the parameter values.
    pub fn build_insert(&self, values: &[(String, ExtractedValue)]) -> Result<(String, Vec<ExtractedValue>)> {
        if values.is_empty() {
            return Err(DataBridgeError::Query("Cannot insert with no values".to_string()));
        }

        // Validate column names
        for (col, _) in values {
            Self::validate_identifier(col)?;
        }

        let mut sql = format!("INSERT INTO {} (", Self::quote_identifier(&self.table));
        let columns: Vec<String> = values.iter().map(|(col, _)| Self::quote_identifier(col)).collect();
        sql.push_str(&columns.join(", "));
        sql.push_str(") VALUES (");

        let placeholders: Vec<String> = (1..=values.len()).map(|i| format!("${}", i)).collect();
        sql.push_str(&placeholders.join(", "));
        sql.push_str(") RETURNING *");

        let params: Vec<ExtractedValue> = values.iter().map(|(_, val)| val.clone()).collect();

        Ok((sql, params))
    }

    /// Builds an UPDATE SQL query string with parameter placeholders.
    ///
    /// Returns the SQL string with $1, $2, etc. placeholders and the parameter values.
    pub fn build_update(&self, values: &[(String, ExtractedValue)]) -> Result<(String, Vec<ExtractedValue>)> {
        if values.is_empty() {
            return Err(DataBridgeError::Query("Cannot update with no values".to_string()));
        }

        // Validate column names
        for (col, _) in values {
            Self::validate_identifier(col)?;
        }

        let mut sql = format!("UPDATE {} SET ", Self::quote_identifier(&self.table));
        let mut params: Vec<ExtractedValue> = Vec::new();

        // SET clause
        let set_parts: Vec<String> = values.iter().map(|(col, val)| {
            params.push(val.clone());
            format!("{} = ${}", Self::quote_identifier(col), params.len())
        }).collect();
        sql.push_str(&set_parts.join(", "));

        // WHERE clause
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let mut where_parts: Vec<String> = Vec::new();

            for cond in &self.where_conditions {
                let part = match cond.operator {
                    Operator::InSubquery => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("{} IN ({})", Self::quote_identifier(&cond.field), adjusted_sql)
                        } else {
                            format!("{} IN (NULL)", Self::quote_identifier(&cond.field))
                        }
                    }
                    Operator::NotInSubquery => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("{} NOT IN ({})", Self::quote_identifier(&cond.field), adjusted_sql)
                        } else {
                            format!("{} NOT IN (NULL)", Self::quote_identifier(&cond.field))
                        }
                    }
                    Operator::Exists => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("EXISTS ({})", adjusted_sql)
                        } else {
                            "EXISTS (NULL)".to_string()
                        }
                    }
                    Operator::NotExists => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("NOT EXISTS ({})", adjusted_sql)
                        } else {
                            "NOT EXISTS (NULL)".to_string()
                        }
                    }
                    Operator::IsNull | Operator::IsNotNull => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        format!("{} {}", quoted_field, cond.operator.to_sql())
                    }
                    Operator::In | Operator::NotIn => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} (${})", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} (NULL)", quoted_field, cond.operator.to_sql())
                        }
                    }
                    Operator::JsonContains | Operator::JsonContainedBy => {
                        // JSONB contains uses the JSON value directly (cast to jsonb)
                        if let Some(ExtractedValue::String(json)) = &cond.value {
                            format!("{} {} '{}'::jsonb",
                                Self::quote_identifier(&cond.field),
                                cond.operator.to_sql(),
                                json.replace("'", "''")  // Escape single quotes
                            )
                        } else {
                            format!("{} {} NULL", Self::quote_identifier(&cond.field), cond.operator.to_sql())
                        }
                    }
                    Operator::JsonKeyExists => {
                        // Use parameterized query for key
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                        }
                    }
                    Operator::JsonAnyKeyExists | Operator::JsonAllKeysExist => {
                        // Array operators use literal array
                        if let Some(ExtractedValue::String(arr)) = &cond.value {
                            format!("{} {} {}",
                                Self::quote_identifier(&cond.field),
                                cond.operator.to_sql(),
                                arr
                            )
                        } else {
                            format!("{} {} NULL", Self::quote_identifier(&cond.field), cond.operator.to_sql())
                        }
                    }
                    _ => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                        }
                    }
                };
                where_parts.push(part);
            }

            sql.push_str(&where_parts.join(" AND "));
        }

        // RETURNING clause
        if !self.returning.is_empty() {
            sql.push_str(" RETURNING ");
            if self.returning.contains(&"*".to_string()) {
                sql.push('*');
            } else {
                let cols: Vec<String> = self.returning.iter()
                    .map(|c| Self::quote_identifier(c))
                    .collect();
                sql.push_str(&cols.join(", "));
            }
        }

        Ok((sql, params))
    }

    /// Builds an UPSERT SQL query (INSERT ON CONFLICT UPDATE).
    ///
    /// # Arguments
    ///
    /// * `values` - Column values to insert
    /// * `conflict_target` - Columns for ON CONFLICT clause (unique constraint)
    /// * `update_columns` - Optional columns to update on conflict (None = all except conflict_target)
    ///
    /// # Returns
    ///
    /// Returns the SQL string with $1, $2, etc. placeholders and the parameter values.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let qb = QueryBuilder::new("users")?;
    /// let values = vec![
    ///     ("email".to_string(), ExtractedValue::String("alice@example.com".to_string())),
    ///     ("name".to_string(), ExtractedValue::String("Alice".to_string())),
    /// ];
    /// let conflict_target = vec!["email".to_string()];
    /// let (sql, params) = qb.build_upsert(&values, &conflict_target, None)?;
    /// // Result: "INSERT INTO users (email, name) VALUES ($1, $2)
    /// //          ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name RETURNING *"
    /// ```
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
        let mut sql = format!("INSERT INTO {} (", Self::quote_identifier(&self.table));
        let columns: Vec<String> = values.iter().map(|(col, _)| Self::quote_identifier(col)).collect();
        sql.push_str(&columns.join(", "));
        sql.push_str(") VALUES (");

        let placeholders: Vec<String> = (1..=values.len()).map(|i| format!("${}", i)).collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        // Build ON CONFLICT clause
        sql.push_str(" ON CONFLICT (");
        let quoted_targets: Vec<String> = conflict_target.iter().map(|c| Self::quote_identifier(c)).collect();
        sql.push_str(&quoted_targets.join(", "));
        sql.push_str(") DO UPDATE SET ");

        // Determine which columns to update
        let columns_to_update: Vec<String> = if let Some(update_cols) = update_columns {
            update_cols.to_vec()
        } else {
            // Update all columns except conflict target
            values.iter()
                .map(|(col, _)| col.clone())
                .filter(|col| !conflict_target.contains(col))
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
            .map(|col| format!("{} = EXCLUDED.{}", Self::quote_identifier(col), Self::quote_identifier(col)))
            .collect();
        sql.push_str(&set_parts.join(", "));

        // Add RETURNING *
        sql.push_str(" RETURNING *");

        let params: Vec<ExtractedValue> = values.iter().map(|(_, val)| val.clone()).collect();

        Ok((sql, params))
    }

    /// Builds a DELETE SQL query string with parameter placeholders.
    ///
    /// Returns the SQL string with $1, $2, etc. placeholders and the parameter values.
    pub fn build_delete(&self) -> (String, Vec<ExtractedValue>) {
        let mut sql = format!("DELETE FROM {}", Self::quote_identifier(&self.table));
        let mut params: Vec<ExtractedValue> = Vec::new();

        // WHERE clause
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let mut where_parts: Vec<String> = Vec::new();

            for cond in &self.where_conditions {
                let part = match cond.operator {
                    Operator::InSubquery => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("{} IN ({})", Self::quote_identifier(&cond.field), adjusted_sql)
                        } else {
                            format!("{} IN (NULL)", Self::quote_identifier(&cond.field))
                        }
                    }
                    Operator::NotInSubquery => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("{} NOT IN ({})", Self::quote_identifier(&cond.field), adjusted_sql)
                        } else {
                            format!("{} NOT IN (NULL)", Self::quote_identifier(&cond.field))
                        }
                    }
                    Operator::Exists => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("EXISTS ({})", adjusted_sql)
                        } else {
                            "EXISTS (NULL)".to_string()
                        }
                    }
                    Operator::NotExists => {
                        if let Some(ref sq) = cond.subquery {
                            let adjusted_sql = Self::adjust_param_indices(&sq.sql, params.len());
                            params.extend(sq.params.clone());
                            format!("NOT EXISTS ({})", adjusted_sql)
                        } else {
                            "NOT EXISTS (NULL)".to_string()
                        }
                    }
                    Operator::IsNull | Operator::IsNotNull => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        format!("{} {}", quoted_field, cond.operator.to_sql())
                    }
                    Operator::In | Operator::NotIn => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} (${})", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} (NULL)", quoted_field, cond.operator.to_sql())
                        }
                    }
                    Operator::JsonContains | Operator::JsonContainedBy => {
                        // JSONB contains uses the JSON value directly (cast to jsonb)
                        if let Some(ExtractedValue::String(json)) = &cond.value {
                            format!("{} {} '{}'::jsonb",
                                Self::quote_identifier(&cond.field),
                                cond.operator.to_sql(),
                                json.replace("'", "''")  // Escape single quotes
                            )
                        } else {
                            format!("{} {} NULL", Self::quote_identifier(&cond.field), cond.operator.to_sql())
                        }
                    }
                    Operator::JsonKeyExists => {
                        // Use parameterized query for key
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                        }
                    }
                    Operator::JsonAnyKeyExists | Operator::JsonAllKeysExist => {
                        // Array operators use literal array
                        if let Some(ExtractedValue::String(arr)) = &cond.value {
                            format!("{} {} {}",
                                Self::quote_identifier(&cond.field),
                                cond.operator.to_sql(),
                                arr
                            )
                        } else {
                            format!("{} {} NULL", Self::quote_identifier(&cond.field), cond.operator.to_sql())
                        }
                    }
                    _ => {
                        let quoted_field = Self::quote_identifier(&cond.field);
                        if let Some(ref value) = cond.value {
                            params.push(value.clone());
                            format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                        } else {
                            format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                        }
                    }
                };
                where_parts.push(part);
            }

            sql.push_str(&where_parts.join(" AND "));
        }

        // RETURNING clause
        if !self.returning.is_empty() {
            sql.push_str(" RETURNING ");
            if self.returning.contains(&"*".to_string()) {
                sql.push('*');
            } else {
                let cols: Vec<String> = self.returning.iter()
                    .map(|c| Self::quote_identifier(c))
                    .collect();
                sql.push_str(&cols.join(", "));
            }
        }

        (sql, params)
    }

    /// Quotes a SQL identifier.
    ///
    /// Handles schema-qualified names by quoting each part separately.
    pub fn quote_identifier(name: &str) -> String {
        if name.contains('.') {
            name.split('.')
                .map(|part| format!("\"{}\"", part))
                .collect::<Vec<_>>()
                .join(".")
        } else {
            format!("\"{}\"", name)
        }
    }

    /// Validates a SQL identifier (table/column name).
    ///
    /// Supports both simple identifiers and schema-qualified names (e.g., "public.users").
    pub fn validate_identifier(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataBridgeError::Query("Identifier cannot be empty".to_string()));
        }

        // Check if this is a schema-qualified name (e.g., "public.users")
        if name.contains('.') {
            let parts: Vec<&str> = name.split('.').collect();

            // Only allow schema.table format (two parts)
            if parts.len() != 2 {
                return Err(DataBridgeError::Query(
                    format!("Invalid schema-qualified identifier '{}': must be in format 'schema.table'", name)
                ));
            }

            // Validate each part separately
            for part in parts {
                Self::validate_identifier_part(part)?;
            }

            return Ok(());
        }

        // Simple identifier - validate as a single part
        Self::validate_identifier_part(name)
    }

    /// Builds the SQL for an aggregate function.
    fn build_aggregate_sql(&self, func: &AggregateFunction) -> String {
        match func {
            AggregateFunction::Count => "COUNT(*)".to_string(),
            AggregateFunction::CountColumn(col) => format!("COUNT({})", Self::quote_identifier(col)),
            AggregateFunction::CountDistinct(col) => format!("COUNT(DISTINCT {})", Self::quote_identifier(col)),
            AggregateFunction::Sum(col) => format!("SUM({})", Self::quote_identifier(col)),
            AggregateFunction::Avg(col) => format!("AVG({})", Self::quote_identifier(col)),
            AggregateFunction::Min(col) => format!("MIN({})", Self::quote_identifier(col)),
            AggregateFunction::Max(col) => format!("MAX({})", Self::quote_identifier(col)),
        }
    }

    /// Builds the SQL for a window function expression.
    fn build_window_sql(&self, expr: &WindowExpression) -> String {
        let func_sql = match &expr.function {
            WindowFunction::RowNumber => "ROW_NUMBER()".to_string(),
            WindowFunction::Rank => "RANK()".to_string(),
            WindowFunction::DenseRank => "DENSE_RANK()".to_string(),
            WindowFunction::Ntile(n) => format!("NTILE({})", n),
            WindowFunction::Lag(col, offset, _) => {
                let off = offset.unwrap_or(1);
                format!("LAG({}, {})", Self::quote_identifier(col), off)
            }
            WindowFunction::Lead(col, offset, _) => {
                let off = offset.unwrap_or(1);
                format!("LEAD({}, {})", Self::quote_identifier(col), off)
            }
            WindowFunction::FirstValue(col) => {
                format!("FIRST_VALUE({})", Self::quote_identifier(col))
            }
            WindowFunction::LastValue(col) => {
                format!("LAST_VALUE({})", Self::quote_identifier(col))
            }
            WindowFunction::Sum(col) => format!("SUM({})", Self::quote_identifier(col)),
            WindowFunction::Avg(col) => format!("AVG({})", Self::quote_identifier(col)),
            WindowFunction::Count => "COUNT(*)".to_string(),
            WindowFunction::CountColumn(col) => {
                format!("COUNT({})", Self::quote_identifier(col))
            }
            WindowFunction::Min(col) => format!("MIN({})", Self::quote_identifier(col)),
            WindowFunction::Max(col) => format!("MAX({})", Self::quote_identifier(col)),
        };

        let mut over_parts = Vec::new();

        if !expr.spec.partition_by.is_empty() {
            let cols: Vec<String> = expr
                .spec
                .partition_by
                .iter()
                .map(|c| Self::quote_identifier(c))
                .collect();
            over_parts.push(format!("PARTITION BY {}", cols.join(", ")));
        }

        if !expr.spec.order_by.is_empty() {
            let cols: Vec<String> = expr
                .spec
                .order_by
                .iter()
                .map(|(c, d)| format!("{} {}", Self::quote_identifier(c), d.to_sql()))
                .collect();
            over_parts.push(format!("ORDER BY {}", cols.join(", ")));
        }

        format!(
            "{} OVER ({}) AS {}",
            func_sql,
            over_parts.join(" "),
            Self::quote_identifier(&expr.alias)
        )
    }

    /// Adjusts parameter indices in SQL by adding an offset.
    ///
    /// This is used when combining CTE parameters with main query parameters.
    /// For example, if a CTE has $1 and $2, and the offset is 0, they stay as $1 and $2.
    /// But if there's already another CTE with 2 params, the offset would be 2,
    /// so $1 becomes $3 and $2 becomes $4.
    fn adjust_param_indices(sql: &str, offset: usize) -> String {
        if offset == 0 {
            return sql.to_string();
        }

        let mut result = String::with_capacity(sql.len());
        let mut chars = sql.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                // Found a parameter marker, extract the number
                let mut num_str = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_digit() {
                        num_str.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if !num_str.is_empty() {
                    if let Ok(num) = num_str.parse::<usize>() {
                        // Adjust the parameter index
                        result.push('$');
                        result.push_str(&(num + offset).to_string());
                    } else {
                        // Failed to parse, keep as-is
                        result.push('$');
                        result.push_str(&num_str);
                    }
                } else {
                    // No digits after $, keep as-is
                    result.push('$');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Validates a single part of an identifier (no dots allowed).
    fn validate_identifier_part(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataBridgeError::Query("Identifier part cannot be empty".to_string()));
        }

        // Normalize to NFKC to prevent Unicode confusables
        // This prevents attacks using visually similar characters:
        // - Cyrillic 'а' (U+0430) vs ASCII 'a' (U+0061)
        // - Greek 'Α' (U+0391) vs ASCII 'A' (U+0041)
        // - Fullwidth characters (U+FF01-U+FF5E)
        let name = name.nfkc().collect::<String>();

        // Check length (PostgreSQL limit is 63 bytes per part)
        if name.len() > 63 {
            return Err(DataBridgeError::Query(
                format!("Identifier '{}' exceeds maximum length of 63", name)
            ));
        }

        // Must start with letter or underscore
        let first_char = name.chars().next()
            .ok_or_else(|| DataBridgeError::Query(
                format!("Identifier '{}' is empty or invalid", name)
            ))?;
        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return Err(DataBridgeError::Query(
                format!("Identifier '{}' must start with a letter or underscore", name)
            ));
        }

        // Rest must be alphanumeric or underscore
        for ch in name.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return Err(DataBridgeError::Query(
                    format!("Identifier '{}' contains invalid character '{}'", name, ch)
                ));
            }
        }

        // Prevent system schema access
        let name_lower = name.to_lowercase();
        if name_lower.starts_with("pg_") {
            return Err(DataBridgeError::Query(
                format!("Access to PostgreSQL system catalog '{}' is not allowed", name)
            ));
        }

        if name_lower == "information_schema" {
            return Err(DataBridgeError::Query(
                "Access to information_schema is not allowed".to_string()
            ));
        }

        // Prevent SQL keywords
        const SQL_KEYWORDS: &[&str] = &[
            "select", "insert", "update", "delete", "drop", "create", "alter",
            "truncate", "grant", "revoke", "exec", "execute", "union", "declare",
            "table", "index", "view", "schema", "database", "user", "role",
            "from", "where", "join", "inner", "outer", "left", "right",
            "on", "using", "and", "or", "not", "in", "exists", "between",
            "like", "ilike", "is", "null", "true", "false", "case", "when",
            "then", "else", "end", "as", "order", "by", "group", "having",
            "limit", "offset", "distinct", "all", "any", "some",
        ];

        if SQL_KEYWORDS.contains(&name_lower.as_str()) {
            return Err(DataBridgeError::Query(
                format!("Identifier '{}' is a reserved SQL keyword", name)
            ));
        }

        Ok(())
    }

    /// Builds a query and returns (SQL, parameters) tuple.
    ///
    /// This is a convenience method for SELECT queries.
    pub fn build(&self) -> (String, Vec<ExtractedValue>) {
        self.build_select()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let qb = QueryBuilder::new("users").unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_select_with_columns() {
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec!["id".to_string(), "name".to_string()]).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT \"id\", \"name\" FROM \"users\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_select_with_where() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("id", Operator::Eq, ExtractedValue::Int(42)).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"id\" = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_select_with_multiple_where() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("age", Operator::Gt, ExtractedValue::Int(18)).unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("active".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"age\" > $1 AND \"status\" = $2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_select_with_order_by() {
        let qb = QueryBuilder::new("users").unwrap()
            .order_by("created_at", OrderDirection::Desc).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" ORDER BY \"created_at\" DESC");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_select_with_limit_offset() {
        let qb = QueryBuilder::new("users").unwrap()
            .limit(10)
            .offset(20);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" LIMIT $1 OFFSET $2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_complex_select() {
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec!["id".to_string(), "name".to_string(), "email".to_string()]).unwrap()
            .where_clause("age", Operator::Gte, ExtractedValue::Int(18)).unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap()
            .order_by("name", OrderDirection::Asc).unwrap()
            .limit(50)
            .offset(100);
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\", \"email\" FROM \"users\" WHERE \"age\" >= $1 AND \"active\" = $2 ORDER BY \"name\" ASC LIMIT $3 OFFSET $4"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn test_insert_query() {
        let qb = QueryBuilder::new("users").unwrap();
        let values = vec![
            ("name".to_string(), ExtractedValue::String("Alice".to_string())),
            ("age".to_string(), ExtractedValue::Int(30)),
        ];
        let (sql, params) = qb.build_insert(&values).unwrap();
        assert_eq!(sql, "INSERT INTO \"users\" (\"name\", \"age\") VALUES ($1, $2) RETURNING *");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_update_query() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("id", Operator::Eq, ExtractedValue::Int(42)).unwrap();
        let values = vec![
            ("name".to_string(), ExtractedValue::String("Bob".to_string())),
            ("age".to_string(), ExtractedValue::Int(35)),
        ];
        let (sql, params) = qb.build_update(&values).unwrap();
        assert_eq!(sql, "UPDATE \"users\" SET \"name\" = $1, \"age\" = $2 WHERE \"id\" = $3");
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_delete_query() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("id", Operator::Eq, ExtractedValue::Int(42)).unwrap();
        let (sql, params) = qb.build_delete();
        assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_where_is_null() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_null("email").unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"email\" IS NULL");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_where_is_not_null() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_not_null("email").unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"email\" IS NOT NULL");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_validate_identifier_valid() {
        assert!(QueryBuilder::validate_identifier("users").is_ok());
        assert!(QueryBuilder::validate_identifier("user_table").is_ok());
        assert!(QueryBuilder::validate_identifier("_private").is_ok());
        assert!(QueryBuilder::validate_identifier("table123").is_ok());
    }

    #[test]
    fn test_validate_identifier_invalid() {
        // Empty
        assert!(QueryBuilder::validate_identifier("").is_err());

        // Starts with number
        assert!(QueryBuilder::validate_identifier("123table").is_err());

        // Contains special characters
        assert!(QueryBuilder::validate_identifier("user-table").is_err());
        assert!(QueryBuilder::validate_identifier("user$table").is_err());

        // SQL keywords
        assert!(QueryBuilder::validate_identifier("select").is_err());
        assert!(QueryBuilder::validate_identifier("drop").is_err());
        assert!(QueryBuilder::validate_identifier("DELETE").is_err());

        // System catalogs
        assert!(QueryBuilder::validate_identifier("pg_catalog").is_err());
        assert!(QueryBuilder::validate_identifier("information_schema").is_err());

        // Too long (>63 characters)
        let long_name = "a".repeat(64);
        assert!(QueryBuilder::validate_identifier(&long_name).is_err());
    }

    #[test]
    fn test_validate_schema_qualified_identifiers() {
        // Valid schema-qualified names
        assert!(QueryBuilder::validate_identifier("public.users").is_ok());
        assert!(QueryBuilder::validate_identifier("public.bench_insert_one_db").is_ok());
        assert!(QueryBuilder::validate_identifier("myschema.mytable").is_ok());
        assert!(QueryBuilder::validate_identifier("_private._internal").is_ok());

        // Invalid: too many dots
        assert!(QueryBuilder::validate_identifier("schema.table.column").is_err());
        assert!(QueryBuilder::validate_identifier("a.b.c.d").is_err());

        // Invalid: empty parts
        assert!(QueryBuilder::validate_identifier(".table").is_err());
        assert!(QueryBuilder::validate_identifier("schema.").is_err());
        assert!(QueryBuilder::validate_identifier(".").is_err());

        // Invalid: system schema in qualified name
        assert!(QueryBuilder::validate_identifier("pg_catalog.users").is_err());
        assert!(QueryBuilder::validate_identifier("public.pg_internal").is_err());

        // Invalid: SQL keyword in qualified name
        assert!(QueryBuilder::validate_identifier("public.select").is_err());
        assert!(QueryBuilder::validate_identifier("drop.users").is_err());

        // Invalid: starts with number
        assert!(QueryBuilder::validate_identifier("public.123table").is_err());
        assert!(QueryBuilder::validate_identifier("123schema.table").is_err());

        // Invalid: special characters in parts
        assert!(QueryBuilder::validate_identifier("public.user-table").is_err());
        assert!(QueryBuilder::validate_identifier("my-schema.users").is_err());
    }

    #[test]
    fn test_new_with_invalid_table() {
        assert!(QueryBuilder::new("drop").is_err());
        assert!(QueryBuilder::new("pg_catalog").is_err());
        assert!(QueryBuilder::new("123table").is_err());
    }

    #[test]
    fn test_new_with_schema_qualified_table() {
        // Valid schema-qualified table names
        assert!(QueryBuilder::new("public.users").is_ok());
        assert!(QueryBuilder::new("public.bench_insert_one_db").is_ok());
        assert!(QueryBuilder::new("myschema.mytable").is_ok());

        // Test that queries work with schema-qualified names
        let qb = QueryBuilder::new("public.users").unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"public\".\"users\"");
        assert_eq!(params.len(), 0);

        // Test with WHERE clause
        let qb = QueryBuilder::new("public.bench_insert_one_db").unwrap()
            .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"public\".\"bench_insert_one_db\" WHERE \"id\" = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_operators() {
        assert_eq!(Operator::Eq.to_sql(), "=");
        assert_eq!(Operator::Ne.to_sql(), "!=");
        assert_eq!(Operator::Gt.to_sql(), ">");
        assert_eq!(Operator::Gte.to_sql(), ">=");
        assert_eq!(Operator::Lt.to_sql(), "<");
        assert_eq!(Operator::Lte.to_sql(), "<=");
        assert_eq!(Operator::In.to_sql(), "IN");
        assert_eq!(Operator::NotIn.to_sql(), "NOT IN");
        assert_eq!(Operator::Like.to_sql(), "LIKE");
        assert_eq!(Operator::ILike.to_sql(), "ILIKE");
        assert_eq!(Operator::IsNull.to_sql(), "IS NULL");
        assert_eq!(Operator::IsNotNull.to_sql(), "IS NOT NULL");
        assert_eq!(Operator::InSubquery.to_sql(), "IN");
        assert_eq!(Operator::NotInSubquery.to_sql(), "NOT IN");
        assert_eq!(Operator::Exists.to_sql(), "EXISTS");
        assert_eq!(Operator::NotExists.to_sql(), "NOT EXISTS");
    }

    #[test]
    fn test_order_direction() {
        assert_eq!(OrderDirection::Asc.to_sql(), "ASC");
        assert_eq!(OrderDirection::Desc.to_sql(), "DESC");
    }

    #[test]
    fn test_like_operator() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("name", Operator::Like, ExtractedValue::String("%John%".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"name\" LIKE $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_in_operator() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("status", Operator::In,
                ExtractedValue::Array(vec![
                    ExtractedValue::String("active".to_string()),
                    ExtractedValue::String("pending".to_string())
                ])
            ).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"status\" IN ($1)");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_empty_insert_fails() {
        let qb = QueryBuilder::new("users").unwrap();
        let values: Vec<(String, ExtractedValue)> = vec![];
        assert!(qb.build_insert(&values).is_err());
    }

    #[test]
    fn test_empty_update_fails() {
        let qb = QueryBuilder::new("users").unwrap();
        let values: Vec<(String, ExtractedValue)> = vec![];
        assert!(qb.build_update(&values).is_err());
    }

    #[test]
    fn test_insert_with_invalid_column_name() {
        let qb = QueryBuilder::new("users").unwrap();
        let values = vec![
            ("drop".to_string(), ExtractedValue::String("value".to_string())),
        ];
        assert!(qb.build_insert(&values).is_err());
    }

    #[test]
    fn test_update_with_invalid_column_name() {
        let qb = QueryBuilder::new("users").unwrap();
        let values = vec![
            ("select".to_string(), ExtractedValue::String("value".to_string())),
        ];
        assert!(qb.build_update(&values).is_err());
    }

    #[test]
    fn test_multiple_order_by() {
        let qb = QueryBuilder::new("users").unwrap()
            .order_by("created_at", OrderDirection::Desc).unwrap()
            .order_by("name", OrderDirection::Asc).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" ORDER BY \"created_at\" DESC, \"name\" ASC");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_complex_where_4_conditions() {
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("age", Operator::Gte, ExtractedValue::Int(18)).unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("active".to_string())).unwrap()
            .where_clause("score", Operator::Lt, ExtractedValue::Int(100)).unwrap()
            .where_clause("verified", Operator::Eq, ExtractedValue::Bool(true)).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT * FROM \"users\" WHERE \"age\" >= $1 AND \"status\" = $2 AND \"score\" < $3 AND \"verified\" = $4"
        );
        assert_eq!(params.len(), 4);

        // Verify parameter values
        match &params[0] {
            ExtractedValue::Int(18) => {},
            _ => panic!("Expected Int(18)"),
        }
        match &params[1] {
            ExtractedValue::String(s) if s == "active" => {},
            _ => panic!("Expected String(active)"),
        }
        match &params[2] {
            ExtractedValue::Int(100) => {},
            _ => panic!("Expected Int(100)"),
        }
        match &params[3] {
            ExtractedValue::Bool(true) => {},
            _ => panic!("Expected Bool(true)"),
        }
    }

    #[test]
    fn test_schema_qualified_table_names() {
        // Test SELECT with schema-qualified table
        let qb = QueryBuilder::new("public.users").unwrap()
            .select(vec!["id".to_string(), "name".to_string()]).unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT \"id\", \"name\" FROM \"public\".\"users\" WHERE \"active\" = $1");
        assert_eq!(params.len(), 1);

        // Test INSERT with schema-qualified table
        let qb = QueryBuilder::new("myschema.products").unwrap();
        let values = vec![
            ("name".to_string(), ExtractedValue::String("Widget".to_string())),
            ("price".to_string(), ExtractedValue::Int(999)),
        ];
        let (sql, params) = qb.build_insert(&values).unwrap();
        assert_eq!(sql, "INSERT INTO \"myschema\".\"products\" (\"name\", \"price\") VALUES ($1, $2) RETURNING *");
        assert_eq!(params.len(), 2);

        // Test UPDATE with schema-qualified table
        let qb = QueryBuilder::new("analytics.events").unwrap()
            .where_clause("id", Operator::Eq, ExtractedValue::Int(42)).unwrap();
        let values = vec![
            ("processed".to_string(), ExtractedValue::Bool(true)),
        ];
        let (sql, params) = qb.build_update(&values).unwrap();
        assert_eq!(sql, "UPDATE \"analytics\".\"events\" SET \"processed\" = $1 WHERE \"id\" = $2");
        assert_eq!(params.len(), 2);

        // Test DELETE with schema-qualified table
        let qb = QueryBuilder::new("logs.audit_log").unwrap()
            .where_clause("created_at", Operator::Lt, ExtractedValue::String("2020-01-01".to_string())).unwrap();
        let (sql, params) = qb.build_delete();
        assert_eq!(sql, "DELETE FROM \"logs\".\"audit_log\" WHERE \"created_at\" < $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_very_large_limit() {
        // Test with i64::MAX
        let qb = QueryBuilder::new("users").unwrap()
            .limit(i64::MAX);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" LIMIT $1");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::BigInt(val) if *val == i64::MAX => {},
            _ => panic!("Expected BigInt(i64::MAX)"),
        }

        // Test with a very large but reasonable limit
        let qb = QueryBuilder::new("users").unwrap()
            .limit(1_000_000_000);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" LIMIT $1");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::BigInt(1_000_000_000) => {},
            _ => panic!("Expected BigInt(1_000_000_000)"),
        }
    }

    #[test]
    fn test_zero_limit() {
        // Test LIMIT 0 behavior - should be valid SQL
        let qb = QueryBuilder::new("users").unwrap()
            .limit(0);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" LIMIT $1");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::BigInt(0) => {},
            _ => panic!("Expected BigInt(0)"),
        }

        // Test LIMIT 0 with WHERE clause
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap()
            .limit(0);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"active\" = $1 LIMIT $2");
        assert_eq!(params.len(), 2);

        // Test LIMIT 0 with ORDER BY
        let qb = QueryBuilder::new("users").unwrap()
            .order_by("created_at", OrderDirection::Desc).unwrap()
            .limit(0);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" ORDER BY \"created_at\" DESC LIMIT $1");
        assert_eq!(params.len(), 1);

        // Test LIMIT 0 with OFFSET
        let qb = QueryBuilder::new("users").unwrap()
            .limit(0)
            .offset(10);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" LIMIT $1 OFFSET $2");
        assert_eq!(params.len(), 2);
        match &params[0] {
            ExtractedValue::BigInt(0) => {},
            _ => panic!("Expected BigInt(0) for limit"),
        }
        match &params[1] {
            ExtractedValue::BigInt(10) => {},
            _ => panic!("Expected BigInt(10) for offset"),
        }
    }

    #[test]
    fn test_negative_offset() {
        // PostgreSQL allows negative offsets (they're treated as 0)
        // The query builder should accept them and let PostgreSQL handle it
        let qb = QueryBuilder::new("users").unwrap()
            .offset(-10);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" OFFSET $1");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::BigInt(-10) => {},
            _ => panic!("Expected BigInt(-10)"),
        }

        // Test negative offset with positive limit
        let qb = QueryBuilder::new("users").unwrap()
            .limit(20)
            .offset(-5);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" LIMIT $1 OFFSET $2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_empty_select_columns() {
        // When no columns are specified, should default to SELECT *
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec![]).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\"");
        assert_eq!(params.len(), 0);

        // Empty select with WHERE clause
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec![]).unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"active\" = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_in_operator_with_empty_list() {
        // IN operator with empty array
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("status", Operator::In, ExtractedValue::Array(vec![])).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"status\" IN ($1)");
        assert_eq!(params.len(), 1);

        // Verify the parameter is an empty array
        match &params[0] {
            ExtractedValue::Array(arr) if arr.is_empty() => {},
            _ => panic!("Expected empty Array"),
        }

        // NOT IN with empty array
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("user_role", Operator::NotIn, ExtractedValue::Array(vec![])).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"user_role\" NOT IN ($1)");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_in_operator_with_many_values() {
        // Create an array with 150 values
        let values: Vec<ExtractedValue> = (0..150)
            .map(|i| ExtractedValue::Int(i))
            .collect();

        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("id", Operator::In, ExtractedValue::Array(values.clone())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"id\" IN ($1)");
        assert_eq!(params.len(), 1);

        // Verify the parameter contains 150 values
        match &params[0] {
            ExtractedValue::Array(arr) if arr.len() == 150 => {},
            _ => panic!("Expected Array with 150 elements"),
        }

        // Test with strings
        let string_values: Vec<ExtractedValue> = (0..100)
            .map(|i| ExtractedValue::String(format!("value_{}", i)))
            .collect();

        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("status", Operator::In, ExtractedValue::Array(string_values)).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"status\" IN ($1)");
        assert_eq!(params.len(), 1);

        match &params[0] {
            ExtractedValue::Array(arr) if arr.len() == 100 => {},
            _ => panic!("Expected Array with 100 elements"),
        }
    }

    #[test]
    fn test_like_patterns_escaping() {
        // Test basic LIKE patterns
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("name", Operator::Like, ExtractedValue::String("%John%".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"name\" LIKE $1");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::String(s) if s == "%John%" => {},
            _ => panic!("Expected String(%John%)"),
        }

        // Test LIKE with special characters (underscore wildcard)
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("email", Operator::Like, ExtractedValue::String("user_@%.com".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"email\" LIKE $1");
        match &params[0] {
            ExtractedValue::String(s) if s == "user_@%.com" => {},
            _ => panic!("Expected String(user_@%.com)"),
        }

        // Test ILIKE (case-insensitive)
        let qb = QueryBuilder::new("products").unwrap()
            .where_clause("description", Operator::ILike, ExtractedValue::String("%widget%".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"products\" WHERE \"description\" ILIKE $1");
        match &params[0] {
            ExtractedValue::String(s) if s == "%widget%" => {},
            _ => panic!("Expected String(%widget%)"),
        }

        // Test multiple LIKE conditions
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("first_name", Operator::Like, ExtractedValue::String("J%".to_string())).unwrap()
            .where_clause("last_name", Operator::Like, ExtractedValue::String("%son".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"first_name\" LIKE $1 AND \"last_name\" LIKE $2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_ilike_case_insensitive() {
        // Test ILIKE basic usage
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("name", Operator::ILike, ExtractedValue::String("%john%".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"name\" ILIKE $1");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::String(s) if s == "%john%" => {},
            _ => panic!("Expected String(%john%)"),
        }

        // Test ILIKE with mixed case pattern
        let qb = QueryBuilder::new("products").unwrap()
            .where_clause("name", Operator::ILike, ExtractedValue::String("%WiDgEt%".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"products\" WHERE \"name\" ILIKE $1");
        match &params[0] {
            ExtractedValue::String(s) if s == "%WiDgEt%" => {},
            _ => panic!("Expected String(%WiDgEt%)"),
        }

        // Test ILIKE with underscore wildcard
        let qb = QueryBuilder::new("emails").unwrap()
            .where_clause("address", Operator::ILike, ExtractedValue::String("USER_@example.com".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"emails\" WHERE \"address\" ILIKE $1");
        match &params[0] {
            ExtractedValue::String(s) if s == "USER_@example.com" => {},
            _ => panic!("Expected String(USER_@example.com)"),
        }

        // Test combining ILIKE with other conditions
        let qb = QueryBuilder::new("articles").unwrap()
            .where_clause("title", Operator::ILike, ExtractedValue::String("%RUST%".to_string())).unwrap()
            .where_clause("published", Operator::Eq, ExtractedValue::Bool(true)).unwrap()
            .order_by("created_at", OrderDirection::Desc).unwrap()
            .limit(10);
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT * FROM \"articles\" WHERE \"title\" ILIKE $1 AND \"published\" = $2 ORDER BY \"created_at\" DESC LIMIT $3"
        );
        assert_eq!(params.len(), 3);

        // Test multiple ILIKE conditions
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("first_name", Operator::ILike, ExtractedValue::String("j%".to_string())).unwrap()
            .where_clause("last_name", Operator::ILike, ExtractedValue::String("%SON".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"first_name\" ILIKE $1 AND \"last_name\" ILIKE $2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_order_by_multiple_columns() {
        // Test ORDER BY with 3 columns
        let qb = QueryBuilder::new("users").unwrap()
            .order_by("department", OrderDirection::Asc).unwrap()
            .order_by("salary", OrderDirection::Desc).unwrap()
            .order_by("name", OrderDirection::Asc).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" ORDER BY \"department\" ASC, \"salary\" DESC, \"name\" ASC");
        assert_eq!(params.len(), 0);

        // Test with 5 columns
        let qb = QueryBuilder::new("products").unwrap()
            .order_by("category", OrderDirection::Asc).unwrap()
            .order_by("subcategory", OrderDirection::Asc).unwrap()
            .order_by("price", OrderDirection::Desc).unwrap()
            .order_by("rating", OrderDirection::Desc).unwrap()
            .order_by("name", OrderDirection::Asc).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT * FROM \"products\" ORDER BY \"category\" ASC, \"subcategory\" ASC, \"price\" DESC, \"rating\" DESC, \"name\" ASC"
        );
        assert_eq!(params.len(), 0);

        // Test ORDER BY with WHERE clause
        let qb = QueryBuilder::new("users").unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap()
            .order_by("created_at", OrderDirection::Desc).unwrap()
            .order_by("id", OrderDirection::Asc).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"active\" = $1 ORDER BY \"created_at\" DESC, \"id\" ASC");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_combined_query_builder() {
        // Test combining all query options: SELECT columns, WHERE (multiple conditions),
        // ORDER BY (multiple), LIMIT, and OFFSET
        let qb = QueryBuilder::new("orders").unwrap()
            .select(vec![
                "id".to_string(),
                "customer_id".to_string(),
                "total".to_string(),
                "status".to_string(),
                "created_at".to_string(),
            ]).unwrap()
            .where_clause("status", Operator::In, ExtractedValue::Array(vec![
                ExtractedValue::String("pending".to_string()),
                ExtractedValue::String("processing".to_string()),
                ExtractedValue::String("shipped".to_string()),
            ])).unwrap()
            .where_clause("total", Operator::Gte, ExtractedValue::Int(100)).unwrap()
            .where_clause("customer_id", Operator::Ne, ExtractedValue::Int(0)).unwrap()
            .where_not_null("payment_method").unwrap()
            .order_by("created_at", OrderDirection::Desc).unwrap()
            .order_by("total", OrderDirection::Desc).unwrap()
            .order_by("id", OrderDirection::Asc).unwrap()
            .limit(50)
            .offset(100);

        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT \"id\", \"customer_id\", \"total\", \"status\", \"created_at\" FROM \"orders\" WHERE \"status\" IN ($1) AND \"total\" >= $2 AND \"customer_id\" != $3 AND \"payment_method\" IS NOT NULL ORDER BY \"created_at\" DESC, \"total\" DESC, \"id\" ASC LIMIT $4 OFFSET $5"
        );
        assert_eq!(params.len(), 5);

        // Verify parameter types
        match &params[0] {
            ExtractedValue::Array(arr) if arr.len() == 3 => {},
            _ => panic!("Expected Array with 3 elements"),
        }
        match &params[1] {
            ExtractedValue::Int(100) => {},
            _ => panic!("Expected Int(100)"),
        }
        match &params[2] {
            ExtractedValue::Int(0) => {},
            _ => panic!("Expected Int(0)"),
        }
        match &params[3] {
            ExtractedValue::BigInt(50) => {},
            _ => panic!("Expected BigInt(50)"),
        }
        match &params[4] {
            ExtractedValue::BigInt(100) => {},
            _ => panic!("Expected BigInt(100)"),
        }

        // Test combined query with schema-qualified table
        let qb = QueryBuilder::new("public.analytics_events").unwrap()
            .select(vec!["event_type".to_string(), "user_id".to_string(), "timestamp".to_string()]).unwrap()
            .where_clause("event_type", Operator::Like, ExtractedValue::String("click%".to_string())).unwrap()
            .where_clause("timestamp", Operator::Gte, ExtractedValue::String("2024-01-01".to_string())).unwrap()
            .order_by("timestamp", OrderDirection::Desc).unwrap()
            .limit(1000);

        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT \"event_type\", \"user_id\", \"timestamp\" FROM \"public\".\"analytics_events\" WHERE \"event_type\" LIKE $1 AND \"timestamp\" >= $2 ORDER BY \"timestamp\" DESC LIMIT $3"
        );
        assert_eq!(params.len(), 3);
    }

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

        assert!(sql.contains("INSERT INTO \"users\""));
        assert!(sql.contains("ON CONFLICT (\"email\")"));
        assert!(sql.contains("DO UPDATE SET"));
        assert!(sql.contains("\"name\" = EXCLUDED.\"name\""));
        assert!(sql.contains("\"age\" = EXCLUDED.\"age\""));
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
        let update_cols = vec!["name".to_string()];
        let (sql, params) = qb.build_upsert(&values, &conflict_target, Some(&update_cols)).unwrap();

        assert!(sql.contains("DO UPDATE SET \"name\" = EXCLUDED.\"name\""));
        assert!(!sql.contains("\"age\" = EXCLUDED.\"age\""));
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

        assert!(sql.contains("ON CONFLICT (\"email\", \"department\")"));
        assert!(sql.contains("DO UPDATE SET \"name\" = EXCLUDED.\"name\""));
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

    #[test]
    fn test_inner_join() {
        let condition = JoinCondition::new("author_id", "users", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .select(vec!["posts.id".to_string(), "posts.title".to_string(), "users.name".to_string()]).unwrap()
            .inner_join("users", None, condition).unwrap();

        let (sql, _) = qb.build_select();
        assert!(sql.contains("INNER JOIN \"users\" ON \"posts\".\"author_id\" = \"users\".\"id\""));
        assert_eq!(sql, "SELECT \"posts\".\"id\", \"posts\".\"title\", \"users\".\"name\" FROM \"posts\" INNER JOIN \"users\" ON \"posts\".\"author_id\" = \"users\".\"id\"");
    }

    #[test]
    fn test_left_join_with_alias() {
        let condition = JoinCondition::new("author_id", "u", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .select(vec!["p.id".to_string(), "u.name".to_string()]).unwrap()
            .left_join("users", Some("u"), condition).unwrap();

        let (sql, _) = qb.build_select();
        assert!(sql.contains("LEFT JOIN \"users\" AS \"u\" ON \"posts\".\"author_id\" = \"u\".\"id\""));
        assert_eq!(sql, "SELECT \"p\".\"id\", \"u\".\"name\" FROM \"posts\" LEFT JOIN \"users\" AS \"u\" ON \"posts\".\"author_id\" = \"u\".\"id\"");
    }

    #[test]
    fn test_multiple_joins() {
        let condition1 = JoinCondition::new("author_id", "u", "id").unwrap();
        let condition2 = JoinCondition::new("category_id", "c", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .inner_join("users", Some("u"), condition1).unwrap()
            .left_join("categories", Some("c"), condition2).unwrap();

        let (sql, _) = qb.build_select();
        assert!(sql.contains("INNER JOIN"));
        assert!(sql.contains("LEFT JOIN"));
        assert_eq!(sql, "SELECT * FROM \"posts\" INNER JOIN \"users\" AS \"u\" ON \"posts\".\"author_id\" = \"u\".\"id\" LEFT JOIN \"categories\" AS \"c\" ON \"posts\".\"category_id\" = \"c\".\"id\"");
    }

    #[test]
    fn test_right_join() {
        let condition = JoinCondition::new("author_id", "users", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .select(vec!["posts.id".to_string(), "users.name".to_string()]).unwrap()
            .right_join("users", None, condition).unwrap();

        let (sql, _) = qb.build_select();
        assert!(sql.contains("RIGHT JOIN \"users\" ON \"posts\".\"author_id\" = \"users\".\"id\""));
    }

    #[test]
    fn test_full_join() {
        let condition = JoinCondition::new("author_id", "users", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .select(vec!["posts.id".to_string(), "users.name".to_string()]).unwrap()
            .full_join("users", None, condition).unwrap();

        let (sql, _) = qb.build_select();
        assert!(sql.contains("FULL OUTER JOIN \"users\" ON \"posts\".\"author_id\" = \"users\".\"id\""));
    }

    #[test]
    fn test_join_with_where() {
        let condition = JoinCondition::new("author_id", "u", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .select(vec!["posts.id".to_string(), "posts.title".to_string(), "users.name".to_string()]).unwrap()
            .inner_join("users", Some("u"), condition).unwrap()
            .where_clause("posts.published", Operator::Eq, ExtractedValue::Bool(true)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("INNER JOIN"));
        assert!(sql.contains("WHERE \"posts\".\"published\" = $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_join_with_order_and_limit() {
        let condition = JoinCondition::new("author_id", "u", "id").unwrap();
        let qb = QueryBuilder::new("posts").unwrap()
            .select(vec!["posts.id".to_string(), "users.name".to_string()]).unwrap()
            .inner_join("users", Some("u"), condition).unwrap()
            .order_by("posts.created_at", OrderDirection::Desc).unwrap()
            .limit(10);

        let (sql, params) = qb.build_select();
        assert!(sql.contains("INNER JOIN"));
        assert!(sql.contains("ORDER BY \"posts\".\"created_at\" DESC"));
        assert!(sql.contains("LIMIT $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_join_invalid_table_name() {
        let result = QueryBuilder::new("posts").unwrap()
            .inner_join("drop", None, JoinCondition::new("author_id", "users", "id").unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_join_invalid_alias() {
        let result = QueryBuilder::new("posts").unwrap()
            .inner_join("users", Some("select"), JoinCondition::new("author_id", "users", "id").unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_join_with_schema_qualified_table() {
        let condition = JoinCondition::new("author_id", "u", "id").unwrap();
        let qb = QueryBuilder::new("public.posts").unwrap()
            .select(vec!["p.id".to_string(), "u.name".to_string()]).unwrap()
            .inner_join("public.users", Some("u"), condition).unwrap();

        let (sql, _) = qb.build_select();
        assert!(sql.contains("FROM \"public\".\"posts\""));
        assert!(sql.contains("INNER JOIN \"public\".\"users\" AS \"u\""));
    }

    #[test]
    fn test_join_condition_valid() {
        let condition = JoinCondition::new("author_id", "users", "id").unwrap();
        let sql = condition.to_sql("posts");
        assert_eq!(sql, "\"posts\".\"author_id\" = \"users\".\"id\"");
    }

    #[test]
    fn test_join_condition_with_alias() {
        let condition = JoinCondition::new("author_id", "u", "id").unwrap();
        let sql = condition.to_sql("posts");
        assert_eq!(sql, "\"posts\".\"author_id\" = \"u\".\"id\"");
    }

    #[test]
    fn test_join_condition_invalid_left_column() {
        let result = JoinCondition::new("drop", "users", "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_join_condition_invalid_right_table() {
        let result = JoinCondition::new("author_id", "select", "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_join_condition_invalid_right_column() {
        let result = JoinCondition::new("author_id", "users", "delete");
        assert!(result.is_err());
    }

    #[test]
    fn test_join_condition_sql_injection_attempt() {
        // Attempt SQL injection in column name
        let result = JoinCondition::new("author_id; DROP TABLE users--", "users", "id");
        assert!(result.is_err());

        // Attempt SQL injection in table name
        let result = JoinCondition::new("author_id", "users; DROP TABLE posts--", "id");
        assert!(result.is_err());

        // Attempt SQL injection in right column
        let result = JoinCondition::new("author_id", "users", "id OR 1=1--");
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_count_all() {
        let qb = QueryBuilder::new("users").unwrap()
            .aggregate(AggregateFunction::Count, None).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(*) FROM \"users\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_count_with_alias() {
        let qb = QueryBuilder::new("users").unwrap()
            .aggregate(AggregateFunction::Count, Some("total")).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(*) AS \"total\" FROM \"users\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_count_column() {
        let qb = QueryBuilder::new("users").unwrap()
            .aggregate(AggregateFunction::CountColumn("email".to_string()), Some("email_count")).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(\"email\") AS \"email_count\" FROM \"users\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_count_distinct() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::CountDistinct("customer_id".to_string()), Some("unique_customers")).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(DISTINCT \"customer_id\") AS \"unique_customers\" FROM \"orders\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_sum() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Sum("total".to_string()), Some("revenue")).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT SUM(\"total\") AS \"revenue\" FROM \"orders\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_avg() {
        let qb = QueryBuilder::new("products").unwrap()
            .aggregate(AggregateFunction::Avg("price".to_string()), Some("avg_price")).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT AVG(\"price\") AS \"avg_price\" FROM \"products\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_min_max() {
        let qb = QueryBuilder::new("temperatures").unwrap()
            .aggregate(AggregateFunction::Min("value".to_string()), Some("min_temp")).unwrap()
            .aggregate(AggregateFunction::Max("value".to_string()), Some("max_temp")).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT MIN(\"value\") AS \"min_temp\", MAX(\"value\") AS \"max_temp\" FROM \"temperatures\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_with_group_by() {
        let qb = QueryBuilder::new("sales").unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total_sales")).unwrap()
            .group_by(&["region", "product"]).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT \"region\", \"product\", SUM(\"amount\") AS \"total_sales\" FROM \"sales\" GROUP BY \"region\", \"product\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_with_where() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, Some("order_count")).unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("completed".to_string())).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(*) AS \"order_count\" FROM \"orders\" WHERE \"status\" = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_aggregate_with_where_and_group_by() {
        let qb = QueryBuilder::new("sales").unwrap()
            .aggregate(AggregateFunction::Sum("revenue".to_string()), Some("total")).unwrap()
            .aggregate(AggregateFunction::Avg("revenue".to_string()), Some("average")).unwrap()
            .where_clause("year", Operator::Eq, ExtractedValue::Int(2024)).unwrap()
            .group_by(&["department"]).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT \"department\", SUM(\"revenue\") AS \"total\", AVG(\"revenue\") AS \"average\" FROM \"sales\" WHERE \"year\" = $1 GROUP BY \"department\""
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_aggregate_with_group_by_and_order_by() {
        let qb = QueryBuilder::new("sales").unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total")).unwrap()
            .group_by(&["category"]).unwrap()
            .order_by("total", OrderDirection::Desc).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT \"category\", SUM(\"amount\") AS \"total\" FROM \"sales\" GROUP BY \"category\" ORDER BY \"total\" DESC"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_with_limit() {
        let qb = QueryBuilder::new("products").unwrap()
            .aggregate(AggregateFunction::Count, None).unwrap()
            .limit(1);
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(*) FROM \"products\" LIMIT $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_clear_aggregates() {
        let qb = QueryBuilder::new("users").unwrap()
            .aggregate(AggregateFunction::Count, Some("total")).unwrap()
            .group_by(&["department"]).unwrap()
            .clear_aggregates();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT * FROM \"users\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_aggregate_invalid_column_name() {
        let result = QueryBuilder::new("users").unwrap()
            .aggregate(AggregateFunction::Sum("drop".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_invalid_alias() {
        let result = QueryBuilder::new("users").unwrap()
            .aggregate(AggregateFunction::Count, Some("select"));
        assert!(result.is_err());
    }

    #[test]
    fn test_group_by_invalid_column() {
        let result = QueryBuilder::new("users").unwrap()
            .group_by(&["department", "select"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_aggregates_no_alias() {
        let qb = QueryBuilder::new("sales").unwrap()
            .aggregate(AggregateFunction::Count, None).unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), None).unwrap()
            .aggregate(AggregateFunction::Avg("amount".to_string()), None).unwrap();
        let (sql, params) = qb.build_select();
        assert_eq!(sql, "SELECT COUNT(*), SUM(\"amount\"), AVG(\"amount\") FROM \"sales\"");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_complex_aggregate_query() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, Some("total_orders")).unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total_revenue")).unwrap()
            .aggregate(AggregateFunction::Avg("amount".to_string()), Some("avg_order_value")).unwrap()
            .aggregate(AggregateFunction::CountDistinct("customer_id".to_string()), Some("unique_customers")).unwrap()
            .where_clause("status", Operator::In, ExtractedValue::Array(vec![
                ExtractedValue::String("completed".to_string()),
                ExtractedValue::String("shipped".to_string()),
            ])).unwrap()
            .where_clause("created_at", Operator::Gte, ExtractedValue::String("2024-01-01".to_string())).unwrap()
            .group_by(&["region", "product_category"]).unwrap()
            .order_by("total_revenue", OrderDirection::Desc).unwrap()
            .limit(10);
        let (sql, params) = qb.build_select();
        assert_eq!(
            sql,
            "SELECT \"region\", \"product_category\", COUNT(*) AS \"total_orders\", SUM(\"amount\") AS \"total_revenue\", AVG(\"amount\") AS \"avg_order_value\", COUNT(DISTINCT \"customer_id\") AS \"unique_customers\" FROM \"orders\" WHERE \"status\" IN ($1) AND \"created_at\" >= $2 GROUP BY \"region\", \"product_category\" ORDER BY \"total_revenue\" DESC LIMIT $3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_having_basic() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, Some("order_count")).unwrap()
            .group_by(&["status"]).unwrap()
            .having(AggregateFunction::Count, Operator::Gt, ExtractedValue::Int(10)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("HAVING COUNT(*) > $"));
        assert_eq!(params.len(), 1);
        match &params[0] {
            ExtractedValue::Int(10) => {},
            _ => panic!("Expected Int(10)"),
        }
    }

    #[test]
    fn test_having_with_sum() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total")).unwrap()
            .group_by(&["customer_id"]).unwrap()
            .having(AggregateFunction::Sum("amount".to_string()), Operator::Gte, ExtractedValue::Float(1000.0)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("HAVING SUM(\"amount\") >= $"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_having_multiple_conditions() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, None).unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), None).unwrap()
            .group_by(&["status"]).unwrap()
            .having(AggregateFunction::Count, Operator::Gt, ExtractedValue::Int(5)).unwrap()
            .having(AggregateFunction::Sum("amount".to_string()), Operator::Lt, ExtractedValue::Float(10000.0)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("HAVING COUNT(*) > $"));
        assert!(sql.contains(" AND SUM(\"amount\") < $"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_having_with_where() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, Some("order_count")).unwrap()
            .where_clause("year", Operator::Eq, ExtractedValue::Int(2024)).unwrap()
            .group_by(&["status"]).unwrap()
            .having(AggregateFunction::Count, Operator::Gte, ExtractedValue::Int(100)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("WHERE \"year\" = $1"));
        assert!(sql.contains("HAVING COUNT(*) >= $2"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_having_with_avg() {
        let qb = QueryBuilder::new("students").unwrap()
            .aggregate(AggregateFunction::Avg("score".to_string()), Some("avg_score")).unwrap()
            .group_by(&["class_id"]).unwrap()
            .having(AggregateFunction::Avg("score".to_string()), Operator::Gt, ExtractedValue::Float(75.0)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("HAVING AVG(\"score\") > $"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_having_with_count_distinct() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::CountDistinct("customer_id".to_string()), Some("unique_customers")).unwrap()
            .group_by(&["region"]).unwrap()
            .having(AggregateFunction::CountDistinct("customer_id".to_string()), Operator::Gte, ExtractedValue::Int(50)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("HAVING COUNT(DISTINCT \"customer_id\") >= $"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_having_with_min_max() {
        let qb = QueryBuilder::new("products").unwrap()
            .aggregate(AggregateFunction::Max("price".to_string()), Some("max_price")).unwrap()
            .group_by(&["category"]).unwrap()
            .having(AggregateFunction::Max("price".to_string()), Operator::Lt, ExtractedValue::Float(1000.0)).unwrap();

        let (sql, params) = qb.build_select();
        assert!(sql.contains("HAVING MAX(\"price\") < $"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_clear_having() {
        let qb = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, Some("order_count")).unwrap()
            .group_by(&["status"]).unwrap()
            .having(AggregateFunction::Count, Operator::Gt, ExtractedValue::Int(10)).unwrap()
            .clear_having();

        let (sql, params) = qb.build_select();
        assert!(!sql.contains("HAVING"));
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_having_invalid_column() {
        let result = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Count, None).unwrap()
            .group_by(&["status"]).unwrap()
            .having(AggregateFunction::Sum("drop".to_string()), Operator::Gt, ExtractedValue::Int(100));
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_having_query() {
        let qb = QueryBuilder::new("sales").unwrap()
            .aggregate(AggregateFunction::Sum("revenue".to_string()), Some("total_revenue")).unwrap()
            .aggregate(AggregateFunction::Count, Some("sale_count")).unwrap()
            .aggregate(AggregateFunction::Avg("revenue".to_string()), Some("avg_revenue")).unwrap()
            .where_clause("year", Operator::Eq, ExtractedValue::Int(2024)).unwrap()
            .group_by(&["region", "category"]).unwrap()
            .having(AggregateFunction::Sum("revenue".to_string()), Operator::Gte, ExtractedValue::Float(100000.0)).unwrap()
            .having(AggregateFunction::Count, Operator::Gt, ExtractedValue::Int(10)).unwrap()
            .order_by("total_revenue", OrderDirection::Desc).unwrap()
            .limit(20);

        let (sql, params) = qb.build_select();
        assert!(sql.contains("SELECT \"region\", \"category\", SUM(\"revenue\") AS \"total_revenue\", COUNT(*) AS \"sale_count\", AVG(\"revenue\") AS \"avg_revenue\""));
        assert!(sql.contains("WHERE \"year\" = $1"));
        assert!(sql.contains("GROUP BY \"region\", \"category\""));
        assert!(sql.contains("HAVING SUM(\"revenue\") >= $2 AND COUNT(*) > $3"));
        assert!(sql.contains("ORDER BY \"total_revenue\" DESC"));
        assert!(sql.contains("LIMIT $4"));
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn test_distinct_basic() {
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec!["email".to_string()]).unwrap()
            .distinct();
        let (sql, _) = qb.build_select();
        assert!(sql.starts_with("SELECT DISTINCT "));
        assert!(sql.contains("\"email\""));
    }

    #[test]
    fn test_distinct_on_single_column() {
        let qb = QueryBuilder::new("orders").unwrap()
            .distinct_on(&["user_id"]).unwrap()
            .order_by("user_id", OrderDirection::Asc).unwrap();
        let (sql, _) = qb.build_select();
        assert!(sql.contains("DISTINCT ON (\"user_id\")"));
    }

    #[test]
    fn test_distinct_on_multiple_columns() {
        let qb = QueryBuilder::new("orders").unwrap()
            .distinct_on(&["user_id", "status"]).unwrap();
        let (sql, _) = qb.build_select();
        assert!(sql.contains("DISTINCT ON (\"user_id\", \"status\")"));
    }

    #[test]
    fn test_distinct_on_validates_columns() {
        let result = QueryBuilder::new("orders").unwrap()
            .distinct_on(&["user_id; DROP TABLE orders"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_distinct_with_where_and_order() {
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec!["email".to_string(), "name".to_string()]).unwrap()
            .distinct()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap()
            .order_by("email", OrderDirection::Asc).unwrap();
        let (sql, _) = qb.build_select();
        assert!(sql.starts_with("SELECT DISTINCT "));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("ORDER BY"));
    }

    #[test]
    fn test_distinct_on_takes_precedence_over_distinct() {
        let qb = QueryBuilder::new("orders").unwrap()
            .distinct()
            .distinct_on(&["user_id"]).unwrap();
        let (sql, _) = qb.build_select();
        assert!(sql.contains("DISTINCT ON (\"user_id\")"));
        assert!(!sql.contains("SELECT DISTINCT DISTINCT ON"));
    }

    #[test]
    fn test_clear_distinct() {
        let qb = QueryBuilder::new("users").unwrap()
            .select(vec!["email".to_string()]).unwrap()
            .distinct()
            .clear_distinct();
        let (sql, _) = qb.build_select();
        assert!(!sql.contains("DISTINCT"));
    }

    #[test]
    fn test_clear_distinct_on() {
        let qb = QueryBuilder::new("orders").unwrap()
            .distinct_on(&["user_id"]).unwrap()
            .clear_distinct();
        let (sql, _) = qb.build_select();
        assert!(!sql.contains("DISTINCT ON"));
    }

    #[test]
    fn test_cte_basic() {
        let cte_query = QueryBuilder::new("orders").unwrap()
            .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total")).unwrap()
            .group_by(&["user_id"]).unwrap();

        let main_query = QueryBuilder::new("order_totals").unwrap()
            .with_cte("order_totals", cte_query).unwrap();

        let (sql, _) = main_query.build_select();
        assert!(sql.starts_with("WITH "));
        assert!(sql.contains("\"order_totals\" AS ("));
        assert!(sql.contains("SELECT \"user_id\", SUM(\"amount\") AS \"total\""));
        assert!(sql.contains("FROM \"orders\""));
        assert!(sql.contains("GROUP BY \"user_id\""));
        assert!(sql.contains(") SELECT * FROM \"order_totals\""));
    }

    #[test]
    fn test_cte_with_where() {
        let cte_query = QueryBuilder::new("orders").unwrap()
            .select(vec!["user_id".to_string(), "amount".to_string()]).unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("completed".to_string())).unwrap();

        let main_query = QueryBuilder::new("completed_orders").unwrap()
            .with_cte("completed_orders", cte_query).unwrap()
            .where_clause("amount", Operator::Gt, ExtractedValue::Float(100.0)).unwrap();

        let (sql, params) = main_query.build_select();
        assert!(sql.contains("WITH"));
        assert!(sql.contains("\"completed_orders\" AS ("));
        assert_eq!(params.len(), 2); // One from CTE, one from main query

        // First param is from CTE (status = 'completed')
        match &params[0] {
            ExtractedValue::String(s) => assert_eq!(s, "completed"),
            _ => panic!("Expected String for first param"),
        }

        // Second param is from main query (amount > 100.0)
        match &params[1] {
            ExtractedValue::Float(f) => assert_eq!(*f, 100.0),
            _ => panic!("Expected Float for second param"),
        }
    }

    #[test]
    fn test_cte_name_validation() {
        let cte_query = QueryBuilder::new("orders").unwrap();
        let result = QueryBuilder::new("test").unwrap()
            .with_cte("invalid; DROP TABLE", cte_query);
        assert!(result.is_err());
        if let Err(e) = result {
            // Should contain error about invalid characters (semicolon and space)
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("invalid character") || error_msg.contains("contains invalid"));
        }
    }

    #[test]
    fn test_multiple_ctes() {
        let cte1 = QueryBuilder::new("orders").unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("completed".to_string())).unwrap();
        let cte2 = QueryBuilder::new("users").unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap();

        let main = QueryBuilder::new("completed_orders").unwrap()
            .with_cte("completed_orders", cte1).unwrap()
            .with_cte("active_users", cte2).unwrap();

        let (sql, params) = main.build_select();
        assert!(sql.contains("WITH \"completed_orders\" AS"));
        assert!(sql.contains("\"active_users\" AS"));
        assert_eq!(params.len(), 2);

        // Verify params are in correct order
        match &params[0] {
            ExtractedValue::String(s) => assert_eq!(s, "completed"),
            _ => panic!("Expected String for first param"),
        }
        match &params[1] {
            ExtractedValue::Bool(b) => assert_eq!(*b, true),
            _ => panic!("Expected Bool for second param"),
        }
    }

    #[test]
    fn test_cte_with_raw_sql() {
        let main_query = QueryBuilder::new("high_value").unwrap()
            .with_cte_raw(
                "high_value",
                "SELECT user_id, SUM(amount) as total FROM orders GROUP BY user_id HAVING SUM(amount) > $1",
                vec![ExtractedValue::Float(1000.0)]
            ).unwrap()
            .where_clause("total", Operator::Gt, ExtractedValue::Float(5000.0)).unwrap();

        let (sql, params) = main_query.build_select();
        assert!(sql.starts_with("WITH "));
        assert!(sql.contains("\"high_value\" AS ("));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_clear_ctes() {
        let cte_query = QueryBuilder::new("orders").unwrap();
        let main_query = QueryBuilder::new("test").unwrap()
            .with_cte("cte1", cte_query).unwrap()
            .clear_ctes();

        let (sql, _) = main_query.build_select();
        assert!(!sql.contains("WITH"));
    }

    #[test]
    fn test_cte_parameter_adjustment() {
        // Test that parameter indices are correctly adjusted when combining CTEs
        let cte1 = QueryBuilder::new("t1").unwrap()
            .where_clause("a", Operator::Eq, ExtractedValue::Int(1)).unwrap();
        let cte2 = QueryBuilder::new("t2").unwrap()
            .where_clause("b", Operator::Eq, ExtractedValue::Int(2)).unwrap();

        let main = QueryBuilder::new("result").unwrap()
            .with_cte("cte1", cte1).unwrap()
            .with_cte("cte2", cte2).unwrap()
            .where_clause("c", Operator::Eq, ExtractedValue::Int(3)).unwrap();

        let (sql, params) = main.build_select();

        // The SQL should have properly adjusted parameter indices
        // CTE1 uses $1, CTE2 should use $2, main query should use $3
        assert!(sql.contains("WITH \"cte1\" AS (SELECT * FROM \"t1\" WHERE \"a\" = $1), \"cte2\" AS (SELECT * FROM \"t2\" WHERE \"b\" = $2)"));
        assert!(sql.contains("WHERE \"c\" = $3"));

        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ExtractedValue::Int(1));
        assert_eq!(params[1], ExtractedValue::Int(2));
        assert_eq!(params[2], ExtractedValue::Int(3));
    }

    #[test]
    fn test_where_in_subquery() {
        let subquery = QueryBuilder::new("orders").unwrap()
            .select(vec!["user_id".to_string()]).unwrap()
            .where_clause("total", Operator::Gt, ExtractedValue::Float(1000.0)).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_in_subquery("id", subquery).unwrap();

        let (sql, params) = main.build_select();
        assert!(sql.contains("WHERE \"id\" IN (SELECT"));
        assert!(sql.contains("\"user_id\""));
        assert!(sql.contains("\"total\" > $1"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ExtractedValue::Float(1000.0));
    }

    #[test]
    fn test_where_not_in_subquery() {
        let subquery = QueryBuilder::new("inactive_users").unwrap()
            .select(vec!["user_id".to_string()]).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_not_in_subquery("id", subquery).unwrap();

        let (sql, _) = main.build_select();
        assert!(sql.contains("WHERE \"id\" NOT IN (SELECT"));
        assert!(sql.contains("\"user_id\""));
    }

    #[test]
    fn test_where_exists() {
        let subquery = QueryBuilder::new("orders").unwrap()
            .where_clause("user_id", Operator::Eq, ExtractedValue::Int(1)).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_exists(subquery).unwrap();

        let (sql, params) = main.build_select();
        assert!(sql.contains("WHERE EXISTS (SELECT"));
        assert!(sql.contains("\"user_id\" = $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_where_not_exists() {
        let subquery = QueryBuilder::new("orders").unwrap()
            .where_clause("user_id", Operator::Eq, ExtractedValue::Int(1)).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_not_exists(subquery).unwrap();

        let (sql, params) = main.build_select();
        assert!(sql.contains("WHERE NOT EXISTS (SELECT"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_subquery_with_multiple_params() {
        let subquery = QueryBuilder::new("orders").unwrap()
            .where_clause("total", Operator::Gt, ExtractedValue::Float(100.0)).unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("completed".to_string())).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap()
            .where_in_subquery("id", subquery).unwrap();

        let (sql, params) = main.build_select();
        assert_eq!(params.len(), 3); // 1 from main + 2 from subquery
        // Main query param comes first
        assert_eq!(params[0], ExtractedValue::Bool(true));
        // Then subquery params
        assert_eq!(params[1], ExtractedValue::Float(100.0));
        assert_eq!(params[2], ExtractedValue::String("completed".to_string()));

        // Verify parameter indices are adjusted correctly
        assert!(sql.contains("\"active\" = $1"));
        assert!(sql.contains("\"total\" > $2"));
        assert!(sql.contains("\"status\" = $3"));
    }

    #[test]
    fn test_subquery_parameter_index_adjustment() {
        // Test that subquery parameter indices are correctly adjusted
        let subquery = QueryBuilder::new("orders").unwrap()
            .where_clause("total", Operator::Gt, ExtractedValue::Float(500.0)).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_clause("age", Operator::Gte, ExtractedValue::Int(18)).unwrap()
            .where_clause("status", Operator::Eq, ExtractedValue::String("active".to_string())).unwrap()
            .where_in_subquery("id", subquery).unwrap();

        let (sql, params) = main.build_select();

        // Should have 3 params: age, status, total
        assert_eq!(params.len(), 3);

        // Main query params should use $1 and $2
        assert!(sql.contains("\"age\" >= $1"));
        assert!(sql.contains("\"status\" = $2"));

        // Subquery param should be adjusted to $3
        assert!(sql.contains("\"total\" > $3"));
    }

    #[test]
    fn test_multiple_subqueries() {
        let subquery1 = QueryBuilder::new("orders").unwrap()
            .select(vec!["user_id".to_string()]).unwrap()
            .where_clause("total", Operator::Gt, ExtractedValue::Float(1000.0)).unwrap();

        let subquery2 = QueryBuilder::new("banned_users").unwrap()
            .select(vec!["user_id".to_string()]).unwrap();

        let main = QueryBuilder::new("users").unwrap()
            .where_in_subquery("id", subquery1).unwrap()
            .where_not_in_subquery("id", subquery2).unwrap();

        let (sql, params) = main.build_select();
        assert!(sql.contains("\"id\" IN (SELECT"));
        assert!(sql.contains("\"id\" NOT IN (SELECT"));
        assert_eq!(params.len(), 1);
    }
}
#[test]
fn debug_cte_sql() {
    let cte_query = QueryBuilder::new("orders").unwrap()
        .aggregate(AggregateFunction::Sum("amount".to_string()), Some("total")).unwrap()
        .group_by(&["user_id"]).unwrap();

    let main_query = QueryBuilder::new("order_totals").unwrap()
        .with_cte("order_totals", cte_query).unwrap()
        .where_clause("total", Operator::Gt, ExtractedValue::Float(1000.0)).unwrap();

    let (sql, params) = main_query.build_select();
    eprintln!("Generated SQL:\n{}", sql);
    eprintln!("Parameters: {:?}", params);

    // Verify the structure
    assert!(sql.starts_with("WITH "));
    assert!(sql.contains("\"order_totals\" AS ("));
}

#[test]
fn test_window_row_number() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .window(
            WindowFunction::RowNumber,
            WindowSpec::new().order_by("amount", OrderDirection::Desc),
            "rank",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("ROW_NUMBER() OVER (ORDER BY \"amount\" DESC) AS \"rank\""));
}

#[test]
fn test_window_with_partition() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .window(
            WindowFunction::Sum("amount".to_string()),
            WindowSpec::new()
                .partition_by(&["user_id"])
                .order_by("created_at", OrderDirection::Asc),
            "running_total",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("SUM(\"amount\") OVER"));
    assert!(sql.contains("PARTITION BY \"user_id\""));
    assert!(sql.contains("ORDER BY \"created_at\" ASC"));
    assert!(sql.contains("AS \"running_total\""));
}

#[test]
fn test_window_lag() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .window(
            WindowFunction::Lag("amount".to_string(), Some(1), None),
            WindowSpec::new().order_by("created_at", OrderDirection::Asc),
            "prev_amount",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("LAG(\"amount\", 1) OVER"));
    assert!(sql.contains("ORDER BY \"created_at\" ASC"));
    assert!(sql.contains("AS \"prev_amount\""));
}

#[test]
fn test_window_multiple_functions() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .window(
            WindowFunction::RowNumber,
            WindowSpec::new().order_by("amount", OrderDirection::Desc),
            "rank",
        )
        .unwrap()
        .window(
            WindowFunction::Sum("amount".to_string()),
            WindowSpec::new().partition_by(&["user_id"]),
            "user_total",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("ROW_NUMBER()"));
    assert!(sql.contains("SUM(\"amount\")"));
    assert!(sql.contains("AS \"rank\""));
    assert!(sql.contains("AS \"user_total\""));
}

#[test]
fn test_window_with_select_columns() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .select(vec!["id".to_string(), "amount".to_string()])
        .unwrap()
        .window(
            WindowFunction::RowNumber,
            WindowSpec::new().order_by("amount", OrderDirection::Desc),
            "rank",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("\"id\", \"amount\""));
    assert!(sql.contains("ROW_NUMBER() OVER"));
}

#[test]
fn test_window_ntile() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .window(
            WindowFunction::Ntile(4),
            WindowSpec::new().order_by("amount", OrderDirection::Desc),
            "quartile",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("NTILE(4) OVER"));
    assert!(sql.contains("AS \"quartile\""));
}

#[test]
fn test_window_first_last_value() {
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .window(
            WindowFunction::FirstValue("amount".to_string()),
            WindowSpec::new()
                .partition_by(&["user_id"])
                .order_by("created_at", OrderDirection::Asc),
            "first_amount",
        )
        .unwrap()
        .window(
            WindowFunction::LastValue("amount".to_string()),
            WindowSpec::new()
                .partition_by(&["user_id"])
                .order_by("created_at", OrderDirection::Asc),
            "last_amount",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("FIRST_VALUE(\"amount\")"));
    assert!(sql.contains("LAST_VALUE(\"amount\")"));
}

#[test]
fn debug_window_sql() {
    // This test demonstrates the generated SQL for window functions
    let qb = QueryBuilder::new("orders")
        .unwrap()
        .select(vec!["id".to_string(), "user_id".to_string(), "amount".to_string()])
        .unwrap()
        .window(
            WindowFunction::RowNumber,
            WindowSpec::new().order_by("amount", OrderDirection::Desc),
            "rank",
        )
        .unwrap()
        .window(
            WindowFunction::Sum("amount".to_string()),
            WindowSpec::new()
                .partition_by(&["user_id"])
                .order_by("created_at", OrderDirection::Asc),
            "running_total",
        )
        .unwrap();
    let (sql, _) = qb.build_select();
    eprintln!("Generated Window SQL:\n{}", sql);

    // Verify the expected SQL structure
    assert!(sql.contains("SELECT \"id\", \"user_id\", \"amount\""));
    assert!(sql.contains("ROW_NUMBER() OVER (ORDER BY \"amount\" DESC) AS \"rank\""));
    assert!(sql.contains("SUM(\"amount\") OVER (PARTITION BY \"user_id\" ORDER BY \"created_at\" ASC) AS \"running_total\""));
}

#[test]
fn test_union_basic() {
    let q1 = QueryBuilder::new("active_users").unwrap()
        .select(vec!["id".to_string(), "name".to_string()]).unwrap();
    let q2 = QueryBuilder::new("archived_users").unwrap()
        .select(vec!["id".to_string(), "name".to_string()]).unwrap();

    let combined = q1.union(q2);
    let (sql, _) = combined.build_select();
    assert!(sql.contains(" UNION "));
    assert!(sql.contains("\"active_users\""));
    assert!(sql.contains("\"archived_users\""));
}

#[test]
fn test_union_all() {
    let q1 = QueryBuilder::new("orders").unwrap();
    let q2 = QueryBuilder::new("returns").unwrap();

    let combined = q1.union_all(q2);
    let (sql, _) = combined.build_select();
    assert!(sql.contains(" UNION ALL "));
}

#[test]
fn test_intersect() {
    let q1 = QueryBuilder::new("all_orders").unwrap()
        .select(vec!["id".to_string()]).unwrap();
    let q2 = QueryBuilder::new("paid_orders").unwrap()
        .select(vec!["id".to_string()]).unwrap();

    let combined = q1.intersect(q2);
    let (sql, _) = combined.build_select();
    assert!(sql.contains(" INTERSECT "));
}

#[test]
fn test_except() {
    let q1 = QueryBuilder::new("all_products").unwrap();
    let q2 = QueryBuilder::new("discontinued").unwrap();

    let combined = q1.except(q2);
    let (sql, _) = combined.build_select();
    assert!(sql.contains(" EXCEPT "));
}

#[test]
fn test_union_with_params() {
    let q1 = QueryBuilder::new("users").unwrap()
        .where_clause("active", Operator::Eq, ExtractedValue::Bool(true)).unwrap();
    let q2 = QueryBuilder::new("admins").unwrap()
        .where_clause("user_role", Operator::Eq, ExtractedValue::String("admin".to_string())).unwrap();

    let combined = q1.union(q2);
    let (sql, params) = combined.build_select();
    assert_eq!(params.len(), 2);
    // Check that parameter indices are adjusted
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));
}

#[test]
fn test_multiple_unions() {
    let q1 = QueryBuilder::new("table1").unwrap();
    let q2 = QueryBuilder::new("table2").unwrap();
    let q3 = QueryBuilder::new("table3").unwrap();

    let combined = q1.union(q2).union(q3);
    let (sql, _) = combined.build_select();
    // Should have two UNION keywords
    assert_eq!(sql.matches(" UNION ").count(), 2);
}

#[test]
fn test_set_operations_comprehensive() {
    // Test SQL generation for all set operations
    let q1 = QueryBuilder::new("active_users").unwrap()
        .select(vec!["id".to_string(), "name".to_string()]).unwrap();
    let q2 = QueryBuilder::new("archived_users").unwrap()
        .select(vec!["id".to_string(), "name".to_string()]).unwrap();

    let combined = q1.union(q2);
    let (sql, _) = combined.build_select();
    eprintln!("UNION SQL:\n{}", sql);
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"active_users\" UNION SELECT \"id\", \"name\" FROM \"archived_users\""
    );

    // Test INTERSECT ALL
    let q1 = QueryBuilder::new("orders").unwrap();
    let q2 = QueryBuilder::new("paid_orders").unwrap();
    let combined = q1.intersect_all(q2);
    let (sql, _) = combined.build_select();
    eprintln!("INTERSECT ALL SQL:\n{}", sql);
    assert!(sql.contains(" INTERSECT ALL "));

    // Test EXCEPT ALL
    let q1 = QueryBuilder::new("products").unwrap();
    let q2 = QueryBuilder::new("discontinued").unwrap();
    let combined = q1.except_all(q2);
    let (sql, _) = combined.build_select();
    eprintln!("EXCEPT ALL SQL:\n{}", sql);
    assert!(sql.contains(" EXCEPT ALL "));
}

#[test]
fn test_json_contains() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contains("metadata", r#"{"role": "admin"}"#).unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("@>"), "SQL should contain @> operator");
    assert!(sql.contains("::jsonb"), "SQL should cast to jsonb");
    assert!(sql.contains(r#"'{"role": "admin"}'"#), "SQL should contain JSON value");
}

#[test]
fn test_json_contained_by() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contained_by("metadata", r#"{"premium": true}"#).unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("<@"), "SQL should contain <@ operator");
    assert!(sql.contains("::jsonb"), "SQL should cast to jsonb");
}

#[test]
fn test_json_key_exists() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_key_exists("metadata", "email").unwrap();
    let (sql, params) = qb.build_select();
    assert!(sql.contains("?"), "SQL should contain ? operator");
    assert_eq!(params.len(), 1, "Should have one parameter");
    if let ExtractedValue::String(s) = &params[0] {
        assert_eq!(s, "email");
    } else {
        panic!("Expected String parameter");
    }
}

#[test]
fn test_json_any_key_exists() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_any_key_exists("metadata", &["email", "phone"]).unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("?|"), "SQL should contain ?| operator");
    assert!(sql.contains("ARRAY["), "SQL should contain ARRAY");
    assert!(sql.contains("'email'"), "SQL should contain email key");
    assert!(sql.contains("'phone'"), "SQL should contain phone key");
}

#[test]
fn test_json_all_keys_exist() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_all_keys_exist("metadata", &["email", "name"]).unwrap();
    let (sql, _) = qb.build_select();
    assert!(sql.contains("?&"), "SQL should contain ?& operator");
    assert!(sql.contains("ARRAY["), "SQL should contain ARRAY");
    assert!(sql.contains("'email'"), "SQL should contain email key");
    assert!(sql.contains("'name'"), "SQL should contain name key");
}

#[test]
fn test_json_contains_with_quote_escaping() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contains("metadata", r#"{"name": "O'Brien"}"#).unwrap();
    let (sql, _) = qb.build_select();
    // Single quotes should be escaped
    assert!(sql.contains("O''Brien"), "Single quotes should be escaped");
}

#[test]
fn test_multiple_json_conditions() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contains("metadata", r#"{"role": "admin"}"#).unwrap()
        .where_json_key_exists("metadata", "email").unwrap();
    let (sql, params) = qb.build_select();
    assert!(sql.contains("@>"), "SQL should contain @> operator");
    assert!(sql.contains("?"), "SQL should contain ? operator");
    assert!(sql.contains("AND"), "SQL should have AND between conditions");
    assert_eq!(params.len(), 1, "Should have one parameter for key exists");
}

#[test]
fn debug_json_sql() {
    // This test demonstrates the generated SQL for JSONB operations
    eprintln!("\n=== JSONB Operator SQL Generation ===\n");

    // Test 1: JSON contains
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contains("metadata", r#"{"role": "admin"}"#).unwrap();
    let (sql, params) = qb.build_select();
    eprintln!("1. JSON Contains (@>):");
    eprintln!("   SQL: {}", sql);
    eprintln!("   Params: {:?}\n", params);

    // Test 2: JSON contained by
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contained_by("settings", r#"{"theme": "dark"}"#).unwrap();
    let (sql, params) = qb.build_select();
    eprintln!("2. JSON Contained By (<@):");
    eprintln!("   SQL: {}", sql);
    eprintln!("   Params: {:?}\n", params);

    // Test 3: JSON key exists
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_key_exists("metadata", "email").unwrap();
    let (sql, params) = qb.build_select();
    eprintln!("3. JSON Key Exists (?):");
    eprintln!("   SQL: {}", sql);
    eprintln!("   Params: {:?}\n", params);

    // Test 4: JSON any key exists
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_any_key_exists("metadata", &["email", "phone"]).unwrap();
    let (sql, params) = qb.build_select();
    eprintln!("4. JSON Any Key Exists (?|):");
    eprintln!("   SQL: {}", sql);
    eprintln!("   Params: {:?}\n", params);

    // Test 5: JSON all keys exist
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_all_keys_exist("metadata", &["email", "name", "age"]).unwrap();
    let (sql, params) = qb.build_select();
    eprintln!("5. JSON All Keys Exist (?&):");
    eprintln!("   SQL: {}", sql);
    eprintln!("   Params: {:?}\n", params);

    // Test 6: Multiple conditions
    let qb = QueryBuilder::new("users").unwrap()
        .where_json_contains("metadata", r#"{"role": "admin"}"#).unwrap()
        .where_json_key_exists("metadata", "verified").unwrap()
        .where_json_all_keys_exist("profile", &["name", "email"]).unwrap();
    let (sql, params) = qb.build_select();
    eprintln!("6. Multiple JSONB Conditions:");
    eprintln!("   SQL: {}", sql);
    eprintln!("   Params: {:?}\n", params);

    eprintln!("=== End of JSONB SQL Examples ===\n");
}

#[test]
fn test_update_returning_specific_columns() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap()
        .returning(&["id", "name", "updated_at"]).unwrap();

    let (sql, _) = qb.build_update(&[("status".to_string(), ExtractedValue::String("inactive".to_string()))]).unwrap();
    assert!(sql.contains("RETURNING"));
    assert!(sql.contains("\"id\""));
    assert!(sql.contains("\"name\""));
    assert!(sql.contains("\"updated_at\""));
    assert_eq!(sql, "UPDATE \"users\" SET \"status\" = $1 WHERE \"id\" = $2 RETURNING \"id\", \"name\", \"updated_at\"");
}

#[test]
fn test_update_returning_all() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap()
        .returning_all();

    let (sql, _) = qb.build_update(&[("status".to_string(), ExtractedValue::String("inactive".to_string()))]).unwrap();
    assert!(sql.contains("RETURNING *"));
    assert_eq!(sql, "UPDATE \"users\" SET \"status\" = $1 WHERE \"id\" = $2 RETURNING *");
}

#[test]
fn test_delete_returning() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap()
        .returning(&["id", "email"]).unwrap();

    let (sql, _) = qb.build_delete();
    assert!(sql.contains("DELETE FROM"));
    assert!(sql.contains("RETURNING"));
    assert!(sql.contains("\"id\""));
    assert!(sql.contains("\"email\""));
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1 RETURNING \"id\", \"email\"");
}

#[test]
fn test_delete_returning_all() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap()
        .returning_all();

    let (sql, _) = qb.build_delete();
    assert!(sql.contains("RETURNING *"));
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1 RETURNING *");
}

#[test]
fn test_returning_validates_identifiers() {
    let result = QueryBuilder::new("users").unwrap()
        .returning(&["id; DROP TABLE users"]);
    assert!(result.is_err());
}

#[test]
fn test_returning_allows_asterisk() {
    let result = QueryBuilder::new("users").unwrap()
        .returning(&["*"]);
    assert!(result.is_ok());
    let qb = result.unwrap();
    let (sql, _) = qb.build_delete();
    assert!(sql.contains("RETURNING *"));
}

#[test]
fn test_clear_returning() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap()
        .returning(&["id", "name"]).unwrap()
        .clear_returning();

    let (sql, _) = qb.build_update(&[("status".to_string(), ExtractedValue::String("inactive".to_string()))]).unwrap();
    assert!(!sql.contains("RETURNING"));
    assert_eq!(sql, "UPDATE \"users\" SET \"status\" = $1 WHERE \"id\" = $2");
}

#[test]
fn test_update_without_returning() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap();

    let (sql, _) = qb.build_update(&[("status".to_string(), ExtractedValue::String("inactive".to_string()))]).unwrap();
    assert!(!sql.contains("RETURNING"));
}

#[test]
fn test_delete_without_returning() {
    let qb = QueryBuilder::new("users").unwrap()
        .where_clause("id", Operator::Eq, ExtractedValue::Int(1)).unwrap();

    let (sql, _) = qb.build_delete();
    assert!(!sql.contains("RETURNING"));
}
