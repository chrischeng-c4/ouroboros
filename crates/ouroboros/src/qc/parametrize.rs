//! Parametrize types for test parameterization.

use ouroboros_qc::parametrize::{Parameter, ParameterSet, ParameterValue, ParametrizedTest};
use pyo3::prelude::*;
use pyo3::types::PyDict;

// =====================
// ParameterValue
// =====================

/// Python ParameterValue class
#[pyclass(name = "ParameterValue")]
#[derive(Clone)]
pub struct PyParameterValue {
    pub(super) inner: ParameterValue,
}

#[pymethods]
impl PyParameterValue {
    /// Create an integer parameter value
    #[staticmethod]
    fn int(value: i64) -> Self {
        Self {
            inner: ParameterValue::Int(value),
        }
    }

    /// Create a float parameter value
    #[staticmethod]
    fn float(value: f64) -> Self {
        Self {
            inner: ParameterValue::Float(value),
        }
    }

    /// Create a string parameter value
    #[staticmethod]
    fn string(value: String) -> Self {
        Self {
            inner: ParameterValue::String(value),
        }
    }

    /// Create a boolean parameter value
    #[staticmethod]
    fn bool(value: bool) -> Self {
        Self {
            inner: ParameterValue::Bool(value),
        }
    }

    /// Create a None parameter value
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: ParameterValue::None,
        }
    }

    /// Create from Python object (auto-conversion)
    #[staticmethod]
    fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(v) = obj.extract::<i64>() {
            Ok(Self::int(v))
        } else if let Ok(v) = obj.extract::<f64>() {
            Ok(Self::float(v))
        } else if let Ok(v) = obj.extract::<String>() {
            Ok(Self::string(v))
        } else if let Ok(v) = obj.extract::<bool>() {
            Ok(Self::bool(v))
        } else if obj.is_none() {
            Ok(Self::none())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                format!("Unsupported parameter type: {}", obj.get_type())
            ))
        }
    }

    /// Format for test name
    fn format_for_name(&self) -> String {
        self.inner.format_for_name()
    }

    /// Convert to Python object
    #[allow(deprecated)]
    fn to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        use pyo3::ToPyObject;
        match &self.inner {
            ParameterValue::Int(v) => Ok(v.to_object(py)),
            ParameterValue::Float(v) => Ok(v.to_object(py)),
            ParameterValue::String(v) => Ok(v.to_object(py)),
            ParameterValue::Bool(v) => Ok(v.to_object(py)),
            ParameterValue::None => Ok(py.None()),
            ParameterValue::List(_) => Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                "List parameter values not yet supported for Python conversion"
            )),
            ParameterValue::Dict(_) => Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                "Dict parameter values not yet supported for Python conversion"
            )),
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ParameterValue({})", self.inner)
    }
}

// =====================
// ParameterSet
// =====================

/// Python ParameterSet class
#[pyclass(name = "ParameterSet")]
#[derive(Clone)]
pub struct PyParameterSet {
    pub(super) inner: ParameterSet,
}

#[pymethods]
impl PyParameterSet {
    #[new]
    fn new() -> Self {
        Self {
            inner: ParameterSet::new(),
        }
    }

    /// Add a parameter
    fn add(&mut self, name: String, value: PyParameterValue) {
        self.inner.add(name, value.inner);
    }

    /// Get a parameter value
    fn get(&self, name: &str) -> Option<PyParameterValue> {
        self.inner.get(name).map(|v| PyParameterValue { inner: v.clone() })
    }

    /// Format for test name
    fn format_for_name(&self) -> String {
        self.inner.format_for_name()
    }

    /// Convert to Python dict
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.params {
            let py_val = PyParameterValue { inner: v.clone() }.to_py(py)?;
            dict.set_item(k, py_val)?;
        }
        Ok(dict.to_object(py))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("ParameterSet({})", self.inner.format_for_name())
    }
}

// =====================
// Parameter
// =====================

/// Python Parameter class
#[pyclass(name = "Parameter")]
#[derive(Clone)]
pub struct PyParameter {
    pub(super) inner: Parameter,
}

#[pymethods]
impl PyParameter {
    #[new]
    fn new(name: String, values: Vec<PyParameterValue>) -> Self {
        let values = values.into_iter().map(|v| v.inner).collect();
        Self {
            inner: Parameter::new(name, values),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn values(&self) -> Vec<PyParameterValue> {
        self.inner.values.iter().map(|v| PyParameterValue { inner: v.clone() }).collect()
    }

    /// Validate the parameter
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e)
        })
    }

    fn __repr__(&self) -> String {
        format!("Parameter(name='{}', values={} items)", self.inner.name, self.inner.values.len())
    }
}

// =====================
// ParametrizedTest
// =====================

/// Python ParametrizedTest class
#[pyclass(name = "ParametrizedTest")]
#[derive(Clone)]
pub struct PyParametrizedTest {
    inner: ParametrizedTest,
}

#[pymethods]
impl PyParametrizedTest {
    #[new]
    fn new(base_name: String) -> Self {
        Self {
            inner: ParametrizedTest::new(base_name),
        }
    }

    /// Add a parameter
    fn add_parameter(&mut self, param: PyParameter) -> PyResult<()> {
        self.inner.add_parameter(param.inner).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e)
        })
    }

    /// Expand into test instances
    fn expand(&self) -> Vec<(String, PyParameterSet)> {
        self.inner.expand().into_iter().map(|(name, set)| {
            (name, PyParameterSet { inner: set })
        }).collect()
    }

    /// Count total instances
    fn count_instances(&self) -> usize {
        self.inner.count_instances()
    }

    #[getter]
    fn base_name(&self) -> &str {
        &self.inner.base_name
    }

    fn __repr__(&self) -> String {
        format!(
            "ParametrizedTest(base_name='{}', parameters={}, instances={})",
            self.inner.base_name,
            self.inner.parameters.len(),
            self.inner.count_instances()
        )
    }
}
