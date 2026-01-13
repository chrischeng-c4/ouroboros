# Migration Tool Examples

Real-world examples of using the pytest-to-data-bridge-test migration tools.

## Example 1: Simple Migration

### Before (pytest)

```python
# tests/unit/test_user.py
import pytest

class TestUser:
    def test_create_user(self):
        user = {"name": "Alice", "age": 30}
        assert user["name"] == "Alice"
        assert user["age"] > 0

    def test_update_user(self):
        user = {"name": "Bob", "age": 25}
        user["age"] = 26
        assert user["age"] == 26
```

### Command

```bash
python tools/migrate_to_data_bridge_test.py tests/unit/test_user.py
```

### After (data-bridge-test)

```python
# tests/unit/test_user.py
from data_bridge.test import TestSuite, test, fixture, expect, parametrize

class TestUser(TestSuite):
    @test
    def test_create_user(self):
        user = {'name': 'Alice', 'age': 30}
        expect(user['name']).to_equal('Alice')
        expect(user['age']).to_be_greater_than(0)

    @test
    def test_update_user(self):
        user = {'name': 'Bob', 'age': 25}
        user['age'] = 26
        expect(user['age']).to_equal(26)
```

## Example 2: Fixtures

### Before (pytest)

```python
import pytest

class TestDatabase:
    @pytest.fixture(scope="class")
    def db_connection(self):
        conn = create_connection()
        yield conn
        conn.close()

    @pytest.fixture
    def test_data(self):
        return [1, 2, 3, 4, 5]

    def test_query(self, db_connection, test_data):
        results = db_connection.query("SELECT * FROM data")
        assert len(results) == len(test_data)
```

### Command

```bash
python tools/migrate_to_data_bridge_test.py tests/test_database.py
```

### After (data-bridge-test)

```python
from data_bridge.test import TestSuite, test, fixture, expect, parametrize

class TestDatabase(TestSuite):
    @fixture(scope='class')
    def db_connection(self):
        conn = create_connection()
        yield conn
        conn.close()

    @fixture
    def test_data(self):
        return [1, 2, 3, 4, 5]

    @test
    def test_query(self, db_connection, test_data):
        results = db_connection.query('SELECT * FROM data')
        expect(len(results)).to_equal(len(test_data))
```

## Example 3: Parametrized Tests

### Before (pytest)

```python
import pytest

class TestCalculator:
    @pytest.mark.parametrize("a,b,expected", [
        (1, 2, 3),
        (5, 3, 8),
        (10, -5, 5),
    ])
    def test_addition(self, a, b, expected):
        result = a + b
        assert result == expected

    @pytest.mark.parametrize("value", [1, 2, 3, 4, 5])
    @pytest.mark.parametrize("multiplier", [2, 10])
    def test_multiply(self, value, multiplier):
        result = value * multiplier
        assert result > 0
```

### Command

```bash
python tools/migrate_to_data_bridge_test.py tests/test_calculator.py
```

### After (data-bridge-test)

Note: The migration tool converts `@pytest.mark.parametrize` to `@parametrize`, but you'll need to manually adjust parameter format if using tuples.

```python
from data_bridge.test import TestSuite, test, fixture, expect, parametrize

class TestCalculator(TestSuite):
    # Manual adjustment needed: split tuple parameters
    @test
    @parametrize("a", [1, 5, 10])
    @parametrize("b", [2, 3, -5])
    @parametrize("expected", [3, 8, 5])
    def test_addition(self, a, b, expected):
        result = a + b
        expect(result).to_equal(expected)

    @test
    @parametrize('value', [1, 2, 3, 4, 5])
    @parametrize('multiplier', [2, 10])
    def test_multiply(self, value, multiplier):
        result = value * multiplier
        expect(result).to_be_greater_than(0)
```

## Example 4: Async Tests

### Before (pytest)

```python
import pytest

class TestAsyncAPI:
    @pytest.mark.asyncio
    async def test_fetch_user(self):
        user = await fetch_user(123)
        assert user is not None
        assert user["id"] == 123

    @pytest.mark.asyncio
    async def test_create_user(self):
        user = await create_user({"name": "Alice"})
        assert user["name"] == "Alice"
```

### Command

```bash
python tools/migrate_to_data_bridge_test.py tests/test_async_api.py
```

### After (data-bridge-test)

```python
from data_bridge.test import TestSuite, test, fixture, expect, parametrize

class TestAsyncAPI(TestSuite):
    @test
    async def test_fetch_user(self):
        user = await fetch_user(123)
        expect(user).to_not_be_none()
        expect(user['id']).to_equal(123)

    @test
    async def test_create_user(self):
        user = await create_user({'name': 'Alice'})
        expect(user['name']).to_equal('Alice')
```

Note: `@pytest.mark.asyncio` is automatically removed since data-bridge-test has implicit async support.

## Example 5: Multiple Files Migration

### Directory Structure

```
tests/
├── unit/
│   ├── test_user.py
│   ├── test_auth.py
│   └── test_validation.py
└── integration/
    ├── test_api.py
    └── test_database.py
```

### Command

```bash
# Dry-run first to preview changes
python tools/migrate_to_data_bridge_test.py tests/ --recursive --dry-run

# Review warnings
python tools/migrate_to_data_bridge_test.py tests/ --recursive --warnings warnings.txt
cat warnings.txt

# Migrate for real
python tools/migrate_to_data_bridge_test.py tests/ --recursive

# Validate migration
python tools/validate_migration.py tests/ --recursive --report=validation.json
```

### Output

