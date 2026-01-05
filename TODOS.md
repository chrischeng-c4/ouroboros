# data-bridge TODO List

Atomic, testable tasks organized by priority and component.

**Last Updated**: 2025-01-05 (P5 SQLAlchemy parity roadmap added)
**Branch**: `main`

---

## 🔴 Audit Findings (2025-12-30)

### P0 - Critical (Must Fix Before Production)

| ID | Issue | Location | Status |
|----|-------|----------|--------|
| P0-AUDIT-01 | Production `unwrap()` calls can crash Python | `row.rs:155,291,550,709`, `validation.rs:33`, `query.rs:753` | DONE (2025-12-30) |
| P0-AUDIT-02 | No PyO3 panic boundary protection | `crates/data-bridge/src/postgres.rs` | DONE (2025-12-30) |
| P0-AUDIT-03 | Error messages leak table/constraint names | `row.rs:798-801` | DONE (2025-12-30) |
| P0-AUDIT-04 | Password exposed in Python memory | `python/data_bridge/postgres/connection.py:64` | DONE (2025-12-30) |

### P1 - High (Must Fix Before GA)

| ID | Issue | Location | Status |
|----|-------|----------|--------|
| P1-AUDIT-01 | SQL injection format! audit incomplete | 25+ format! calls in row.rs, query.rs, schema.rs | DONE (2025-12-30) |
| P1-AUDIT-02 | Missing Unicode normalization in identifier validation | `validation.rs`, `query.rs` | DONE (2025-12-30) |
| P1-AUDIT-03 | No logging/audit trail | Entire codebase | DONE (2025-12-30) |
| P1-AUDIT-04 | 6 eager loading tests skipped (NULL FK issues) | `test_eager_loading.py` | DONE (2025-12-30) |

### P2 - Medium (Should Fix)

| ID | Issue | Location | Status |
|----|-------|----------|--------|
| P2-AUDIT-01 | Example docs have hardcoded passwords | `connection.rs:199`, `connection.py:42` | DONE (2025-12-30) |
| P2-AUDIT-02 | Missing Unicode validation tests | `test_security.py` | DONE (2025-12-30) |
| P2-AUDIT-03 | No cascade delete with nested relationships tests | `test_cascade_delete.py` | DONE (2025-12-30) |
| P2-BUILD-01 | PyO3 compilation errors fixed | `crates/data-bridge/src/postgres.rs` | DONE (2025-12-30) |

---

## Legend

- `[ ]` Not started
- `[~]` In progress
- `[x]` Complete
- `[!]` Blocked

---

## P0 - Critical Security (Immediate)

### SQL Injection Vulnerabilities

- [x] P0-SEC-01: Fix SQL Injection in JOIN ON conditions (2025-12-30)
  - **Location**: `query.rs:303-316`, `row.rs:477-483`
  - **Issue**: `on_condition` parameter inserted directly without validation
  - **Fix**: Validate ON conditions or create structured builder
  - **Test**: Attempt injection with malicious ON clause

- [x] P0-SEC-02: Fix SQL Injection in Cascade Delete format! (2025-12-30)
  - **Location**: `row.rs:754-812`
  - **Issue**: `backref.source_table/source_column` in `format!` without validation
  - **Fix**: Pre-validate BackRef fields before SQL generation
  - **Test**: Attempt injection via BackRef struct fields

- [x] P0-SEC-03: Add RelationConfig field validation (2025-12-30)
  - **Location**: `row.rs:454-486`
  - **Issue**: RelationConfig fields bypass validation if constructed externally
  - **Fix**: Validate all fields at construction time
  - **Test**: Construct RelationConfig with malicious field names

---

## P1 - High Priority (This Week)

### Bug Fixes

- [x] P1-BUG-01: Fix NULL FK column aliasing (6 skipped tests) (2025-12-30)
  - **Location**: `tests/postgres/integration/test_eager_loading.py`
  - **Issue**: Column aliasing fails for NULL foreign keys in JOINs
  - **Tests**:
    - `test_fetch_with_relations_null_fk_multiple_posts`
    - `test_fetch_many_with_relations_null_fk_mixed`
    - `test_fetch_with_relations_nested_null_intermediate`
    - `test_fetch_with_relations_invalid_relation_name`
    - `test_fetch_many_with_relations_complex_scenario`
    - `test_fetch_many_with_relations_performance`
  - **Fix**: Implement proper column aliasing for LEFT JOIN NULL cases

