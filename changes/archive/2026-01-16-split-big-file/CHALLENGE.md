# Challenge Report: split-big-file

## Summary

| Severity | Count |
|----------|-------|
| HIGH | 2 |
| MEDIUM | 4 |
| LOW | 2 |

---

## HIGH Severity Issues

### Issue 1: `tests.rs` Would Exceed 2000 Lines

**Location**: Task 1.1 Step 9 (`query/tests.rs`)

**Problem**: The proposal estimates `query/tests.rs` at ~2000 lines, which **violates the <500 line goal** and even the <1000 line hard limit. The tests run from line 2254 to 4430 (2176 lines).

**Evidence**:
```
query.rs total: 4430 lines
tests start:    line 2254
tests size:     2176 lines (142 test functions)
```

**Recommendation**: Split tests into multiple files by category:
```
query/tests/
├── mod.rs           # test module declaration
├── select_tests.rs  # SELECT query tests
├── insert_tests.rs  # INSERT query tests
├── update_tests.rs  # UPDATE/DELETE tests
├── join_tests.rs    # JOIN tests
├── window_tests.rs  # Window function tests
├── cte_tests.rs     # CTE/subquery tests
└── json_tests.rs    # JSON operator tests
```

---

### Issue 2: Single `impl QueryBuilder` Block is 1800 Lines

**Location**: `ouroboros-postgres/src/query.rs:454-2251`

**Problem**: The proposal splits `QueryBuilder` methods into `select.rs` and `modify.rs`, but there's a single 1800-line `impl QueryBuilder { }` block. This requires careful splitting across files using multiple impl blocks.

**Evidence**:
```rust
// query.rs:454
impl QueryBuilder {
    // ... 1797 lines of methods ...
}
// query.rs:2251
```

**Technical Note**: Rust allows multiple `impl` blocks for the same struct across files, but:
1. All sub-modules must import `QueryBuilder` from a shared location
2. Private fields require `pub(super)` visibility in the struct definition
3. Helper methods used across impl blocks need `pub(crate)` or `pub(super)`

**Recommendation**: Update `builder.rs` to export struct with `pub(super)` fields:
```rust
// query/builder.rs
pub struct QueryBuilder {
    pub(super) table: String,
    pub(super) select_columns: Vec<String>,
    pub(super) where_conditions: Vec<WhereCondition>,
    // ... other fields
}

// Core methods only
impl QueryBuilder {
    pub fn new(table: &str) -> Result<Self> { ... }
    pub(super) fn validate_identifier(s: &str) -> Result<()> { ... }
}
```

Then `select.rs` and `modify.rs` can add methods:
```rust
// query/select.rs
use super::builder::QueryBuilder;

impl QueryBuilder {
    pub fn select(mut self, columns: Vec<String>) -> Result<Self> { ... }
    pub fn where_clause(...) -> Result<Self> { ... }
}
```

---

## MEDIUM Severity Issues

### Issue 3: `register_module` Must Be Preserved in mod.rs

**Location**: Tasks 1.2, 1.3, 1.4 (postgres.rs, qc.rs, mongodb.rs)

**Problem**: Each PyO3 module has a `pub fn register_module()` at the end that registers all `#[pyfunction]` and `#[pyclass]` items. When splitting, this function must remain in `mod.rs` and import from all sub-modules.

**Evidence** (postgres.rs:4083):
```rust
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(close, m)?)?;
    m.add_function(wrap_pyfunction!(insert_one, m)?)?;
    // ... 30+ more registrations
}
```

**Recommendation**:
1. Keep `register_module` in `mod.rs`
2. Re-export all `#[pyfunction]` with `pub use` from sub-modules
3. Test with `maturin develop` after each split

---

### Issue 4: Task 1.2 crud.rs Estimated at 800 Lines

**Location**: Task 1.2 Step 6 (`postgres/crud.rs`)

**Problem**: The proposed `crud.rs` at ~800 lines exceeds the 500-line soft limit.

**Recommendation**: Split CRUD into:
- `crud/insert.rs` (~200 lines) - `insert_one`, `insert_many`, `upsert_*`
- `crud/fetch.rs` (~300 lines) - `fetch_one`, `fetch_all`, `count`
- `crud/update.rs` (~150 lines) - `update_one`, `update_many`
- `crud/delete.rs` (~150 lines) - `delete_one`, `delete_many`, `delete_*`

---

### Issue 5: Task 1.4 crud.rs Also Estimated at 800 Lines

**Location**: Task 1.4 Step 6 (`mongodb/crud.rs`)

**Problem**: Same issue as Issue 4 for MongoDB bindings.

**Recommendation**: Same split pattern as postgres crud.

---

### Issue 6: Phase 3 Lacks Specific Split Plans

**Location**: Tasks 3.1-3.15

**Problem**: Phase 3 tasks say "follow the same pattern" but each file has unique structure. Without specific analysis, implementations may miss optimal split boundaries.

**Recommendation**: Before implementing Phase 3, run detailed analysis on each file to create specific split plans. At minimum, add:
- For each file: list of structs/enums/impls with line counts
- Proposed module boundaries based on actual code structure

---

## LOW Severity Issues

### Issue 7: Benchmark Tests Not Addressed

**Location**: `ouroboros-qc/src/benchmark.rs` (1,744 lines)

**Problem**: Task 3.4 lists this file but doesn't specify if it has tests that need separate handling.

**Recommendation**: Check for `#[cfg(test)]` blocks and plan test splitting if present.

---

### Issue 8: Consistent Naming Convention Unclear

**Location**: All tasks

**Problem**: Proposal uses inconsistent file naming:
- `operator.rs` vs `operators.rs`
- `types.rs` vs `enums.rs`
- `cte.rs` vs `ctes.rs`

**Recommendation**: Establish naming convention:
- Singular for single-concept files: `operator.rs`, `type.rs`
- Or plural for collections: `operators.rs`, `types.rs`
- Apply consistently across all splits

---

## Architecture Validation

### Validated Patterns ✓

1. **Module conversion**: `foo.rs` → `foo/mod.rs` is standard Rust pattern
2. **Re-exports**: `pub use submodule::*` maintains API compatibility
3. **Multiple impl blocks**: Rust supports `impl Struct` across files
4. **Feature flags preserved**: `#[cfg(feature = "postgres")]` unaffected

### Potential Conflicts

1. **Circular dependencies**: None detected - proposed structure is tree-like
2. **Naming conflicts**: `query/` exists in `ouroboros-sheet-db` but different crate, no conflict

---

## Recommended Actions

1. **Update tasks.md** to:
   - Split `tests.rs` into multiple test files
   - Split `crud.rs` into insert/fetch/update/delete
   - Add `pub(super)` visibility notes for struct fields

2. **Add verification step**: After each file split, verify:
   ```bash
   cargo check -p <crate-name>
   cargo test -p <crate-name>
   ```

3. **Prioritize**: Address HIGH issues before starting implementation

---

## Next Steps

- If issues acceptable: `/agentd:implement split-big-file`
- If issues need fixing: `/agentd:reproposal split-big-file`
