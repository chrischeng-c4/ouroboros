## 1. Implementation
- [x] 1.1 **Enhance `TestSuite`**: Modify `python/ouroboros/qc/suite.py` to:
    - Discover `@fixture` decorated methods during initialization.
    - Register fixture metadata with `FixtureRegistry` (Rust).
    - Implement a `FixtureRunner` or similar logic within `TestSuite` to manage fixture lifecycle and cache (scopes).
    - Update `run()` (and `_run_parallel`) to resolve dependencies and inject fixture values into test methods.
    - Handle `yield` (generators) for setup/teardown in both sync and async fixtures.
- [x] 1.2 **Update `FixtureScope` Support**: Ensure `FixtureScope.Class` and `FixtureScope.Module` are correctly handled (setup once, cached, teardown at end of scope).
- [x] 1.3 **Async Support**: Ensure async fixtures are properly awaited and async generators (`yield`) are handled using `anext()`.
- [x] 1.4 **Update Migration Tool**: Modify `python/tools/migrate_to_ouroboros_test.py` to:
    - Convert `@pytest.fixture` to `@fixture`.
    - Preserve `yield` syntax.
    - Ensure dependencies are correctly mapped.

## 2. Testing
- [x] 2.1 **Unit Tests**: Add tests in `python/tests/test_fixtures_runtime.py` (new file) to verify:
    - Dependency injection (param matching).
    - Fixture ordering (dependencies run first).
    - Lifecycle (setup -> test -> teardown).
    - Scoping (caching of class/module fixtures).
    - Async fixture execution.
    - Error handling (fixture failure fails the test).
- [x] 2.2 **Migration Tests**: Verify the migration tool correctly transforms a sample pytest file with fixtures.
