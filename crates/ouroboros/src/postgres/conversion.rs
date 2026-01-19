//! Conversion helpers between Python and Rust types.

use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use base64::{Engine as _, engine::general_purpose};
use ouroboros_postgres::Connection;
use std::sync::Arc;

use super::PG_POOL;

/// Gets the PostgreSQL connection pool or returns an error if not initialized.
pub(super) fn get_connection() -> PyResult<Arc<Connection>> {
    PG_POOL
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to acquire pool lock: {}", e)))?
        .clone()
        .ok_or_else(|| PyRuntimeError::new_err("PostgreSQL connection not initialized. Call init() first."))
}

/// Converts Python dict to ExtractedValue for query parameters
pub(super) fn py_dict_to_extracted_values(
    py: Python<'_>,
    dict: &Bound<'_, PyDict>,
) -> PyResult<Vec<(String, ouroboros_postgres::ExtractedValue)>> {
    let mut result = Vec::new();

    for (key, value) in dict.iter() {
        let key_str = key.extract::<String>()?;
        let extracted_value = py_value_to_extracted(py, &value)?;
        result.push((key_str, extracted_value));
    }

    Ok(result)
}

/// Converts Python value to ExtractedValue
///
/// Optimized to check Python type name first, avoiding sequential type extractions.
/// This reduces overhead by jumping directly to the correct type extraction.
pub(super) fn py_value_to_extracted(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
) -> PyResult<ouroboros_postgres::ExtractedValue> {
    use ouroboros_postgres::ExtractedValue;

    // Fast path: check None first (very common)
    if value.is_none() {
        return Ok(ExtractedValue::Null);
    }

    // Get Python type name once - much faster than multiple failed extractions
    let type_name = value.get_type().name()?;

    match type_name.to_cow()?.as_ref() {
        // Most common types first
        "int" => {
            // Try i64 first (most common), then i32, then i16
            if let Ok(i) = value.extract::<i64>() {
                Ok(ExtractedValue::BigInt(i))
            } else if let Ok(i) = value.extract::<i32>() {
                Ok(ExtractedValue::Int(i))
            } else if let Ok(i) = value.extract::<i16>() {
                Ok(ExtractedValue::SmallInt(i))
            } else {
                Err(PyTypeError::new_err("Integer out of range"))
            }
        }
        "str" => {
            let s = value.extract::<String>()?;
            Ok(ExtractedValue::String(s))
        }
        "bool" => {
            let b = value.extract::<bool>()?;
            Ok(ExtractedValue::Bool(b))
        }
        "float" => {
            // Try f64 first (more common), then f32
            if let Ok(f) = value.extract::<f64>() {
                Ok(ExtractedValue::Double(f))
            } else {
                let f = value.extract::<f32>()?;
                Ok(ExtractedValue::Float(f))
            }
        }
        "bytes" | "bytearray" => {
            let bytes = value.extract::<Vec<u8>>()?;
            Ok(ExtractedValue::Bytes(bytes))
        }
        "list" | "tuple" => {
            // Handle both lists and tuples by converting to list
            let list = if let Ok(l) = value.downcast::<PyList>() {
                l.clone()
            } else if let Ok(t) = value.downcast::<PyTuple>() {
                t.to_list()
            } else {
                return Err(PyTypeError::new_err("Expected list or tuple"));
            };

            let mut vec = Vec::with_capacity(list.len());
            for item in list.iter() {
                vec.push(py_value_to_extracted(py, &item)?);
            }
            Ok(ExtractedValue::Array(vec))
        }
        "dict" => {
            let dict = value.downcast::<PyDict>()?;
            let values = py_dict_to_extracted_values(py, dict)?;
            Ok(ExtractedValue::Json(serde_json::json!(
                values.into_iter()
                    .map(|(k, v)| (k, extracted_to_json(&v)))
                    .collect::<serde_json::Map<String, serde_json::Value>>()
            )))
        }
        "NoneType" => Ok(ExtractedValue::Null),
        "datetime" => {
            // Handle datetime by converting to string representation
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        "date" => {
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        "UUID" | "uuid" => {
            let s = value.str()?.to_string();
            Ok(ExtractedValue::String(s))
        }
        "Decimal" => {
            let s = value.str()?.to_string();
            Ok(ExtractedValue::Decimal(s))
        }
        _ => {
            // Fallback: try common extractions for custom types
            if let Ok(s) = value.extract::<String>() {
                Ok(ExtractedValue::String(s))
            } else if let Ok(i) = value.extract::<i64>() {
                Ok(ExtractedValue::BigInt(i))
            } else if let Ok(f) = value.extract::<f64>() {
                Ok(ExtractedValue::Double(f))
            } else {
                // Last resort: convert to string representation
                let s = value.str()?.to_string();
                Ok(ExtractedValue::String(s))
            }
        }
    }
}

/// Helper to convert ExtractedValue to JSON for nested structures
pub(super) fn extracted_to_json(value: &ouroboros_postgres::ExtractedValue) -> serde_json::Value {
    use ouroboros_postgres::ExtractedValue;

    match value {
        ExtractedValue::Null => serde_json::Value::Null,
        ExtractedValue::Bool(b) => serde_json::Value::Bool(*b),
        ExtractedValue::SmallInt(i) => serde_json::Value::Number((*i).into()),
        ExtractedValue::Int(i) => serde_json::Value::Number((*i).into()),
        ExtractedValue::BigInt(i) => serde_json::Value::Number((*i).into()),
        ExtractedValue::Float(f) => serde_json::Number::from_f64(*f as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ExtractedValue::Double(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ExtractedValue::String(s) => serde_json::Value::String(s.clone()),
        ExtractedValue::Bytes(b) => serde_json::Value::String(general_purpose::STANDARD.encode(b)),
        ExtractedValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(extracted_to_json).collect())
        }
        ExtractedValue::Json(j) => j.clone(),
        ExtractedValue::Uuid(u) => serde_json::Value::String(u.to_string()),
        ExtractedValue::Date(d) => serde_json::Value::String(d.to_string()),
        ExtractedValue::Time(t) => serde_json::Value::String(t.to_string()),
        ExtractedValue::Timestamp(ts) => serde_json::Value::String(ts.to_string()),
        ExtractedValue::TimestampTz(ts) => serde_json::Value::String(ts.to_rfc3339()),
        ExtractedValue::Decimal(d) => serde_json::Value::String(d.clone()),
    }
}

/// Static regex for adjusting placeholders (compiled once)
pub(super) static PLACEHOLDER_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"\$(\d+)").expect("placeholder regex is valid")
});

