//! Panic boundary protection for FFI safety.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::panic::catch_unwind;

/// Safely execute a potentially panicking synchronous operation, converting panics to Python errors.
///
/// This is critical for FFI safety: any Rust panic that crosses the FFI boundary will
/// crash the entire Python process. This wrapper catches panics and converts them to
/// PyRuntimeError exceptions that Python can handle gracefully.
///
/// # Example
/// ```rust
/// #[pyfunction]
/// fn risky_operation(py: Python) -> PyResult<i32> {
///     safe_call(|| {
///         // Code that might panic
///         Ok(42)
///     })
/// }
/// ```
#[allow(dead_code)]
pub(super) fn safe_call<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce() -> PyResult<T> + std::panic::UnwindSafe,
{
    match catch_unwind(f) {
        Ok(result) => result,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                format!("Rust panic in ouroboros-postgres: {}", s)
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                format!("Rust panic in ouroboros-postgres: {}", s)
            } else {
                "Rust panic in ouroboros-postgres: unknown error".to_string()
            };
            Err(PyRuntimeError::new_err(msg))
        }
    }
}
