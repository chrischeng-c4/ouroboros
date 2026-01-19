//! OpenAPI 3.1 Schema Generation
//!
//! Generates OpenAPI specifications from handler metadata.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::validation::{TypeDescriptor, StringFormat};
use crate::request::HttpMethod;

#[cfg(test)]
use crate::validation::{StringConstraints, NumericConstraints, FieldDescriptor, ListConstraints};
#[cfg(test)]
use crate::request::SerializableValue;

/// OpenAPI 3.1 Specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<Server>,
    pub paths: HashMap<String, PathItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
}

impl OpenApiSpec {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            openapi: "3.1.0".to_string(),
            info: Info {
                title: title.into(),
                version: version.into(),
                description: None,
                terms_of_service: None,
                contact: None,
                license: None,
            },
            servers: Vec::new(),
            paths: HashMap::new(),
            components: None,
            tags: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.info.description = Some(desc.into());
        self
    }

    pub fn server(mut self, url: impl Into<String>, description: Option<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description,
        });
        self
    }

    pub fn tag(mut self, name: impl Into<String>, description: Option<String>) -> Self {
        self.tags.push(Tag {
            name: name.into(),
            description,
        });
        self
    }

    /// Add an operation to the spec
    pub fn add_operation(
        &mut self,
        method: HttpMethod,
        path: &str,
        operation: Operation,
    ) {
        let path_item = self.paths.entry(path.to_string()).or_default();
        match method {
            HttpMethod::Get => path_item.get = Some(operation),
            HttpMethod::Post => path_item.post = Some(operation),
            HttpMethod::Put => path_item.put = Some(operation),
            HttpMethod::Patch => path_item.patch = Some(operation),
            HttpMethod::Delete => path_item.delete = Some(operation),
            HttpMethod::Head => path_item.head = Some(operation),
            HttpMethod::Options => path_item.options = Some(operation),
        }
    }

    /// Add a schema to components
    pub fn add_schema(&mut self, name: impl Into<String>, schema: Schema) {
        let components = self.components.get_or_insert_with(Components::default);
        components.schemas.insert(name.into(), schema);
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "termsOfService")]
    pub terms_of_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Operation>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none", rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    pub responses: HashMap<String, Response>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub security: Vec<HashMap<String, Vec<String>>>,
}

impl Operation {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        responses.insert("200".to_string(), Response {
            description: "Successful response".to_string(),
            content: None,
        });

        Self {
            operation_id: None,
            summary: None,
            description: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            request_body: None,
            responses,
            deprecated: None,
            security: Vec::new(),
        }
    }

    pub fn operation_id(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn parameter(mut self, param: Parameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn request_body(mut self, body: RequestBody) -> Self {
        self.request_body = Some(body);
        self
    }

    pub fn response(mut self, status: &str, response: Response) -> Self {
        self.responses.insert(status.to_string(), response);
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.deprecated = Some(true);
        self
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: ParameterLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Query,
    Header,
    Path,
    Cookie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: HashMap<String, MediaType>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Components {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub schemas: HashMap<String, Schema>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty", rename = "securitySchemes")]
    pub security_schemes: HashMap<String, SecurityScheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "bearerFormat")]
    pub bearer_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "in")]
    pub location: Option<String>,
}

/// JSON Schema (subset for OpenAPI)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    // String constraints
    #[serde(skip_serializing_if = "Option::is_none", rename = "minLength")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxLength")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    // Numeric constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "exclusiveMinimum")]
    pub exclusive_minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "exclusiveMaximum")]
    pub exclusive_maximum: Option<f64>,
    // Array constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "minItems")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxItems")]
    pub max_items: Option<usize>,
    // Object constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "additionalProperties")]
    pub additional: Option<Box<Schema>>,
    // Composition
    #[serde(skip_serializing_if = "Option::is_none", rename = "oneOf")]
    pub one_of: Option<Vec<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "anyOf")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "allOf")]
    pub all_of: Option<Vec<Schema>>,
    // Enum
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    // Nullable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    // Reference
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub reference: Option<String>,
}

