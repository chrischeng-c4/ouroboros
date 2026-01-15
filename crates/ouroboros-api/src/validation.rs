//! Request validation for HTTP APIs
//!
//! This module provides comprehensive validation for HTTP request parameters,
//! following the same security-focused approach as `crates/ouroboros/src/validation.rs`.
//!
//! # Architecture
//!
//! ## Validation Layers
//! 1. **Type Validation**: Ensures values match expected types (string, int, bool, etc.)
//! 2. **Constraint Validation**: Enforces limits (min/max length, numeric ranges, patterns)
//! 3. **Format Validation**: Validates special formats (email, URL, UUID, DateTime)
//! 4. **Security Validation**: Prevents injection attacks and malicious inputs
//!
//! ## Pre-compiled Validators
//! Request validators are built once from handler metadata and reused for all requests,
//! minimizing validation overhead.
//!
//! # Example
//!
//! ```rust
//! use ouroboros_api::validation::{RequestValidator, ParamValidator, ParamLocation, TypeDescriptor};
//! use ouroboros_api::request::SerializableValue;
//! use std::collections::HashMap;
//!
//! // Build validator once (from handler metadata)
//! let mut validator = RequestValidator::new();
//! validator.path_params.push(ParamValidator {
//!     name: "user_id".to_string(),
//!     location: ParamLocation::Path,
//!     type_desc: TypeDescriptor::String(Default::default()),
//!     required: true,
//!     default: None,
//! });
//!
//! // Validate many requests
//! let mut path_params = HashMap::new();
//! path_params.insert("user_id".to_string(), "user-123".to_string());
//!
//! let result = validator.validate(
//!     &path_params,
//!     &HashMap::new(),
//!     &HashMap::new(),
//!     None,
//! );
//! assert!(result.is_ok());
//! ```

use crate::error::{ValidationError, ValidationErrors};
use crate::request::SerializableValue;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

// ============================================================================
// Type Descriptors
// ============================================================================

/// Type descriptor for validation
#[derive(Debug, Clone)]
pub enum TypeDescriptor {
    // Basic types
    /// String type with constraints
    String(StringConstraints),
    /// Integer type with constraints (i64)
    Int(NumericConstraints<i64>),
    /// Float type with constraints (f64)
    Float(NumericConstraints<f64>),
    /// Boolean type
    Bool,
    /// Binary data
    Bytes,

    // Collection types
    /// List/Array type with constraints
    List {
        items: Box<TypeDescriptor>,
        min_items: Option<usize>,
        max_items: Option<usize>,
    },
    /// Tuple type (fixed-length ordered collection)
    Tuple {
        items: Vec<TypeDescriptor>,
    },
    /// Set type (unique items)
    Set {
        items: Box<TypeDescriptor>,
    },
    /// Object/Dict type with optional additional properties
    Object {
        fields: Vec<FieldDescriptor>,
        additional_properties: Option<Box<TypeDescriptor>>,
    },

    // Nullable/Union types
    /// Optional type (nullable)
    Optional(Box<TypeDescriptor>),
    /// Union type (multiple possible types)
    Union {
        variants: Vec<TypeDescriptor>,
        nullable: bool,
    },

    // Special types
    /// UUID type (validated format)
    Uuid,
    /// Email type (validated format)
    Email,
    /// URL type (validated format)
    Url,
    /// DateTime type (ISO 8601)
    DateTime,
    /// Date type (YYYY-MM-DD)
    Date,
    /// Time type (HH:MM:SS)
    Time,
    /// Decimal type (high precision)
    Decimal,

    // Enum types
    /// Enum type (value must match one of the allowed values)
    Enum {
        values: Vec<SerializableValue>,
    },
    /// Literal type (value must match one of the literal values)
    Literal {
        values: Vec<SerializableValue>,
    },

    /// Any type (no validation)
    Any,
}

impl TypeDescriptor {
    /// Get human-readable type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Bool => "boolean",
            Self::Bytes => "bytes",
            Self::List { .. } => "array",
            Self::Tuple { .. } => "tuple",
            Self::Set { .. } => "set",
            Self::Object { .. } => "object",
            Self::Optional(_) => "optional",
            Self::Union { .. } => "union",
            Self::Any => "any",
            Self::Uuid => "uuid",
            Self::Email => "email",
            Self::Url => "url",
            Self::DateTime => "datetime",
            Self::Date => "date",
            Self::Time => "time",
            Self::Decimal => "decimal",
            Self::Enum { .. } => "enum",
            Self::Literal { .. } => "literal",
        }
    }
}

/// String constraints
#[derive(Debug, Clone, Default)]
pub struct StringConstraints {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>, // Regex pattern
    pub format: Option<StringFormat>,
}

/// String format validators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringFormat {
    Email,
    Url,
    Uuid,
    DateTime,
    Date,
    Time,
}

/// Numeric constraints (generic over i64 and f64)
#[derive(Debug, Clone, Default)]
pub struct NumericConstraints<T> {
    pub minimum: Option<T>,
    pub maximum: Option<T>,
    pub exclusive_minimum: Option<T>,
    pub exclusive_maximum: Option<T>,
    pub multiple_of: Option<T>,
}

/// Field descriptor for object validation
#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    pub name: String,
    pub type_desc: TypeDescriptor,
    pub required: bool,
    pub default: Option<SerializableValue>,
    pub description: Option<String>,
}

// ============================================================================
// Parameter Validators
// ============================================================================

/// Parameter location in HTTP request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
    Body,
}

impl ParamLocation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
            Self::Header => "header",
            Self::Body => "body",
        }
    }
}

/// Parameter validator (pre-compiled from handler metadata)
#[derive(Debug, Clone)]
pub struct ParamValidator {
    pub name: String,
    pub location: ParamLocation,
    pub type_desc: TypeDescriptor,
    pub required: bool,
    pub default: Option<SerializableValue>,
}

