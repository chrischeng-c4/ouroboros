//! PyLoop module registration for ouroboros
//!
//! This module exposes the ouroboros-pyloop crate as a Python submodule.

use pyo3::prelude::*;

/// Register the pyloop module with Python
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Re-export PyLoop from ouroboros-pyloop
    m.add_class::<ouroboros_pyloop::PyLoop>()?;
    m.add_class::<ouroboros_pyloop::PyFuture>()?;
    m.add_class::<ouroboros_pyloop::Handle>()?;
    m.add_class::<ouroboros_pyloop::TimerHandle>()?;
    m.add_class::<ouroboros_pyloop::Task>()?;

    // Register the CancelledError exception
    m.add("CancelledError", m.py().get_type::<ouroboros_pyloop::PyCancelledError>())?;

    // Add module metadata
    m.add("__doc__", "Rust-native Python asyncio event loop backed by Tokio")?;

    Ok(())
}
