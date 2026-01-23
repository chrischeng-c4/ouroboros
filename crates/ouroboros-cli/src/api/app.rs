//! `ob api app` command implementation
//!
//! Manages API applications (entry points).

use anyhow::{Context, Result};
use clap::Args;
use std::fs;

use super::config::find_pyproject;
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
{name} API application.
"""
from ouroboros.api import App

app = App(
    title="{name} API",
    description="{description}",
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
    import asyncio
    asyncio.run(app.serve(host="0.0.0.0", port={port}))
"#,
        name = args.name,
        description = args.description.as_deref().unwrap_or(&format!("{} API", args.name)),
        port = port
    );
    fs::write(app_dir.join("app.py"), app_content)?;

    // Create routes.py (RouteConfig SSOT)
    let routes_content = format!(
        r#""""
Route configuration for {name} app.

This file serves as the Single Source of Truth (SSOT) for all routes.
Handlers and tests import from here to ensure consistency.

Usage:
    from apps.{name}.routes import TodoRoutes as R

    # In handler:
    @router.route(R.LIST.method, R.LIST.path)
    async def list_todos(): ...

    # In test:
    response = await server.request(R.LIST.method, f"{{R.PREFIX}}{{R.LIST.path}}")
"""
from dataclasses import dataclass


@dataclass(frozen=True)
class Endpoint:
    """Single endpoint configuration."""
    path: str
    method: str
    handler: str
    summary: str = ""


# Module route classes are added here by `ob api feat route`
# Example:
#
# class TodoRoutes:
#     """Routes for todo module."""
#     PREFIX = "/todo"
#
#     LIST = Endpoint("/", "GET", "list_todos", "List all todos")
#     CREATE = Endpoint("/", "POST", "create_todo", "Create a todo")
#     GET = Endpoint("/{{id}}", "GET", "get_todo", "Get todo by ID")
#     UPDATE = Endpoint("/{{id}}", "PUT", "update_todo", "Update a todo")
#     DELETE = Endpoint("/{{id}}", "DELETE", "delete_todo", "Delete a todo")
"#,
        name = args.name
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
