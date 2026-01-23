//! `ob api core` command implementation
//!
//! Manages core modules (shared, widely-depended modules like auth, users).

use anyhow::Result;
use clap::Args;
use std::fs;

use super::codegen;
use super::config::{find_pyproject, DbType};
use super::fields::{parse_fields, parse_fields_json, FieldDef};
use super::CoreAction;

/// Parse fields from either simple syntax or JSON syntax
fn parse_fields_arg(fields: &Option<String>, fields_json: &Option<String>) -> Result<Vec<FieldDef>> {
    if let Some(ref json) = fields_json {
        parse_fields_json(json)
    } else if let Some(ref simple) = fields {
        parse_fields(simple)
    } else {
        Ok(Vec::new())
    }
}

/// Arguments for `ob api core create`
#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Name of the core module to create
    pub name: String,

    /// Database type override (defaults to project default)
    #[arg(long)]
    pub db: Option<DbType>,
}

/// Arguments for `ob api core model`
#[derive(Debug, Args)]
pub struct ModelArgs {
    /// Name of the core module
    pub module: String,

    /// Name of the model to create
    pub name: String,

    /// Database type override
    #[arg(long)]
    pub db: Option<DbType>,

    /// Field definitions - simple syntax
    #[arg(long)]
    pub fields: Option<String>,

    /// Field definitions - JSON syntax
    #[arg(long, conflicts_with = "fields")]
    pub fields_json: Option<String>,
}

/// Arguments for `ob api core service`
#[derive(Debug, Args)]
pub struct ServiceArgs {
    /// Name of the core module
    pub module: String,

    /// Name of the service to create
    pub name: String,

    /// Database type override
    #[arg(long)]
    pub db: Option<DbType>,

    /// Field definitions - simple syntax
    #[arg(long)]
    pub fields: Option<String>,

    /// Field definitions - JSON syntax
    #[arg(long, conflicts_with = "fields")]
    pub fields_json: Option<String>,
}

/// Arguments for `ob api core route`
#[derive(Debug, Args)]
pub struct RouteArgs {
    /// Name of the core module
    pub module: String,

    /// Target app for the routes
    #[arg(long)]
    pub app: Option<String>,

    /// Model name for route generation
    #[arg(long)]
    pub model: Option<String>,

    /// Field definitions - simple syntax
    #[arg(long)]
    pub fields: Option<String>,

    /// Field definitions - JSON syntax
    #[arg(long, conflicts_with = "fields")]
    pub fields_json: Option<String>,
}

/// Arguments for `ob api core endpoint`
#[derive(Debug, Args)]
pub struct EndpointArgs {
    /// Name of the core module
    pub module: String,

    /// Name of the endpoint to create
    pub name: String,

    /// HTTP method (GET, POST, PUT, DELETE, PATCH)
    #[arg(long, default_value = "GET")]
    pub method: String,

    /// Path for the endpoint (relative to module prefix)
    #[arg(long, default_value = "/")]
    pub path: String,
}

/// Arguments for `ob api core schema`
#[derive(Debug, Args)]
pub struct SchemaArgs {
    /// Name of the core module
    pub module: String,

    /// Name of the schema to create
    pub name: String,

    /// Schema type (request, response, or both)
    #[arg(long, default_value = "both")]
    pub r#type: String,

    /// Field definitions - simple syntax
    #[arg(long)]
    pub fields: Option<String>,

    /// Field definitions - JSON syntax
    #[arg(long, conflicts_with = "fields")]
    pub fields_json: Option<String>,
}

/// Execute a core action
pub async fn execute(action: CoreAction) -> Result<()> {
    match action {
        CoreAction::Create(args) => create(args).await,
        CoreAction::Model(args) => model(args).await,
        CoreAction::Service(args) => service(args).await,
        CoreAction::Route(args) => route(args).await,
        CoreAction::Endpoint(args) => endpoint(args).await,
        CoreAction::Schema(args) => schema(args).await,
        CoreAction::List => list().await,
    }
}

