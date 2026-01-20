//! `ob api core` command implementation
//!
//! Manages core modules (shared, widely-depended modules like auth, users).

use anyhow::{Context, Result};
use clap::Args;
use std::fs;

use super::config::{find_pyproject, DbType};
use super::CoreAction;

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
}

/// Arguments for `ob api core service`
#[derive(Debug, Args)]
pub struct ServiceArgs {
    /// Name of the core module
    pub module: String,

    /// Name of the service to create
    pub name: String,
}

/// Arguments for `ob api core route`
#[derive(Debug, Args)]
pub struct RouteArgs {
    /// Name of the core module
    pub module: String,

    /// Target app for the routes
    #[arg(long)]
    pub app: Option<String>,
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

    let model_code = generate_model_code(&args.name, db_type);

    // Append to models.py
    let mut content = fs::read_to_string(&models_path).unwrap_or_default();
    content.push_str("\n\n");
    content.push_str(&model_code);
    fs::write(&models_path, content)?;

    println!("Added model '{}' to core/{}/models.py", args.name, args.module);
    println!("  Database: {}", db_type);

    Ok(())
}

/// Generate model code based on database type
fn generate_model_code(name: &str, db_type: DbType) -> String {
    let table_name = to_snake_case(name);
    match db_type {
        DbType::Pg => format!(
            r#"
class {name}(Model):
    """
    {name} model.

    PostgreSQL table: {table_name}s

    Field types mapping (Python -> PostgreSQL):
      - int -> INTEGER/BIGINT
      - str -> VARCHAR/TEXT
      - float -> DOUBLE PRECISION
      - bool -> BOOLEAN
      - datetime -> TIMESTAMPTZ
      - date -> DATE
      - UUID -> UUID
      - dict -> JSONB
      - list -> ARRAY
    """
    __tablename__ = "{table_name}s"

    # Primary key (auto-increment)
    id: int = Field(primary_key=True, column_type="BIGSERIAL")

    # Timestamps
    created_at: datetime = Field(
        default_factory=datetime.utcnow,
        column_type="TIMESTAMPTZ",
        index=True,
    )
    updated_at: datetime = Field(
        default_factory=datetime.utcnow,
        column_type="TIMESTAMPTZ",
        onupdate=datetime.utcnow,
    )

    # Add your fields here, examples:
    # name: str = Field(max_length=255, nullable=False)
    # email: str = Field(max_length=255, unique=True, nullable=False)
    # status: str = Field(default="active", column_type="VARCHAR(50)")
    # amount: float = Field(default=0.0, column_type="NUMERIC(10,2)")
    # metadata: dict = Field(default_factory=dict, column_type="JSONB")
    # user_id: int = Field(foreign_key="users.id", ondelete="CASCADE")
"#,
            name = name,
            table_name = table_name
        ),
        DbType::Mongo => format!(
            r#"
class {name}(Document):
    """
    {name} document.

    MongoDB collection: {table_name}s

    Field types mapping (Python -> MongoDB):
      - int -> int32/int64
      - str -> string
      - float -> double
      - bool -> bool
      - datetime -> date
      - dict -> object
      - list -> array
      - ObjectId -> objectId
    """
    __collection__ = "{table_name}s"

    # Timestamps
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)

    # Add your fields here, examples:
    # name: str = Field(max_length=255, required=True)
    # email: str = Field(max_length=255, unique=True, required=True)
    # status: str = Field(default="active")
    # amount: float = Field(default=0.0)
    # tags: list[str] = Field(default_factory=list)
    # metadata: dict = Field(default_factory=dict)
"#,
            name = name,
            table_name = table_name
        ),
    }
}

/// Add a service to a core module
async fn service(args: ServiceArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, _pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    let module_dir = project_root.join("core").join(&args.module);
    if !module_dir.exists() {
        anyhow::bail!("Core module '{}' not found", args.module);
    }

    let services_path = module_dir.join("services.py");
    let service_code = generate_service_code(&args.name, &args.module);

    let mut content = fs::read_to_string(&services_path).unwrap_or_default();
    content.push_str("\n\n");
    content.push_str(&service_code);
    fs::write(&services_path, content)?;

    println!("Added service '{}' to core/{}/services.py", args.name, args.module);

    Ok(())
}

