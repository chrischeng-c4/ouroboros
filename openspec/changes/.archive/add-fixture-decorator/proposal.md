# Change: Add pytest-compatible Fixture System

## Why
The current testing framework (`ouroboros.qc`) lacks a robust fixture system with dependency injection and lifecycle management (`yield` support). Developers migrating from `pytest` expect these features. The current implementation has basic metadata support but no runtime execution logic for fixtures.

## What Changes
- **Runtime Support**: Implement fixture execution, dependency resolution, and value injection in `TestSuite`.
- **Lifecycle Management**: Add support for `yield`-based setup/teardown in fixtures (sync and async).
- **Scoping**: Implement `function`, `class`, `module` (suite-level), and `session` scopes.
- **Migration Tool**: Update `migrate_to_ouroboros_test.py` to automatically convert `pytest` fixtures to `ouroboros.qc` fixtures.
- **New Spec**: Create `testing-fixtures` capability spec.

## Impact
- **Affected Specs**: `testing-fixtures` (New)
- **Affected Code**:
    - `python/ouroboros/qc/suite.py`: Core logic for fixture resolution and execution.
    - `python/ouroboros/qc/decorators.py`: Enhanced `@fixture` decorator.
    - `python/tools/migrate_to_ouroboros_test.py`: Migration logic.
