//! Assertion types: PyExpectation and expect function.

use pyo3::prelude::*;
use pyo3::types::PyDict;

// =====================
// Expectation (Assertions)
// =====================

/// Python Expectation class for assertions
#[pyclass(name = "Expectation")]
pub struct PyExpectation {
    value: PyObject,
    negated: bool,
}

#[pymethods]
impl PyExpectation {
    /// Create a new expectation
    #[new]
    fn new(value: PyObject) -> Self {
        Self {
            value,
            negated: false,
        }
    }

    /// Negate the expectation
    #[getter]
    fn not_(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            value: self.value.clone_ref(py),
            negated: !self.negated,
        })
    }

    /// Assert equality
    fn to_equal(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        let result = self.value.bind(py).eq(expected.bind(py))?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT equal {:?}", self.value, expected)
            } else {
                format!("Expected {:?} to equal {:?}", self.value, expected)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert truthiness
    fn to_be_true(&self, py: Python<'_>) -> PyResult<()> {
        let result = self.value.bind(py).is_truthy()?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to be falsy", self.value)
            } else {
                format!("Expected {:?} to be truthy", self.value)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert falsiness
    fn to_be_false(&self, py: Python<'_>) -> PyResult<()> {
        let result = !self.value.bind(py).is_truthy()?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to be truthy", self.value)
            } else {
                format!("Expected {:?} to be falsy", self.value)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert is None
    fn to_be_none(&self, py: Python<'_>) -> PyResult<()> {
        let result = self.value.bind(py).is_none();
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                "Expected value to NOT be None".to_string()
            } else {
                format!("Expected None, but got {:?}", self.value)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert is NOT None (convenience method)
    fn to_not_be_none(&self, py: Python<'_>) -> PyResult<()> {
        let result = !self.value.bind(py).is_none();
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected value to be None, but got {:?}", self.value)
            } else {
                "Expected value to NOT be None, but got None".to_string()
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert value equals expected (alias for to_equal for simple comparisons)
    fn to_be(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        self.to_equal(py, expected)
    }

    /// Assert greater than
    fn to_be_greater_than(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        let result = self.value.bind(py).gt(expected.bind(py))?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT be greater than {:?}", self.value, expected)
            } else {
                format!("Expected {:?} to be greater than {:?}", self.value, expected)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert less than
    fn to_be_less_than(&self, py: Python<'_>, expected: PyObject) -> PyResult<()> {
        let result = self.value.bind(py).lt(expected.bind(py))?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT be less than {:?}", self.value, expected)
            } else {
                format!("Expected {:?} to be less than {:?}", self.value, expected)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert contains
    fn to_contain(&self, py: Python<'_>, item: PyObject) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let bound_item = item.bind(py);
        let result = bound_value.contains(bound_item)?;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT contain {:?}", self.value, item)
            } else {
                format!("Expected {:?} to contain {:?}", self.value, item)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert has key (for dicts)
    fn to_have_key(&self, py: Python<'_>, key: PyObject) -> PyResult<()> {
        let bound_value = self.value.bind(py);

        // Try to access as dict
        let result = if let Ok(dict) = bound_value.downcast::<PyDict>() {
            dict.contains(&key)?
        } else {
            // Try __contains__ method
            bound_value.contains(&key)?
        };

        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected {:?} to NOT have key {:?}", self.value, key)
            } else {
                format!("Expected {:?} to have key {:?}", self.value, key)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert length
    fn to_have_length(&self, py: Python<'_>, expected_len: usize) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let actual_len = bound_value.len()?;
        let result = actual_len == expected_len;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected length to NOT be {}, but got {}", expected_len, actual_len)
            } else {
                format!("Expected length {}, but got {}", expected_len, actual_len)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert empty
    fn to_be_empty(&self, py: Python<'_>) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let result = bound_value.len()? == 0;
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                "Expected value to NOT be empty".to_string()
            } else {
                format!("Expected empty value, but got length {}", bound_value.len()?)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert starts with (for strings)
    fn to_start_with(&self, py: Python<'_>, prefix: &str) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let s: String = bound_value.extract()?;
        let result = s.starts_with(prefix);
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected '{}' to NOT start with '{}'", s, prefix)
            } else {
                format!("Expected '{}' to start with '{}'", s, prefix)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert ends with (for strings)
    fn to_end_with(&self, py: Python<'_>, suffix: &str) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let s: String = bound_value.extract()?;
        let result = s.ends_with(suffix);
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected '{}' to NOT end with '{}'", s, suffix)
            } else {
                format!("Expected '{}' to end with '{}'", s, suffix)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert matches regex
    fn to_match(&self, py: Python<'_>, pattern: &str) -> PyResult<()> {
        let bound_value = self.value.bind(py);
        let s: String = bound_value.extract()?;

        let regex = regex::Regex::new(pattern)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex: {}", e)))?;

        let result = regex.is_match(&s);
        let passed = if self.negated { !result } else { result };

        if passed {
            Ok(())
        } else {
            let msg = if self.negated {
                format!("Expected '{}' to NOT match pattern '{}'", s, pattern)
            } else {
                format!("Expected '{}' to match pattern '{}'", s, pattern)
            };
            Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(msg))
        }
    }

    /// Assert that calling a callable raises a specific exception type.
    ///
    /// Usage: `expect(lambda: some_func()).to_raise(ValueError)`
    ///
    /// The value should be a callable (typically a lambda) that when called
    /// should raise the specified exception type.
    fn to_raise(&self, py: Python<'_>, exception_type: PyObject) -> PyResult<()> {
        let bound_callable = self.value.bind(py);

        // Verify the value is callable
        if !bound_callable.is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "to_raise() expects a callable (e.g., lambda: func())",
            ));
        }

        // Call the callable and see what happens
        let call_result = bound_callable.call0();

        match call_result {
            Ok(_) => {
                // No exception was raised
                if self.negated {
                    // expect(...).not().to_raise(E) - we expected NO exception, and none was raised
                    Ok(())
                } else {
                    // expect(...).to_raise(E) - we expected an exception, but none was raised
                    let exc_name = exception_type
                        .bind(py)
                        .getattr("__name__")
                        .map(|n| n.to_string())
                        .unwrap_or_else(|_| format!("{:?}", exception_type));
                    Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(format!(
                        "Expected {} to be raised, but no exception was raised",
                        exc_name
                    )))
                }
            }
            Err(err) => {
                // An exception was raised - check if it's the right type
                let raised_type = err.get_type(py);
                let expected_type = exception_type.bind(py);

                // Check if the raised exception is an instance of the expected type
                // Using PyAny::is_instance to handle inheritance properly
                let is_expected_type = raised_type.is_subclass(expected_type).unwrap_or(false);

                if self.negated {
                    // expect(...).not().to_raise(E) - we expected NO exception of type E
                    if is_expected_type {
                        let exc_name = expected_type
                            .getattr("__name__")
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| format!("{:?}", exception_type));
                        Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(format!(
                            "Expected {} NOT to be raised, but it was: {}",
                            exc_name,
                            err
                        )))
                    } else {
                        // A different exception was raised, which is fine for negated case
                        // But we should re-raise it since it's unexpected
                        Err(err)
                    }
                } else {
                    // expect(...).to_raise(E) - we expected exception of type E
                    if is_expected_type {
                        Ok(())
                    } else {
                        let expected_name = expected_type
                            .getattr("__name__")
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| format!("{:?}", exception_type));
                        let raised_name = raised_type
                            .getattr("__name__")
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| "Unknown".to_string());
                        Err(PyErr::new::<pyo3::exceptions::PyAssertionError, _>(format!(
                            "Expected {} to be raised, but got {}: {}",
                            expected_name, raised_name, err
                        )))
                    }
                }
            }
        }
    }
}

/// Create an expectation from a value
#[pyfunction]
pub fn expect(value: PyObject) -> PyExpectation {
    PyExpectation::new(value)
}
