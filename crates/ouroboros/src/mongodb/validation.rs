//! Python bindings for MongoDB validation
//!
//! This module provides PyO3 wrappers around the pure Rust validation logic
//! from ouroboros-mongodb.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::collections::HashMap;

// Re-export the pure Rust types from ouroboros-mongodb
pub use ouroboros_mongodb::validation::{
    ValidatedCollectionName, ValidatedFieldName, ObjectIdParser,
    validate_query, BsonConstraints, BsonTypeDescriptor,
    validate_field, validate_document,
};

// =====================
// Python Helper Functions
// =====================

/// Create BsonConstraints from a Python dictionary
///
/// Expected format from Python:
/// - {'min_length': 3, 'max_length': 50}
/// - {'min': 0, 'max': 100}
/// - {'format': 'email'}
pub fn bson_constraints_from_py_dict(dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<BsonConstraints> {
    use ouroboros_validation::{StringConstraints, NumericConstraints, StringFormat};

    let mut bson_constraints = BsonConstraints::default();

    // Check for string constraints
    let min_length = dict.get_item("min_length")?.and_then(|v| v.extract().ok());
    let max_length = dict.get_item("max_length")?.and_then(|v| v.extract().ok());
    let format_str: Option<String> = dict.get_item("format")?.and_then(|v| v.extract().ok());

    if min_length.is_some() || max_length.is_some() || format_str.is_some() {
        let format = format_str.and_then(|f| match f.as_str() {
            "email" => Some(StringFormat::Email),
            "url" => Some(StringFormat::Url),
            "uuid" => Some(StringFormat::Uuid),
            "datetime" => Some(StringFormat::DateTime),
            "date" => Some(StringFormat::Date),
            "time" => Some(StringFormat::Time),
            _ => None,
        });

        bson_constraints.string = Some(StringConstraints {
            min_length,
            max_length,
            pattern: None,
            format,
        });
    }

    // Check for numeric constraints
    let min = dict.get_item("min")?.and_then(|v| v.extract().ok());
    let max = dict.get_item("max")?.and_then(|v| v.extract().ok());

    if min.is_some() || max.is_some() {
        bson_constraints.numeric = Some(NumericConstraints {
            minimum: min,
            maximum: max,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        });
    }

    Ok(bson_constraints)
}

/// Create BsonTypeDescriptor from a Python dict
///
/// Expected format from Python:
/// - {'type': 'string'}
/// - {'type': 'string', 'constraints': {'min_length': 3, 'max_length': 50}}
/// - {'type': 'int64'}
/// - {'type': 'int64', 'constraints': {'min': 0, 'max': 100}}
/// - {'type': 'optional', 'inner': {'type': 'string'}}
/// - {'type': 'array', 'items': {'type': 'int64'}}
/// - {'type': 'object', 'schema': {...}}
pub fn bson_type_descriptor_from_py_dict(
    py: Python<'_>,
    dict: &Bound<'_, pyo3::types::PyDict>
) -> PyResult<BsonTypeDescriptor> {
    let type_str: String = dict
        .get_item("type")?
        .ok_or_else(|| PyValueError::new_err("Type descriptor missing 'type' key"))?
        .extract()?;

    // Parse constraints if present
    let constraints = if let Some(constraints_item) = dict.get_item("constraints")? {
        let constraints_dict = constraints_item.downcast::<pyo3::types::PyDict>()?;
        bson_constraints_from_py_dict(constraints_dict)?
    } else {
        BsonConstraints::default()
    };

    match type_str.as_str() {
        "string" => Ok(BsonTypeDescriptor::String { constraints }),
        "int64" => Ok(BsonTypeDescriptor::Int64 { constraints }),
        "double" => Ok(BsonTypeDescriptor::Double { constraints }),
        "bool" => Ok(BsonTypeDescriptor::Bool),
        "null" => Ok(BsonTypeDescriptor::Null),
        "binary" => Ok(BsonTypeDescriptor::Binary),
        "datetime" => Ok(BsonTypeDescriptor::DateTime),
        "decimal128" => Ok(BsonTypeDescriptor::Decimal128 { constraints }),
        "objectid" => Ok(BsonTypeDescriptor::ObjectId),
        "any" => Ok(BsonTypeDescriptor::Any),
        "optional" => {
            let inner_item = dict
                .get_item("inner")?
                .ok_or_else(|| PyValueError::new_err("Optional type missing 'inner' key"))?;
            let inner_dict = inner_item.downcast::<pyo3::types::PyDict>()?;
            let inner = bson_type_descriptor_from_py_dict(py, inner_dict)?;
            Ok(BsonTypeDescriptor::Optional {
                inner: Box::new(inner),
            })
        }
        "array" => {
            let items_item = dict
                .get_item("items")?
                .ok_or_else(|| PyValueError::new_err("Array type missing 'items' key"))?;
            let items_dict = items_item.downcast::<pyo3::types::PyDict>()?;
            let items = bson_type_descriptor_from_py_dict(py, items_dict)?;
            Ok(BsonTypeDescriptor::Array {
                items: Box::new(items),
            })
        }
        "object" => {
            let schema_item = dict
                .get_item("schema")?
                .ok_or_else(|| PyValueError::new_err("Object type missing 'schema' key"))?;
            let schema_dict = schema_item.downcast::<pyo3::types::PyDict>()?;

            let mut schema = HashMap::new();
            for (key, value) in schema_dict.iter() {
                let field_name: String = key.extract()?;
                let field_dict = value.downcast::<pyo3::types::PyDict>()?;
                let field_type = bson_type_descriptor_from_py_dict(py, field_dict)?;
                schema.insert(field_name, field_type);
            }

            Ok(BsonTypeDescriptor::Object { schema })
        }
        _ => Err(PyValueError::new_err(format!(
            "Unknown type descriptor: {}",
            type_str
        ))),
    }
}

// =====================
// Python Wrapper Functions
// =====================

/// Python wrapper for ValidatedCollectionName::new
pub fn py_validated_collection_name(name: &str) -> PyResult<ValidatedCollectionName> {
    ValidatedCollectionName::new(name)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Python wrapper for ValidatedFieldName::new
pub fn py_validated_field_name(name: &str, allow_operators: bool) -> PyResult<ValidatedFieldName> {
    ValidatedFieldName::new(name, allow_operators)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Python wrapper for validate_query
pub fn py_validate_query(query: &bson::Bson) -> PyResult<()> {
    validate_query(query)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Python wrapper for validate_field
pub fn py_validate_field(
    field_path: &str,
    value: &bson::Bson,
    expected: &BsonTypeDescriptor,
) -> PyResult<()> {
    validate_field(field_path, value, expected)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Python wrapper for validate_document
pub fn py_validate_document(
    data: &bson::Document,
    schema: &HashMap<String, BsonTypeDescriptor>,
) -> PyResult<()> {
    validate_document(data, schema)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}
