//! Database schema introspection.
//!
//! This module provides utilities for inspecting the database schema,
//! useful for validation, documentation, and migration generation.

use crate::{Connection, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// PostgreSQL column data type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    /// SMALLINT
    SmallInt,
    /// INTEGER
    Integer,
    /// BIGINT
    BigInt,
    /// REAL
    Real,
    /// DOUBLE PRECISION
    DoublePrecision,
    /// NUMERIC(precision, scale)
    Numeric(Option<i32>, Option<i32>),
    /// VARCHAR(length)
    Varchar(Option<i32>),
    /// TEXT
    Text,
    /// BOOLEAN
    Boolean,
    /// BYTEA
    Bytea,
    /// UUID
    Uuid,
    /// DATE
    Date,
    /// TIME
    Time,
    /// TIMESTAMP
    Timestamp,
    /// TIMESTAMPTZ
    TimestampTz,
    /// JSON
    Json,
    /// JSONB
    Jsonb,
    /// ARRAY of type
    Array(Box<ColumnType>),
    /// Custom/Unknown type
    Custom(String),
}

/// Represents a column in a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name
    pub name: String,
    /// Data type
    pub data_type: ColumnType,
    /// Is nullable
    pub nullable: bool,
    /// Default value expression
    pub default: Option<String>,
    /// Is primary key
    pub is_primary_key: bool,
    /// Is unique
    pub is_unique: bool,
}

/// Represents a table index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// Index name
    pub name: String,
    /// Columns in the index
    pub columns: Vec<String>,
    /// Is unique index
    pub is_unique: bool,
    /// Index type (btree, hash, gin, gist, etc.)
    pub index_type: String,
}

/// Represents a foreign key constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    /// Constraint name
    pub name: String,
    /// Source columns
    pub columns: Vec<String>,
    /// Referenced table
    pub referenced_table: String,
    /// Referenced columns
    pub referenced_columns: Vec<String>,
    /// ON DELETE action
    pub on_delete: String,
    /// ON UPDATE action
    pub on_update: String,
}

/// Represents a database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    /// Schema name (default: "public")
    pub schema: String,
    /// Columns
    pub columns: Vec<ColumnInfo>,
    /// Indexes
    pub indexes: Vec<IndexInfo>,
    /// Foreign keys
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

/// Schema introspection utilities.
pub struct SchemaInspector {
    conn: Connection,
}

impl SchemaInspector {
    /// Creates a new schema inspector.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Lists all tables in the database.
    pub async fn list_tables(&self, schema: Option<&str>) -> Result<Vec<String>> {
        let schema_name = schema.unwrap_or("public");

        let rows = sqlx::query(
            "SELECT tablename FROM pg_tables
             WHERE schemaname = $1
             ORDER BY tablename"
        )
        .bind(schema_name)
        .fetch_all(self.conn.pool())
        .await?;

        let tables = rows
            .iter()
            .filter_map(|row| row.try_get::<String, _>("tablename").ok())
            .collect();

        Ok(tables)
    }

    /// Gets detailed information about a table.
    pub async fn inspect_table(&self, table: &str, schema: Option<&str>) -> Result<TableInfo> {
        let schema_name = schema.unwrap_or("public");

        // Check if table exists
        if !self.table_exists(table, Some(schema_name)).await? {
            return Err(crate::DataBridgeError::Query(format!(
                "Table '{}.{}' does not exist",
                schema_name, table
            )));
        }

        // Get columns, indexes, and foreign keys
        let columns = self.get_columns(table, Some(schema_name)).await?;
        let indexes = self.get_indexes(table, Some(schema_name)).await?;
        let foreign_keys = self.get_foreign_keys(table, Some(schema_name)).await?;

        Ok(TableInfo {
            name: table.to_string(),
            schema: schema_name.to_string(),
            columns,
            indexes,
            foreign_keys,
        })
    }

