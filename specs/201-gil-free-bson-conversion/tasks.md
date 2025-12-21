# Tasks: GIL-Free BSON Conversion

**Input**: Design documents from `/specs/201-gil-free-bson-conversion/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: This feature follows TDD principles per CLAUDE.md constitution. All test tasks are REQUIRED before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Rust crates**: `crates/data-bridge/src/`, `crates/data-bridge-test/src/`
- **Python package**: `python/data_bridge/`
- **Tests**: `tests/unit/`, `tests/integration/`, `tests/mongo/benchmarks/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create core conversion utilities that all user stories depend on

- [ ] T001 Create new Rust module `crates/data-bridge/src/conversion.rs` with module declaration
- [ ] T002 Add `mod conversion;` and `pub use conversion::*;` to `crates/data-bridge/src/lib.rs`
- [ ] T003 [P] Define `SerializablePyValue` enum with 13 BSON type variants in `crates/data-bridge/src/conversion.rs`
- [ ] T004 [P] Define `ConversionContext` struct with security config, max_depth, max_size in `crates/data-bridge/src/conversion.rs`
- [ ] T005 [P] Define `ConversionError` enum with 8 error variants and `From<ConversionError> for PyErr` impl in `crates/data-bridge/src/conversion.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core conversion functions that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Rust Unit Tests (Write FIRST)

- [ ] T006 [P] Write test `test_extract_null` for None → SerializablePyValue::Null in `crates/data-bridge/src/conversion.rs`
- [ ] T007 [P] Write test `test_extract_bool` for bool → SerializablePyValue::Bool in `crates/data-bridge/src/conversion.rs`
- [ ] T008 [P] Write test `test_extract_int_i32_range` for int → SerializablePyValue::Int in `crates/data-bridge/src/conversion.rs`
- [ ] T009 [P] Write test `test_extract_int_i64_range` for large int → SerializablePyValue::Int in `crates/data-bridge/src/conversion.rs`
- [ ] T010 [P] Write test `test_extract_float` for float → SerializablePyValue::Float with NaN/Inf in `crates/data-bridge/src/conversion.rs`
- [ ] T011 [P] Write test `test_extract_string` for str → SerializablePyValue::String with unicode in `crates/data-bridge/src/conversion.rs`
- [ ] T012 [P] Write test `test_extract_bytes` for bytes → SerializablePyValue::Bytes in `crates/data-bridge/src/conversion.rs`
- [ ] T013 [P] Write test `test_extract_list_simple` for list → SerializablePyValue::List in `crates/data-bridge/src/conversion.rs`
- [ ] T014 [P] Write test `test_extract_list_nested` for nested list with depth check in `crates/data-bridge/src/conversion.rs`
- [ ] T015 [P] Write test `test_extract_dict_simple` for dict → SerializablePyValue::Dict in `crates/data-bridge/src/conversion.rs`
- [ ] T016 [P] Write test `test_extract_dict_nested` for nested dict with depth limit in `crates/data-bridge/src/conversion.rs`
- [ ] T017 [P] Write test `test_extract_objectid` for ObjectId/str(24 hex) → SerializablePyValue::ObjectId in `crates/data-bridge/src/conversion.rs`
- [ ] T018 [P] Write test `test_extract_datetime` for datetime → SerializablePyValue::DateTime in `crates/data-bridge/src/conversion.rs`
- [ ] T019 [P] Write test `test_depth_limit_exceeded` expecting DepthLimitExceeded error in `crates/data-bridge/src/conversion.rs`
- [ ] T020 [P] Write test `test_document_size_limit` expecting DocumentTooLarge error in `crates/data-bridge/src/conversion.rs`
- [ ] T021 [P] Write test `test_invalid_objectid` expecting InvalidObjectId error in `crates/data-bridge/src/conversion.rs`

### Core Conversion Functions (Implement to pass tests)

- [ ] T022 Implement `extract_py_value(py: Python, value: &Bound<PyAny>, context: &ConversionContext) -> PyResult<SerializablePyValue>` in `crates/data-bridge/src/conversion.rs`
- [ ] T023 Implement `extract_dict_items(py: Python, dict: &Bound<PyDict>, context: &ConversionContext) -> PyResult<Vec<(String, SerializablePyValue)>>` in `crates/data-bridge/src/conversion.rs`
- [ ] T024 [P] Implement `serializable_to_bson(value: &SerializablePyValue) -> Result<Bson, ConversionError>` (GIL-free) in `crates/data-bridge/src/conversion.rs`
- [ ] T025 [P] Implement `items_to_bson_document(items: &[(String, SerializablePyValue)]) -> Result<BsonDocument, ConversionError>` (GIL-free) in `crates/data-bridge/src/conversion.rs`
- [ ] T026 [P] Implement `bson_to_serializable(bson: &Bson) -> SerializablePyValue` (GIL-free) in `crates/data-bridge/src/conversion.rs`
- [ ] T027 Implement `serializable_to_py_dict(py: Python, value: &SerializablePyValue) -> PyResult<Bound<PyDict>>` in `crates/data-bridge/src/conversion.rs`

### Rust Unit Tests for Conversion Functions

- [ ] T028 [P] Write test `test_serializable_to_bson_all_types` for type matrix in `crates/data-bridge/src/conversion.rs`
- [ ] T029 [P] Write test `test_bson_to_serializable_all_types` for inverse type matrix in `crates/data-bridge/src/conversion.rs`
- [ ] T030 [P] Write test `test_roundtrip_equivalence` for Python → Serializable → BSON → Serializable → Python in `crates/data-bridge/src/conversion.rs`

### Build and Verify Foundation

- [ ] T031 Run `cargo test -p data-bridge` to verify all conversion tests pass
- [ ] T032 Run `maturin develop` to build Python extension with new conversion module

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Fast Single Document Retrieval (Priority: P1) 🎯 MVP

**Goal**: Optimize find_one operation to complete in ≤3.5ms by releasing GIL during BSON conversion

**Independent Test**: Execute concurrent find_one queries measuring (a) latency ≤3.5ms and (b) no thread blocking

**Performance Target**: FR-005 - find_one completes in ≤3.5ms (vs current 8.9ms)

### Tests for User Story 1 (Write FIRST) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T033 [P] [US1] Create test file `tests/unit/test_gil_release.py` with test infrastructure
- [ ] T034 [P] [US1] Write test `test_concurrent_find_one_no_gil_blocking` measuring concurrent execution time in `tests/unit/test_gil_release.py`
- [ ] T035 [P] [US1] Write test `test_find_one_latency_target` asserting ≤3.5ms in `tests/mongo/benchmarks/bench_find_one.py`
- [ ] T036 [P] [US1] Create test file `tests/integration/test_conversion_semantics.py`
- [ ] T037 [P] [US1] Write test `test_find_one_preserves_all_bson_types` with type matrix in `tests/integration/test_conversion_semantics.py`
- [ ] T038 [P] [US1] Write test `test_find_one_nested_documents` for complex nested structures in `tests/integration/test_conversion_semantics.py`
- [ ] T039 [P] [US1] Write test `test_find_one_error_messages_unchanged` comparing error outputs in `tests/integration/test_conversion_semantics.py`

### Implementation for User Story 1

- [ ] T040 [US1] Refactor `find_one` function in `crates/data-bridge/src/mongodb.rs` lines 1165-1203 to use two-phase conversion
- [ ] T041 [US1] Extract filter dict using `extract_dict_items` (GIL held) in `find_one` before async block
- [ ] T042 [US1] Move filter conversion to `py.allow_threads(|| items_to_bson_document(&filter_items))` inside async block
- [ ] T043 [US1] Convert BSON result to PyDict using `bson_to_serializable` then `serializable_to_py_dict` in `find_one`
- [ ] T044 [US1] Remove intermediate `RustDocument.to_dict()` call in `python/data_bridge/_engine.py` find_one function (line 60-64)
- [ ] T045 [US1] Update `find_one` in `_engine.py` to return PyDict directly from Rust

### Validation for User Story 1

- [ ] T046 [US1] Run `cargo test -p data-bridge test_find` to verify Rust unit tests pass
- [ ] T047 [US1] Run `uv run pytest tests/unit/test_gil_release.py::test_concurrent_find_one_no_gil_blocking -v` to verify GIL release
- [ ] T048 [US1] Run `MONGODB_URI="mongodb://localhost:27017/bench" uv run python tests/mongo/benchmarks/bench_find_one.py` and verify ≤3.5ms
- [ ] T049 [US1] Run `uv run pytest tests/integration/test_conversion_semantics.py::test_find_one* -v` to verify semantic equivalence
- [ ] T050 [US1] Run `SKIP_INTEGRATION=true uv run pytest tests/` to ensure no regression in existing tests

**Checkpoint**: At this point, User Story 1 should be fully functional - find_one is ≤3.5ms with GIL released

---

## Phase 4: User Story 2 - Efficient Bulk Updates (Priority: P1) 🎯 MVP

**Goal**: Optimize update_many operation to complete in ≤150ms by releasing GIL during BSON conversion

**Independent Test**: Execute update_many with 300-document filter and measure execution time ≤150ms

**Performance Target**: FR-006 - update_many completes in ≤150ms (vs current 805ms)

### Tests for User Story 2 (Write FIRST) ⚠️

- [ ] T051 [P] [US2] Write test `test_concurrent_update_many_no_gil_blocking` in `tests/unit/test_gil_release.py`
- [ ] T052 [P] [US2] Write test `test_update_many_latency_target` asserting ≤150ms in `tests/mongo/benchmarks/bench_update.py`
- [ ] T053 [P] [US2] Write test `test_update_many_preserves_semantics` comparing update results in `tests/integration/test_conversion_semantics.py`
- [ ] T054 [P] [US2] Write test `test_update_many_complex_operations` with nested field updates in `tests/integration/test_conversion_semantics.py`

### Implementation for User Story 2

- [ ] T055 [US2] Refactor `update_many` function in `crates/data-bridge/src/mongodb.rs` lines 1597-1629 to use two-phase conversion
- [ ] T056 [US2] Extract filter and update dicts using `extract_dict_items` (GIL held) before async block in `update_many`
- [ ] T057 [US2] Move filter/update conversion to `py.allow_threads(|| items_to_bson_document(&items))` inside async block
- [ ] T058 [US2] Remove operator check logic (lines 1616-1620) from `update_many` - trust MongoDB validation

### Validation for User Story 2

- [ ] T059 [US2] Run `cargo test -p data-bridge test_update` to verify Rust unit tests pass
- [ ] T060 [US2] Run `uv run pytest tests/unit/test_gil_release.py::test_concurrent_update_many_no_gil_blocking -v`
- [ ] T061 [US2] Run `MONGODB_URI="mongodb://localhost:27017/bench" uv run python tests/mongo/benchmarks/bench_update.py` and verify ≤150ms
- [ ] T062 [US2] Run `uv run pytest tests/integration/test_conversion_semantics.py::test_update_many* -v` to verify semantic equivalence
- [ ] T063 [US2] Run `SKIP_INTEGRATION=true uv run pytest tests/` to ensure no regression

**Checkpoint**: User Stories 1 AND 2 both achieve performance targets independently

---

## Phase 5: User Story 3 - Concurrent Read Operations (Priority: P2)

**Goal**: Verify concurrent operations scale linearly with <10% overhead at 100 concurrent threads

**Independent Test**: Run 100 simultaneous find operations and measure <10% latency increase vs sequential

**Performance Target**: SC-003 - Concurrent operations show <10% latency increase

### Tests for User Story 3 (Write FIRST) ⚠️

- [ ] T064 [P] [US3] Create benchmark file `tests/mongo/benchmarks/bench_gil_contention.py`
- [ ] T065 [P] [US3] Write test `test_100_concurrent_find_one` measuring concurrent vs sequential latency in `bench_gil_contention.py`
- [ ] T066 [P] [US3] Write test `test_mixed_operations_concurrent` with find/update/insert mix in `bench_gil_contention.py`
- [ ] T067 [P] [US3] Write test `test_sustained_load_1000_rps` measuring p95 latency under load in `bench_gil_contention.py`

### Implementation for User Story 3

- [ ] T068 [P] [US3] Refactor `find` (find_many) function in `crates/data-bridge/src/mongodb.rs` lines 1215-1261 to use two-phase conversion
- [ ] T069 [US3] Update `find` to return list of PyDict directly (skip RustDocument intermediate)
- [ ] T070 [US3] Remove `result.to_dict()` calls in `python/data_bridge/_engine.py` find function

### Validation for User Story 3

- [ ] T071 [US3] Run `MONGODB_URI="mongodb://localhost:27017/bench" uv run python tests/mongo/benchmarks/bench_gil_contention.py`
- [ ] T072 [US3] Verify concurrent overhead <10% in benchmark output
- [ ] T073 [US3] Run `uv run pytest tests/integration/test_conversion_semantics.py` to verify no semantic changes

**Checkpoint**: All P1 and P2 user stories functional with concurrent scalability verified

---

## Phase 6: User Story 4 - All CRUD Operations Benefit (Priority: P3)

**Goal**: Apply GIL-release optimization to remaining 5 operations without performance regression

**Independent Test**: Run full benchmark suite comparing all 8 operations before/after

**Performance Target**: FR-007 - All 313+ tests pass, no performance regression on any operation

### Tests for User Story 4 (Write FIRST) ⚠️

- [ ] T074 [P] [US4] Write test `test_insert_one_semantics_preserved` in `tests/integration/test_conversion_semantics.py`
- [ ] T075 [P] [US4] Write test `test_insert_many_semantics_preserved` in `tests/integration/test_conversion_semantics.py`
- [ ] T076 [P] [US4] Write test `test_update_one_semantics_preserved` in `tests/integration/test_conversion_semantics.py`
- [ ] T077 [P] [US4] Write test `test_delete_one_semantics_preserved` in `tests/integration/test_conversion_semantics.py`
- [ ] T078 [P] [US4] Write test `test_delete_many_semantics_preserved` in `tests/integration/test_conversion_semantics.py`

### Implementation for User Story 4

- [ ] T079 [P] [US4] Refactor `insert_one` function in `crates/data-bridge/src/mongodb.rs` lines 1263-1307 to use two-phase conversion
- [ ] T080 [P] [US4] Refactor `insert_many` function in `crates/data-bridge/src/mongodb.rs` lines 1309-1463 to use two-phase conversion
- [ ] T081 [P] [US4] Refactor `update_one` function in `crates/data-bridge/src/mongodb.rs` lines 1465-1595 to use two-phase conversion
- [ ] T082 [P] [US4] Refactor `delete_one` function in `crates/data-bridge/src/mongodb.rs` lines 1640-1670 to use two-phase conversion
- [ ] T083 [P] [US4] Refactor `delete_many` function in `crates/data-bridge/src/mongodb.rs` lines 1672-1704 to use two-phase conversion

### Validation for User Story 4

- [ ] T084 [US4] Run `cargo test -p data-bridge` to verify all Rust unit tests pass
- [ ] T085 [US4] Run `uv run pytest tests/integration/test_conversion_semantics.py -v` to verify all operations preserve semantics
- [ ] T086 [US4] Run `uv run pytest tests/` to verify all 313+ Python tests pass without modification
- [ ] T087 [US4] Run `MONGODB_URI="mongodb://localhost:27017/bench" uv run python tests/mongo/benchmarks/bench_insert.py` and verify no regression
- [ ] T088 [US4] Run `MONGODB_URI="mongodb://localhost:27017/bench" uv run python tests/mongo/benchmarks/bench_delete.py` and verify no regression

**Checkpoint**: All user stories (US1-US4) complete, all 8 operations optimized

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Code cleanup, documentation, final validation

### Code Quality

- [ ] T089 [P] Run `cargo clippy -- -D warnings` on `crates/data-bridge` and fix all warnings
- [ ] T090 [P] Run `cargo fmt` on all Rust code
- [ ] T091 [P] Add documentation comments to all public functions in `crates/data-bridge/src/conversion.rs`
- [ ] T092 [P] Add module-level documentation to `crates/data-bridge/src/conversion.rs` explaining GIL-release pattern

### Edge Cases Validation

- [ ] T093 [P] Write test `test_unsupported_type_error` for custom Python objects in `tests/integration/test_conversion_semantics.py`
- [ ] T094 [P] Write test `test_16mb_document_handling` for maximum size documents in `tests/integration/test_conversion_semantics.py`
- [ ] T095 [P] Write test `test_error_messages_exact_match` comparing current vs new error messages in `tests/integration/test_conversion_semantics.py`

### Final Validation

- [ ] T096 Run full test suite: `cargo test && uv run pytest tests/ -v`
- [ ] T097 Run all benchmarks and record results in `.benchmarks/` directory
- [ ] T098 Verify performance targets met: find_one ≤3.5ms, update_many ≤150ms, concurrent <10% overhead
- [ ] T099 Run `cargo audit` to check for security vulnerabilities
- [ ] T100 Update CHANGELOG.md with performance improvements and technical details

### Documentation

- [ ] T101 [P] Update README.md with performance comparison table vs Beanie
- [ ] T102 [P] Add GIL-release pattern documentation to CONTRIBUTING.md
- [ ] T103 [P] Update API documentation in `python/data_bridge/` docstrings if needed

**Checkpoint**: Feature complete, all tests passing, documentation updated

---

## Dependencies and Execution Order

### Critical Path (Must Complete Sequentially)

```
Phase 1 (Setup) → Phase 2 (Foundation) → User Stories can proceed in parallel
```

### User Story Dependencies

```
US1 (find_one) ──┐
                 ├──→ US3 (concurrent reads) - depends on US1 pattern proven
