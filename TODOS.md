# data-bridge TODO List

This document tracks all remaining work for the data-bridge project, organized by component and priority.

**Last Updated**: 2025-12-29
**Current Branch**: `feature/postgres-improve`
**Status**: PostgreSQL ORM development in progress

---

## Table of Contents

- [Current State Overview](#current-state-overview)
- [PostgreSQL ORM (data-bridge-postgres)](#postgresql-orm-data-bridge-postgres)
- [MongoDB ORM (data-bridge-mongodb)](#mongodb-orm-data-bridge-mongodb)
- [HTTP Client (data-bridge-http)](#http-client-data-bridge-http)
- [Test Framework (data-bridge-test)](#test-framework-data-bridge-test)
- [Documentation & Tooling](#documentation--tooling)
- [Performance Optimization](#performance-optimization)

---

## Current State Overview

### Completed Recently (Dec 26-29, 2025)

- ✅ PostgreSQL migration system (up/down migrations, checksum validation)
- ✅ Schema introspection (tables, columns, indexes, foreign keys)
- ✅ Transaction support with isolation levels
- ✅ Raw SQL execution with parameterized queries
- ✅ Security hardening (removed unsafe unwrap() calls)
- ✅ Benchmark infrastructure for PostgreSQL
- ✅ Performance baseline documentation (ROADMAP.md)
- ✅ Foreign key validation and ForeignKeyProxy (lazy loading)

### In Progress

- 🔄 **Upsert operations** - Rust backend complete, PyO3/Python pending
- 🔄 **Relationship features** - Phase 1 (basic lazy loading) done, Phase 2-4 pending

### High Priority (Next 2 Weeks)

1. Complete upsert implementation (PyO3 + Python + tests)
2. Implement auto-migration generation
3. Add JOIN-based eager loading for relationships
4. Optimize single-row PostgreSQL operations (0.74x → 1.5x target)

---

## PostgreSQL ORM (data-bridge-postgres)

**Component**: `crates/data-bridge-postgres/`, `crates/data-bridge/src/postgres.rs`, `python/data_bridge/postgres/`

### Phase 1: Core CRUD and Schema Management ✅ COMPLETE

- [x] Connection pooling (min/max connections, timeout configuration)
- [x] Type mapping (25+ PostgreSQL types to Python)
- [x] Basic CRUD operations (insert, fetch, update, delete)
- [x] Query builder (WHERE, ORDER BY, LIMIT, OFFSET, operators)
- [x] Transaction support (4 isolation levels, ACID guarantees)
- [x] Raw SQL execution (parameterized queries, auto-type detection)
- [x] Schema introspection (list tables, get columns/indexes/foreign keys)
- [x] Migration management (up/down SQL, checksum validation, file loading)
- [x] Security validation (identifier validation, SQL injection prevention)
- [x] Bulk operations (insert_many with Rayon parallelization ≥50 docs)
- [x] 123 Rust unit tests passing
- [x] 10+ Python integration tests passing

### Phase 2: Upsert Operations ⏳ IN PROGRESS (80% Complete)

**Priority**: HIGH
**Estimated Time**: 2-4 hours
**Blocked By**: None

#### Rust Backend (data-bridge-postgres) ✅ COMPLETE

- [x] `QueryBuilder::build_upsert()` - Generates INSERT ... ON CONFLICT SQL
- [x] `Row::upsert()` - Single row upsert
- [x] `Row::upsert_many()` - Bulk upsert with parallel processing
- [x] Unit tests (5 tests covering conflict targets, selective updates, edge cases)
  - [x] `test_upsert_single_conflict` - Single column conflict
  - [x] `test_upsert_selective_update` - Update subset of columns
  - [x] `test_upsert_composite_key` - Multi-column unique constraint
  - [x] `test_upsert_empty_conflict_target` - Error handling
  - [x] `test_upsert_invalid_column_name` - Security validation

#### PyO3 Bindings (data-bridge/src/postgres.rs) ❌ NOT STARTED

**File**: `/Users/chrischeng/projects/data-bridge/crates/data-bridge/src/postgres.rs`

**Tasks**:

- [ ] Add `upsert_one()` function
  - Similar to `insert_one()` but calls `Row::upsert()`
  - Parameters: `table: String`, `data: HashMap`, `conflict_columns: Vec<String>`, `update_columns: Option<Vec<String>>`
  - Return: `PyResult<Bound<'py, PyDict>>` (inserted/updated row)
  - GIL release during async execution

- [ ] Add `upsert_many()` function
  - Similar to `insert_many()` but calls `Row::upsert_many()`
  - Parameters: `table: String`, `rows: Vec<HashMap>`, `conflict_columns: Vec<String>`, `update_columns: Option<Vec<String>>`
  - Return: `PyResult<Bound<'py, PyList>>` (all inserted/updated rows)
  - GIL release during parallel processing

- [ ] Register functions in `postgres_module()`
  - Add `m.add_function(wrap_pyfunction!(upsert_one, m)?)?;`
  - Add `m.add_function(wrap_pyfunction!(upsert_many, m)?)?;`

**Example Implementation Pattern**:

```rust
#[pyfunction]
fn upsert_one<'py>(
    py: Python<'py>,
    table: String,
    data: HashMap<String, PyObject>,
    conflict_columns: Vec<String>,
    update_columns: Option<Vec<String>>,
) -> PyResult<Bound<'py, PyDict>> {
    let conn = get_connection()?;
    let extracted_data = extract_hashmap_data(py, data)?;

    future_into_py(py, async move {
        let row = Row::upsert(&conn, &table, &extracted_data, &conflict_columns, update_columns.as_deref())
            .await
            .map_err(to_pyerr)?;

        Python::with_gil(|py| {
            RowWrapper::from_row(&row)?
                .into_pyobject(py)
                .map_err(|e| e.into())
        })
    })
}
```

#### Python Wrappers (python/data_bridge/postgres/) ❌ NOT STARTED

**File**: `/Users/chrischeng/projects/data-bridge/python/data_bridge/postgres/connection.py`

**Tasks**:

- [ ] Add `upsert_one()` function
  ```python
  async def upsert_one(
      table: str,
      data: Dict[str, Any],
      conflict_columns: List[str],
      update_columns: Optional[List[str]] = None,
  ) -> Dict[str, Any]:
      """
      Insert or update a single row.

      Uses PostgreSQL's INSERT ... ON CONFLICT ... DO UPDATE.

      Args:
          table: Table name
          data: Row data as dictionary
          conflict_columns: Columns that define uniqueness (e.g., ["email"])
          update_columns: Columns to update on conflict (default: all except conflict columns)

      Returns:
          The inserted or updated row

      Example:
          >>> await upsert_one(
          ...     "users",
          ...     {"email": "alice@example.com", "name": "Alice", "age": 30},
          ...     conflict_columns=["email"],
          ... )
      """
      return await _engine.upsert_one(table, data, conflict_columns, update_columns)
  ```

- [ ] Add `upsert_many()` function
  ```python
  async def upsert_many(
      table: str,
      rows: List[Dict[str, Any]],
      conflict_columns: List[str],
      update_columns: Optional[List[str]] = None,
  ) -> List[Dict[str, Any]]:
      """
      Insert or update multiple rows in bulk.

      Uses parallel processing for ≥50 rows.

      Args:
          table: Table name
          rows: List of row dictionaries
          conflict_columns: Columns that define uniqueness
          update_columns: Columns to update on conflict (default: all except conflict columns)

      Returns:
          List of all inserted/updated rows

      Example:
          >>> await upsert_many(
          ...     "users",
          ...     [
          ...         {"email": "alice@example.com", "name": "Alice"},
          ...         {"email": "bob@example.com", "name": "Bob"},
          ...     ],
          ...     conflict_columns=["email"],
          ... )
      """
      return await _engine.upsert_many(table, rows, conflict_columns, update_columns)
  ```

**File**: `/Users/chrischeng/projects/data-bridge/python/data_bridge/postgres/__init__.py`

**Tasks**:

- [ ] Export `upsert_one` and `upsert_many` in `__all__`

#### Integration Tests ❌ NOT STARTED

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/unit/test_upsert.py` (new file)

**Tasks**:

- [ ] Test `upsert_one()` - insert new row
- [ ] Test `upsert_one()` - update existing row
- [ ] Test `upsert_one()` - selective column update
- [ ] Test `upsert_many()` - bulk insert (no conflicts)
- [ ] Test `upsert_many()` - bulk update (all conflicts)
- [ ] Test `upsert_many()` - mixed insert/update
- [ ] Test composite unique constraints
- [ ] Test error handling (invalid conflict columns, missing columns)
- [ ] Test parallel processing threshold (50+ rows)

**Estimated Test Count**: 9-12 tests

#### Acceptance Criteria

- [ ] All PyO3 functions compile without warnings
- [ ] `cargo clippy` passes with no warnings
- [ ] All integration tests pass
- [ ] Performance: bulk upsert ≥50 rows uses parallel processing
- [ ] Documentation: docstrings complete with examples
- [ ] Exported in Python public API

---

### Phase 3: Relationships and Foreign Keys

**Priority**: HIGH (after upsert)
**Estimated Time**: 9-13 days

#### Phase 3a: Foreign Key Lazy Loading ✅ COMPLETE

- [x] Foreign key validation (`validate_foreign_key_reference()`)
- [x] Foreign key introspection (`Connection::get_foreign_keys()`)
- [x] ForeignKeyProxy class (lazy loading with `.fetch()`, `.ref`, `.id`)
- [x] `find_by_foreign_key()` helper function
- [x] 10 unit tests for validation and introspection
- [x] Python API exports

#### Phase 3b: JOIN-Based Eager Loading ⏳ NOT STARTED

**Priority**: MEDIUM
**Estimated Time**: 3-4 days
**Depends On**: Phase 3a ✅

**Goals**:
- Eliminate N+1 query problem
- Add `fetch_links` parameter for eager loading
- Support INNER/LEFT/RIGHT JOIN strategies
- Batched relationship loading

**Tasks**:

##### Rust Backend (data-bridge-postgres)

- [ ] Add `JoinType` enum (Inner, Left, Right, Full)
- [ ] Add `Join` struct (table, condition, join_type)
- [ ] Extend `QueryBuilder` with `.join()` method
- [ ] Implement `build_select_with_joins()` - Generate SQL with JOIN clauses
- [ ] Add `fetch_with_relations()` - Execute query and populate relationships
- [ ] Add `fetch_many_with_relations()` - Bulk fetch with batched joins
- [ ] Unit tests (8-10 tests)
  - [ ] Test INNER JOIN generation
  - [ ] Test LEFT JOIN with NULL handling
  - [ ] Test multiple joins (2+ tables)
  - [ ] Test nested relationships (user → post → comments)
  - [ ] Test circular relationship detection
  - [ ] Test join condition validation

##### PyO3 Bindings

**File**: `crates/data-bridge/src/postgres.rs`

- [ ] Add `fetch_one_with_relations()` function
  - Parameters: `table`, `filter`, `relations: Vec<String>`
  - Returns: `PyDict` with nested relationship data
- [ ] Add `fetch_many_with_relations()` function
  - Batched JOIN queries for efficiency
- [ ] Add helper: `parse_relation_spec()` - Parse "author.posts" syntax

##### Python API

**File**: `python/data_bridge/postgres/connection.py`

- [ ] Add `fetch_links` parameter to `fetch_one()`
  ```python
  async def fetch_one(
      table: str,
      filter: SqlExpr,
      fetch_links: Optional[List[str]] = None,
  ) -> Optional[Dict[str, Any]]:
      """
      Fetch a single row with optional eager loading.

      Args:
          fetch_links: List of relationships to eager-load (e.g., ["author", "comments"])

      Example:
          >>> post = await fetch_one("posts", Post.id == 1, fetch_links=["author"])
          >>> print(post["author"]["name"])  # No additional query!
      """
  ```

- [ ] Add `fetch_links` parameter to `fetch_many()`
- [ ] Update Table class to support `fetch_links` in ORM methods

##### Integration Tests

**File**: `tests/postgres/unit/test_relationships.py` (new file)

- [ ] Test fetch_one with single relationship (INNER JOIN)
- [ ] Test fetch_one with multiple relationships
- [ ] Test fetch_many with relationship (batched loading)
- [ ] Test LEFT JOIN (nullable foreign keys)
- [ ] Test nested relationships (3+ levels)
- [ ] Test performance: 1 query vs N+1 queries
- [ ] Test circular relationship handling
- [ ] Test missing foreign key (NULL handling)

**Estimated Test Count**: 12-15 tests

##### Performance Targets

- 1 query for single-level eager loading (vs N+1 without)
- Batched loading for fetch_many: ≤5 queries for any relationship depth
- Memory overhead: <20% vs lazy loading

#### Phase 3c: BackReference and Cascade Operations ⏳ NOT STARTED

**Priority**: MEDIUM
**Estimated Time**: 2-3 days
**Depends On**: Phase 3b

**Goals**:
- Reverse relationships (User → Posts, Post → User)
- Cascade rules (ON DELETE CASCADE, RESTRICT, SET NULL)
- Write rules for referential integrity

**Tasks**:

##### Rust Backend

- [ ] Add `BackRef` struct (source_table, source_column, target_table)
- [ ] Add `CascadeRule` enum (Cascade, Restrict, SetNull, NoAction)
- [ ] Add `WriteRule` enum (Write, Deny)
- [ ] Implement `Connection::get_backreferences()` - Find reverse FK relationships
- [ ] Implement cascade delete logic in `Row::delete()`
- [ ] Implement cascade update logic in `Row::update()`
- [ ] Unit tests (8-10 tests)

##### PyO3 Bindings

- [ ] Add `get_backreferences()` function
- [ ] Extend delete/update to handle cascade rules

##### Python API

- [ ] Add `BackReference` descriptor class
  ```python
  class User(Table):
      email: str

      # Automatically populated
      posts: BackReference["Post"] = BackReference("posts", cascade_delete=True)
  ```

- [ ] Add cascade rule configuration in `Column()`
  ```python
  class Post(Table):
      author_id: int = Column(foreign_key="users.id", on_delete="CASCADE")
  ```

##### Integration Tests

**File**: `tests/postgres/unit/test_cascade.py` (new file)

- [ ] Test ON DELETE CASCADE (delete user → delete posts)
- [ ] Test ON DELETE RESTRICT (prevent delete if references exist)
- [ ] Test ON DELETE SET NULL (delete user → set posts.author_id = NULL)
- [ ] Test ON UPDATE CASCADE (update user.id → update posts.author_id)
- [ ] Test BackReference query (user.posts → fetch all posts)
- [ ] Test nested cascade (user → posts → comments)
- [ ] Test WriteRule.Deny (prevent creating orphan posts)

**Estimated Test Count**: 10-12 tests

#### Phase 3d: Many-to-Many Relationships ⏳ NOT STARTED

**Priority**: LOW
**Estimated Time**: 2-3 days
**Depends On**: Phase 3c

**Goals**:
- Join table support (explicit and auto-generated)
- `through` relationships
- Efficient bulk operations

**Tasks**:

##### Rust Backend

- [ ] Add `ManyToMany` struct (source_table, target_table, join_table, source_fk, target_fk)
- [ ] Implement `create_join_table()` - Auto-generate join tables
- [ ] Implement `add_relation()` - Insert into join table
- [ ] Implement `remove_relation()` - Delete from join table
- [ ] Implement `fetch_related()` - Query through join table
- [ ] Implement `bulk_add_relations()` - Bulk insert into join table
- [ ] Unit tests (8-10 tests)

##### Python API

- [ ] Add `ManyToMany` descriptor
  ```python
  class User(Table):
      email: str

      roles: ManyToMany["Role"] = ManyToMany(
          through="user_roles",  # Optional: auto-created if not specified
          source_fk="user_id",
          target_fk="role_id",
      )
  ```

- [ ] Add relationship methods
  ```python
  # Add relationship
  await user.roles.add(admin_role)

  # Remove relationship
  await user.roles.remove(admin_role)

  # Fetch all related
  roles = await user.roles.fetch()

  # Bulk add
  await user.roles.add_many([role1, role2, role3])
  ```

##### Integration Tests

**File**: `tests/postgres/unit/test_many_to_many.py` (new file)

- [ ] Test auto-generated join table creation
- [ ] Test explicit join table (with extra columns)
- [ ] Test add_relation() - Single relation
- [ ] Test bulk_add_relations() - Multiple relations
- [ ] Test remove_relation()
- [ ] Test fetch_related() - Query through join table
- [ ] Test symmetric relationships (User ↔ User friendships)
- [ ] Test cascade delete (delete user → delete join table rows)

**Estimated Test Count**: 10-12 tests

---

### Phase 4: Auto-Migration Generation ⏳ NOT STARTED

**Priority**: HIGH
**Estimated Time**: 3-4 days
**Depends On**: Phase 1 ✅, Phase 3a ✅

**User Requirement**: Currently, migrations require manual SQL writing. Users want automatic migration generation based on schema changes (similar to Alembic's `alembic revision --autogenerate`).

**Goals**:
- Diff current database schema vs. Python Table definitions
- Auto-generate up/down SQL migrations
- Detect: table creation/deletion, column add/remove/modify, index changes, FK changes
- Generate reversible migrations (up + down SQL)

**Tasks**:

#### Rust Backend (data-bridge-postgres)

**File**: `crates/data-bridge-postgres/src/schema.rs`

- [ ] Add `SchemaComparator` struct
- [ ] Implement `compare_schemas()` - Diff database vs. expected schema
  - Compare tables (added, removed, modified)
  - Compare columns (added, removed, type changed, constraint changed)
  - Compare indexes (added, removed)
  - Compare foreign keys (added, removed)
- [ ] Add `SchemaDiff` struct to represent changes
  ```rust
  pub struct SchemaDiff {
      pub tables_added: Vec<String>,
      pub tables_removed: Vec<String>,
      pub columns_added: Vec<(String, ColumnInfo)>,  // (table, column)
      pub columns_removed: Vec<(String, String)>,     // (table, column)
      pub columns_modified: Vec<ColumnChange>,
      pub indexes_added: Vec<(String, IndexInfo)>,
      pub indexes_removed: Vec<(String, String)>,
      pub foreign_keys_added: Vec<(String, ForeignKeyInfo)>,
      pub foreign_keys_removed: Vec<(String, String)>,
  }
  ```

**File**: `crates/data-bridge-postgres/src/migration.rs`

- [ ] Implement `MigrationGenerator::generate_from_diff()` - Convert SchemaDiff → SQL
  - Generate CREATE TABLE statements
  - Generate DROP TABLE statements
  - Generate ALTER TABLE ADD COLUMN statements
  - Generate ALTER TABLE DROP COLUMN statements
  - Generate ALTER TABLE ALTER COLUMN statements
  - Generate CREATE INDEX statements
  - Generate DROP INDEX statements
  - Generate ALTER TABLE ADD CONSTRAINT (foreign keys)
  - Generate ALTER TABLE DROP CONSTRAINT
- [ ] Implement `generate_down_migration()` - Reverse operations
  - CREATE TABLE → DROP TABLE
  - DROP TABLE → CREATE TABLE (requires schema snapshot)
  - ADD COLUMN → DROP COLUMN
  - DROP COLUMN → ADD COLUMN (with data type)
  - Etc.
- [ ] Unit tests (15-20 tests)
  - [ ] Test table creation SQL generation
  - [ ] Test column addition SQL generation
  - [ ] Test column type change SQL generation
  - [ ] Test index creation SQL generation
  - [ ] Test foreign key creation SQL generation
  - [ ] Test down migration reversal
  - [ ] Test complex scenario: multiple changes in one migration
  - [ ] Test edge cases: column rename detection (heuristic based on name similarity)

#### PyO3 Bindings

**File**: `crates/data-bridge/src/postgres.rs`

- [ ] Add `autogenerate_migration()` function
  ```rust
  #[pyfunction]
  fn autogenerate_migration<'py>(
      py: Python<'py>,
      version: String,
      name: String,
      expected_schema: HashMap<String, TableSchema>,  // From Python Table classes
  ) -> PyResult<Bound<'py, PyDict>> {
      // Returns { "up": "...", "down": "...", "diff": {...} }
  }
  ```

#### Python API

**File**: `python/data_bridge/postgres/migrations.py`

- [ ] Add `autogenerate()` function
  ```python
  async def autogenerate(
      version: str,
      name: str,
      tables: List[Type[Table]],
  ) -> Migration:
      """
      Auto-generate migration from current schema vs. database.

      Args:
          version: Migration version (e.g., "20250129_120000")
          name: Migration description (e.g., "add_users_table")
          tables: List of Table classes representing expected schema

      Returns:
          Migration object with auto-generated up/down SQL

      Example:
          >>> from data_bridge.postgres import autogenerate
          >>> migration = await autogenerate(
          ...     "20250129_120000",
          ...     "add_users_table",
          ...     tables=[User, Post, Comment],
          ... )
          >>> print(migration.up)
          CREATE TABLE users (...);
      """
  ```

- [ ] Add CLI command: `python -m data_bridge.postgres.migrations autogenerate`
  ```bash
  # Auto-generate migration
  python -m data_bridge.postgres.migrations autogenerate \
      --name "add_users_table" \
      --tables "myapp.models:User,Post,Comment"

  # Output: migrations/20250129_120000_add_users_table.sql
  ```

#### Integration Tests

**File**: `tests/postgres/unit/test_autogenerate.py` (new file)

- [ ] Test autogenerate: new table creation
- [ ] Test autogenerate: add column to existing table
- [ ] Test autogenerate: remove column
- [ ] Test autogenerate: change column type
- [ ] Test autogenerate: add index
- [ ] Test autogenerate: add foreign key
- [ ] Test autogenerate: multiple changes (complex migration)
- [ ] Test down migration reversal
- [ ] Test no changes (empty migration)
- [ ] Test column rename detection (heuristic)
- [ ] Test error handling: conflicting changes

**Estimated Test Count**: 15-18 tests

#### Performance Targets

- Schema introspection: <500ms for 100 tables
- Diff computation: <100ms for 100 tables
- SQL generation: <50ms for any diff size

#### Acceptance Criteria

- [ ] Auto-generate up/down SQL for all schema changes
- [ ] Reversible migrations (down SQL works)
- [ ] CLI command for easy usage
- [ ] Documentation with examples
- [ ] Integration tests cover 90% of schema change scenarios
- [ ] Performance targets met

---

### Phase 5: Advanced Query Features ⏳ NOT STARTED

**Priority**: MEDIUM
**Estimated Time**: 4-5 days

**Tasks**:

- [ ] Subqueries (WHERE id IN (SELECT ...))
- [ ] Aggregation functions (COUNT, SUM, AVG, MIN, MAX)
- [ ] GROUP BY / HAVING clauses
- [ ] Window functions (ROW_NUMBER, RANK, LAG, LEAD)
- [ ] Common Table Expressions (WITH ... AS ...)
- [ ] UNION / INTERSECT / EXCEPT
- [ ] DISTINCT ON
- [ ] Array operators (ANY, ALL, CONTAINS)
- [ ] JSONB query operators (-> , ->>, @>, etc.)
- [ ] Full-text search (to_tsvector, to_tsquery)

**Estimated Test Count**: 25-30 tests

---

## MongoDB ORM (data-bridge-mongodb)

**Component**: `crates/data-bridge-mongodb/`, `crates/data-bridge/src/mongodb.rs`

**Status**: ✅ Feature Complete for Series 1xx (Type Validation), ✅ Series 9xx (Infrastructure)

### Completed Features

- ✅ Series 1xx: Type Validation System (COMPLETE)
  - Copy-on-Write state management
  - Lazy validation
  - Fast-path bulk operations
  - Rust query execution
  - Type schema extraction
  - Basic and complex type validation
  - Constraint validation
- ✅ Series 9xx: Infrastructure (COMPLETE)
  - HTTP client integration
  - Test framework

### Remaining Work

#### Series 2xx: Performance Optimization (IN PROGRESS)

**Current Performance** (vs Beanie):
- Inserts: 3.2x faster ✅ (target: 2.8x)
- Finds: 1.4x faster ✅ (target: 1.2x)

**Goals**: Push to 4-5x faster

**Tasks**:

- [ ] Feature 201: GIL-Free BSON Conversion ✅ (COMPLETE)
  - [x] Two-phase extraction pattern
  - [x] Rayon parallelization for ≥50 docs
  - [x] Vector pre-allocation
  - [x] GIL release during conversion

- [ ] Feature 202: Zero-Copy Deserialization (NOT STARTED)
  - [ ] Implement zero-copy BSON parsing
  - [ ] Direct Python-compatible memory layouts
  - [ ] Target: 4x faster than Beanie
  - **Estimated Time**: 5-7 days

- [ ] Feature 203: Bulk Operation Improvements (NOT STARTED)
  - [ ] Optimize parallel threshold (current: 50 docs)
  - [ ] Benchmark: Find optimal batch size
  - [ ] Implement streaming bulk operations (memory efficiency)
  - **Estimated Time**: 2-3 days

#### Series 3xx: Relations & References (NOT STARTED)

**Priority**: LOW (PostgreSQL relationships take precedence)
**Estimated Time**: 6-8 days

- [ ] Feature 301: Document References
  - [ ] Link and BackLink fields
  - [ ] Lazy loading with `.fetch()`
  - [ ] Eager loading with `fetch_links=True`
  - [ ] Cascade delete/update rules

- [ ] Feature 302: Reference Resolution
  - [ ] Batched reference loading
  - [ ] Circular reference detection
  - [ ] Deep reference chains (3+ levels)

**Estimated Test Count**: 20-25 tests

#### Series 4xx: Query Builder Enhancements (NOT STARTED)

**Priority**: LOW
**Estimated Time**: 4-5 days

- [ ] Feature 401: Advanced Query Operators
  - [ ] Text search ($text, $search)
  - [ ] Geospatial queries ($near, $geoWithin)
  - [ ] Array query operators ($elemMatch, $size)
  - [ ] Aggregation pipeline support

**Estimated Test Count**: 15-20 tests

---

## HTTP Client (data-bridge-http)

**Component**: `crates/data-bridge-http/`, `crates/data-bridge/src/http.rs`

**Status**: ✅ Feature Complete (Series 9xx)

### Completed Features

- ✅ High-performance HTTP client with connection pooling
- ✅ Request builder with method chaining
- ✅ Response wrapper with JSON/text/bytes support
- ✅ GIL release during network I/O
- ✅ Comprehensive error handling
- ✅ 30+ unit tests

### Performance Status

**Current**: 0.98x parity with httpx
**Target**: 1.3x faster (see ROADMAP.md Phase 1)

### Remaining Work

#### Feature HTTP-01: Lazy Response Parsing (NOT STARTED)

**Priority**: MEDIUM
**Estimated Time**: 2-3 days

**Goal**: Delay PyObject creation until accessed

**Tasks**:

- [ ] Implement lazy header parsing
  - Store headers in Rust HashMap
  - Convert to PyDict only when `.headers` is accessed
- [ ] Implement lazy body parsing
  - Store body as bytes in Rust
  - Convert to PyBytes/PyString only when accessed
- [ ] Add zero-copy bytes support (where possible)
- [ ] Benchmark: Target 1.3x faster than httpx
- [ ] Unit tests (5-8 tests)

**Estimated Test Count**: 5-8 tests

---

## Test Framework (data-bridge-test)

**Component**: `crates/data-bridge-test/`, `crates/data-bridge/src/test.rs`

**Status**: ✅ Feature Complete (Series 9xx)

### Completed Features

- ✅ Benchmarking engine (40,199 lines)
- ✅ Custom assertions (19,283 lines)
- ✅ Test runner (12,883 lines)
- ✅ Comprehensive test suite (313+ Python tests)

### Remaining Work

#### Feature TEST-01: Rust-Accelerated Discovery (NOT STARTED)

**Priority**: LOW
**Estimated Time**: 2-3 days

**Goal**: Handle 10,000+ test files in <100ms

**Tasks**:

- [ ] Implement parallel test file discovery
- [ ] Optimize file tree traversal
- [ ] Add caching for test metadata
- [ ] Benchmark with large codebases
- [ ] Target: <100ms for 10,000 files

**Estimated Test Count**: 5-8 tests

---

## Documentation & Tooling

### High Priority

- [ ] **Relationship Usage Guide** (NOT STARTED)
  - Comprehensive examples for ForeignKey, BackReference, ManyToMany
  - Best practices for eager vs lazy loading
  - Performance considerations
  - **Estimated Time**: 1 day

- [ ] **Upsert Guide** (BLOCKED: Waiting for Phase 2 completion)
  - Usage examples
  - Performance tips
  - Comparison with insert + update
  - **Estimated Time**: 2-3 hours

- [ ] **Auto-Migration Guide** (BLOCKED: Waiting for Phase 4 completion)
  - Workflow documentation
  - CLI usage examples
  - Troubleshooting common issues
  - **Estimated Time**: 1 day

### Medium Priority

- [ ] **PostgreSQL ORM Tutorial** (NOT STARTED)
  - Getting started guide
  - CRUD operations walkthrough
  - Transaction usage
  - Schema management
  - **Estimated Time**: 2-3 days

- [ ] **Performance Tuning Guide** (NOT STARTED)
  - Relationship query optimization
  - Bulk operation best practices
  - Index usage
  - Connection pool tuning
  - **Estimated Time**: 1-2 days

### Low Priority

- [ ] **API Reference** (Partially complete)
  - Complete docstrings for all public APIs
  - Add code examples to all functions
  - **Estimated Time**: 2-3 days

- [ ] **Migration from SQLAlchemy** (NOT STARTED)
  - Feature comparison
  - Code migration examples
  - Performance comparison
  - **Estimated Time**: 1-2 days

---

## Performance Optimization

**See**: `/Users/chrischeng/projects/data-bridge/ROADMAP.md` for detailed roadmap

### Phase 1: High-Frequency Operations (Q1 2026)

**Priority**: HIGH

#### POSTGRES-01: Single-Row Insert Fast-Path

**Current**: 0.74x slower than SQLAlchemy (BOTTLENECK)
**Target**: 1.5x faster

**Tasks**:

- [ ] Profile single-row insert overhead
- [ ] Reduce FFI boundary crossings
- [ ] Avoid HashMap construction for single values
- [ ] Implement specialized fast-path entry point
- [ ] Benchmark vs SQLAlchemy
- [ ] Target: <0.5ms per insert (vs 0.67ms SQLAlchemy)

**Estimated Time**: 2-3 days

#### HTTP-01: Lazy Response Parsing

(See HTTP Client section above)

#### CORE-01: Connection Pool Lock-Free Path

**Current**: Global RwLock for all connections
**Target**: Thread-local or lock-free atomic references

**Tasks**:

- [ ] Profile lock contention under high concurrency
- [ ] Implement thread-local connection caching
- [ ] Benchmark with 10+ concurrent workers
- [ ] Target: 2x higher throughput under contention

**Estimated Time**: 3-4 days

### Phase 2: Advanced Data Handling (Q2 2026)

#### MONGO-01: Zero-Copy Deserialization

(See MongoDB section above)

#### POSTGRES-02: Binary Protocol Optimization

**Tasks**:

- [ ] Leverage sqlx binary protocol more efficiently
- [ ] Reduce intermediate allocations
- [ ] Benchmark large result sets (10,000+ rows)
- [ ] Target: 30% faster than current text protocol

**Estimated Time**: 3-4 days

---

## Priority Summary

### Week 1-2 (Jan 2026)

1. ✅ **URGENT**: Complete upsert implementation (2-4 hours)
   - PyO3 bindings
   - Python wrappers
   - Integration tests

2. **HIGH**: Implement auto-migration generation (3-4 days)
   - Schema diffing
   - SQL generation
   - CLI command

3. **HIGH**: Optimize PostgreSQL single-row operations (2-3 days)
   - Profile bottlenecks
   - Implement fast-path
   - Benchmark vs SQLAlchemy

### Week 3-4 (Jan 2026)

4. **MEDIUM**: JOIN-based eager loading (3-4 days)
   - QueryBuilder JOIN support
   - fetch_links parameter
   - Batched loading

5. **MEDIUM**: BackReference and cascade (2-3 days)
   - Reverse relationships
   - Cascade delete/update

### Week 5-6 (Feb 2026)

6. **MEDIUM**: Documentation sprint (4-5 days)
   - Relationship usage guide
   - PostgreSQL ORM tutorial
   - Performance tuning guide

7. **LOW**: Many-to-many relationships (2-3 days)
   - Join table support
   - Bulk operations

### Later (Q1-Q2 2026)

8. **LOW**: Advanced query features (4-5 days)
9. **LOW**: MongoDB zero-copy deserialization (5-7 days)
10. **LOW**: HTTP lazy response parsing (2-3 days)

---

## Success Metrics

### PostgreSQL ORM

- [ ] All CRUD operations faster than SQLAlchemy (currently 0.74x on single-row)
- [ ] Relationship queries: 1 query with eager loading vs N+1 without
- [ ] Auto-migration: <500ms for 100-table schema diff
- [ ] Test coverage: >85% (currently ~80%)
- [ ] Zero unsafe `unwrap()` calls in production code ✅

### MongoDB ORM

- [ ] 4x faster than Beanie (currently 3.2x on inserts, 1.4x on finds)
- [ ] Zero Python heap pressure for BSON operations ✅
- [ ] Reference resolution: <10ms for 100 document batch

### HTTP Client

- [ ] 1.3x faster than httpx (currently 0.98x)
- [ ] Zero-copy response bodies where possible
- [ ] <5ms overhead for JSON parsing

### Overall

- [ ] 100% GIL release during I/O ✅
- [ ] 100% parameterized queries (SQL injection prevention) ✅
- [ ] Comprehensive documentation for all public APIs
- [ ] Production-ready (no panics, proper error handling) ✅

---

**End of TODO List**
