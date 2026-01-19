//! Validation module - wrapper for ouroboros-validation PyO3 bindings
//!
//! This module exposes the ouroboros-validation library's Python bindings
//! to Python code. It provides Pydantic-like validation with Rust performance.
//!
//! # Example (Python)
//!
//! ```python
//! from ouroboros.validation import validate
//!
//! # Define type descriptor
//! type_desc = {
//!     "type": "string",
//!     "constraints": {
//!         "min_length": 3,
//!         "max_length": 100
//!     }
//! }
//!
//! # Validate value
//! validate("hello", type_desc)  # OK
//! validate("hi", type_desc)     # Raises ValueError: String too short
//! ```

use pyo3::prelude::*;

/// Validate a Python value against a type descriptor
///
/// This function validates Python values using the ouroboros-validation library.
/// It provides Pydantic-like validation with Rust performance.
///
/// # Arguments
///
/// * `value` - The Python value to validate
/// * `type_desc` - A dictionary describing the expected type and constraints
///
/// # Returns
///
/// * `Ok(())` if validation succeeds
/// * `Err(PyValueError)` if validation fails with detailed error message
///
/// # Type Descriptors
///
/// Type descriptors are dictionaries with a "type" key and optional "constraints":
///
/// ## Primitive Types
/// - `{"type": "string"}` - String type
/// - `{"type": "int64"}` or `{"type": "integer"}` - 64-bit integer
/// - `{"type": "float64"}` or `{"type": "number"}` - 64-bit float
/// - `{"type": "bool"}` - Boolean
/// - `{"type": "null"}` - Null/None
/// - `{"type": "bytes"}` - Binary data
///
/// ## String Constraints
/// ```python
/// {
///     "type": "string",
///     "constraints": {
///         "min_length": 3,      # Minimum length (optional)
///         "max_length": 100,    # Maximum length (optional)
///         "pattern": "^[a-z]+$" # Regex pattern (optional)
///     }
/// }
/// ```
///
/// ## Numeric Constraints
/// ```python
/// {
///     "type": "int64",
///     "constraints": {
///         "minimum": 0,              # Inclusive minimum (optional)
///         "maximum": 100,            # Inclusive maximum (optional)
///         "exclusive_minimum": -1,   # Exclusive minimum (optional)
///         "exclusive_maximum": 101,  # Exclusive maximum (optional)
///         "multiple_of": 5           # Must be multiple of (optional)
///     }
/// }
/// ```
///
/// ## Format Types (validated strings)
/// - `{"type": "email"}` - Email address format
/// - `{"type": "url"}` - HTTP/HTTPS URL format
/// - `{"type": "uuid"}` - UUID v4 format
/// - `{"type": "datetime"}` - ISO 8601 datetime
/// - `{"type": "date"}` - Date in YYYY-MM-DD format
/// - `{"type": "time"}` - Time in HH:MM:SS format
///
/// ## Collection Types
/// ```python
/// # List
/// {
///     "type": "list",
///     "items": {"type": "string"},
///     "constraints": {
///         "min_items": 1,     # Minimum items (optional)
///         "max_items": 10,    # Maximum items (optional)
///         "unique_items": True # Items must be unique (optional)
///     }
/// }
///
/// # Tuple (fixed-length ordered)
/// {
///     "type": "tuple",
///     "items": [
///         {"type": "string"},
///         {"type": "int64"}
///     ]
/// }
///
/// # Set (unique items only)
/// {
///     "type": "set",
///     "items": {"type": "string"}
/// }
///
/// # Object/Dict
/// {
///     "type": "object",
///     "fields": [
///         {
///             "name": "email",
///             "type": {"type": "email"},
///             "required": True
///         },
///         {
///             "name": "age",
///             "type": {"type": "int64"},
///             "required": False,
///             "default": 0
///         }
///     ],
///     "additional": None  # Type for additional properties (optional)
/// }
/// ```
///
/// ## Special Types
/// ```python
/// # Optional (nullable)
/// {
///     "type": "optional",
///     "inner": {"type": "string"}
/// }
///
/// # Union
/// {
///     "type": "union",
///     "variants": [
///         {"type": "string"},
///         {"type": "int64"}
///     ],
///     "nullable": False
/// }
///
/// # Enum
/// {
///     "type": "enum",
///     "values": ["red", "green", "blue"]
/// }
///
/// # Literal
/// {
///     "type": "literal",
///     "values": [42, "exact"]
/// }
/// ```
///
/// ## Any Type
/// - `{"type": "any"}` - No validation (accepts any value)
#[pyfunction]
pub fn validate(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    type_desc: &Bound<'_, pyo3::types::PyDict>,
) -> PyResult<()> {
    // Delegate to ouroboros-validation's Python bindings
    ouroboros_validation::validate_py(py, value, type_desc)
}

/// Register the validation module functions
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    Ok(())
}
