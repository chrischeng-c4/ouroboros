# Implementation Plan: GIL-Free BSON Conversion

**Branch**: `201-gil-free-bson-conversion` | **Date**: 2025-12-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/201-gil-free-bson-conversion/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Optimize BSON conversion performance by releasing the Python Global Interpreter Lock (GIL) during conversion operations, targeting 2.5x improvement for find_one (8.9ms → 3.5ms) and 5.4x improvement for update_many (805ms → 150ms). Implementation uses a two-phase conversion pattern: extract Python data while holding GIL minimally, then convert to/from BSON with GIL released. This eliminates thread blocking during database operations, enabling true concurrent processing in multi-threaded Python applications.

**Key Innovation**: Move BSON conversion from synchronous (GIL-held) to asynchronous context (GIL-released) using intermediate representation that is Send + Sync in Rust.

## Technical Context

**Language/Version**: Rust 1.70+ (edition 2021), Python 3.12+
**Primary Dependencies**:
- PyO3 0.24+ (Python bindings with stable ABI)
- bson 2.13 (BSON serialization)
- mongodb 3.1 (Rust MongoDB driver)
- pyo3-async-runtimes (Python/Rust async bridge)

**Storage**: MongoDB 4.0+ (primary data store)
**Testing**:
- Rust: cargo test (unit tests for conversion functions)
- Python: pytest (integration tests, 313+ existing tests must pass)
- Benchmarks: custom benchmark framework in data-bridge-test crate

**Target Platform**: Linux/macOS/Windows (cross-platform via PyO3)
**Project Type**: Hybrid Rust library with Python bindings (cdylib + Python package)

**Performance Goals**:
- find_one: ≤3.5ms (current: 8.9ms, baseline: 5.4ms Beanie)
- update_many: ≤150ms (current: 805ms, baseline: 253ms Beanie)
- GIL hold time: <1ms per operation (current: holds GIL for entire conversion)
- Concurrent scalability: <10% latency increase at 100 concurrent operations

**Constraints**:
- Zero API changes (100% backward compatible)
- All 313+ Python tests pass unmodified
- Security validations preserved (collection/field name validation, query validation)
- Memory overhead ≤2x document size during conversion
- No semantic changes to BSON conversion (data types, error handling identical)

**Scale/Scope**:
- 8 MongoDB operations affected (find_one, find, insert_one, insert_many, update_one, update_many, delete_one, delete_many)
- ~500 lines of new Rust code (conversion utilities)
- ~200 lines of refactored Rust code (operation functions)
- 3 new test files (GIL verification, conversion correctness, performance regression)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Using CLAUDE.md principles as project constitution:

### Zero Python Byte Handling ✅
**Principle**: All BSON serialization/deserialization in Rust, minimizing Python heap pressure.
**Compliance**: This feature strengthens compliance by reducing GIL hold time during BSON operations. Conversion still happens 100% in Rust, just moved to async context.

### GIL Release Strategy ✅
**Principle**: Release GIL during BSON conversion and network I/O.
**Compliance**: **PRIMARY GOAL** - This feature directly implements this principle by releasing GIL during BSON conversion (currently violated).

### Security First ✅
**Principle**: Validate collection names, field names, prevent NoSQL injection.
**Compliance**: All security validations preserved without modification (FR-004, FR-006 in spec).

### Copy-on-Write State Management ✅
**Principle**: Field-level change tracking without full deepcopy.
**Compliance**: No impact - conversion optimization is orthogonal to state management.

### Lazy Validation ✅
**Principle**: Defer validation until save(), type validation in Rust.
**Compliance**: No impact - validation logic unchanged, just execution timing optimized.

### Beanie Compatibility ✅
**Principle**: Maintain compatible API for user migration.
**Compliance**: Zero API changes (FR-012), all tests pass unmodified (FR-007).

### Testing Requirements ✅
**Principle**: cargo test + pytest before commit, no regressions.
**Compliance**: Extensive testing mandated (User Stories 1-4, Edge Cases, FR-007).

### Performance Verification ✅
**Principle**: Run benchmarks before PR, verify targets met.
**Compliance**: Specific performance targets defined (FR-005, FR-006, SC-001, SC-002).

**GATE RESULT**: ✅ **PASS** - No constitutional violations. Feature directly addresses GIL Release Strategy principle gap.

## Project Structure

### Documentation (this feature)

```text
specs/201-gil-free-bson-conversion/
├── spec.md              # Feature specification (/speckit.specify output)
├── plan.md              # This file (/speckit.plan output)
├── research.md          # Phase 0 output (/speckit.plan - see below)
├── data-model.md        # Phase 1 output (/speckit.plan - see below)
├── quickstart.md        # Phase 1 output (/speckit.plan - see below)
├── contracts/           # Phase 1 output (/speckit.plan - see below)
│   └── bson-conversion-api.md
├── checklists/          # Quality gates
│   └── requirements.md  # Spec validation (already created)
└── tasks.md             # Phase 2 output (/speckit.tasks - NOT created yet)
```

### Source Code (repository root)

```text
crates/
├── data-bridge/                    # PyO3 bindings (main entry point)
│   └── src/
│       ├── lib.rs                  # Module registration
│       ├── mongodb.rs              # MODIFIED: All 8 operations refactored
│       ├── conversion.rs           # NEW: GIL-free conversion utilities
│       └── validation.rs           # Unchanged (security validation)
│
├── data-bridge-mongodb/            # Pure Rust MongoDB ORM
│   └── src/
│       ├── connection.rs           # Unchanged
│       ├── document.rs             # Unchanged
│       └── query.rs                # Unchanged
│
├── data-bridge-test/               # Rust test framework
│   └── src/
│       ├── benchmark.rs            # Unchanged (framework)
│       └── gil_monitor.rs          # NEW: GIL hold time measurement
│
└── data-bridge-common/             # Shared utilities
    └── src/
        └── error.rs                # Unchanged

python/data_bridge/
├── __init__.py                     # Unchanged
├── _engine.py                      # MODIFIED: Remove .to_dict() calls
├── document.py                     # Unchanged
├── query.py                        # Unchanged
└── (all other files unchanged)

tests/
├── unit/                           # Python unit tests
│   └── test_gil_release.py         # NEW: GIL verification tests
│
├── integration/                    # Python integration tests
│   └── test_conversion_semantics.py # NEW: Semantic equivalence tests
│
└── mongo/benchmarks/               # Performance tests
    ├── bench_find_one.py           # Unchanged (existing benchmark)
    ├── bench_update.py             # Unchanged (existing benchmark)
    └── bench_gil_contention.py     # NEW: Concurrent load test
```

**Structure Decision**: Hybrid Rust/Python project with PyO3 bridge. Changes concentrated in:
1. **crates/data-bridge/src/mongodb.rs** - Refactor 8 operation functions to use two-phase conversion
2. **crates/data-bridge/src/conversion.rs** - New module with GIL-free conversion utilities
3. **python/data_bridge/_engine.py** - Remove intermediate RustDocument.to_dict() calls
4. **tests/** - Add 3 new test files for GIL verification, semantic equivalence, performance regression

## Complexity Tracking

**No violations requiring justification** - All constitution checks passed.

This feature reduces complexity by:
- Eliminating "fast-path" vs "standard path" dichotomy (single code path with GIL release)
- Removing operator check overhead in update operations (Phase 3)
- Simplifying conversion flow (extract once, convert once vs current multiple passes)