### Missing Implementations

- [x] P1-FUNC-01: Implement `from_python()` type conversion (2025-12-30)
  - **Location**: `types.rs:70-85`
  - **Status**: Already implemented elsewhere, removed stale TODO

- [x] P1-FUNC-02: Implement `to_python()` type conversion (2025-12-30)
  - **Location**: `types.rs:96-111`
  - **Status**: Already implemented elsewhere, removed stale TODO

- [x] P1-FUNC-03: Implement QueryBuilder execute methods (2025-12-30)
  - **Location**: `query.rs:653-675`
  - **Status**: Already implemented elsewhere, removed stale TODO

---

## P2 - Medium Priority (Next 2 Weeks)

### Testing

- [x] P2-TEST-01: Add Migration ALTER TABLE tests (2025-12-30)
  - **Location**: `tests/postgres/unit/test_autogenerate_migration.py`
  - **Tests**:
    - Alter column type
    - Alter column nullable
    - Alter column default
    - Alter multiple columns
  - **Expected**: Valid ALTER TABLE SQL generated

- [x] P2-TEST-02: Add Migration DROP TABLE tests (2025-12-30)
  - **Location**: `tests/postgres/unit/test_autogenerate_migration.py`
  - **Tests**:
    - Drop table with foreign keys
    - Drop table with indexes
    - Drop multiple tables
  - **Expected**: Valid DROP TABLE SQL with CASCADE if needed

### Performance

- [x] P2-PERF-01: Reduce clone() in bulk operations (2025-12-30)
  - **Location**: `row.rs:155, 291` (20+ clone() calls in hot paths)
  - **Issue**: Unnecessary clones degrade performance
  - **Fix**: Use references or move semantics where possible
  - **Test**: Benchmark before/after

- [x] P2-PERF-02: Replace unwrap() with error handling in hot paths (2025-12-30)
  - **Location**: `row.rs` hot paths
  - **Issue**: unwrap() can panic, poor error messages
  - **Fix**: Use proper Result/Option handling
  - **Test**: Verify errors propagate correctly
  - **Note**: Already safe - no unwrap() in hot paths

- [x] P2-PERF-03: Optimize Array binding (avoid JSON fallback) (2025-12-30)
  - **Location**: `types.rs:213-229`
  - **Issue**: Arrays fallback to JSON serialization
  - **Fix**: Use native PostgreSQL array binding
  - **Test**: Benchmark array insert performance

### Security

- [x] P2-SEC-02: Add `catch_unwind` at PyO3 boundary (2025-12-30)
  - **Issue**: Rust panics crash Python instead of raising exception
  - **Fix**: Wrap all FFI entry points with `catch_unwind`
  - **Test**: Trigger panic, verify Python exception raised
  - **Note**: Low risk - current code is panic-safe (no unwrap/expect in production). Defensive measure for dependency panics.

- [x] P2-SEC-01: Audit all format!/push_str for SQL injection (2025-12-30)
  - **Progress**: `validate_identifier()` implemented, need full audit
  - **Locations**: All SQL generation in `query.rs`, `row.rs`, `schema.rs`
  - **Fix**: Ensure all dynamic values use parameterization or validation
  - **Test**: Attempt injection at each SQL construction point
  - **Result**: No vulnerabilities found, all identifiers validated

### Bug Fixes

- [x] P2-BUG-01: Fix column name collisions in JOINs (2025-12-30)
  - **Issue**: Multiple tables with same column names (id, created_at)
  - **Example**: `users.id` vs `posts.id` both returned as `id`
  - **Fix**: Implement consistent column aliasing strategy
  - **Test**: JOIN 3+ tables with overlapping column names

---

## P3 - Lower Priority (Next Month)

### P3-CASCADE: BackReference and Cascade Operations

**Goal**: Support reverse relationships and cascade delete/update

#### Python API

- [x] P3-CASCADE-08: Add `BackReference[T]` descriptor class (2025-12-30)
  - **Test**: `user.posts` returns query for related posts

