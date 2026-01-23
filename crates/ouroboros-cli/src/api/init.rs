//! `ob api init` command implementation
//!
//! Initializes a new ouroboros API project with:
//! - pyproject.toml with [tool.ouroboros] configuration
//! - Standard directory structure (apps/, core/, features/, shared/, infra/)
//! - Infrastructure files (configs.py, dependencies.py, lifespans.py, apps.py)
//! - Optional Kubernetes probe endpoints and metrics

use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::Path;

use super::codegen;
use super::config::{DbType, PyProject};

/// Arguments for `ob api init`
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Project name (defaults to current directory name)
    #[arg(long)]
    pub name: Option<String>,

    /// Database type to use
    #[arg(long, default_value = "pg")]
    pub db: DbType,

    /// Initialize with minimal structure (no k8s probes/metrics)
    #[arg(long)]
    pub minimal: bool,

    /// Initialize with full structure (k8s probes, metrics, sample app)
    #[arg(long)]
    pub full: bool,

    /// Skip creating pyproject.toml if it exists
    #[arg(long)]
    pub no_overwrite: bool,

    /// API services to create (comma-separated, e.g., "admin,inbox,task")
    #[arg(long, value_delimiter = ',')]
    pub services: Option<Vec<String>>,
}

/// Execute the init command
pub async fn execute(args: InitArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let project_name = args.name.unwrap_or_else(|| {
        current_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "myproject".to_string())
    });

    println!("Initializing ouroboros API project: {}", project_name);
    println!("Database: {}", args.db);

    // Create or update pyproject.toml
    let pyproject_path = current_dir.join("pyproject.toml");
    if pyproject_path.exists() && args.no_overwrite {
        println!("pyproject.toml exists, skipping (--no-overwrite)");
    } else {
        let pyproject = PyProject::new(&project_name, args.db);
        pyproject.save(&pyproject_path)?;
        println!("Created pyproject.toml");
    }

    // Create directory structure
    create_directory_structure(&current_dir, args.minimal, args.full)?;

    // Create shared __init__.py
    create_shared_init(&current_dir)?;

    // Create infrastructure files (configs.py, dependencies.py, lifespans.py, apps.py)
    let services = args.services.unwrap_or_else(|| vec!["admin".to_string()]);
    create_infrastructure_files(&current_dir, &project_name, args.db, &services)?;

    if !args.minimal {
        // Create infra module with k8s probes and metrics
        create_infra_module(&current_dir, args.db)?;
    }

    if args.full {
        // Create sample app
        create_sample_app(&current_dir, &project_name)?;
    }

    println!("\nProject initialized successfully!");
    println!("\nNext steps:");
    println!("  1. Update pyproject.toml with your database URL");
    println!("  2. Create an app: ob api app create main");
    println!("  3. Create a feature: ob api feat create orders");
    println!("  4. Add models: ob api feat model orders Order");

    Ok(())
}

/// Create the standard directory structure
fn create_directory_structure(base: &Path, minimal: bool, full: bool) -> Result<()> {
    let dirs = if minimal {
        vec!["apps", "core", "features", "shared"]
    } else {
        vec!["apps", "core", "features", "shared", "infra", "migrations"]
    };

    for dir in dirs {
        let path = base.join(dir);
        if !path.exists() {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create {}", path.display()))?;
            // Create __init__.py
            fs::write(path.join("__init__.py"), "")?;
            println!("Created {}/", dir);
        }
    }

    Ok(())
}

