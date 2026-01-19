//! Python bindings for ouroboros-validation
//!
//! This module provides PyO3 bindings to expose validation functionality to Python.
//! Enable with the "python" feature.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyNone, PyString};

use crate::{
    constraints::{FieldDescriptor, ListConstraints, NumericConstraints, StringConstraints, StringFormat},
    types::{TypeDescriptor, Value},
    validators::validate,
    ValidationErrors,
};

// ============================================================================
// Python Value → Rust Value Conversion
// ============================================================================

/// Convert Python value to Rust Value enum
///
/// This function handles the conversion of PyO3 `PyAny` objects to our internal
/// `Value` enum representation. It recursively handles lists and dictionaries.
pub fn py_value_to_rust_value(py_value: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Handle None
    if py_value.is_instance_of::<PyNone>() {
        return Ok(Value::Null);
    }

    // Handle bool (must check before int, as bool is subclass of int in Python)
    if let Ok(b) = py_value.downcast::<PyBool>() {
        return Ok(Value::Bool(b.is_true()));
    }

    // Handle int
    if let Ok(i) = py_value.downcast::<PyInt>() {
        let val = i.extract::<i64>()?;
        return Ok(Value::Int(val));
    }

    // Handle float
    if let Ok(f) = py_value.downcast::<PyFloat>() {
        let val = f.extract::<f64>()?;
        return Ok(Value::Float(val));
    }

    // Handle string
    if let Ok(s) = py_value.downcast::<PyString>() {
        let val = s.extract::<String>()?;
        return Ok(Value::String(val));
    }

    // Handle bytes
    if let Ok(b) = py_value.downcast::<PyBytes>() {
        let val = b.as_bytes().to_vec();
        return Ok(Value::Bytes(val));
    }

    // Handle list (recursive)
    if let Ok(list) = py_value.downcast::<PyList>() {
        let mut rust_list = Vec::new();
        for item in list.iter() {
            rust_list.push(py_value_to_rust_value(&item)?);
        }
        return Ok(Value::List(rust_list));
    }

    // Handle dict (recursive)
    if let Ok(dict) = py_value.downcast::<PyDict>() {
        let mut rust_obj = Vec::new();
        for (key, value) in dict.iter() {
            let key_str = key.downcast::<PyString>()?.extract::<String>()?;
            let rust_value = py_value_to_rust_value(&value)?;
            rust_obj.push((key_str, rust_value));
        }
        return Ok(Value::Object(rust_obj));
    }

    // Unsupported type
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Unsupported Python type for validation: {}",
        py_value.get_type().name()?
    )))
}

// ============================================================================
// Python Dict → TypeDescriptor Conversion
// ============================================================================

