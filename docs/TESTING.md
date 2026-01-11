# Testing Guide - data-bridge

## Overview

data-bridge uses **data-bridge-test**, a native Rust-backed testing framework that provides superior performance compared to pytest while maintaining familiar Python APIs.

**Key Benefits**:
- ✅ **2-5x faster** test execution (Rust-backed)
- ✅ **Native async support** (no plugins needed)
- ✅ **Comprehensive features**: fixtures, parametrize, hooks, test server
- ✅ **pytest-compatible** API for easy migration
- ✅ **Better error messages** from Rust engine

---

## Quick Start

```bash
# Run all tests
uv run python -m data_bridge.test tests/ -v

# Run specific directory
uv run python tests/unit/

# Run specific file
uv run python tests/unit/test_validation.py

# With coverage
uv run python -m data_bridge.test tests/ --coverage --format=html
```

---

## Writing Tests

### Basic Test

```python
from data_bridge.test import TestSuite, test, expect

class TestUser(TestSuite):
    @test
    async def test_create_user(self):
        """Test user creation."""
        user = await User(email="test@example.com", name="Test").save()
        expect(user.id).to_not_be_none()
        expect(user.email).to_equal("test@example.com")
```

### Assertions

data-bridge-test uses the `expect()` API for more readable assertions:

```python
# Equality
expect(result).to_equal(42)
expect(result).to_not_equal(0)

# Comparisons
expect(value).to_be_greater_than(10)
expect(value).to_be_less_than(100)
expect(value).to_be_greater_than_or_equal(50)
expect(value).to_be_less_than_or_equal(75)

# Truthiness
expect(condition).to_be_true()
expect(condition).to_be_false()
expect(value).to_be_none()
expect(value).to_not_be_none()

# Containment
expect([1, 2, 3]).to_contain(2)
expect("hello world").to_contain("world")

# Exceptions
expect(lambda: risky_operation()).to_raise(ValueError)
```

---

## Fixtures

Fixtures provide setup/teardown and resource management:

```python
from data_bridge.test import TestSuite, test, fixture, expect

class TestDatabase(TestSuite):
    @fixture(scope="class")
    async def db_connection(self):
        """Class-scoped database connection."""
        await init("mongodb://localhost:27017/test")
        yield
        await close()

    @fixture(scope="function")
    async def clean_users(self, db_connection):
        """Clean users before each test."""
        await User.delete_many({})
        yield
        await User.delete_many({})  # Cleanup after test

    @test
    async def test_create_user(self, clean_users):
        """Fixture auto-injected via parameter."""
        user = await User(email="test@example.com").save()
        expect(user.id).to_not_be_none()
```

### Fixture Scopes

| Scope | Description | Usage |
|-------|-------------|-------|
| `function` | Run before/after each test | Default |
| `class` | Run once per test class | Database connections |
| `module` | Run once per module | Expensive setup |
| `session` | Run once per test session | Global resources |

---

## Parametrization

Run the same test with different inputs:

```python
from data_bridge.test import TestSuite, test, parametrize, expect

class TestValidation(TestSuite):
    @test
    @parametrize("value", [10, 100, 1000, 10000])
    async def test_batch_insert(self, value):
        """Runs 4 times with different batch sizes."""
        docs = [{"id": i} for i in range(value)]
        result = await Collection.insert_many(docs)
        expect(len(result.inserted_ids)).to_equal(value)

    @test
    @parametrize("method", ["GET", "POST", "PUT", "DELETE"])
    @parametrize("auth", [True, False])
    async def test_http_methods(self, method, auth):
        """Generates 8 test cases (4 × 2 Cartesian product)."""
        response = await client.request(method, "/api/test", auth=auth)
        expect(response.status).to_equal(200)
```

**Test names** are auto-generated:
- `test_batch_insert[value=1000]`
- `test_http_methods[auth=true,method=GET]`

---

## Setup/Teardown Hooks

Lifecycle hooks for class and method-level setup:

```python
class TestLifecycle(TestSuite):
    async def setup_class(self):
        """Run once before all tests in class."""
        self.db = await init_database()

    async def teardown_class(self):
        """Run once after all tests in class."""
        await self.db.close()

    async def setup_method(self):
        """Run before each test method."""
        await self.db.clear()

    async def teardown_method(self):
        """Run after each test method."""
        pass  # Optional cleanup

    @test
    async def test_insert(self):
        # setup_class → setup_method → test → teardown_method → teardown_class
        result = await self.db.insert({"name": "test"})
        expect(result).to_not_be_none()
```

**Execution Order**:
```
setup_class
├── setup_method → test_1 → teardown_method
├── setup_method → test_2 → teardown_method
└── teardown_class
```

---

## Test Server

For API integration testing with automatic subprocess management:

```python
from data_bridge.test import TestSuite, test, fixture, TestServer, expect

class TestAPI(TestSuite):
    @fixture(scope="class")
    async def server(self):
        """Auto-start Python app server."""
        server = TestServer.from_app(
            app_module="tests.fixtures.test_app",
            app_callable="app",
            port=18765,
            startup_timeout=10.0,
            health_endpoint="/health",
        )
        await server.start()  # Spawns subprocess, waits for health
        yield server
        await server.stop()   # Graceful shutdown

    @test
    async def test_health_endpoint(self, server):
        """Test server health check."""
        response = await server.client.get("/health")
        expect(response.status).to_equal(200)
        expect(response.json()["status"]).to_equal("ok")
```

---

## Async Testing

**No decorators needed** - just use `async def`:

