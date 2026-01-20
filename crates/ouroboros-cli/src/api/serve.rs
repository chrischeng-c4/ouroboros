//! `ob api serve` command implementation
//!
//! Start API server (supports both dev and production modes).

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

/// Arguments for `ob api serve`
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Application import path (e.g., "python.app:create_app")
    #[arg(default_value = "")]
    pub app: String,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind to
    #[arg(long, default_value = "8000")]
    pub port: u16,

    /// Enable auto-reload (development mode)
    #[arg(long)]
    pub reload: bool,

    /// Directories to watch for reload (can be used multiple times)
    #[arg(long = "reload-dir")]
    pub reload_dir: Vec<String>,

    /// File patterns to include for reload (e.g., "*.py,*.rs")
    #[arg(long = "reload-include", default_value = "*.py")]
    pub reload_include: String,

    /// File patterns to exclude from reload
    #[arg(long = "reload-exclude", default_value = "__pycache__,*.pyc,.git")]
    pub reload_exclude: String,

    /// Delay between detecting changes and reloading (seconds)
    #[arg(long = "reload-delay", default_value = "0.25")]
    pub reload_delay: f64,

    /// Log level (debug, info, warning, error, critical)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Enable access logging
    #[arg(long)]
    pub access_log: bool,
}

/// Execute the serve command
pub fn execute(args: ServeArgs) -> Result<()> {
    // This function is synchronous because the server runs its own runtime
    crate::run_api_server(
        args.app,
        args.host,
        args.port,
        args.reload,
        args.reload_dir,
        args.reload_include,
        args.reload_exclude,
        args.reload_delay,
        args.log_level,
        args.access_log,
    )
}
