//! HTTP client PyO3 bindings
//!
//! Provides Python bindings for the data-bridge-http crate,
//! following the same patterns as mongodb.rs.

use data_bridge_http::{HttpClient, HttpClientConfig, HttpResponse as RustResponse};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error_handling::sanitize_error_message;

/// Python HttpClient class
#[pyclass(name = "HttpClient")]
pub struct PyHttpClient {
    inner: Arc<HttpClient>,
}

#[pymethods]
impl PyHttpClient {
    /// Create a new HTTP client
    ///
    /// # Arguments
    /// * `base_url` - Base URL for all requests (optional)
    /// * `timeout` - Total request timeout in seconds (default: 30.0)
    /// * `connect_timeout` - Connection timeout in seconds (default: 10.0)
    /// * `pool_max_idle_per_host` - Max idle connections per host (default: 10)
    /// * `follow_redirects` - Whether to follow redirects (default: true)
    /// * `max_redirects` - Maximum redirects to follow (default: 10)
    #[new]
    #[pyo3(signature = (
        base_url = None,
        timeout = 30.0,
        connect_timeout = 10.0,
        pool_max_idle_per_host = 10,
        follow_redirects = true,
        max_redirects = 10,
        user_agent = None,
        danger_accept_invalid_certs = false
    ))]
    fn new(
        base_url: Option<String>,
        timeout: f64,
        connect_timeout: f64,
        pool_max_idle_per_host: usize,
        follow_redirects: bool,
        max_redirects: usize,
        user_agent: Option<String>,
        danger_accept_invalid_certs: bool,
    ) -> PyResult<Self> {
        let mut config = HttpClientConfig::new()
            .timeout_secs(timeout)
            .connect_timeout_secs(connect_timeout)
            .pool_max_idle_per_host(pool_max_idle_per_host)
            .follow_redirects(follow_redirects)
            .max_redirects(max_redirects)
            .danger_accept_invalid_certs(danger_accept_invalid_certs);

        if let Some(url) = base_url {
            config = config.base_url(url);
        }

        if let Some(ua) = user_agent {
            config = config.user_agent(ua);
        }

        let client = HttpClient::new(config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(client),
        })
    }

    /// Get the base URL
    #[getter]
    fn base_url(&self) -> Option<String> {
        self.inner.base_url().map(String::from)
    }

    /// Send a GET request
    #[pyo3(signature = (path, headers = None, params = None, timeout = None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "GET", path, headers, params, None, None, timeout)
    }

    /// Send a POST request
    #[pyo3(signature = (path, headers = None, params = None, json = None, form = None, timeout = None))]
    fn post<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        form: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "POST", path, headers, params, json, form, timeout)
    }

    /// Send a PUT request
    #[pyo3(signature = (path, headers = None, params = None, json = None, form = None, timeout = None))]
    fn put<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        form: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "PUT", path, headers, params, json, form, timeout)
    }

    /// Send a PATCH request
    #[pyo3(signature = (path, headers = None, params = None, json = None, form = None, timeout = None))]
    fn patch<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        form: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "PATCH", path, headers, params, json, form, timeout)
    }

    /// Send a DELETE request
    #[pyo3(signature = (path, headers = None, params = None, timeout = None))]
    fn delete<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "DELETE", path, headers, params, None, None, timeout)
    }

    /// Send a HEAD request
    #[pyo3(signature = (path, headers = None, params = None, timeout = None))]
    fn head<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "HEAD", path, headers, params, None, None, timeout)
    }

    /// Send an OPTIONS request
    #[pyo3(signature = (path, headers = None, params = None, timeout = None))]
    fn options<'py>(
        &self,
        py: Python<'py>,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "OPTIONS", path, headers, params, None, None, timeout)
    }

    /// Generic request method
    #[pyo3(signature = (method, path, headers = None, params = None, json = None, form = None, timeout = None))]
    fn request<'py>(
        &self,
        py: Python<'py>,
        method: &str,
        path: String,
        headers: Option<&Bound<'_, PyDict>>,
        params: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        form: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Phase 1: Extract Python values with GIL
        let extracted_headers = extract_dict_to_vec(headers)?;
        let extracted_params = extract_dict_to_vec(params)?;
        let extracted_body = extract_body(py, json, form)?;
        let method_str = method.to_uppercase();
        let timeout_ms = timeout.map(|t| (t * 1000.0) as u64);

        let client = self.inner.clone();

        // Phase 2: Execute async without GIL
        future_into_py(py, async move {
            use data_bridge_http::request::{ExtractedAuth, ExtractedRequest, HttpMethod};
            use std::str::FromStr;

            let method = HttpMethod::from_str(&method_str)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

            let request = ExtractedRequest {
                method,
                url: path,
                headers: extracted_headers,
                query_params: extracted_params,
                body: extracted_body,
                auth: ExtractedAuth::None,
                timeout_ms,
            };

            let response = client.execute(request).await.map_err(|e| {
                let sanitized = sanitize_error_message(&e.to_string());
                match e.category() {
                    data_bridge_http::error::HttpErrorCategory::Connection => {
                        PyErr::new::<pyo3::exceptions::PyConnectionError, _>(sanitized)
                    }
                    data_bridge_http::error::HttpErrorCategory::Timeout => {
                        PyErr::new::<pyo3::exceptions::PyTimeoutError, _>(sanitized)
                    }
                    _ => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(sanitized),
                }
            })?;

            // Phase 3: Convert response to Python
            Python::with_gil(|py| rust_response_to_py(py, response))
        })
    }
}