impl Schema {
    pub fn new(schema_type: impl Into<String>) -> Self {
        Self {
            schema_type: Some(schema_type.into()),
            ..Default::default()
        }
    }

    pub fn string() -> Self {
        Self::new("string")
    }

    pub fn integer() -> Self {
        Self::new("integer")
    }

    pub fn number() -> Self {
        Self::new("number")
    }

    pub fn boolean() -> Self {
        Self::new("boolean")
    }

    pub fn array(items: Schema) -> Self {
        Self {
            schema_type: Some("array".to_string()),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    pub fn object() -> Self {
        Self::new("object")
    }

    pub fn reference(ref_path: impl Into<String>) -> Self {
        Self {
            reference: Some(ref_path.into()),
            ..Default::default()
        }
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = Some(true);
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Convert TypeDescriptor to JSON Schema
pub fn type_descriptor_to_schema(desc: &TypeDescriptor) -> Schema {
    match desc {
        TypeDescriptor::String(constraints) => {
            let mut schema = Schema::string();
            schema.min_length = constraints.min_length;
            schema.max_length = constraints.max_length;
            schema.pattern = constraints.pattern.clone();
            if let Some(ref format) = constraints.format {
                schema.format = Some(string_format_to_openapi(format));
            }
            schema
        }
        TypeDescriptor::Int64(constraints) => {
            let mut schema = Schema::integer();
            schema.minimum = constraints.minimum.map(|v| v as f64);
            schema.maximum = constraints.maximum.map(|v| v as f64);
            schema.exclusive_minimum = constraints.exclusive_minimum.map(|v| v as f64);
            schema.exclusive_maximum = constraints.exclusive_maximum.map(|v| v as f64);
            schema
        }
        TypeDescriptor::Float64(constraints) => {
            let mut schema = Schema::number();
            schema.minimum = constraints.minimum;
            schema.maximum = constraints.maximum;
            schema.exclusive_minimum = constraints.exclusive_minimum;
            schema.exclusive_maximum = constraints.exclusive_maximum;
            schema
        }
        TypeDescriptor::Bool => Schema::boolean(),
        TypeDescriptor::Null => Schema::default().nullable(),
        TypeDescriptor::Bytes => Schema::string().format("binary"),
        TypeDescriptor::List { items, constraints } => {
            let mut schema = Schema::array(type_descriptor_to_schema(items));
            schema.min_items = constraints.min_items;
            schema.max_items = constraints.max_items;
            schema
        }
        TypeDescriptor::Tuple { items } => {
            // OpenAPI 3.1 doesn't have native tuple support, use array with prefixItems
            // For simplicity, we'll just use array with first item schema
            if let Some(first) = items.first() {
                Schema::array(type_descriptor_to_schema(first))
            } else {
                Schema::array(Schema::default())
            }
        }
        TypeDescriptor::Set { items } => {
            // Sets are represented as arrays with uniqueItems in OpenAPI 3.0
            // In 3.1, we can just use array (uniqueness is a runtime constraint)
            Schema::array(type_descriptor_to_schema(items))
        }
        TypeDescriptor::Object { fields, additional } => {
            let mut schema = Schema::object();
            let mut properties = HashMap::new();
            let mut required_fields = Vec::new();

            for field in fields {
                let mut field_schema = type_descriptor_to_schema(&field.type_desc);
                if let Some(ref desc) = field.description {
                    field_schema.description = Some(desc.clone());
                }
                if let Some(ref default) = field.default {
                    field_schema.default = Some(default.clone().into());
                }
                properties.insert(field.name.clone(), field_schema);
                if field.required {
                    required_fields.push(field.name.clone());
                }
            }

            schema.properties = Some(properties);
            if !required_fields.is_empty() {
                schema.required = Some(required_fields);
            }
            if let Some(ref add_props) = additional {
                schema.additional = Some(Box::new(type_descriptor_to_schema(add_props)));
            }
            schema
        }
        TypeDescriptor::Optional(inner) => {
            type_descriptor_to_schema(inner).nullable()
        }
        TypeDescriptor::Union { variants, nullable } => {
            let schemas: Vec<Schema> = variants.iter()
                .map(type_descriptor_to_schema)
                .collect();
            Schema {
                any_of: Some(schemas),
                nullable: if *nullable { Some(true) } else { None },
                ..Default::default()
            }
        }
        TypeDescriptor::Uuid => Schema::string().format("uuid"),
        TypeDescriptor::Email => Schema::string().format("email"),
        TypeDescriptor::Url => Schema::string().format("uri"),
        TypeDescriptor::DateTime => Schema::string().format("date-time"),
        TypeDescriptor::Date => Schema::string().format("date"),
        TypeDescriptor::Time => Schema::string().format("time"),
        TypeDescriptor::Decimal(_) => Schema::string().format("decimal"),
        TypeDescriptor::Enum { values } => {
            let enum_values: Vec<serde_json::Value> = values.iter()
                .map(|v| v.clone().into())
                .collect();
            Schema {
                enum_values: Some(enum_values),
                ..Default::default()
            }
        }
        TypeDescriptor::Literal { values } => {
            let enum_values: Vec<serde_json::Value> = values.iter()
                .map(|v| v.clone().into())
                .collect();
            Schema {
                enum_values: Some(enum_values),
                ..Default::default()
            }
        }
        TypeDescriptor::Any => Schema::default(),
    }
}

/// Convert StringFormat to OpenAPI format string
fn string_format_to_openapi(format: &StringFormat) -> String {
    match format {
        StringFormat::Email => "email".to_string(),
        StringFormat::Url => "uri".to_string(),
        StringFormat::Uuid => "uuid".to_string(),
        StringFormat::DateTime => "date-time".to_string(),
        StringFormat::Date => "date".to_string(),
        StringFormat::Time => "time".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_creation() {
        let spec = OpenApiSpec::new("Test API", "1.0.0")
            .description("A test API")
            .server("https://api.example.com", Some("Production".to_string()));

        assert_eq!(spec.openapi, "3.1.0");
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.servers.len(), 1);
    }

    #[test]
    fn test_operation_builder() {
        let op = Operation::new()
            .operation_id("getUser")
            .summary("Get a user")
            .tag("users")
            .parameter(Parameter {
                name: "user_id".to_string(),
                location: ParameterLocation::Path,
                description: Some("User ID".to_string()),
                required: true,
                deprecated: None,
                schema: Some(Schema::string()),
                example: None,
            });

        assert_eq!(op.operation_id, Some("getUser".to_string()));
        assert_eq!(op.tags, vec!["users"]);
        assert_eq!(op.parameters.len(), 1);
    }

    #[test]
    fn test_schema_string_constraints() {
        let desc = TypeDescriptor::String(StringConstraints {
            min_length: Some(1),
            max_length: Some(100),
            pattern: Some(r"^[a-z]+$".to_string()),
            format: None,
        });

        let schema = type_descriptor_to_schema(&desc);

        assert_eq!(schema.schema_type, Some("string".to_string()));
        assert_eq!(schema.min_length, Some(1));
        assert_eq!(schema.max_length, Some(100));
        assert_eq!(schema.pattern, Some(r"^[a-z]+$".to_string()));
    }

    #[test]
    fn test_schema_numeric_constraints() {
        let desc = TypeDescriptor::Int64(NumericConstraints {
            minimum: Some(0),
            maximum: Some(100),
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        });

        let schema = type_descriptor_to_schema(&desc);

        assert_eq!(schema.schema_type, Some("integer".to_string()));
        assert_eq!(schema.minimum, Some(0.0));
        assert_eq!(schema.maximum, Some(100.0));
    }

    #[test]
    fn test_schema_object() {
        let desc = TypeDescriptor::Object {
            fields: vec![
                FieldDescriptor {
                    name: "name".to_string(),
                    type_desc: TypeDescriptor::String(StringConstraints::default()),
                    required: true,
                    default: None,
                    description: None,
                },
                FieldDescriptor {
                    name: "age".to_string(),
                    type_desc: TypeDescriptor::Int64(NumericConstraints::default()),
                    required: false,
                    default: None,
                    description: None,
                },
            ],
            additional: None,
        };

        let schema = type_descriptor_to_schema(&desc);

        assert_eq!(schema.schema_type, Some("object".to_string()));
        assert!(schema.properties.is_some());
        let props = schema.properties.as_ref().unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("age"));
        assert_eq!(schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn test_schema_array() {
        let desc = TypeDescriptor::List {
            items: Box::new(TypeDescriptor::String(StringConstraints::default())),
            constraints: ListConstraints {
                min_items: Some(1),
                max_items: Some(10),
                unique_items: false,
            },
        };

        let schema = type_descriptor_to_schema(&desc);

        assert_eq!(schema.schema_type, Some("array".to_string()));
        assert!(schema.items.is_some());
        assert_eq!(schema.min_items, Some(1));
        assert_eq!(schema.max_items, Some(10));
    }

    #[test]
    fn test_schema_optional() {
        let desc = TypeDescriptor::Optional(Box::new(TypeDescriptor::String(StringConstraints::default())));

        let schema = type_descriptor_to_schema(&desc);

        assert_eq!(schema.schema_type, Some("string".to_string()));
        assert_eq!(schema.nullable, Some(true));
    }

    #[test]
    fn test_schema_enum() {
        let desc = TypeDescriptor::Enum {
            values: vec![
                ouroboros_validation::Value::String("active".to_string()),
                ouroboros_validation::Value::String("inactive".to_string()),
            ],
        };

        let schema = type_descriptor_to_schema(&desc);

        assert!(schema.enum_values.is_some());
        let enums = schema.enum_values.as_ref().unwrap();
        assert_eq!(enums.len(), 2);
    }

    #[test]
    fn test_schema_special_formats() {
        let test_cases = vec![
            (TypeDescriptor::Uuid, "uuid"),
            (TypeDescriptor::Email, "email"),
            (TypeDescriptor::Url, "uri"),
            (TypeDescriptor::DateTime, "date-time"),
            (TypeDescriptor::Date, "date"),
            (TypeDescriptor::Time, "time"),
        ];

        for (desc, expected_format) in test_cases {
            let schema = type_descriptor_to_schema(&desc);
            assert_eq!(schema.format, Some(expected_format.to_string()));
        }
    }

    #[test]
    fn test_schema_to_json() {
        let mut spec = OpenApiSpec::new("Test", "1.0.0");
        spec.add_operation(
            HttpMethod::Get,
            "/users/{id}",
            Operation::new()
                .operation_id("getUser")
                .parameter(Parameter {
                    name: "id".to_string(),
                    location: ParameterLocation::Path,
                    description: None,
                    required: true,
                    deprecated: None,
                    schema: Some(Schema::string()),
                    example: None,
                }),
        );

        let json = spec.to_json().unwrap();
        assert!(json.contains("getUser"));
        assert!(json.contains("/users/{id}"));
    }

    #[test]
    fn test_schema_to_yaml() {
        let spec = OpenApiSpec::new("Test API", "1.0.0")
            .description("Test description");

        let yaml = spec.to_yaml().unwrap();
        assert!(yaml.contains("openapi: 3.1.0"));
        assert!(yaml.contains("title: Test API"));
    }

    #[test]
    fn test_nested_objects() {
        let desc = TypeDescriptor::Object {
            fields: vec![
                FieldDescriptor {
                    name: "user".to_string(),
                    type_desc: TypeDescriptor::Object {
                        fields: vec![
                            FieldDescriptor {
                                name: "name".to_string(),
                                type_desc: TypeDescriptor::String(StringConstraints::default()),
                                required: true,
                                default: None,
                                description: Some("User name".to_string()),
                            },
                        ],
                        additional: None,
                    },
                    required: true,
                    default: None,
                    description: Some("User object".to_string()),
                },
            ],
            additional: None,
        };

        let schema = type_descriptor_to_schema(&desc);

        assert!(schema.properties.is_some());
        let props = schema.properties.as_ref().unwrap();
        assert!(props.contains_key("user"));

        let user_schema = &props["user"];
        assert_eq!(user_schema.description, Some("User object".to_string()));
        assert!(user_schema.properties.is_some());
    }
}
