//! SELECT query building methods for QueryBuilder.

use crate::{ExtractedValue, Result};
use super::builder::{QueryBuilder, WhereCondition};
use super::types::{
    CommonTableExpression, Subquery, Operator, AggregateFunction, HavingCondition,
    OrderDirection, JoinType, SetOperation, SetQuery,
};
use super::join::{JoinClause, JoinCondition};
use super::window::{WindowFunction, WindowSpec, WindowExpression};
use super::helpers::{quote_identifier, build_aggregate_sql, build_window_sql, adjust_param_indices};

impl QueryBuilder {
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

    /// Defer loading of specific columns (exclude from initial SELECT).
    ///
    /// This is useful for optimizing queries that don't need large columns
    /// like blobs or text fields in the initial fetch.
    ///
    /// # Arguments
    ///
    /// * `columns` - Column names to defer loading
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Load users without the large_blob column
    /// let query = QueryBuilder::new("users")?
    ///     .defer(&["large_blob", "full_description"])?
    ///     .build();
    /// // Deferred columns will be excluded from SELECT
    /// ```
    pub fn defer(mut self, columns: &[&str]) -> Result<Self> {
        for col in columns {
            Self::validate_identifier(col)?;
            self.deferred_columns.push(col.to_string());
        }
        Ok(self)
    }

    /// Select only the specified columns (all other columns are excluded).
    ///
    /// This is the opposite of `defer()` - instead of excluding specific columns,
    /// you specify exactly which columns to include.
    ///
    /// # Arguments
    ///
    /// * `columns` - Column names to select exclusively
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Only load id and name columns
    /// let query = QueryBuilder::new("users")?
    ///     .only(&["id", "name"])?
    ///     .build();
    /// // Generated SQL: SELECT "id", "name" FROM "users"
    /// ```
    pub fn only(mut self, columns: &[&str]) -> Result<Self> {
        let mut cols = Vec::with_capacity(columns.len());
        for col in columns {
            Self::validate_identifier(col)?;
            cols.push(col.to_string());
        }
        self.only_columns = Some(cols);
        Ok(self)
    }

    /// Clear deferred columns
    pub fn clear_defer(mut self) -> Self {
        self.deferred_columns.clear();
        self
    }

