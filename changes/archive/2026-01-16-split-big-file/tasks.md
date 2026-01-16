# Implementation Tasks

## Phase 1: Critical Files (>3000 lines) - Must Split First

### Task 1.1: Split `query.rs` (4,430 lines) → `query/`
**Crate**: `ouroboros-postgres`
**Complexity**: High
**Dependencies**: None

| Step | Action | Target File | Est. Lines |
|------|--------|-------------|------------|
| 1 | Create `query/mod.rs` with re-exports | `query/mod.rs` | ~50 |
| 2 | Extract `Operator`, `OrderDirection` enums | `query/operator.rs` | ~150 |
| 3 | Extract `AggregateFunction`, `WindowFunction`, `WindowSpec` | `query/window.rs` | ~200 |
| 4 | Extract `CommonTableExpression`, `Subquery`, `SetOperation` | `query/cte.rs` | ~150 |
| 5 | Extract `JoinType`, `JoinCondition`, `JoinClause` | `query/join.rs` | ~150 |
| 6 | Extract `QueryBuilder` struct and core impl | `query/builder.rs` | ~400 |
| 7 | Move SELECT/WHERE logic | `query/select.rs` | ~500 |
| 8 | Move INSERT/UPDATE/DELETE logic | `query/modify.rs` | ~400 |
| 9 | Move tests to `query/tests.rs` | `query/tests.rs` | ~2000 |

### Task 1.2: Split `postgres.rs` (4,206 lines) → `postgres/`
**Crate**: `ouroboros`
**Complexity**: High
**Dependencies**: None

| Step | Action | Target File | Est. Lines |
|------|--------|-------------|------------|
| 1 | Create `postgres/mod.rs` with re-exports | `postgres/mod.rs` | ~100 |
| 2 | Extract `safe_call` panic boundary | `postgres/safety.rs` | ~50 |
| 3 | Extract `RowWrapper`, `OptionalRowWrapper`, `RowsWrapper` | `postgres/row.rs` | ~150 |
| 4 | Extract `py_value_to_extracted`, `extracted_to_py_value`, `extracted_to_json` | `postgres/conversion.rs` | ~300 |
| 5 | Extract connection pool (`PG_POOL`, `init`, `close`, `is_connected`) | `postgres/connection.rs` | ~200 |
| 6 | Extract CRUD operations (`insert_one`, `insert_many`, `fetch_*`) | `postgres/crud.rs` | ~800 |
| 7 | Extract relation operations (`fetch_*_with_relations`, `m2m_*`) | `postgres/relations.rs` | ~600 |
| 8 | Extract transaction handling (`PyTransaction`, `begin_transaction`) | `postgres/transaction.rs` | ~400 |
| 9 | Extract schema operations (`list_tables`, `describe_*`) | `postgres/schema.rs` | ~400 |

### Task 1.3: Split `qc.rs` (3,987 lines) → `qc/`
**Crate**: `ouroboros`
**Complexity**: High
**Dependencies**: None

| Step | Action | Target File | Est. Lines |
|------|--------|-------------|------------|
| 1 | Create `qc/mod.rs` with re-exports | `qc/mod.rs` | ~80 |
| 2 | Extract `PyTestType`, `PyTestStatus`, `PyReportFormat` enums | `qc/enums.rs` | ~150 |
| 3 | Extract `PyTestMeta`, `PyTestResult`, `PyTestSummary` | `qc/results.rs` | ~300 |
| 4 | Extract `PyTestRunner` | `qc/runner.rs` | ~400 |
| 5 | Extract `PyExpectation` (assertion library) | `qc/expect.rs` | ~400 |
| 6 | Extract `PyReporter`, `PyTestReport` | `qc/reporter.rs` | ~300 |
| 7 | Extract `PyCoverageInfo`, `PyFileCoverage` | `qc/coverage.rs` | ~200 |
| 8 | Extract benchmark classes (`PyBenchmarkStats`, `PyBenchmarkResult`, etc.) | `qc/benchmark.rs` | ~500 |
| 9 | Extract profiler classes (`PyProfilePhase`, `PyPhaseTiming`, etc.) | `qc/profiler.rs` | ~400 |
| 10 | Extract discovery classes | `qc/discovery.rs` | ~300 |

