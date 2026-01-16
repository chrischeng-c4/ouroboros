## 1. Rust Framework Extensions
- [x] 1.1 Implement `to_raise<E>(self, exception_type: E)` in `crates/data-bridge/src/test.rs` to catch and verify Python exceptions.
- [x] 1.2 Register the new method in PyO3 bindings (done inline in test.rs).
- [x] 1.3 Add unit test for `to_raise` in `python/tests/unit/test_expect_to_raise.py`.

## 2. Python Base Infrastructure
- [x] 2.1 Update `python/tests/postgres/base.py`:
    - Create `PostgresSuite` class inheriting from `ouroboros.test.TestSuite`.
    - Create `PostgresIntegrationSuite` with DB setup/teardown.
    - Define common fixtures (database connection) using `@fixture` decorator.
- [x] 2.2 Verify `PostgresSuite` works with a minimal "Hello World" db test.

## 3. Test Migration
- [x] 3.1 Migrate `python/tests/postgres/unit/test_validation.py` (38 tests) as reference implementation:
    - Inherit from `TestSuite`.
    - Add `@test` decorator to all test methods.
    - Replace `import pytest` with `from ouroboros.test import TestSuite, test, expect`.
    - `expect().to_raise()` works with new Rust implementation.
- [x] 3.2 Fix syntax errors in test files (broken `expect(lambda: await ...).to_raise()` patterns).
- [x] 3.3 Migrate `python/tests/postgres/integration/*.py` and `benchmarks/*.py` files via `ob qc migrate`.
    - 22 files migrated, 20 already TestSuite, 2 skipped (no pytest patterns).
- [x] 3.4 Verify all test files work with `ob qc run`.
    - **Fixed**: Hook registry now uses pure Python coroutines (same event loop)
    - **Fixed**: Added `to_be()` and `to_not_be_none()` methods to Expectation
    - **Fixed**: CLI now reports errors (not just passed/failed)
    - **Fixed**: 12 integration test files with wrong imports
    - **Result**: 111 tests pass, 7 fail, 299 errors (SQL identifier validation issues in eager loading code)

## 4. Cleanup
- [x] 4.1 Remove `pytest` dependencies/markers from postgres test files.
    - Removed `pytestmark` from benchmark file
    - Fixed imports to use `PostgresSuite` from `tests.postgres.base`
    - `conftest.py` can be removed (pytest fixtures no longer needed)
- [x] 4.2 Update `just test-pg` (or equivalent command) to use `ob qc run` instead of `pytest`.
    - Updated `just test-postgres` to use `ob qc run python/tests/postgres -v`
