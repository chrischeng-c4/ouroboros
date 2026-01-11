# pytest to data-bridge-test Migration Tools

Automated tools for migrating pytest tests to the data-bridge-test framework.

## Tools

### 1. `migrate_to_data_bridge_test.py`

AST-based migration tool that automatically transforms pytest code to data-bridge-test.

**Features:**
- Converts `import pytest` to `from data_bridge.test import ...`
- Transforms decorators: `@pytest.fixture` → `@fixture`
- Removes `@pytest.mark.asyncio` (implicit async support)
- Converts `@pytest.mark.parametrize` → `@parametrize`
- Transforms assertions: `assert x == y` → `expect(x).to_equal(y)`
- Adds `TestSuite` base class to test classes
- Adds `@test` decorator to test methods
- Converts `pytest.raises()` to `expect().to_raise()`

**Usage:**

```bash
# Migrate single file
python tools/migrate_to_data_bridge_test.py tests/unit/test_example.py

# Dry-run (preview changes without modifying)
python tools/migrate_to_data_bridge_test.py tests/unit/*.py --dry-run

# Recursively migrate all tests
python tools/migrate_to_data_bridge_test.py tests/ --recursive

# Save warnings to file
python tools/migrate_to_data_bridge_test.py tests/ --recursive --warnings warnings.txt
```

**Transformation Examples:**

```python
# Before (pytest)
import pytest

class TestExample:
    @pytest.fixture(scope="class")
    def data(self):
        return {"value": 42}

    @pytest.mark.parametrize("x", [1, 2, 3])
    @pytest.mark.asyncio
    async def test_something(self, x, data):
        assert x > 0
        assert data["value"] == 42

        with pytest.raises(ValueError):
            raise ValueError("error")

# After (data-bridge-test)
from data_bridge.test import TestSuite, test, fixture, expect, parametrize

class TestExample(TestSuite):
    @fixture(scope='class')
    def data(self):
        return {'value': 42}

    @parametrize('x', [1, 2, 3])
    @test
    async def test_something(self, x, data):
        expect(x).to_be_greater_than(0)
        expect(data['value']).to_equal(42)

        expect(lambda: raise ValueError('error')).to_raise(ValueError)
```

### 2. `validate_migration.py`

Validation tool that ensures migrated tests preserve behavior.

**Features:**
- Runs both pytest and data-bridge-test on the same files
- Compares test counts (total, passed, failed, skipped)
- Generates detailed reports
- Saves results to JSON

**Usage:**

```bash
# Validate single file
python tools/validate_migration.py tests/unit/test_example.py

# Validate all tests recursively
python tools/validate_migration.py tests/ --recursive

# Skip pytest (for already migrated files)
python tools/validate_migration.py tests/ --recursive --skip-pytest

# Save report to JSON
python tools/validate_migration.py tests/ --recursive --report=validation.json
```

**Example Output:**

```
Found 5 test file(s)

Validating: tests/unit/test_example.py
  Running pytest...
    5/5 tests passed
  Running data-bridge-test...
    5/5 tests passed
  Result: ✓ PASS

============================================================
Validation Report
============================================================
Total files:      5
Matching:         5
Mismatched:       0
Total issues:     0

Details:
  ✓ tests/unit/test_example.py
     pytest:       5/5 passed
     data-bridge:  5/5 passed
```

## Transformation Rules

| pytest Pattern | data-bridge-test Equivalent | Notes |
|----------------|----------------------------|-------|
| `import pytest` | `from data_bridge.test import TestSuite, test, expect, ...` | |
| `@pytest.fixture(scope="class")` | `@fixture(scope="class")` | |
| `@pytest.mark.asyncio` | (removed) | Implicit async support |
| `@pytest.mark.parametrize("x", [1,2,3])` | `@parametrize("x", [1,2,3])` | |
| `assert x == y` | `expect(x).to_equal(y)` | |
| `assert x != y` | `expect(x).to_not_equal(y)` | |
| `assert x > y` | `expect(x).to_be_greater_than(y)` | |
| `assert x < y` | `expect(x).to_be_less_than(y)` | |
| `assert x >= y` | `expect(x).to_be_greater_than_or_equal(y)` | |
| `assert x <= y` | `expect(x).to_be_less_than_or_equal(y)` | |
| `assert x in y` | `expect(y).to_contain(x)` | Note: operands swapped |
| `assert x not in y` | `expect(y).to_not_contain(x)` | Note: operands swapped |
| `assert x is None` | `expect(x).to_be_none()` | |
| `assert x is not None` | `expect(x).to_not_be_none()` | |
| `assert x` | `expect(x).to_be_truthy()` | |
| `assert not x` | `expect(x).to_be_falsy()` | |
| `with pytest.raises(E):` | `expect(lambda: ...).to_raise(E)` | Converts to lambda |
| `class TestFoo:` | `class TestFoo(TestSuite):` | Adds base class |
| `def test_*():` | `@test\ndef test_*():` | Adds decorator |

## Error Handling

The migration tool handles edge cases gracefully:

- **Syntax errors**: Reports error and continues with next file
- **Unsupported patterns**: Preserves original code and emits warning
- **Complex comparisons**: Keeps as-is with warning for manual review
- **AST generation errors**: Reports failure without corrupting file

## Testing

The tools include comprehensive tests:

```bash
# Test migration tool
uv run python tests/tools/test_migrate_tool.py

# Test validation tool
uv run python tests/tools/test_validate_tool.py
```

**Test fixtures:**
- `tests/tools/fixtures/simple_pytest.py` - Example pytest file for testing

## Limitations

Current limitations (require manual migration):

1. **Complex pytest.raises patterns**: Some complex uses of `pytest.raises` with message matching
2. **Fixture chains**: Very complex fixture dependency chains may need review
3. **Custom pytest plugins**: Plugin-specific functionality must be migrated manually
4. **pytest.mark.skip/xfail**: Not yet supported (will preserve with warning)
5. **Indirect parametrization**: `pytest.mark.parametrize(..., indirect=True)` not supported

## Workflow

Recommended migration workflow:

```bash
# 1. Migrate tests with dry-run to preview changes
python tools/migrate_to_data_bridge_test.py tests/unit/ --recursive --dry-run

# 2. Review warnings
python tools/migrate_to_data_bridge_test.py tests/unit/ --recursive --warnings warnings.txt
cat warnings.txt

# 3. Migrate for real
python tools/migrate_to_data_bridge_test.py tests/unit/ --recursive

# 4. Validate migration
python tools/validate_migration.py tests/unit/ --recursive --report=validation.json

# 5. Review validation report
cat validation.json | jq '.summary'

# 6. Fix any issues identified
# ... manual fixes ...

# 7. Re-validate
python tools/validate_migration.py tests/unit/ --recursive --skip-pytest
```

## Contributing

When adding new transformation rules:

1. Update `PytestToDataBridgeTransformer` in `migrate_to_data_bridge_test.py`
2. Add test case in `tests/tools/test_migrate_tool.py`
3. Update transformation table in this README
4. Test on real pytest files to ensure it works

## See Also

- [data-bridge-test Documentation](../python/data_bridge/test/__init__.py)
- [Test Suite Guide](../python/data_bridge/test/suite.py)
- [Assertion API](../crates/data-bridge-test/src/assertion.rs)
