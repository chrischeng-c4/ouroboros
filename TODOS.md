# data-bridge TODO List

Atomic, testable tasks organized by priority and component.

**Last Updated**: 2025-12-29
**Branch**: `feature/postgres-improve`

---

## Legend

- `[ ]` Not started
- `[~]` In progress
- `[x]` Complete
- `[!]` Blocked

---

## Priority 1: Critical (This Week)

*No critical tasks at this time.*

---

## Priority 2: High (Next 2 Weeks)

*No high priority tasks at this time.*

---

## Priority 3: Medium (Next Month)

### P3-EAGER: JOIN-Based Eager Loading

**Goal**: Eliminate N+1 query problem with `fetch_links` parameter

#### Rust Backend

- [ ] P3-EAGER-01: Add `JoinType` enum (Inner, Left, Right, Full)
  - Test: Enum compiles, all variants usable
- [ ] P3-EAGER-02: Add `Join` struct (table, alias, condition, join_type)
  - Test: Struct compiles with required fields
- [ ] P3-EAGER-03: Add `QueryBuilder::join()` method
  - Test: Calling `.join()` adds Join to internal list
- [ ] P3-EAGER-04: Implement `build_select_with_joins()` SQL generation
  - Test: Generates valid SELECT ... JOIN ... SQL
- [ ] P3-EAGER-05: Handle column aliasing for joined tables
  - Test: No column name collisions in output
- [ ] P3-EAGER-06: Implement `Row::fetch_with_relations()` single row
  - Test: Returns row with nested relationship data
- [ ] P3-EAGER-07: Implement `Row::fetch_many_with_relations()` batch
  - Test: Returns list with nested data, uses single query
- [ ] P3-EAGER-08: Add unit tests for JOIN generation (8+ tests)
  - Test: `cargo test join` passes

#### PyO3 Bindings

- [ ] P3-EAGER-09: Add `fetch_one_with_relations()` PyO3 function
  - Test: Function callable, returns dict with nested dicts
- [ ] P3-EAGER-10: Add `fetch_many_with_relations()` PyO3 function
  - Test: Function callable, returns list of dicts

#### Python API

- [ ] P3-EAGER-11: Add `fetch_links` parameter to `fetch_one()`
  - Test: `await fetch_one("posts", ..., fetch_links=["author"])` works
- [ ] P3-EAGER-12: Add `fetch_links` parameter to `fetch_many()`
  - Test: `await fetch_many("posts", ..., fetch_links=["author"])` works
- [ ] P3-EAGER-13: Parse dot notation for nested relations
  - Test: `fetch_links=["author.company"]` works

#### Integration Tests

- [ ] P3-EAGER-14: Test single relation eager load (post.author)
  - Test: 1 query instead of N+1
- [ ] P3-EAGER-15: Test multiple relations (post.author, post.comments)
  - Test: All relations loaded in single query
- [ ] P3-EAGER-16: Test nullable foreign key (LEFT JOIN)
  - Test: NULL values handled correctly
- [ ] P3-EAGER-17: Test nested relations (post.author.company)
  - Test: 3-level nesting works
- [ ] P3-EAGER-18: Benchmark: eager vs lazy for 100 rows
  - Test: Eager loading ≥10x faster

---

### P3-CASCADE: BackReference and Cascade Operations

**Goal**: Support reverse relationships and cascade delete/update

#### Rust Backend

- [ ] P3-CASCADE-01: Add `BackRef` struct (source_table, source_column, target_table)
  - Test: Struct compiles
- [ ] P3-CASCADE-02: Add `CascadeRule` enum (Cascade, Restrict, SetNull, NoAction)
  - Test: Enum compiles
- [ ] P3-CASCADE-03: Implement `get_backreferences()` - find reverse FK relations
  - Test: Returns accurate BackRef list for table
- [ ] P3-CASCADE-04: Implement cascade delete in `Row::delete()`
  - Test: Deleting parent deletes children when CASCADE
- [ ] P3-CASCADE-05: Implement cascade restrict in `Row::delete()`
  - Test: Delete blocked when children exist and RESTRICT
- [ ] P3-CASCADE-06: Implement cascade set null in `Row::delete()`
  - Test: Children FK set to NULL when SET NULL
- [ ] P3-CASCADE-07: Add unit tests for cascade operations (10+ tests)
  - Test: `cargo test cascade` passes

#### Python API

