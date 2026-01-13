//! PyLoop module registration for data-bridge
//!
//! This module exposes the data-bridge-pyloop crate as a Python submodule.

use pyo3::prelude::*;

/// Register the pyloop module with Python
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Re-export PyLoop from data-bridge-pyloop
    m.add_class::<data_bridge_pyloop::PyLoop>()?;
    m.add_class::<data_bridge_pyloop::PyFuture>()?;
    m.add_class::<data_bridge_pyloop::Handle>()?;
    m.add_class::<data_bridge_pyloop::TimerHandle>()?;
    m.add_class::<data_bridge_pyloop::Task>()?;

    // Register the CancelledError exception
    m.add("CancelledError", m.py().get_type::<data_bridge_pyloop::PyCancelledError>())?;

    // Add module metadata
    m.add("__doc__", "Rust-native Python asyncio event loop backed by Tokio")?;

    Ok(())
}
