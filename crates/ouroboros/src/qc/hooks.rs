//! Hook types.

use ouroboros_qc::HookType;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::enums::PyHookType;

// =====================
// HookRegistry
// =====================

/// Python HookRegistry class
#[pyclass(name = "HookRegistry")]
pub struct PyHookRegistry {
    hooks: Arc<Mutex<HashMap<HookType, Vec<PyObject>>>>,
}

#[pymethods]
impl PyHookRegistry {
    /// Create a new hook registry
    #[new]
    fn new() -> Self {
        Self {
            hooks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a hook function
    fn register_hook(&self, hook_type: PyHookType, hook_fn: PyObject) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks
            .entry(hook_type.into())
            .or_insert_with(Vec::new)
            .push(hook_fn);
    }

    /// Clear all hooks of a specific type
    fn clear_hooks(&self, hook_type: PyHookType) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.remove(&hook_type.into());
    }

    /// Clear all hooks
    fn clear_all(&self) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.clear();
    }

    /// Get the number of registered hooks for a specific type
    fn hook_count(&self, hook_type: PyHookType) -> usize {
        let hooks = self.hooks.lock().unwrap();
        hooks
            .get(&hook_type.into())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Run hooks of a specific type (async method)
    ///
    /// Returns a Python coroutine that runs on the caller's event loop.
    /// This ensures async hooks (like database setup) work correctly
    /// because they share the same event loop as the test runner.
    #[allow(deprecated)]
    fn run_hooks<'py>(
        &self,
        py: Python<'py>,
        hook_type: PyHookType,
        suite_instance: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let hook_type_rust: HookType = hook_type.into();
        let is_teardown = hook_type_rust.is_teardown();

        // Clone hooks while holding the lock
        let hooks_to_run: Vec<PyObject> = {
            let hooks = self.hooks.lock().unwrap();
            hooks
                .get(&hook_type_rust)
                .map(|v| v.iter().map(|obj| obj.clone_ref(py)).collect())
                .unwrap_or_default()
        };

        // Create Python async function to run hooks on caller's event loop
        let code = c"
async def _run_hooks(hooks, instance, is_teardown, asyncio):
    if not hooks:
        return None

    errors = []
    for idx, hook_fn in enumerate(hooks):
        hook_name = f\"hook[{idx}]\"
        try:
            if asyncio.iscoroutinefunction(hook_fn):
                if instance is not None:
                    await hook_fn(instance)
                else:
                    await hook_fn()
            else:
                if instance is not None:
                    hook_fn(instance)
                else:
                    hook_fn()
        except Exception as e:
            error_msg = f\"{hook_name} failed: {e}\"
            errors.append(error_msg)
            if not is_teardown:
                raise

    return \"; \".join(errors) if errors else None
";

        // Execute the code to define _run_hooks
        let globals = pyo3::types::PyDict::new(py);
        py.run(code, Some(&globals), None)?;

        // Get the function and call it
        let run_hooks_fn = globals.get_item("_run_hooks")?.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to get _run_hooks function")
        })?;

        let asyncio = py.import("asyncio")?;
        let hooks_list = pyo3::types::PyList::new(py, hooks_to_run.iter().map(|h| h.bind(py)))?;

        // Return the coroutine (not awaited yet - caller will await it)
        let coro = run_hooks_fn.call1((hooks_list, suite_instance, is_teardown, asyncio))?;
        Ok(coro)
    }

    fn __repr__(&self) -> String {
        let hooks = self.hooks.lock().unwrap();
        format!(
            "HookRegistry(setup_class={}, teardown_class={}, setup_method={}, teardown_method={}, setup_module={}, teardown_module={})",
            hooks.get(&HookType::SetupClass).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::TeardownClass).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::SetupMethod).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::TeardownMethod).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::SetupModule).map(|v| v.len()).unwrap_or(0),
            hooks.get(&HookType::TeardownModule).map(|v| v.len()).unwrap_or(0),
        )
    }
}