    /// Checks if a table exists.
    pub async fn table_exists(&self, table: &str, schema: Option<&str>) -> Result<bool> {
        let schema_name = schema.unwrap_or("public");

        let row = sqlx::query(
            "SELECT EXISTS (
                SELECT 1 FROM pg_tables
                WHERE tablename = $1 AND schemaname = $2
             ) as exists"
        )
        .bind(table)
        .bind(schema_name)
        .fetch_one(self.conn.pool())
        .await?;

        let exists: bool = row.try_get("exists")?;
        Ok(exists)
    }

    /// Gets column information for a table.
    pub async fn get_columns(&self, table: &str, schema: Option<&str>) -> Result<Vec<ColumnInfo>> {
        let schema_name = schema.unwrap_or("public");

        // Query column information from information_schema
        let rows = sqlx::query(
            "SELECT
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                c.character_maximum_length,
                c.numeric_precision,
                c.numeric_scale,
                CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key,
                CASE WHEN u.column_name IS NOT NULL THEN true ELSE false END as is_unique
             FROM information_schema.columns c
             LEFT JOIN (
                 SELECT ku.column_name
                 FROM information_schema.table_constraints tc
                 JOIN information_schema.key_column_usage ku
                     ON tc.constraint_name = ku.constraint_name
                     AND tc.table_schema = ku.table_schema
                 WHERE tc.constraint_type = 'PRIMARY KEY'
                     AND tc.table_name = $1
                     AND tc.table_schema = $2
             ) pk ON c.column_name = pk.column_name
             LEFT JOIN (
                 SELECT ku.column_name
                 FROM information_schema.table_constraints tc
                 JOIN information_schema.key_column_usage ku
                     ON tc.constraint_name = ku.constraint_name
                     AND tc.table_schema = ku.table_schema
                 WHERE tc.constraint_type = 'UNIQUE'
                     AND tc.table_name = $1
                     AND tc.table_schema = $2
             ) u ON c.column_name = u.column_name
             WHERE c.table_name = $1 AND c.table_schema = $2
             ORDER BY c.ordinal_position"
        )
        .bind(table)
        .bind(schema_name)
        .fetch_all(self.conn.pool())
        .await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.try_get("column_name")?;
            let data_type_str: String = row.try_get("data_type")?;
            let nullable_str: String = row.try_get("is_nullable")?;
            let default: Option<String> = row.try_get("column_default").ok();
            let is_primary_key: bool = row.try_get("is_primary_key")?;
            let is_unique: bool = row.try_get("is_unique")?;

            // Parse data type
            let data_type = Self::parse_column_type(
                &data_type_str,
                row.try_get("character_maximum_length").ok(),
                row.try_get("numeric_precision").ok(),
                row.try_get("numeric_scale").ok(),
            );

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: nullable_str == "YES",
                default,
                is_primary_key,
                is_unique,
            });
        }

        Ok(columns)
    }

    /// Parses PostgreSQL data type string into ColumnType enum.
    fn parse_column_type(
        type_str: &str,
        max_length: Option<i32>,
        precision: Option<i32>,
        scale: Option<i32>,
    ) -> ColumnType {
        match type_str {
            "smallint" => ColumnType::SmallInt,
            "integer" => ColumnType::Integer,
            "bigint" => ColumnType::BigInt,
            "real" => ColumnType::Real,
            "double precision" => ColumnType::DoublePrecision,
            "numeric" | "decimal" => ColumnType::Numeric(precision, scale),
            "character varying" | "varchar" => ColumnType::Varchar(max_length),
            "text" => ColumnType::Text,
            "boolean" => ColumnType::Boolean,
            "bytea" => ColumnType::Bytea,
            "uuid" => ColumnType::Uuid,
            "date" => ColumnType::Date,
            "time" | "time without time zone" => ColumnType::Time,
            "timestamp" | "timestamp without time zone" => ColumnType::Timestamp,
            "timestamp with time zone" | "timestamptz" => ColumnType::TimestampTz,
            "json" => ColumnType::Json,
            "jsonb" => ColumnType::Jsonb,
            "ARRAY" => {
                // For arrays, we'd need more parsing, but for now use Custom
                ColumnType::Custom(type_str.to_string())
            }
            _ => ColumnType::Custom(type_str.to_string()),
        }
    }

    /// Gets index information for a table.
    pub async fn get_indexes(&self, table: &str, schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        let schema_name = schema.unwrap_or("public");

        // Query index information from pg_indexes and pg_class/pg_index
        let rows = sqlx::query(
            "SELECT
                i.indexname as name,
                i.indexdef as definition,
                ix.indisunique as is_unique,
                am.amname as index_type,
                ARRAY(
                    SELECT a.attname
                    FROM pg_attribute a
                    WHERE a.attrelid = ix.indexrelid
                    AND a.attnum > 0
                    ORDER BY a.attnum
                ) as columns
             FROM pg_indexes i
             JOIN pg_class c ON c.relname = i.tablename
             JOIN pg_index ix ON ix.indexrelid = (i.schemaname || '.' || i.indexname)::regclass
             JOIN pg_class ic ON ic.oid = ix.indexrelid
             JOIN pg_am am ON am.oid = ic.relam
             WHERE i.tablename = $1 AND i.schemaname = $2
             ORDER BY i.indexname"
        )
        .bind(table)
        .bind(schema_name)
        .fetch_all(self.conn.pool())
        .await?;

        let mut indexes = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")?;
            let is_unique: bool = row.try_get("is_unique")?;
            let index_type: String = row.try_get("index_type")?;
            let columns: Vec<String> = row.try_get("columns")?;

            indexes.push(IndexInfo {
                name,
                columns,
                is_unique,
                index_type,
            });
        }

        Ok(indexes)
    }

    /// Gets foreign key information for a table.
    ///
    /// Queries PostgreSQL's information_schema to retrieve foreign key constraints
    /// and their associated metadata.
    pub async fn get_foreign_keys(&self, table: &str, schema: Option<&str>) -> Result<Vec<ForeignKeyInfo>> {
        let schema_name = schema.unwrap_or("public");

        let rows = sqlx::query(
            "SELECT
                tc.constraint_name,
                ARRAY_AGG(DISTINCT kcu.column_name::TEXT ORDER BY kcu.column_name::TEXT)::TEXT[] as columns,
                ccu.table_name AS referenced_table,
                ARRAY_AGG(DISTINCT ccu.column_name::TEXT ORDER BY ccu.column_name::TEXT)::TEXT[] as referenced_columns,
                rc.update_rule,
                rc.delete_rule
            FROM information_schema.table_constraints AS tc
            JOIN information_schema.key_column_usage AS kcu
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            JOIN information_schema.constraint_column_usage AS ccu
                ON ccu.constraint_name = tc.constraint_name
                AND ccu.table_schema = tc.table_schema
            JOIN information_schema.referential_constraints AS rc
                ON rc.constraint_name = tc.constraint_name
                AND rc.constraint_schema = tc.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
                AND tc.table_name = $1
                AND tc.table_schema = $2
            GROUP BY tc.constraint_name, ccu.table_name, rc.update_rule, rc.delete_rule
            ORDER BY tc.constraint_name"
        )
        .bind(table)
        .bind(schema_name)
        .fetch_all(self.conn.pool())
        .await?;

        let mut foreign_keys = Vec::new();
        for row in rows {
            let name: String = row.try_get("constraint_name")?;
            let columns: Vec<String> = row.try_get("columns")?;
            let referenced_table: String = row.try_get("referenced_table")?;
            let referenced_columns: Vec<String> = row.try_get("referenced_columns")?;
            let update_rule: String = row.try_get("update_rule")?;
            let delete_rule: String = row.try_get("delete_rule")?;

            foreign_keys.push(ForeignKeyInfo {
                name,
                columns,
                referenced_table,
                referenced_columns,
                on_delete: delete_rule,
                on_update: update_rule,
            });
        }

        Ok(foreign_keys)
    }
}