- [x] P3-CASCADE-09: Add `on_delete` parameter to `Column(foreign_key=...)` (2025-12-30)
  - **Test**: `Column(foreign_key="users.id", on_delete="CASCADE")` works

- [x] P3-CASCADE-10: Add `on_update` parameter to `Column(foreign_key=...)` (2025-12-30)
  - **Test**: `Column(foreign_key="users.id", on_update="CASCADE")` works

---

### P3-DOCS: Relationship Documentation

**Goal**: Document ForeignKeyProxy and relationship features

- [x] P3-DOCS-01: Write quick start example for ForeignKey usage (2025-12-30)
  - **Test**: Example code runs successfully

- [x] P3-DOCS-02: Document lazy loading (`.fetch()` vs `.ref` vs `.id`) (2025-12-30)
  - **Test**: All three patterns explained with examples

- [x] P3-DOCS-03: Document N+1 problem and how to avoid (2025-12-30)
  - **Test**: Performance comparison shown

- [x] P3-DOCS-04: Document nullable foreign keys (2025-12-30)
  - **Test**: Example with Optional[ForeignKey] works

- [x] P3-DOCS-05: Document circular relationships (2025-12-30)
  - **Test**: Self-referential example provided

- [x] P3-DOCS-06: Add troubleshooting section (2025-12-30)
  - **Test**: Common errors and solutions listed

- [x] P3-DOCS-07: Create `docs/postgres/relationships.md` (2025-12-30)
  - **Test**: File exists with all sections

---

### P3-FUNC: Additional Features

- [x] P3-FUNC-01: Implement Savepoint support (2025-12-30)
  - **Location**: `transaction.rs:100-126`
  - **Features**:
    - Create savepoint
    - Rollback to savepoint
    - Release savepoint
  - **Test**: Nested transaction with rollback to savepoint

### P3-SEC: Security Documentation

- [ ] P3-SEC-01: Add TLS configuration documentation
  - **Location**: `connection.rs`
  - **Topics**: sslmode, certificate verification, client certificates
  - **Test**: Example TLS connection works

- [ ] P3-SEC-02: Remove sensitive data from error logs
  - **Issue**: Error messages may leak query parameters
  - **Fix**: Redact parameter values in error messages
  - **Test**: Verify no sensitive data in logged errors

---

## P4 - Backlog (Future)

### P4-PERF: Performance Improvements

- [ ] P4-PERF-01: Server-side cursors for large results
  - **Issue**: Memory exhaustion on large result sets
  - **Fix**: Implement streaming with server-side cursors
  - **Test**: Fetch 1M+ rows without OOM

### P4-QUERY: Advanced Query Features (SQLAlchemy Parity)

- [x] P4-QUERY-01: Implement subqueries (WHERE id IN (SELECT ...)) (2025-01-04)
- [x] P4-QUERY-02: Implement COUNT/SUM/AVG/MIN/MAX aggregations (2025-01-04)
- [x] P4-QUERY-03: Implement GROUP BY clause (2025-01-04)
- [x] P4-QUERY-04: Implement HAVING clause (2025-01-04)
- [x] P4-QUERY-05: Implement window functions (ROW_NUMBER, RANK) (2025-01-04)
- [x] P4-QUERY-06: Implement CTE (WITH ... AS ...) (2025-01-04)
- [x] P4-QUERY-07: Implement UNION/INTERSECT/EXCEPT (2025-01-04)
- [x] P4-QUERY-08: Implement DISTINCT ON (2025-01-04)
- [x] P4-QUERY-09: Implement JSONB operators (2025-01-04)
- [x] P4-QUERY-10: Support RETURNING clause in Updates/Deletes (2025-01-04)
- [x] P4-QUERY-11: Add integration tests (100+ tests) (2025-01-05)

### P4-M2M: Many-to-Many Relationships

- [x] P4-M2M-01: Add `ManyToMany` struct in Rust (2025-01-05)
- [x] P4-M2M-02: Implement auto join table creation (2025-01-05)
- [x] P4-M2M-03: Implement `add_relation()` - insert into join table (2025-01-05)
- [x] P4-M2M-04: Implement `remove_relation()` - delete from join table (2025-01-05)
- [x] P4-M2M-05: Implement `fetch_related()` - query through join table (2025-01-05)
- [x] P4-M2M-06: Add `ManyToMany[T]` Python descriptor (2025-01-05)
- [x] P4-M2M-07: Support explicit join table with extra columns (2025-01-05)
- [x] P4-M2M-08: Add integration tests (18 tests) (2025-01-05)

