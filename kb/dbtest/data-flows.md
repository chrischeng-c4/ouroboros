# Data Flows

> Part of [dbtest Architecture Documentation](./README.md)

This document shows the sequence of operations and data flow through the dbtest system for different execution paths.

## Test Discovery & Execution Flow (Rust Runner)

```mermaid
sequenceDiagram
    participant User
    participant CLI as Python CLI
    participant PyO3 as PyO3 Bridge
    participant Walk as Rust File Walker
    participant Reg as Rust Registry
    participant Run as Rust Runner<br/>(Tokio)
    participant Task1 as Tokio Task 1
    participant Task2 as Tokio Task 2
    participant TaskN as Tokio Task N
    participant Lazy as Python Lazy Loader
    participant PyTest as Python Test Func
    participant Rep as Rust Reporter

    User->>CLI: dbtest unit
    CLI->>CLI: parse args
    CLI->>PyO3: hand off to Rust

    PyO3->>Walk: walk_files(config)
    Walk->>Walk: walkdir crate (~2ms)
    Walk->>Walk: filter by pattern (test_*.py)
    Walk-->>PyO3: Vec<FileInfo>

    PyO3->>Reg: register file paths
    Reg->>Reg: apply tag filters
    Reg-->>PyO3: filtered FileInfo list

    PyO3->>Run: execute_tests_parallel()
    Note over Run: Rust Tokio Runtime<br/>Manages Execution

    par Parallel Test Execution
        Run->>Task1: tokio::spawn(test_file_1)
        Task1->>Lazy: lazy_load_test_suite(file_1)
        Lazy-->>Task1: TestSuite classes
        Task1->>Task1: with_gil(py)
        Task1->>PyTest: call Python async test
        PyTest-->>Task1: result
        Task1->>Task1: GIL released
        Task1-->>Run: TestResult

    and
        Run->>Task2: tokio::spawn(test_file_2)
        Task2->>Lazy: lazy_load_test_suite(file_2)
        Lazy-->>Task2: TestSuite classes
        Task2->>Task2: with_gil(py)
        Task2->>PyTest: call Python async test
        PyTest-->>Task2: result
        Task2->>Task2: GIL released
        Task2-->>Run: TestResult

    and
        Run->>TaskN: tokio::spawn(test_file_N)
        TaskN->>Lazy: lazy_load_test_suite(file_N)
        Lazy-->>TaskN: TestSuite classes
        TaskN->>TaskN: with_gil(py)
        TaskN->>PyTest: call Python async test
        PyTest-->>TaskN: result
        TaskN->>TaskN: GIL released
        TaskN-->>Run: TestResult
    end

    Run->>Run: aggregate results
    Run->>Rep: generate_report(results)
    Rep->>Rep: format (console/json/md)
    Rep-->>CLI: formatted report
    CLI-->>User: display results
```

**Key Points**:
1. **Rust is the Runner**: Tokio runtime manages all test execution
2. **Parallel Execution**: Multiple tests run concurrently via tokio::spawn
3. **GIL Management**: Each task acquires/releases GIL as needed
4. **Fast Discovery**: Rust walkdir finds files in ~2ms
5. **Lazy Loading**: Python modules loaded on-demand by Rust
6. **Performance**: N tests in ~T/N time (near-linear scaling)

## Benchmark Discovery & Execution Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI as Python CLI
    participant PyO3 as PyO3 Bridge
    participant Walk as Rust File Walker
    participant Reg as Rust Registry
    participant Run as Rust Runner<br/>(Tokio)
    participant Task1 as Tokio Task 1
    participant Task2 as Tokio Task 2
    participant Lazy as Python Lazy Loader
    participant BG as BenchmarkGroup
    participant Rep as Rust Reporter

    User->>CLI: dbtest bench
    CLI->>PyO3: discover_benchmarks("tests/")

    PyO3->>Walk: walk_files(config)
    Walk->>Walk: walkdir crate (~2ms)
    Walk->>Walk: filter by pattern (bench_*.py)
    Walk-->>PyO3: Vec<FileInfo>

    PyO3->>Reg: register file paths
    Reg->>Reg: apply pattern filters
    Reg-->>PyO3: filtered FileInfo list

    PyO3->>Run: run_benchmarks_parallel()
    Note over Run: Rust Tokio Runtime<br/>Manages Benchmark Execution

    par Parallel Benchmark Execution
        Run->>Task1: tokio::spawn(bench_file_1)
        Task1->>Lazy: lazy_load_benchmark(file_1)
        Lazy->>Lazy: importlib.util.spec_from_file
        Note over Lazy,BG: Module execution triggers<br/>@decorator and register_group()
        Lazy-->>Task1: BenchmarkGroup instances
        Task1->>Task1: with_gil(py)
        Task1->>Task1: calibrate iterations
        Task1->>Task1: run warmup rounds (3x)
        Task1->>BG: execute timed rounds (5x)
        BG-->>Task1: timing results
        Task1->>Task1: calculate statistics
        Task1->>Task1: GIL released
        Task1-->>Run: BenchmarkResult

    and
        Run->>Task2: tokio::spawn(bench_file_2)
        Task2->>Lazy: lazy_load_benchmark(file_2)
        Lazy-->>Task2: BenchmarkGroup instances
        Task2->>Task2: with_gil(py)
        Task2->>Task2: calibrate + warmup + execute
        Task2->>BG: execute timed rounds
        BG-->>Task2: timing results
        Task2->>Task2: GIL released
        Task2-->>Run: BenchmarkResult
    end

    Run->>Run: aggregate results
    Run->>Rep: generate_comparison_report(results)
    Rep->>Rep: format comparison table
    Rep-->>CLI: formatted report
    CLI-->>User: display results
