//! MongoDB conversion functions for Python <-> BSON.

use bson::{Bson, Document as BsonDocument, Decimal128, Binary, oid::ObjectId};
use bson::spec::BinarySubtype;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use pyo3::conversion::IntoPyObject;
use std::str::FromStr;

use crate::config::{get_config, ObjectIdConversionMode, SecurityConfig};

use super::types::ExtractedValue;

/// Extract Python value to intermediate representation (call with GIL held)
pub(super) fn extract_py_value(py: Python<'_>, value: &Bound<'_, PyAny>, config: &SecurityConfig) -> PyResult<ExtractedValue> {
    // None
    if value.is_none() {
        return Ok(ExtractedValue::Null);
    }

    // Boolean (must check before int since bool is subclass of int in Python)
    if value.is_instance_of::<pyo3::types::PyBool>() {
        return Ok(ExtractedValue::Bool(value.extract::<bool>()?));
    }

    // Bytes
    if let Ok(bytes) = value.downcast::<PyBytes>() {
        return Ok(ExtractedValue::Bytes(bytes.as_bytes().to_vec()));
    }

    // Get type name for checking special types
    let type_name = value.get_type().name().map(|s| s.to_string()).unwrap_or_default();

    // DateTime
    if type_name == "datetime" {
        if let Ok(timestamp_method) = value.getattr("timestamp") {
            if let Ok(timestamp_result) = timestamp_method.call0() {
                if let Ok(timestamp) = timestamp_result.extract::<f64>() {
                    let millis = (timestamp * 1000.0) as i64;
                    return Ok(ExtractedValue::DateTimeMillis(millis));
                }
            }
        }
        return Err(PyValueError::new_err("Failed to convert datetime to timestamp"));
    }

    // Date
    if type_name == "date" {
        // Convert date to datetime at midnight UTC
        if let Ok(datetime_module) = py.import("datetime") {
            if let Ok(datetime_class) = datetime_module.getattr("datetime") {
                if let Ok(year) = value.getattr("year") {
                    if let Ok(month) = value.getattr("month") {
                    if let Ok(day) = value.getattr("day") {
                        if let Ok(dt) = datetime_class.call1((year, month, day)) {
                            if let Ok(timestamp_method) = dt.getattr("timestamp") {
                                if let Ok(ts_result) = timestamp_method.call0() {
                                    if let Ok(timestamp) = ts_result.extract::<f64>() {
                                        let millis = (timestamp * 1000.0) as i64;
                                        return Ok(ExtractedValue::DateTimeMillis(millis));
                                    }
                                }
                            }
                        }
                    }
                    }
                }
            }
        }
        return Err(PyValueError::new_err("Failed to convert date to datetime"));
    }

    // Decimal
    if type_name == "Decimal" {
        if let Ok(s) = value.str() {
            return Ok(ExtractedValue::Decimal(s.to_string()));
        }
        return Err(PyValueError::new_err("Failed to convert Decimal to string"));
    }

    // ObjectId wrapper
    if type_name == "PydanticObjectId" || type_name == "ObjectId" {
        if let Ok(s) = value.str() {
            return Ok(ExtractedValue::ObjectIdString(s.to_string()));
        }
        return Err(PyValueError::new_err("Failed to convert ObjectId to string"));
    }

    // Integer (try i32 first, then i64)
    if let Ok(i) = value.extract::<i32>() {
        return Ok(ExtractedValue::Int32(i));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(ExtractedValue::Int64(i));
    }

    // Float
    if let Ok(f) = value.extract::<f64>() {
        return Ok(ExtractedValue::Double(f));
    }

    // String
    if let Ok(s) = value.extract::<String>() {
        // Check if should convert to ObjectId
        let should_convert = match config.objectid_mode {
            ObjectIdConversionMode::Lenient => {
                s.len() == 24 && s.chars().all(|c| c.is_ascii_hexdigit())
            }
            ObjectIdConversionMode::TypeHinted | ObjectIdConversionMode::Strict => false,
        };

        if should_convert {
            return Ok(ExtractedValue::ObjectIdString(s));
        }
        return Ok(ExtractedValue::String(s));
    }

    // Dict
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut doc = Vec::with_capacity(dict.len());
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let extracted = extract_py_value(py, &value, config)?;
            doc.push((key, extracted));
        }
        return Ok(ExtractedValue::Document(doc));
    }

    // List
    if let Ok(list) = value.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(extract_py_value(py, &item, config)?);
        }
        return Ok(ExtractedValue::Array(arr));
    }

    // Fallback: try to convert to string
    if let Ok(s) = value.str() {
        return Ok(ExtractedValue::String(s.to_string()));
    }

    Err(PyValueError::new_err(format!(
        "Unsupported type for BSON conversion: {:?}",
        type_name
    )))
}

