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
- [ ] 3.4 Verify all test files work with `ob qc run`.
    - **Partial**: 33 tests pass (tests without DB fixtures)
    - **Blocked**: Tests with fixture parameters (`test_table`, `sample_data`) don't run
    - **Fix needed**: Change test classes to inherit `PostgresSuite` instead of `TestSuite`

## 4. Cleanup
- [ ] 4.1 Remove `pytest` dependencies/markers from postgres test files.
    - `conftest.py` still has pytest fixtures (can delete after fixture migration)
- [ ] 4.2 Update `just test-pg` (or equivalent command) to use `ob qc run` instead of `pytest`.
