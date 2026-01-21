//! Tool PyO3 bindings

use ouroboros_agent_tools::{
    global_registry, FunctionTool, Tool as RustTool, ToolParameter, ToolRegistry,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;

use super::utils::{json_to_py, py_to_json};
use crate::error_handling::sanitize_error_message;

/// Python Tool wrapper
///
/// Wraps a Python async function as a tool that can be called by agents.
///
/// Example:
///     >>> from ouroboros.agent import Tool
///     >>> @Tool(name="search", description="Search the web")
///     >>> async def search(query: str) -> dict:
///     ...     return {"results": []}
#[pyclass(name = "Tool")]
#[derive(Clone)]
pub struct PyTool {
    inner: Arc<dyn RustTool>,
}

#[pymethods]
impl PyTool {
    /// Create a tool from a Python function
    ///
    /// Args:
    ///     name: Tool name
    ///     description: Tool description
    ///     parameters: List of parameter definitions
    ///     function: Python async function to execute
    #[new]
    #[pyo3(signature = (name, description, parameters, function))]
    fn new(
        name: String,
        description: String,
        parameters: Vec<Bound<'_, PyDict>>,
        function: PyObject,
    ) -> PyResult<Self> {
        // Convert parameters
        let rust_params: Vec<ToolParameter> = parameters
            .iter()
            .map(|param_dict| {
                let param_name: String = param_dict.get_item("name")?.unwrap().extract()?;
                let param_desc: String = param_dict.get_item("description")?.unwrap().extract()?;
                let param_type: String = param_dict
                    .get_item("type")?
                    .map(|v| v.extract())
                    .unwrap_or_else(|| Ok("string".to_string()))?;
                let required: bool = param_dict
                    .get_item("required")?
                    .map(|v| v.extract())
                    .unwrap_or_else(|| Ok(true))?;

                Ok(ToolParameter {
                    name: param_name,
                    description: param_desc,
                    required,
                    parameter_type: param_type,
                })
            })
            .collect::<PyResult<Vec<_>>>()?;

        // Wrap the Python function for tool execution
        // The function is stored as PyObject and will be called with GIL acquired
        let tool = FunctionTool::new(name, description, rust_params, move |args| {
            // Clone the function reference for this execution
            let func = Python::with_gil(|py| function.clone_ref(py));
            Box::pin(async move {
                // Convert JSON args to Python and call function
                let result = Python::with_gil(|py| {
                    // Convert JSON args to Python dict
                    let py_args = json_to_py(py, &args).map_err(|e| {
                        ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                            "Failed to convert arguments to Python: {}",
                            e
                        ))
                    })?;

                    // Call the Python function with keyword arguments
                    // If py_args is a dict, use it as **kwargs; otherwise pass as single arg
                    let result = if let Ok(py_dict) = py_args.downcast_bound::<pyo3::types::PyDict>(py) {
                        // Call with keyword arguments (**kwargs)
                        func.call_bound(py, (), Some(&py_dict)).map_err(|e| {
                            ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                                "Python function call failed: {}",
                                sanitize_error_message(&e.to_string())
                            ))
                        })?
                    } else {
                        // Fallback: call with single positional argument
                        func.call1(py, (py_args,)).map_err(|e| {
                            ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                                "Python function call failed: {}",
                                sanitize_error_message(&e.to_string())
                            ))
                        })?
                    };

                    Ok::<_, ouroboros_agent_tools::ToolError>(result)
                })?;

                // Check if result is a coroutine (async) and await it
                let is_coroutine = Python::with_gil(|py| {
                    let result_obj = result.bind(py);
                    result_obj.hasattr("__await__").unwrap_or(false)
                });

                if is_coroutine {
                    // It's an async function - convert to Rust future and await
                    let future = Python::with_gil(|py| {
                        let bound = result.bind(py);
                        pyo3_async_runtimes::tokio::into_future(bound.clone())
                    })
                    .map_err(|e| {
                        ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                            "Failed to convert coroutine to future: {}",
                            e
                        ))
                    })?;

                    let awaited = future.await.map_err(|e| {
                        ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                            "Async function execution failed: {}",
                            sanitize_error_message(&e.to_string())
                        ))
                    })?;

                    // Convert awaited result to JSON
                    Python::with_gil(|py| {
                        py_to_json(awaited.bind(py).as_any())
                    })
                    .map_err(|e| {
                        ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                            "Failed to convert async result to JSON: {}",
                            e
                        ))
                    })
                } else {
                    // Synchronous function - result is ready, convert to JSON
                    Python::with_gil(|py| {
                        py_to_json(result.bind(py).as_any())
                    })
                    .map_err(|e| {
                        ouroboros_agent_tools::ToolError::ExecutionFailed(format!(
                            "Failed to convert result to JSON: {}",
                            e
                        ))
                    })
                }
            })
        });

        Ok(Self {
            inner: Arc::new(tool),
        })
    }

    /// Get tool name
    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Get tool description
    #[getter]
    fn description(&self) -> String {
        self.inner.description().to_string()
    }

    /// Get tool parameters
    #[getter]
    fn parameters(&self, py: Python) -> PyResult<PyObject> {
        let params = self.inner.parameters();
        let list = pyo3::types::PyList::empty(py);

        for param in params {
            let dict = PyDict::new(py);
            dict.set_item("name", param.name)?;
            dict.set_item("description", param.description)?;
            dict.set_item("type", param.parameter_type)?;
            dict.set_item("required", param.required)?;
            list.append(dict)?;
        }

        Ok(list.into())
    }

    /// Execute the tool
    ///
    /// Args:
    ///     arguments: Dictionary of arguments
    ///
    /// Returns:
    ///     Tool execution result
    fn execute<'py>(
        &self,
        py: Python<'py>,
        arguments: Bound<'_, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let tool = self.inner.clone();
        let args_json = py_to_json(arguments.as_any())?;

        future_into_py(py, async move {
            let result = tool.execute(args_json).await.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(sanitize_error_message(
                    &e.to_string(),
                ))
            })?;

            Python::with_gil(|py| {
                // Convert JSON result to Python
                json_to_py(py, &result)
            })
        })
    }
}