/// Extract dict fields to intermediate representation
pub(super) fn extract_dict_fields(py: Python<'_>, dict: &Bound<'_, PyDict>, config: &SecurityConfig) -> PyResult<Vec<(String, ExtractedValue)>> {
    let mut fields = Vec::with_capacity(dict.len());
    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        let extracted = extract_py_value(py, &value, config)?;
        fields.push((key, extracted));
    }
    Ok(fields)
}

/// Convert Python dict to BSON document
pub(super) fn py_dict_to_bson(py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<BsonDocument> {
    // Get config once per document instead of per-field
    let config = get_config();
    let mut doc = BsonDocument::new();

    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        let bson_value = py_to_bson(py, &value, &config)?;
        doc.insert(key, bson_value);
    }

    Ok(doc)
}

/// Convert Python value to BSON value
///
/// # Arguments
/// * `py` - Python interpreter handle
/// * `value` - Python value to convert
/// * `config` - Security configuration (passed to avoid repeated RwLock acquisition)
pub(super) fn py_to_bson(py: Python<'_>, value: &Bound<'_, PyAny>, config: &SecurityConfig) -> PyResult<Bson> {
    // None
    if value.is_none() {
        return Ok(Bson::Null);
    }

    // Boolean (must check before int since bool is subclass of int in Python)
    if let Ok(b) = value.extract::<bool>() {
        // Check if it's actually a bool, not just truthy int
        if value.is_instance_of::<pyo3::types::PyBool>() {
            return Ok(Bson::Boolean(b));
        }
    }

    // Bytes -> Binary
    if let Ok(bytes) = value.downcast::<PyBytes>() {
        let data = bytes.as_bytes().to_vec();
        return Ok(Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: data,
        }));
    }

    // Get type name for checking special types
    let type_name = value.get_type().name().map(|s| s.to_string()).unwrap_or_default();

    // DateTime (check by type name since PyDateTime not available in abi3)
    if type_name == "datetime" {
        // Get timestamp from datetime
        if let Ok(timestamp) = value.call_method0("timestamp") {
            if let Ok(ts) = timestamp.extract::<f64>() {
                let millis = (ts * 1000.0) as i64;
                return Ok(Bson::DateTime(bson::DateTime::from_millis(millis)));
            }
        }
    }

    // Check for date (without time)
    if type_name == "date" {
        // Convert date to datetime at midnight UTC
        let datetime_mod = py.import("datetime")?;
        let datetime_cls = datetime_mod.getattr("datetime")?;
        let combined = datetime_cls.call_method1("combine", (value, datetime_mod.getattr("time")?.call0()?))?;
        if let Ok(timestamp) = combined.call_method0("timestamp") {
            if let Ok(ts) = timestamp.extract::<f64>() {
                let millis = (ts * 1000.0) as i64;
                return Ok(Bson::DateTime(bson::DateTime::from_millis(millis)));
            }
        }
    }
    // Check for Decimal type (Python's decimal.Decimal)
    if type_name == "Decimal" {
        if let Ok(s) = value.str() {
            let decimal_str = s.to_string();
            if let Ok(dec) = Decimal128::from_str(&decimal_str) {
                return Ok(Bson::Decimal128(dec));
            }
        }
    }

    // Check for ObjectId wrapper class
    if type_name == "ObjectId" || type_name == "PydanticObjectId" {
        if let Ok(s) = value.str() {
            let oid_str = s.to_string();
            if let Ok(oid) = ObjectId::parse_str(&oid_str) {
                return Ok(Bson::ObjectId(oid));
            }
        }
    }

    // Integer
    if let Ok(i) = value.extract::<i64>() {
        return Ok(Bson::Int64(i));
    }

    // Float
    if let Ok(f) = value.extract::<f64>() {
        return Ok(Bson::Double(f));
    }

    // String
    if let Ok(s) = value.extract::<String>() {
        // Security: Use type-aware ObjectId conversion to prevent NoSQL injection
        // Note: config is passed as parameter to avoid repeated RwLock acquisition

        let should_convert = match config.objectid_mode {
            ObjectIdConversionMode::Lenient => {
                // Backward compatible: auto-convert 24-char hex strings
                // DEPRECATED: Will be removed in v2.0
                s.len() == 24 && s.chars().all(|c| c.is_ascii_hexdigit())
            }
            ObjectIdConversionMode::TypeHinted => {
                // Secure: Only convert if type_name indicates ObjectId
                // This relies on explicit wrapper classes (already handled above at lines 274-282)
                // Plain strings with 24 hex chars will NOT be auto-converted
                false
            }
            ObjectIdConversionMode::Strict => {
                // Most secure: Never auto-convert, require explicit wrapper
                false
            }
        };

        if should_convert {
            if let Ok(oid) = ObjectId::parse_str(&s) {
                // Emit deprecation warning for Lenient mode
                if config.objectid_mode == ObjectIdConversionMode::Lenient {
                    eprintln!(
                        "WARNING: Auto-converting string '{}...' to ObjectId. \
                        This behavior is deprecated and will be removed in v2.0. \
                        Use explicit ObjectId wrapper or set ObjectIdConversionMode.TYPE_HINTED.",
                        &s[..8]
                    );
                }
                return Ok(Bson::ObjectId(oid));
            }
        }
        return Ok(Bson::String(s));
    }

    // Dict
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut doc = BsonDocument::new();
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let bson_value = py_to_bson(py, &value, config)?;
            doc.insert(key, bson_value);
        }
        return Ok(Bson::Document(doc));
    }

    // List
    if let Ok(list) = value.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_to_bson(py, &item, config)?);
        }
        return Ok(Bson::Array(arr));
    }

    // Fallback: try to convert to string
    if let Ok(s) = value.str() {
        return Ok(Bson::String(s.to_string()));
    }

    Err(PyValueError::new_err(format!(
        "Cannot convert Python type to BSON: {:?}",
        value.get_type().name()
    )))
}