/// Generate service code
fn generate_service_code(name: &str, _module: &str) -> String {
    let class_name = to_pascal_case(name);
    format!(
        r#"
class {class_name}Service:
    """
    Service for {name} operations.
    """

    async def get(self, id: int):
        """Get a {name} by ID."""
        pass

    async def list(self, skip: int = 0, limit: int = 100):
        """List all {name}s with pagination."""
        pass

    async def create(self, data: dict):
        """Create a new {name}."""
        pass

    async def update(self, id: int, data: dict):
        """Update an existing {name}."""
        pass

    async def delete(self, id: int):
        """Delete a {name}."""
        pass


{name}_service = {class_name}Service()
"#,
        class_name = class_name,
        name = name
    )
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
    if routes_path.exists() {
        anyhow::bail!("Routes already exist for core/{}. Use 'ob api core endpoint' to add endpoints.", args.module);
    }

    let routes_code = generate_routes_code(&args.module, true);
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
    if let Some(app) = args.app {
        println!("\nAdd to apps/{}/app.py:", app);
        println!("  from core.{}.routes import router as {}_router", args.module, args.module);
        println!("  app.include_router({}_router, prefix=\"/{}\", tags=[\"{}\"])", args.module, args.module, args.module);
    }

    Ok(())
}

/// Generate routes code
fn generate_routes_code(module: &str, _is_core: bool) -> String {
    format!(
        r#""""
{module} routes.

API endpoints for the {module} module.
"""
from fastapi import APIRouter, Depends, HTTPException, status

router = APIRouter()


@router.get("/")
async def list_{module}s():
    """List all {module}s."""
    return []


@router.get("/{{id}}")
async def get_{module}(id: int):
    """Get a {module} by ID."""
    raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="{module} not found")


@router.post("/", status_code=status.HTTP_201_CREATED)
async def create_{module}():
    """Create a new {module}."""
    return {{"id": 1}}


@router.put("/{{id}}")
async def update_{module}(id: int):
    """Update a {module}."""
    return {{"id": id}}


@router.delete("/{{id}}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_{module}(id: int):
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
    let schema_code = generate_schema_code(&args.name, &args.r#type);

    let mut content = fs::read_to_string(&schemas_path).unwrap_or_default();
    content.push_str("\n\n");
    content.push_str(&schema_code);
    fs::write(&schemas_path, content)?;

    println!("Added schema '{}' to core/{}/schemas.py", args.name, args.module);

    Ok(())
}

/// Generate schema code based on type
fn generate_schema_code(name: &str, schema_type: &str) -> String {
    let mut code = String::new();

    if schema_type == "request" || schema_type == "both" {
        code.push_str(&format!(
            r#"
class {name}Create(Schema):
    """
    {name} creation request schema.

    Validation examples (from ouroboros.validation):
      - Field(min_length=1, max_length=255) - string length
      - Field(ge=0, le=100) - numeric range (>=, <=)
      - Field(gt=0, lt=1000) - numeric range (>, <)
      - Field(pattern=r"^[a-z]+$") - regex pattern
      - Field(email=True) - email validation
      - Field(url=True) - URL validation
      - EmailStr, HttpUrl - typed validators

    Model validators:
      @model_validator(mode="before") - pre-processing
      @model_validator(mode="after") - post-processing

    Field validators:
      @field_validator("field_name")
      def validate_field(cls, v): ...
    """
    # Add your fields here, examples:
    # name: str = Field(min_length=1, max_length=255)
    # email: EmailStr
    # amount: float = Field(ge=0)
    # status: Literal["active", "inactive"] = "active"
    pass


class {name}Update(Schema):
    """
    {name} update request schema.

    All fields are optional for partial updates.
    Use Optional[T] or T | None for nullable fields.
    """
    # Add your fields here, examples:
    # name: str | None = None
    # email: EmailStr | None = None
    # status: Literal["active", "inactive"] | None = None
    pass
"#,
            name = name
        ));
    }

    if schema_type == "response" || schema_type == "both" {
        code.push_str(&format!(
            r#"
class {name}Response(Schema):
    """
    {name} response schema.

    Computed fields (from ouroboros.validation):
      @computed_field
      @property
      def full_name(self) -> str:
          return f"{{self.first_name}} {{self.last_name}}"

    Serialization options:
      model_config = ConfigDict(
          from_attributes=True,  # Allow ORM mode
          json_encoders={{datetime: lambda v: v.isoformat()}},
      )
    """
    id: int
    created_at: datetime
    updated_at: datetime
    # Add your fields here


class {name}ListResponse(Schema):
    """{name} paginated list response."""
    items: list[{name}Response]
    total: int
    page: int = 1
    page_size: int = 20
    has_next: bool = False
    has_prev: bool = False
"#,
            name = name
        ));
    }

    code
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

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
