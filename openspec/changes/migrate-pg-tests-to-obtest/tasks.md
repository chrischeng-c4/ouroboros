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
- [ ] 3.2 Migrate remaining `python/tests/postgres/unit/*.py` files (23 files remaining).
- [ ] 3.3 Migrate `python/tests/postgres/integration/*.py` files (20 files).
- [ ] 3.4 Verify all 45 test files work with `ob-test` runner.

## 4. Cleanup
- [ ] 4.1 Remove `pytest` dependencies/markers from postgres test files.
- [ ] 4.2 Update `just test-pg` (or equivalent command) to use `dbtest` instead of `pytest`.
