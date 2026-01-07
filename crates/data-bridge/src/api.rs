//! PyO3 bindings for data-bridge-api
//!
//! Follows the two-phase GIL pattern:
//! 1. Extract: Python objects → SerializableRequest (GIL held, fast)
//! 2. Process: Validation, routing, serialization (GIL released)
//! 3. Materialize: SerializableResponse → Python objects (GIL held, fast)

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyBool, PyBytes};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use data_bridge_api::{
    Router,
    request::{HttpMethod, SerializableValue, SerializableRequest},
    response::{SerializableResponse, ResponseBody},
    validation::{
        TypeDescriptor, StringConstraints, NumericConstraints,
        RequestValidator, ParamValidator, ParamLocation,
    },
    handler::HandlerMeta,
};

use crate::error_handling::sanitize_error_message;

// ============================================================================
// Python Value Conversion
// ============================================================================

/// Extract Python value to SerializableValue (GIL held)
fn py_to_serializable(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<SerializableValue> {
    if obj.is_none() {
        return Ok(SerializableValue::Null);
    }
    if let Ok(b) = obj.downcast::<PyBool>() {
        return Ok(SerializableValue::Bool(b.is_true()));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(SerializableValue::Int(i));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(SerializableValue::Float(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(SerializableValue::String(s));
    }
    if let Ok(bytes) = obj.downcast::<PyBytes>() {
        return Ok(SerializableValue::Bytes(bytes.as_bytes().to_vec()));
    }
    if let Ok(list) = obj.downcast::<PyList>() {
        let items: PyResult<Vec<SerializableValue>> = list
            .iter()
            .map(|item| py_to_serializable(py, &item))
            .collect();
        return Ok(SerializableValue::List(items?));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let pairs: PyResult<Vec<(String, SerializableValue)>> = dict
            .iter()
            .map(|(k, v)| {
                let key: String = k.extract()?;
                let value = py_to_serializable(py, &v)?;
                Ok((key, value))
            })
            .collect();
        return Ok(SerializableValue::Object(pairs?));
    }
    // Fallback: convert to string
    let s = obj.str()?.to_string();
    Ok(SerializableValue::String(s))
}

/// Materialize SerializableValue to Python object (GIL held)
fn serializable_to_py(py: Python<'_>, value: &SerializableValue) -> PyResult<PyObject> {
    match value {
        SerializableValue::Null => Ok(py.None()),
        SerializableValue::Bool(b) => Ok(b.to_object(py)),
        SerializableValue::Int(i) => Ok(i.to_object(py)),
        SerializableValue::Float(f) => Ok(f.to_object(py)),
        SerializableValue::String(s) => Ok(s.to_object(py)),
        SerializableValue::Bytes(b) => Ok(PyBytes::new(py, b).to_object(py)),
        SerializableValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(serializable_to_py(py, item)?)?;
            }
            Ok(list.to_object(py))
        }
        SerializableValue::Object(pairs) => {
            let dict = PyDict::new(py);
            for (k, v) in pairs {
                dict.set_item(k, serializable_to_py(py, v)?)?;
            }
            Ok(dict.to_object(py))
        }
    }
}

// ============================================================================
// Python API Application
// ============================================================================

/// Python API application
#[pyclass(name = "ApiApp")]
pub struct PyApiApp {
    /// Inner Rust app (Arc for thread-safe sharing)
    inner: Arc<RwLock<AppState>>,
}

struct AppState {
    title: String,
    version: String,
    router: Router,
    /// Python handlers stored by route ID
    handlers: HashMap<String, PyObject>,
    /// Route counter
    route_counter: usize,
}

#[pymethods]
impl PyApiApp {
    #[new]
    #[pyo3(signature = (title = "API", version = "1.0.0"))]
    fn new(title: &str, version: &str) -> Self {
        Self {
            inner: Arc::new(RwLock::new(AppState {
                title: title.to_string(),
                version: version.to_string(),
                router: Router::new(),
                handlers: HashMap::new(),
                route_counter: 0,
            })),
        }
    }

    /// Register a route handler
    #[pyo3(signature = (method, path, handler, validator_dict = None, metadata_dict = None))]
    fn register_route(
        &self,
        py: Python<'_>,
        method: &str,
        path: &str,
        handler: PyObject,
        validator_dict: Option<&Bound<'_, PyDict>>,
        metadata_dict: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        let http_method = method.parse::<HttpMethod>()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                sanitize_error_message(&format!("Invalid HTTP method: {}", e))
            ))?;

        let validator = if let Some(dict) = validator_dict {
            extract_validator(py, dict)?
        } else {
            RequestValidator::new()
        };

        let metadata = if let Some(dict) = metadata_dict {
            extract_metadata(py, dict)?
        } else {
            HandlerMeta::new("handler".to_string())
        };

        let mut state = self.inner.write().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                sanitize_error_message(&format!("Lock error: {}", e))
            )
        })?;

        let route_id = format!("route_{}", state.route_counter);
        state.route_counter += 1;

        // Store Python handler
        state.handlers.insert(route_id.clone(), handler);

        // Create Rust handler that calls Python
        let inner = Arc::clone(&self.inner);
        let rid = route_id.clone();
        let rust_handler = Arc::new(move |_req: data_bridge_api::request::Request, _validated: data_bridge_api::validation::ValidatedRequest| {
            let _inner = Arc::clone(&inner);
            let _rid = rid.clone();
            Box::pin(async move {
                // For now, return placeholder
                // TODO: Implement Python handler invocation
                Ok(data_bridge_api::response::Response::ok())
            }) as data_bridge_api::router::BoxFuture<'static, data_bridge_api::error::ApiResult<data_bridge_api::response::Response>>
        });

        state.router.route(http_method, path, rust_handler, validator, metadata)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                sanitize_error_message(&e.to_string())
            ))?;

        Ok(route_id)
    }

    /// Get OpenAPI JSON
    fn openapi_json(&self) -> PyResult<String> {
        let state = self.inner.read().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                sanitize_error_message(&format!("Lock error: {}", e))
            )
        })?;

        // Generate basic OpenAPI structure
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "info": {
                "title": state.title,
                "version": state.version
            },
            "paths": {}
        });

        serde_json::to_string_pretty(&openapi)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                sanitize_error_message(&e.to_string())
            ))
    }

    /// Match a request to a route (for testing)
    fn match_route(&self, method: &str, path: &str) -> PyResult<Option<(String, HashMap<String, String>)>> {
        let http_method = method.parse::<HttpMethod>()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                sanitize_error_message(&format!("Invalid HTTP method: {}", e))
            ))?;

        let state = self.inner.read().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                sanitize_error_message(&format!("Lock error: {}", e))
            )
        })?;

        if let Some(matched) = state.router.match_route(http_method, path) {
            Ok(Some(("matched".to_string(), matched.params)))
        } else {
            Ok(None)
        }
    }
}