---

## P5 - Performance Roadmap (Long-term, Low Priority)

> **Note**: 目前性能已達可用標準，這些是長期優化目標
> (Current performance meets production standards; these are long-term optimization goals)

### P5-HTTP: HTTP Client Optimization

- [ ] P5-HTTP-01: Lazy Response Parsing (Target: 1.3x faster)
  - **Current**: Parse all headers/body immediately
  - **Goal**: Parse only what's accessed
  - **Benchmark**: vs httpx, aiohttp

### P5-CORE: Core Optimization

- [ ] P5-CORE-01: Connection Pool Lock-Free Path
  - **Current**: Mutex contention on pool access
  - **Goal**: Lock-free fast path for available connections
  - **Benchmark**: 10K+ concurrent requests

### P5-MONGO: MongoDB Optimization

- [ ] P5-MONGO-01: Zero-Copy Deserialization (Target: 4x faster)
  - **Current**: Copy BSON bytes to Rust structs
  - **Goal**: Read BSON in-place with borsh/zerocopy
  - **Benchmark**: vs current implementation

### P5-PG: PostgreSQL Optimization

- [ ] P5-PG-01: Binary Protocol Optimization
  - **Current**: Text protocol for most types
  - **Goal**: Binary protocol for numeric/date types
  - **Benchmark**: Large numeric datasets

### P5-TOOL: Tooling Optimization

- [ ] P5-TOOL-01: Type-Safe SQL Generation
  - **Current**: String-based SQL construction
  - **Goal**: Type-safe query builder with compile-time checks
  - **Benefit**: Eliminate SQL syntax errors at compile time

- [ ] P5-TOOL-02: Migration Engine (Target: 10x faster than Alembic)
  - **Current**: Python-based schema diffing
  - **Goal**: Rust-based parallel schema analysis
  - **Benchmark**: Large schema (100+ tables)

---

## P5 - SQLAlchemy Parity (Future)

> **Goal**: Achieve feature parity with SQLAlchemy ORM for Python-level usage

### P5-ORM: Session & Unit of Work

- [x] P5-ORM-01: Identity Map - Cache objects by primary key, single instance per PK (2025-01-05)
- [x] P5-ORM-02: Dirty Tracking - Track which fields changed since load (2025-01-05)
- [x] P5-ORM-03: Auto-flush - Flush pending changes before query (2025-01-05)
- [x] P5-ORM-04: Unit of Work - Accumulate INSERT/UPDATE/DELETE, execute on commit (2025-01-05)
- [x] P5-ORM-05: Session context - `async with Session()` pattern (2025-01-05)

### P5-LOAD: Loading Strategies (COMPLETED 2025-01-05)

| ID | Feature | Description |
|----|---------|-------------|
| P5-LOAD-01 | Lazy loading | Load relationships on access (default) |
| P5-LOAD-02 | Eager (joined) | Load via JOIN (DONE - already implemented) |
| P5-LOAD-03 | Subquery eager | Load via separate subquery |
| P5-LOAD-04 | noload | Never load relationship |
| P5-LOAD-05 | raise/raise_on_sql | Raise error if lazy load triggered |
| P5-LOAD-06 | selectinload | Batch load with IN clause |
| P5-LOAD-07 | defer()/undefer() | Defer loading of large columns |

### P5-EVENT: Event System (COMPLETED 2025-01-05)

| ID | Feature | Description |
|----|---------|-------------|
| P5-EVENT-01 | before_insert | Hook before INSERT |
| P5-EVENT-02 | after_insert | Hook after INSERT |
| P5-EVENT-03 | before_update | Hook before UPDATE |
| P5-EVENT-04 | after_update | Hook after UPDATE |
| P5-EVENT-05 | before_delete | Hook before DELETE |
| P5-EVENT-06 | after_delete | Hook after DELETE |
| P5-EVENT-07 | before_flush | Hook before session flush |
| P5-EVENT-08 | after_commit | Hook after transaction commit |
| P5-EVENT-09 | on_attribute_change | Hook on field modification |
| P5-EVENT-10 | @listens_for() | Decorator API for event registration |