/// Request validator (pre-compiled from handler metadata)
///
/// Built once from route handler metadata and reused for all requests to that endpoint.
#[derive(Debug, Clone)]
pub struct RequestValidator {
    pub path_params: Vec<ParamValidator>,
    pub query_params: Vec<ParamValidator>,
    pub header_params: Vec<ParamValidator>,
    pub body_validator: Option<TypeDescriptor>,
}

impl RequestValidator {
    /// Create a new empty request validator
    pub fn new() -> Self {
        Self {
            path_params: Vec::new(),
            query_params: Vec::new(),
            header_params: Vec::new(),
            body_validator: None,
        }
    }

    /// Validate a request and return validated data
    ///
    /// # Arguments
    /// * `path_params` - Path parameters extracted from route
    /// * `query_params` - Query string parameters
    /// * `headers` - HTTP headers (lowercase keys)
    /// * `body` - Request body (optional)
    ///
    /// # Returns
    /// `Ok(ValidatedRequest)` if all validations pass, `Err(ValidationErrors)` otherwise
    pub fn validate(
        &self,
        path_params: &HashMap<String, String>,
        query_params: &HashMap<String, SerializableValue>,
        headers: &HashMap<String, String>,
        body: Option<&SerializableValue>,
    ) -> Result<ValidatedRequest, ValidationErrors> {
        let mut errors = ValidationErrors::new();

        // Validate path params - start with all raw params, then apply validation
        let validated_path = self.validate_params_with_passthrough(
            &self.path_params,
            path_params,
            &mut errors,
        );

        // Validate query params - start with all raw params, then apply validation
        let validated_query = self.validate_params_with_value_passthrough(
            &self.query_params,
            query_params,
            &mut errors,
        );

        // Validate headers
        let validated_headers = self.validate_params(
            &self.header_params,
            |name| {
                headers
                    .get(&name.to_lowercase())
                    .map(|s| SerializableValue::String(s.clone()))
            },
            &mut errors,
        );

        // Validate body
        let validated_body = if let Some(body_type) = &self.body_validator {
            if let Some(body_value) = body {
                validate_type(body_value, body_type, "body", "", &mut errors);
                Some(body_value.clone())
            } else {
                errors.add(ValidationError {
                    location: "body".to_string(),
                    field: "".to_string(),
                    message: "Request body is required".to_string(),
                    error_type: "missing".to_string(),
                });
                None
            }
        } else {
            body.cloned()
        };

        if errors.is_empty() {
            Ok(ValidatedRequest {
                path_params: validated_path,
                query_params: validated_query,
                headers: validated_headers,
                body: validated_body,
            })
        } else {
            Err(errors)
        }
    }

    /// Validate path parameters with pass-through for router-extracted params
    ///
    /// This method ensures ALL path parameters from the router are available in the handler,
    /// with optional validation applied to those that have validators defined.
    fn validate_params_with_passthrough(
        &self,
        validators: &[ParamValidator],
        raw_params: &HashMap<String, String>,
        errors: &mut ValidationErrors,
    ) -> HashMap<String, SerializableValue> {
        // Start with all raw path params from the router
        let mut result: HashMap<String, SerializableValue> = raw_params
            .iter()
            .map(|(k, v)| (k.clone(), SerializableValue::String(v.clone())))
            .collect();

        // Apply validation to parameters that have validators
        for validator in validators {
            if let Some(raw_value) = raw_params.get(&validator.name) {
                let value = SerializableValue::String(raw_value.clone());

                // Validate the value
                validate_type(
                    &value,
                    &validator.type_desc,
                    validator.location.as_str(),
                    &validator.name,
                    errors,
                );

                // Overwrite with validated value (which may have been type-converted)
                result.insert(validator.name.clone(), value);
            } else if validator.required {
                // Required validator but no value in raw params
                if let Some(default) = &validator.default {
                    result.insert(validator.name.clone(), default.clone());
                } else {
                    errors.add(ValidationError {
                        location: validator.location.as_str().to_string(),
                        field: validator.name.clone(),
                        message: "Required path parameter is missing".to_string(),
                        error_type: "missing".to_string(),
                    });
                }
            } else if let Some(default) = &validator.default {
                // Optional with default
                result.insert(validator.name.clone(), default.clone());
            }
        }

        result
    }

    /// Validate query/header parameters with pass-through for all extracted params
    ///
    /// This method ensures ALL query/header parameters from the request are available in the handler,
    /// with optional validation applied to those that have validators defined.
    fn validate_params_with_value_passthrough(
        &self,
        validators: &[ParamValidator],
        raw_params: &HashMap<String, SerializableValue>,
        errors: &mut ValidationErrors,
    ) -> HashMap<String, SerializableValue> {
        // Start with all raw params from the request
        let mut result: HashMap<String, SerializableValue> = raw_params.clone();

        // Apply validation to parameters that have validators
        for validator in validators {
            if let Some(value) = raw_params.get(&validator.name) {
                // Validate the value
                validate_type(
                    value,
                    &validator.type_desc,
                    validator.location.as_str(),
                    &validator.name,
                    errors,
                );

                // Overwrite with validated value (which may have been type-converted)
                result.insert(validator.name.clone(), value.clone());
            } else if validator.required {
                // Required validator but no value in raw params
                if let Some(default) = &validator.default {
                    result.insert(validator.name.clone(), default.clone());
                } else {
                    errors.add(ValidationError {
                        location: validator.location.as_str().to_string(),
                        field: validator.name.clone(),
                        message: format!("Required {} parameter is missing", validator.location.as_str()),
                        error_type: "missing".to_string(),
                    });
                }
            } else if let Some(default) = &validator.default {
                // Optional with default
                result.insert(validator.name.clone(), default.clone());
            }
        }

        result
    }

