//! Field definition parsing for `ob api` CLI
//!
//! Supports two syntaxes:
//!
//! 1. Simple syntax: `name:type=default?`
//!    Examples:
//!      - `title:str` - required string
//!      - `completed:bool=false` - bool with default
//!      - `priority:int?` - optional int
//!
//! 2. JSON syntax: `[{"name": "title", "type": "str", "index": true}]`
//!    Supports full PostgreSQL features: indexes, unique, foreign keys, etc.

use anyhow::{bail, Result};
use serde::Deserialize;

use super::config::DbType;

/// Supported field types
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Str,
    #[serde(alias = "string")]
    String,
    Int,
    #[serde(alias = "integer")]
    Integer,
    Float,
    #[serde(alias = "double")]
    Double,
    Decimal,
    Bool,
    #[serde(alias = "boolean")]
    Boolean,
    Datetime,
    Date,
    Uuid,
    #[serde(alias = "dict", alias = "jsonb")]
    Json,
    #[serde(alias = "list")]
    Array,
}

impl FieldType {
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "str" | "string" => Ok(Self::Str),
            "int" | "integer" => Ok(Self::Int),
            "float" | "double" => Ok(Self::Float),
            "decimal" => Ok(Self::Decimal),
            "bool" | "boolean" => Ok(Self::Bool),
            "datetime" => Ok(Self::Datetime),
            "date" => Ok(Self::Date),
            "uuid" => Ok(Self::Uuid),
            "dict" | "json" | "jsonb" => Ok(Self::Json),
            "list" | "array" => Ok(Self::Array),
            _ => bail!("Unknown type: '{}'. Supported: str, int, float, decimal, bool, datetime, date, uuid, json, array", s),
        }
    }

    /// Normalize aliases to canonical type
    fn normalize(&self) -> &Self {
        match self {
            Self::String => &Self::Str,
            Self::Integer => &Self::Int,
            Self::Double => &Self::Float,
            Self::Boolean => &Self::Bool,
            _ => self,
        }
    }

    /// Get Python type annotation
    pub fn python_type(&self) -> &'static str {
        match self.normalize() {
            Self::Str | Self::String => "str",
            Self::Int | Self::Integer => "int",
            Self::Float | Self::Double | Self::Decimal => "float",
            Self::Bool | Self::Boolean => "bool",
            Self::Datetime => "datetime",
            Self::Date => "date",
            Self::Uuid => "UUID",
            Self::Json => "dict",
            Self::Array => "list",
        }
    }

    /// Get PostgreSQL column type (default, without size/precision)
    pub fn pg_column_type(&self) -> &'static str {
        match self.normalize() {
            Self::Str | Self::String => "VARCHAR(255)",
            Self::Int | Self::Integer => "BIGINT",
            Self::Float | Self::Double => "DOUBLE PRECISION",
            Self::Decimal => "NUMERIC",
            Self::Bool | Self::Boolean => "BOOLEAN",
            Self::Datetime => "TIMESTAMPTZ",
            Self::Date => "DATE",
            Self::Uuid => "UUID",
            Self::Json => "JSONB",
            Self::Array => "TEXT[]",
        }
    }

    /// Get PostgreSQL column type with size/precision
    pub fn pg_column_type_sized(&self, max_length: Option<u32>, precision: Option<(u8, u8)>) -> String {
        match self.normalize() {
            Self::Str | Self::String => {
                if let Some(len) = max_length {
                    format!("VARCHAR({})", len)
                } else {
                    "VARCHAR(255)".to_string()
                }
            }
            Self::Decimal => {
                if let Some((total, scale)) = precision {
                    format!("NUMERIC({}, {})", total, scale)
                } else {
                    "NUMERIC(10, 2)".to_string()
                }
            }
            _ => self.pg_column_type().to_string(),
        }
    }
}

/// Parsed field definition with full PostgreSQL support
#[derive(Debug, Clone, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub optional: bool,

    // PostgreSQL specific
    #[serde(default)]
    pub max_length: Option<u32>,
    #[serde(default)]
    pub precision: Option<(u8, u8)>,
    #[serde(default)]
    pub index: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub pk: bool,
    #[serde(default)]
    pub fk: Option<String>,
    #[serde(default)]
    pub check: Option<String>,
}

