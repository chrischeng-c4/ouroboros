//! Type mapping between Python and PostgreSQL.
//!
//! This module handles conversion between Python objects and PostgreSQL types,
//! similar to data-bridge-mongodb's BSON type handling.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::{Arguments, Column, Row as SqlxRow, Type, TypeInfo, Postgres};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{DataBridgeError, Result};

/// Represents a value extracted from Python for PostgreSQL conversion.
///
/// This enum captures all Python types that can be mapped to PostgreSQL types.
/// The conversion happens entirely in Rust to avoid Python heap pressure.
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractedValue {
    /// NULL value
    Null,
    /// Boolean (BOOLEAN)
    Bool(bool),
    /// Small integer (SMALLINT)
    SmallInt(i16),
    /// Integer (INTEGER)
    Int(i32),
    /// Big integer (BIGINT)
    BigInt(i64),
    /// Single-precision float (REAL)
    Float(f32),
    /// Double-precision float (DOUBLE PRECISION)
    Double(f64),
    /// Variable-length string (VARCHAR, TEXT)
    String(String),
    /// Binary data (BYTEA)
    Bytes(Vec<u8>),
    /// UUID (UUID)
    Uuid(Uuid),
    /// Date (DATE)
    Date(NaiveDate),
    /// Time (TIME)
    Time(NaiveTime),
    /// Timestamp without timezone (TIMESTAMP)
    Timestamp(NaiveDateTime),
    /// Timestamp with timezone (TIMESTAMPTZ)
    TimestampTz(DateTime<Utc>),
    /// JSON/JSONB (JSON, JSONB)
    Json(JsonValue),
    /// Array of values (ARRAY)
    Array(Vec<ExtractedValue>),
    /// Decimal/Numeric (NUMERIC, DECIMAL)
    Decimal(String), // Store as string to avoid precision loss
}

impl ExtractedValue {
    /// Converts Python object to ExtractedValue.
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL guard
    /// * `obj` - Python object to convert
    ///
    /// # Errors
    ///
    /// Returns error if Python type cannot be mapped to PostgreSQL type.
    pub fn from_python(/* py: Python, obj: &PyAny */) -> Result<Self> {
        // TODO: Implement Python to ExtractedValue conversion
        // - Handle None -> Null
        // - Handle bool -> Bool
        // - Handle int -> Int/BigInt (based on range)
        // - Handle float -> Double
        // - Handle str -> String
        // - Handle bytes -> Bytes
        // - Handle datetime -> TimestampTz
        // - Handle date -> Date
        // - Handle time -> Time
        // - Handle UUID -> Uuid
        // - Handle dict/list -> Json
        // - Handle list of same type -> Array
        // - Validate types and ranges
        todo!("Implement ExtractedValue::from_python")
    }

    /// Converts ExtractedValue to Python object.
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL guard
    ///
    /// # Errors
    ///
    /// Returns error if conversion fails.
    pub fn to_python(/* &self, py: Python */) -> Result<()> {
        // TODO: Implement ExtractedValue to Python conversion
        // - Null -> None
        // - Bool -> bool
        // - Int/BigInt -> int
        // - Double -> float
        // - String -> str
        // - Bytes -> bytes
        // - TimestampTz -> datetime
        // - Date -> date
        // - Time -> time
        // - Uuid -> UUID
        // - Json -> dict/list
        // - Array -> list
        todo!("Implement ExtractedValue::to_python")
    }

    /// Returns the PostgreSQL type name for this value.
    pub fn pg_type_name(&self) -> &'static str {
        match self {
            ExtractedValue::Null => "NULL",
            ExtractedValue::Bool(_) => "BOOLEAN",
            ExtractedValue::SmallInt(_) => "SMALLINT",
            ExtractedValue::Int(_) => "INTEGER",
            ExtractedValue::BigInt(_) => "BIGINT",
            ExtractedValue::Float(_) => "REAL",
            ExtractedValue::Double(_) => "DOUBLE PRECISION",
            ExtractedValue::String(_) => "TEXT",
            ExtractedValue::Bytes(_) => "BYTEA",
            ExtractedValue::Uuid(_) => "UUID",
            ExtractedValue::Date(_) => "DATE",
            ExtractedValue::Time(_) => "TIME",
            ExtractedValue::Timestamp(_) => "TIMESTAMP",
            ExtractedValue::TimestampTz(_) => "TIMESTAMPTZ",
            ExtractedValue::Json(_) => "JSONB",
            ExtractedValue::Array(_) => "ARRAY",
            ExtractedValue::Decimal(_) => "NUMERIC",
        }
    }

    /// Bind this value to a sqlx query.
    ///
    /// This method adds the value as a parameter to the query, enabling
    /// GIL-free query construction.
    ///
    /// # Arguments
    ///
    /// * `arguments` - Mutable reference to PgArguments for binding
    ///
    /// # Errors
    ///
    /// Returns error if binding fails (e.g., type incompatibility).
    pub fn bind_to_arguments(&self, arguments: &mut PgArguments) -> Result<()> {
        match self {
            ExtractedValue::Null => {
                // For null, we need to bind as a typed null
                // Using Option<i32> as a generic nullable type
                arguments.add(Option::<i32>::None)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind NULL: {}", e)))?;
            }
            ExtractedValue::Bool(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind BOOL: {}", e)))?;
            }
            ExtractedValue::SmallInt(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind SMALLINT: {}", e)))?;
            }
            ExtractedValue::Int(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind INT: {}", e)))?;
            }
            ExtractedValue::BigInt(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind BIGINT: {}", e)))?;
            }
            ExtractedValue::Float(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind FLOAT: {}", e)))?;
            }
            ExtractedValue::Double(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind DOUBLE: {}", e)))?;
            }
            ExtractedValue::String(v) => {
                arguments.add(v.as_str())
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind STRING: {}", e)))?;
            }
            ExtractedValue::Bytes(v) => {
                arguments.add(v.as_slice())
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind BYTES: {}", e)))?;
            }
            ExtractedValue::Uuid(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind UUID: {}", e)))?;
            }
            ExtractedValue::Date(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind DATE: {}", e)))?;
            }
            ExtractedValue::Time(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind TIME: {}", e)))?;
            }
            ExtractedValue::Timestamp(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind TIMESTAMP: {}", e)))?;
            }
            ExtractedValue::TimestampTz(v) => {
                arguments.add(*v)
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind TIMESTAMPTZ: {}", e)))?;
            }
            ExtractedValue::Json(v) => {
                arguments.add(v.clone())
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind JSON: {}", e)))?;
            }
            ExtractedValue::Array(values) => {
                // For arrays, we need to determine the element type and bind accordingly
                // This is a simplified implementation - in production, you'd want to handle
                // homogeneous arrays more efficiently
                if values.is_empty() {
                    // Empty array - bind as NULL array
                    arguments.add(Option::<Vec<i32>>::None)
                        .map_err(|e| DataBridgeError::Query(format!("Failed to bind empty ARRAY: {}", e)))?;
                } else {
                    // For now, convert to JSON array as a fallback
                    // This handles heterogeneous arrays but may not be optimal
                    let json_array: Vec<JsonValue> = values
                        .iter()
                        .map(extracted_to_json)
                        .collect::<Result<Vec<_>>>()?;
                    arguments.add(JsonValue::Array(json_array))
                        .map_err(|e| DataBridgeError::Query(format!("Failed to bind ARRAY: {}", e)))?;
                }
            }
            ExtractedValue::Decimal(v) => {
                // For now, bind decimals as strings
                // TODO: Enable rust_decimal feature in sqlx for native decimal support
                arguments.add(v.as_str())
                    .map_err(|e| DataBridgeError::Query(format!("Failed to bind DECIMAL: {}", e)))?;
            }
        }
        Ok(())
    }
}

