//! Agent framework PyO3 bindings
//!
//! Provides Python bindings for the ouroboros-agent framework,
//! including LLM providers, tools, and agent execution.

use pyo3::prelude::*;

mod py_agent;
mod py_llm;
mod py_tools;
mod utils;

pub use py_agent::PyAgent;
pub use py_llm::PyOpenAI;
pub use py_tools::{PyTool, PyToolRegistry};

/// Register the agent module with Python
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register LLM providers
    m.add_class::<PyOpenAI>()?;

    // Register tools
    m.add_class::<PyTool>()?;
    m.add_class::<PyToolRegistry>()?;

    // Register agent
    m.add_class::<PyAgent>()?;

    // Add helper functions
    m.add_function(wrap_pyfunction!(py_tools::get_global_registry, m)?)?;

    Ok(())
}
