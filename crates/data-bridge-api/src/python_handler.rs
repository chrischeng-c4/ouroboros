//! Python handler integration
//!
//! This module provides the PythonHandler struct that wraps Python callable
//! objects and integrates them with the Rust HTTP server via PyLoop.

use crate::error::{ApiError, ApiResult};
use crate::request::{Request, SerializableValue};
use crate::response::Response;
use crate::validation::ValidatedRequest;
use data_bridge_pyloop::PyLoop;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::sync::Arc;

/// Python handler that wraps a Python callable and executes it via PyLoop
///
/// This handler bridges Rust's HTTP server with Python business logic:
/// 1. Converts Rust Request → Python dict
/// 2. Spawns execution on PyLoop (handles both sync and async Python functions)
/// 3. Awaits result
/// 4. Converts Python response → Rust Response
///
/// # Example
///
/// ```python
/// # Python side
/// async def my_handler(request):
///     return {"status": "ok", "user_id": request["path_params"]["id"]}
/// ```
///
/// ```rust,no_run
/// // Rust side
/// use pyo3::prelude::*;
/// use data_bridge_api::python_handler::PythonHandler;
/// use data_bridge_pyloop::PyLoop;
/// use std::sync::Arc;
///
/// # fn main() -> PyResult<()> {
/// Python::with_gil(|py| {
///     let handler_fn = py.eval("lambda req: {'status': 'ok'}", None, None)?;
///     let pyloop = Arc::new(PyLoop::new()?);
///
///     let handler = PythonHandler::new(
///         handler_fn.into(),
///         pyloop,
///     );
///     Ok(())
/// })
/// # }
/// ```
pub struct PythonHandler {
    /// Python callable (sync or async function)
    callable: PyObject,
    /// PyLoop instance for executing the Python function
    pyloop: Arc<PyLoop>,
}

impl PythonHandler {
    /// Create a new Python handler
    ///
    /// # Arguments
    ///
    /// * `callable` - Python function (sync or async) that accepts a request dict
    /// * `pyloop` - PyLoop instance for execution
    ///
    /// # Returns
    ///
    /// A new PythonHandler instance
    pub fn new(callable: PyObject, pyloop: Arc<PyLoop>) -> Self {
        Self { callable, pyloop }
    }

    /// Execute the Python handler
    ///
    /// This is the core dispatch logic:
    /// 1. Convert Request to Python dict
    /// 2. Spawn on PyLoop
    /// 3. Await result
    /// 4. Convert to Response
    pub async fn execute(&self, req: Request, _validated: ValidatedRequest) -> ApiResult<Response> {
        // Phase 1: Extract request data (prepare for GIL)
        let req_data = req.inner;

        // Phase 2: Convert to Python and execute (GIL-held in spawn_python_handler)
        let py_request = Python::with_gil(|py| {
            // Clone callable with GIL held
            let callable_clone = self.callable.clone_ref(py);
            let py_req = convert_request_to_py(py, req_data)
                .map_err(|e| ApiError::Internal(format!("Failed to convert request to Python: {}", e)))?;
            Ok::<(PyObject, PyObject), ApiError>((callable_clone, py_req))
        })?;

        let (callable, py_req) = py_request;
        let pyloop = self.pyloop.clone();

        let result = pyloop
            .spawn_python_handler(callable, py_req)
            .await
            .map_err(|e| ApiError::Internal(format!("Python handler execution failed: {}", e)))?;

        // Phase 3: Convert Python result to Response
        Python::with_gil(|py| convert_py_to_response(py, result))
    }
}