/// Tool Registry
///
/// Thread-safe registry for managing tools.
///
/// Example:
///     >>> from ouroboros.agent import ToolRegistry, get_global_registry
///     >>> registry = ToolRegistry()
///     >>> registry.register(my_tool)
///     >>> # Or use global registry
///     >>> global_reg = get_global_registry()
///     >>> global_reg.register(my_tool)
#[pyclass(name = "ToolRegistry")]
pub struct PyToolRegistry {
    inner: Arc<ToolRegistry>,
}

#[pymethods]
impl PyToolRegistry {
    /// Create a new tool registry
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(ToolRegistry::new()),
        }
    }

    /// Register a tool
    ///
    /// Args:
    ///     tool: Tool to register
    fn register(&self, tool: &PyTool) -> PyResult<()> {
        self.inner
            .register(tool.inner.clone())
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(sanitize_error_message(
                    &e.to_string(),
                ))
            })?;
        Ok(())
    }

    /// Unregister a tool by name
    ///
    /// Args:
    ///     name: Tool name
    fn unregister(&self, name: &str) -> PyResult<()> {
        self.inner.unregister(name).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(sanitize_error_message(&e.to_string()))
        })?;
        Ok(())
    }

    /// Check if a tool is registered
    ///
    /// Args:
    ///     name: Tool name
    ///
    /// Returns:
    ///     True if tool is registered
    fn contains(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    /// Get all registered tool names
    ///
    /// Returns:
    ///     List of tool names
    fn tool_names(&self) -> Vec<String> {
        self.inner.tool_names()
    }

    /// Get count of registered tools
    fn count(&self) -> usize {
        self.inner.count()
    }

    /// Clear all tools
    fn clear(&self) {
        self.inner.clear();
    }
}

/// Get the global tool registry
///
/// Returns:
///     The global tool registry singleton
#[pyfunction]
pub fn get_global_registry() -> PyToolRegistry {
    PyToolRegistry {
        inner: Arc::new(ToolRegistry::new()),
    }
}

// Helper to get actual global registry reference
pub(crate) fn get_rust_global_registry() -> &'static ToolRegistry {
    global_registry()
}
