//! `ob api g test` command implementation
//!
//! Generates test files using the ouroboros-qc test framework.

use anyhow::Result;
use clap::Args;
use std::fs;
use std::path::Path;

use super::config::find_pyproject;

/// Arguments for `ob api g test`
#[derive(Debug, Args)]
pub struct TestArgs {
    /// Name of the module to generate tests for
    pub module: String,

    /// Generate only service tests
    #[arg(long)]
    pub service: bool,

    /// Generate only route/API tests
    #[arg(long)]
    pub route: bool,

    /// Target app for route tests (required if --route is specified)
    #[arg(long)]
    pub app: Option<String>,

    /// Whether this is a core module (default: feature)
    #[arg(long)]
    pub core: bool,

    /// Generate with fixtures
    #[arg(long, default_value = "true")]
    pub fixtures: bool,
}

/// Execute test generation
pub async fn execute(args: TestArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let (pyproject_path, _pyproject) = find_pyproject(&current_dir)?;
    let project_root = pyproject_path.parent().unwrap();

    // Determine module path
    let module_type = if args.core { "core" } else { "features" };
    let module_dir = project_root.join(module_type).join(&args.module);

    if !module_dir.exists() {
        anyhow::bail!(
            "{} module '{}' does not exist. Create it first with `ob api {} create {}`",
            if args.core { "Core" } else { "Feature" },
            args.module,
            if args.core { "core" } else { "feat" },
            args.module
        );
    }

    // Create tests directory
    let tests_dir = module_dir.join("tests");
    fs::create_dir_all(&tests_dir)?;

    // Determine what to generate
    let generate_service = args.service || (!args.service && !args.route);
    let generate_route = args.route || (!args.service && !args.route);

    // Generate __init__.py
    generate_tests_init(&tests_dir, &args.module)?;

    // Generate conftest.py with fixtures
    if args.fixtures {
        generate_conftest(&tests_dir, &args.module, args.core)?;
    }

    // Generate service tests
    if generate_service {
        generate_service_tests(&tests_dir, &args.module)?;
        println!("  ✓ Created tests/test_service.py");
    }

    // Generate route tests
    if generate_route {
        if let Some(app) = &args.app {
            generate_route_tests(&tests_dir, &args.module, app)?;
            println!("  ✓ Created tests/test_routes_{}.py", app);
        } else {
            // Generate generic route tests
            generate_route_tests(&tests_dir, &args.module, "api")?;
            println!("  ✓ Created tests/test_routes.py");
        }
    }

    println!("\n✅ Tests generated for '{}' module", args.module);
    println!("\nRun tests with:");
    println!("  ob test {}/{}/tests", module_type, args.module);

    Ok(())
}