```

**Key Points**:
1. **Rust is the Runner**: Tokio runtime manages all benchmark execution
2. **Parallel Execution**: Multiple benchmark files run concurrently via tokio::spawn
3. **GIL Management**: Each task acquires/releases GIL as needed
4. **Same Discovery**: Uses same Rust walkdir approach
5. **Lazy Loading**: Benchmark files loaded on-demand by Rust
6. **Auto-Registration**: BenchmarkGroup registers during module import
7. **Statistics**: Mean, median, stddev, percentiles calculated in Rust
8. **Performance**: N benchmark files in ~T/N time (near-linear scaling)

## Filtering Flow

```mermaid
sequenceDiagram
    participant CLI as Python CLI
    participant Walk as Rust File Walker
    participant Reg as Rust Registry

    CLI->>Walk: walk_files(pattern="test_*.py")
    Walk-->>Walk: Find all test_*.py files

    Walk->>Reg: register(FileInfo { path, module_name })

    CLI->>Reg: filter_by_pattern("*crud*")
    Reg-->>Reg: Keep only files matching *crud*

    CLI->>Reg: filter_by_tags(["unit"])
    Note over Reg: Tags require loading module<br/>to inspect decorators
    Reg-->>Reg: Filtered FileInfo list

    Reg-->>CLI: Ready for execution
```

**Filtering Strategy**:
- **File Pattern**: Applied during walkdir (fast, no I/O)
- **Name Pattern**: Applied on FileInfo list (fast, string match)
- **Tags**: Requires lazy loading module to inspect decorators (slower)

## Error Handling Flow

```mermaid
sequenceDiagram
    participant CLI as Python CLI
    participant Walk as Rust File Walker
    participant Run as Rust Runner<br/>(Tokio)
    participant Task as Tokio Task
    participant Lazy as Python Lazy Loader

    CLI->>Walk: walk_files("tests/")

    alt File system error
        Walk-->>CLI: Error: Permission denied
        CLI-->>User: Exit code 1
    else Files found
        Walk-->>CLI: Vec<FileInfo>
    end

    CLI->>Run: execute_tests_parallel()

    Run->>Task: tokio::spawn(test_file)
    Task->>Lazy: lazy_load_test_suite(file_path)

    alt Import error
        Lazy-->>Task: Error: Module import failed
        Task->>Task: Mark test as Error
        Task-->>Run: TestResult::Error
    else Module loaded
        Lazy-->>Task: TestSuite classes
    end

    Task->>Task: with_gil(py)
    Task->>Task: execute_test()

    alt Test fails
        Task->>Task: Mark as Failed
        Task-->>Run: TestResult::Failed
    else Test passes
        Task->>Task: Mark as Passed
        Task-->>Run: TestResult::Passed
    end

    Task->>Task: GIL released

    Run->>Run: aggregate results from all tasks
    Run-->>CLI: TestResults (including errors)
    CLI-->>User: Display all results + error summary
```

**Error Handling Principles**:
- **Fail Fast for Discovery**: File system errors exit immediately
- **Collect Test Errors**: Import/execution errors are collected per task, not fatal
- **Task Isolation**: Each Tokio task handles its own errors independently
- **Final Report**: Rust runner aggregates all results including errors
- **Exit Code**: Non-zero if any tests failed or errored
- **GIL Safety**: GIL released even on error paths

## Performance Optimization Points

### Critical Path (Discovery to Execution)

```
1. CLI startup              ~200-300ms  (Python import overhead)
   ↓
2. walkdir file discovery   ~2-3ms      (Rust walkdir, 100 files)
   ↓
3. Filtering                ~0.1-1ms    (Rust string matching)
   ↓
4. Tokio runtime init       ~1-2ms      (Rust async runtime)
   ↓
5. Parallel execution       ~T/N        (N tests in parallel)
   │
   ├─ Lazy module loading   ~10-50ms    (Python importlib, per file, parallel)
   ├─ GIL acquire           ~0.1ms      (per task, minimal contention)
   ├─ Test execution        Variable    (User test code)
   └─ GIL release           ~0.1ms      (per task)
   ↓
6. Result aggregation       ~1-5ms      (Rust, concurrent collection)
   ↓
7. Report generation        ~10-50ms    (Rust formatting)
```

**Performance Gains from Rust Runner**:
- **Parallel Execution**: N tests in ~T/N time (near-linear scaling)
- **Task Scheduling**: <1ms overhead per task (Tokio)
- **GIL Management**: Minimal contention (released between Python calls)
- **Concurrent Loading**: Multiple modules load in parallel
- **Efficient Aggregation**: Rust collects results without Python overhead

**Bottlenecks**:
- **CLI Startup**: Python import overhead (~200-300ms) - unavoidable
- **Module Loading**: Mitigated by parallel loading (all files load concurrently)
- **Test Execution**: Dominated by actual test logic (but parallelized)

**Optimizations**:
- ✅ Use Rust walkdir (10-50x faster than Python glob)
- ✅ Lazy loading (don't load filtered-out files)
- ✅ Filtering in Rust (faster than Python)
- ✅ Parallel execution with Tokio (N tests → T/N time)
- ✅ GIL release management (no contention)
- ✅ Concurrent module loading (all files load in parallel)
- ❌ Not caching (complexity not worth <3ms savings)

## See Also

- [Architecture](./architecture.md) - System architecture
- [State Machines](./state-machines.md) - Lifecycle states
- [Components](./components.md) - Component responsibilities
