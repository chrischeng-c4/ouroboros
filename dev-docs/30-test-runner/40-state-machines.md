---
title: Test Framework State Machines
status: implemented
component: data-bridge-test
type: state-machine
---

# State Machines

> Part of [Test Framework Documentation](./index.md)

This document defines the state machines for the dbtest system, including discovery, test execution, benchmark execution, and CLI workflows.

## Discovery State Machine

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Scanning: Start discovery
    Scanning --> Filtering: Files found
    Filtering --> Ready: Filters applied
    Scanning --> Empty: No files found
    Scanning --> Failed: Walk error
    Empty --> [*]
    Failed --> [*]
    Ready --> [*]

    note right of Scanning
        Rust: walkdir crate
        find test_*.py, bench_*.py
        ~2ms for 100 files
    end note

    note right of Filtering
        Rust: Apply filters
        tags, patterns, types
        Store FileInfo structs
    end note
```

**States**:
- **Idle**: Initial state, no discovery started
- **Scanning**: Rust walkdir traversing file system
- **Filtering**: Applying pattern/tag filters in Rust
- **Ready**: FileInfo list ready for lazy loading
- **Empty**: No matching files found
- **Failed**: File system error during walk

## Test Execution State Machine

```mermaid
stateDiagram-v2
    [*] --> Discovered
    Discovered --> Filtered: Apply filters
    Filtered --> Queued: Selected for run
    Queued --> SettingUp: Start test suite
    SettingUp --> Ready: Setup complete
    Ready --> Running: Execute test
    Running --> TearingDown: Test complete
    TearingDown --> Completed: Teardown done

    Filtered --> Skipped: Filtered out
    SettingUp --> Failed: Setup error
    Running --> Passed: Assertions pass
    Running --> Failed: Assertions fail
    Running --> Error: Exception/timeout
    TearingDown --> Error: Teardown error

    Skipped --> [*]
    Passed --> [*]
    Failed --> [*]
    Error --> [*]
    Completed --> [*]

    note right of SettingUp
        Run setup_suite()
        Initialize fixtures
        Lazy load Python module
    end note

    note right of Running
        Execute test function
        Collect metrics
        Check assertions
    end note

    note right of TearingDown
        Run teardown_suite()
        Cleanup resources
    end note
```

**States**:
- **Discovered**: Test found during file discovery (Rust)
- **Filtered**: After tag/pattern filtering (Rust)
- **Queued**: Scheduled for execution
- **SettingUp**: Running setup_suite() (Python module loaded here)
- **Ready**: Setup complete, ready to run test
- **Running**: Test function executing
- **TearingDown**: Running teardown_suite()
- **Completed**: All phases done
- **Passed/Failed/Error/Skipped**: Final states (TestStatus)

## Benchmark Execution State Machine

```mermaid
stateDiagram-v2
    [*] --> Discovered
    Discovered --> Filtered: Apply filters
    Filtered --> Queued: Selected for run
    Queued --> Calibrating: Start benchmark
    Calibrating --> WarmingUp: Iterations determined
    WarmingUp --> Running: Warmup complete
    Running --> Analyzing: Rounds complete
    Analyzing --> Completed: Stats calculated

    Filtered --> Skipped: Filtered out
    Calibrating --> Failed: Calibration error
    WarmingUp --> Failed: Warmup error
    Running --> Failed: Execution error

    Skipped --> [*]
    Failed --> [*]
    Completed --> [*]

    note right of Calibrating
        Auto-calibrate iterations
        Target: 100ms per round
        Lazy load Python module
    end note

    note right of WarmingUp
        Run 3 warmup rounds
        Prime caches
    end note

    note right of Running
        Execute 5 rounds
        Collect timing data
    end note

    note right of Analyzing
        Calculate statistics:
        - Mean, median, stddev
        - Percentiles (P25, P50, P75, P95, P99)
        - Outlier detection
        - Confidence intervals
    end note
```

**States**:
- **Discovered**: Benchmark found during file discovery (Rust)
- **Filtered**: After pattern filtering (Rust)
- **Queued**: Scheduled for execution
- **Calibrating**: Determining iteration count (Python module loaded here)
- **WarmingUp**: Running warmup rounds
- **Running**: Executing timed rounds
- **Analyzing**: Computing statistics
- **Completed**: All phases done, stats ready
- **Skipped/Failed**: Terminal states

## CLI Execution State Machine

```mermaid
stateDiagram-v2
    [*] --> Parsing
    Parsing --> Discovering: Args parsed
    Discovering --> Registering: Files discovered
    Registering --> Filtering: Metadata in Rust
    Filtering --> Executing: Filtered set ready
    Executing --> Reporting: Execution complete
    Reporting --> [*]: Report displayed

    Parsing --> Error: Invalid args
    Discovering --> Error: Path not found
    Registering --> Error: Walk error
    Executing --> Error: Execution failed

    Error --> [*]: Exit code 1

    note right of Parsing
        Python: argparse
        Validate CLI options
    end note

    note right of Discovering
        Rust: walkdir
        Find test/bench files
    end note

    note right of Filtering
        Rust: Registry
        Apply pattern/tag filters
    end note

    note right of Executing
        Rust: Runner
        Lazy load + execute
    end note

    note right of Reporting
        Rust: Reporter
        Format: console/json/md
    end note
