## ADDED Requirements

### Requirement: Assertion API
The system SHALL provide a fluent assertion API for validating test outcomes, including value equality and exception handling.

#### Scenario: Assert Exception Raised
- **WHEN** a test expects a specific exception
- **AND** the code under test is executed within `expect(lambda: ...).to_raise(...)`
- **THEN** the test passes if the exception is raised
- **AND** the test fails if no exception (or the wrong exception) is raised

### Requirement: Test Fixtures
The system SHALL provide a fixture mechanism to manage test setup, teardown, and dependency injection.

#### Scenario: Async Fixture Injection
- **WHEN** a test method requests a fixture by name as an argument
- **THEN** the fixture is initialized (asynchronously if needed)
- **AND** the result is passed to the test method
- **AND** the fixture is torn down after the test completes