impl Default for FieldDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            field_type: FieldType::Str,
            default: None,
            optional: false,
            max_length: None,
            precision: None,
            index: false,
            unique: false,
            pk: false,
            fk: None,
            check: None,
        }
    }
}

impl FieldDef {
    /// Create from simple syntax (legacy)
    pub fn from_simple(name: String, field_type: FieldType, default: Option<String>, is_optional: bool) -> Self {
        Self {
            name,
            field_type,
            default: default.map(serde_json::Value::String),
            optional: is_optional,
            ..Default::default()
        }
    }

    /// Check if this is an auto-generated field (id, created_at, updated_at)
    pub fn is_auto_field(&self) -> bool {
        matches!(
            self.name.as_str(),
            "id" | "created_at" | "updated_at" | "_id"
        )
    }

    /// Generate Python Field definition for Model
    pub fn to_model_field(&self, db_type: DbType) -> String {
        let py_type = self.field_type.python_type();
        let type_annotation = if self.optional {
            format!("{} | None", py_type)
        } else {
            py_type.to_string()
        };

        let mut field_args = Vec::new();

        // Add default value
        if let Some(ref default) = self.default {
            let default_str = self.format_default_value(default);
            field_args.push(format!("default={}", default_str));
        } else if self.optional {
            field_args.push("default=None".to_string());
        }

        // Add database-specific options
        match db_type {
            DbType::Pg => {
                // Column type with size/precision
                let col_type = self.field_type.pg_column_type_sized(self.max_length, self.precision);
                field_args.push(format!("column_type=\"{}\"", col_type));

                // Index
                if self.index {
                    field_args.push("index=True".to_string());
                }

                // Unique
                if self.unique {
                    field_args.push("unique=True".to_string());
                }

                // Primary key
                if self.pk {
                    field_args.push("primary_key=True".to_string());
                }

                // Foreign key
                if let Some(ref fk) = self.fk {
                    field_args.push(format!("foreign_key=\"{}\"", fk));
                }
            }
            DbType::Mongo => {
                // MongoDB index
                if self.index {
                    field_args.push("index=True".to_string());
                }
                if self.unique {
                    field_args.push("unique=True".to_string());
                }
            }
        }

        if field_args.is_empty() {
            format!("    {}: {}", self.name, type_annotation)
        } else {
            format!(
                "    {}: {} = Field({})",
                self.name,
                type_annotation,
                field_args.join(", ")
            )
        }
    }

    /// Generate Python Field definition for Schema
    pub fn to_schema_field(&self, for_update: bool) -> String {
        let py_type = self.field_type.python_type();

        // For update schemas, all fields are optional
        let (type_annotation, default) = if for_update || self.optional {
            (format!("{} | None", py_type), "None".to_string())
        } else if let Some(ref d) = self.default {
            (py_type.to_string(), self.format_default_value(d))
        } else {
            // Required field with no default
            return format!("    {}: {}", self.name, py_type);
        };

        format!("    {}: {} = {}", self.name, type_annotation, default)
    }

    /// Format default value for Python code
    fn format_default_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => {
                // Check if it's a bool string
                if matches!(self.field_type.normalize(), FieldType::Bool | FieldType::Boolean) {
                    if s.to_lowercase() == "true" || s == "1" {
                        return "True".to_string();
                    } else {
                        return "False".to_string();
                    }
                }
                format!("\"{}\"", s)
            }
            serde_json::Value::Bool(b) => {
                if *b { "True".to_string() } else { "False".to_string() }
            }
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Array(_) => "[]".to_string(),
            serde_json::Value::Object(_) => "{}".to_string(),
            serde_json::Value::Null => "None".to_string(),
        }
    }

    /// Generate sample test value
    pub fn sample_value(&self) -> String {
        match self.field_type.normalize() {
            FieldType::Str | FieldType::String => format!("\"test_{}\"", self.name),
            FieldType::Int | FieldType::Integer => "42".to_string(),
            FieldType::Float | FieldType::Double | FieldType::Decimal => "3.14".to_string(),
            FieldType::Bool | FieldType::Boolean => "True".to_string(),
            FieldType::Datetime => "datetime.utcnow()".to_string(),
            FieldType::Date => "date.today()".to_string(),
            FieldType::Uuid => "uuid4()".to_string(),
            FieldType::Json => "{}".to_string(),
            FieldType::Array => "[]".to_string(),
        }
    }
}