/// Convert Python dict to TypeDescriptor
///
/// Expected dict format:
/// ```python
/// {
///     "type": "string",  # or "int64", "float64", "bool", "email", etc.
///     "constraints": {   # optional
///         "min_length": 3,
///         "max_length": 100,
///         "pattern": "^[a-z]+$"
///     }
/// }
/// ```
pub fn py_dict_to_type_descriptor(py_dict: &Bound<'_, PyDict>) -> PyResult<TypeDescriptor> {
    // Get the "type" field
    let type_str = py_dict
        .get_item("type")?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'type' key in type descriptor"))?
        .downcast::<PyString>()?
        .extract::<String>()?;

    // Parse based on type string
    match type_str.as_str() {
        // Primitives
        "string" => {
            let constraints = parse_string_constraints(py_dict)?;
            Ok(TypeDescriptor::String(constraints))
        }
        "int64" | "integer" => {
            let constraints = parse_numeric_constraints_i64(py_dict)?;
            Ok(TypeDescriptor::Int64(constraints))
        }
        "float64" | "float" | "number" => {
            let constraints = parse_numeric_constraints_f64(py_dict)?;
            Ok(TypeDescriptor::Float64(constraints))
        }
        "bool" | "boolean" => Ok(TypeDescriptor::Bool),
        "null" => Ok(TypeDescriptor::Null),
        "bytes" => Ok(TypeDescriptor::Bytes),

        // Collections
        "list" | "array" => {
            let items_value = py_dict
                .get_item("items")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'items' for list type"))?;
            let items_dict = items_value.downcast::<PyDict>()?;
            let items = Box::new(py_dict_to_type_descriptor(items_dict)?);
            let constraints = parse_list_constraints(py_dict)?;
            Ok(TypeDescriptor::List { items, constraints })
        }
        "tuple" => {
            let items_value = py_dict
                .get_item("items")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'items' for tuple type"))?;
            let items_list = items_value.downcast::<PyList>()?;
            let mut items = Vec::new();
            for item_dict in items_list.iter() {
                let item_dict = item_dict.downcast::<PyDict>()?;
                items.push(py_dict_to_type_descriptor(item_dict)?);
            }
            Ok(TypeDescriptor::Tuple { items })
        }
        "set" => {
            let items_value = py_dict
                .get_item("items")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'items' for set type"))?;
            let items_dict = items_value.downcast::<PyDict>()?;
            let items = Box::new(py_dict_to_type_descriptor(items_dict)?);
            Ok(TypeDescriptor::Set { items })
        }
        "object" | "dict" => {
            let fields = if let Some(fields_list) = py_dict.get_item("fields")? {
                let fields_list = fields_list.downcast::<PyList>()?;
                let mut fields = Vec::new();
                for field_dict in fields_list.iter() {
                    let field_dict = field_dict.downcast::<PyDict>()?;
                    fields.push(parse_field_descriptor(field_dict)?);
                }
                fields
            } else {
                Vec::new()
            };

            let additional = if let Some(additional_dict) = py_dict.get_item("additional")? {
                let additional_dict = additional_dict.downcast::<PyDict>()?;
                Some(Box::new(py_dict_to_type_descriptor(additional_dict)?))
            } else {
                None
            };

            Ok(TypeDescriptor::Object { fields, additional })
        }

        // Special types
        "optional" => {
            let inner_value = py_dict
                .get_item("inner")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'inner' for optional type"))?;
            let inner_dict = inner_value.downcast::<PyDict>()?;
            let inner = Box::new(py_dict_to_type_descriptor(inner_dict)?);
            Ok(TypeDescriptor::Optional(inner))
        }
        "union" => {
            let variants_value = py_dict
                .get_item("variants")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'variants' for union type"))?;
            let variants_list = variants_value.downcast::<PyList>()?;
            let mut variants = Vec::new();
            for variant_dict in variants_list.iter() {
                let variant_dict = variant_dict.downcast::<PyDict>()?;
                variants.push(py_dict_to_type_descriptor(variant_dict)?);
            }
            let nullable = py_dict
                .get_item("nullable")?
                .and_then(|v| v.extract::<bool>().ok())
                .unwrap_or(false);
            Ok(TypeDescriptor::Union { variants, nullable })
        }
        "enum" => {
            let values_value = py_dict
                .get_item("values")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'values' for enum type"))?;
            let values_list = values_value.downcast::<PyList>()?;
            let mut values = Vec::new();
            for value in values_list.iter() {
                values.push(py_value_to_rust_value(&value)?);
            }
            Ok(TypeDescriptor::Enum { values })
        }
        "literal" => {
            let values_value = py_dict
                .get_item("values")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'values' for literal type"))?;
            let values_list = values_value.downcast::<PyList>()?;
            let mut values = Vec::new();
            for value in values_list.iter() {
                values.push(py_value_to_rust_value(&value)?);
            }
            Ok(TypeDescriptor::Literal { values })
        }

        // Format types
        "email" => Ok(TypeDescriptor::Email),
        "url" => Ok(TypeDescriptor::Url),
        "uuid" => Ok(TypeDescriptor::Uuid),
        "datetime" => Ok(TypeDescriptor::DateTime),
        "date" => Ok(TypeDescriptor::Date),
        "time" => Ok(TypeDescriptor::Time),
        "decimal" => {
            let constraints = parse_numeric_constraints_f64(py_dict)?;
            Ok(TypeDescriptor::Decimal(constraints))
        }

        // BSON types (if feature enabled)
        #[cfg(feature = "bson")]
        "objectid" => Ok(TypeDescriptor::ObjectId),
        #[cfg(feature = "bson")]
        "bson_datetime" => Ok(TypeDescriptor::BsonDateTime),
        #[cfg(feature = "bson")]
        "bson_decimal128" => Ok(TypeDescriptor::BsonDecimal128),
        #[cfg(feature = "bson")]
        "bson_binary" => Ok(TypeDescriptor::BsonBinary),

        // Any type
        "any" => Ok(TypeDescriptor::Any),

        // Unknown type
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown type descriptor: {}",
            type_str
        ))),
    }
}

