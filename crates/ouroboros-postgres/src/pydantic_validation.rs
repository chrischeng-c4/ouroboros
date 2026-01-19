//! Pydantic-style validation integration for model fields.
//!
//! Provides support for custom field validators, model validators,
//! and computed fields similar to Pydantic/SQLModel.

use crate::{DataBridgeError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Validation Error
// ============================================================================

/// Validation error with location information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error location (field path)
    pub loc: Vec<String>,
    /// Error message
    pub msg: String,
    /// Error type
    pub error_type: String,
    /// Input value that caused the error (as string)
    pub input: Option<String>,
}

impl ValidationError {
    /// Create a field validation error.
    pub fn field(field: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            loc: vec![field.into()],
            msg: msg.into(),
            error_type: "value_error".to_string(),
            input: None,
        }
    }

    /// Create with a specific error type.
    pub fn with_type(mut self, error_type: impl Into<String>) -> Self {
        self.error_type = error_type.into();
        self
    }

    /// Set the input value.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Create a nested error location.
    pub fn nested(mut self, parent: impl Into<String>) -> Self {
        self.loc.insert(0, parent.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.loc.join("."), self.msg)
    }
}

/// Collection of validation errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationErrors {
    /// List of errors
    pub errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Create empty validation errors.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error.
    pub fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get error count.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Convert to Result.
    pub fn into_result<T>(self, value: T) -> Result<T> {
        if self.is_empty() {
            Ok(value)
        } else {
            Err(DataBridgeError::Validation(self.to_string()))
        }
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msgs: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
        write!(f, "Validation failed: {}", msgs.join("; "))
    }
}

// ============================================================================
// Field Validator
// ============================================================================

/// Validation mode for field validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationMode {
    /// Run before other validators
    Before,
    /// Run after type coercion (default)
    #[default]
    After,
    /// Replace the value entirely
    Wrap,
    /// Only validate without modifying
    Plain,
}

/// Field validator configuration.
#[derive(Debug, Clone)]
pub struct FieldValidatorConfig {
    /// Field name to validate
    pub field_name: String,
    /// Validation mode
    pub mode: ValidationMode,
    /// Check fields for validation context
    pub check_fields: bool,
    /// Validator ID (for tracking)
    pub validator_id: String,
}

impl FieldValidatorConfig {
    /// Create a new field validator config.
    pub fn new(field_name: impl Into<String>) -> Self {
        let field_name = field_name.into();
        Self {
            validator_id: format!("validator_{}", field_name),
            field_name,
            mode: ValidationMode::default(),
            check_fields: true,
        }
    }

    /// Set validation mode.
    pub fn mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set check_fields flag.
    pub fn check_fields(mut self, check: bool) -> Self {
        self.check_fields = check;
        self
    }
}

/// Trait for field validators.
pub trait FieldValidator: Send + Sync {
    /// Get validator configuration.
    fn config(&self) -> &FieldValidatorConfig;

    /// Validate a string value.
    fn validate_string(&self, value: &str) -> std::result::Result<String, ValidationError>;

    /// Validate an integer value.
    fn validate_int(&self, value: i64) -> std::result::Result<i64, ValidationError> {
        Ok(value)
    }

    /// Validate a float value.
    fn validate_float(&self, value: f64) -> std::result::Result<f64, ValidationError> {
        Ok(value)
    }

    /// Validate a boolean value.
    fn validate_bool(&self, value: bool) -> std::result::Result<bool, ValidationError> {
        Ok(value)
    }
}

// ============================================================================
// Built-in Validators
// ============================================================================

/// Email validator.
pub struct EmailValidator {
    config: FieldValidatorConfig,
}

impl EmailValidator {
    /// Create a new email validator.
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            config: FieldValidatorConfig::new(field_name),
        }
    }
}

impl FieldValidator for EmailValidator {
    fn config(&self) -> &FieldValidatorConfig {
        &self.config
    }