/// Python HttpResponse class
#[pyclass(name = "HttpResponse")]
pub struct PyHttpResponse {
    #[pyo3(get)]
    status_code: u16,
    #[pyo3(get)]
    latency_ms: u64,
    #[pyo3(get)]
    url: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[pymethods]
impl PyHttpResponse {
    /// Get response headers as a dict
    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (key, value) in &self.headers {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    /// Check if response is successful (2xx)
    fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Check if response is client error (4xx)
    fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    /// Check if response is server error (5xx)
    fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    /// Get body as text (UTF-8)
    fn text(&self) -> PyResult<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyUnicodeDecodeError, _>(e.to_string()))
    }

    /// Get body as JSON (using pythonize for efficient conversion)
    fn json(&self, py: Python<'_>) -> PyResult<PyObject> {
        let value: serde_json::Value = serde_json::from_slice(&self.body)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON decode error: {}", e)))?;
        pythonize::pythonize(py, &value)
            .map(|bound| bound.unbind())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("JSON conversion error: {}", e)))
    }

    /// Get body as bytes
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.body)
    }

    /// Get content length
    fn content_length(&self) -> usize {
        self.body.len()
    }

    /// Get a header value (case-insensitive)
    fn header(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.clone())
    }

    /// Get content type
    fn content_type(&self) -> Option<String> {
        self.header("content-type")
    }

    fn __repr__(&self) -> String {
        format!(
            "<HttpResponse status={} latency={}ms size={}>",
            self.status_code,
            self.latency_ms,
            self.body.len()
        )
    }
}

// Helper functions

/// Extract Python dict to Vec of (key, value) pairs
fn extract_dict_to_vec(dict: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, String)>> {
    match dict {
        Some(d) => {
            let mut result = Vec::new();
            for (key, value) in d.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                result.push((k, v));
            }
            Ok(result)
        }
        None => Ok(Vec::new()),
    }
}

/// Extract body from JSON or form data
fn extract_body(
    py: Python<'_>,
    json: Option<&Bound<'_, PyAny>>,
    form: Option<&Bound<'_, PyDict>>,
) -> PyResult<data_bridge_http::request::ExtractedBody> {
    use data_bridge_http::request::ExtractedBody;

    if let Some(json_data) = json {
        let value = py_to_json(py, json_data)?;
        return Ok(ExtractedBody::Json(value));
    }

    if let Some(form_data) = form {
        let fields = extract_dict_to_vec(Some(form_data))?;
        return Ok(ExtractedBody::Form(fields));
    }

    Ok(ExtractedBody::None)
}

/// Convert Python object to serde_json::Value
fn py_to_json(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    use pyo3::types::{PyBool, PyFloat, PyInt, PyList, PyNone, PyString};

    if obj.is_instance_of::<PyNone>() {
        return Ok(serde_json::Value::Null);
    }

    if obj.is_instance_of::<PyBool>() {
        let b: bool = obj.extract()?;
        return Ok(serde_json::Value::Bool(b));
    }

    if obj.is_instance_of::<PyInt>() {
        let i: i64 = obj.extract()?;
        return Ok(serde_json::Value::Number(i.into()));
    }

    if obj.is_instance_of::<PyFloat>() {
        let f: f64 = obj.extract()?;
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Ok(serde_json::Value::Number(n));
        }
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot convert float to JSON",
        ));
    }

    if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        return Ok(serde_json::Value::String(s));
    }

    if obj.is_instance_of::<PyList>() {
        let list = obj.downcast::<PyList>()?;
        let arr: Vec<serde_json::Value> = list
            .iter()
            .map(|item| py_to_json(py, &item))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(serde_json::Value::Array(arr));
    }

    if obj.is_instance_of::<PyDict>() {
        let dict = obj.downcast::<PyDict>()?;
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let k: String = key.extract()?;
            let v = py_to_json(py, &value)?;
            map.insert(k, v);
        }
        return Ok(serde_json::Value::Object(map));
    }

    // Try to convert via str() as fallback
    let s: String = obj.str()?.extract()?;
    Ok(serde_json::Value::String(s))
}

/// Convert Rust HttpResponse to Python PyHttpResponse
fn rust_response_to_py(py: Python<'_>, response: RustResponse) -> PyResult<PyObject> {
    let py_response = PyHttpResponse {
        status_code: response.status_code,
        latency_ms: response.latency_ms,
        url: response.url,
        headers: response.headers,
        body: response.body,
    };
    Ok(py_response.into_pyobject(py)?.into_any().unbind())
}

/// Register HTTP module functions and classes
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHttpClient>()?;
    m.add_class::<PyHttpResponse>()?;
    m.add("__doc__", "High-performance async HTTP client with Rust backend")?;
    Ok(())
}