### P5-INHERIT: Inheritance Patterns (COMPLETED 2025-01-05)

| ID | Feature | Description |
|----|---------|-------------|
| P5-INHERIT-01 | Single Table | All classes share one table + discriminator |
| P5-INHERIT-02 | Joined Table | Each class has own table with FK to parent |
| P5-INHERIT-03 | Concrete Table | Each class has complete own table |
| P5-INHERIT-04 | Polymorphic loading | Load mixed types from one query |
| P5-INHERIT-05 | Discriminator column | `type` column for class discrimination |

### P5-COMPUTED: Computed Attributes (COMPLETED 2025-01-05)

| ID | Feature | Description |
|----|---------|-------------|
| P5-COMPUTED-01 | @hybrid_property | Works in Python AND SQL expressions |
| P5-COMPUTED-02 | column_property() | SQL expression as read-only column |
| P5-COMPUTED-03 | GENERATED AS | PostgreSQL computed column support |
| P5-COMPUTED-04 | Default factories | `default=lambda: datetime.now()` |

### P5-QUERY: Query Builder Enhancements

| ID | Feature | Description |
|----|---------|-------------|
| P5-QUERY-01 | filter_by() | Keyword argument filtering syntax |
| P5-QUERY-02 | and_()/or_() | Explicit boolean combinators |
| P5-QUERY-03 | any()/has() | Relationship existence operators |
| P5-QUERY-04 | Query subclassing | Extend QueryBuilder per-model |
| P5-QUERY-05 | Query composition | Reusable query fragments |
| P5-QUERY-06 | aliased() | For self-joins and multiple refs |

### P5-VALID: ORM-Level Validation

| ID | Feature | Description |
|----|---------|-------------|
| P5-VALID-01 | @validates() | Field validator decorator |
| P5-VALID-02 | Custom types | User-defined type classes |
| P5-VALID-03 | Auto-coercion | Auto-convert on assignment |

### P5-ASYNC: Async Enhancements

| ID | Feature | Description |
|----|---------|-------------|
| P5-ASYNC-01 | AsyncSession | Full async session support |
| P5-ASYNC-02 | Async relationships | Lazy load in async context |
| P5-ASYNC-03 | run_sync() | Escape hatch to sync code |

---

## Completed

### [x] Phase 1: Core CRUD and Schema Management
- [x] Connection pooling
- [x] Type mapping (25+ PostgreSQL types)
- [x] Basic CRUD (insert, fetch, update, delete)
- [x] Query builder (WHERE, ORDER BY, LIMIT, OFFSET)
- [x] Transaction support (4 isolation levels)
- [x] Raw SQL execution
- [x] Schema introspection
- [x] Migration management (up/down SQL)
- [x] Security validation
- [x] Bulk operations with Rayon parallelization

### [x] Phase 2: Upsert Operations (2025-12-29)
- [x] Rust `QueryBuilder::build_upsert()`
- [x] Rust `Row::upsert()` and `Row::upsert_many()`
- [x] PyO3 `upsert_one()` and `upsert_many()`
- [x] Python wrappers in `connection.py`
- [x] Integration tests (13 tests)

### [x] Phase 3a: Foreign Key Lazy Loading
- [x] Foreign key validation
- [x] ForeignKeyProxy class
- [x] `find_by_foreign_key()` helper

### [x] P1-PERF: Single-Row Insert Performance (2025-12-29)

**Result**: 0.74x → 1.54x faster than SQLAlchemy (exceeded 1.5x target)

**Performance Improvements**:
- Single-row insert: 1.54x faster than SQLAlchemy
- Bulk operations (100 rows): 3.93x faster than SQLAlchemy
- No regression in bulk operations (maintained performance)

**Completed Tasks**:
- [x] P1-PERF-01: Profile `insert_one` with flamegraph to identify bottleneck
  - Result: Identified HashMap construction and GIL overhead as bottlenecks
- [x] P1-PERF-02: Measure FFI boundary crossing overhead in `insert_one`
  - Result: Measured and optimized Python→Rust→Python transitions
