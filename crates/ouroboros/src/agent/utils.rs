//! Utility functions for agent module

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

/// Convert Python value to serde_json::Value
pub fn py_to_json(py_obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if py_obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = py_obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = py_obj.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = py_obj.extract::<f64>() {
        Ok(Value::Number(
            serde_json::Number::from_f64(f)
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid float"))?,
        ))
    } else if let Ok(s) = py_obj.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(dict) = py_obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract()?;
            let json_value = py_to_json(&value)?;
            map.insert(key_str, json_value);
        }
        Ok(Value::Object(map))
    } else if let Ok(list) = py_obj.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(py_to_json(&item)?);
        }
        Ok(Value::Array(vec))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "Unsupported Python type: {:?}",
            py_obj
        )))
    }
}

/// Convert serde_json::Value to Python object
pub fn json_to_py(py: Python, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid number",
                ))
            }
        }
        Value::String(s) => Ok(s.to_object(py)),
        Value::Array(arr) => {
            let list = pyo3::types::PyList::empty(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.to_object(py))
        }
        Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (key, value) in obj {
                dict.set_item(key, json_to_py(py, value)?)?;
            }
            Ok(dict.to_object(py))
        }
    }
}
