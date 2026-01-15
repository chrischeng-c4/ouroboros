## ADDED Requirements

### Requirement: Rust-Based Discovery
The system SHALL use a Rust-based file walker for rapid test discovery.

#### Scenario: Discovery Speed
- **WHEN** `ob qc collect` is run on a large project
- **THEN** file discovery completes in under 3ms for 100 files

### Requirement: TestSuite Auto-Collection
The system SHALL automatically discover and collect test cases by identifying Python classes that inherit from `TestSuite`, without requiring explicit `if __name__ == "__main__"` blocks.

#### Scenario: Class Discovery
- **WHEN** a file contains `class MyTest(TestSuite): ...`
- **THEN** `ob qc run` detects and executes the tests within `MyTest`
- **AND** no manual registration is required

### Requirement: Parallel Execution
The system SHALL execute tests in parallel using a Tokio-based scheduler.

#### Scenario: Concurrent Tests
- **WHEN** multiple tests are selected
- **THEN** they run concurrently on available CPU cores
- **AND** the total execution time is reduced

### Requirement: Lazy Module Loading
The system SHALL load Python test modules only when they are required for execution.

#### Scenario: Filtered Run
- **WHEN** a specific test file is selected via `-k` or path
- **THEN** only that file and its dependencies are imported
- **AND** unrelated test files are not loaded

### Requirement: Async Hook Management
The system SHALL correctly execute asynchronous `setup_method` and `teardown_method` hooks, automatically ensuring the necessary runtime context (e.g., database connections) is available.

#### Scenario: Database in Hooks
- **WHEN** a test class defines `async def setup_method(self): await MyModel.find_one(...)`
- **THEN** the hook executes successfully without `RuntimeError` regarding missing loops or connections
- **AND** the test method runs immediately after

### Requirement: Pattern Filtering
The system SHALL support filtering tests by name or pattern using the `-k` flag.

#### Scenario: Filter by Name
- **WHEN** user runs `ob qc run -k "auth"`
- **THEN** only tests containing "auth" in their name (case-insensitive) are executed

### Requirement: Benchmark Support
The system SHALL provide native support for defining and running benchmarks.

#### Scenario: Benchmark Execution
- **WHEN** a benchmark is defined
- **THEN** it can be run via `ob qc run --bench`
- **AND** performance metrics are reported
