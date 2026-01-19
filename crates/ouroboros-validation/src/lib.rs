//! Ouroboros Validation
//!
//! Unified validation library for the Ouroboros framework.
//!
//! This crate provides Pydantic-like validation with Rust performance,
//! serving as the validation foundation for:
//! - `ouroboros-api`: HTTP request validation
//! - `ouroboros`: MongoDB/BSON validation
//! - `ouroboros-postgres`: PostgreSQL identifier validation
//! - `ouroboros-sheet-*`: Spreadsheet validation
//!
//! # Architecture Vision
//!
//! ```text
//! ouroboros.pyloop      == uvloop             (event loop)
//! ouroboros.api         == uvicorn + fastapi  (web framework)
//! ouroboros.validation  == pydantic + orjson  (validation + JSON) ⭐
//! ```
//!
//! # Features
//!
//! - **Default**: Core validation without serialization
//! - **serde**: JSON serialization with `serde_json`
//! - **sonic**: High-performance JSON with `sonic-rs` (3-7x faster)
//! - **python**: PyO3 bindings for Python integration
//! - **bson**: MongoDB BSON type support
//!
//! # Example
//!
//! ```rust
//! use ouroboros_validation::{TypeDescriptor, Value, validate};
//! use ouroboros_validation::constraints::StringConstraints;
//!
//! // Define type
//! let email_type = TypeDescriptor::Email;
//!
//! // Validate value
//! let value = Value::String("user@example.com".to_string());
//! let result = validate(&value, &email_type);
//! assert!(result.is_ok());
//!
//! // Invalid email
//! let invalid = Value::String("not-an-email".to_string());
//! let result = validate(&invalid, &email_type);
//! assert!(result.is_err());
//! ```

// Public modules
pub mod computed;
pub mod constraints;
pub mod custom_validators;
pub mod errors;
pub mod formats;
pub mod serializers;
pub mod types;
pub mod validators;

// Python bindings (feature-gated)
#[cfg(feature = "python")]
pub mod python;

// Re-export commonly used types
pub use constraints::{
    FieldDescriptor, ListConstraints, NumericConstraints, StringConstraints, StringFormat,
};
pub use custom_validators::{
    BoxedFieldValidator, BoxedModelValidator, FieldValidator, FnFieldValidator, FnModelValidator,
    ModelValidator, ValidatorCollection, ValidatorContext, ValidatorMode,
    field_error, custom_error,
};
pub use serializers::{
    BoxedFieldSerializer, BoxedModelSerializer, FieldSerializer, FnFieldSerializer,
    FnModelSerializer, MaskSerializer, ModelSerializer, SerializerCollection, SerializerContext,
    SerializerMode,
};
pub use computed::{
    BoxedComputedField, ComputedField, ComputedFieldCollection, ConcatComputed, FnComputedField,
    get_bool_field, get_float_field, get_int_field, get_string_field,
};
pub use errors::{ErrorType, ValidationContext, ValidationError, ValidationErrors, ValidationResult};
pub use types::{TypeDescriptor, Value};
pub use validators::{validate, validate_value, validate_with_context};

// Re-export Python bindings
#[cfg(feature = "python")]
pub use python::{py_dict_to_type_descriptor, py_value_to_rust_value, validate_py};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
