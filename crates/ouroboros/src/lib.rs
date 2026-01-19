//! ouroboros: High-performance Rust-powered Python platform
//!
//! This library provides Rust-based implementations for various data sources,
//! with all serialization/deserialization happening in Rust for maximum performance.
//!
//! # Features
//! - MongoDB ORM (Beanie-compatible)
//! - PostgreSQL ORM
//! - HTTP Client
//! - Task Queue
//! - KV Store
//! - Zero Python byte handling
//! - Full async/await support
//!
//! # Usage
//! ```python
//! import ouroboros as ob
//! from ouroboros.mongodb import Document, init
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

// Security and configuration modules
pub mod config;
pub mod error_handling;

// Core types (ObjectId, etc.)
pub mod types;

// BSON conversion with GIL-free processing (Feature 201)
pub mod conversion;

// Validation module (Pydantic-like validation with Rust performance)
pub mod validation;

#[cfg(feature = "mongodb")]
mod mongodb;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "qc")]
mod qc;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "kv")]
mod kv;

#[cfg(feature = "api")]
mod api;

#[cfg(feature = "tasks")]
mod tasks;

#[cfg(feature = "pyloop")]
mod pyloop;

/// ouroboros Python module
#[pymodule]
fn ouroboros(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Add core types (ObjectId) at top level
    types::register_module(m)?;

    // Add security configuration functions
    config::register_functions(m)?;

    // Add validation module (Pydantic-like validation)
    let validation_module = PyModule::new(py, "validation")?;
    validation::register_module(&validation_module)?;
    m.add_submodule(&validation_module)?;

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
    #[cfg(feature = "qc")]
    {
        let test_module = PyModule::new(py, "qc")?;
        qc::register_module(&test_module)?;
        m.add_submodule(&test_module)?;
    }

    // Add PostgreSQL module if enabled
    #[cfg(feature = "postgres")]
    {
        let postgres_module = PyModule::new(py, "postgres")?;
        postgres::register_module(&postgres_module)?;
        m.add_submodule(&postgres_module)?;
    }

    // Add KV module if enabled
    #[cfg(feature = "kv")]
    {
        let kv_module = PyModule::new(py, "kv")?;
        kv::register_module(&kv_module)?;
        m.add_submodule(&kv_module)?;
    }

    // Add API module if enabled
    #[cfg(feature = "api")]
    {
        let api_module = PyModule::new(py, "api")?;
        api::register_module(&api_module)?;
        m.add_submodule(&api_module)?;
    }

    // Add Tasks module if enabled
    #[cfg(feature = "tasks")]
    {
        let tasks_module = PyModule::new(py, "tasks")?;
        tasks::register_module(&tasks_module)?;
        m.add_submodule(&tasks_module)?;
    }

    // Add PyLoop module if enabled
    #[cfg(feature = "pyloop")]
    {
        let pyloop_module = PyModule::new(py, "_pyloop")?;
        pyloop::register_module(&pyloop_module)?;
        m.add_submodule(&pyloop_module)?;
    }

    Ok(())
}