// ============================================================================
// Type Descriptor Extraction
// ============================================================================

/// Extract TypeDescriptor from Python dict
fn extract_type_descriptor(_py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<TypeDescriptor> {
    let type_name: String = dict.get_item("type")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'type' field"))?
        .extract()?;

    match type_name.as_str() {
        "string" => {
            let mut constraints = StringConstraints::default();
            if let Some(min) = dict.get_item("min_length")? {
                constraints.min_length = Some(min.extract()?);
            }
            if let Some(max) = dict.get_item("max_length")? {
                constraints.max_length = Some(max.extract()?);
            }
            if let Some(pattern) = dict.get_item("pattern")? {
                constraints.pattern = Some(pattern.extract()?);
            }
            Ok(TypeDescriptor::String(constraints))
        }
        "int" | "integer" => {
            let mut constraints = NumericConstraints::default();
            if let Some(min) = dict.get_item("minimum")? {
                constraints.minimum = Some(min.extract()?);
            }
            if let Some(max) = dict.get_item("maximum")? {
                constraints.maximum = Some(max.extract()?);
            }
            Ok(TypeDescriptor::Int(constraints))
        }
        "float" | "number" => {
            let mut constraints = NumericConstraints::default();
            if let Some(min) = dict.get_item("minimum")? {
                constraints.minimum = Some(min.extract()?);
            }
            if let Some(max) = dict.get_item("maximum")? {
                constraints.maximum = Some(max.extract()?);
            }
            Ok(TypeDescriptor::Float(constraints))
        }
        "bool" | "boolean" => Ok(TypeDescriptor::Bool),
        "uuid" => Ok(TypeDescriptor::Uuid),
        "email" => Ok(TypeDescriptor::Email),
        "url" => Ok(TypeDescriptor::Url),
        "datetime" => Ok(TypeDescriptor::DateTime),
        "any" => Ok(TypeDescriptor::Any),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            sanitize_error_message(&format!("Unknown type: {}", type_name))
        )),
    }
}