    fn validate_string(&self, value: &str) -> std::result::Result<String, ValidationError> {
        if !value.contains('@') || !value.contains('.') {
            return Err(
                ValidationError::field(&self.config.field_name, "Invalid email format")
                    .with_type("value_error.email")
                    .with_input(value),
            );
        }

        // Basic email validation
        let parts: Vec<&str> = value.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(
                ValidationError::field(&self.config.field_name, "Invalid email format")
                    .with_type("value_error.email")
                    .with_input(value),
            );
        }

        Ok(value.to_lowercase())
    }
}

/// URL validator.
pub struct UrlValidator {
    config: FieldValidatorConfig,
    allowed_schemes: Vec<String>,
}

impl UrlValidator {
    /// Create a new URL validator.
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            config: FieldValidatorConfig::new(field_name),
            allowed_schemes: vec!["http".to_string(), "https".to_string()],
        }
    }

    /// Set allowed URL schemes.
    pub fn allowed_schemes(mut self, schemes: Vec<String>) -> Self {
        self.allowed_schemes = schemes;
        self
    }
}

impl FieldValidator for UrlValidator {
    fn config(&self) -> &FieldValidatorConfig {
        &self.config
    }

    fn validate_string(&self, value: &str) -> std::result::Result<String, ValidationError> {
        // Basic URL validation
        let has_scheme = self.allowed_schemes.iter().any(|s| {
            value.starts_with(&format!("{}://", s))
        });

        if !has_scheme {
            return Err(ValidationError::field(
                &self.config.field_name,
                format!("URL must start with one of: {:?}", self.allowed_schemes),
            )
            .with_type("value_error.url")
            .with_input(value));
        }

        Ok(value.to_string())
    }
}

/// Length validator.
pub struct LengthValidator {
    config: FieldValidatorConfig,
    min_length: Option<usize>,
    max_length: Option<usize>,
}

impl LengthValidator {
    /// Create a new length validator.
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            config: FieldValidatorConfig::new(field_name),
            min_length: None,
            max_length: None,
        }
    }

    /// Set minimum length.
    pub fn min(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Set maximum length.
    pub fn max(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }
}

impl FieldValidator for LengthValidator {
    fn config(&self) -> &FieldValidatorConfig {
        &self.config
    }

    fn validate_string(&self, value: &str) -> std::result::Result<String, ValidationError> {
        let len = value.len();

        if let Some(min) = self.min_length {
            if len < min {
                return Err(ValidationError::field(
                    &self.config.field_name,
                    format!("String must be at least {} characters", min),
                )
                .with_type("value_error.string.min_length")
                .with_input(value));
            }
        }

        if let Some(max) = self.max_length {
            if len > max {
                return Err(ValidationError::field(
                    &self.config.field_name,
                    format!("String must be at most {} characters", max),
                )
                .with_type("value_error.string.max_length")
                .with_input(value));
            }
        }

        Ok(value.to_string())
    }
}

/// Range validator for numbers.
pub struct RangeValidator {
    config: FieldValidatorConfig,
    min: Option<f64>,
    max: Option<f64>,
    exclusive_min: bool,
    exclusive_max: bool,
}

impl RangeValidator {
    /// Create a new range validator.
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            config: FieldValidatorConfig::new(field_name),
            min: None,
            max: None,
            exclusive_min: false,
            exclusive_max: false,
        }
    }

    /// Set minimum value (inclusive).
    pub fn ge(mut self, min: f64) -> Self {
        self.min = Some(min);
        self.exclusive_min = false;
        self
    }

    /// Set minimum value (exclusive).
    pub fn gt(mut self, min: f64) -> Self {
        self.min = Some(min);
        self.exclusive_min = true;
        self
    }

    /// Set maximum value (inclusive).
    pub fn le(mut self, max: f64) -> Self {
        self.max = Some(max);
        self.exclusive_max = false;
        self
    }

    /// Set maximum value (exclusive).
    pub fn lt(mut self, max: f64) -> Self {
        self.max = Some(max);
        self.exclusive_max = true;
        self
    }
}

impl FieldValidator for RangeValidator {
    fn config(&self) -> &FieldValidatorConfig {
        &self.config
    }

    fn validate_string(&self, value: &str) -> std::result::Result<String, ValidationError> {
        Ok(value.to_string())
    }

    fn validate_int(&self, value: i64) -> std::result::Result<i64, ValidationError> {
        self.validate_float(value as f64)?;
        Ok(value)
    }