```python
class TestAsync(TestSuite):
    @test
    async def test_async_operation(self):
        """Async tests work natively."""
        result = await some_async_function()
        expect(result).to_be_true()
```

---

## Running Tests

### Command Line

```bash
# All tests
uv run python -m data_bridge.test tests/ -v

# Specific directory
uv run python tests/unit/ -v
uv run python tests/integration/ -v

# Specific file
uv run python tests/unit/test_validation.py

# Parallel execution (default)
uv run python -m data_bridge.test tests/ --parallel

# Sequential execution
uv run python -m data_bridge.test tests/ --sequential

# With coverage
uv run python -m data_bridge.test tests/ --coverage --format=html

# Filter by name
uv run python -m data_bridge.test tests/ -k "test_create"

# Stop on first failure
uv run python -m data_bridge.test tests/ -x
```

### Programmatic

```python
from data_bridge.test import discover_tests, TestRunner

# Discover tests
tests = discover_tests("tests/unit/")

# Run tests
runner = TestRunner()
result = await runner.run_all(tests)

# Check results
print(f"Passed: {result.passed}/{result.total}")
```

---

## Benchmarks

For performance testing:

```python
from data_bridge.test import BenchmarkGroup, register_group

# Create benchmark group
group = BenchmarkGroup("Insert Performance")

@group.add("data-bridge")
async def bench_data_bridge_insert():
    user = await User(email="test@example.com").save()
    await user.delete()

@group.add("baseline")
async def bench_baseline_insert():
    # Baseline implementation
    pass

# Register for discovery
register_group(group)

# Run
results = await group.run(rounds=5, warmup=2)
print(results.summary())
```

**Run benchmarks**:
```bash
uv run python benchmarks/bench_comparison.py --rounds 5 --warmup 2
```

---

## Migration from pytest

### Import Changes

```python
# Before (pytest)
import pytest

# After (data-bridge-test)
from data_bridge.test import TestSuite, test, fixture, expect, parametrize
```

### Decorator Changes

```python
# Fixtures
@pytest.fixture(scope="class")    → @fixture(scope="class")

# Async (removed - implicit)
@pytest.mark.asyncio               → (remove)

# Parametrize
@pytest.mark.parametrize(...)      → @parametrize(...)
```

### Assertion Changes

```python
# Before (pytest)
assert x == y
assert x > y
assert x is None
with pytest.raises(ValueError):
    func()

# After (data-bridge-test)
expect(x).to_equal(y)
expect(x).to_be_greater_than(y)
expect(x).to_be_none()
expect(lambda: func()).to_raise(ValueError)
```

### Automated Migration

Use the migration tool:

```bash
# Preview changes
python tools/migrate_to_data_bridge_test.py tests/unit/ --dry-run

# Migrate
python tools/migrate_to_data_bridge_test.py tests/unit/

# Validate
python tools/validate_migration.py tests/unit/
```

**See**: [tools/README.md](../tools/README.md) for complete migration guide

---

## Best Practices

### 1. Test Organization

```
tests/
├── unit/                  # Fast tests, no external dependencies
├── integration/           # Database/API tests
├── fixtures/              # Shared test fixtures
└── conftest.py           # Shared fixture definitions
```

### 2. Test Naming

- Use descriptive names: `test_user_creation_with_valid_email()`
- Group related tests in classes
- Use parametrize for variations

### 3. Fixtures

- Use appropriate scopes (function vs class vs module)
- Keep fixtures focused (single responsibility)
- Clean up resources in teardown

### 4. Assertions

- One logical assertion per test
- Use descriptive expect() calls
- Test both positive and negative cases

### 5. Performance

- Use `async`/`await` for I/O operations
- Run tests in parallel when possible
- Mock external services

---

## Comparison: pytest vs data-bridge-test

| Feature | pytest | data-bridge-test | Winner |
|---------|--------|------------------|--------|
| **Speed** | Baseline | 2-5x faster | ✅ data-bridge-test |
| **Async** | Plugin required | Native | ✅ data-bridge-test |
| **Fixtures** | ✅ | ✅ | Equal |
| **Parametrize** | ✅ | ✅ | Equal |
| **Test Server** | Manual | Auto-managed | ✅ data-bridge-test |
| **Setup/Teardown** | ✅ | ✅ | Equal |
| **Parallel Execution** | Plugin | Native | ✅ data-bridge-test |
| **Error Messages** | Python | Rust-backed | ✅ data-bridge-test |

---

## Troubleshooting

### Tests Not Found

Ensure files follow naming convention:
- Files: `test_*.py` or `*_test.py`
- Classes: `Test*`
- Methods: `test_*()`

### Import Errors

```bash
# Rebuild extension
uv run maturin develop

# Check installation
uv run python -c "import data_bridge.test; print(data_bridge.test.__version__)"
```

### Async Issues

All tests are async by default. No `@pytest.mark.asyncio` needed:

```python
@test
async def test_something(self):  # ✅ Correct
    result = await async_function()
```

### Database Connection

For integration tests, ensure MongoDB/PostgreSQL is running:

```bash
# MongoDB
docker run -d -p 27017:27017 mongo

# PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres
```

---

## Further Reading

- [Migration Guide](../tools/README.md) - Migrate from pytest
- [Migration Examples](../tools/EXAMPLES.md) - Real-world scenarios
- [CLAUDE.md](../CLAUDE.md) - Project testing standards
- [API Reference](../crates/data-bridge-test/) - Rust implementation details

---

## Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/data-bridge/issues)
- **Documentation**: See `docs/` directory
- **Examples**: See `tests/` directory for real examples

---

**data-bridge-test**: Native Rust performance meets Python simplicity ✨
