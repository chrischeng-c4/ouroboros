//! Request validation for HTTP APIs
//!
//! This module provides comprehensive validation for HTTP request parameters,
//! using the unified `ouroboros-validation` crate for core validation logic.
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
//! use ouroboros_api::validation::{RequestValidator, ParamValidator, ParamLocation};
//! use ouroboros_api::request::SerializableValue;
//! use ouroboros_validation::TypeDescriptor;
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
use std::collections::HashMap;

// Re-export commonly used types from ouroboros-validation
pub use ouroboros_validation::{
    TypeDescriptor, StringConstraints, StringFormat, NumericConstraints,
    FieldDescriptor, ListConstraints,
};

// ============================================================================
// HTTP-Specific Types
// ============================================================================

/// Parameter location in HTTP request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    /// Path parameter (e.g., /users/{id})
    Path,
    /// Query parameter (e.g., ?page=1)
    Query,
    /// HTTP header
    Header,
    /// Request body
    Body,
}

impl ParamLocation {
    /// Get string representation for error messages
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
// Type Conversion: SerializableValue ↔ ouroboros_validation::Value
// ============================================================================

/// Convert SerializableValue to ouroboros_validation::Value for validation
fn to_validation_value(value: &SerializableValue) -> ouroboros_validation::Value {
    match value {
        SerializableValue::Null => ouroboros_validation::Value::Null,
        SerializableValue::Bool(b) => ouroboros_validation::Value::Bool(*b),
        SerializableValue::Int(i) => ouroboros_validation::Value::Int(*i),
        SerializableValue::Float(f) => ouroboros_validation::Value::Float(*f),
        SerializableValue::String(s) => ouroboros_validation::Value::String(s.clone()),
        SerializableValue::Bytes(b) => ouroboros_validation::Value::Bytes(b.clone()),
        SerializableValue::List(items) => {
            ouroboros_validation::Value::List(
                items.iter().map(to_validation_value).collect()
            )
        }
        SerializableValue::Object(pairs) => {
            ouroboros_validation::Value::Object(
                pairs.iter().map(|(k, v)| (k.clone(), to_validation_value(v))).collect()
            )
        }
    }
}

// ============================================================================
// Validation Function (Wrapper around ouroboros-validation)
// ============================================================================

/// Validate a value against a type descriptor
///
/// This is a thin wrapper around `ouroboros_validation::validate_value`
/// that converts between SerializableValue and the validation crate's Value type.
///
/// # Arguments
/// * `value` - Value to validate
/// * `type_desc` - Expected type
/// * `location` - Parameter location (for error messages: "path", "query", "header", "body")
/// * `field` - Field name/path (for error messages)
/// * `errors` - Error accumulator
pub fn validate_type(
    value: &SerializableValue,
    type_desc: &TypeDescriptor,
    location: &str,
    field: &str,
    errors: &mut ValidationErrors,
) {
    // Convert SerializableValue to validation Value
    let validation_value = to_validation_value(value);

    // Create validation context
    let mut ctx = ouroboros_validation::ValidationContext::with_location(location);
    if !field.is_empty() {
        ctx.push(field);
    }

    // Run validation using ouroboros-validation
    let mut validation_errors = ouroboros_validation::ValidationErrors::new();
    ouroboros_validation::validate_value(
        &validation_value,
        type_desc,
        &mut ctx,
        &mut validation_errors,
    );

    // Convert validation errors back to our error format
    for error in validation_errors.as_slice() {
        errors.add(ValidationError {
            location: error.location.clone(),
            field: error.field.clone(),
            message: error.message.clone(),
            error_type: error.error_type.to_string(),
        });
    }
}