/// Create a new core module
async fn create(args: CreateArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, mut pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.name);
    if module_dir.exists() {
        anyhow::bail!("Core module '{}' already exists", args.name);
    }

    fs::create_dir_all(&module_dir)?;

    // Create __init__.py
    let init_content = format!(
        r#""""
{} core module.

This is a core module that may be depended on by multiple features.
Keep it stable and well-documented.
"""
from .models import *
from .schemas import *
from .services import *

__all__ = []  # Explicitly export public API
"#,
        args.name
    );
    fs::write(module_dir.join("__init__.py"), init_content)?;

    // Create empty models.py
    let db_type = args.db.unwrap_or(pyproject.ouroboros().default_db);
    let models_content = match db_type {
        DbType::Pg => format!(
            r#""""
{} models (PostgreSQL).

Uses ouroboros.pg for ORM functionality with:
- Async connection pooling
- Transaction support with savepoints
- Automatic migration generation
- Query builder with joins, CTEs, window functions
"""
from datetime import datetime
from typing import Optional
from uuid import UUID

from ouroboros.pg import Model, Field

# Add your models here
"#,
            args.name
        ),
        DbType::Mongo => format!(
            r#""""
{} models (MongoDB).

Uses ouroboros.mongo for ODM functionality with:
- Async motor driver
- Schema validation
- Index management
- Aggregation pipeline support
"""
from datetime import datetime
from typing import Optional
from bson import ObjectId

from ouroboros.mongo import Document, Field

# Add your models here
"#,
            args.name
        ),
    };
    fs::write(module_dir.join("models.py"), models_content)?;

    // Create empty schemas.py
    let schemas_content = format!(
        r#""""
{} schemas (Pydantic).

Request/response schemas for API validation.

Uses ouroboros.validation for:
- Field validation (length, range, pattern, email, url)
- Model validators (before/after)
- Field validators
- Computed fields
- Serialization options
"""
from datetime import datetime
from typing import Optional, Literal

from ouroboros.validation import (
    Schema,
    Field,
    field_validator,
    model_validator,
    computed_field,
    EmailStr,
    HttpUrl,
    ConfigDict,
)

# Add your schemas here
"#,
        args.name
    );
    fs::write(module_dir.join("schemas.py"), schemas_content)?;

    // Create empty services.py
    let services_content = format!(
        r#""""
{} services.

Business logic for the {} module.
"""

# Add your services here
"#,
        args.name, args.name
    );
    fs::write(module_dir.join("services.py"), services_content)?;

    // Update pyproject.toml
    pyproject.ouroboros_mut().add_core(&args.name, args.db);
    pyproject.save(&pyproject_path)?;

    println!("Created core module: {}", args.name);
    println!("  Directory: {}", module_dir.display());
    println!("  Database: {}", db_type);
    println!("\nNext steps:");
    println!("  1. Add models: ob api core model {} User", args.name);
    println!("  2. Add schemas: ob api core schema {} User", args.name);
    println!("  3. Add services: ob api core service {} user", args.name);

    Ok(())
}

/// Add a model to a core module
async fn model(args: ModelArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.module);
    if !module_dir.exists() {
        anyhow::bail!("Core module '{}' not found. Create it first with: ob api core create {}", args.module, args.module);
    }

    let models_path = module_dir.join("models.py");
    let db_type = args.db.unwrap_or_else(|| pyproject.ouroboros().get_db_for_module(&args.module, true));

    // Parse fields (simple or JSON syntax)
    let fields = parse_fields_arg(&args.fields, &args.fields_json)?;

    let model_code = codegen::generate_model_code(&args.name, &fields, db_type);

    // Append to models.py
    let mut content = fs::read_to_string(&models_path).unwrap_or_default();
    content.push_str("\n");
    content.push_str(&model_code);
    fs::write(&models_path, content)?;

    println!("Added model '{}' to core/{}/models.py", args.name, args.module);
    println!("  Database: {}", db_type);
    if !fields.is_empty() {
        println!("  Fields: {}", fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "));
    }

    Ok(())
}

/// Add a service to a core module
async fn service(args: ServiceArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.module);
    if !module_dir.exists() {
        anyhow::bail!("Core module '{}' not found", args.module);
    }

    let services_path = module_dir.join("services.py");
    let db_type = args.db.unwrap_or_else(|| pyproject.ouroboros().get_db_for_module(&args.module, true));

    // Parse fields (simple or JSON syntax)
    let fields = parse_fields_arg(&args.fields, &args.fields_json)?;

    let service_code = codegen::generate_service_code(&args.name, &fields, db_type);

    let mut content = fs::read_to_string(&services_path).unwrap_or_default();
    content.push_str("\n");
    content.push_str(&service_code);
    fs::write(&services_path, content)?;

    println!("Added service '{}' to core/{}/services.py", args.name, args.module);
    println!("  Database: {}", db_type);
    if !fields.is_empty() {
        println!("  Fields: {}", fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "));
    }

    Ok(())
}

/// Initialize routes for a core module
async fn route(args: RouteArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, _pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.module);
    if !module_dir.exists() {
        anyhow::bail!("Core module '{}' not found", args.module);
    }

    let routes_path = module_dir.join("routes.py");
    let routes_exist = routes_path.exists();

    // If routes already exist, just register to app (if specified)
    if routes_exist {
        if let Some(ref app_name) = args.app {
            let app_path = project_root.join("apps").join(app_name).join("app.py");
            if app_path.exists() {
                codegen::register_router_in_app(&app_path, &args.module, true)?;
                println!("Registered core/{} router in apps/{}/app.py", args.module, app_name);
            } else {
                println!("App '{}' not found. Create it first with: ob api app create {}", app_name, app_name);
            }
        } else {
            println!("Routes already exist for core/{}.", args.module);
            println!("  - To register in an app: ob api core route {} --app <app_name>", args.module);
            println!("  - To add endpoints: ob api core endpoint {} <name>", args.module);
        }
        return Ok(());
    }

    // Parse fields (simple or JSON syntax)
    let fields = parse_fields_arg(&args.fields, &args.fields_json)?;

    // Use model name if provided, otherwise use module name (PascalCase)
    let model_name = args.model.unwrap_or_else(|| codegen::to_pascal_case(&args.module));

    // Generate endpoints.py (SSOT) - always generated
    let endpoints_path = module_dir.join("endpoints.py");
    let endpoints_code = codegen::generate_endpoints_code(&args.module, &model_name);
    fs::write(&endpoints_path, endpoints_code)?;
    println!("  Created endpoints.py (SSOT)");

    // Generate routes.py referencing endpoints.py
    let routes_code = if args.app.is_some() {
        // Generate routes that reference endpoints.py
        codegen::generate_routes_code(&args.module, &model_name, &fields)
    } else {
        // Standalone routes (no RouteConfig reference)
        generate_routes_code(&args.module)
    };
    fs::write(&routes_path, routes_code)?;

    // Update __init__.py to export router
    let init_path = module_dir.join("__init__.py");
    let mut init_content = fs::read_to_string(&init_path)?;
    if !init_content.contains("from .routes import") {
        init_content.push_str("\nfrom .routes import router\n");
        init_content = init_content.replace("__all__ = []", "__all__ = [\"router\"]");
        fs::write(&init_path, init_content)?;
    }

    println!("Created routes for core/{}", args.module);
    println!("  File: {}", routes_path.display());

    // Auto-register with app.py if --app is specified
    if let Some(ref app) = args.app {
        let app_path = project_root.join("apps").join(app).join("app.py");
        if app_path.exists() {
            codegen::register_router_in_app(&app_path, &args.module, true)?;
            println!("  Registered in apps/{}/app.py", app);
        } else {
            println!("\nAdd to apps/{}/app.py:", app);
            println!("  from core.{}.routes import router as {}_router", args.module, args.module);
            println!("  app.include_router({}_router, prefix=\"/{}\", tags=[\"{}\"])", args.module, args.module, args.module);
        }
    }

    Ok(())
}

/// Generate standalone routes code (when --app not specified)
fn generate_routes_code(module: &str) -> String {
    format!(
        r#""""
{module} routes.

API endpoints for the {module} core module.

Note: Use --app to enable RouteConfig SSOT pattern.
"""
from ouroboros.api import Router, Path, Query, HTTPException

router = Router(prefix="/{module}", tags=["{module}"])


@router.route("GET", "/")
async def list_{module}s():
    """List all {module}s."""
    return []


@router.route("GET", "/{{id}}")
async def get_{module}(id: int = Path()):
    """Get a {module} by ID."""
    raise HTTPException(404, "{module} not found")


@router.route("POST", "/", status_code=201)
async def create_{module}():
    """Create a new {module}."""
    return {{"id": 1}}


@router.route("PUT", "/{{id}}")
async def update_{module}(id: int = Path()):
    """Update a {module}."""
    return {{"id": id}}


@router.route("DELETE", "/{{id}}", status_code=204)
async def delete_{module}(id: int = Path()):
    """Delete a {module}."""
    pass
"#,
        module = module
    )
}

/// Add an endpoint to a core module's routes
async fn endpoint(args: EndpointArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, _pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.module);
    if !module_dir.exists() {
        anyhow::bail!("Core module '{}' not found", args.module);
    }

    let routes_path = module_dir.join("routes.py");
    if !routes_path.exists() {
        anyhow::bail!("Routes not initialized. Run: ob api core route {}", args.module);
    }

    // Check if endpoint already exists
    if codegen::check_endpoint_exists(&routes_path, &args.method, &args.path)? {
        println!("  ⚠ Endpoint {} {} already exists, skipping", args.method.to_uppercase(), args.path);
        return Ok(());
    }

    let endpoint_code = generate_endpoint_code(&args.name, &args.method, &args.path);

    let mut content = fs::read_to_string(&routes_path)?;
    content.push_str("\n\n");
    content.push_str(&endpoint_code);
    fs::write(&routes_path, content)?;

    println!("Added endpoint '{}' to core/{}/routes.py", args.name, args.module);
    println!("  {} {}", args.method.to_uppercase(), args.path);

    Ok(())
}

/// Generate endpoint code
fn generate_endpoint_code(name: &str, method: &str, path: &str) -> String {
    let method_lower = method.to_lowercase();
    let decorator = match method_lower.as_str() {
        "post" => format!("@router.post(\"{}\", status_code=status.HTTP_201_CREATED)", path),
        "delete" => format!("@router.delete(\"{}\", status_code=status.HTTP_204_NO_CONTENT)", path),
        _ => format!("@router.{}(\"{}\")", method_lower, path),
    };

    format!(
        r#"{decorator}
async def {name}():
    """{name} endpoint."""
    pass
"#,
        decorator = decorator,
        name = to_snake_case(name)
    )
}

/// Add a schema to a core module
async fn schema(args: SchemaArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, _pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.module);
    if !module_dir.exists() {
        anyhow::bail!("Core module '{}' not found", args.module);
    }

    let schemas_path = module_dir.join("schemas.py");

    // Parse fields (simple or JSON syntax)
    let fields = parse_fields_arg(&args.fields, &args.fields_json)?;

    let schema_code = codegen::generate_schema_code(&args.name, &fields);

    let mut content = fs::read_to_string(&schemas_path).unwrap_or_default();
    content.push_str("\n");
    content.push_str(&schema_code);
    fs::write(&schemas_path, content)?;

    println!("Added schema '{}' to core/{}/schemas.py", args.name, args.module);
    if !fields.is_empty() {
        println!("  Fields: {}", fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "));
    }

    Ok(())
}

/// List all core modules
async fn list() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let core_dir = project_root.join("core");
    if !core_dir.exists() {
        println!("No core directory found. Run 'ob api init' first.");
        return Ok(());
    }

    println!("Core modules:");
    let config = pyproject.ouroboros();

    for entry in fs::read_dir(&core_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with('_') {
                continue;
            }
            let module_config = config.core.get(name.as_ref());
            let db = module_config.and_then(|m| m.db).unwrap_or(config.default_db);
            println!("  {} (db: {})", name, db);
        }
    }

    Ok(())
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