/// Adjusts parameter placeholders in SQL to account for offset
/// Example: "age > $1 AND status = $2" with offset 3 becomes "age > $4 AND status = $5"
/// Returns error if the SQL contains malformed placeholders
pub(super) fn adjust_placeholders(sql: &str, offset: usize) -> Result<String, String> {
    let mut last_error = None;
    let result = PLACEHOLDER_RE.replace_all(sql, |caps: &regex::Captures| {
        match caps[1].parse::<usize>() {
            Ok(num) => format!("${}", num + offset),
            Err(e) => {
                last_error = Some(format!("Invalid placeholder number '{}': {}", &caps[1], e));
                caps[0].to_string() // Return original on error
            }
        }
    }).to_string();

    if let Some(err) = last_error {
        Err(err)
    } else {
        Ok(result)
    }
}

/// Converts ExtractedValue back to Python object
pub(super) fn extracted_to_py_value(py: Python<'_>, value: &ouroboros_postgres::ExtractedValue) -> PyResult<PyObject> {
    use ouroboros_postgres::ExtractedValue;

    Ok(match value {
        ExtractedValue::Null => py.None(),
        ExtractedValue::Bool(b) => b.to_object(py),
        ExtractedValue::SmallInt(i) => i.to_object(py),
        ExtractedValue::Int(i) => i.to_object(py),
        ExtractedValue::BigInt(i) => i.to_object(py),
        ExtractedValue::Float(f) => f.to_object(py),
        ExtractedValue::Double(f) => f.to_object(py),
        ExtractedValue::String(s) => s.to_object(py),
        ExtractedValue::Bytes(b) => b.to_object(py),
        ExtractedValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(extracted_to_py_value(py, item)?)?;
            }
            list.to_object(py)
        }
        ExtractedValue::Json(j) => pythonize::pythonize(py, j)?.into(),
        ExtractedValue::Uuid(u) => u.to_string().to_object(py),
        ExtractedValue::Date(d) => {
            let datetime = py.import("datetime")?;
            let date = datetime.getattr("date")?;
            date.call_method1("fromisoformat", (d.to_string(),))?.to_object(py)
        }
        ExtractedValue::Time(t) => {
            let datetime = py.import("datetime")?;
            let time = datetime.getattr("time")?;
            time.call_method1("fromisoformat", (t.to_string(),))?.to_object(py)
        }
        ExtractedValue::Timestamp(ts) => {
            // Convert NaiveDateTime to Python datetime (no timezone)
            let datetime = py.import("datetime")?;
            let dt = datetime.getattr("datetime")?;
            dt.call_method1("fromisoformat", (ts.to_string(),))?.to_object(py)
        }
        ExtractedValue::TimestampTz(ts) => {
            // Convert to Python datetime with timezone
            let datetime = py.import("datetime")?;
            let dt = datetime.getattr("datetime")?;
            dt.call_method1("fromisoformat", (ts.to_rfc3339(),))?.to_object(py)
        }
        ExtractedValue::Decimal(d) => {
            // Convert to Python Decimal
            let decimal_mod = py.import("decimal")?;
            let decimal_cls = decimal_mod.getattr("Decimal")?;
            decimal_cls.call1((d,))?.to_object(py)
        }
    })
}
