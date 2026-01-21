//! Field definition parsing for `ob api` CLI
//!
//! Supports syntax: `name:type=default?`
//! Examples:
//!   - `title:str` - required string
//!   - `completed:bool=false` - bool with default
//!   - `priority:int?` - optional int
//!   - `due_date:datetime?` - optional datetime

use anyhow::{bail, Result};

use super::config::DbType;

/// Supported field types
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Str,
    Int,
    Float,
    Bool,
    Datetime,
    Date,
    Uuid,
    Dict,
    List,
}

impl FieldType {
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "str" | "string" => Ok(Self::Str),
            "int" | "integer" => Ok(Self::Int),
            "float" | "double" => Ok(Self::Float),
            "bool" | "boolean" => Ok(Self::Bool),
            "datetime" => Ok(Self::Datetime),
            "date" => Ok(Self::Date),
            "uuid" => Ok(Self::Uuid),
            "dict" | "json" | "jsonb" => Ok(Self::Dict),
            "list" | "array" => Ok(Self::List),
            _ => bail!("Unknown type: '{}'. Supported: str, int, float, bool, datetime, date, uuid, dict, list", s),
        }
    }

    /// Get Python type annotation
    pub fn python_type(&self) -> &'static str {
        match self {
            Self::Str => "str",
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Datetime => "datetime",
            Self::Date => "date",
            Self::Uuid => "UUID",
            Self::Dict => "dict",
            Self::List => "list",
        }
    }

    /// Get PostgreSQL column type
    pub fn pg_column_type(&self) -> &'static str {
        match self {
            Self::Str => "VARCHAR(255)",
            Self::Int => "BIGINT",
            Self::Float => "DOUBLE PRECISION",
            Self::Bool => "BOOLEAN",
            Self::Datetime => "TIMESTAMPTZ",
            Self::Date => "DATE",
            Self::Uuid => "UUID",
            Self::Dict => "JSONB",
            Self::List => "TEXT[]",
        }
    }

    /// Get MongoDB field type hint
    pub fn mongo_type_hint(&self) -> &'static str {
        match self {
            Self::Str => "string",
            Self::Int => "int64",
            Self::Float => "double",
            Self::Bool => "bool",
            Self::Datetime => "date",
            Self::Date => "date",
            Self::Uuid => "string",
            Self::Dict => "object",
            Self::List => "array",
        }
    }
}

/// Parsed field definition
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub default: Option<String>,
    pub is_optional: bool,
}

impl FieldDef {
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
        let type_annotation = if self.is_optional {
            format!("Optional[{}]", py_type)
        } else {
            py_type.to_string()
        };

        let mut field_args = Vec::new();

        // Add default value
        if let Some(ref default) = self.default {
            let default_str = self.format_default_value(default);
            field_args.push(format!("default={}", default_str));
        } else if self.is_optional {
            field_args.push("default=None".to_string());
        }

        // Add database-specific column type
        match db_type {
            DbType::Pg => {
                field_args.push(format!(
                    "column_type=\"{}\"",
                    self.field_type.pg_column_type()
                ));
            }
            DbType::Mongo => {
                // MongoDB doesn't need explicit column type
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
        let (type_annotation, default) = if for_update || self.is_optional {
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
    fn format_default_value(&self, value: &str) -> String {
        match self.field_type {
            FieldType::Str => format!("\"{}\"", value),
            FieldType::Bool => {
                if value.to_lowercase() == "true" || value == "1" {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            FieldType::Int | FieldType::Float => value.to_string(),
            FieldType::Dict => "{}".to_string(),
            FieldType::List => "[]".to_string(),
            _ => format!("\"{}\"", value),
        }
    }

    /// Generate sample test value
    pub fn sample_value(&self) -> String {
        match self.field_type {
            FieldType::Str => format!("\"test_{}\"", self.name),
            FieldType::Int => "42".to_string(),
            FieldType::Float => "3.14".to_string(),
            FieldType::Bool => "True".to_string(),
            FieldType::Datetime => "datetime.utcnow()".to_string(),
            FieldType::Date => "date.today()".to_string(),
            FieldType::Uuid => "uuid4()".to_string(),
            FieldType::Dict => "{}".to_string(),
            FieldType::List => "[]".to_string(),
        }
    }
}

/// Parse fields from CLI argument string
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

/// Parse a single field definition
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

    Ok(FieldDef {
        name,
        field_type,
        default,
        is_optional,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_field() {
        let fields = parse_fields("title:str").unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "title");
        assert_eq!(fields[0].field_type, FieldType::Str);
        assert!(!fields[0].is_optional);
        assert!(fields[0].default.is_none());
    }

    #[test]
    fn test_parse_field_with_default() {
        let fields = parse_fields("completed:bool=false").unwrap();
        assert_eq!(fields[0].name, "completed");
        assert_eq!(fields[0].field_type, FieldType::Bool);
        assert_eq!(fields[0].default, Some("false".to_string()));
    }

    #[test]
    fn test_parse_optional_field() {
        let fields = parse_fields("priority:int?").unwrap();
        assert_eq!(fields[0].name, "priority");
        assert!(fields[0].is_optional);
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