/// Convert SerializableRequest to Python dict
///
/// Creates a dict with the following structure:
/// ```python
/// {
///     "method": "GET",
///     "path": "/api/users/123",
///     "url": "http://localhost:8000/api/users/123?page=1",
///     "headers": {"content-type": "application/json"},
///     "query_params": {"page": "1"},
///     "path_params": {"user_id": "123"},
///     "body": {...},  // Parsed JSON or None
/// }
/// ```
fn convert_request_to_py(
    py: Python<'_>,
    req: crate::request::SerializableRequest,
) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);

    // Basic request info
    dict.set_item("method", req.method.as_str())?;
    dict.set_item("path", req.path)?;
    dict.set_item("url", req.url)?;

    // Headers
    let headers_dict = PyDict::new_bound(py);
    for (key, value) in req.headers.iter() {
        headers_dict.set_item(key, value)?;
    }
    dict.set_item("headers", headers_dict)?;

    // Query parameters
    let query_dict = PyDict::new_bound(py);
    for (key, value) in req.query_params.iter() {
        query_dict.set_item(key, serializable_value_to_py(py, value)?)?;
    }
    dict.set_item("query_params", query_dict)?;

    // Path parameters
    let path_dict = PyDict::new_bound(py);
    for (key, value) in req.path_params.iter() {
        path_dict.set_item(key, value)?;
    }
    dict.set_item("path_params", path_dict)?;

    // Body (if present)
    if let Some(body) = req.body {
        dict.set_item("body", serializable_value_to_py(py, &body)?)?;
    } else {
        dict.set_item("body", py.None())?;
    }

    // Form data (if present)
    if let Some(form_data) = req.form_data {
        let form_dict = PyDict::new_bound(py);

        // Fields
        let fields_dict = PyDict::new_bound(py);
        for (key, value) in form_data.fields.iter() {
            fields_dict.set_item(key, value)?;
        }
        form_dict.set_item("fields", fields_dict)?;

        // Files (if any)
        let files_list = pyo3::types::PyList::empty_bound(py);
        for file in form_data.files.iter() {
            let file_dict = PyDict::new_bound(py);
            file_dict.set_item("field_name", &file.field_name)?;
            file_dict.set_item("filename", &file.filename)?;
            file_dict.set_item("content_type", &file.content_type)?;
            file_dict.set_item("data", pyo3::types::PyBytes::new_bound(py, &file.data))?;
            files_list.append(file_dict)?;
        }
        form_dict.set_item("files", files_list)?;

        dict.set_item("form_data", form_dict)?;
    } else {
        dict.set_item("form_data", py.None())?;
    }

    #[allow(deprecated)] // PyO3 API transition - to_object will be replaced by IntoPyObject
    Ok(dict.to_object(py))
}