/// Parse fields from CLI argument string (simple syntax)
///
/// Format: "name:type=default?,name2:type2"
/// Examples:
///   - "title:str,completed:bool=false"
///   - "user_id:int,amount:float=0.0,status:str=pending?"
pub fn parse_fields(input: &str) -> Result<Vec<FieldDef>> {
    let mut fields = Vec::new();

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let field = parse_single_field(part)?;
        fields.push(field);
    }

    if fields.is_empty() {
        bail!("No fields specified. Use format: name:type=default?");
    }

    Ok(fields)
}

/// Parse fields from JSON syntax
///
/// Format: `[{"name": "title", "type": "str", "index": true}]`
pub fn parse_fields_json(input: &str) -> Result<Vec<FieldDef>> {
    let fields: Vec<FieldDef> = serde_json::from_str(input)
        .map_err(|e| anyhow::anyhow!("Invalid JSON fields: {}", e))?;

    if fields.is_empty() {
        bail!("No fields specified in JSON array");
    }

    Ok(fields)
}

/// Parse a single field definition (simple syntax)
fn parse_single_field(input: &str) -> Result<FieldDef> {
    // Check for optional marker at end
    let (input, is_optional) = if input.ends_with('?') {
        (&input[..input.len() - 1], true)
    } else {
        (input, false)
    };

    // Split by colon to get name and type+default
    let colon_pos = input
        .find(':')
        .ok_or_else(|| anyhow::anyhow!("Invalid field format '{}'. Expected: name:type", input))?;

    let name = input[..colon_pos].trim().to_string();
    let rest = &input[colon_pos + 1..];

    // Validate name
    if name.is_empty() {
        bail!("Field name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        bail!(
            "Field name '{}' must be snake_case (lowercase letters, digits, underscores)",
            name
        );
    }

    // Split type and default by '='
    let (type_str, default) = if let Some(eq_pos) = rest.find('=') {
        let t = rest[..eq_pos].trim();
        let d = rest[eq_pos + 1..].trim();
        (t, Some(d.to_string()))
    } else {
        (rest.trim(), None)
    };

    let field_type = FieldType::from_str(type_str)?;

    Ok(FieldDef::from_simple(name, field_type, default, is_optional))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_field() {
        let fields = parse_fields("title:str").unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "title");
        assert!(matches!(fields[0].field_type, FieldType::Str));
        assert!(!fields[0].optional);
        assert!(fields[0].default.is_none());
    }

    #[test]
    fn test_parse_field_with_default() {
        let fields = parse_fields("completed:bool=false").unwrap();
        assert_eq!(fields[0].name, "completed");
        assert!(matches!(fields[0].field_type, FieldType::Bool));
        assert!(fields[0].default.is_some());
    }

    #[test]
    fn test_parse_optional_field() {
        let fields = parse_fields("priority:int?").unwrap();
        assert_eq!(fields[0].name, "priority");
        assert!(fields[0].optional);
    }

    #[test]
    fn test_parse_multiple_fields() {
        let fields =
            parse_fields("title:str,completed:bool=false,priority:int?,due_date:datetime?")
                .unwrap();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].name, "title");
        assert_eq!(fields[1].name, "completed");
        assert_eq!(fields[2].name, "priority");
        assert_eq!(fields[3].name, "due_date");
    }

    #[test]
    fn test_parse_json_fields() {
        let json = r#"[
            {"name": "title", "type": "str", "max_length": 255, "index": true},
            {"name": "email", "type": "str", "unique": true},
            {"name": "user_id", "type": "int", "fk": "users.id"}
        ]"#;
        let fields = parse_fields_json(json).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "title");
        assert_eq!(fields[0].max_length, Some(255));
        assert!(fields[0].index);
        assert!(fields[1].unique);
        assert_eq!(fields[2].fk, Some("users.id".to_string()));
    }

    #[test]
    fn test_invalid_type() {
        let result = parse_fields("name:invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_colon() {
        let result = parse_fields("name");
        assert!(result.is_err());
    }
}