// ============================================================================
// Helper Functions for Parsing Constraints
// ============================================================================

fn parse_string_constraints(py_dict: &Bound<'_, PyDict>) -> PyResult<StringConstraints> {
    let constraints_dict = py_dict.get_item("constraints")?;

    let mut constraints = StringConstraints::default();

    if let Some(constraints_dict) = constraints_dict {
        let constraints_dict = constraints_dict.downcast::<PyDict>()?;

        if let Some(min_length) = constraints_dict.get_item("min_length")? {
            constraints.min_length = Some(min_length.extract::<usize>()?);
        }

        if let Some(max_length) = constraints_dict.get_item("max_length")? {
            constraints.max_length = Some(max_length.extract::<usize>()?);
        }

        if let Some(pattern) = constraints_dict.get_item("pattern")? {
            constraints.pattern = Some(pattern.extract::<String>()?);
        }

        if let Some(format) = constraints_dict.get_item("format")? {
            let format_str = format.extract::<String>()?;
            constraints.format = Some(match format_str.as_str() {
                "email" => StringFormat::Email,
                "url" => StringFormat::Url,
                "uuid" => StringFormat::Uuid,
                "datetime" => StringFormat::DateTime,
                "date" => StringFormat::Date,
                "time" => StringFormat::Time,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown string format: {}",
                        format_str
                    )))
                }
            });
        }
    }

    Ok(constraints)
}

fn parse_numeric_constraints_i64(py_dict: &Bound<'_, PyDict>) -> PyResult<NumericConstraints<i64>> {
    let constraints_dict = py_dict.get_item("constraints")?;

    let mut constraints = NumericConstraints::default();

    if let Some(constraints_dict) = constraints_dict {
        let constraints_dict = constraints_dict.downcast::<PyDict>()?;

        if let Some(minimum) = constraints_dict.get_item("minimum")? {
            constraints.minimum = Some(minimum.extract::<i64>()?);
        }

        if let Some(maximum) = constraints_dict.get_item("maximum")? {
            constraints.maximum = Some(maximum.extract::<i64>()?);
        }

        if let Some(exclusive_minimum) = constraints_dict.get_item("exclusive_minimum")? {
            constraints.exclusive_minimum = Some(exclusive_minimum.extract::<i64>()?);
        }

        if let Some(exclusive_maximum) = constraints_dict.get_item("exclusive_maximum")? {
            constraints.exclusive_maximum = Some(exclusive_maximum.extract::<i64>()?);
        }

        if let Some(multiple_of) = constraints_dict.get_item("multiple_of")? {
            constraints.multiple_of = Some(multiple_of.extract::<i64>()?);
        }
    }

    Ok(constraints)
}

fn parse_numeric_constraints_f64(py_dict: &Bound<'_, PyDict>) -> PyResult<NumericConstraints<f64>> {
    let constraints_dict = py_dict.get_item("constraints")?;

    let mut constraints = NumericConstraints::default();

    if let Some(constraints_dict) = constraints_dict {
        let constraints_dict = constraints_dict.downcast::<PyDict>()?;

        if let Some(minimum) = constraints_dict.get_item("minimum")? {
            constraints.minimum = Some(minimum.extract::<f64>()?);
        }

        if let Some(maximum) = constraints_dict.get_item("maximum")? {
            constraints.maximum = Some(maximum.extract::<f64>()?);
        }

        if let Some(exclusive_minimum) = constraints_dict.get_item("exclusive_minimum")? {
            constraints.exclusive_minimum = Some(exclusive_minimum.extract::<f64>()?);
        }

        if let Some(exclusive_maximum) = constraints_dict.get_item("exclusive_maximum")? {
            constraints.exclusive_maximum = Some(exclusive_maximum.extract::<f64>()?);
        }

        if let Some(multiple_of) = constraints_dict.get_item("multiple_of")? {
            constraints.multiple_of = Some(multiple_of.extract::<f64>()?);
        }
    }

    Ok(constraints)
}

