//! JSON Schema export for validation types
//!
//! Generates JSON Schema 2020-12 compatible schemas from TypeDescriptors
//! for OpenAPI 3.1 compatibility.

use std::collections::HashMap;
use crate::types::{TypeDescriptor, Value};
use crate::constraints::StringFormat;

// ============================================================================
// JSON Schema Types
// ============================================================================

/// JSON Schema representation
#[derive(Debug, Clone, Default)]
pub struct JsonSchema {
    /// Schema type
    pub schema_type: Option<String>,
    /// Schema format
    pub format: Option<String>,
    /// Title
    pub title: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Default value (as JSON string)
    pub default: Option<String>,
    /// Examples
    pub examples: Vec<String>,
    /// Enum values
    pub enum_values: Option<Vec<String>>,

    // String constraints
    /// Minimum string length
    pub min_length: Option<usize>,
    /// Maximum string length
    pub max_length: Option<usize>,
    /// Regex pattern
    pub pattern: Option<String>,

    // Numeric constraints
    /// Minimum value
    pub minimum: Option<f64>,
    /// Maximum value
    pub maximum: Option<f64>,
    /// Exclusive minimum
    pub exclusive_minimum: Option<f64>,
    /// Exclusive maximum
    pub exclusive_maximum: Option<f64>,
    /// Multiple of
    pub multiple_of: Option<f64>,

    // Array constraints
    /// Array items schema
    pub items: Option<Box<JsonSchema>>,
    /// Minimum items
    pub min_items: Option<usize>,
    /// Maximum items
    pub max_items: Option<usize>,
    /// Unique items
    pub unique_items: Option<bool>,

    // Object constraints
    /// Object properties
    pub properties: Option<HashMap<String, JsonSchema>>,
    /// Required field names
    pub required: Option<Vec<String>>,
    /// Additional properties schema
    pub additional_properties: Option<Box<JsonSchema>>,

    // Composition
    /// Any of (union types)
    pub any_of: Option<Vec<JsonSchema>>,
    /// All of (intersection types)
    pub all_of: Option<Vec<JsonSchema>>,
    /// One of (exclusive union)
    pub one_of: Option<Vec<JsonSchema>>,
    /// Not
    pub not: Option<Box<JsonSchema>>,

    /// Nullable
    pub nullable: bool,
}