/// Extract RequestValidator from Python dict
fn extract_validator(py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<RequestValidator> {
    let mut validator = RequestValidator::new();

    if let Some(path_params) = dict.get_item("path_params")? {
        if let Ok(list) = path_params.downcast::<PyList>() {
            for item in list.iter() {
                let param_dict = item.downcast::<PyDict>()?;
                validator.path_params.push(extract_param_validator(py, param_dict, ParamLocation::Path)?);
            }
        }
    }

    if let Some(query_params) = dict.get_item("query_params")? {
        if let Ok(list) = query_params.downcast::<PyList>() {
            for item in list.iter() {
                let param_dict = item.downcast::<PyDict>()?;
                validator.query_params.push(extract_param_validator(py, param_dict, ParamLocation::Query)?);
            }
        }
    }

    if let Some(header_params) = dict.get_item("header_params")? {
        if let Ok(list) = header_params.downcast::<PyList>() {
            for item in list.iter() {
                let param_dict = item.downcast::<PyDict>()?;
                validator.header_params.push(extract_param_validator(py, param_dict, ParamLocation::Header)?);
            }
        }
    }

    if let Some(body_type) = dict.get_item("body")? {
        if let Ok(d) = body_type.downcast::<PyDict>() {
            validator.body_validator = Some(extract_type_descriptor(py, d)?);
        }
    }

    Ok(validator)
}

/// Extract ParamValidator from Python dict
fn extract_param_validator(py: Python<'_>, dict: &Bound<'_, PyDict>, location: ParamLocation) -> PyResult<ParamValidator> {
    let name: String = dict.get_item("name")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'name' field"))?
        .extract()?;

    let type_dict = dict.get_item("type")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'type' field"))?;
    let type_desc = if let Ok(d) = type_dict.downcast::<PyDict>() {
        extract_type_descriptor(py, d)?
    } else {
        TypeDescriptor::Any
    };

    let required: bool = dict.get_item("required")?
        .map(|v| v.extract().unwrap_or(true))
        .unwrap_or(true);

    let default = if let Some(d) = dict.get_item("default")? {
        Some(py_to_serializable(py, &d)?)
    } else {
        None
    };

    Ok(ParamValidator {
        name,
        location,
        type_desc,
        required,
        default,
    })
}

/// Extract HandlerMeta from Python dict
fn extract_metadata(_py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<HandlerMeta> {
    let name = dict.get_item("name")?
        .map(|v| v.extract().unwrap_or_else(|_| "handler".to_string()))
        .unwrap_or_else(|| "handler".to_string());

    let mut meta = HandlerMeta::new(name);

    if let Some(v) = dict.get_item("operation_id")? {
        meta.operation_id = Some(v.extract()?);
    }
    if let Some(v) = dict.get_item("summary")? {
        meta.summary = Some(v.extract()?);
    }
    if let Some(v) = dict.get_item("description")? {
        meta.description = Some(v.extract()?);
    }
    if let Some(v) = dict.get_item("tags")? {
        let list = v.downcast::<PyList>()?;
        meta.tags = list.iter().map(|t| t.extract()).collect::<PyResult<Vec<String>>>()?;
    }
    if let Some(v) = dict.get_item("status_code")? {
        meta.status_code = v.extract()?;
    }
    if let Some(v) = dict.get_item("deprecated")? {
        meta.deprecated = v.extract()?;
    }

    Ok(meta)
}

// ============================================================================
// Python Request Wrapper
// ============================================================================

/// Python request wrapper
#[pyclass(name = "Request")]
pub struct PyRequest {
    inner: SerializableRequest,
}

#[pymethods]
impl PyRequest {
    #[getter]
    fn method(&self) -> &str {
        self.inner.method.as_str()
    }

    #[getter]
    fn path(&self) -> &str {
        &self.inner.path
    }

    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    fn path_param(&self, name: &str) -> Option<String> {
        self.inner.path_params.get(name).cloned()
    }

    fn query_param(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        match self.inner.query_params.get(name) {
            Some(v) => serializable_to_py(py, v),
            None => Ok(py.None()),
        }
    }

    fn header(&self, name: &str) -> Option<String> {
        self.inner.headers.get(&name.to_lowercase()).cloned()
    }

    fn body_json(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.inner.body {
            Some(v) => serializable_to_py(py, v),
            None => Ok(py.None()),
        }
    }
}

// ============================================================================
// Python Response Builder
// ============================================================================

/// Python response builder
#[pyclass(name = "Response")]
pub struct PyResponse {
    inner: SerializableResponse,
}

#[pymethods]
impl PyResponse {
    #[new]
    #[pyo3(signature = (status_code = 200))]
    fn new(status_code: u16) -> Self {
        Self {
            inner: SerializableResponse::new(status_code),
        }
    }

    #[staticmethod]
    fn json(py: Python<'_>, body: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value = py_to_serializable(py, body)?;
        Ok(Self {
            inner: SerializableResponse::json(value),
        })
    }

    #[staticmethod]
    fn text(body: &str) -> Self {
        Self {
            inner: SerializableResponse::text(body),
        }
    }

    fn status(&mut self, code: u16) {
        self.inner.status_code = code;
    }

    fn header(&mut self, name: &str, value: &str) {
        self.inner.headers.insert(name.to_lowercase(), value.to_string());
    }

    #[getter]
    fn status_code(&self) -> u16 {
        self.inner.status_code
    }
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register the api module
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyApiApp>()?;
    m.add_class::<PyRequest>()?;
    m.add_class::<PyResponse>()?;
    Ok(())
}