    /// Validate a set of parameters
    fn validate_params<F>(
        &self,
        validators: &[ParamValidator],
        get_value: F,
        errors: &mut ValidationErrors,
    ) -> HashMap<String, SerializableValue>
    where
        F: Fn(&str) -> Option<SerializableValue>,
    {
        let mut result = HashMap::new();

        for validator in validators {
            let value = get_value(&validator.name);

            match value {
                Some(v) => {
                    validate_type(
                        &v,
                        &validator.type_desc,
                        validator.location.as_str(),
                        &validator.name,
                        errors,
                    );
                    result.insert(validator.name.clone(), v);
                }
                None if validator.required => {
                    if let Some(default) = &validator.default {
                        result.insert(validator.name.clone(), default.clone());
                    } else {
                        errors.add(ValidationError {
                            location: validator.location.as_str().to_string(),
                            field: validator.name.clone(),
                            message: format!("Field '{}' is required", validator.name),
                            error_type: "missing".to_string(),
                        });
                    }
                }
                None => {
                    if let Some(default) = &validator.default {
                        result.insert(validator.name.clone(), default.clone());
                    }
                }
            }
        }

        result
    }
}

impl Default for RequestValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validated request data
#[derive(Debug, Clone)]
pub struct ValidatedRequest {
    pub path_params: HashMap<String, SerializableValue>,
    pub query_params: HashMap<String, SerializableValue>,
    pub headers: HashMap<String, SerializableValue>,
    pub body: Option<SerializableValue>,
}

// ============================================================================
// Type Validation
// ============================================================================

/// Validate a value against a type descriptor
///
/// # Arguments
/// * `value` - Value to validate
/// * `type_desc` - Expected type
/// * `location` - Parameter location (for error messages)
/// * `field` - Field name (for error messages)
/// * `errors` - Error accumulator
pub fn validate_type(
    value: &SerializableValue,
    type_desc: &TypeDescriptor,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match type_desc {
        TypeDescriptor::String(constraints) => {
            validate_string(value, constraints, location, path, errors);
        }
        TypeDescriptor::Int(constraints) => {
            validate_int(value, constraints, location, path, errors);
        }
        TypeDescriptor::Float(constraints) => {
            validate_float(value, constraints, location, path, errors);
        }
        TypeDescriptor::Bool => {
            validate_bool(value, location, path, errors);
        }
        TypeDescriptor::Bytes => {
            validate_bytes(value, location, path, errors);
        }
        TypeDescriptor::List { items, min_items, max_items } => {
            validate_list(value, items, *min_items, *max_items, location, path, errors);
        }
        TypeDescriptor::Tuple { items } => {
            validate_tuple(value, items, location, path, errors);
        }
        TypeDescriptor::Set { items } => {
            validate_set(value, items, location, path, errors);
        }
        TypeDescriptor::Object { fields, additional_properties } => {
            validate_object(value, fields, additional_properties.as_deref(), location, path, errors);
        }
        TypeDescriptor::Optional(inner) => {
            if !matches!(value, SerializableValue::Null) {
                validate_type(value, inner, location, path, errors);
            }
        }
        TypeDescriptor::Union { variants, nullable } => {
            validate_union(value, variants, *nullable, location, path, errors);
        }
        TypeDescriptor::Uuid => {
            validate_uuid(value, location, path, errors);
        }
        TypeDescriptor::Email => {
            validate_email(value, location, path, errors);
        }
        TypeDescriptor::Url => {
            validate_url(value, location, path, errors);
        }
        TypeDescriptor::DateTime => {
            validate_datetime(value, location, path, errors);
        }
        TypeDescriptor::Date => {
            validate_date(value, location, path, errors);
        }
        TypeDescriptor::Time => {
            validate_time(value, location, path, errors);
        }
        TypeDescriptor::Decimal => {
            validate_decimal(value, location, path, errors);
        }
        TypeDescriptor::Enum { values } => {
            validate_enum(value, values, location, path, errors);
        }
        TypeDescriptor::Literal { values } => {
            validate_literal(value, values, location, path, errors);
        }
        TypeDescriptor::Any => {
            // No validation
        }
    }
}

// ============================================================================
// String Validation
// ============================================================================

/// Email regex pattern
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

/// URL regex pattern (http/https)
static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
});

/// UUID regex pattern (v4)
static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$").unwrap()
});

/// ISO 8601 DateTime regex pattern
static DATETIME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$").unwrap()
});

/// Date regex pattern (YYYY-MM-DD)
static DATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());

/// Time regex pattern (HH:MM:SS)
static TIME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d{1,9})?$").unwrap());

fn validate_string(
    value: &SerializableValue,
    constraints: &StringConstraints,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            // Check length constraints
            let char_count = s.chars().count();

            if let Some(min) = constraints.min_length {
                if char_count < min {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("String must be at least {} characters (got {})", min, char_count),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            if let Some(max) = constraints.max_length {
                if char_count > max {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("String must be at most {} characters (got {})", max, char_count),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            // Check pattern (regex)
            if let Some(pattern) = &constraints.pattern {
                match Regex::new(pattern) {
                    Ok(re) => {
                        if !re.is_match(s) {
                            errors.add(ValidationError {
                                location: location.to_string(),
                                field: field.to_string(),
                                message: format!("String does not match pattern: {}", pattern),
                                error_type: "value_error".to_string(),
                            });
                        }
                    }
                    Err(_) => {
                        errors.add(ValidationError {
                            location: location.to_string(),
                            field: field.to_string(),
                            message: format!("Invalid regex pattern: {}", pattern),
                            error_type: "value_error".to_string(),
                        });
                    }
                }
            }

            // Check format
            if let Some(format) = constraints.format {
                match format {
                    StringFormat::Email => validate_email_format(s, location, field, errors),
                    StringFormat::Url => validate_url_format(s, location, field, errors),
                    StringFormat::Uuid => validate_uuid_format(s, location, field, errors),
                    StringFormat::DateTime => validate_datetime_format(s, location, field, errors),
                    StringFormat::Date => validate_date_format(s, location, field, errors),
                    StringFormat::Time => validate_time_format(s, location, field, errors),
                }
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected string".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_email_format(s: &str, location: &str, field: &str, errors: &mut ValidationErrors) {
    if !EMAIL_REGEX.is_match(s) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Invalid email format".to_string(),
            error_type: "value_error".to_string(),
        });
    }
}

fn validate_url_format(s: &str, location: &str, field: &str, errors: &mut ValidationErrors) {
    if !URL_REGEX.is_match(s) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Invalid URL format".to_string(),
            error_type: "value_error".to_string(),
        });
    }
}