/// Generate tests/__init__.py
fn generate_tests_init(tests_dir: &Path, module_name: &str) -> Result<()> {
    let content = format!(
        r#""""
Tests for the {} module.

Uses ouroboros.test (ouroboros-qc) for:
- Async test support
- expect-style assertions
- Fixtures with scoping
- Parametrized tests
- Performance benchmarking
"""
"#,
        module_name
    );

    fs::write(tests_dir.join("__init__.py"), content)?;
    println!("  ✓ Created tests/__init__.py");
    Ok(())
}

/// Generate conftest.py with fixtures
fn generate_conftest(tests_dir: &Path, module_name: &str, is_core: bool) -> Result<()> {
    let module_type = if is_core { "core" } else { "features" };
    let content = format!(
        r#""""
Test fixtures for {} module.

Fixtures are automatically discovered and injected by ouroboros.test.
"""
import pytest
from ouroboros.test import fixture, FixtureScope

# Import the module under test
from {module_type}.{module_name} import *


@fixture(scope=FixtureScope.Function)
async def sample_{module_name}_data():
    """
    Provides sample test data for {module_name} tests.

    Scope: Function - created fresh for each test.
    """
    return {{
        "id": "test-id-001",
        "name": "Test {module_name_title}",
        "created_at": "2024-01-01T00:00:00Z",
    }}


@fixture(scope=FixtureScope.Module)
async def {module_name}_service():
    """
    Provides a configured service instance.

    Scope: Module - shared across all tests in this module.
    """
    from {module_type}.{module_name}.services import {module_name_title}Service

    service = {module_name_title}Service()
    yield service
    # Cleanup if needed
    # await service.cleanup()


@fixture(scope=FixtureScope.Session)
async def db_session():
    """
    Provides a database session for integration tests.

    Scope: Session - shared across all test modules.
    """
    from ouroboros.pg import Connection, PoolConfig
    import os

    db_url = os.getenv("DATABASE_URL", "postgresql://localhost/test_db")
    conn = await Connection.new(db_url, PoolConfig.default())
    yield conn
    # Connection automatically cleaned up
"#,
        module_name,
        module_type = module_type,
        module_name = module_name,
        module_name_title = to_pascal_case(module_name),
    );

    fs::write(tests_dir.join("conftest.py"), content)?;
    println!("  ✓ Created tests/conftest.py");
    Ok(())
}

/// Generate service tests
fn generate_service_tests(tests_dir: &Path, module_name: &str) -> Result<()> {
    let content = format!(
        r#""""
Service tests for {module_name} module.

Tests business logic in isolation from HTTP layer.
"""
from ouroboros.test import TestSuite, test, expect, parametrize


class Test{module_name_title}Service(TestSuite):
    """Tests for {module_name_title}Service."""

    @test(tags=["unit", "service"])
    async def test_create(self, {module_name}_service, sample_{module_name}_data):
        """Test creating a new {module_name} entity."""
        data = sample_{module_name}_data

        result = await {module_name}_service.create(data)

        expect(result).to_not_be_none()
        expect(result.name).to_equal(data["name"])

    @test(tags=["unit", "service"])
    async def test_get_by_id(self, {module_name}_service):
        """Test retrieving a {module_name} by ID."""
        entity_id = "test-id-001"

        result = await {module_name}_service.get_by_id(entity_id)

        expect(result).to_not_be_none()
        expect(result.id).to_equal(entity_id)

    @test(tags=["unit", "service"])
    async def test_get_by_id_not_found(self, {module_name}_service):
        """Test getting non-existent {module_name} returns None."""
        result = await {module_name}_service.get_by_id("non-existent-id")

        expect(result).to_be_none()

    @test(tags=["unit", "service"])
    async def test_update(self, {module_name}_service, sample_{module_name}_data):
        """Test updating an existing {module_name}."""
        data = sample_{module_name}_data
        updated_data = {{**data, "name": "Updated Name"}}

        result = await {module_name}_service.update(data["id"], updated_data)

        expect(result).to_not_be_none()
        expect(result.name).to_equal("Updated Name")

    @test(tags=["unit", "service"])
    async def test_delete(self, {module_name}_service, sample_{module_name}_data):
        """Test deleting a {module_name}."""
        entity_id = sample_{module_name}_data["id"]

        result = await {module_name}_service.delete(entity_id)

        expect(result).to_be_true()

    @parametrize([
        {{"input": "", "expected_error": "name is required"}},
        {{"input": "a" * 256, "expected_error": "name too long"}},
    ])
    @test(tags=["unit", "validation"])
    async def test_create_validation(self, {module_name}_service, input, expected_error):
        """Test validation errors on create."""
        try:
            await {module_name}_service.create({{"name": input}})
            expect(True).to_be_false()  # Should not reach here
        except ValueError as e:
            expect(str(e)).to_contain(expected_error)


class Test{module_name_title}ServiceIntegration(TestSuite):
    """Integration tests with database."""

    @test(tags=["integration", "db"], timeout=10.0)
    async def test_create_and_retrieve(self, db_session, sample_{module_name}_data):
        """Test full create and retrieve flow with real database."""
        from features.{module_name}.services import {module_name_title}Service

        service = {module_name_title}Service(db=db_session)
        data = sample_{module_name}_data

        # Create
        created = await service.create(data)
        expect(created.id).to_not_be_none()

        # Retrieve
        retrieved = await service.get_by_id(created.id)
        expect(retrieved).to_not_be_none()
        expect(retrieved.name).to_equal(data["name"])

        # Cleanup
        await service.delete(created.id)
"#,
        module_name = module_name,
        module_name_title = to_pascal_case(module_name),
    );

    fs::write(tests_dir.join("test_service.py"), content)?;
    Ok(())
}

/// Generate route/API tests
fn generate_route_tests(tests_dir: &Path, module_name: &str, app_name: &str) -> Result<()> {
    let filename = if app_name == "api" {
        "test_routes.py".to_string()
    } else {
        format!("test_routes_{}.py", app_name)
    };

    let content = format!(
        r#""""
Route tests for {module_name} module ({app_name} app).

Tests HTTP endpoints using ouroboros.test's TestServer.
"""
from ouroboros.test import TestSuite, test, expect, TestServer, parametrize


class Test{module_name_title}Routes(TestSuite):
    """API endpoint tests for {module_name}."""

    @test(tags=["api", "routes"])
    async def test_list_{module_name}(self):
        """Test GET /{module_name} returns list."""
        async with TestServer() as server:
            response = await server.get("/{module_name}")

            expect(response.status_code).to_equal(200)
            expect(response.json()).to_be_instance_of(list)

    @test(tags=["api", "routes"])
    async def test_get_{module_name}_by_id(self):
        """Test GET /{module_name}/{{id}} returns single item."""
        async with TestServer() as server:
            response = await server.get("/{module_name}/test-id-001")

            expect(response.status_code).to_equal(200)
            data = response.json()
            expect(data["id"]).to_equal("test-id-001")

    @test(tags=["api", "routes"])
    async def test_get_{module_name}_not_found(self):
        """Test GET /{module_name}/{{id}} returns 404 for non-existent."""
        async with TestServer() as server:
            response = await server.get("/{module_name}/non-existent")

            expect(response.status_code).to_equal(404)

    @test(tags=["api", "routes"])
    async def test_create_{module_name}(self, sample_{module_name}_data):
        """Test POST /{module_name} creates new item."""
        async with TestServer() as server:
            response = await server.post(
                "/{module_name}",
                json=sample_{module_name}_data
            )

            expect(response.status_code).to_equal(201)
            data = response.json()
            expect(data["name"]).to_equal(sample_{module_name}_data["name"])

    @test(tags=["api", "routes"])
    async def test_update_{module_name}(self, sample_{module_name}_data):
        """Test PUT /{module_name}/{{id}} updates item."""
        async with TestServer() as server:
            updated = {{**sample_{module_name}_data, "name": "Updated"}}
            response = await server.put(
                "/{module_name}/test-id-001",
                json=updated
            )

            expect(response.status_code).to_equal(200)
            data = response.json()
            expect(data["name"]).to_equal("Updated")

    @test(tags=["api", "routes"])
    async def test_delete_{module_name}(self):
        """Test DELETE /{module_name}/{{id}} removes item."""
        async with TestServer() as server:
            response = await server.delete("/{module_name}/test-id-001")

            expect(response.status_code).to_equal(204)

    @parametrize([
        {{"payload": {{}}, "expected_status": 422}},
        {{"payload": {{"name": ""}}, "expected_status": 422}},
    ])
    @test(tags=["api", "validation"])
    async def test_create_validation(self, payload, expected_status):
        """Test POST /{module_name} validation errors."""
        async with TestServer() as server:
            response = await server.post("/{module_name}", json=payload)

            expect(response.status_code).to_equal(expected_status)


class Test{module_name_title}RoutesPerformance(TestSuite):
    """Performance tests for {module_name} routes."""

    @test(tags=["perf", "routes"], benchmark=True)
    async def test_list_performance(self):
        """Benchmark GET /{module_name} response time."""
        async with TestServer() as server:
            # Warm up
            await server.get("/{module_name}")

            # Benchmark
            for _ in range(100):
                response = await server.get("/{module_name}")
                expect(response.status_code).to_equal(200)
"#,
        module_name = module_name,
        module_name_title = to_pascal_case(module_name),
        app_name = app_name,
    );

    fs::write(tests_dir.join(&filename), content)?;
    Ok(())
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}
