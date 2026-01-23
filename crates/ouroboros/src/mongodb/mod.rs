//! MongoDB module for Python bindings
//!
//! This module provides Python bindings for MongoDB operations using PyO3.
//! All BSON serialization/deserialization happens in Rust for maximum performance.

use pyo3::prelude::*;

// Sub-modules
pub mod types;
pub mod connection;
pub mod conversion;
pub mod document;
pub mod validation;

// Re-export for external use
pub use document::RustDocument;

/// Register the mongodb module
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(connection::init, m)?)?;
    m.add_function(wrap_pyfunction!(connection::is_connected, m)?)?;
    m.add_function(wrap_pyfunction!(connection::close, m)?)?;
    m.add_function(wrap_pyfunction!(connection::reset, m)?)?;
    m.add_function(wrap_pyfunction!(connection::available_features, m)?)?;
    m.add_class::<RustDocument>()?;

    // Add module docstring
    m.add("__doc__", "MongoDB ORM module with Beanie compatibility")?;

    Ok(())
}
