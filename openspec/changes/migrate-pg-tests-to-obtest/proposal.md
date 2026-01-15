# Change: Migrate Postgres Tests to ob-test

## Why
The current PostgreSQL tests in `python/tests/postgres` use a hybrid setup: `pytest` as the runner but `ouroboros.test` for some assertions. This creates dependency on `pytest`, slower execution due to Python-side discovery/execution overhead, and inconsistency with the project's "Rust-first" philosophy. Migrating to `ob-test` (the native Rust test runner) will unify the testing infrastructure and enable future performance optimizations.

## What Changes
- **Rust Framework**: Implement `to_raise` assertion in `Expectation` struct to replace `pytest.raises`.
- **Python Tests**:
  - Convert `PostgresTestBase` to inherit from `ouroboros.test.TestSuite`.
  - Replace `pytest.fixture` usage with `ouroboros.test.fixture`.
  - Replace `pytest.mark.asyncio` with native async test support.
  - Fix syntax errors in 8 files (await usage).
  - Update all 45 test files to match `ob-test` conventions.

## Impact
- **Affected Specs**: `test-framework`
- **Affected Code**: 
  - `python/tests/postgres/**`
  - `crates/data-bridge-test/src/assertions.rs`