```
Found 5 test file(s)

✓ Migrated: tests/unit/test_user.py
✓ Migrated: tests/unit/test_auth.py
✓ Migrated: tests/unit/test_validation.py
✓ Migrated: tests/integration/test_api.py
✓ Migrated: tests/integration/test_database.py

============================================================
Migration complete:
  Success: 5
  Failed: 0
  Warnings: 3

Warnings:
  - tests/integration/test_api.py: pytest.raises() requires manual migration: with pytest.raises(ValueError)...
  - tests/integration/test_database.py: pytest.raises() requires manual migration: with pytest.raises(ConnectionError)...
  - tests/unit/test_validation.py: pytest.raises() requires manual migration: with pytest.raises(ValidationError)...
```

## Example 6: Assertion Conversions

### Before (pytest)

```python
def test_assertions():
    # Equality
    assert x == y
    assert x != y

    # Comparison
    assert x > y
    assert x < y
    assert x >= y
    assert x <= y

    # Membership
    assert x in items
    assert x not in items

    # Identity
    assert x is None
    assert x is not None

    # Boolean
    assert condition
    assert not condition
```

### After (data-bridge-test)

```python
@test
def test_assertions():
    # Equality
    expect(x).to_equal(y)
    expect(x).to_not_equal(y)

    # Comparison
    expect(x).to_be_greater_than(y)
    expect(x).to_be_less_than(y)
    expect(x).to_be_greater_than_or_equal(y)
    expect(x).to_be_less_than_or_equal(y)

    # Membership (note: operands swapped)
    expect(items).to_contain(x)
    expect(items).to_not_contain(x)

    # Identity
    expect(x).to_be_none()
    expect(x).to_not_be_none()

    # Boolean
    expect(condition).to_be_truthy()
    expect(condition).to_be_falsy()
```

## Example 7: Manual Migration (pytest.raises)

The migration tool cannot automatically convert `pytest.raises()` because it requires complex lambda conversion. Manual migration is needed.

### Before (pytest)

```python
def test_exception():
    with pytest.raises(ValueError):
        raise ValueError("Invalid value")

    with pytest.raises(KeyError) as exc_info:
        d = {}
        _ = d["missing_key"]
    assert "missing_key" in str(exc_info.value)
```

### After (data-bridge-test) - Manual

```python
@test
def test_exception():
    # Option 1: Use try/except with expect
    try:
        raise ValueError("Invalid value")
        expect(False).to_equal(True)  # Should not reach here
    except ValueError:
        pass  # Expected

    # Option 2: Keep pytest.raises (requires pytest)
    # This works but you'll need pytest as a dependency
    with pytest.raises(ValueError):
        raise ValueError("Invalid value")
```

## Example 8: Validation After Migration

After migrating, validate that tests still work correctly:

```bash
# Validate single file
python tools/validate_migration.py tests/unit/test_user.py

# Output:
# Validating: tests/unit/test_user.py
#   Running pytest...
#     5/5 tests passed
#   Running data-bridge-test...
#     5/5 tests passed
#   Result: ✓ PASS
#
# ============================================================
# Validation Report
# ============================================================
# Total files:      1
# Matching:         1
# Mismatched:       0
# Total issues:     0
```

## Example 9: Handling Warnings

The migration tool provides warnings for patterns that need manual review:

```bash
python tools/migrate_to_data_bridge_test.py tests/ --recursive --warnings warnings.txt
```

### warnings.txt

```
tests/unit/test_api.py: pytest.raises() requires manual migration: with pytest.raises(ValueError)...
tests/unit/test_api.py: Unsupported pytest.mark decorator: @pytest.mark.skip(reason="...")
tests/integration/test_db.py: pytest.raises() requires manual migration: with pytest.raises(ConnectionError)...
```

Review each warning and manually migrate the affected code.

## Best Practices

1. **Always use dry-run first**: Preview changes before applying them
   ```bash
   python tools/migrate_to_data_bridge_test.py tests/ --recursive --dry-run
   ```

2. **Save warnings for review**: Keep track of patterns requiring manual migration
   ```bash
   python tools/migrate_to_data_bridge_test.py tests/ --recursive --warnings warnings.txt
   ```

3. **Validate after migration**: Ensure tests still pass
   ```bash
   python tools/validate_migration.py tests/ --recursive --report=validation.json
   ```

4. **Commit in stages**: Don't migrate everything at once
   ```bash
   # Migrate one directory at a time
   python tools/migrate_to_data_bridge_test.py tests/unit/ --recursive
   # Test, commit
   python tools/migrate_to_data_bridge_test.py tests/integration/ --recursive
   # Test, commit
   ```

5. **Keep backups**: Make git commits before migration
   ```bash
   git add tests/
   git commit -m "backup before migration"
   python tools/migrate_to_data_bridge_test.py tests/ --recursive
   ```

## Troubleshooting

### Issue: Tests don't run after migration

**Cause**: Missing TestSuite base class or @test decorator

**Solution**: Manually add them:
```python
from data_bridge.test import TestSuite, test

class TestExample(TestSuite):  # Add TestSuite
    @test  # Add @test decorator
    def test_something(self):
        ...
```

### Issue: Fixtures not working

**Cause**: Fixture parameters not matching function signature

**Solution**: Ensure fixture names match function parameters:
```python
@fixture
def my_fixture(self):  # Fixture name: my_fixture
    return "value"

@test
def test_something(self, my_fixture):  # Parameter name must match
    expect(my_fixture).to_equal("value")
```

### Issue: Parametrize not generating tests

**Cause**: Parameter names don't match function signature

**Solution**: Check parameter names:
```python
@parametrize("value", [1, 2, 3])  # Parameter: "value"
def test_something(self, value):  # Must match
    expect(value).to_be_greater_than(0)
```

## See Also

- [Migration Tool README](README.md)
- [data-bridge-test Documentation](../python/data_bridge/test/__init__.py)
