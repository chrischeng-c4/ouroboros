//! Task type for Python coroutines
//!
//! A Task wraps a Python coroutine and schedules it for execution on the Tokio runtime.

use pyo3::exceptions::{PyException, PyRuntimeError, PyStopIteration};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

// Define a custom CancelledError exception
pyo3::create_exception!(
    data_bridge_pyloop,
    PyCancelledError,
    PyException,
    "Task was cancelled"
);

/// Result of polling a Python coroutine once
pub enum PollResult {
    /// Coroutine yielded a value (not done yet)
    Pending(PyObject),
    /// Coroutine finished with a return value
    Ready(PyObject),
}

/// Poll a Python coroutine once
///
/// This function advances the coroutine by one step by calling its `send(None)` method.
/// It returns either:
/// - `PollResult::Pending` if the coroutine yielded a value
/// - `PollResult::Ready` if the coroutine finished (StopIteration raised)
/// - `Err` if the coroutine raised an exception
pub fn poll_coroutine(py: Python, coro: &Bound<PyAny>) -> PyResult<PollResult> {
    // Try to send None to the coroutine (which steps it forward)
    let result = coro.call_method1("send", (py.None(),));

    match result {
        Ok(value) => {
            // Coroutine yielded a value (not done yet)
            Ok(PollResult::Pending(value.unbind()))
        }
        Err(e) => {
            // Check if it's StopIteration (coroutine finished)
            if e.is_instance_of::<PyStopIteration>(py) {
                // Extract the return value from StopIteration.value
                let stop_iteration = e.value(py);
                let return_value = stop_iteration
                    .getattr("value")
                    .unwrap_or_else(|_| py.None().into_bound(py))
                    .unbind();
                Ok(PollResult::Ready(return_value))
            } else {
                // Some other exception
                Err(e)
            }
        }
    }
}

/// Task wrapping a Python coroutine
///
/// A Task represents a scheduled coroutine that will run to completion.
/// It can be awaited, cancelled, or have its result retrieved.
///
/// # Thread Safety
///
/// Task is thread-safe and uses atomic flags for state management.
///
/// # Example
///
/// ```python
/// async def my_coro():
///     await asyncio.sleep(1)
///     return 42
///
/// task = loop.create_task(my_coro())
/// result = await task  # Returns 42
/// ```
#[pyclass]
#[derive(Clone)]
pub struct Task {
    /// The coroutine being executed
    /// Kept for future use (e.g., introspection, debugging)
    #[allow(dead_code)]
    coro: Arc<Mutex<Option<PyObject>>>,

    /// Whether the task has been cancelled
    cancelled: Arc<AtomicBool>,

    /// Whether the task is done
    done: Arc<AtomicBool>,

    /// Result of the task (when done)
    result: Arc<Mutex<Option<PyObject>>>,

    /// Exception raised by the task (if any)
    exception: Arc<Mutex<Option<PyObject>>>,

    /// Tokio join handle
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,

    /// Optional task name
    name: Option<String>,
}

#[pymethods]
impl Task {
    /// Cancel the task
    ///
    /// Request that the task be cancelled. If the task has already completed,
    /// this method has no effect. Returns True if the task was successfully
    /// cancelled, False otherwise.
    ///
    /// Returns:
    ///     bool: True if task was cancelled, False if already done
    ///
    /// # Example
    ///
    /// ```python
    /// task = loop.create_task(my_coro())
    /// success = task.cancel()
    /// assert task.cancelled()
    /// ```
    fn cancel(&self) -> bool {
        if self.done() {
            return false;
        }

        self.cancelled.store(true, Ordering::Release);
        self.done.store(true, Ordering::Release);

        // Abort the Tokio task
        if let Some(handle) = self.task_handle.lock().unwrap().take() {
            handle.abort();
        }

        true
    }

