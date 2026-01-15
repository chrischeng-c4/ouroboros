//! Error types for ouroboros-pyloop

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use thiserror::Error;

/// Errors that can occur in PyLoop operations
#[derive(Debug, Error)]
pub enum PyLoopError {
    /// Failed to initialize Tokio runtime
    #[error("Failed to initialize Tokio runtime: {0}")]
    RuntimeInit(String),

    /// Failed to spawn task on runtime
    #[error("Failed to spawn task: {0}")]
    TaskSpawn(String),

    /// Failed to execute future
    #[error("Failed to execute future: {0}")]
    FutureExecution(String),

    /// Invalid event loop state
    #[error("Invalid event loop state: {0}")]
    InvalidState(String),

    /// Python exception during execution
    #[error("Python exception: {0}")]
    PythonException(String),
}

impl From<PyLoopError> for PyErr {
    fn from(err: PyLoopError) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}