    /// Clear only columns (revert to select all)
    pub fn clear_only(mut self) -> Self {
        self.only_columns = None;
        self
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
    pub fn where_exists(mut self, subquery: QueryBuilder) -> Result<Self> {
        let (sql, params) = subquery.build_select();
        self.where_conditions.push(WhereCondition {
            field: String::new(),
            operator: Operator::Exists,
            value: None,
            subquery: Some(Subquery { sql, params }),
        });
        Ok(self)
    }

    /// Add a WHERE NOT EXISTS (subquery) condition
    pub fn where_not_exists(mut self, subquery: QueryBuilder) -> Result<Self> {
        let (sql, params) = subquery.build_select();
        self.where_conditions.push(WhereCondition {
            field: String::new(),
            operator: Operator::NotExists,
            value: None,
            subquery: Some(Subquery { sql, params }),
        });
        Ok(self)
    }

    /// Add a WHERE column IN (raw SQL subquery) condition
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
    pub fn where_exists_raw_sql(mut self, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        self.where_conditions.push(WhereCondition {
            field: String::new(),
            operator: Operator::Exists,
            value: None,
            subquery: Some(Subquery { sql: sql.to_string(), params }),
        });
        Ok(self)
    }

    /// Add a WHERE NOT EXISTS (raw SQL subquery) condition
    pub fn where_not_exists_raw_sql(mut self, sql: &str, params: Vec<ExtractedValue>) -> Result<Self> {
        self.where_conditions.push(WhereCondition {
            field: String::new(),
            operator: Operator::NotExists,
            value: None,
            subquery: Some(Subquery { sql: sql.to_string(), params }),
        });
        Ok(self)
    }

    /// Filter where JSONB column contains the given JSON
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
    pub fn where_json_any_key_exists(mut self, field: &str, keys: &[&str]) -> Result<Self> {
        Self::validate_identifier(field)?;
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

    // Array operators

    /// Filter where value equals any element in the array column (value = ANY(array_column))
    ///
    /// # Example
    /// ```ignore
    /// // Find rows where 'admin' is in the roles array
    /// builder.where_any("roles", ExtractedValue::String("admin".to_string()))
    /// ```
    pub fn where_any(mut self, field: &str, value: ExtractedValue) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::Any,
            value: Some(value),
            subquery: None,
        });
        Ok(self)
    }

    /// Alias for where_any - Filter where array column has the given element
    pub fn where_has(self, field: &str, value: ExtractedValue) -> Result<Self> {
        self.where_any(field, value)
    }

    /// Filter where array column contains all the given values (array @> ARRAY[values])
    ///
    /// # Example
    /// ```ignore
    /// // Find rows where tags contain both "rust" and "postgres"
    /// builder.where_array_contains("tags", vec![
    ///     ExtractedValue::String("rust".to_string()),
    ///     ExtractedValue::String("postgres".to_string()),
    /// ])
    /// ```
    pub fn where_array_contains(mut self, field: &str, values: Vec<ExtractedValue>) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::ArrayContains,
            value: Some(ExtractedValue::Array(values)),
            subquery: None,
        });
        Ok(self)
    }

    /// Alias for where_array_contains - Filter where array has all the given elements
    pub fn where_has_all(self, field: &str, values: Vec<ExtractedValue>) -> Result<Self> {
        self.where_array_contains(field, values)
    }

    /// Filter where array column is contained by the given values (array <@ ARRAY[values])
    pub fn where_array_contained_by(mut self, field: &str, values: Vec<ExtractedValue>) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::ArrayContainedBy,
            value: Some(ExtractedValue::Array(values)),
            subquery: None,
        });
        Ok(self)
    }

    /// Filter where array column overlaps with given values (array && ARRAY[values])
    ///
    /// # Example
    /// ```ignore
    /// // Find rows where tags contain any of "rust", "python", or "go"
    /// builder.where_array_overlaps("tags", vec![
    ///     ExtractedValue::String("rust".to_string()),
    ///     ExtractedValue::String("python".to_string()),
    ///     ExtractedValue::String("go".to_string()),
    /// ])
    /// ```
    pub fn where_array_overlaps(mut self, field: &str, values: Vec<ExtractedValue>) -> Result<Self> {
        Self::validate_identifier(field)?;
        self.where_conditions.push(WhereCondition {
            field: field.to_string(),
            operator: Operator::ArrayOverlaps,
            value: Some(ExtractedValue::Array(values)),
            subquery: None,
        });
        Ok(self)
    }

    /// Alias for where_array_overlaps - Filter where array has any of the given elements
    pub fn where_has_any(self, field: &str, values: Vec<ExtractedValue>) -> Result<Self> {
        self.where_array_overlaps(field, values)
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
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Use DISTINCT ON to get first row for each unique combination (PostgreSQL-specific).
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

    /// Add a raw SQL CTE
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
    pub fn window(
        mut self,
        function: WindowFunction,
        spec: WindowSpec,
        alias: &str,
    ) -> Result<Self> {
        Self::validate_identifier(alias)?;
        for col in &spec.partition_by {
            Self::validate_identifier(col)?;
        }
        for (col, _) in &spec.order_by {
            Self::validate_identifier(col)?;
        }
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
    pub fn inner_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Inner, table, alias, condition)
    }

    /// Add a LEFT JOIN.
    pub fn left_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Left, table, alias, condition)
    }

    /// Add a RIGHT JOIN.
    pub fn right_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Right, table, alias, condition)
    }

    /// Add a FULL OUTER JOIN.
    pub fn full_join(self, table: &str, alias: Option<&str>, condition: JoinCondition) -> Result<Self> {
        self.join(JoinType::Full, table, alias, condition)
    }

    /// Add an aggregate function to the query.
    pub fn aggregate(mut self, func: AggregateFunction, alias: Option<&str>) -> Result<Self> {
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
    pub fn having(
        mut self,
        aggregate: AggregateFunction,
        operator: Operator,
        value: ExtractedValue,
    ) -> Result<Self> {
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
                    let cte_param_offset = params.len();
                    params.extend(cte.params.clone());
                    let adjusted_sql = adjust_param_indices(&cte.sql, cte_param_offset);
                    format!("{} AS ({})", quote_identifier(&cte.name), adjusted_sql)
                })
                .collect();
            sql.push_str(&cte_parts.join(", "));
            sql.push(' ');
        }

        // SELECT clause
        sql.push_str("SELECT ");

        // Add DISTINCT ON if specified
        if !self.distinct_on_columns.is_empty() {
            let cols: Vec<String> = self.distinct_on_columns
                .iter()
                .map(|c| quote_identifier(c))
                .collect();
            sql.push_str(&format!("DISTINCT ON ({}) ", cols.join(", ")));
        } else if self.distinct {
            sql.push_str("DISTINCT ");
        }

        // SELECT columns or aggregates or *
        let mut select_parts = Vec::new();

        if !self.aggregates.is_empty() {
            for col in &self.group_by_columns {
                select_parts.push(quote_identifier(col));
            }

            let agg_parts: Vec<String> = self.aggregates.iter()
                .map(|(func, alias)| {
                    let agg_sql = build_aggregate_sql(func);
                    if let Some(alias_str) = alias {
                        format!("{} AS {}", agg_sql, quote_identifier(alias_str))
                    } else {
                        agg_sql
                    }
                })
                .collect();
            select_parts.extend(agg_parts);
        } else if let Some(ref only_cols) = self.only_columns {
            // `only()` takes precedence - select exactly these columns
            let quoted_cols: Vec<String> = only_cols.iter()
                .map(|c| quote_identifier(c))
                .collect();
            select_parts.extend(quoted_cols);
        } else if !self.select_columns.is_empty() {
            // Filter out deferred columns from select_columns
            let quoted_cols: Vec<String> = self.select_columns.iter()
                .filter(|c| !self.deferred_columns.contains(c))
                .map(|c| quote_identifier(c))
                .collect();
            select_parts.extend(quoted_cols);
        }

        // Add window functions
        for expr in &self.windows {
            select_parts.push(build_window_sql(expr));
        }

        if select_parts.is_empty() {
            // If deferred columns exist but no explicit columns, we can't use * easily
            // For now, we output * (caller should use only() or select() for deferred loading)
            // In a full implementation, we'd introspect the schema to exclude deferred columns
            sql.push('*');
        } else {
            sql.push_str(&select_parts.join(", "));
        }

        sql.push_str(" FROM ");
        sql.push_str(&quote_identifier(&self.table));

        // JOIN clauses
        for join in &self.joins {
            let table_ref = match &join.alias {
                Some(alias) => format!("{} AS \"{}\"", quote_identifier(&join.table), alias),
                None => quote_identifier(&join.table),
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
                let part = self.build_where_condition(cond, &mut params);
                where_parts.push(part);
            }

            sql.push_str(&where_parts.join(" AND "));
        }

        // GROUP BY clause
        if !self.group_by_columns.is_empty() {
            sql.push_str(" GROUP BY ");
            let group_parts: Vec<String> = self.group_by_columns.iter()
                .map(|col| quote_identifier(col))
                .collect();
            sql.push_str(&group_parts.join(", "));
        }

        // HAVING clause
        if !self.having_conditions.is_empty() {
            sql.push_str(" HAVING ");
            let having_parts: Vec<String> = self.having_conditions
                .iter()
                .map(|cond| {
                    let agg_sql = build_aggregate_sql(&cond.aggregate);
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
                .map(|(field, dir)| format!("{} {}", quote_identifier(field), dir.to_sql()))
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

        // Set operations
        for set_op in &self.set_operations {
            sql.push_str(set_op.operation.to_sql());
            let adjusted_sql = adjust_param_indices(&set_op.sql, params.len());
            sql.push_str(&adjusted_sql);
            params.extend(set_op.params.clone());
        }

        (sql, params)
    }

    /// Helper to build a single WHERE condition
    fn build_where_condition(&self, cond: &WhereCondition, params: &mut Vec<ExtractedValue>) -> String {
        match cond.operator {
            Operator::InSubquery => {
                if let Some(ref sq) = cond.subquery {
                    let adjusted_sql = adjust_param_indices(&sq.sql, params.len());
                    params.extend(sq.params.clone());
                    format!("{} IN ({})", quote_identifier(&cond.field), adjusted_sql)
                } else {
                    format!("{} IN (NULL)", quote_identifier(&cond.field))
                }
            }
            Operator::NotInSubquery => {
                if let Some(ref sq) = cond.subquery {
                    let adjusted_sql = adjust_param_indices(&sq.sql, params.len());
                    params.extend(sq.params.clone());
                    format!("{} NOT IN ({})", quote_identifier(&cond.field), adjusted_sql)
                } else {
                    format!("{} NOT IN (NULL)", quote_identifier(&cond.field))
                }
            }
            Operator::Exists => {
                if let Some(ref sq) = cond.subquery {
                    let adjusted_sql = adjust_param_indices(&sq.sql, params.len());
                    params.extend(sq.params.clone());
                    format!("EXISTS ({})", adjusted_sql)
                } else {
                    "EXISTS (NULL)".to_string()
                }
            }
            Operator::NotExists => {
                if let Some(ref sq) = cond.subquery {
                    let adjusted_sql = adjust_param_indices(&sq.sql, params.len());
                    params.extend(sq.params.clone());
                    format!("NOT EXISTS ({})", adjusted_sql)
                } else {
                    "NOT EXISTS (NULL)".to_string()
                }
            }
            Operator::IsNull | Operator::IsNotNull => {
                let quoted_field = quote_identifier(&cond.field);
                format!("{} {}", quoted_field, cond.operator.to_sql())
            }
            Operator::In | Operator::NotIn => {
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("{} {} (${})", quoted_field, cond.operator.to_sql(), params.len())
                } else {
                    format!("{} {} (NULL)", quoted_field, cond.operator.to_sql())
                }
            }
            Operator::JsonContains | Operator::JsonContainedBy => {
                if let Some(ExtractedValue::String(json)) = &cond.value {
                    format!("{} {} '{}'::jsonb",
                        quote_identifier(&cond.field),
                        cond.operator.to_sql(),
                        json.replace("'", "''")
                    )
                } else {
                    format!("{} {} NULL", quote_identifier(&cond.field), cond.operator.to_sql())
                }
            }
            Operator::JsonKeyExists => {
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                } else {
                    format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                }
            }
            Operator::JsonAnyKeyExists | Operator::JsonAllKeysExist => {
                if let Some(ExtractedValue::String(arr)) = &cond.value {
                    format!("{} {} {}",
                        quote_identifier(&cond.field),
                        cond.operator.to_sql(),
                        arr
                    )
                } else {
                    format!("{} {} NULL", quote_identifier(&cond.field), cond.operator.to_sql())
                }
            }
            // Array operators
            Operator::Any | Operator::Has => {
                // value = ANY(array_column)
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("${} = ANY({})", params.len(), quoted_field)
                } else {
                    format!("NULL = ANY({})", quoted_field)
                }
            }
            Operator::ArrayContains | Operator::HasAll => {
                // array_column @> ARRAY[values]
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("{} @> ${}", quoted_field, params.len())
                } else {
                    format!("{} @> NULL", quoted_field)
                }
            }
            Operator::ArrayContainedBy => {
                // array_column <@ ARRAY[values]
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("{} <@ ${}", quoted_field, params.len())
                } else {
                    format!("{} <@ NULL", quoted_field)
                }
            }
            Operator::ArrayOverlaps | Operator::HasAny => {
                // array_column && ARRAY[values]
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("{} && ${}", quoted_field, params.len())
                } else {
                    format!("{} && NULL", quoted_field)
                }
            }
            _ => {
                let quoted_field = quote_identifier(&cond.field);
                if let Some(ref value) = cond.value {
                    params.push(value.clone());
                    format!("{} {} ${}", quoted_field, cond.operator.to_sql(), params.len())
                } else {
                    format!("{} {} NULL", quoted_field, cond.operator.to_sql())
                }
            }
        }
    }
}