fn validate_uuid_format(s: &str, location: &str, field: &str, errors: &mut ValidationErrors) {
    if !UUID_REGEX.is_match(s) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Invalid UUID format".to_string(),
            error_type: "value_error".to_string(),
        });
    }
}

fn validate_datetime_format(s: &str, location: &str, field: &str, errors: &mut ValidationErrors) {
    if !DATETIME_REGEX.is_match(s) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Invalid datetime format (expected ISO 8601)".to_string(),
            error_type: "value_error".to_string(),
        });
    }
}

fn validate_date_format(s: &str, location: &str, field: &str, errors: &mut ValidationErrors) {
    if !DATE_REGEX.is_match(s) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Invalid date format (expected YYYY-MM-DD)".to_string(),
            error_type: "value_error".to_string(),
        });
    }
}

fn validate_time_format(s: &str, location: &str, field: &str, errors: &mut ValidationErrors) {
    if !TIME_REGEX.is_match(s) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Invalid time format (expected HH:MM:SS)".to_string(),
            error_type: "value_error".to_string(),
        });
    }
}

// ============================================================================
// Numeric Validation
// ============================================================================

fn validate_int(
    value: &SerializableValue,
    constraints: &NumericConstraints<i64>,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::Int(n) => {
            // Minimum (inclusive)
            if let Some(min) = constraints.minimum {
                if *n < min {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("Value must be >= {} (got {})", min, n),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            // Maximum (inclusive)
            if let Some(max) = constraints.maximum {
                if *n > max {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("Value must be <= {} (got {})", max, n),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            // Exclusive minimum
            if let Some(min) = constraints.exclusive_minimum {
                if *n <= min {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("Value must be > {} (got {})", min, n),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            // Exclusive maximum
            if let Some(max) = constraints.exclusive_maximum {
                if *n >= max {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("Value must be < {} (got {})", max, n),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            // Multiple of
            if let Some(multiple) = constraints.multiple_of {
                if multiple != 0 && n % multiple != 0 {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: field.to_string(),
                        message: format!("Value must be a multiple of {}", multiple),
                        error_type: "value_error".to_string(),
                    });
                }
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected integer".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_float(
    value: &SerializableValue,
    constraints: &NumericConstraints<f64>,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    let num = match value {
        SerializableValue::Float(f) => *f,
        SerializableValue::Int(i) => *i as f64,
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected number".to_string(),
                error_type: "type_error".to_string(),
            });
            return;
        }
    };

    // Minimum (inclusive)
    if let Some(min) = constraints.minimum {
        if num < min {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: format!("Value must be >= {} (got {})", min, num),
                error_type: "value_error".to_string(),
            });
        }
    }

    // Maximum (inclusive)
    if let Some(max) = constraints.maximum {
        if num > max {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: format!("Value must be <= {} (got {})", max, num),
                error_type: "value_error".to_string(),
            });
        }
    }

    // Exclusive minimum
    if let Some(min) = constraints.exclusive_minimum {
        if num <= min {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: format!("Value must be > {} (got {})", min, num),
                error_type: "value_error".to_string(),
            });
        }
    }

    // Exclusive maximum
    if let Some(max) = constraints.exclusive_maximum {
        if num >= max {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: format!("Value must be < {} (got {})", max, num),
                error_type: "value_error".to_string(),
            });
        }
    }

    // Multiple of
    if let Some(multiple) = constraints.multiple_of {
        if multiple != 0.0 {
            // Use a relative epsilon for floating point comparison
            let remainder = (num % multiple).abs();
            let tolerance = multiple * 0.0001; // 0.01% tolerance
            if remainder > tolerance && (multiple - remainder).abs() > tolerance {
                errors.add(ValidationError {
                    location: location.to_string(),
                    field: field.to_string(),
                    message: format!("Value must be a multiple of {}", multiple),
                    error_type: "value_error".to_string(),
                });
            }
        }
    }
}

// ============================================================================
// Other Type Validation
// ============================================================================

fn validate_bool(
    value: &SerializableValue,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    if !matches!(value, SerializableValue::Bool(_)) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: field.to_string(),
            message: "Expected boolean".to_string(),
            error_type: "type_error".to_string(),
        });
    }
}

