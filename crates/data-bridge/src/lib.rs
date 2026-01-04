//! data-bridge: High-performance data access library for Python
//!
//! This library provides Rust-based implementations for various data sources,
//! with all serialization/deserialization happening in Rust for maximum performance.
//!
//! # Features
//! - MongoDB ORM (Beanie-compatible)
//! - Zero Python byte handling
//! - Full async/await support
//! - Type-safe operations
//!
//! # Usage
//! ```python
//! from data_bridge.mongodb import Document, init
//!
//! # Initialize MongoDB
//! await init("mongodb://localhost:27017/mydb")
//!
//! # Define models
//! class User(Document):
//!     email: str
//!     class Settings:
//!         name = "users"
//!
//! # Use ORM
//! user = await User.find_one(User.email == "test@example.com")
//! ```

use pyo3::prelude::*;

// Security and validation modules
pub mod validation;
pub mod config;
pub mod error_handling;

// BSON conversion with GIL-free processing (Feature 201)
pub mod conversion;

#[cfg(feature = "mongodb")]
mod mongodb;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "test")]
mod test;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "tasks")]
mod tasks;

/// data-bridge Python module
#[pymodule]
fn data_bridge(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Add security configuration functions
    config::register_functions(m)?;

    // Add MongoDB module if enabled
    #[cfg(feature = "mongodb")]
    {
        let mongodb_module = PyModule::new(py, "mongodb")?;
        mongodb::register_module(&mongodb_module)?;
        m.add_submodule(&mongodb_module)?;
    }

    // Add HTTP module if enabled
    #[cfg(feature = "http")]
    {
        let http_module = PyModule::new(py, "http")?;
        http::register_module(&http_module)?;
        m.add_submodule(&http_module)?;
    }

    // Add Test module if enabled
    #[cfg(feature = "test")]
    {
        let test_module = PyModule::new(py, "test")?;
        test::register_module(&test_module)?;
        m.add_submodule(&test_module)?;
    }

    // Add PostgreSQL module if enabled
    #[cfg(feature = "postgres")]
    {
        let postgres_module = PyModule::new(py, "postgres")?;
        postgres::register_module(&postgres_module)?;
        m.add_submodule(&postgres_module)?;
    }

    // Add Tasks module if enabled
    #[cfg(feature = "tasks")]
    {
        let tasks_module = PyModule::new(py, "tasks")?;
        tasks::register_module(&tasks_module)?;
        m.add_submodule(&tasks_module)?;
    }

    Ok(())
}