    /// Check if the task has been cancelled
    ///
    /// Returns:
    ///     bool: True if cancelled, False otherwise
    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Check if the task is done
    ///
    /// A task is done when it has finished execution (successfully or with an
    /// exception) or when it has been cancelled.
    ///
    /// Returns:
    ///     bool: True if done, False otherwise
    fn done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Get the task result
    ///
    /// Return the result of the task. If the task is done, the result is
    /// returned (or the exception is re-raised). If the task has been
    /// cancelled, a CancelledError is raised. If the task is not done yet,
    /// a RuntimeError is raised.
    ///
    /// Returns:
    ///     object: The result of the coroutine
    ///
    /// Raises:
    ///     RuntimeError: If task is not done yet
    ///     CancelledError: If task was cancelled
    ///     Exception: The exception raised by the coroutine
    ///
    /// # Example
    ///
    /// ```python
    /// task = loop.create_task(my_coro())
    /// # ... wait for task to complete ...
    /// result = task.result()  # Returns the coroutine's return value
    /// ```
    fn result(&self, py: Python) -> PyResult<PyObject> {
        if !self.done() {
            return Err(PyRuntimeError::new_err("Task is not done yet"));
        }

        // Check for cancellation first
        if self.cancelled() {
            return Err(PyCancelledError::new_err("Task was cancelled"));
        }

        // Check for exception
        if let Some(exc) = self.exception.lock().unwrap().as_ref() {
            let exc_bound = exc.clone_ref(py).into_bound(py);
            return Err(PyErr::from_value(exc_bound));
        }

        // Return result
        Ok(self
            .result
            .lock()
            .unwrap()
            .as_ref()
            .map(|r| r.clone_ref(py))
            .unwrap_or_else(|| py.None()))
    }

    /// Get the task name
    ///
    /// Returns:
    ///     Optional[str]: The task name if set, None otherwise
    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Set the task name
    ///
    /// Args:
    ///     name (str): The new task name
    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Get debug representation
    fn __repr__(&self) -> String {
        let state = if self.done() {
            if self.cancelled() {
                "cancelled"
            } else {
                "done"
            }
        } else {
            "pending"
        };

        match &self.name {
            Some(name) => format!("Task(name='{}', state={})", name, state),
            None => format!("Task(state={})", state),
        }
    }
}

impl Task {
    /// Create a new Task wrapping a coroutine
    ///
    /// # Arguments
    ///
    /// * `coro` - The Python coroutine object
    /// * `name` - Optional task name
    /// * `task_handle` - Tokio task handle for cancellation
    pub fn new(coro: PyObject, name: Option<String>, task_handle: JoinHandle<()>) -> Self {
        Self {
            coro: Arc::new(Mutex::new(Some(coro))),
            cancelled: Arc::new(AtomicBool::new(false)),
            done: Arc::new(AtomicBool::new(false)),
            result: Arc::new(Mutex::new(None)),
            exception: Arc::new(Mutex::new(None)),
            task_handle: Arc::new(Mutex::new(Some(task_handle))),
            name,
        }
    }

    /// Mark the task as done with a result
    ///
    /// # Arguments
    ///
    /// * `result` - The result value
    pub fn mark_done(&self, result: PyObject) {
        *self.result.lock().unwrap() = Some(result);
        self.done.store(true, Ordering::Release);
    }

    /// Mark the task as done with an exception
    ///
    /// # Arguments
    ///
    /// * `exc` - The exception object
    pub fn mark_exception(&self, exc: PyObject) {
        *self.exception.lock().unwrap() = Some(exc);
        self.done.store(true, Ordering::Release);
    }

    /// Check if the task was cancelled (internal use)
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Check if the task is done (internal Rust API)
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Get the task result (internal Rust API)
    pub fn get_result(&self, py: Python) -> PyResult<PyObject> {
        if !self.is_done() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Task is not done yet"));
        }

        // Check for cancellation first
        if self.is_cancelled() {
            return Err(PyCancelledError::new_err("Task was cancelled"));
        }

        // Check for exception
        if let Some(exc) = self.exception.lock().unwrap().as_ref() {
            let exc_bound = exc.clone_ref(py).into_bound(py);
            return Err(PyErr::from_value(exc_bound));
        }

        // Return result
        Ok(self
            .result
            .lock()
            .unwrap()
            .as_ref()
            .map(|r| r.clone_ref(py))
            .unwrap_or_else(|| py.None()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_result_is_send() {
        // Ensure PollResult can be sent between threads
        fn assert_send<T: Send>() {}
        assert_send::<PollResult>();
    }
}