fn validate_list(
    value: &SerializableValue,
    item_type: &TypeDescriptor,
    min_items: Option<usize>,
    max_items: Option<usize>,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::List(items) => {
            // Check length constraints
            if let Some(min) = min_items {
                if items.len() < min {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: path.to_string(),
                        message: format!("List must have at least {} items (got {})", min, items.len()),
                        error_type: "value_error".to_string(),
                    });
                }
            }
            if let Some(max) = max_items {
                if items.len() > max {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: path.to_string(),
                        message: format!("List must have at most {} items (got {})", max, items.len()),
                        error_type: "value_error".to_string(),
                    });
                }
            }

            // Validate each item
            for (i, item) in items.iter().enumerate() {
                let item_path = if path.is_empty() {
                    format!("[{}]", i)
                } else {
                    format!("{}[{}]", path, i)
                };
                validate_type(item, item_type, location, &item_path, errors);
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected array".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_object(
    value: &SerializableValue,
    fields: &[FieldDescriptor],
    additional_properties: Option<&TypeDescriptor>,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::Object(pairs) => {
            let obj_map: HashMap<&str, &SerializableValue> =
                pairs.iter().map(|(k, v)| (k.as_str(), v)).collect();

            // Validate required and known fields
            for field_desc in fields {
                let field_path = if path.is_empty() {
                    field_desc.name.clone()
                } else {
                    format!("{}.{}", path, field_desc.name)
                };

                match obj_map.get(field_desc.name.as_str()) {
                    Some(field_value) => {
                        validate_type(
                            field_value,
                            &field_desc.type_desc,
                            location,
                            &field_path,
                            errors,
                        );
                    }
                    None if field_desc.required => {
                        if field_desc.default.is_none() {
                            errors.add(ValidationError {
                                location: location.to_string(),
                                field: field_path,
                                message: format!("Field '{}' is required", field_desc.name),
                                error_type: "missing".to_string(),
                            });
                        }
                    }
                    None => {
                        // Optional field missing, no error
                    }
                }
            }

            // Validate additional properties if specified
            if let Some(additional_type) = additional_properties {
                let known_fields: std::collections::HashSet<&str> =
                    fields.iter().map(|f| f.name.as_str()).collect();

                for (key, val) in pairs {
                    if !known_fields.contains(key.as_str()) {
                        let field_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        validate_type(val, additional_type, location, &field_path, errors);
                    }
                }
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected object".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_uuid(
    value: &SerializableValue,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            validate_uuid_format(s, location, field, errors);
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected UUID string".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_email(
    value: &SerializableValue,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            validate_email_format(s, location, field, errors);
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected email string".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_url(
    value: &SerializableValue,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            validate_url_format(s, location, field, errors);
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected URL string".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_datetime(
    value: &SerializableValue,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            validate_datetime_format(s, location, field, errors);
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: field.to_string(),
                message: "Expected datetime string (ISO 8601)".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

// ============================================================================
// New Type Validation Functions
// ============================================================================

fn validate_bytes(
    value: &SerializableValue,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    if !matches!(value, SerializableValue::Bytes(_)) {
        errors.add(ValidationError {
            location: location.to_string(),
            field: path.to_string(),
            message: "Expected bytes".to_string(),
            error_type: "type_error".to_string(),
        });
    }
}

fn validate_tuple(
    value: &SerializableValue,
    items: &[TypeDescriptor],
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::List(list) => {
            // Check exact length
            if list.len() != items.len() {
                errors.add(ValidationError {
                    location: location.to_string(),
                    field: path.to_string(),
                    message: format!("Tuple must have exactly {} items (got {})", items.len(), list.len()),
                    error_type: "value_error".to_string(),
                });
                return;
            }

            // Validate each item with its corresponding type
            for (i, (item, item_type)) in list.iter().zip(items.iter()).enumerate() {
                let item_path = if path.is_empty() {
                    format!("[{}]", i)
                } else {
                    format!("{}[{}]", path, i)
                };
                validate_type(item, item_type, location, &item_path, errors);
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected tuple (array)".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_set(
    value: &SerializableValue,
    items: &TypeDescriptor,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::List(list) => {
            // Check for duplicates
            let mut seen = std::collections::HashSet::new();
            for (i, item) in list.iter().enumerate() {
                if !seen.insert(format!("{:?}", item)) {
                    errors.add(ValidationError {
                        location: location.to_string(),
                        field: path.to_string(),
                        message: format!("Set contains duplicate value at index {}", i),
                        error_type: "value_error".to_string(),
                    });
                }

                // Validate item type
                let item_path = if path.is_empty() {
                    format!("[{}]", i)
                } else {
                    format!("{}[{}]", path, i)
                };
                validate_type(item, items, location, &item_path, errors);
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected set (array with unique items)".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_union(
    value: &SerializableValue,
    variants: &[TypeDescriptor],
    nullable: bool,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    // Check null
    if matches!(value, SerializableValue::Null) {
        if nullable {
            return; // null is allowed
        } else {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Value cannot be null".to_string(),
                error_type: "type_error".to_string(),
            });
            return;
        }
    }

    // Try each variant
    for variant in variants {
        let mut temp_errors = ValidationErrors::new();
        validate_type(value, variant, location, path, &mut temp_errors);
        if temp_errors.is_empty() {
            return; // Match successful
        }
    }

    // No variant matched
    let variant_types: Vec<String> = variants.iter()
        .map(|v| v.type_name().to_string())
        .collect();
    errors.add(ValidationError {
        location: location.to_string(),
        field: path.to_string(),
        message: format!("Value does not match any of: [{}]", variant_types.join(", ")),
        error_type: "type_error".to_string(),
    });
}

fn validate_enum(
    value: &SerializableValue,
    allowed_values: &[SerializableValue],
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    if !allowed_values.contains(value) {
        // Format allowed values for error message
        let formatted_values: Vec<String> = allowed_values.iter()
            .map(|v| match v {
                SerializableValue::String(s) => format!("\"{}\"", s),
                SerializableValue::Int(i) => i.to_string(),
                SerializableValue::Float(f) => f.to_string(),
                SerializableValue::Bool(b) => b.to_string(),
                _ => format!("{:?}", v),
            })
            .collect();

        errors.add(ValidationError {
            location: location.to_string(),
            field: path.to_string(),
            message: format!("Value must be one of: [{}]", formatted_values.join(", ")),
            error_type: "value_error".to_string(),
        });
    }
}

fn validate_literal(
    value: &SerializableValue,
    allowed_values: &[SerializableValue],
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    // Literal and Enum validation logic is the same
    validate_enum(value, allowed_values, location, path, errors);
}

fn validate_date(
    value: &SerializableValue,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            validate_date_format(s, location, path, errors);
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected date string (YYYY-MM-DD)".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_time(
    value: &SerializableValue,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::String(s) => {
            validate_time_format(s, location, path, errors);
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected time string (HH:MM:SS)".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

fn validate_decimal(
    value: &SerializableValue,
    location: &str,
    path: &str,
    errors: &mut ValidationErrors,
) {
    match value {
        SerializableValue::Float(_) | SerializableValue::Int(_) => {
            // Accept both float and int for decimal
        }
        SerializableValue::String(s) => {
            // Also accept string representation of decimal
            if s.parse::<f64>().is_err() {
                errors.add(ValidationError {
                    location: location.to_string(),
                    field: path.to_string(),
                    message: "Invalid decimal format".to_string(),
                    error_type: "value_error".to_string(),
                });
            }
        }
        _ => {
            errors.add(ValidationError {
                location: location.to_string(),
                field: path.to_string(),
                message: "Expected decimal (number or string)".to_string(),
                error_type: "type_error".to_string(),
            });
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_string_length() {
        let mut errors = ValidationErrors::new();
        let constraints = StringConstraints {
            min_length: Some(3),
            max_length: Some(10),
            pattern: None,
            format: None,
        };

        // Too short
        validate_string(
            &SerializableValue::String("ab".to_string()),
            &constraints,
            "body",
            "name",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 1);

        // Just right
        errors = ValidationErrors::new();
        validate_string(
            &SerializableValue::String("abc".to_string()),
            &constraints,
            "body",
            "name",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 0);

        // Too long
        errors = ValidationErrors::new();
        validate_string(
            &SerializableValue::String("abcdefghijk".to_string()),
            &constraints,
            "body",
            "name",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_validate_email() {
        let mut errors = ValidationErrors::new();

        validate_email(
            &SerializableValue::String("test@example.com".to_string()),
            "query",
            "email",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 0);

        validate_email(
            &SerializableValue::String("invalid-email".to_string()),
            "query",
            "email",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_validate_int_range() {
        let mut errors = ValidationErrors::new();
        let constraints = NumericConstraints {
            minimum: Some(0),
            maximum: Some(100),
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        };

        // Below minimum
        validate_int(
            &SerializableValue::Int(-1),
            &constraints,
            "query",
            "age",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 1);

        // Within range
        errors = ValidationErrors::new();
        validate_int(
            &SerializableValue::Int(50),
            &constraints,
            "query",
            "age",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 0);

        // Above maximum
        errors = ValidationErrors::new();
        validate_int(
            &SerializableValue::Int(101),
            &constraints,
            "query",
            "age",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_validate_uuid() {
        let mut errors = ValidationErrors::new();

        // Valid UUID v4
        validate_uuid(
            &SerializableValue::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
            "path",
            "id",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 0);

        // Invalid UUID
        validate_uuid(
            &SerializableValue::String("not-a-uuid".to_string()),
            "path",
            "id",
            &mut errors,
        );
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_request_validator() {
        let mut validator = RequestValidator::new();
        validator.path_params.push(ParamValidator {
            name: "user_id".to_string(),
            location: ParamLocation::Path,
            type_desc: TypeDescriptor::Int(Default::default()),
            required: true,
            default: None,
        });

        let mut path_params = HashMap::new();
        path_params.insert("user_id".to_string(), "123".to_string());

        let result = validator.validate(&path_params, &HashMap::new(), &HashMap::new(), None);

        // Should fail because path params are strings, not parsed ints
        assert!(result.is_err());
    }

    #[test]
    fn test_path_params_passthrough() {
        // Test that path params are passed through even when no validators are defined
        let validator = RequestValidator::new();

        let mut path_params = HashMap::new();
        path_params.insert("user_id".to_string(), "user-123".to_string());
        path_params.insert("resource_id".to_string(), "resource-456".to_string());

        let result = validator.validate(&path_params, &HashMap::new(), &HashMap::new(), None);

        assert!(result.is_ok());
        let validated = result.unwrap();

        // Both path params should be present
        assert_eq!(validated.path_params.len(), 2);
        assert_eq!(
            validated.path_params.get("user_id"),
            Some(&SerializableValue::String("user-123".to_string()))
        );
        assert_eq!(
            validated.path_params.get("resource_id"),
            Some(&SerializableValue::String("resource-456".to_string()))
        );
    }

    #[test]
    fn test_path_params_with_partial_validation() {
        // Test that some path params can have validators while others pass through
        let mut validator = RequestValidator::new();
        validator.path_params.push(ParamValidator {
            name: "user_id".to_string(),
            location: ParamLocation::Path,
            type_desc: TypeDescriptor::String(StringConstraints {
                min_length: Some(5),
                max_length: None,
                pattern: None,
                format: None,
            }),
            required: true,
            default: None,
        });

        let mut path_params = HashMap::new();
        path_params.insert("user_id".to_string(), "user-123".to_string());
        path_params.insert("resource_id".to_string(), "resource-456".to_string());

        let result = validator.validate(&path_params, &HashMap::new(), &HashMap::new(), None);

        assert!(result.is_ok());
        let validated = result.unwrap();

        // Both path params should be present
        assert_eq!(validated.path_params.len(), 2);
        assert_eq!(
            validated.path_params.get("user_id"),
            Some(&SerializableValue::String("user-123".to_string()))
        );
        assert_eq!(
            validated.path_params.get("resource_id"),
            Some(&SerializableValue::String("resource-456".to_string()))
        );
    }

    #[test]
    fn test_validate_list() {
        let mut errors = ValidationErrors::new();
        let item_type = TypeDescriptor::Int(Default::default());

        let value = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::Int(2),
            SerializableValue::Int(3),
        ]);

        validate_list(&value, &item_type, None, None, "body", "items", &mut errors);
        assert_eq!(errors.errors.len(), 0);

        // Invalid item type
        let value = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::String("not an int".to_string()),
        ]);

        validate_list(&value, &item_type, None, None, "body", "items", &mut errors);
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_nested_object_validation() {
        let address_type = TypeDescriptor::Object {
            fields: vec![
                FieldDescriptor {
                    name: "city".to_string(),
                    type_desc: TypeDescriptor::String(StringConstraints::default()),
                    required: true,
                    default: None,
                    description: None,
                },
                FieldDescriptor {
                    name: "zip".to_string(),
                    type_desc: TypeDescriptor::String(StringConstraints::default()),
                    required: false,
                    default: None,
                    description: None,
                },
            ],
            additional_properties: None,
        };

        let user_type = TypeDescriptor::Object {
            fields: vec![
                FieldDescriptor {
                    name: "name".to_string(),
                    type_desc: TypeDescriptor::String(StringConstraints::default()),
                    required: true,
                    default: None,
                    description: None,
                },
                FieldDescriptor {
                    name: "address".to_string(),
                    type_desc: address_type,
                    required: true,
                    default: None,
                    description: None,
                },
            ],
            additional_properties: None,
        };

        // Valid nested object
        let valid_user = SerializableValue::Object(vec![
            ("name".to_string(), SerializableValue::String("Alice".to_string())),
            ("address".to_string(), SerializableValue::Object(vec![
                ("city".to_string(), SerializableValue::String("NYC".to_string())),
            ])),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&valid_user, &user_type, "body", "", &mut errors);
        assert!(errors.is_empty());

        // Missing nested field
        let invalid_user = SerializableValue::Object(vec![
            ("name".to_string(), SerializableValue::String("Alice".to_string())),
            ("address".to_string(), SerializableValue::Object(vec![])),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&invalid_user, &user_type, "body", "", &mut errors);
        assert!(!errors.is_empty());
        assert!(errors.errors[0].field.contains("address.city"));
    }

    #[test]
    fn test_list_item_validation() {
        let list_type = TypeDescriptor::List {
            items: Box::new(TypeDescriptor::Int(NumericConstraints {
                minimum: Some(0),
                maximum: Some(100),
                exclusive_minimum: None,
                exclusive_maximum: None,
                multiple_of: None,
            })),
            min_items: Some(1),
            max_items: Some(5),
        };

        // Valid list
        let valid_list = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::Int(50),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&valid_list, &list_type, "body", "scores", &mut errors);
        assert!(errors.is_empty());

        // Invalid item in list
        let invalid_list = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::Int(200), // > 100
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&invalid_list, &list_type, "body", "scores", &mut errors);
        assert!(!errors.is_empty());
        assert!(errors.errors[0].field.contains("scores[1]"));

        // Too few items
        let too_few = SerializableValue::List(vec![]);
        let mut errors = ValidationErrors::new();
        validate_type(&too_few, &list_type, "body", "scores", &mut errors);
        assert!(!errors.is_empty());

        // Too many items
        let too_many = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::Int(2),
            SerializableValue::Int(3),
            SerializableValue::Int(4),
            SerializableValue::Int(5),
            SerializableValue::Int(6), // 6th item
        ]);
        let mut errors = ValidationErrors::new();
        validate_type(&too_many, &list_type, "body", "scores", &mut errors);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_union_validation() {
        let union_type = TypeDescriptor::Union {
            variants: vec![
                TypeDescriptor::String(StringConstraints::default()),
                TypeDescriptor::Int(NumericConstraints::default()),
            ],
            nullable: false,
        };

        // String is valid
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::String("hello".to_string()), &union_type, "body", "value", &mut errors);
        assert!(errors.is_empty());

        // Int is valid
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::Int(42), &union_type, "body", "value", &mut errors);
        assert!(errors.is_empty());

        // Float is invalid
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::Float(3.14), &union_type, "body", "value", &mut errors);
        assert!(!errors.is_empty());

        // Null is invalid
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::Null, &union_type, "body", "value", &mut errors);
        assert!(!errors.is_empty());

        // Test nullable union
        let nullable_union = TypeDescriptor::Union {
            variants: vec![TypeDescriptor::String(StringConstraints::default())],
            nullable: true,
        };
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::Null, &nullable_union, "body", "value", &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_enum_validation() {
        let enum_type = TypeDescriptor::Enum {
            values: vec![
                SerializableValue::String("active".to_string()),
                SerializableValue::String("inactive".to_string()),
                SerializableValue::String("pending".to_string()),
            ],
        };

        // Valid enum value
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::String("active".to_string()), &enum_type, "body", "status", &mut errors);
        assert!(errors.is_empty());

        // Invalid enum value
        let mut errors = ValidationErrors::new();
        validate_type(&SerializableValue::String("unknown".to_string()), &enum_type, "body", "status", &mut errors);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_tuple_validation() {
        let tuple_type = TypeDescriptor::Tuple {
            items: vec![
                TypeDescriptor::String(StringConstraints::default()),
                TypeDescriptor::Int(NumericConstraints::default()),
                TypeDescriptor::Bool,
            ],
        };

        // Valid tuple
        let valid_tuple = SerializableValue::List(vec![
            SerializableValue::String("test".to_string()),
            SerializableValue::Int(42),
            SerializableValue::Bool(true),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&valid_tuple, &tuple_type, "body", "data", &mut errors);
        assert!(errors.is_empty());

        // Wrong length
        let wrong_length = SerializableValue::List(vec![
            SerializableValue::String("test".to_string()),
            SerializableValue::Int(42),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&wrong_length, &tuple_type, "body", "data", &mut errors);
        assert!(!errors.is_empty());

        // Wrong type at index
        let wrong_type = SerializableValue::List(vec![
            SerializableValue::String("test".to_string()),
            SerializableValue::String("not an int".to_string()), // Should be int
            SerializableValue::Bool(true),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&wrong_type, &tuple_type, "body", "data", &mut errors);
        assert!(!errors.is_empty());
        assert!(errors.errors[0].field.contains("data[1]"));
    }

    #[test]
    fn test_set_validation() {
        let set_type = TypeDescriptor::Set {
            items: Box::new(TypeDescriptor::Int(NumericConstraints::default())),
        };

        // Valid set (unique items)
        let valid_set = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::Int(2),
            SerializableValue::Int(3),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&valid_set, &set_type, "body", "numbers", &mut errors);
        assert!(errors.is_empty());

        // Invalid set (duplicate items)
        let invalid_set = SerializableValue::List(vec![
            SerializableValue::Int(1),
            SerializableValue::Int(2),
            SerializableValue::Int(1), // Duplicate
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&invalid_set, &set_type, "body", "numbers", &mut errors);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_additional_properties() {
        let obj_type = TypeDescriptor::Object {
            fields: vec![
                FieldDescriptor {
                    name: "known_field".to_string(),
                    type_desc: TypeDescriptor::String(StringConstraints::default()),
                    required: true,
                    default: None,
                    description: None,
                },
            ],
            additional_properties: Some(Box::new(TypeDescriptor::Int(NumericConstraints::default()))),
        };

        // Valid with additional int property
        let valid = SerializableValue::Object(vec![
            ("known_field".to_string(), SerializableValue::String("value".to_string())),
            ("extra_field".to_string(), SerializableValue::Int(42)),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&valid, &obj_type, "body", "", &mut errors);
        assert!(errors.is_empty());

        // Invalid additional property (wrong type)
        let invalid = SerializableValue::Object(vec![
            ("known_field".to_string(), SerializableValue::String("value".to_string())),
            ("extra_field".to_string(), SerializableValue::String("not an int".to_string())),
        ]);

        let mut errors = ValidationErrors::new();
        validate_type(&invalid, &obj_type, "body", "", &mut errors);
        assert!(!errors.is_empty());
        assert!(errors.errors[0].field.contains("extra_field"));
    }

    #[test]
    fn test_query_params_passthrough() {
        // Test that query params are passed through even when no validators are defined
        let validator = RequestValidator::new();

        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), SerializableValue::String("test".to_string()));
        query_params.insert("page".to_string(), SerializableValue::Int(1));
        query_params.insert("limit".to_string(), SerializableValue::Int(10));

        let result = validator.validate(&HashMap::new(), &query_params, &HashMap::new(), None);

        assert!(result.is_ok());
        let validated = result.unwrap();

        // All query params should be present
        assert_eq!(validated.query_params.len(), 3);
        assert_eq!(
            validated.query_params.get("search"),
            Some(&SerializableValue::String("test".to_string()))
        );
        assert_eq!(
            validated.query_params.get("page"),
            Some(&SerializableValue::Int(1))
        );
        assert_eq!(
            validated.query_params.get("limit"),
            Some(&SerializableValue::Int(10))
        );
    }

    #[test]
    fn test_query_params_with_partial_validation() {
        // Test that some query params can have validators while others pass through
        let mut validator = RequestValidator::new();
        validator.query_params.push(ParamValidator {
            name: "page".to_string(),
            location: ParamLocation::Query,
            type_desc: TypeDescriptor::Int(NumericConstraints {
                minimum: Some(1),
                maximum: Some(1000),
                exclusive_minimum: None,
                exclusive_maximum: None,
                multiple_of: None,
            }),
            required: false,
            default: Some(SerializableValue::Int(1)),
        });

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), SerializableValue::Int(5));
        query_params.insert("search".to_string(), SerializableValue::String("test".to_string()));
        query_params.insert("filter".to_string(), SerializableValue::String("active".to_string()));

        let result = validator.validate(&HashMap::new(), &query_params, &HashMap::new(), None);

        assert!(result.is_ok());
        let validated = result.unwrap();

        // All query params should be present, with page validated
        assert_eq!(validated.query_params.len(), 3);
        assert_eq!(
            validated.query_params.get("page"),
            Some(&SerializableValue::Int(5))
        );
        assert_eq!(
            validated.query_params.get("search"),
            Some(&SerializableValue::String("test".to_string()))
        );
        assert_eq!(
            validated.query_params.get("filter"),
            Some(&SerializableValue::String("active".to_string()))
        );
    }

    #[test]
    fn test_query_params_validation_failure_still_passes_through() {
        // Test that even when validation fails, other params are still passed through
        let mut validator = RequestValidator::new();
        validator.query_params.push(ParamValidator {
            name: "page".to_string(),
            location: ParamLocation::Query,
            type_desc: TypeDescriptor::Int(NumericConstraints {
                minimum: Some(1),
                maximum: Some(100),
                exclusive_minimum: None,
                exclusive_maximum: None,
                multiple_of: None,
            }),
            required: true,
            default: None,
        });

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), SerializableValue::Int(999)); // Invalid (> 100)
        query_params.insert("search".to_string(), SerializableValue::String("test".to_string()));

        let result = validator.validate(&HashMap::new(), &query_params, &HashMap::new(), None);

        // Should fail validation
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors.len(), 1);
        assert_eq!(errors.errors[0].field, "page");
    }
}
