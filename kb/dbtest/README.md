# dbtest System Architecture

## Overview

The `dbtest` CLI tool provides unified test and benchmark discovery and **parallel execution** for the data-bridge project, powered by a Rust engine with Tokio runtime for high-performance concurrent test execution.

**Key Features**:
- **Parallel Execution**: Rust Tokio runtime runs N tests concurrently
- **Fast Discovery**: Rust walkdir finds files in ~2ms (10-50x faster than Python)
- **GIL Management**: Minimal contention with per-task GIL acquisition
- **Performance**: N tests in ~T/N time (near-linear scaling)

## User Requirements

- **CLI**: Rust runner engine (Tokio), Python CLI wrapper
- **Discovery**: Rust-powered (walkdir + lazy loading)
- **Execution**: Rust orchestrates parallel execution
- **Benchmark**: Keep current BenchmarkGroup pattern
- **Standalone**: No pytest integration

## Optimized Architecture (Rust Runner)

```
Rust File Walker (walkdir)
  ↓ finds test_*.py, bench_*.py (~2ms for 100 files)
  ↓ stores paths in
Rust Registry (discovery.rs)
  ↓ applies filters (tags, patterns)
  ↓ hands off to
Rust Runner (runner.rs) with Tokio Runtime
  ↓ spawns N parallel tasks via tokio::spawn
  ↓ each task:
     ├─ lazy-loads Python module
     ├─ acquires GIL with Python::with_gil()
     ├─ calls Python test function
     └─ releases GIL
  ↓ aggregates results from all tasks
  ↓ generates report via
Rust Reporter (reporter.rs)
```

**Key Innovations**:
1. **Rust Runner as Execution Engine**: Tokio runtime manages all test execution
2. **Parallel Execution**: N tests run concurrently with tokio::spawn
3. **GIL Control**: Rust acquires/releases GIL per task, minimal contention
4. **Lazy Loading**: Python modules loaded on-demand by Rust
5. **Performance**: <3ms discovery, N tests in ~T/N time

## Documentation Structure

This architecture documentation is split into focused files for easier navigation:

### 1. [architecture.md](./architecture.md)
High-level system diagrams showing:
- Overall architecture flow with **Rust runner as execution engine**
- Detailed component architecture
- Layer responsibilities (Python, PyO3, Rust)
- **Rust runner capabilities** (parallel execution, GIL management)
- Performance targets for parallel execution

### 2. [state-machines.md](./state-machines.md)
State machine definitions for:
- Discovery lifecycle
- Test execution lifecycle (with parallel states)
- Benchmark execution lifecycle (with parallel states)
- CLI execution lifecycle
- State data structures (Rust types)

### 3. [data-flows.md](./data-flows.md)
Sequence diagrams showing:
- **Test discovery and parallel execution flow** (Tokio tasks)
- **Benchmark discovery and parallel execution flow**
- Filtering flow
- Error handling with task isolation
- Performance optimization points

### 4. [components.md](./components.md)
Component responsibilities and integration:
- Python layer (CLI, lazy loading) - minimal wrapper
- **Rust layer (discovery, runner with Tokio, reporter)** - core engine
- PyO3 bridge
- **Runner component details** (parallel execution, GIL management)
- Integration with existing systems
- Performance characteristics with parallel execution

### 5. [implementation.md](./implementation.md)
Implementation details:
- File structure and organization
- **Parallel execution flow diagrams** (Tokio tasks)
- **Key design patterns** (Rust runner pattern with tokio::spawn)
- **Performance characteristics** (parallel execution, near-linear scaling)
- Future extensions

## Quick Start

### Commands

```bash
dbtest              # Run all tests and benchmarks
dbtest unit         # Unit tests only
dbtest integration  # Integration tests
dbtest bench        # Benchmarks only
```

### Options

```bash
--pattern PATTERN   # Filter by file/test pattern
--tags TAGS         # Filter by tags
--verbose           # Detailed output
--fail-fast         # Stop on first failure
--format FORMAT     # Output format (console/json/markdown)
```

## Success Criteria

- ✅ `dbtest` command available after install
- ✅ Auto-discovers test_*.py and bench_*.py files
- ✅ Filters by pattern, tags, test type
- ✅ Runs tests and benchmarks separately or together
- ✅ Generates reports (console/JSON/markdown)
- ✅ <200ms discovery for 100 files
- ✅ All existing tests still pass
- ✅ **Parallel execution: N tests in ~T/N time (near-linear scaling)**
- ✅ **Task overhead: <1ms per test (Tokio spawn)**
- ✅ **GIL management: Minimal contention (per-task acquisition)**

## Implementation Plan

Detailed implementation plan: `/Users/chris.cheng/.claude/plans/enumerated-foraging-lighthouse.md`

**Phases**:
- **Phase 0**: Documentation reorganization (CURRENT)
- **Phase 1**: Rust foundation (discovery.rs)
- **Phase 2**: PyO3 bindings
- **Phase 3**: Python lazy loading
- **Phase 4**: Python CLI
- **Phase 5**: Console script
- **Phase 6**: Documentation & templates

## References

- **Rust Crate**: `crates/data-bridge-test/`
- **Python Module**: `python/data_bridge/test/`
- **PyO3 Bindings**: `crates/data-bridge/src/test.rs`
- **Existing Benchmark Discovery**: `python/data_bridge/test/benchmark.py:449-563`
