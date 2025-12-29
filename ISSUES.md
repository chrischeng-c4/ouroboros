# data-bridge Known Issues & Architectural Challenges

This document tracks significant technical challenges, unresolved bugs, and feature gaps identified during development and security assessment.

---

## 1. Core ORM & Query Engine

### [ISSUE-01] Column Name Collisions in JOIN Queries
- **Status**: OPEN
- **Description**: When performing eager loading via `JOIN`s, if multiple tables have the same column names (e.g., `id`, `name`, `created_at`), the results may collide in the `HashMap` representation if not explicitly aliased.
- **Impact**: Incorrect data being returned for joined relations or main table fields.
- **Needed Fix**: Implement robust, unique aliasing for all columns in joined queries and ensure the extraction logic correctly maps these aliases back to the nested relation structures.

### [ISSUE-02] Memory Exhaustion on Large Result Sets
- **Status**: OPEN
- **Description**: Current `fetch_all` and `find_many` implementations load the entire result set into Rust memory and then convert it to Python objects at once.
- **Impact**: High memory pressure or OOM (Out of Memory) crashes when querying millions of rows.
- **Needed Fix**: Implement **Server-side Cursors** and streaming results (e.g., an `async for` compatible iterator) to process data in batches.

### [ISSUE-03] Limited Query Expressiveness
- **Status**: OPEN
- **Description**: `QueryBuilder` lacks support for `GROUP BY`, `HAVING`, aggregations (`SUM`, `AVG` with distinct, etc.), CTEs, and Window Functions.
- **Impact**: Users must fall back to raw `execute()` for complex analytical queries, losing type safety and convenience.
- **Needed Fix**: Expand `QueryBuilder` API to support standard SQL analytical clauses.

---

## 2. Security & Robustness

### [ISSUE-04] Dynamic SQL Identifier Protection
- **Status**: IN PROGRESS
- **Description**: While basic validation exists, the system needs a unified, audited mechanism for quoting all SQL identifiers (tables, columns, schema parts) to prevent injection via metadata.
- **Impact**: Risk of SQL injection if a malicious user can influence table or column names.
- **Progress**: Quoting added to `QueryBuilder`, but needs exhaustive audit of all `format!` and `push_str` calls.

### [ISSUE-05] FFI Boundary Panic Safety
- **Status**: OPEN
- **Description**: If Rust code panics (e.g., an unexpected `unwrap()` failure), it may crash the entire Python process rather than raising a Python exception.
- **Impact**: Application instability and potential DoS vector.
- **Needed Fix**: Implement a consistent wrapper or use `std::panic::catch_unwind` at the PyO3 boundary to convert panics into `PyRuntimeError`.

### [ISSUE-06] Sensitive Data in Logs
- **Status**: OPEN
- **Description**: Error messages from `sqlx` or `data-bridge` might include raw query parameters or schema details that should be redacted.
- **Impact**: Information leakage in production logs.
- **Needed Fix**: Implement a log redactor or ensure `ExtractedValue` doesn't leak contents in its `Debug` implementation when used in sensitive contexts.

---

## 3. PostgreSQL Compatibility

### [ISSUE-07] NUMERIC/DECIMAL Type Mapping
- **Status**: PARTIALLY FIXED
- **Description**: Mapping between `NUMERIC` (Postgres) and Python `Decimal` is complex. Aggregate functions often return `NUMERIC` types that `sqlx` fails to extract if the target type isn't exact.
- **Fix**: Enabled `rust_decimal` in `sqlx` and implemented a multi-stage fallback (Decimal -> f64 -> String).
- **Residual Task**: Ensure precision is maintained across all edge cases without unnecessary conversion to float.

---

## 4. Maintenance & Developer Experience

### [ISSUE-08] Rust `format!` Syntax Fragility
- **Status**: ACTIVE
- **Description**: Writing complex SQL generation logic using `format!` in Rust has proven error-prone (e.g., escaping `{}` vs `{{}}`, mismatched arguments).
- **Impact**: Frequent compilation failures during development.
- **Needed Fix**: Consider moving towards a more structured SQL DSL or a dedicated template engine for complex SQL generation to reduce manual string manipulation.

### [ISSUE-09] Integration Test Environment Dependency
- **Status**: OPEN
- **Description**: A large portion of the test suite (80+ integration tests) requires a live PostgreSQL instance.
- **Impact**: Tests are frequently skipped or fail in environments without DB access.
- **Needed Fix**: Improve Docker-based test orchestration and consider mocking `sqlx` for a subset of integration tests.