fn parse_list_constraints(py_dict: &Bound<'_, PyDict>) -> PyResult<ListConstraints> {
    let constraints_dict = py_dict.get_item("constraints")?;

    let mut constraints = ListConstraints::default();

    if let Some(constraints_dict) = constraints_dict {
        let constraints_dict = constraints_dict.downcast::<PyDict>()?;

        if let Some(min_items) = constraints_dict.get_item("min_items")? {
            constraints.min_items = Some(min_items.extract::<usize>()?);
        }

        if let Some(max_items) = constraints_dict.get_item("max_items")? {
            constraints.max_items = Some(max_items.extract::<usize>()?);
        }

        if let Some(unique_items) = constraints_dict.get_item("unique_items")? {
            constraints.unique_items = unique_items.extract::<bool>()?;
        }
    }

    Ok(constraints)
}

fn parse_field_descriptor(py_dict: &Bound<'_, PyDict>) -> PyResult<FieldDescriptor> {
    let name = py_dict
        .get_item("name")?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'name' in field descriptor"))?
        .extract::<String>()?;

    let type_value = py_dict
        .get_item("type")?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("Missing 'type' in field descriptor"))?;
    let type_dict = type_value.downcast::<PyDict>()?;
    let field_type = py_dict_to_type_descriptor(type_dict)?;

    let required = py_dict
        .get_item("required")?
        .and_then(|v| v.extract::<bool>().ok())
        .unwrap_or(true);

    let default_value = if let Some(default) = py_dict.get_item("default")? {
        Some(py_value_to_rust_value(&default)?)
    } else {
        None
    };

    Ok(FieldDescriptor {
        name,
        type_desc: field_type,
        required,
        default: default_value,
        description: None,
    })
}

// ============================================================================
// Main Validation Function
// ============================================================================

/// Validate a Python value against a type descriptor
///
/// This is the main entry point for Python validation. It converts Python
/// values to Rust types, performs validation, and converts errors back to Python.
///
/// # Arguments
///
/// * `py` - Python interpreter state
/// * `value` - Python value to validate
/// * `type_desc_dict` - Python dict describing the expected type
///
/// # Returns
///
/// * `Ok(())` if validation succeeds
/// * `Err(PyValueError)` if validation fails, with detailed error message
///
/// # Example (Python)
///
/// ```python
/// from ouroboros.validation import validate
///
/// type_desc = {"type": "email"}
/// validate("user@example.com", type_desc)  # OK
/// validate("invalid", type_desc)           # Raises ValueError
/// ```
#[pyfunction]
pub fn validate_py(
    _py: Python<'_>,
    value: &Bound<'_, PyAny>,
    type_desc_dict: &Bound<'_, PyDict>,
) -> PyResult<()> {
    // Convert Python value to Rust Value
    let rust_value = py_value_to_rust_value(value)?;

    // Convert Python dict to TypeDescriptor
    let type_desc = py_dict_to_type_descriptor(type_desc_dict)?;

    // Perform validation (GIL-free!)
    match validate(&rust_value, &type_desc) {
        Ok(()) => Ok(()),
        Err(errors) => {
            // Convert validation errors to Python exception
            let error_message = format_validation_errors(&errors);
            Err(pyo3::exceptions::PyValueError::new_err(error_message))
        }
    }
}

/// Format validation errors for Python display
fn format_validation_errors(errors: &ValidationErrors) -> String {
    let error_list: Vec<String> = errors
        .errors
        .iter()
        .map(|e| {
            if e.location.is_empty() {
                format!("- {}", e.message)
            } else {
                format!("- {}: {}", e.location, e.message)
            }
        })
        .collect();

    if error_list.len() == 1 {
        error_list[0].trim_start_matches("- ").to_string()
    } else {
        format!("Validation failed:\n{}", error_list.join("\n"))
    }
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register Python module functions
///
/// This should be called from the parent module's registration function.
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_py, m)?)?;
    Ok(())
}