/// Convert SerializableValue to Python object
fn serializable_value_to_py(py: Python<'_>, value: &SerializableValue) -> PyResult<PyObject> {
    match value {
        SerializableValue::Null => Ok(py.None()),
        SerializableValue::Bool(b) => Ok(b.to_object(py)),
        SerializableValue::Int(i) => Ok(i.to_object(py)),
        SerializableValue::Float(f) => Ok(f.to_object(py)),
        SerializableValue::String(s) => Ok(s.to_object(py)),
        SerializableValue::Bytes(b) => Ok(pyo3::types::PyBytes::new_bound(py, b).to_object(py)),
        SerializableValue::List(arr) => {
            let py_list = pyo3::types::PyList::empty_bound(py);
            for item in arr {
                py_list.append(serializable_value_to_py(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        }
        SerializableValue::Object(obj) => {
            let py_dict = PyDict::new_bound(py);
            for (key, val) in obj {
                py_dict.set_item(key, serializable_value_to_py(py, val)?)?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

/// Convert Python return value to Response
///
/// Supports several return types:
/// 1. Dict with "status", "body", "headers" keys
/// 2. Tuple (status_code, body)
/// 3. Direct value (assumes 200 OK)
fn convert_py_to_response(py: Python<'_>, result: PyObject) -> ApiResult<Response> {
    let result_ref = result.bind(py);

    // Try to interpret as dict first (most common case)
    if let Ok(dict) = result_ref.downcast::<PyDict>() {
        // Extract status code (default: 200)
        let status_code = if let Ok(status) = dict.get_item("status") {
            if let Some(status_val) = status {
                status_val
                    .extract::<u16>()
                    .unwrap_or(200)
            } else {
                200
            }
        } else {
            200
        };

        // Extract body
        let body = if let Ok(Some(body_val)) = dict.get_item("body") {
            py_to_serializable_value(py, body_val.into())
                .map_err(|e| ApiError::Internal(format!("Failed to convert body: {}", e)))?
        } else {
            SerializableValue::Null
        };

        // Create response
        let mut response = Response::json(body).status(status_code);

        // Extract headers (if present)
        if let Ok(Some(headers)) = dict.get_item("headers") {
            if let Ok(headers_dict) = headers.downcast::<PyDict>() {
                for (key, value) in headers_dict.iter() {
                    let key_str: String = key.extract()
                        .map_err(|e| ApiError::Internal(format!("Invalid header name: {}", e)))?;
                    let value_str: String = value.extract()
                        .map_err(|e| ApiError::Internal(format!("Invalid header value: {}", e)))?;
                    response = response.header(&key_str, &value_str);
                }
            }
        }

        return Ok(response);
    }

    // Try to interpret as tuple (status_code, body)
    if let Ok(tuple) = result_ref.downcast::<PyTuple>() {
        if tuple.len() == 2 {
            let status_code: u16 = tuple.get_item(0)
                .map_err(|e| ApiError::Internal(format!("Failed to get status code: {}", e)))?
                .extract()
                .map_err(|e| ApiError::Internal(format!("Invalid status code: {}", e)))?;
            let body = py_to_serializable_value(py, tuple.get_item(1)
                .map_err(|e| ApiError::Internal(format!("Failed to get body: {}", e)))?
                .into())
                .map_err(|e| ApiError::Internal(format!("Failed to convert body: {}", e)))?;
            return Ok(Response::json(body).status(status_code));
        }
    }

    // Default: treat as body with 200 OK
    let body = py_to_serializable_value(py, result)
        .map_err(|e| ApiError::Internal(format!("Failed to convert response: {}", e)))?;
    Ok(Response::json(body))
}

/// Convert Python object to SerializableValue
fn py_to_serializable_value(py: Python<'_>, obj: PyObject) -> PyResult<SerializableValue> {
    let obj_ref = obj.bind(py);

    // None → Null
    if obj_ref.is_none() {
        return Ok(SerializableValue::Null);
    }

    // Bool
    if let Ok(b) = obj_ref.extract::<bool>() {
        return Ok(SerializableValue::Bool(b));
    }

    // Int
    if let Ok(i) = obj_ref.extract::<i64>() {
        return Ok(SerializableValue::Int(i));
    }

    // Float
    if let Ok(f) = obj_ref.extract::<f64>() {
        return Ok(SerializableValue::Float(f));
    }

    // String
    if let Ok(s) = obj_ref.extract::<String>() {
        return Ok(SerializableValue::String(s));
    }

    // Bytes
    if let Ok(bytes) = obj_ref.downcast::<pyo3::types::PyBytes>() {
        return Ok(SerializableValue::Bytes(bytes.as_bytes().to_vec()));
    }

    // List
    if let Ok(list) = obj_ref.downcast::<pyo3::types::PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(py_to_serializable_value(py, item.into())?);
        }
        return Ok(SerializableValue::List(arr));
    }

    // Dict
    if let Ok(dict) = obj_ref.downcast::<PyDict>() {
        let mut map = Vec::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract()?;
            map.push((key_str, py_to_serializable_value(py, value.into())?));
        }
        return Ok(SerializableValue::Object(map));
    }

    // Fallback: convert to string
    let repr = obj_ref.str()?.extract::<String>()?;
    Ok(SerializableValue::String(repr))
}

/// Convert PythonHandler to HandlerFn for router registration
///
/// This allows using PythonHandler with the existing Router::route() API
impl PythonHandler {
    /// Convert this handler into a HandlerFn that can be registered with the router
    pub fn into_handler_fn(self) -> crate::router::HandlerFn {
        let handler = Arc::new(self);
        Arc::new(move |req, validated| {
            let handler = handler.clone();
            Box::pin(async move { handler.execute(req, validated).await })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{HttpMethod, SerializableRequest};

    #[test]
    fn test_convert_request_to_py() {
        Python::with_gil(|py| {
            let mut req = SerializableRequest::new(HttpMethod::Get, "/api/users/123");
            req.path_params.insert("user_id".to_string(), "123".to_string());
            req.query_params.insert("page".to_string(), SerializableValue::String("1".to_string()));

            let py_obj = convert_request_to_py(py, req).unwrap();
            let dict = py_obj.downcast_bound::<PyDict>(py).unwrap();

            assert_eq!(dict.get_item("method").unwrap().unwrap().extract::<String>().unwrap(), "GET");
            assert_eq!(dict.get_item("path").unwrap().unwrap().extract::<String>().unwrap(), "/api/users/123");
        });
    }

    #[test]
    fn test_py_to_serializable_value_primitives() {
        Python::with_gil(|py| {
            // None
            let result = py_to_serializable_value(py, py.None()).unwrap();
            assert!(matches!(result, SerializableValue::Null));

            // Bool
            let result = py_to_serializable_value(py, true.to_object(py)).unwrap();
            assert!(matches!(result, SerializableValue::Bool(true)));

            // Int
            let result = py_to_serializable_value(py, 42i64.to_object(py)).unwrap();
            assert!(matches!(result, SerializableValue::Int(42)));

            // String
            let result = py_to_serializable_value(py, "hello".to_object(py)).unwrap();
            assert!(matches!(result, SerializableValue::String(s) if s == "hello"));
        });
    }

    #[test]
    fn test_convert_py_to_response_dict() {
        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item("status", 201).unwrap();
            dict.set_item("body", PyDict::new_bound(py)).unwrap();

            #[allow(deprecated)]
            let response = convert_py_to_response(py, dict.to_object(py)).unwrap();
            let serializable = response.into_serializable();

            assert_eq!(serializable.status_code, 201);
        });
    }

    #[test]
    fn test_convert_py_to_response_tuple() {
        Python::with_gil(|py| {
            let tuple = PyTuple::new_bound(py, &[404.to_object(py), "Not found".to_object(py)]);

            #[allow(deprecated)]
            let response = convert_py_to_response(py, tuple.to_object(py)).unwrap();
            let serializable = response.into_serializable();

            assert_eq!(serializable.status_code, 404);
        });
    }
}