    fn validate_float(&self, value: f64) -> std::result::Result<f64, ValidationError> {
        if let Some(min) = self.min {
            let valid = if self.exclusive_min {
                value > min
            } else {
                value >= min
            };
            if !valid {
                let op = if self.exclusive_min { ">" } else { ">=" };
                return Err(ValidationError::field(
                    &self.config.field_name,
                    format!("Value must be {} {}", op, min),
                )
                .with_type("value_error.number.not_ge")
                .with_input(value.to_string()));
            }
        }

        if let Some(max) = self.max {
            let valid = if self.exclusive_max {
                value < max
            } else {
                value <= max
            };
            if !valid {
                let op = if self.exclusive_max { "<" } else { "<=" };
                return Err(ValidationError::field(
                    &self.config.field_name,
                    format!("Value must be {} {}", op, max),
                )
                .with_type("value_error.number.not_le")
                .with_input(value.to_string()));
            }
        }

        Ok(value)
    }
}

/// Pattern validator for regex matching.
pub struct PatternValidator {
    config: FieldValidatorConfig,
    pattern: String,
}

impl PatternValidator {
    /// Create a new pattern validator.
    pub fn new(field_name: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            config: FieldValidatorConfig::new(field_name),
            pattern: pattern.into(),
        }
    }
}

impl FieldValidator for PatternValidator {
    fn config(&self) -> &FieldValidatorConfig {
        &self.config
    }

    fn validate_string(&self, value: &str) -> std::result::Result<String, ValidationError> {
        // Use basic pattern matching (in real impl, would use regex crate)
        // For now, we just check if the pattern exists in the value
        if !value.contains(&self.pattern) && !self.pattern.is_empty() {
            return Err(ValidationError::field(
                &self.config.field_name,
                format!("String does not match pattern: {}", self.pattern),
            )
            .with_type("value_error.string.pattern")
            .with_input(value));
        }
        Ok(value.to_string())
    }
}

// ============================================================================
// Model Validator
// ============================================================================

/// Model validator configuration.
#[derive(Debug, Clone)]
pub struct ModelValidatorConfig {
    /// Validation mode
    pub mode: ValidationMode,
    /// Validator ID
    pub validator_id: String,
}

impl Default for ModelValidatorConfig {
    fn default() -> Self {
        Self {
            mode: ValidationMode::After,
            validator_id: "model_validator".to_string(),
        }
    }
}

/// Trait for model-level validators (cross-field validation).
pub trait ModelValidator: Send + Sync {
    /// Get validator configuration.
    fn config(&self) -> &ModelValidatorConfig;

    /// Validate model data.
    fn validate(&self, data: &HashMap<String, serde_json::Value>) -> ValidationErrors;
}

// ============================================================================
// Computed Field
// ============================================================================

/// Computed field configuration.
#[derive(Debug, Clone)]
pub struct ComputedFieldConfig {
    /// Field name
    pub field_name: String,
    /// Whether to include in serialization
    pub repr: bool,
    /// Return type description
    pub return_type: String,
}

impl ComputedFieldConfig {
    /// Create a new computed field config.
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            repr: true,
            return_type: "string".to_string(),
        }
    }

    /// Set repr flag.
    pub fn repr(mut self, repr: bool) -> Self {
        self.repr = repr;
        self
    }

    /// Set return type.
    pub fn return_type(mut self, return_type: impl Into<String>) -> Self {
        self.return_type = return_type.into();
        self
    }
}

/// Trait for computed fields.
pub trait ComputedField: Send + Sync {
    /// Get computed field configuration.
    fn config(&self) -> &ComputedFieldConfig;

    /// Compute the field value.
    fn compute(&self, data: &HashMap<String, serde_json::Value>) -> serde_json::Value;
}

// ============================================================================
// Validation Registry
// ============================================================================

/// Registry for validators and computed fields.
pub struct ValidationRegistry {
    field_validators: Vec<Box<dyn FieldValidator>>,
    model_validators: Vec<Box<dyn ModelValidator>>,
    computed_fields: Vec<Box<dyn ComputedField>>,
}

