# PRD: Split Large Files

## Problem Statement

The ouroboros codebase contains **21 files exceeding 1,000 lines**, with the largest file (`query.rs`) at 4,430 lines. According to the project's coding standards in CLAUDE.md:

- Files >= 1,000 lines **MUST** be split
- Files >= 500 lines **SHOULD CONSIDER** split

Large files cause:
1. **Cognitive overload** - Developers struggle to understand 4,000+ line files
2. **Merge conflicts** - Multiple contributors editing the same large file
3. **Slow code reviews** - Reviewers can't effectively review massive files
4. **Poor discoverability** - Related functionality buried in monolithic files
5. **Editor performance** - LSP and syntax highlighting degrade with large files

## Scope

### Priority 1: Critical (>3,000 lines) - 4 files

| Lines | File | Description |
|-------|------|-------------|
| 4,430 | `ouroboros-postgres/src/query.rs` | Query builder with SELECT/INSERT/UPDATE/DELETE, CTEs, window functions |
| 4,206 | `ouroboros/src/postgres.rs` | PostgreSQL PyO3 bindings with 30+ `#[pyfunction]` exports |
| 3,987 | `ouroboros/src/qc.rs` | Test framework bindings with runners, reporters, benchmarks |
| 3,684 | `ouroboros/src/mongodb.rs` | MongoDB PyO3 bindings with BSON conversion |

### Priority 2: High (2,000-3,000 lines) - 2 files

| Lines | File | Description |
|-------|------|-------------|
| 2,070 | `ouroboros-api/src/validation.rs` | HTTP request validation with type descriptors |
| 2,011 | `ouroboros-postgres/src/schema.rs` | Database schema introspection |

### Priority 3: Medium (1,000-2,000 lines) - 15 files

| Lines | File | Description |
|-------|------|-------------|
| 1,938 | `ouroboros-sheet-core/src/sheet.rs` | Spreadsheet sheet |
| 1,899 | `ouroboros-postgres/src/types.rs` | Type mapping |
| 1,864 | `ouroboros-sheet-wasm/src/api.rs` | WASM bindings |
| 1,744 | `ouroboros-qc/src/benchmark.rs` | Benchmarking |
| 1,614 | `ouroboros-sheet-history/src/command.rs` | Undo commands |
| 1,394 | `ouroboros-postgres/src/row.rs` | Row abstraction |
| 1,380 | `ouroboros-sheet-core/src/spatial.rs` | Spatial indexing |
| 1,172 | `ouroboros-qc/src/reporter.rs` | Test reporter |
| 1,145 | `ouroboros/src/validation.rs` | MongoDB validation |
| 1,141 | `ouroboros-sheet-formula/src/evaluator.rs` | Formula eval |
| 1,062 | `ouroboros-pyloop/src/loop_impl.rs` | PyLoop impl |
| 1,051 | `ouroboros-api/src/server.rs` | HTTP server |
| 1,045 | `ouroboros/src/tasks.rs` | Task queue bindings |
| 1,035 | `ouroboros-sheet-core/src/chunk.rs` | Chunk storage |
| 1,019 | `ouroboros/src/conversion.rs` | BSON conversion |

## Goals

1. **No file exceeds 500 lines** after splitting (hard limit: 1000)
2. **Preserve public API** - No breaking changes to module exports
3. **Improve cohesion** - Each new file has single responsibility
4. **Enable parallel work** - Multiple developers can work on different sub-modules

## Success Criteria

- [ ] All 21 files reduced to <500 lines each (soft) / <1000 lines (hard)
- [ ] All tests pass after refactoring (`cargo test`)
- [ ] No public API changes (re-exports maintain compatibility)
- [ ] Python bindings still work (`maturin develop`)
- [ ] Compilation time not increased significantly

## Non-Goals

- Changing functionality or adding features
- Refactoring internal logic (only file organization)
- Changing public APIs (only internal restructuring)

## Impact

- **Developer productivity**: Faster onboarding to unfamiliar modules
- **Code review quality**: Smaller, focused files easier to review
- **Merge conflicts**: Reduced conflicts on large files
- **Affected Specs**: `code-style`, `architecture`
