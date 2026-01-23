//! Centralized code generation for `ob api` CLI
//!
//! Provides generation functions for:
//! - Models (with field definitions)
//! - Schemas (Create, Update, Response)
//! - Services (full CRUD implementation)
//! - Routes (wired to services)
//! - App integration (automatic router registration)

use anyhow::Result;
use std::fs;
use std::path::Path;

use super::config::DbType;
use super::fields::FieldDef;

/// Convert string to snake_case
pub fn to_snake_case(s: &str) -> String {
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
pub fn to_pascal_case(s: &str) -> String {
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

/// Generate Model code with field definitions
pub fn generate_model_code(name: &str, fields: &[FieldDef], db_type: DbType) -> String {
    let table_name = to_snake_case(name);

    let field_lines: Vec<String> = fields
        .iter()
        .filter(|f| !f.is_auto_field())
        .map(|f| f.to_model_field(db_type))
        .collect();

    let fields_code = if field_lines.is_empty() {
        "    pass".to_string()
    } else {
        field_lines.join("\n")
    };

    match db_type {
        DbType::Pg => format!(
            r#"
class {name}(Model):
    """
    {name} model.

    PostgreSQL table: {table_name}s
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

    # Fields
{fields_code}
"#,
            name = name,
            table_name = table_name,
            fields_code = fields_code
        ),
        DbType::Mongo => format!(
            r#"
class {name}(Document):
    """
    {name} document.

    MongoDB collection: {table_name}s
    """
    __collection__ = "{table_name}s"

    # Timestamps
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)

    # Fields
{fields_code}
"#,
            name = name,
            table_name = table_name,
            fields_code = fields_code
        ),
    }
}

/// Generate Schema code (Create, Update, Response) from fields
pub fn generate_schema_code(name: &str, fields: &[FieldDef]) -> String {
    let user_fields: Vec<&FieldDef> = fields.iter().filter(|f| !f.is_auto_field()).collect();

    // Create schema - required fields for creation
    let create_fields: Vec<String> = user_fields.iter().map(|f| f.to_schema_field(false)).collect();

    let create_fields_code = if create_fields.is_empty() {
        "    pass".to_string()
    } else {
        create_fields.join("\n")
    };

    // Update schema - all fields optional
    let update_fields: Vec<String> = user_fields.iter().map(|f| f.to_schema_field(true)).collect();

    let update_fields_code = if update_fields.is_empty() {
        "    pass".to_string()
    } else {
        update_fields.join("\n")
    };

    // Response schema - all fields including auto fields
    let response_fields: Vec<String> = user_fields.iter().map(|f| f.to_schema_field(false)).collect();

    let response_fields_code = if response_fields.is_empty() {
        String::new()
    } else {
        format!("\n{}", response_fields.join("\n"))
    };

    format!(
        r#"
class {name}Create(Schema):
    """Request schema for creating {name}."""
{create_fields}


class {name}Update(Schema):
    """Request schema for updating {name}. All fields optional."""
{update_fields}


class {name}Response(Schema):
    """Response schema for {name}."""
    model_config = ConfigDict(from_attributes=True)

    id: int
    created_at: datetime
    updated_at: datetime{response_fields}


class {name}ListResponse(Schema):
    """Paginated list response for {name}."""
    items: list[{name}Response]
    total: int
    page: int = 1
    page_size: int = 20
    has_next: bool = False
    has_prev: bool = False
"#,
        name = name,
        create_fields = create_fields_code,
        update_fields = update_fields_code,
        response_fields = response_fields_code
    )
}

/// Generate Service code with full CRUD implementation
pub fn generate_service_code(name: &str, fields: &[FieldDef], db_type: DbType) -> String {
    let class_name = to_pascal_case(name);
    let snake_name = to_snake_case(name);

    let user_fields: Vec<&FieldDef> = fields.iter().filter(|f| !f.is_auto_field()).collect();

    // Generate field assignment for create
    let create_assignments: Vec<String> = user_fields
        .iter()
        .map(|f| format!("            {}=data.{},", f.name, f.name))
        .collect();
    let create_assignments_code = create_assignments.join("\n");

    // Generate field update for update method
    let update_assignments: Vec<String> = user_fields
        .iter()
        .map(|f| {
            format!(
                "        if data.{} is not None:\n            {}.{} = data.{}",
                f.name, snake_name, f.name, f.name
            )
        })
        .collect();
    let update_assignments_code = update_assignments.join("\n");

    match db_type {
        DbType::Pg => format!(
            r#"
from typing import Optional
from .models import {name}
from .schemas import {name}Create, {name}Update, {name}Response, {name}ListResponse


class {class_name}Service:
    """Service for {name} CRUD operations."""

    def __init__(self, db):
        self.db = db

    async def get(self, id: int) -> Optional[{name}]:
        """Get {name} by ID."""
        return await self.db.fetch_one(
            {name},
            {name}.id == id
        )

    async def list(
        self,
        page: int = 1,
        page_size: int = 20
    ) -> {name}ListResponse:
        """List {name}s with pagination."""
        offset = (page - 1) * page_size

        items = await self.db.fetch_all(
            {name},
            limit=page_size,
            offset=offset,
            order_by={name}.created_at.desc()
        )

        total = await self.db.count({name})

        return {name}ListResponse(
            items=[{name}Response.model_validate(item) for item in items],
            total=total,
            page=page,
            page_size=page_size,
            has_next=offset + page_size < total,
            has_prev=page > 1
        )

    async def create(self, data: {name}Create) -> {name}:
        """Create a new {name}."""
        {snake_name} = {name}(
{create_assignments}
        )
        await self.db.insert({snake_name})
        return {snake_name}

    async def update(self, id: int, data: {name}Update) -> Optional[{name}]:
        """Update an existing {name}."""
        {snake_name} = await self.get(id)
        if not {snake_name}:
            return None

{update_assignments}

        await self.db.update({snake_name})
        return {snake_name}

    async def delete(self, id: int) -> bool:
        """Delete a {name}."""
        {snake_name} = await self.get(id)
        if not {snake_name}:
            return False

        await self.db.delete({snake_name})
        return True
"#,
            name = name,
            class_name = class_name,
            snake_name = snake_name,
            create_assignments = create_assignments_code,
            update_assignments = update_assignments_code
        ),
        DbType::Mongo => format!(
            r#"
from typing import Optional
from bson import ObjectId
from .models import {name}
from .schemas import {name}Create, {name}Update, {name}Response, {name}ListResponse


class {class_name}Service:
    """Service for {name} CRUD operations."""

    def __init__(self, db):
        self.db = db
        self.collection = db["{snake_name}s"]

    async def get(self, id: str) -> Optional[{name}]:
        """Get {name} by ID."""
        doc = await self.collection.find_one({{"_id": ObjectId(id)}})
        return {name}(**doc) if doc else None

    async def list(
        self,
        page: int = 1,
        page_size: int = 20
    ) -> {name}ListResponse:
        """List {name}s with pagination."""
        skip = (page - 1) * page_size

        cursor = self.collection.find().sort("created_at", -1).skip(skip).limit(page_size)
        items = [{name}(**doc) async for doc in cursor]

        total = await self.collection.count_documents({{}})

        return {name}ListResponse(
            items=[{name}Response.model_validate(item) for item in items],
            total=total,
            page=page,
            page_size=page_size,
            has_next=skip + page_size < total,
            has_prev=page > 1
        )

    async def create(self, data: {name}Create) -> {name}:
        """Create a new {name}."""
        {snake_name} = {name}(
{create_assignments}
        )
        result = await self.collection.insert_one({snake_name}.model_dump())
        {snake_name}.id = str(result.inserted_id)
        return {snake_name}

    async def update(self, id: str, data: {name}Update) -> Optional[{name}]:
        """Update an existing {name}."""
        update_data = data.model_dump(exclude_unset=True)
        if not update_data:
            return await self.get(id)

        result = await self.collection.update_one(
            {{"_id": ObjectId(id)}},
            {{"$set": update_data}}
        )

        if result.modified_count == 0:
            return None

        return await self.get(id)

    async def delete(self, id: str) -> bool:
        """Delete a {name}."""
        result = await self.collection.delete_one({{"_id": ObjectId(id)}})
        return result.deleted_count > 0
"#,
            name = name,
            class_name = class_name,
            snake_name = snake_name,
            create_assignments = create_assignments_code
        ),
    }
}

/// Generate endpoints.py file for a feature module (SSOT)
pub fn generate_endpoints_code(module: &str, model_name: &str) -> String {
    let class_name = to_pascal_case(module);
    let snake_module = to_snake_case(module);

    format!(
        r#""""
{module} endpoint definitions (SSOT).

All endpoint configurations are defined here. Route handlers and tests
import from this file to ensure consistency.

Usage:
    from .endpoints import {class_name}Endpoints as E

    # In routes.py:
    @router.route(E.LIST.method.value, E.LIST.path, status_code=E.LIST.status_code.value)
    async def list_{snake}s(): ...

    # In tests:
    response = await server.request(E.LIST.method.value, f"{{E.PREFIX}}{{E.LIST.path}}")
"""
from ouroboros.api import Endpoint, HTTPMethod, HTTPStatus

from .schemas import {model}Create, {model}Update, {model}Response, {model}ListResponse


class {class_name}Endpoints:
    """{model} endpoint configurations."""

    PREFIX = "/{snake}"

    LIST = Endpoint(
        path="/",
        method=HTTPMethod.GET,
        handler="list_{snake}s",
        summary="List all {module}s",
        response_model={model}ListResponse,
    )
    CREATE = Endpoint(
        path="/",
        method=HTTPMethod.POST,
        handler="create_{snake}",
        summary="Create a {module}",
        status_code=HTTPStatus.CREATED,
        request_model={model}Create,
        response_model={model}Response,
    )
    GET = Endpoint(
        path="/{{id}}",
        method=HTTPMethod.GET,
        handler="get_{snake}",
        summary="Get {module} by ID",
        response_model={model}Response,
    )
    UPDATE = Endpoint(
        path="/{{id}}",
        method=HTTPMethod.PUT,
        handler="update_{snake}",
        summary="Update a {module}",
        request_model={model}Update,
        response_model={model}Response,
    )
    DELETE = Endpoint(
        path="/{{id}}",
        method=HTTPMethod.DELETE,
        handler="delete_{snake}",
        summary="Delete a {module}",
        status_code=HTTPStatus.NO_CONTENT,
    )
"#,
        module = module,
        model = model_name,
        class_name = class_name,
        snake = snake_module,
    )
}

/// Generate routes.py code referencing local endpoints.py
pub fn generate_routes_code(module: &str, model_name: &str, _fields: &[FieldDef]) -> String {
    let snake_module = to_snake_case(module);
    let endpoints_class = to_pascal_case(module);

    format!(
        r#""""
{module} routes.

API endpoints for the {module} feature.
Handlers reference endpoint definitions from .endpoints (SSOT).
"""
from ouroboros.api import Router, Path, Query, Body, HTTPException

from .endpoints import {endpoints_class}Endpoints as E
from .schemas import {model}Create, {model}Update, {model}Response, {model}ListResponse
from .services import {model}Service


router = Router(prefix=E.PREFIX, tags=["{module}"])


def get_service():
    """Dependency to get service instance."""
    from ouroboros.pg import get_db
    return {model}Service(get_db())


@router.route(E.LIST.method.value, E.LIST.path, status_code=E.LIST.status_code.value)
async def list_{snake}s(
    page: int = Query(default=1, ge=1),
    page_size: int = Query(default=20, ge=1, le=100),
):
    """{{E.LIST.summary}}"""
    service = get_service()
    return await service.list(page=page, page_size=page_size)


@router.route(E.GET.method.value, E.GET.path, status_code=E.GET.status_code.value)
async def get_{snake}(id: int = Path()):
    """{{E.GET.summary}}"""
    service = get_service()
    result = await service.get(id)
    if not result:
        raise HTTPException(404, "{model} not found")
    return result


@router.route(E.CREATE.method.value, E.CREATE.path, status_code=E.CREATE.status_code.value)
async def create_{snake}(data: {model}Create = Body()):
    """{{E.CREATE.summary}}"""
    service = get_service()
    return await service.create(data)


@router.route(E.UPDATE.method.value, E.UPDATE.path, status_code=E.UPDATE.status_code.value)
async def update_{snake}(id: int = Path(), data: {model}Update = Body()):
    """{{E.UPDATE.summary}}"""
    service = get_service()
    result = await service.update(id, data)
    if not result:
        raise HTTPException(404, "{model} not found")
    return result


@router.route(E.DELETE.method.value, E.DELETE.path, status_code=E.DELETE.status_code.value)
async def delete_{snake}(id: int = Path()):
    """{{E.DELETE.summary}}"""
    service = get_service()
    success = await service.delete(id)
    if not success:
        raise HTTPException(404, "{model} not found")
"#,
        module = module,
        model = model_name,
        snake = snake_module,
        endpoints_class = endpoints_class,
    )
}

/// Check if an endpoint already exists in routes.py
pub fn check_endpoint_exists(routes_path: &Path, method: &str, path: &str) -> Result<bool> {
    let content = fs::read_to_string(routes_path)?;

    // Look for decorator pattern: @router.{method}("{path}")
    // Note: path uses single braces {id}, not double {{id}}
    let pattern = format!(
        "@router.{}(\"{}\"",
        method.to_lowercase(),
        path
    );

    Ok(content.contains(&pattern))
}

/// Register router in app.py
pub fn register_router_in_app(app_path: &Path, module: &str, is_core: bool) -> Result<()> {
    let mut content = fs::read_to_string(app_path)?;

    let module_type = if is_core { "core" } else { "features" };
    let import_line = format!(
        "from {}.{}.routes import router as {}_router",
        module_type, module, module
    );
    let include_line = format!(
        "app.include_router({}_router, prefix=\"/{}\", tags=[\"{}\"])",
        module, module, module
    );

    // Check if already registered
    if content.contains(&import_line) {
        return Ok(());
    }

    // Find the right place to insert
    // Look for the comment marker or existing router includes
    if let Some(pos) = content.find("# Add feature/core routers here:") {
        // Insert after the comment
        let insert_pos = content[pos..].find('\n').map(|p| pos + p + 1).unwrap_or(pos);
        let insertion = format!("{}\n{}\n\n", import_line, include_line);
        content.insert_str(insert_pos, &insertion);
    } else if let Some(pos) = content.rfind("app.include_router") {
        // Insert after the last include_router
        let insert_pos = content[pos..].find('\n').map(|p| pos + p + 1).unwrap_or(content.len());
        let insertion = format!("\n{}\n{}", import_line, include_line);
        content.insert_str(insert_pos, &insertion);
    } else {
        // Append before if __name__ == "__main__": or at the end
        if let Some(pos) = content.find("if __name__") {
            let insertion = format!("{}\n{}\n\n\n", import_line, include_line);
            content.insert_str(pos, &insertion);
        } else {
            content.push_str(&format!("\n\n{}\n{}\n", import_line, include_line));
        }
    }

    fs::write(app_path, content)?;
    Ok(())
}

// =============================================================================
// Infrastructure Templates (P1)
// =============================================================================

/// Generate configs.py for project configuration
pub fn generate_configs_code(project_name: &str, db_type: DbType) -> String {
    let db_config = match db_type {
        DbType::Pg => r#"
# PostgreSQL Configuration
DATABASE_URL: str = os.getenv(
    "DATABASE_URL",
    "postgresql://postgres:postgres@localhost:5432/{project_name}"
)

class DatabaseSettings(BaseSettings):
    """PostgreSQL database settings."""
    url: str = DATABASE_URL
    pool_size: int = 5
    max_overflow: int = 10
    echo: bool = ENV == Environment.DEV

    model_config = SettingsConfigDict(env_prefix="DB_")

db_settings = DatabaseSettings()"#,
        DbType::Mongo => r#"
# MongoDB Configuration
MONGO_URL: str = os.getenv(
    "MONGO_URL",
    "mongodb://localhost:27017/{project_name}"
)

class DatabaseSettings(BaseSettings):
    """MongoDB database settings."""
    url: str = MONGO_URL
    database: str = "{project_name}"

    model_config = SettingsConfigDict(env_prefix="MONGO_")

db_settings = DatabaseSettings()"#,
    };

    format!(
        r#""""
Project configuration.

Environment-based settings for the application.
"""
from __future__ import annotations

import os
import pathlib
from enum import StrEnum

from pydantic_settings import BaseSettings, SettingsConfigDict


class Environment(StrEnum):
    """Application environments."""
    DEV = "dev"
    TEST = "test"
    UAT = "uat"
    PROD = "prod"


# Core settings
PROJECT_DIR: pathlib.Path = pathlib.Path(__file__).parent.parent
ENV: Environment = Environment(os.getenv("ENV", "dev"))
DEBUG: bool = ENV in {{Environment.DEV, Environment.TEST}}
{db_config}


class AppSettings(BaseSettings):
    """Application settings."""
    name: str = "{project_name}"
    version: str = "0.1.0"
    host: str = "0.0.0.0"
    port: int = 8000

    model_config = SettingsConfigDict(env_prefix="APP_")

app_settings = AppSettings()
"#,
        project_name = project_name,
        db_config = db_config.replace("{project_name}", project_name)
    )
}

/// Generate dependencies.py for FastAPI dependency injection
pub fn generate_dependencies_code(db_type: DbType) -> String {
    let db_dependency = match db_type {
        DbType::Pg => r#"
async def get_db() -> AsyncGenerator[AsyncSession, None]:
    """Get database session."""
    async with async_session_maker() as session:
        try:
            yield session
        finally:
            await session.close()


DbSession = Annotated[AsyncSession, Depends(get_db)]"#,
        DbType::Mongo => r#"
def get_db() -> Database:
    """Get MongoDB database."""
    from .lifespans import mongo_client
    from .configs import db_settings
    return mongo_client[db_settings.database]


DbSession = Annotated[Database, Depends(get_db)]"#,
    };

    format!(
        r#""""
FastAPI dependencies.

Common dependency injection functions for routes.
"""
from __future__ import annotations

import logging
from typing import Annotated, AsyncGenerator, NamedTuple

import fastapi
from fastapi import Depends, Query, Request

from .configs import ENV, Environment


# Logging
def get_logger(request: Request) -> logging.LoggerAdapter:
    """Get logger with request context."""
    logger = logging.getLogger(__name__)
    return logging.LoggerAdapter(logger, {{"request_id": id(request)}})

Logger = Annotated[logging.LoggerAdapter, Depends(get_logger)]


# Pagination
class Pagination(NamedTuple):
    """Pagination parameters."""
    page: int
    page_size: int
    offset: int
    limit: int


def get_pagination(
    page: Annotated[int, Query(ge=1, description="Page number")] = 1,
    page_size: Annotated[int, Query(ge=1, le=100, description="Items per page")] = 20,
) -> Pagination:
    """Get pagination parameters."""
    return Pagination(
        page=page,
        page_size=page_size,
        offset=(page - 1) * page_size,
        limit=page_size,
    )

PaginationDep = Annotated[Pagination, Depends(get_pagination)]


# Database
{db_dependency}
"#,
        db_dependency = db_dependency
    )
}

/// Generate lifespans.py for application lifecycle
pub fn generate_lifespans_code(db_type: DbType) -> String {
    let (imports, startup, shutdown) = match db_type {
        DbType::Pg => (
            r#"from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession

from .configs import db_settings"#,
            r#"    # Initialize PostgreSQL connection
    global async_engine, async_session_maker
    async_engine = create_async_engine(
        db_settings.url,
        pool_size=db_settings.pool_size,
        max_overflow=db_settings.max_overflow,
        echo=db_settings.echo,
    )
    async_session_maker = async_sessionmaker(
        async_engine,
        class_=AsyncSession,
        expire_on_commit=False,
    )
    app.state.db_engine = async_engine
    print(f"✓ PostgreSQL connected")"#,
            r#"    # Close PostgreSQL connection
    if hasattr(app.state, "db_engine"):
        await app.state.db_engine.dispose()
        print("✓ PostgreSQL disconnected")"#,
        ),
        DbType::Mongo => (
            r#"from motor.motor_asyncio import AsyncIOMotorClient

from .configs import db_settings"#,
            r#"    # Initialize MongoDB connection
    global mongo_client
    mongo_client = AsyncIOMotorClient(db_settings.url)
    app.state.mongo_client = mongo_client
    print(f"✓ MongoDB connected")"#,
            r#"    # Close MongoDB connection
    if hasattr(app.state, "mongo_client"):
        app.state.mongo_client.close()
        print("✓ MongoDB disconnected")"#,
        ),
    };

    format!(
        r#""""
Application lifecycle management.

Handles startup and shutdown events for the application.
"""
from __future__ import annotations

import contextlib
import logging
from typing import TYPE_CHECKING

{imports}

if TYPE_CHECKING:
    from fastapi import FastAPI

# Module-level references (set during startup)
async_engine = None
async_session_maker = None
mongo_client = None


@contextlib.asynccontextmanager
async def default_lifespan(app: "FastAPI"):
    """Default application lifespan handler."""
    # Startup
    logging.basicConfig(level=logging.INFO)
    print(f"🚀 Starting {{app.title}}...")

{startup}

    yield

    # Shutdown
    print(f"👋 Shutting down {{app.title}}...")

{shutdown}

    print("✓ Shutdown complete")
"#,
        imports = imports,
        startup = startup,
        shutdown = shutdown
    )
}

/// Generate apps.py for multi-service factory
pub fn generate_apps_code(project_name: &str, services: &[&str]) -> String {
    let service_enum = services
        .iter()
        .map(|s| format!("    {} = \"{}\"", s.to_uppercase(), s.to_lowercase()))
        .collect::<Vec<_>>()
        .join("\n");

    let service_cases = services
        .iter()
        .map(|s| {
            let upper = s.to_uppercase();
            let lower = s.to_lowercase();
            format!(
                r#"    if service == APIService.{upper}:
        app = FastAPI(
            title="{project_name} {upper} API",
            version=app_settings.version,
            lifespan=default_lifespan,
            root_path="/{lower}-api",
        )
        # Include {lower}-specific routers here
        # app.include_router({lower}_router)"#,
                upper = upper,
                lower = lower,
                project_name = project_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n    el");

    format!(
        r#""""
Multi-service application factory.

Creates FastAPI applications for different services.
"""
from __future__ import annotations

from enum import StrEnum

from fastapi import FastAPI

from .configs import app_settings
from .lifespans import default_lifespan


class APIService(StrEnum):
    """Available API services."""
{service_enum}


def create_app(service: APIService) -> FastAPI:
    """Create a FastAPI application for the specified service."""
{service_cases}
    else:
        raise ValueError(f"Unknown service: {{service}}")

    # Common middleware
    # app.middleware("http")(your_middleware)

    # Common exception handlers
    # app.exception_handler(Exception)(generic_handler)

    return app


# Default app for development
app = create_app(APIService.{default_service})


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host=app_settings.host, port=app_settings.port)
"#,
        service_enum = service_enum,
        service_cases = service_cases,
        default_service = services.first().map(|s| s.to_uppercase()).unwrap_or_else(|| "ADMIN".to_string())
    )
}

/// Generate constants.py for project constants
pub fn generate_constants_code() -> String {
    r#""""
Project constants.

Common constants used across the application.
"""
from __future__ import annotations

from enum import StrEnum


class HTTPStatus(StrEnum):
    """Common HTTP status codes."""
    OK = "200"
    CREATED = "201"
    NO_CONTENT = "204"
    BAD_REQUEST = "400"
    UNAUTHORIZED = "401"
    FORBIDDEN = "403"
    NOT_FOUND = "404"
    CONFLICT = "409"
    UNPROCESSABLE_ENTITY = "422"
    INTERNAL_SERVER_ERROR = "500"
"#.to_string()
}