/// Create shared/__init__.py with common imports
fn create_shared_init(base: &Path) -> Result<()> {
    let shared_init = base.join("shared/__init__.py");
    if !shared_init.exists() || fs::read_to_string(&shared_init)?.is_empty() {
        let content = r#""""
Shared utilities and configurations.

This module contains:
- Base configurations
- Common utilities
- Shared types and constants
"""
from typing import TypeVar, Generic

# Type variable for generic repository pattern
T = TypeVar("T")
"#;
        fs::write(&shared_init, content)?;
    }
    Ok(())
}

/// Create infrastructure files (configs.py, dependencies.py, lifespans.py, apps.py)
fn create_infrastructure_files(
    base: &Path,
    project_name: &str,
    db_type: DbType,
    services: &[String],
) -> Result<()> {
    let shared_dir = base.join("shared");
    fs::create_dir_all(&shared_dir)?;

    // configs.py
    let configs_path = shared_dir.join("configs.py");
    if !configs_path.exists() {
        let content = codegen::generate_configs_code(project_name, db_type);
        fs::write(&configs_path, content)?;
        println!("Created shared/configs.py");
    }

    // dependencies.py
    let deps_path = shared_dir.join("dependencies.py");
    if !deps_path.exists() {
        let content = codegen::generate_dependencies_code(db_type);
        fs::write(&deps_path, content)?;
        println!("Created shared/dependencies.py");
    }

    // lifespans.py
    let lifespans_path = shared_dir.join("lifespans.py");
    if !lifespans_path.exists() {
        let content = codegen::generate_lifespans_code(db_type);
        fs::write(&lifespans_path, content)?;
        println!("Created shared/lifespans.py");
    }

    // apps.py (multi-service factory)
    let apps_path = shared_dir.join("apps.py");
    if !apps_path.exists() {
        let service_refs: Vec<&str> = services.iter().map(|s| s.as_str()).collect();
        let content = codegen::generate_apps_code(project_name, &service_refs);
        fs::write(&apps_path, content)?;
        println!("Created shared/apps.py with services: {:?}", services);
    }

    // constants.py
    let constants_path = shared_dir.join("constants.py");
    if !constants_path.exists() {
        let content = codegen::generate_constants_code();
        fs::write(&constants_path, content)?;
        println!("Created shared/constants.py");
    }

    Ok(())
}

/// Create infra module with k8s probes and metrics
fn create_infra_module(base: &Path, db_type: DbType) -> Result<()> {
    let infra_dir = base.join("infra");
    fs::create_dir_all(&infra_dir)?;

    // Create __init__.py
    let init_content = r#""""
Infrastructure module.

Contains:
- Health check endpoints (liveness, readiness)
- Metrics endpoint
- Database connection management
"""
from .health import router as health_router
from .metrics import router as metrics_router

__all__ = ["health_router", "metrics_router"]
"#;
    fs::write(infra_dir.join("__init__.py"), init_content)?;

    // Create health.py with k8s probes
    let health_content = match db_type {
        DbType::Pg => include_str!("templates/infra_health_pg.py.tmpl"),
        DbType::Mongo => include_str!("templates/infra_health_mongo.py.tmpl"),
    };
    fs::write(infra_dir.join("health.py"), health_content)?;

    // Create metrics.py
    let metrics_content = include_str!("templates/infra_metrics.py.tmpl");
    fs::write(infra_dir.join("metrics.py"), metrics_content)?;

    println!("Created infra/ with health checks and metrics");
    Ok(())
}

/// Create a sample app for --full initialization
fn create_sample_app(base: &Path, project_name: &str) -> Result<()> {
    let apps_dir = base.join("apps");
    let main_app = apps_dir.join("main");
    fs::create_dir_all(&main_app)?;

    // Create __init__.py
    let init_content = format!(
        r#""""
Main API application.

This is the primary entry point for the {} API.
"""
from .app import app

__all__ = ["app"]
"#,
        project_name
    );
    fs::write(main_app.join("__init__.py"), init_content)?;

    // Create app.py
    let app_content = format!(
        r#""""
FastAPI application configuration.
"""
from fastapi import FastAPI
from infra import health_router, metrics_router

app = FastAPI(
    title="{} API",
    description="API for {}",
    version="0.1.0",
)

# Infrastructure routes
app.include_router(health_router, prefix="/health", tags=["health"])
app.include_router(metrics_router, prefix="/metrics", tags=["metrics"])

# Add feature routers here:
# from features.orders.routes import router as orders_router
# app.include_router(orders_router, prefix="/orders", tags=["orders"])
"#,
        project_name, project_name
    );
    fs::write(main_app.join("app.py"), app_content)?;

    println!("Created apps/main/ with sample FastAPI app");
    Ok(())
}