/// Convert BSON value to Python value
pub(super) fn bson_to_py(py: Python<'_>, bson: &Bson) -> PyResult<PyObject> {
    match bson {
        Bson::Null => Ok(py.None()),
        Bson::Boolean(b) => {
            let obj = (*b).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Int32(i) => {
            let obj = (*i).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Int64(i) => {
            let obj = (*i).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Double(f) => {
            let obj = (*f).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::String(s) => {
            let obj = s.as_str().into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::ObjectId(oid) => {
            let obj = oid.to_hex().into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Document(doc) => {
            let dict = PyDict::new(py);
            for (key, value) in doc.iter() {
                dict.set_item(key, bson_to_py(py, value)?)?;
            }
            Ok(dict.into())
        }
        Bson::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(bson_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        Bson::DateTime(dt) => {
            // Convert to Python datetime
            let datetime_mod = py.import("datetime")?;
            let datetime_cls = datetime_mod.getattr("datetime")?;
            // Get milliseconds and convert to datetime
            let millis = dt.timestamp_millis();
            let secs = millis / 1000;
            let micros = ((millis % 1000) * 1000) as u32;
            let utc = datetime_mod.getattr("timezone")?.getattr("utc")?;
            let dt_obj = datetime_cls.call_method1("fromtimestamp", (secs as f64 + micros as f64 / 1_000_000.0, utc))?;
            Ok(dt_obj.into())
        }
        Bson::Binary(binary) => {
            // Convert to Python bytes
            Ok(PyBytes::new(py, &binary.bytes).into())
        }
        Bson::Decimal128(dec) => {
            // Convert to Python Decimal
            let decimal_mod = py.import("decimal")?;
            let decimal_cls = decimal_mod.getattr("Decimal")?;
            let dec_str = dec.to_string();
            let dec_obj = decimal_cls.call1((dec_str,))?;
            Ok(dec_obj.into())
        }
        Bson::RegularExpression(regex) => {
            // Return as dict with pattern and options
            let dict = PyDict::new(py);
            dict.set_item("$regex", &regex.pattern)?;
            dict.set_item("$options", &regex.options)?;
            Ok(dict.into())
        }
        Bson::Timestamp(ts) => {
            // Return as dict with time and increment
            let dict = PyDict::new(py);
            dict.set_item("t", ts.time)?;
            dict.set_item("i", ts.increment)?;
            Ok(dict.into())
        }
        _ => {
            // For other types, convert to string representation
            let obj = bson.to_string().into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
    }
}

/// Convert BSON document to Python dict
pub(super) fn bson_doc_to_py_dict(py: Python<'_>, doc: &BsonDocument) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    for (key, value) in doc.iter() {
        dict.set_item(key, bson_to_py(py, value)?)?;
    }
    Ok(dict.into())
}

/// Convert raw BSON value directly to Python (skip intermediate representation)
pub(super) fn raw_bson_to_py(py: Python<'_>, raw_bson: bson::raw::RawBsonRef<'_>) -> PyResult<PyObject> {
    use bson::raw::RawBsonRef;

    match raw_bson {
        RawBsonRef::Double(f) => Ok(f.into_pyobject(py)?.to_owned().into_any().unbind()),
        RawBsonRef::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        RawBsonRef::Document(doc) => {
            let py_dict = PyDict::new(py);
            for element in doc.iter_elements().flatten() {
                let key = element.key();
                if let Ok(value) = element.value() {
                    py_dict.set_item(key, raw_bson_to_py(py, value)?)?;
                }
            }
            Ok(py_dict.into())
        }
        RawBsonRef::Array(arr) => {
            let py_list = PyList::empty(py);
            for value in arr.into_iter().flatten() {
                py_list.append(raw_bson_to_py(py, value)?)?;
            }
            Ok(py_list.into())
        }
        RawBsonRef::Binary(bin) => Ok(PyBytes::new(py, bin.bytes).into()),
        RawBsonRef::ObjectId(oid) => Ok(oid.to_hex().into_pyobject(py)?.into_any().unbind()),
        RawBsonRef::Boolean(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        RawBsonRef::DateTime(dt) => {
            let datetime_module = py.import("datetime")?;
            let datetime_class = datetime_module.getattr("datetime")?;
            let from_timestamp = datetime_class.getattr("fromtimestamp")?;
            let timestamp_secs = (dt.timestamp_millis() as f64) / 1000.0;
            Ok(from_timestamp.call1((timestamp_secs,))?.into())
        }
        RawBsonRef::Null => Ok(py.None()),
        RawBsonRef::Int32(i) => Ok(i.into_pyobject(py)?.to_owned().into_any().unbind()),
        RawBsonRef::Int64(i) => Ok(i.into_pyobject(py)?.to_owned().into_any().unbind()),
        RawBsonRef::Decimal128(d) => Ok(d.to_string().into_pyobject(py)?.into_any().unbind()),
        // Handle other types as strings
        _ => Ok(format!("{:?}", raw_bson).into_pyobject(py)?.into_any().unbind()),
    }
}
