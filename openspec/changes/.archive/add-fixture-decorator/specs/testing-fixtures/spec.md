# Capability: Testing Fixtures

The `ouroboros.qc` framework SHALL support a pytest-compatible fixture system to manage test resources and dependencies.

## ADDED Requirements

### Requirement: Fixture Definition
The system SHALL provide a `@fixture` decorator to mark functions/methods as fixtures with a specific scope.

#### Scenario: Defining a fixture
- **WHEN** a developer decorates a method with `@fixture(scope="class")`
- **THEN** the method is registered as a fixture available to tests in that class (and subclasses).
- **AND** the fixture is instantiated once per test class.

### Requirement: Lifecycle Management (Yield)
The system SHALL support `yield` statements in fixtures to separate setup and teardown logic.

#### Scenario: Setup and Teardown
- **WHEN** a fixture contains code before and after a `yield` statement
- **THEN** the code before `yield` executes before the test (or scope start).
- **AND** the yielded value is injected into the test.
- **AND** the code after `yield` executes after the test (or scope end), even if the test fails.

### Requirement: Automatic Injection
The system SHALL automatically inject fixtures into test methods based on parameter names.

#### Scenario: Injecting a fixture
- **WHEN** a test method signature is `async def test_foo(self, my_fixture):`
- **AND** a fixture named `my_fixture` exists in the registry
- **THEN** the return value (or yielded value) of `my_fixture` is passed as the `my_fixture` argument.

### Requirement: Dependency Resolution
The system SHALL resolve dependencies between fixtures (fixtures requesting other fixtures).

#### Scenario: Fixture depending on fixture
- **WHEN** `fixture_b` requests `fixture_a` (via argument)
- **AND** a test requests `fixture_b`
- **THEN** `fixture_a` is set up first.
- **AND** `fixture_b` receives the value of `fixture_a`.
- **AND** `fixture_b` is injected into the test.
- **AND** teardown occurs in reverse order (`b` then `a`).

### Requirement: Async Support
The system SHALL support `async` fixtures and `async` generators.

#### Scenario: Async fixture
- **WHEN** a fixture is defined as `async def my_fix(): ... yield val ...`
- **THEN** the runner awaits the setup phase.
- **AND** the runner awaits the teardown phase.

### Requirement: Migration Support
The system SHALL provide a tool to migrate existing `pytest` fixtures to `ouroboros.qc` fixtures.

#### Scenario: Migrating pytest fixtures
- **WHEN** `migrate_to_ouroboros_test.py` runs on a file with `@pytest.fixture`
- **THEN** it converts it to `@fixture`.
- **AND** preserves the `yield` logic.
- **AND** converts `@pytest.mark.usefixtures` if applicable (or warns).