- [x] P1-PERF-03: Measure HashMap construction overhead for single values
  - Result: HashMap construction was significant overhead for single values
- [x] P1-PERF-04: Implement fast-path entry point for single-row insert
  - Result: Implemented optimized single-row path bypassing HashMap
- [x] P1-PERF-05: Reduce GIL lock/unlock cycles in single-row path
  - Result: Minimized GIL transitions in hot path
- [x] P1-PERF-06: Benchmark optimized insert_one vs SQLAlchemy
  - Result: Achieved 1.54x faster (exceeded 1.5x target)
- [x] P1-PERF-07: Verify bulk operations not regressed
  - Result: Bulk operations maintained at 3.93x faster than SQLAlchemy

### [x] P2-MIGRATE: Auto-Migration Generation (2025-12-29)

**Result**: Implemented full auto-migration generation from schema diffs

**Features**:
- SchemaDiff struct for comparing current vs desired schemas
- Detects table/column/index/foreign key changes
- Generates UP and DOWN migration SQL
- PyO3 bindings for Python access
- 9 unit tests + 21 Rust tests for schema diffing

**Files Added/Modified**:
- `crates/data-bridge-postgres/src/schema.rs` - SchemaDiff, SQL generation
- `crates/data-bridge/src/postgres.rs` - autogenerate_migration PyO3 function
- `python/data_bridge/postgres/migrations.py` - Python wrapper
- `tests/postgres/unit/test_autogenerate_migration.py` - 9 tests

**Completed Tasks**:

#### Schema Diffing (Rust)

- [x] P2-MIGRATE-01: Add `SchemaDiff` struct to `schema.rs` (2025-12-29)
  - Result: Struct implemented with tables/columns/indexes/fks added/removed fields
- [x] P2-MIGRATE-02: Implement `get_current_schema()` - introspect all tables (2025-12-29)
  - Result: Returns accurate TableInfo for existing tables
- [x] P2-MIGRATE-03: Implement `compare_tables()` - detect added/removed tables (2025-12-29)
  - Result: Detects new table, dropped table
- [x] P2-MIGRATE-04: Implement `compare_columns()` - detect column changes (2025-12-29)
  - Result: Detects added column, removed column, type change
- [x] P2-MIGRATE-05: Implement `compare_indexes()` - detect index changes (2025-12-29)
  - Result: Detects added index, removed index
- [x] P2-MIGRATE-06: Implement `compare_foreign_keys()` - detect FK changes (2025-12-29)
  - Result: Detects added FK, removed FK
- [x] P2-MIGRATE-07: Add unit tests for schema diffing (10+ tests) (2025-12-29)
  - Result: 21 Rust tests passing in `cargo test schema_diff`

#### SQL Generation (Rust)

- [x] P2-MIGRATE-08: Implement `generate_create_table()` SQL (2025-12-29)
  - Result: Output matches valid CREATE TABLE syntax
- [x] P2-MIGRATE-09: Implement `generate_drop_table()` SQL (2025-12-29)
  - Result: Output matches DROP TABLE IF EXISTS syntax
- [x] P2-MIGRATE-10: Implement `generate_add_column()` SQL (2025-12-29)
  - Result: Output matches ALTER TABLE ADD COLUMN syntax
- [x] P2-MIGRATE-11: Implement `generate_drop_column()` SQL (2025-12-29)
  - Result: Output matches ALTER TABLE DROP COLUMN syntax
- [x] P2-MIGRATE-12: Implement `generate_alter_column()` SQL (2025-12-29)
  - Result: Handles type change, nullable change, default change
- [x] P2-MIGRATE-13: Implement `generate_create_index()` SQL (2025-12-29)
  - Result: Output matches CREATE INDEX syntax
- [x] P2-MIGRATE-14: Implement `generate_drop_index()` SQL (2025-12-29)
  - Result: Output matches DROP INDEX syntax
- [x] P2-MIGRATE-15: Implement `generate_add_foreign_key()` SQL (2025-12-29)
  - Result: Output matches ALTER TABLE ADD CONSTRAINT syntax
- [x] P2-MIGRATE-16: Implement `generate_drop_foreign_key()` SQL (2025-12-29)
  - Result: Output matches ALTER TABLE DROP CONSTRAINT syntax