impl ValidationRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            field_validators: Vec::new(),
            model_validators: Vec::new(),
            computed_fields: Vec::new(),
        }
    }

    /// Register a field validator.
    pub fn add_field_validator(&mut self, validator: Box<dyn FieldValidator>) {
        self.field_validators.push(validator);
    }

    /// Register a model validator.
    pub fn add_model_validator(&mut self, validator: Box<dyn ModelValidator>) {
        self.model_validators.push(validator);
    }

    /// Register a computed field.
    pub fn add_computed_field(&mut self, field: Box<dyn ComputedField>) {
        self.computed_fields.push(field);
    }

    /// Validate a string field.
    pub fn validate_string_field(
        &self,
        field_name: &str,
        value: &str,
    ) -> std::result::Result<String, ValidationErrors> {
        let mut errors = ValidationErrors::new();
        let mut result = value.to_string();

        for validator in &self.field_validators {
            if validator.config().field_name == field_name {
                match validator.validate_string(&result) {
                    Ok(v) => result = v,
                    Err(e) => errors.add(e),
                }
            }
        }

        if errors.is_empty() {
            Ok(result)
        } else {
            Err(errors)
        }
    }

    /// Validate model data.
    pub fn validate_model(
        &self,
        data: &HashMap<String, serde_json::Value>,
    ) -> ValidationErrors {
        let mut errors = ValidationErrors::new();

        for validator in &self.model_validators {
            let model_errors = validator.validate(data);
            for error in model_errors.errors {
                errors.add(error);
            }
        }

        errors
    }

    /// Compute all computed fields.
    pub fn compute_fields(
        &self,
        data: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();

        for field in &self.computed_fields {
            let value = field.compute(data);
            result.insert(field.config().field_name.clone(), value);
        }

        result
    }
}

impl Default for ValidationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validator() {
        let validator = EmailValidator::new("email");

        assert!(validator.validate_string("test@example.com").is_ok());
        assert!(validator.validate_string("invalid").is_err());
        assert!(validator.validate_string("@example.com").is_err());
        assert!(validator.validate_string("test@").is_err());
    }

    #[test]
    fn test_email_normalizes_to_lowercase() {
        let validator = EmailValidator::new("email");
        let result = validator.validate_string("Test@Example.COM").unwrap();
        assert_eq!(result, "test@example.com");
    }

    #[test]
    fn test_length_validator() {
        let validator = LengthValidator::new("name").min(3).max(10);

        assert!(validator.validate_string("hello").is_ok());
        assert!(validator.validate_string("ab").is_err());
        assert!(validator.validate_string("this is too long").is_err());
    }

    #[test]
    fn test_range_validator() {
        let validator = RangeValidator::new("age").ge(0.0).le(120.0);

        assert!(validator.validate_int(25).is_ok());
        assert!(validator.validate_int(-1).is_err());
        assert!(validator.validate_int(150).is_err());
    }

    #[test]
    fn test_range_validator_exclusive() {
        let validator = RangeValidator::new("score").gt(0.0).lt(100.0);

        assert!(validator.validate_float(50.0).is_ok());
        assert!(validator.validate_float(0.0).is_err());
        assert!(validator.validate_float(100.0).is_err());
    }

    #[test]
    fn test_url_validator() {
        let validator = UrlValidator::new("website");

        assert!(validator.validate_string("https://example.com").is_ok());
        assert!(validator.validate_string("http://example.com").is_ok());
        assert!(validator.validate_string("ftp://example.com").is_err());
        assert!(validator.validate_string("example.com").is_err());
    }

    #[test]
    fn test_validation_errors() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());

        errors.add(ValidationError::field("name", "Required"));
        errors.add(ValidationError::field("email", "Invalid"));

        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validation_registry() {
        let mut registry = ValidationRegistry::new();
        registry.add_field_validator(Box::new(EmailValidator::new("email")));
        registry.add_field_validator(Box::new(LengthValidator::new("name").min(2)));

        let email_result = registry.validate_string_field("email", "test@example.com");
        assert!(email_result.is_ok());

        let name_result = registry.validate_string_field("name", "a");
        assert!(name_result.is_err());
    }
}