/// Convert a PgRow to a HashMap of column name -> ExtractedValue.
///
/// This function enables GIL-free extraction of PostgreSQL rows into
/// our intermediate representation.
///
/// # Arguments
///
/// * `row` - PostgreSQL row from sqlx query result
///
/// # Errors
///
/// Returns error if column extraction or type conversion fails.
pub fn row_to_extracted(row: &PgRow) -> Result<HashMap<String, ExtractedValue>> {
    let mut columns = HashMap::new();

    // Iterate over all columns in the row
    for (idx, column) in row.columns().iter().enumerate() {
        let column_name = column.name().to_string();
        let type_info = column.type_info();
        let type_name = type_info.name();

        // Extract value based on PostgreSQL type
        let value = match type_name {
            "BOOL" | "BOOLEAN" => {
                match row.try_get::<Option<bool>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Bool(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract BOOL from column '{}': {}", column_name, e)
                    )),
                }
            }
            "INT2" | "SMALLINT" => {
                match row.try_get::<Option<i16>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::SmallInt(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract SMALLINT from column '{}': {}", column_name, e)
                    )),
                }
            }
            "INT4" | "INTEGER" | "INT" => {
                match row.try_get::<Option<i32>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Int(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract INT from column '{}': {}", column_name, e)
                    )),
                }
            }
            "INT8" | "BIGINT" => {
                match row.try_get::<Option<i64>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::BigInt(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract BIGINT from column '{}': {}", column_name, e)
                    )),
                }
            }
            "FLOAT4" | "REAL" => {
                match row.try_get::<Option<f32>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Float(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract FLOAT from column '{}': {}", column_name, e)
                    )),
                }
            }
            "FLOAT8" | "DOUBLE PRECISION" | "DOUBLE" => {
                match row.try_get::<Option<f64>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Double(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract DOUBLE from column '{}': {}", column_name, e)
                    )),
                }
            }
            "VARCHAR" | "TEXT" | "CHAR" | "BPCHAR" | "NAME" => {
                match row.try_get::<Option<String>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::String(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract STRING from column '{}': {}", column_name, e)
                    )),
                }
            }
            "BYTEA" => {
                match row.try_get::<Option<Vec<u8>>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Bytes(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract BYTES from column '{}': {}", column_name, e)
                    )),
                }
            }
            "UUID" => {
                match row.try_get::<Option<Uuid>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Uuid(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract UUID from column '{}': {}", column_name, e)
                    )),
                }
            }
            "DATE" => {
                match row.try_get::<Option<NaiveDate>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Date(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract DATE from column '{}': {}", column_name, e)
                    )),
                }
            }
            "TIME" => {
                match row.try_get::<Option<NaiveTime>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Time(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract TIME from column '{}': {}", column_name, e)
                    )),
                }
            }
            "TIMESTAMP" => {
                match row.try_get::<Option<NaiveDateTime>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Timestamp(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract TIMESTAMP from column '{}': {}", column_name, e)
                    )),
                }
            }
            "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => {
                match row.try_get::<Option<DateTime<Utc>>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::TimestampTz(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract TIMESTAMPTZ from column '{}': {}", column_name, e)
                    )),
                }
            }
            "JSON" | "JSONB" => {
                match row.try_get::<Option<JsonValue>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Json(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract JSON from column '{}': {}", column_name, e)
                    )),
                }
            }
            "NUMERIC" | "DECIMAL" => {
                // Extract as string for now
                // TODO: Enable rust_decimal feature in sqlx for native decimal support
                match row.try_get::<Option<String>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::Decimal(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract DECIMAL from column '{}': {}", column_name, e)
                    )),
                }
            }
            // Array types - handle common array types
            "_BOOL" => extract_array::<bool>(row, idx, &column_name, ExtractedValue::Bool)?,
            "_INT2" => extract_array::<i16>(row, idx, &column_name, ExtractedValue::SmallInt)?,
            "_INT4" => extract_array::<i32>(row, idx, &column_name, ExtractedValue::Int)?,
            "_INT8" => extract_array::<i64>(row, idx, &column_name, ExtractedValue::BigInt)?,
            "_FLOAT4" => extract_array::<f32>(row, idx, &column_name, ExtractedValue::Float)?,
            "_FLOAT8" => extract_array::<f64>(row, idx, &column_name, ExtractedValue::Double)?,
            "_TEXT" | "_VARCHAR" => extract_array::<String>(row, idx, &column_name, ExtractedValue::String)?,
            "_UUID" => extract_array::<Uuid>(row, idx, &column_name, ExtractedValue::Uuid)?,

            // Unknown type - try to extract as string as fallback
            unknown => {
                tracing::warn!("Unknown PostgreSQL type '{}' for column '{}', attempting string extraction", unknown, column_name);
                match row.try_get::<Option<String>, _>(idx) {
                    Ok(Some(v)) => ExtractedValue::String(v),
                    Ok(None) => ExtractedValue::Null,
                    Err(e) => return Err(DataBridgeError::Query(
                        format!("Failed to extract unknown type '{}' from column '{}': {}", unknown, column_name, e)
                    )),
                }
            }
        };

        columns.insert(column_name, value);
    }

    Ok(columns)
}