- [x] P2-MIGRATE-17: Implement `generate_down_migration()` - reverse operations (2025-12-29)
  - Result: CREATE→DROP, ADD→DROP, etc. correctly reversed
- [x] P2-MIGRATE-18: Add unit tests for SQL generation (15+ tests) (2025-12-29)
  - Result: Tests included in schema_diff suite

#### PyO3 Bindings

- [x] P2-MIGRATE-19: Add `autogenerate_migration()` PyO3 function (2025-12-29)
  - Result: Function callable from Python, returns dict with up/down SQL
- [x] P2-MIGRATE-20: Add `TableSchema` extraction from Python Table classes (2025-12-29)
  - Result: Can extract column types, constraints from Table class

#### Python API

- [x] P2-MIGRATE-21: Add `autogenerate()` function to `migrations.py` (2025-12-29)
  - Result: `await autogenerate("v1", "add_users", [User])` returns Migration
- [x] P2-MIGRATE-22: Add CLI command `python -m data_bridge.postgres.migrations autogenerate` (2025-12-29)
  - Result: CLI runs and creates migration file
- [x] P2-MIGRATE-23: Add migration file writer (saves to migrations/ folder) (2025-12-29)
  - Result: File created with correct naming convention

#### Integration Tests

- [x] P2-MIGRATE-24: Test autogenerate for new table (2025-12-29)
  - Result: Creates valid CREATE TABLE migration
- [x] P2-MIGRATE-25: Test autogenerate for added column (2025-12-29)
  - Result: Creates valid ALTER TABLE ADD COLUMN
- [x] P2-MIGRATE-26: Test autogenerate for removed column (2025-12-29)
  - Result: Creates valid ALTER TABLE DROP COLUMN
- [x] P2-MIGRATE-27: Test autogenerate for type change (2025-12-29)
  - Result: Creates valid ALTER TABLE ALTER COLUMN
- [x] P2-MIGRATE-28: Test autogenerate for added index (2025-12-29)
  - Result: Creates valid CREATE INDEX
- [x] P2-MIGRATE-29: Test autogenerate for added foreign key (2025-12-29)
  - Result: Creates valid ADD CONSTRAINT
- [x] P2-MIGRATE-30: Test down migration execution (2025-12-29)
  - Result: Down migration reverses up migration correctly
- [x] P2-MIGRATE-31: Test no-change scenario (2025-12-29)
  - Result: Returns empty migration when schema matches

### [x] P3-EAGER: JOIN-Based Eager Loading (2025-12-29)

**Result**: Implemented JOIN-based eager loading to eliminate N+1 queries

**Features**:
- JoinType enum (Inner, Left, Right, Full)
- QueryBuilder.join() method with security validation
- fetch_one_eager() - simple tuple-based API
- fetch_one_with_relations() - full configuration API
- fetch_many_with_relations() - batch eager loading
- 10 integration tests passing (6 skipped for edge cases)

**Known Limitations**:
- Column aliasing needed for NULL foreign key edge cases

**Files Added/Modified**:
- `crates/data-bridge-postgres/src/query.rs` - JOIN support
- `crates/data-bridge-postgres/src/row.rs` - RelationConfig, fetch_with_relations
- `crates/data-bridge/src/postgres.rs` - PyO3 bindings
- `python/data_bridge/postgres/connection.py` - Python wrappers
- `tests/postgres/integration/test_eager_loading.py` - 16 tests (10 active)

**Completed Tasks**:

#### Rust Backend

- [x] P3-EAGER-01: Add `JoinType` enum (Inner, Left, Right, Full) (2025-12-29)
  - Result: Enum compiles, all variants usable
- [x] P3-EAGER-02: Add `Join` struct (table, alias, condition, join_type) (2025-12-29)
  - Result: Struct compiles with required fields
- [x] P3-EAGER-03: Add `QueryBuilder::join()` method (2025-12-29)
  - Result: Calling `.join()` adds Join to internal list
- [x] P3-EAGER-04: Implement `build_select_with_joins()` SQL generation (2025-12-29)
  - Result: Generates valid SELECT ... JOIN ... SQL
- [x] P3-EAGER-05: Handle column aliasing for joined tables (2025-12-29)
  - Result: No column name collisions in output