- [ ] P3-CASCADE-08: Add `BackReference[T]` descriptor class
  - Test: `user.posts` returns query for related posts
- [ ] P3-CASCADE-09: Add `on_delete` parameter to `Column(foreign_key=...)`
  - Test: `Column(foreign_key="users.id", on_delete="CASCADE")` works
- [ ] P3-CASCADE-10: Add `on_update` parameter to `Column(foreign_key=...)`
  - Test: `Column(foreign_key="users.id", on_update="CASCADE")` works

#### Integration Tests

- [ ] P3-CASCADE-11: Test ON DELETE CASCADE
  - Test: Delete user → posts deleted
- [ ] P3-CASCADE-12: Test ON DELETE RESTRICT
  - Test: Delete user with posts → error raised
- [ ] P3-CASCADE-13: Test ON DELETE SET NULL
  - Test: Delete user → posts.author_id = NULL
- [ ] P3-CASCADE-14: Test BackReference query
  - Test: `user.posts.fetch()` returns user's posts
- [ ] P3-CASCADE-15: Test nested cascade (user → posts → comments)
  - Test: Delete user cascades through all levels

---

### P3-DOCS: Relationship Documentation

**Goal**: Document ForeignKeyProxy and relationship features

- [ ] P3-DOCS-01: Write quick start example for ForeignKey usage
  - Test: Example code runs successfully
- [ ] P3-DOCS-02: Document lazy loading (`.fetch()` vs `.ref` vs `.id`)
  - Test: All three patterns explained with examples
- [ ] P3-DOCS-03: Document N+1 problem and how to avoid
  - Test: Performance comparison shown
- [ ] P3-DOCS-04: Document nullable foreign keys
  - Test: Example with Optional[ForeignKey] works
- [ ] P3-DOCS-05: Document circular relationships
  - Test: Self-referential example provided
- [ ] P3-DOCS-06: Add troubleshooting section
  - Test: Common errors and solutions listed
- [ ] P3-DOCS-07: Create `docs/postgres/relationships.md`
  - Test: File exists with all sections

---

## Priority 4: Low (Future)

### P4-M2M: Many-to-Many Relationships

- [ ] P4-M2M-01: Add `ManyToMany` struct in Rust
- [ ] P4-M2M-02: Implement auto join table creation
- [ ] P4-M2M-03: Implement `add_relation()` - insert into join table
- [ ] P4-M2M-04: Implement `remove_relation()` - delete from join table
- [ ] P4-M2M-05: Implement `fetch_related()` - query through join table
- [ ] P4-M2M-06: Add `ManyToMany[T]` Python descriptor
- [ ] P4-M2M-07: Support explicit join table with extra columns
- [ ] P4-M2M-08: Add integration tests (10+ tests)

### P4-QUERY: Advanced Query Features

- [ ] P4-QUERY-01: Implement subqueries (WHERE id IN (SELECT ...))
- [ ] P4-QUERY-02: Implement COUNT/SUM/AVG/MIN/MAX aggregations
- [ ] P4-QUERY-03: Implement GROUP BY clause
- [ ] P4-QUERY-04: Implement HAVING clause
- [ ] P4-QUERY-05: Implement window functions (ROW_NUMBER, RANK)
- [ ] P4-QUERY-06: Implement CTE (WITH ... AS ...)
- [ ] P4-QUERY-07: Implement UNION/INTERSECT/EXCEPT
- [ ] P4-QUERY-08: Implement DISTINCT ON
- [ ] P4-QUERY-09: Implement JSONB operators
- [ ] P4-QUERY-10: Add integration tests (25+ tests)

### P4-HTTP: HTTP Client Optimization

- [ ] P4-HTTP-01: Implement lazy header parsing
- [ ] P4-HTTP-02: Implement lazy body parsing
- [ ] P4-HTTP-03: Add zero-copy bytes support
- [ ] P4-HTTP-04: Benchmark vs httpx (target 1.3x faster)

### P4-SECURITY: Security Hardening

- [ ] P4-SECURITY-01: Add TLS/SSL configuration option
- [ ] P4-SECURITY-02: Add connection string validation
- [ ] P4-SECURITY-03: Add credential rotation support
- [ ] P4-SECURITY-04: Security audit before v1.0.0

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
```

### Performance Benchmarks

```bash
# PostgreSQL benchmark
POSTGRES_URI="..." uv run python benchmarks/bench_postgres.py

# Full comparison
uv run python benchmarks/bench_comparison.py
```