US2 (update_many)┘

US4 (all operations) - depends on US1 + US2 patterns validated
```

### Parallel Execution Opportunities

**After Phase 2 completes:**

- US1 and US2 can be implemented in parallel (different operations, both P1)
- Within each US: All [P] tasks can run in parallel
- Tests for different user stories can be written in parallel

**Example parallel execution for US1:**
```bash
# Terminal 1: Write tests
pytest tests/unit/test_gil_release.py  # T033-T034

# Terminal 2: Write more tests
pytest tests/integration/test_conversion_semantics.py  # T036-T039

# Terminal 3: Implement (after tests fail)
nvim crates/data-bridge/src/mongodb.rs  # T040-T043

# Terminal 4: Update Python layer
nvim python/data_bridge/_engine.py  # T044-T045
```

---

## Implementation Strategy

### MVP Scope (Minimum Viable Product)

**Recommended MVP**: User Story 1 (find_one optimization) ONLY

**Rationale**:
- Proves GIL-release pattern works
- Delivers immediate value (most common operation)
- Validates performance assumptions (2.5x improvement)
- Provides foundation for other operations

**MVP Task Range**: T001-T050 (50 tasks)

**MVP Success Criteria**:
- find_one completes in ≤3.5ms ✓
- No thread blocking during concurrent find_one ✓
- All existing tests pass ✓
- BSON type semantics preserved ✓

### Incremental Delivery Plan

1. **Sprint 1** (Week 1): Phase 1 + Phase 2 + Phase 3 (US1)
   - Deliver: find_one optimization
   - Validate: Performance target met, no regressions

2. **Sprint 2** (Week 2): Phase 4 (US2)
   - Deliver: update_many optimization
   - Validate: Both US1 and US2 work independently

3. **Sprint 3** (Week 2-3): Phase 5 (US3) + Phase 6 (US4)
   - Deliver: Concurrent scalability + all operations optimized
   - Validate: Full feature complete

4. **Sprint 4** (Week 3): Phase 7 (Polish)
   - Deliver: Documentation, final validation
   - Validate: Ready for PR

### Testing Strategy

**TDD Workflow** (per CLAUDE.md constitution):
1. Write test (ensure it fails)
2. Get user approval if needed
3. Implement until test passes
4. Refactor
5. Commit

**Test Pyramid**:
- Rust unit tests (T006-T030): Fast, comprehensive type coverage
- Python integration tests (T033-T095): Semantic equivalence, real MongoDB
- Benchmarks (T035, T048, T052, T061, T071): Performance validation

---

## Task Summary

**Total Tasks**: 103

**Task Breakdown by Phase**:
- Phase 1 (Setup): 5 tasks
- Phase 2 (Foundation): 27 tasks (21 tests + 6 implementation)
- Phase 3 (US1): 18 tasks (7 tests + 11 implementation/validation)
- Phase 4 (US2): 13 tasks (4 tests + 9 implementation/validation)
- Phase 5 (US3): 10 tasks (4 tests + 6 implementation/validation)
- Phase 6 (US4): 15 tasks (5 tests + 10 implementation/validation)
- Phase 7 (Polish): 15 tasks (3 tests + 12 quality/docs)

**Parallel Opportunities**: 56 tasks marked [P] (54% parallelizable)

**User Story Coverage**:
- US1 (P1): 18 tasks - Fast single document retrieval
- US2 (P1): 13 tasks - Efficient bulk updates
- US3 (P2): 10 tasks - Concurrent read operations
- US4 (P3): 15 tasks - All CRUD operations benefit

**MVP Scope**: Tasks T001-T050 (49% of total) delivers fully functional find_one optimization

**Success Criteria Validated**:
- ✅ FR-005: find_one ≤3.5ms (validated in T048)
- ✅ FR-006: update_many ≤150ms (validated in T061)
- ✅ FR-007: All 313+ tests pass (validated in T086, T096)
- ✅ FR-008: GIL released (validated in T047, T060, T071)
- ✅ SC-003: Concurrent <10% overhead (validated in T071-T072)

---

**Ready for implementation!** Start with Phase 1 (T001-T005) to set up the conversion module infrastructure.