- [x] P3-EAGER-06: Implement `Row::fetch_with_relations()` single row (2025-12-29)
  - Result: Returns row with nested relationship data
- [x] P3-EAGER-07: Implement `Row::fetch_many_with_relations()` batch (2025-12-29)
  - Result: Returns list with nested data, uses single query
- [x] P3-EAGER-08: Add unit tests for JOIN generation (8+ tests) (2025-12-29)
  - Result: `cargo test join` passes

#### PyO3 Bindings

- [x] P3-EAGER-09: Add `fetch_one_with_relations()` PyO3 function (2025-12-29)
  - Result: Function callable, returns dict with nested dicts
- [x] P3-EAGER-10: Add `fetch_many_with_relations()` PyO3 function (2025-12-29)
  - Result: Function callable, returns list of dicts

#### Python API

- [x] P3-EAGER-11: Add `fetch_links` parameter to `fetch_one()` (2025-12-29)
  - Result: `await fetch_one("posts", ..., fetch_links=["author"])` works
- [x] P3-EAGER-12: Add `fetch_links` parameter to `fetch_many()` (2025-12-29)
  - Result: `await fetch_many("posts", ..., fetch_links=["author"])` works
- [x] P3-EAGER-13: Parse dot notation for nested relations (2025-12-29)
  - Result: `fetch_links=["author.company"]` works

#### Integration Tests

- [x] P3-EAGER-14: Test single relation eager load (post.author) (2025-12-29)
  - Result: 1 query instead of N+1
- [x] P3-EAGER-15: Test multiple relations (post.author, post.comments) (2025-12-29)
  - Result: All relations loaded in single query
- [x] P3-EAGER-16: Test nullable foreign key (LEFT JOIN) (2025-12-29)
  - Result: NULL values handled correctly
- [x] P3-EAGER-17: Test nested relations (post.author.company) (2025-12-29)
  - Result: 3-level nesting works
- [x] P3-EAGER-18: Benchmark: eager vs lazy for 100 rows (2025-12-29)
  - Result: Eager loading ≥10x faster

### [x] P3-CASCADE: Cascade Delete Operations (2025-12-30)

**Result**: Implemented cascade delete with BackRef and CascadeRule support

**Features**:
- CascadeRule enum (Cascade, Restrict, SetNull, SetDefault, NoAction)
- BackRef struct for reverse foreign key relationships
- delete_with_cascade() - handles all cascade rules
- delete_checked() - checks RESTRICT constraints only
- get_backreferences() - introspects reverse FK relations
- 9 integration tests passing

**Files Modified**:
- `crates/data-bridge-postgres/src/schema.rs` - CascadeRule, BackRef structs
- `crates/data-bridge-postgres/src/row.rs` - delete_with_cascade, delete_checked
- `crates/data-bridge/src/postgres.rs` - PyO3 bindings
- `python/data_bridge/postgres/connection.py` - Python wrappers
- `tests/postgres/integration/test_cascade_delete.py` - 9 tests

**Remaining Tasks** (moved to P3-CASCADE Python API section):
- BackReference[T] descriptor class
- on_delete/on_update parameters for Column()

---

## Notes

### Running Tests

```bash
# PostgreSQL integration tests
POSTGRES_URI="postgresql://user:pass@localhost:5432/test_db" \
uv run pytest tests/postgres/integration/ -v

# Specific test file
uv run pytest tests/postgres/integration/test_upsert.py -v

# Rust tests
cargo test -p data-bridge-postgres

# Security-specific tests
cargo test -p data-bridge-postgres validate_identifier
uv run pytest tests/postgres/unit/test_sql_injection.py -v
```

### Performance Benchmarks

```bash
# PostgreSQL benchmark
POSTGRES_URI="..." uv run python benchmarks/bench_postgres.py

# Full comparison
uv run python benchmarks/bench_comparison.py
```

### Security Audit

```bash
# Cargo audit for dependency vulnerabilities
cargo audit

# Check for unwrap/expect in production code
rg "\.unwrap\(\)|\.expect\(" crates/ --type rust

# Check for format! in SQL generation
rg "format!\(|push_str\(" crates/data-bridge-postgres/src/ --type rust
```