### Task 1.4: Split `mongodb.rs` (3,684 lines) → `mongodb/`
**Crate**: `ouroboros`
**Complexity**: High
**Dependencies**: None

| Step | Action | Target File | Est. Lines |
|------|--------|-------------|------------|
| 1 | Create `mongodb/mod.rs` with re-exports | `mongodb/mod.rs` | ~80 |
| 2 | Extract `safe_call` panic boundary | `mongodb/safety.rs` | ~50 |
| 3 | Extract type wrappers (`IndexInfo`, `UpdateResult`, etc.) | `mongodb/types.rs` | ~300 |
| 4 | Extract BSON conversion functions | `mongodb/conversion.rs` | ~400 |
| 5 | Extract connection pool management | `mongodb/connection.rs` | ~200 |
| 6 | Extract CRUD operations | `mongodb/crud.rs` | ~800 |
| 7 | Extract index management | `mongodb/index.rs` | ~300 |
| 8 | Extract aggregation pipeline | `mongodb/aggregation.rs` | ~400 |

## Phase 2: High Priority (2000-3000 lines)

### Task 2.1: Split `validation.rs` (2,070 lines) → `validation/`
**Crate**: `ouroboros-api`
**Complexity**: Medium
**Dependencies**: Phase 1 not required

| Step | Action | Target File | Est. Lines |
|------|--------|-------------|------------|
| 1 | Create `validation/mod.rs` | `validation/mod.rs` | ~50 |
| 2 | Extract `TypeDescriptor`, `TypeCategory` | `validation/types.rs` | ~200 |
| 3 | Extract constraint structs | `validation/constraints.rs` | ~300 |
| 4 | Extract validators | `validation/validators.rs` | ~500 |
| 5 | Extract error types | `validation/error.rs` | ~150 |

### Task 2.2: Split `schema.rs` (2,011 lines) → `schema/`
**Crate**: `ouroboros-postgres`
**Complexity**: Medium
**Dependencies**: Task 1.1

| Step | Action | Target File | Est. Lines |
|------|--------|-------------|------------|
| 1 | Create `schema/mod.rs` | `schema/mod.rs` | ~50 |
| 2 | Extract introspection queries | `schema/queries.rs` | ~400 |
| 3 | Extract type mapping | `schema/mapping.rs` | ~300 |
| 4 | Extract schema inspector | `schema/inspector.rs` | ~400 |

## Phase 3: Medium Priority (1000-2000 lines)

### Task 3.1-3.15: Remaining Files
Each file should follow the same pattern:
1. Analyze structure to identify logical boundaries
2. Create directory `<name>/`
3. Extract types to `types.rs`
4. Extract implementations to logical modules
5. Create `mod.rs` with re-exports

**Files to split** (in priority order):
1. `ouroboros-sheet-core/src/sheet.rs` (1,938 lines)
2. `ouroboros-postgres/src/types.rs` (1,899 lines)
3. `ouroboros-sheet-wasm/src/api.rs` (1,864 lines)
4. `ouroboros-qc/src/benchmark.rs` (1,744 lines)
5. `ouroboros-sheet-history/src/command.rs` (1,614 lines)
6. `ouroboros-postgres/src/row.rs` (1,394 lines)
7. `ouroboros-sheet-core/src/spatial.rs` (1,380 lines)
8. `ouroboros-qc/src/reporter.rs` (1,172 lines)
9. `ouroboros/src/validation.rs` (1,145 lines)
10. `ouroboros-sheet-formula/src/evaluator.rs` (1,141 lines)
11. `ouroboros-pyloop/src/loop_impl.rs` (1,062 lines)
12. `ouroboros-api/src/server.rs` (1,051 lines)
13. `ouroboros/src/tasks.rs` (1,045 lines)
14. `ouroboros-sheet-core/src/chunk.rs` (1,035 lines)
15. `ouroboros/src/conversion.rs` (1,019 lines)

## Verification Checklist

After each task:
- [ ] `cargo check` passes for affected crate
- [ ] `cargo test` passes for affected crate
- [ ] `cargo doc` generates without warnings
- [ ] For `ouroboros` crate: `maturin develop` works
- [ ] No file exceeds 500 lines (soft) / 1000 lines (hard)
- [ ] Public API unchanged (grep for `pub use` in old vs new)
