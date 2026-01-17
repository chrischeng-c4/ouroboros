//! Core infrastructure for Argus
//!
//! Provides configuration, workspace management, and shared utilities.

mod config;

pub use config::{ArgusConfig, LanguageConfig, LintConfig, PythonConfig, RustConfig, TypeScriptConfig};