```

**States**:
- **Parsing**: CLI argument parsing (Python)
- **Discovering**: File discovery (Rust walkdir)
- **Registering**: Creating FileInfo structs (Rust)
- **Filtering**: Applying filters (Rust)
- **Executing**: Running tests/benchmarks (Rust + lazy Python loading)
- **Reporting**: Generating and displaying report (Rust)
- **Error**: Any error occurred, will exit with code 1

## State Transitions & Triggers

### Test Lifecycle Transitions

| From State | Trigger | To State | Action |
|------------|---------|----------|--------|
| Discovered | Filter match | Filtered | Add to filtered set |
| Discovered | Filter mismatch | Skipped | Mark as skipped |
| Filtered | Execution start | Queued | Add to execution queue |
| Queued | Runner picks up | SettingUp | Lazy load module, call setup_suite() |
| SettingUp | Setup succeeds | Ready | Mark ready |
| SettingUp | Setup fails | Failed | Record error |
| Ready | Test starts | Running | Execute test function |
| Running | All assertions pass | Passed | Record success |
| Running | Assertion fails | Failed | Record failure |
| Running | Exception raised | Error | Record error |
| Running | Timeout | Error | Record timeout |
| Passed/Failed/Error | Cleanup needed | TearingDown | Call teardown_suite() |
| TearingDown | Teardown succeeds | Completed | Finalize |
| TearingDown | Teardown fails | Error | Record teardown error |

### Benchmark Lifecycle Transitions

| From State | Trigger | To State | Action |
|------------|---------|----------|--------|
| Discovered | Filter match | Filtered | Add to filtered set |
| Discovered | Filter mismatch | Skipped | Mark as skipped |
| Filtered | Execution start | Queued | Add to execution queue |
| Queued | Runner picks up | Calibrating | Lazy load module, determine iterations |
| Calibrating | Iterations found | WarmingUp | Run warmup rounds |
| Calibrating | Calibration fails | Failed | Record error |
| WarmingUp | Warmup done | Running | Run timed rounds |
| WarmingUp | Warmup fails | Failed | Record error |
| Running | All rounds done | Analyzing | Calculate statistics |
| Running | Round fails | Failed | Record error |
| Analyzing | Stats computed | Completed | Finalize results |

## State Data Structures (Rust)

### Overall Execution State

```rust
/// Overall execution state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionState {
    Idle,
    Discovering,
    Filtering,
    Executing,
    Reporting,
    Completed,
    Failed(String),
}
```

### Test Lifecycle State

```rust
/// Test lifecycle state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestLifecycleState {
    Discovered,
    Filtered,
    Queued,
    SettingUp,
    Ready,
    Running,
    TearingDown,
    Completed(TestStatus),  // Passed/Failed/Error/Skipped
}
```

### Benchmark Lifecycle State

```rust
/// Benchmark lifecycle state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkLifecycleState {
    Discovered,
    Filtered,
    Queued,
    Calibrating,
    WarmingUp,
    Running { current_round: usize, total_rounds: usize },
    Analyzing,
    Completed,
    Failed(String),
}
```

### Test Status (Final Outcome)

```rust
/// Test status (final outcome) - EXISTING in runner.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}
```

### State Tracking

```rust
/// Tracks test execution state
pub struct TestExecution {
    meta: TestMeta,
    state: TestLifecycleState,
    started_at: Option<Instant>,
    completed_at: Option<Instant>,
    result: Option<TestResult>,
}

/// Tracks benchmark execution state
pub struct BenchmarkExecution {
    meta: BenchmarkMeta,
    state: BenchmarkLifecycleState,
    calibration_iterations: Option<usize>,
    warmup_results: Vec<Duration>,
    round_results: Vec<Duration>,
    stats: Option<BenchmarkStats>,
}
```

## State Persistence

**In-Memory Only**:
- States are tracked during execution in Rust
- No persistence to disk
- State machine resets on each CLI invocation

**Rationale**:
- Simplicity: No cache invalidation issues
- Performance: <3ms discovery is fast enough to repeat
- Reliability: Always fresh, no stale state

## See Also

- [Architecture](./00-architecture.md) - System architecture diagrams
- [Data Flows](./20-data-flows.md) - Sequence diagrams
- [Implementation](./30-implementation-details.md) - File structure and patterns
