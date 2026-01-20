//! `ob api app` command implementation
//!
//! Manages API applications (entry points).

use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::Path;

use super::config::{find_pyproject, PyProject};
use super::AppAction;

/// Arguments for `ob api app create`
#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Name of the app to create
    pub name: String,

    /// Port for the app
    #[arg(long)]
    pub port: Option<u16>,

    /// Description of the app
    #[arg(long)]
    pub description: Option<String>,
}

/// Execute an app action
pub async fn execute(action: AppAction) -> Result<()> {
    match action {
        AppAction::Create(args) => create(args).await,
        AppAction::List => list().await,
    }
}

/// Create a new app
async fn create(args: CreateArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // Find and load pyproject.toml
    let (pyproject_path, mut pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    // Create app directory
    let app_dir = project_root.join("apps").join(&args.name);
    if app_dir.exists() {
        anyhow::bail!("App '{}' already exists at {}", args.name, app_dir.display());
    }

    fs::create_dir_all(&app_dir)
        .with_context(|| format!("Failed to create {}", app_dir.display()))?;

    // Create __init__.py
    let init_content = format!(
        r#""""
{} API application.
{}
"""
from .app import app

__all__ = ["app"]
"#,
        args.name,
        args.description.as_deref().unwrap_or("")
    );
    fs::write(app_dir.join("__init__.py"), init_content)?;

    // Create app.py
    let port = args.port.unwrap_or(8000);
    let app_content = format!(
        r#""""
FastAPI application configuration for {}.
"""
from fastapi import FastAPI

app = FastAPI(
    title="{} API",
    description="{}",
    version="0.1.0",
)

# Health check routes (if infra module exists)
try:
    from infra import health_router, metrics_router
    app.include_router(health_router, prefix="/health", tags=["health"])
    app.include_router(metrics_router, prefix="/metrics", tags=["metrics"])
except ImportError:
    pass

# Add feature/core routers here:
# from features.orders.routes import router as orders_router
# app.include_router(orders_router, prefix="/orders", tags=["orders"])


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port={})
"#,
        args.name,
        args.name,
        args.description.as_deref().unwrap_or(&format!("{} API", args.name)),
        port
    );
    fs::write(app_dir.join("app.py"), app_content)?;

    // Create routes.py (RouteConfig SSOT)
    let routes_content = format!(
        r#""""
Route configuration for {} app.

This file serves as the Single Source of Truth (SSOT) for all routes
registered in this app. Import this to understand the full API surface.
"""
from dataclasses import dataclass, field
from typing import List, Optional


@dataclass
class EndpointConfig:
    """Configuration for a single endpoint."""
    path: str
    method: str
    handler: str
    summary: Optional[str] = None
    tags: List[str] = field(default_factory=list)


@dataclass
class RouteConfig:
    """Configuration for a route group."""
    prefix: str
    module: str
    endpoints: List[EndpointConfig] = field(default_factory=list)
    tags: List[str] = field(default_factory=list)


# Route registry - add routes here
ROUTES: List[RouteConfig] = [
    # Example:
    # RouteConfig(
    #     prefix="/orders",
    #     module="features.orders.routes",
    #     tags=["orders"],
    #     endpoints=[
    #         EndpointConfig("/", "GET", "list_orders", "List all orders"),
    #         EndpointConfig("/", "POST", "create_order", "Create a new order"),
    #         EndpointConfig("/{{id}}", "GET", "get_order", "Get order by ID"),
    #     ],
    # ),
]
"#,
        args.name
    );
    fs::write(app_dir.join("routes.py"), routes_content)?;

    // Update pyproject.toml
    pyproject.ouroboros_mut().add_app(&args.name, args.port, args.description);
    pyproject.save(&pyproject_path)?;

    println!("Created app: {}", args.name);
    println!("  Directory: {}", app_dir.display());
    println!("  Port: {}", port);
    println!("\nNext steps:");
    println!("  1. Add routes to apps/{}/routes.py", args.name);
    println!("  2. Include feature routers in apps/{}/app.py", args.name);
    println!("  3. Run: uvicorn apps.{}.app:app --reload", args.name);

    Ok(())
}

/// List all apps
async fn list() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let apps_dir = project_root.join("apps");
    if !apps_dir.exists() {
        println!("No apps directory found. Run 'ob api init' first.");
        return Ok(());
    }

    println!("Apps:");
    let config = pyproject.ouroboros();

    for entry in fs::read_dir(&apps_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with('_') {
                continue;
            }
            let app_config = config.apps.get(name.as_ref());
            let port = app_config.and_then(|a| a.port).map(|p| format!(":{}", p)).unwrap_or_default();
            let desc = app_config.and_then(|a| a.description.as_ref()).map(|d| format!(" - {}", d)).unwrap_or_default();
            println!("  {} {}{}", name, port, desc);
        }
    }

    Ok(())
}