/// Helper function to extract arrays from PostgreSQL rows.
///
/// # Type Parameters
///
/// * `T` - Element type that implements sqlx Type and Clone
/// * `F` - Function to convert T to ExtractedValue
fn extract_array<T>(
    row: &PgRow,
    idx: usize,
    column_name: &str,
    convert: impl Fn(T) -> ExtractedValue
) -> Result<ExtractedValue>
where
    T: for<'a> sqlx::Decode<'a, Postgres> + Type<Postgres> + sqlx::postgres::PgHasArrayType,
{
    match row.try_get::<Option<Vec<T>>, _>(idx) {
        Ok(Some(vec)) => {
            let values: Vec<ExtractedValue> = vec.into_iter()
                .map(convert)
                .collect();
            Ok(ExtractedValue::Array(values))
        }
        Ok(None) => Ok(ExtractedValue::Null),
        Err(e) => Err(DataBridgeError::Query(
            format!("Failed to extract array from column '{}': {}", column_name, e)
        )),
    }
}

/// Helper function to convert ExtractedValue to JSON for array binding.
fn extracted_to_json(value: &ExtractedValue) -> Result<JsonValue> {
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
                .map(extracted_to_json)
                .collect::<Result<Vec<_>>>()?;
            JsonValue::Array(json_values)
        }
        ExtractedValue::Decimal(v) => JsonValue::String(v.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    #[allow(clippy::approx_constant)] // 3.14 is just a test value, not meant to be PI
    fn test_extracted_value_type_names() {
        assert_eq!(ExtractedValue::Null.pg_type_name(), "NULL");
        assert_eq!(ExtractedValue::Bool(true).pg_type_name(), "BOOLEAN");
        assert_eq!(ExtractedValue::SmallInt(42).pg_type_name(), "SMALLINT");
        assert_eq!(ExtractedValue::Int(42).pg_type_name(), "INTEGER");
        assert_eq!(ExtractedValue::BigInt(42).pg_type_name(), "BIGINT");
        assert_eq!(ExtractedValue::Float(3.14).pg_type_name(), "REAL");
        assert_eq!(ExtractedValue::Double(3.14).pg_type_name(), "DOUBLE PRECISION");
        assert_eq!(ExtractedValue::String("test".to_string()).pg_type_name(), "TEXT");
        assert_eq!(ExtractedValue::Bytes(vec![1, 2, 3]).pg_type_name(), "BYTEA");
        assert_eq!(ExtractedValue::Uuid(Uuid::nil()).pg_type_name(), "UUID");
        assert_eq!(ExtractedValue::Json(JsonValue::Null).pg_type_name(), "JSONB");
        assert_eq!(ExtractedValue::Array(vec![]).pg_type_name(), "ARRAY");
        assert_eq!(ExtractedValue::Decimal("123.45".to_string()).pg_type_name(), "NUMERIC");
    }

    #[test]
    fn test_bind_to_arguments() {
        let mut args = PgArguments::default();

        // Test binding various types
        let value = ExtractedValue::Int(42);
        assert!(value.bind_to_arguments(&mut args).is_ok());

        let value = ExtractedValue::String("test".to_string());
        assert!(value.bind_to_arguments(&mut args).is_ok());

        let value = ExtractedValue::Bool(true);
        assert!(value.bind_to_arguments(&mut args).is_ok());

        let value = ExtractedValue::Null;
        assert!(value.bind_to_arguments(&mut args).is_ok());

        let value = ExtractedValue::Uuid(Uuid::nil());
        assert!(value.bind_to_arguments(&mut args).is_ok());
    }

    #[test]
    fn test_extracted_to_json() {
        use serde_json::json;

        // Test basic types
        let result = extracted_to_json(&ExtractedValue::Null).unwrap();
        assert_eq!(result, json!(null));

        let result = extracted_to_json(&ExtractedValue::Bool(true)).unwrap();
        assert_eq!(result, json!(true));

        let result = extracted_to_json(&ExtractedValue::Int(42)).unwrap();
        assert_eq!(result, json!(42));

        let result = extracted_to_json(&ExtractedValue::String("test".to_string())).unwrap();
        assert_eq!(result, json!("test"));

        // Test bytes to hex conversion
        let result = extracted_to_json(&ExtractedValue::Bytes(vec![0xff, 0x00, 0xab])).unwrap();
        assert_eq!(result, json!("ff00ab"));

        // Test nested arrays
        let result = extracted_to_json(&ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Int(2),
            ExtractedValue::Int(3),
        ])).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_bind_array_values() {
        let mut args = PgArguments::default();

        // Empty array
        let value = ExtractedValue::Array(vec![]);
        assert!(value.bind_to_arguments(&mut args).is_ok());

        // Array with values (will be converted to JSON)
        let value = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Int(2),
        ]);
        assert!(value.bind_to_arguments(&mut args).is_ok());
    }

    #[test]
    fn test_null_handling_all_types() {
        use serde_json::json;

        // Verify that Null has correct type name
        assert_eq!(ExtractedValue::Null.pg_type_name(), "NULL");

        // Verify Null can be bound to arguments
        let mut args = PgArguments::default();
        assert!(ExtractedValue::Null.bind_to_arguments(&mut args).is_ok());

        // Verify Null converts to JSON null
        let result = extracted_to_json(&ExtractedValue::Null).unwrap();
        assert_eq!(result, json!(null));
    }

    #[test]
    fn test_string_type_conversion() {
        // Test String type
        let value = ExtractedValue::String("hello world".to_string());
        assert_eq!(value.pg_type_name(), "TEXT");

        // Test binding
        let mut args = PgArguments::default();
        assert!(value.bind_to_arguments(&mut args).is_ok());

        // Test JSON conversion
        let result = extracted_to_json(&value).unwrap();
        assert_eq!(result, serde_json::json!("hello world"));

        // Test empty string
        let empty_value = ExtractedValue::String(String::new());
        assert!(empty_value.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test string with special characters
        let special_value = ExtractedValue::String("test\n\t\"'\\".to_string());
        assert!(special_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
    }

    #[test]
    fn test_numeric_type_conversions() {
        use serde_json::json;

        // Test SmallInt (i16)
        let small_int = ExtractedValue::SmallInt(i16::MAX);
        assert_eq!(small_int.pg_type_name(), "SMALLINT");
        assert!(small_int.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&small_int).unwrap(), json!(i16::MAX));

        let small_int_min = ExtractedValue::SmallInt(i16::MIN);
        assert_eq!(extracted_to_json(&small_int_min).unwrap(), json!(i16::MIN));

        // Test Int (i32)
        let int = ExtractedValue::Int(i32::MAX);
        assert_eq!(int.pg_type_name(), "INTEGER");
        assert!(int.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&int).unwrap(), json!(i32::MAX));

        let int_min = ExtractedValue::Int(i32::MIN);
        assert_eq!(extracted_to_json(&int_min).unwrap(), json!(i32::MIN));

        // Test BigInt (i64)
        let big_int = ExtractedValue::BigInt(i64::MAX);
        assert_eq!(big_int.pg_type_name(), "BIGINT");
        assert!(big_int.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&big_int).unwrap(), json!(i64::MAX));

        let big_int_min = ExtractedValue::BigInt(i64::MIN);
        assert_eq!(extracted_to_json(&big_int_min).unwrap(), json!(i64::MIN));

        // Test zero values
        assert_eq!(extracted_to_json(&ExtractedValue::SmallInt(0)).unwrap(), json!(0));
        assert_eq!(extracted_to_json(&ExtractedValue::Int(0)).unwrap(), json!(0));
        assert_eq!(extracted_to_json(&ExtractedValue::BigInt(0)).unwrap(), json!(0));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_float_type_conversions() {
        use serde_json::json;

        // Test Float (f32)
        let float = ExtractedValue::Float(3.14_f32);
        assert_eq!(float.pg_type_name(), "REAL");
        assert!(float.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test Double (f64)
        let double = ExtractedValue::Double(3.141592653589793_f64);
        assert_eq!(double.pg_type_name(), "DOUBLE PRECISION");
        assert!(double.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test special float values
        let zero = ExtractedValue::Float(0.0);
        assert!(zero.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let negative = ExtractedValue::Float(-123.456);
        assert!(negative.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test NaN and Infinity (should convert to JSON null)
        let nan = ExtractedValue::Float(f32::NAN);
        assert_eq!(extracted_to_json(&nan).unwrap(), json!(null));

        let inf = ExtractedValue::Double(f64::INFINITY);
        assert_eq!(extracted_to_json(&inf).unwrap(), json!(null));

        let neg_inf = ExtractedValue::Double(f64::NEG_INFINITY);
        assert_eq!(extracted_to_json(&neg_inf).unwrap(), json!(null));
    }

    #[test]
    fn test_bool_type_conversion() {
        use serde_json::json;

        // Test true
        let true_value = ExtractedValue::Bool(true);
        assert_eq!(true_value.pg_type_name(), "BOOLEAN");
        assert!(true_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&true_value).unwrap(), json!(true));

        // Test false
        let false_value = ExtractedValue::Bool(false);
        assert_eq!(false_value.pg_type_name(), "BOOLEAN");
        assert!(false_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&false_value).unwrap(), json!(false));
    }

    #[test]
    fn test_datetime_type_conversions() {
        use chrono::{NaiveDate, NaiveTime, NaiveDateTime, DateTime, Utc};

        // Test Date
        let date = NaiveDate::from_ymd_opt(2025, 12, 26).unwrap();
        let date_value = ExtractedValue::Date(date);
        assert_eq!(date_value.pg_type_name(), "DATE");
        assert!(date_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        let json_result = extracted_to_json(&date_value).unwrap();
        assert!(json_result.is_string());

        // Test Time
        let time = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
        let time_value = ExtractedValue::Time(time);
        assert_eq!(time_value.pg_type_name(), "TIME");
        assert!(time_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        let json_result = extracted_to_json(&time_value).unwrap();
        assert!(json_result.is_string());

        // Test Timestamp (without timezone)
        let timestamp = NaiveDateTime::new(date, time);
        let timestamp_value = ExtractedValue::Timestamp(timestamp);
        assert_eq!(timestamp_value.pg_type_name(), "TIMESTAMP");
        assert!(timestamp_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        let json_result = extracted_to_json(&timestamp_value).unwrap();
        assert!(json_result.is_string());

        // Test TimestampTz (with timezone)
        let timestamp_tz = DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc);
        let timestamp_tz_value = ExtractedValue::TimestampTz(timestamp_tz);
        assert_eq!(timestamp_tz_value.pg_type_name(), "TIMESTAMPTZ");
        assert!(timestamp_tz_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        let json_result = extracted_to_json(&timestamp_tz_value).unwrap();
        assert!(json_result.is_string());
        // Should be RFC3339 format
        assert!(json_result.as_str().unwrap().contains('T'));
    }

    #[test]
    fn test_uuid_type_conversion() {
        use uuid::Uuid;
        use serde_json::json;

        // Test nil UUID
        let nil_uuid = ExtractedValue::Uuid(Uuid::nil());
        assert_eq!(nil_uuid.pg_type_name(), "UUID");
        assert!(nil_uuid.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(
            extracted_to_json(&nil_uuid).unwrap(),
            json!("00000000-0000-0000-0000-000000000000")
        );

        // Test random UUID
        let random_uuid = Uuid::new_v4();
        let uuid_value = ExtractedValue::Uuid(random_uuid);
        assert_eq!(uuid_value.pg_type_name(), "UUID");
        assert!(uuid_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        let json_result = extracted_to_json(&uuid_value).unwrap();
        assert!(json_result.is_string());
        assert_eq!(json_result.as_str().unwrap(), random_uuid.to_string());
    }

    #[test]
    fn test_json_type_conversion() {
        use serde_json::json;

        // Test JSON object
        let json_obj = json!({"key": "value", "number": 42});
        let json_value = ExtractedValue::Json(json_obj.clone());
        assert_eq!(json_value.pg_type_name(), "JSONB");
        assert!(json_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&json_value).unwrap(), json_obj);

        // Test JSON array
        let json_arr = json!([1, 2, 3, "four"]);
        let json_value = ExtractedValue::Json(json_arr.clone());
        assert!(json_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&json_value).unwrap(), json_arr);

        // Test JSON null
        let json_null = json!(null);
        let json_value = ExtractedValue::Json(json_null.clone());
        assert!(json_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&json_value).unwrap(), json_null);

        // Test nested JSON
        let nested = json!({
            "outer": {
                "inner": {
                    "deep": [1, 2, 3]
                }
            }
        });
        let json_value = ExtractedValue::Json(nested.clone());
        assert!(json_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&json_value).unwrap(), nested);
    }

    #[test]
    fn test_array_type_conversions() {
        use serde_json::json;

        // Test empty array
        let empty_array = ExtractedValue::Array(vec![]);
        assert_eq!(empty_array.pg_type_name(), "ARRAY");
        assert!(empty_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&empty_array).unwrap(), json!([]));

        // Test homogeneous integer array
        let int_array = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Int(2),
            ExtractedValue::Int(3),
        ]);
        assert!(int_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&int_array).unwrap(), json!([1, 2, 3]));

        // Test homogeneous string array
        let string_array = ExtractedValue::Array(vec![
            ExtractedValue::String("hello".to_string()),
            ExtractedValue::String("world".to_string()),
        ]);
        assert!(string_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&string_array).unwrap(), json!(["hello", "world"]));

        // Test heterogeneous array (mixed types)
        let mixed_array = ExtractedValue::Array(vec![
            ExtractedValue::Int(42),
            ExtractedValue::String("test".to_string()),
            ExtractedValue::Bool(true),
            ExtractedValue::Null,
        ]);
        assert!(mixed_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&mixed_array).unwrap(), json!([42, "test", true, null]));

        // Test nested array
        let nested_array = ExtractedValue::Array(vec![
            ExtractedValue::Array(vec![
                ExtractedValue::Int(1),
                ExtractedValue::Int(2),
            ]),
            ExtractedValue::Array(vec![
                ExtractedValue::Int(3),
                ExtractedValue::Int(4),
            ]),
        ]);
        assert!(nested_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&nested_array).unwrap(), json!([[1, 2], [3, 4]]));

        // Test array with null elements
        let array_with_nulls = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Null,
            ExtractedValue::Int(3),
        ]);
        assert!(array_with_nulls.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&array_with_nulls).unwrap(), json!([1, null, 3]));
    }

    #[test]
    fn test_option_type_conversions() {
        use serde_json::json;

        // ExtractedValue already represents Option through Null variant
        // Test that Null is properly handled
        let null_value = ExtractedValue::Null;
        assert_eq!(null_value.pg_type_name(), "NULL");
        assert!(null_value.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&null_value).unwrap(), json!(null));

        // Test array with Option-like behavior (nulls mixed with values)
        let array_with_options = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Null,
            ExtractedValue::Int(2),
            ExtractedValue::Null,
        ]);
        assert_eq!(
            extracted_to_json(&array_with_options).unwrap(),
            json!([1, null, 2, null])
        );
    }

    #[test]
    fn test_unknown_type_fallback() {
        // This test verifies the pg_type_name method covers all variants
        // In the actual row_to_extracted function, unknown types fall back to string extraction

        // Test that all known types return proper type names
        let all_types = vec![
            ExtractedValue::Null,
            ExtractedValue::Bool(true),
            ExtractedValue::SmallInt(1),
            ExtractedValue::Int(1),
            ExtractedValue::BigInt(1),
            ExtractedValue::Float(1.0),
            ExtractedValue::Double(1.0),
            ExtractedValue::String("test".to_string()),
            ExtractedValue::Bytes(vec![1, 2, 3]),
            ExtractedValue::Uuid(Uuid::nil()),
            ExtractedValue::Date(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ExtractedValue::Time(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
            ExtractedValue::Timestamp(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            )),
            ExtractedValue::TimestampTz(DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                ),
                Utc,
            )),
            ExtractedValue::Json(serde_json::json!(null)),
            ExtractedValue::Array(vec![]),
            ExtractedValue::Decimal("123.45".to_string()),
        ];

        // Verify all types have non-empty type names
        for extracted_value in all_types {
            let type_name = extracted_value.pg_type_name();
            assert!(!type_name.is_empty(), "Type name should not be empty");
            assert!(type_name.chars().all(|c| c.is_ascii_uppercase() || c.is_whitespace()),
                    "Type name should be uppercase: {}", type_name);
        }
    }

    #[test]
    fn test_type_name_mapping() {
        // Comprehensive test of pg_type_name() for all variants

        // NULL
        assert_eq!(ExtractedValue::Null.pg_type_name(), "NULL");

        // Boolean
        assert_eq!(ExtractedValue::Bool(true).pg_type_name(), "BOOLEAN");
        assert_eq!(ExtractedValue::Bool(false).pg_type_name(), "BOOLEAN");

        // Integer types
        assert_eq!(ExtractedValue::SmallInt(0).pg_type_name(), "SMALLINT");
        assert_eq!(ExtractedValue::SmallInt(i16::MAX).pg_type_name(), "SMALLINT");
        assert_eq!(ExtractedValue::SmallInt(i16::MIN).pg_type_name(), "SMALLINT");

        assert_eq!(ExtractedValue::Int(0).pg_type_name(), "INTEGER");
        assert_eq!(ExtractedValue::Int(i32::MAX).pg_type_name(), "INTEGER");
        assert_eq!(ExtractedValue::Int(i32::MIN).pg_type_name(), "INTEGER");

        assert_eq!(ExtractedValue::BigInt(0).pg_type_name(), "BIGINT");
        assert_eq!(ExtractedValue::BigInt(i64::MAX).pg_type_name(), "BIGINT");
        assert_eq!(ExtractedValue::BigInt(i64::MIN).pg_type_name(), "BIGINT");

        // Float types
        assert_eq!(ExtractedValue::Float(0.0).pg_type_name(), "REAL");
        assert_eq!(ExtractedValue::Float(f32::MAX).pg_type_name(), "REAL");
        assert_eq!(ExtractedValue::Float(f32::MIN).pg_type_name(), "REAL");

        assert_eq!(ExtractedValue::Double(0.0).pg_type_name(), "DOUBLE PRECISION");
        assert_eq!(ExtractedValue::Double(f64::MAX).pg_type_name(), "DOUBLE PRECISION");
        assert_eq!(ExtractedValue::Double(f64::MIN).pg_type_name(), "DOUBLE PRECISION");

        // String and bytes
        assert_eq!(ExtractedValue::String(String::new()).pg_type_name(), "TEXT");
        assert_eq!(ExtractedValue::String("test".to_string()).pg_type_name(), "TEXT");
        assert_eq!(ExtractedValue::Bytes(vec![]).pg_type_name(), "BYTEA");
        assert_eq!(ExtractedValue::Bytes(vec![1, 2, 3]).pg_type_name(), "BYTEA");

        // UUID
        assert_eq!(ExtractedValue::Uuid(Uuid::nil()).pg_type_name(), "UUID");
        assert_eq!(ExtractedValue::Uuid(Uuid::new_v4()).pg_type_name(), "UUID");

        // Date and time types
        assert_eq!(
            ExtractedValue::Date(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).pg_type_name(),
            "DATE"
        );
        assert_eq!(
            ExtractedValue::Time(NaiveTime::from_hms_opt(12, 0, 0).unwrap()).pg_type_name(),
            "TIME"
        );
        assert_eq!(
            ExtractedValue::Timestamp(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            )).pg_type_name(),
            "TIMESTAMP"
        );
        assert_eq!(
            ExtractedValue::TimestampTz(DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                ),
                Utc,
            )).pg_type_name(),
            "TIMESTAMPTZ"
        );

        // JSON
        assert_eq!(ExtractedValue::Json(serde_json::json!(null)).pg_type_name(), "JSONB");
        assert_eq!(ExtractedValue::Json(serde_json::json!({})).pg_type_name(), "JSONB");
        assert_eq!(ExtractedValue::Json(serde_json::json!([])).pg_type_name(), "JSONB");

        // Array
        assert_eq!(ExtractedValue::Array(vec![]).pg_type_name(), "ARRAY");
        assert_eq!(
            ExtractedValue::Array(vec![ExtractedValue::Int(1)]).pg_type_name(),
            "ARRAY"
        );

        // Decimal
        assert_eq!(ExtractedValue::Decimal("0".to_string()).pg_type_name(), "NUMERIC");
        assert_eq!(ExtractedValue::Decimal("123.45".to_string()).pg_type_name(), "NUMERIC");
    }

    #[test]
    fn test_bytes_type_conversion() {
        use serde_json::json;

        // Test empty bytes
        let empty_bytes = ExtractedValue::Bytes(vec![]);
        assert_eq!(empty_bytes.pg_type_name(), "BYTEA");
        assert!(empty_bytes.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&empty_bytes).unwrap(), json!(""));

        // Test bytes with data
        let bytes = ExtractedValue::Bytes(vec![0xff, 0x00, 0xab]);
        assert!(bytes.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&bytes).unwrap(), json!("ff00ab"));

        // Test bytes with all zero
        let zero_bytes = ExtractedValue::Bytes(vec![0x00, 0x00, 0x00]);
        assert_eq!(extracted_to_json(&zero_bytes).unwrap(), json!("000000"));
    }

    #[test]
    fn test_decimal_type_conversion() {
        use serde_json::json;

        // Test basic decimal
        let decimal = ExtractedValue::Decimal("123.45".to_string());
        assert_eq!(decimal.pg_type_name(), "NUMERIC");
        assert!(decimal.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&decimal).unwrap(), json!("123.45"));

        // Test large decimal
        let large_decimal = ExtractedValue::Decimal("999999999999999.999999".to_string());
        assert!(large_decimal.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&large_decimal).unwrap(), json!("999999999999999.999999"));

        // Test negative decimal
        let negative_decimal = ExtractedValue::Decimal("-123.45".to_string());
        assert_eq!(extracted_to_json(&negative_decimal).unwrap(), json!("-123.45"));

        // Test zero decimal
        let zero_decimal = ExtractedValue::Decimal("0.00".to_string());
        assert_eq!(extracted_to_json(&zero_decimal).unwrap(), json!("0.00"));
    }

    #[test]
    fn test_string_type_variants() {
        use serde_json::json;

        // Test regular string
        let regular = ExtractedValue::String("hello world".to_string());
        assert_eq!(regular.pg_type_name(), "TEXT");
        assert!(regular.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&regular).unwrap(), json!("hello world"));

        // Test empty string
        let empty = ExtractedValue::String(String::new());
        assert_eq!(empty.pg_type_name(), "TEXT");
        assert!(empty.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&empty).unwrap(), json!(""));

        // Test string with unicode
        let unicode = ExtractedValue::String("Hello 世界 🌍".to_string());
        assert!(unicode.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&unicode).unwrap(), json!("Hello 世界 🌍"));

        // Test string with escape characters
        let escaped = ExtractedValue::String("line1\nline2\ttab\r\nCRLF".to_string());
        assert!(escaped.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test string with quotes
        let quotes = ExtractedValue::String("He said \"Hello\" and 'Goodbye'".to_string());
        assert!(quotes.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test very long string
        let long_string = ExtractedValue::String("a".repeat(10000));
        assert!(long_string.bind_to_arguments(&mut PgArguments::default()).is_ok());
    }

    #[test]
    fn test_numeric_boundaries() {
        use serde_json::json;

        // Test SmallInt (i16) boundaries
        let small_max = ExtractedValue::SmallInt(i16::MAX);
        assert_eq!(small_max.pg_type_name(), "SMALLINT");
        assert!(small_max.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&small_max).unwrap(), json!(32767));

        let small_min = ExtractedValue::SmallInt(i16::MIN);
        assert!(small_min.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&small_min).unwrap(), json!(-32768));

        let small_zero = ExtractedValue::SmallInt(0);
        assert!(small_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&small_zero).unwrap(), json!(0));

        // Test Int (i32) boundaries
        let int_max = ExtractedValue::Int(i32::MAX);
        assert_eq!(int_max.pg_type_name(), "INTEGER");
        assert!(int_max.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&int_max).unwrap(), json!(2147483647));

        let int_min = ExtractedValue::Int(i32::MIN);
        assert!(int_min.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&int_min).unwrap(), json!(-2147483648));

        let int_zero = ExtractedValue::Int(0);
        assert!(int_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&int_zero).unwrap(), json!(0));

        // Test BigInt (i64) boundaries
        let big_max = ExtractedValue::BigInt(i64::MAX);
        assert_eq!(big_max.pg_type_name(), "BIGINT");
        assert!(big_max.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&big_max).unwrap(), json!(9223372036854775807i64));

        let big_min = ExtractedValue::BigInt(i64::MIN);
        assert!(big_min.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&big_min).unwrap(), json!(-9223372036854775808i64));

        let big_zero = ExtractedValue::BigInt(0);
        assert!(big_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&big_zero).unwrap(), json!(0));

        // Test edge values near boundaries
        let small_near_max = ExtractedValue::SmallInt(i16::MAX - 1);
        assert_eq!(extracted_to_json(&small_near_max).unwrap(), json!(32766));

        let int_near_min = ExtractedValue::Int(i32::MIN + 1);
        assert_eq!(extracted_to_json(&int_near_min).unwrap(), json!(-2147483647));
    }

    #[test]
    fn test_unsigned_numeric_types() {
        use serde_json::json;

        // PostgreSQL doesn't have native unsigned types, but we can test
        // that signed types handle positive values that would fit in unsigned ranges

        // Test values that would fit in u8 (0-255)
        let u8_max = ExtractedValue::SmallInt(255);
        assert!(u8_max.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&u8_max).unwrap(), json!(255));

        // Test values that would fit in u16 (0-65535)
        let u16_max = ExtractedValue::Int(65535);
        assert!(u16_max.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&u16_max).unwrap(), json!(65535));

        // Test values that would fit in u32 (0-4294967295)
        let u32_max = ExtractedValue::BigInt(4294967295);
        assert!(u32_max.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&u32_max).unwrap(), json!(4294967295i64));

        // Test zero (valid for all unsigned types)
        let zero = ExtractedValue::Int(0);
        assert!(zero.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&zero).unwrap(), json!(0));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_float_special_values() {
        use serde_json::json;

        // Test f32 special values
        let f32_zero = ExtractedValue::Float(0.0_f32);
        assert_eq!(f32_zero.pg_type_name(), "REAL");
        assert!(f32_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let f32_neg_zero = ExtractedValue::Float(-0.0_f32);
        assert!(f32_neg_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let f32_infinity = ExtractedValue::Float(f32::INFINITY);
        assert!(f32_infinity.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&f32_infinity).unwrap(), json!(null));

        let f32_neg_infinity = ExtractedValue::Float(f32::NEG_INFINITY);
        assert!(f32_neg_infinity.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&f32_neg_infinity).unwrap(), json!(null));

        let f32_nan = ExtractedValue::Float(f32::NAN);
        assert!(f32_nan.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&f32_nan).unwrap(), json!(null));

        let f32_max = ExtractedValue::Float(f32::MAX);
        assert!(f32_max.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let f32_min = ExtractedValue::Float(f32::MIN);
        assert!(f32_min.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test f64 special values
        let f64_zero = ExtractedValue::Double(0.0_f64);
        assert_eq!(f64_zero.pg_type_name(), "DOUBLE PRECISION");
        assert!(f64_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let f64_neg_zero = ExtractedValue::Double(-0.0_f64);
        assert!(f64_neg_zero.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let f64_infinity = ExtractedValue::Double(f64::INFINITY);
        assert!(f64_infinity.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&f64_infinity).unwrap(), json!(null));

        let f64_neg_infinity = ExtractedValue::Double(f64::NEG_INFINITY);
        assert!(f64_neg_infinity.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&f64_neg_infinity).unwrap(), json!(null));

        let f64_nan = ExtractedValue::Double(f64::NAN);
        assert!(f64_nan.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&f64_nan).unwrap(), json!(null));

        let f64_max = ExtractedValue::Double(f64::MAX);
        assert!(f64_max.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let f64_min = ExtractedValue::Double(f64::MIN);
        assert!(f64_min.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test normal float values
        let normal_float = ExtractedValue::Float(3.14_f32);
        assert!(normal_float.bind_to_arguments(&mut PgArguments::default()).is_ok());

        let normal_double = ExtractedValue::Double(2.71828182845904523536_f64);
        assert!(normal_double.bind_to_arguments(&mut PgArguments::default()).is_ok());
    }

    #[test]
    fn test_array_of_primitives() {
        use serde_json::json;

        // Test array of integers
        let int_array = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Int(2),
            ExtractedValue::Int(3),
            ExtractedValue::Int(4),
            ExtractedValue::Int(5),
        ]);
        assert_eq!(int_array.pg_type_name(), "ARRAY");
        assert!(int_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&int_array).unwrap(), json!([1, 2, 3, 4, 5]));

        // Test array of strings
        let string_array = ExtractedValue::Array(vec![
            ExtractedValue::String("foo".to_string()),
            ExtractedValue::String("bar".to_string()),
            ExtractedValue::String("baz".to_string()),
        ]);
        assert!(string_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&string_array).unwrap(), json!(["foo", "bar", "baz"]));

        // Test array of bools
        let bool_array = ExtractedValue::Array(vec![
            ExtractedValue::Bool(true),
            ExtractedValue::Bool(false),
            ExtractedValue::Bool(true),
        ]);
        assert!(bool_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&bool_array).unwrap(), json!([true, false, true]));

        // Test array of floats
        let float_array = ExtractedValue::Array(vec![
            ExtractedValue::Float(1.1),
            ExtractedValue::Float(2.2),
            ExtractedValue::Float(3.3),
        ]);
        assert!(float_array.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Test array of BigInts
        let bigint_array = ExtractedValue::Array(vec![
            ExtractedValue::BigInt(1000000),
            ExtractedValue::BigInt(2000000),
            ExtractedValue::BigInt(3000000),
        ]);
        assert!(bigint_array.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&bigint_array).unwrap(), json!([1000000, 2000000, 3000000]));

        // Test single element array
        let single = ExtractedValue::Array(vec![ExtractedValue::Int(42)]);
        assert!(single.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&single).unwrap(), json!([42]));
    }

    #[test]
    fn test_option_some_values() {
        use serde_json::json;

        // ExtractedValue represents Option through the Null variant
        // Test that non-Null values represent Some(T)

        // Some(bool)
        let some_bool = ExtractedValue::Bool(true);
        assert_eq!(some_bool.pg_type_name(), "BOOLEAN");
        assert!(some_bool.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&some_bool).unwrap(), json!(true));

        // Some(int)
        let some_int = ExtractedValue::Int(42);
        assert_eq!(some_int.pg_type_name(), "INTEGER");
        assert!(some_int.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&some_int).unwrap(), json!(42));

        // Some(string)
        let some_string = ExtractedValue::String("value".to_string());
        assert_eq!(some_string.pg_type_name(), "TEXT");
        assert!(some_string.bind_to_arguments(&mut PgArguments::default()).is_ok());
        assert_eq!(extracted_to_json(&some_string).unwrap(), json!("value"));

        // Some(float)
        let some_float = ExtractedValue::Double(3.14);
        assert_eq!(some_float.pg_type_name(), "DOUBLE PRECISION");
        assert!(some_float.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Some(uuid)
        let some_uuid = ExtractedValue::Uuid(Uuid::nil());
        assert_eq!(some_uuid.pg_type_name(), "UUID");
        assert!(some_uuid.bind_to_arguments(&mut PgArguments::default()).is_ok());

        // Some(bytes)
        let some_bytes = ExtractedValue::Bytes(vec![1, 2, 3]);
        assert_eq!(some_bytes.pg_type_name(), "BYTEA");
        assert!(some_bytes.bind_to_arguments(&mut PgArguments::default()).is_ok());
    }

    #[test]
    fn test_option_none_values() {
        use serde_json::json;

        // ExtractedValue::Null represents Option::None
        let none_value = ExtractedValue::Null;

        // Verify type name
        assert_eq!(none_value.pg_type_name(), "NULL");

        // Verify binding
        let mut args = PgArguments::default();
        assert!(none_value.bind_to_arguments(&mut args).is_ok());

        // Verify JSON conversion
        assert_eq!(extracted_to_json(&none_value).unwrap(), json!(null));

        // Test array with None values
        let array_with_nones = ExtractedValue::Array(vec![
            ExtractedValue::Int(1),
            ExtractedValue::Null,
            ExtractedValue::Int(3),
            ExtractedValue::Null,
        ]);
        assert_eq!(
            extracted_to_json(&array_with_nones).unwrap(),
            json!([1, null, 3, null])
        );

        // Test multiple Null values
        for _ in 0..5 {
            let mut args = PgArguments::default();
            assert!(ExtractedValue::Null.bind_to_arguments(&mut args).is_ok());
        }
    }

    #[test]
    fn test_extracted_value_all_variants() {
        // Ensure all ExtractedValue variants can be constructed and used

        let null = ExtractedValue::Null;
        assert!(matches!(null, ExtractedValue::Null));

        let bool_val = ExtractedValue::Bool(true);
        assert!(matches!(bool_val, ExtractedValue::Bool(_)));

        let small_int = ExtractedValue::SmallInt(42);
        assert!(matches!(small_int, ExtractedValue::SmallInt(_)));

        let int = ExtractedValue::Int(42);
        assert!(matches!(int, ExtractedValue::Int(_)));

        let big_int = ExtractedValue::BigInt(42);
        assert!(matches!(big_int, ExtractedValue::BigInt(_)));

        let float = ExtractedValue::Float(3.14);
        assert!(matches!(float, ExtractedValue::Float(_)));

        let double = ExtractedValue::Double(3.14);
        assert!(matches!(double, ExtractedValue::Double(_)));

        let string = ExtractedValue::String("test".to_string());
        assert!(matches!(string, ExtractedValue::String(_)));

        let bytes = ExtractedValue::Bytes(vec![1, 2, 3]);
        assert!(matches!(bytes, ExtractedValue::Bytes(_)));

        let uuid = ExtractedValue::Uuid(Uuid::nil());
        assert!(matches!(uuid, ExtractedValue::Uuid(_)));

        let date = ExtractedValue::Date(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert!(matches!(date, ExtractedValue::Date(_)));

        let time = ExtractedValue::Time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        assert!(matches!(time, ExtractedValue::Time(_)));

        let timestamp = ExtractedValue::Timestamp(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        ));
        assert!(matches!(timestamp, ExtractedValue::Timestamp(_)));

        let timestamp_tz = ExtractedValue::TimestampTz(DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            ),
            Utc,
        ));
        assert!(matches!(timestamp_tz, ExtractedValue::TimestampTz(_)));

        let json = ExtractedValue::Json(serde_json::json!({"key": "value"}));
        assert!(matches!(json, ExtractedValue::Json(_)));

        let array = ExtractedValue::Array(vec![ExtractedValue::Int(1)]);
        assert!(matches!(array, ExtractedValue::Array(_)));

        let decimal = ExtractedValue::Decimal("123.45".to_string());
        assert!(matches!(decimal, ExtractedValue::Decimal(_)));

        // Test that all variants can be bound
        let all_variants = vec![
            null, bool_val, small_int, int, big_int, float, double,
            string, bytes, uuid, date, time, timestamp, timestamp_tz,
            json, array, decimal,
        ];

        for variant in all_variants {
            let mut args = PgArguments::default();
            assert!(variant.bind_to_arguments(&mut args).is_ok(),
                   "Failed to bind variant: {}", variant.pg_type_name());
        }
    }

    #[test]
    fn test_type_display_formatting() {
        // Test that pg_type_name returns properly formatted PostgreSQL type names

        // Verify NULL
        assert_eq!(ExtractedValue::Null.pg_type_name(), "NULL");

        // Verify exact type name matches for SQL compatibility
        assert_eq!(ExtractedValue::Bool(true).pg_type_name(), "BOOLEAN");
        assert_eq!(ExtractedValue::SmallInt(1).pg_type_name(), "SMALLINT");
        assert_eq!(ExtractedValue::Int(1).pg_type_name(), "INTEGER");
        assert_eq!(ExtractedValue::BigInt(1).pg_type_name(), "BIGINT");
        assert_eq!(ExtractedValue::Float(1.0).pg_type_name(), "REAL");
        assert_eq!(ExtractedValue::Double(1.0).pg_type_name(), "DOUBLE PRECISION");
        assert_eq!(ExtractedValue::String("test".to_string()).pg_type_name(), "TEXT");
        assert_eq!(ExtractedValue::Bytes(vec![]).pg_type_name(), "BYTEA");
        assert_eq!(ExtractedValue::Uuid(Uuid::nil()).pg_type_name(), "UUID");
        assert_eq!(ExtractedValue::Date(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).pg_type_name(), "DATE");
        assert_eq!(ExtractedValue::Time(NaiveTime::from_hms_opt(12, 0, 0).unwrap()).pg_type_name(), "TIME");
        assert_eq!(
            ExtractedValue::Timestamp(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            )).pg_type_name(),
            "TIMESTAMP"
        );
        assert_eq!(
            ExtractedValue::TimestampTz(DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                ),
                Utc,
            )).pg_type_name(),
            "TIMESTAMPTZ"
        );
        assert_eq!(ExtractedValue::Json(serde_json::json!(null)).pg_type_name(), "JSONB");
        assert_eq!(ExtractedValue::Array(vec![]).pg_type_name(), "ARRAY");
        assert_eq!(ExtractedValue::Decimal("1.0".to_string()).pg_type_name(), "NUMERIC");

        // Verify all type names are uppercase
        let all_types = vec![
            ExtractedValue::Null,
            ExtractedValue::Bool(false),
            ExtractedValue::SmallInt(0),
            ExtractedValue::Int(0),
            ExtractedValue::BigInt(0),
            ExtractedValue::Float(0.0),
            ExtractedValue::Double(0.0),
            ExtractedValue::String(String::new()),
            ExtractedValue::Bytes(vec![]),
            ExtractedValue::Uuid(Uuid::nil()),
            ExtractedValue::Date(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ExtractedValue::Time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            ExtractedValue::Timestamp(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )),
            ExtractedValue::TimestampTz(DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                ),
                Utc,
            )),
            ExtractedValue::Json(serde_json::json!(null)),
            ExtractedValue::Array(vec![]),
            ExtractedValue::Decimal("0".to_string()),
        ];

        for extracted_value in all_types {
            let type_name = extracted_value.pg_type_name();
            // All characters should be uppercase letters or spaces
            for ch in type_name.chars() {
                assert!(ch.is_ascii_uppercase() || ch.is_whitespace(),
                       "Type name '{}' contains non-uppercase character: '{}'", type_name, ch);
            }
            // Type name should not be empty
            assert!(!type_name.is_empty(), "Type name should not be empty");
            // Type name should not have leading/trailing whitespace
            assert_eq!(type_name, type_name.trim(), "Type name should not have leading/trailing whitespace");
        }
    }
}