impl JsonSchema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a schema for a specific type
    pub fn with_type(schema_type: impl Into<String>) -> Self {
        Self {
            schema_type: Some(schema_type.into()),
            ..Default::default()
        }
    }

    /// Add format
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Add title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set nullable
    pub fn nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref t) = self.schema_type {
            if self.nullable {
                parts.push(format!(r#""type": ["{}", "null"]"#, t));
            } else {
                parts.push(format!(r#""type": "{}""#, t));
            }
        }

        if let Some(ref f) = self.format {
            parts.push(format!(r#""format": "{}""#, f));
        }

        if let Some(ref t) = self.title {
            parts.push(format!(r#""title": "{}""#, t));
        }

        if let Some(ref d) = self.description {
            parts.push(format!(r#""description": "{}""#, d));
        }

        // String constraints
        if let Some(v) = self.min_length {
            parts.push(format!(r#""minLength": {}"#, v));
        }
        if let Some(v) = self.max_length {
            parts.push(format!(r#""maxLength": {}"#, v));
        }
        if let Some(ref p) = self.pattern {
            parts.push(format!(r#""pattern": "{}""#, p));
        }

        // Numeric constraints
        if let Some(v) = self.minimum {
            parts.push(format!(r#""minimum": {}"#, v));
        }
        if let Some(v) = self.maximum {
            parts.push(format!(r#""maximum": {}"#, v));
        }
        if let Some(v) = self.exclusive_minimum {
            parts.push(format!(r#""exclusiveMinimum": {}"#, v));
        }
        if let Some(v) = self.exclusive_maximum {
            parts.push(format!(r#""exclusiveMaximum": {}"#, v));
        }

        // Array constraints
        if let Some(v) = self.min_items {
            parts.push(format!(r#""minItems": {}"#, v));
        }
        if let Some(v) = self.max_items {
            parts.push(format!(r#""maxItems": {}"#, v));
        }
        if let Some(v) = self.unique_items {
            parts.push(format!(r#""uniqueItems": {}"#, v));
        }
        if let Some(ref items) = self.items {
            parts.push(format!(r#""items": {}"#, items.to_json()));
        }

        // Required fields
        if let Some(ref req) = self.required {
            let req_str: Vec<_> = req.iter().map(|s| format!(r#""{}""#, s)).collect();
            parts.push(format!(r#""required": [{}]"#, req_str.join(", ")));
        }

        format!("{{{}}}", parts.join(", "))
    }
}

// ============================================================================
// Conversion from TypeDescriptor
// ============================================================================

/// Convert a TypeDescriptor to JSON Schema
pub fn type_descriptor_to_json_schema(desc: &TypeDescriptor) -> JsonSchema {
    match desc {
        TypeDescriptor::String(constraints) => {
            let mut schema = JsonSchema::with_type("string");
            if let Some(min) = constraints.min_length {
                schema.min_length = Some(min);
            }
            if let Some(max) = constraints.max_length {
                schema.max_length = Some(max);
            }
            if let Some(ref pattern) = constraints.pattern {
                schema.pattern = Some(pattern.clone());
            }
            if let Some(format) = &constraints.format {
                schema.format = Some(string_format_to_schema(format));
            }
            schema
        }

        TypeDescriptor::Int64(constraints) => {
            let mut schema = JsonSchema::with_type("integer");
            if let Some(min) = constraints.minimum {
                schema.minimum = Some(min as f64);
            }
            if let Some(max) = constraints.maximum {
                schema.maximum = Some(max as f64);
            }
            if let Some(min) = constraints.exclusive_minimum {
                schema.exclusive_minimum = Some(min as f64);
            }
            if let Some(max) = constraints.exclusive_maximum {
                schema.exclusive_maximum = Some(max as f64);
            }
            schema
        }

        TypeDescriptor::Float64(constraints) => {
            let mut schema = JsonSchema::with_type("number");
            if let Some(min) = constraints.minimum {
                schema.minimum = Some(min);
            }
            if let Some(max) = constraints.maximum {
                schema.maximum = Some(max);
            }
            if let Some(min) = constraints.exclusive_minimum {
                schema.exclusive_minimum = Some(min);
            }
            if let Some(max) = constraints.exclusive_maximum {
                schema.exclusive_maximum = Some(max);
            }
            schema
        }

        TypeDescriptor::Bool => JsonSchema::with_type("boolean"),

        TypeDescriptor::List { items, constraints } => {
            let mut schema = JsonSchema::with_type("array");
            schema.items = Some(Box::new(type_descriptor_to_json_schema(items)));
            if let Some(min) = constraints.min_items {
                schema.min_items = Some(min);
            }
            if let Some(max) = constraints.max_items {
                schema.max_items = Some(max);
            }
            if constraints.unique_items {
                schema.unique_items = Some(true);
            }
            schema
        }

        TypeDescriptor::Object { fields, additional } => {
            let mut schema = JsonSchema::with_type("object");
            let mut properties = HashMap::new();
            let mut required = Vec::new();

            for field in fields {
                properties.insert(
                    field.serialization_name().to_string(),
                    type_descriptor_to_json_schema(&field.type_desc),
                );
                if field.required {
                    required.push(field.serialization_name().to_string());
                }
            }

            schema.properties = Some(properties);
            if !required.is_empty() {
                schema.required = Some(required);
            }

            if let Some(ref add) = additional {
                schema.additional_properties = Some(Box::new(type_descriptor_to_json_schema(add)));
            }

            schema
        }

        TypeDescriptor::Optional(inner) => {
            let mut schema = type_descriptor_to_json_schema(inner);
            schema.nullable = true;
            schema
        }

        TypeDescriptor::Union { variants, nullable } => {
            let mut schema = JsonSchema::new();
            schema.any_of = Some(
                variants
                    .iter()
                    .map(type_descriptor_to_json_schema)
                    .collect(),
            );
            schema.nullable = *nullable;
            schema
        }

        TypeDescriptor::Literal { values } => {
            let mut schema = JsonSchema::new();
            schema.enum_values = Some(values.iter().map(value_to_string).collect());
            schema
        }

        TypeDescriptor::Enum { values } => {
            let mut schema = JsonSchema::new();
            schema.enum_values = Some(values.iter().map(value_to_string).collect());
            schema
        }

        TypeDescriptor::Tuple { items: _ } => {
            let mut schema = JsonSchema::with_type("array");
            // For tuples, we could use prefixItems in JSON Schema
            schema.items = Some(Box::new(JsonSchema::new())); // Simplified
            schema
        }

        TypeDescriptor::Set { items } => {
            let mut schema = JsonSchema::with_type("array");
            schema.items = Some(Box::new(type_descriptor_to_json_schema(items)));
            schema.unique_items = Some(true);
            schema
        }

        TypeDescriptor::Null => {
            JsonSchema::with_type("null")
        }

        TypeDescriptor::Email => JsonSchema::with_type("string").format("email"),
        TypeDescriptor::Url => JsonSchema::with_type("string").format("uri"),
        TypeDescriptor::Uuid => JsonSchema::with_type("string").format("uuid"),
        TypeDescriptor::DateTime => JsonSchema::with_type("string").format("date-time"),
        TypeDescriptor::Date => JsonSchema::with_type("string").format("date"),
        TypeDescriptor::Time => JsonSchema::with_type("string").format("time"),
        TypeDescriptor::Bytes => JsonSchema::with_type("string").format("byte"),
        TypeDescriptor::Decimal(_) => JsonSchema::with_type("number"),
        TypeDescriptor::Any => JsonSchema::new(),

        // BSON types (feature-gated)
        #[cfg(feature = "bson")]
        TypeDescriptor::ObjectId => JsonSchema::with_type("string").format("objectid"),
        #[cfg(feature = "bson")]
        TypeDescriptor::BsonDateTime => JsonSchema::with_type("string").format("date-time"),
        #[cfg(feature = "bson")]
        TypeDescriptor::BsonDecimal128 => JsonSchema::with_type("string"),
        #[cfg(feature = "bson")]
        TypeDescriptor::BsonBinary => JsonSchema::with_type("string").format("byte"),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("\"{}\"", s),
        Value::Bytes(_) => "\"<bytes>\"".to_string(),
        Value::List(_) => "\"<list>\"".to_string(),
        Value::Object(_) => "\"<object>\"".to_string(),
    }
}

fn string_format_to_schema(format: &StringFormat) -> String {
    match format {
        StringFormat::Email => "email".to_string(),
        StringFormat::Url => "uri".to_string(),
        StringFormat::Uuid => "uuid".to_string(),
        StringFormat::DateTime => "date-time".to_string(),
        StringFormat::Date => "date".to_string(),
        StringFormat::Time => "time".to_string(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::{FieldDescriptor, StringConstraints, ListConstraints};

    #[test]
    fn test_string_schema() {
        let schema = type_descriptor_to_json_schema(&TypeDescriptor::String(Default::default()));
        assert_eq!(schema.schema_type, Some("string".to_string()));
    }

    #[test]
    fn test_string_with_constraints() {
        let desc = TypeDescriptor::String(StringConstraints {
            min_length: Some(1),
            max_length: Some(100),
            pattern: Some("^[a-z]+$".to_string()),
            format: None,
        });
        let schema = type_descriptor_to_json_schema(&desc);

        assert_eq!(schema.min_length, Some(1));
        assert_eq!(schema.max_length, Some(100));
        assert_eq!(schema.pattern, Some("^[a-z]+$".to_string()));
    }

    #[test]
    fn test_integer_schema() {
        let schema = type_descriptor_to_json_schema(&TypeDescriptor::Int64(Default::default()));
        assert_eq!(schema.schema_type, Some("integer".to_string()));
    }

    #[test]
    fn test_array_schema() {
        let desc = TypeDescriptor::List {
            items: Box::new(TypeDescriptor::String(Default::default())),
            constraints: ListConstraints {
                min_items: Some(1),
                max_items: Some(10),
                unique_items: true,
            },
        };
        let schema = type_descriptor_to_json_schema(&desc);

        assert_eq!(schema.schema_type, Some("array".to_string()));
        assert_eq!(schema.min_items, Some(1));
        assert_eq!(schema.max_items, Some(10));
        assert_eq!(schema.unique_items, Some(true));
        assert!(schema.items.is_some());
    }

    #[test]
    fn test_object_schema() {
        let desc = TypeDescriptor::Object {
            fields: vec![
                FieldDescriptor::new("name", TypeDescriptor::String(Default::default())),
                FieldDescriptor::new("age", TypeDescriptor::Int64(Default::default())).optional(),
            ],
            additional: None,
        };
        let schema = type_descriptor_to_json_schema(&desc);

        assert_eq!(schema.schema_type, Some("object".to_string()));
        let props = schema.properties.unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("age"));
        assert_eq!(schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn test_optional_schema() {
        let desc = TypeDescriptor::Optional(Box::new(TypeDescriptor::String(Default::default())));
        let schema = type_descriptor_to_json_schema(&desc);

        assert!(schema.nullable);
        assert_eq!(schema.schema_type, Some("string".to_string()));
    }

    #[test]
    fn test_email_format() {
        let schema = type_descriptor_to_json_schema(&TypeDescriptor::Email);
        assert_eq!(schema.format, Some("email".to_string()));
    }

    #[test]
    fn test_to_json() {
        let schema = JsonSchema::with_type("string")
            .format("email")
            .description("User email");

        let json = schema.to_json();
        assert!(json.contains(r#""type": "string""#));
        assert!(json.contains(r#""format": "email""#));
        assert!(json.contains(r#""description": "User email""#));
    }
}
