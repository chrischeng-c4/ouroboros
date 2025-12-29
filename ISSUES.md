# data-bridge Known Issues and Decisions

This document tracks known issues, blockers, design decisions, and items requiring user input.

**Last Updated**: 2025-12-29
**Current Branch**: `feature/postgres-improve`

---

## Table of Contents

- [Known Issues](#known-issues)
- [Design Decisions Needed](#design-decisions-needed)
- [Performance Concerns](#performance-concerns)
- [Testing Gaps](#testing-gaps)
- [Documentation Gaps](#documentation-gaps)
- [Breaking Changes](#breaking-changes)
- [Compatibility Notes](#compatibility-notes)
- [Security Considerations](#security-considerations)

---

## Known Issues

### Issue #1: Upsert Implementation Incomplete

**Component**: PostgreSQL ORM
**Status**: ⏳ In Progress (80% complete)
**Severity**: 🟡 Medium
**Reported**: 2025-12-29
**Assignee**: Next session

**Description**:

The upsert (INSERT ... ON CONFLICT) feature is partially implemented:
- ✅ Rust backend (`QueryBuilder::build_upsert`, `Row::upsert`, `Row::upsert_many`) - COMPLETE
- ✅ Unit tests (5 tests) - COMPLETE
- ❌ PyO3 bindings (`upsert_one`, `upsert_many` in postgres.rs) - NOT STARTED
- ❌ Python wrappers (connection.py) - NOT STARTED
- ❌ Integration tests - NOT STARTED

**Impact**:

Users cannot use the upsert feature from Python. This is a common operation for data synchronization and deduplication workflows.

**Resolution Plan**:

1. Add `upsert_one()` and `upsert_many()` to `crates/data-bridge/src/postgres.rs`
2. Add Python wrapper functions in `python/data_bridge/postgres/connection.py`
3. Export functions in `python/data_bridge/postgres/__init__.py`
4. Create integration test file: `tests/postgres/unit/test_upsert.py` (9-12 tests)
5. Run benchmarks to ensure parallel processing works for ≥50 rows

**Estimated Fix Time**: 2-4 hours

**Files Affected**:
- `/Users/chrischeng/projects/data-bridge/crates/data-bridge/src/postgres.rs`
- `/Users/chrischeng/projects/data-bridge/python/data_bridge/postgres/connection.py`
- `/Users/chrischeng/projects/data-bridge/python/data_bridge/postgres/__init__.py`
- `/Users/chrischeng/projects/data-bridge/tests/postgres/unit/test_upsert.py` (new)

**Workaround**:

Currently, users must manually implement upsert logic:
```python
# Current workaround (not optimal)
existing = await fetch_one("users", User.email == "alice@example.com")
if existing:
    await update_one("users", User.email == "alice@example.com", {"name": "Alice", "age": 30})
else:
    await insert_one("users", {"email": "alice@example.com", "name": "Alice", "age": 30})

# Desired API (pending implementation)
await upsert_one(
    "users",
    {"email": "alice@example.com", "name": "Alice", "age": 30},
    conflict_columns=["email"]
)
```

---

### Issue #2: No Auto-Migration Generation

**Component**: PostgreSQL ORM
**Status**: ❌ Not Started
**Severity**: 🔴 High (user requirement)
**Reported**: User request (implicit from Alembic comparison)
**Assignee**: Unassigned

**Description**:

The migration system requires users to manually write SQL for schema changes. There is no auto-generation tool similar to Alembic's `alembic revision --autogenerate`.

Current workflow:
1. User modifies Python `Table` classes
2. User manually writes SQL migration file
3. User runs migration

This is tedious and error-prone for:
- Large schema changes (10+ tables)
- Complex column type changes
- Adding/removing foreign keys
- Index management

**Impact**:

- **Developer Experience**: Tedious manual work for every schema change
- **Error Risk**: Manual SQL writing is error-prone
- **Adoption Blocker**: Users migrating from Alembic/SQLAlchemy expect autogeneration
- **Time Cost**: 5-30 minutes per migration (vs <1 minute with autogeneration)

**Resolution Plan**:

Implement Phase 4 (Auto-Migration Generation) from TODOS.md:

1. **Schema Diffing** (1-2 days)
   - Implement `SchemaComparator` to diff database vs. Python Table definitions
   - Detect: tables added/removed, columns added/removed/modified, indexes, foreign keys

2. **SQL Generation** (1-2 days)
   - Implement `MigrationGenerator::generate_from_diff()`
   - Generate CREATE TABLE, ALTER TABLE, CREATE INDEX, etc.
   - Generate reversible down migrations

3. **CLI Command** (0.5 days)
   - Add `python -m data_bridge.postgres.migrations autogenerate`
   - Integrate with existing migration system

4. **Testing** (0.5-1 day)
   - 15-18 integration tests covering schema change scenarios

**Dependencies**:

- ✅ Schema introspection (COMPLETE)
- ✅ Foreign key introspection (COMPLETE)
- ✅ Migration system (COMPLETE)

**Estimated Fix Time**: 3-4 days

**Priority**: HIGH (after upsert completion)

**Workaround**:

Users must manually write SQL migrations:

```python
# Current workflow (manual)
# 1. Create migration file: migrations/20250129_120000_add_users_table.sql
"""
-- Migration: 20250129_120000_add_users_table
-- Description: Add users table

-- UP
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    name TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- DOWN
DROP TABLE users;
"""

# 2. Run migration
await run_migrations("migrations/")
```

**Desired API** (pending implementation):

```bash
# Auto-generate migration from schema diff
python -m data_bridge.postgres.migrations autogenerate \
    --name "add_users_table" \
    --tables "myapp.models:User,Post,Comment"

# Output: migrations/20250129_120000_add_users_table.sql (auto-generated)
```

---

### Issue #3: N+1 Query Problem with ForeignKeyProxy

**Component**: PostgreSQL ORM - Relationships
**Status**: ✅ Known Limitation (Phase 1)
**Severity**: 🟡 Medium (performance degradation)
**Reported**: 2025-12-29 (design trade-off)

**Description**:

The current `ForeignKeyProxy` implementation uses lazy loading, which creates a separate query for each relationship:

```python
# Fetches 1 query for posts
posts = await fetch_many("posts", limit=10)

# Fetches 10 additional queries (N+1 problem)
for post in posts:
    author = await post.author.fetch()  # Separate query per post
```

This is a classic N+1 query problem: 1 query for the main table + N queries for relationships.

**Impact**:

- **Performance**: 10x slower for 100 rows with relationships (10ms → 100ms)
- **Database Load**: High query count (1 + N queries)
- **Latency**: Network round-trips for each relationship

**Mitigation**:

This is a **deliberate Phase 1 design**. Phase 2 will add JOIN-based eager loading:

```python
# Phase 2 API (pending implementation)
posts = await fetch_many("posts", limit=10, fetch_links=["author"])

# Result: 1 query with JOIN, all authors pre-loaded
for post in posts:
    print(post["author"]["name"])  # No additional query!
```

**Resolution Plan**:

Implement Phase 3b (JOIN-Based Eager Loading) from TODOS.md:

1. Add `JoinType` enum and `Join` struct
2. Extend `QueryBuilder` with `.join()` method
3. Implement `fetch_with_relations()` in Rust
4. Add `fetch_links` parameter to Python API
5. Implement batched relationship loading for efficiency

**Estimated Fix Time**: 3-4 days

**Priority**: MEDIUM (after auto-migration)

**Current Workaround**:

Users can manually use JOINs with raw SQL:

```python
# Manual JOIN (current workaround)
posts_with_authors = await execute("""
    SELECT
        posts.*,
        users.name AS author_name,
        users.email AS author_email
    FROM posts
    LEFT JOIN users ON posts.author_id = users.id
    WHERE posts.status = $1
    LIMIT 10
""", ["published"])
```

---

### Issue #4: PostgreSQL Single-Row Insert Slower Than SQLAlchemy

**Component**: PostgreSQL ORM - Performance
**Status**: 🔴 Active Bottleneck
**Severity**: 🔴 High (performance regression)
**Reported**: 2025-12-26 (benchmark results in ROADMAP.md)

**Description**:

Benchmark results show single-row insert operations are **0.74x slower** than SQLAlchemy:

| Operation | data-bridge | SQLAlchemy | Ratio |
|-----------|-------------|------------|-------|
| Single Insert | 1.35ms | 0.67ms | 0.74x (slower) |
| Bulk Insert (1000) | 547ms | 897ms | 1.6x (faster) |

**Root Cause** (Hypothesis):

1. **FFI Overhead**: PyO3 boundary crossing for each insert
2. **HashMap Construction**: Full HashMap allocation for single values
3. **No Fast-Path**: Single-row operations use same code path as bulk operations
4. **GIL Lock/Unlock**: Overhead for small operations

**Impact**:

- **User Perception**: "Why is this ORM slower for simple inserts?"
- **Adoption Blocker**: Users won't migrate if basic operations are slower
- **Benchmark Failure**: Cannot claim "faster than SQLAlchemy" with this bottleneck

**Performance Target**:

- Current: 1.35ms per insert (0.74x slower)
- **Target**: 0.45ms per insert (1.5x faster than SQLAlchemy)
- **Improvement Needed**: 3x speedup

**Resolution Plan**:

Implement ROADMAP.md Phase 1: POSTGRES-01 Single-Row Insert Fast-Path

1. **Profile Current Implementation** (0.5 days)
   - Identify bottleneck: FFI vs HashMap vs connection pool
   - Measure time spent in each layer

2. **Implement Fast-Path** (1 day)
   - Create specialized entry point for single-row inserts
   - Avoid HashMap allocation (use Vec<(String, ExtractedValue)>)
   - Reduce FFI boundary crossings
   - Optimize parameterized query generation

3. **Benchmark & Iterate** (0.5 days)
   - Run benchmark suite
   - Compare vs SQLAlchemy
   - Iterate if target not met

4. **Verify No Regression** (0.5 days)
   - Ensure bulk operations still fast
   - Run full test suite

**Estimated Fix Time**: 2-3 days

**Priority**: HIGH (Week 1-2)

**Workaround**:

Use bulk operations even for small batches:

```python
# Slower (current)
for user in users:
    await insert_one("users", user)  # 1.35ms each

# Faster (workaround)
await insert_many("users", users)  # 0.55ms per row (bulk)
```

---

### Issue #5: No Documentation for Relationship Usage

**Component**: Documentation
**Status**: ❌ Not Started
**Severity**: 🟡 Medium (usability)
**Reported**: 2025-12-29

**Description**:

The ForeignKeyProxy feature is implemented but lacks comprehensive documentation:
- No usage guide for lazy loading
- No examples of common patterns
- No performance best practices
- No troubleshooting guide

**Impact**:

- Users struggle to understand when to use `.fetch()` vs `.ref`
- No guidance on avoiding N+1 queries
- Unclear how to handle nullable foreign keys

**Resolution Plan**:

Create `/docs/postgres/relationships.md` with:

1. Quick start examples
2. Lazy loading explanation
3. Common patterns (nullable FKs, circular refs)
4. Performance considerations
5. Migration from SQLAlchemy relationship()

**Estimated Fix Time**: 1 day

**Priority**: MEDIUM (after Phase 2 completion)

---

## Design Decisions Needed

### Decision #1: ORM API for Relationships ✅ DECIDED

**Question**: Should we use SQLAlchemy-style `relationship()` or keep it minimal?

**Options**:

1. **Full SQLAlchemy Compatibility**
   - Use `relationship()` function with backref, lazy strategies
   - Maximum compatibility for SQLAlchemy users
   - More complex implementation

2. **Minimal API** (CHOSEN)
   - Just `ForeignKey[T]` and `BackReference[T]` descriptors
   - Simpler mental model
   - Follows data-bridge-mongodb pattern
   - Explicit lazy/eager loading via `fetch_links` parameter

3. **Hybrid Approach**
   - Basic relationships + optional advanced features
   - Progressive complexity

**Decision**: **Option 2 (Minimal)** ✅

**Rationale**:
- Follows data-bridge philosophy: explicit over implicit
- Simpler API surface reduces bugs
- Compatible with data-bridge-mongodb patterns
- Users can migrate to SQLAlchemy if they need advanced features

**Status**: Decided (implemented in Phase 1)

**Implementation**:

```python
# Current API (minimal)
class Post(Table):
    author_id: int = Column(foreign_key="users.id")

# Usage
post = await Post.fetch_one(Post.id == 1)
author = await post.author.fetch()  # Explicit lazy loading

# Or eager loading (Phase 2)
post = await Post.fetch_one(Post.id == 1, fetch_links=["author"])
print(post["author"]["name"])  # No additional query
```

---

### Decision #2: Migration Autogen Scope

**Question**: How comprehensive should auto-migration generation be?

**Options**:

1. **Full Alembic Compatibility**
   - Detect ALL schema changes (tables, columns, indexes, constraints, triggers, functions, views)
   - Handle complex scenarios (column renames, table splits, data migrations)
   - Maximum feature parity

2. **Basic (MVP)** ⭐ RECOMMENDED
   - Tables: CREATE, DROP
   - Columns: ADD, DROP, ALTER (type, nullable, default)
   - Indexes: CREATE, DROP
   - Foreign keys: ADD, DROP
   - Expand later based on user feedback

3. **Advanced**
   - Option 2 + triggers, functions, views, sequences
   - Custom migration hooks for data transformations

**Recommendation**: **Option 2 (Basic)** for Phase 4 MVP

**Rationale**:
- Covers 90% of use cases
- Simpler implementation (3-4 days)
- Can expand to Option 3 later based on user demand
- Alembic took years to reach full feature parity

**Status**: ⏳ Pending User Confirmation

**User Input Needed**:
- What schema changes do you encounter most frequently?
- Do you need trigger/function/view support in Phase 1?
- Are you willing to wait longer (7-10 days) for Option 3?

**Decision Deadline**: Before starting Phase 4 implementation

---

### Decision #3: Many-to-Many Implementation

**Question**: Should join tables be explicit or implicit?

**Options**:

1. **Explicit Only**
   - User always defines join table as a `Table` class
   - Full control over join table schema
   - More verbose

   ```python
   class UserRole(Table):
       user_id: int = Column(foreign_key="users.id")
       role_id: int = Column(foreign_key="roles.id")
       granted_at: datetime = Column(default_factory=datetime.utcnow)
   ```

2. **Implicit Only**
   - Auto-create join tables with standard schema
   - Less control, simpler API
   - Cannot add extra columns to join table

   ```python
   class User(Table):
       roles: ManyToMany["Role"] = ManyToMany()
   # Auto-creates "user_roles" table with (user_id, role_id)
   ```

3. **Both (Hybrid)** ⭐ RECOMMENDED
   - Auto-create by default
   - Allow explicit join table for customization
   - Best of both worlds

   ```python
   # Implicit (auto-created)
   class User(Table):
       roles: ManyToMany["Role"] = ManyToMany()

   # Explicit (custom join table)
   class User(Table):
       roles: ManyToMany["Role"] = ManyToMany(through=UserRole)
   ```

**Recommendation**: **Option 3 (Both)** for flexibility

**Rationale**:
- Implicit is simpler for common cases (no extra columns)
- Explicit allows customization when needed (audit timestamps, soft deletes)
- Django ORM and SQLAlchemy both support this pattern

**Status**: ⏳ Pending User Confirmation

**User Input Needed**:
- Do you frequently need extra columns in join tables?
- Is the hybrid API (Option 3) too complex?
- Preference for default behavior?

**Decision Deadline**: Before starting Phase 3d implementation

---

### Decision #4: Error Handling Strategy for Relationships

**Question**: How should we handle missing foreign key references?

**Options**:

1. **Strict Mode (Raise Error)**
   - `await post.author.fetch()` raises error if author doesn't exist
   - Ensures data integrity
   - May be too strict for some use cases

2. **Nullable Mode (Return None)**
   - `await post.author.fetch()` returns None if author doesn't exist
   - More lenient
   - May hide data integrity issues

3. **Configurable** ⭐ RECOMMENDED
   - Default: Nullable (return None)
   - Option: Strict mode via `Column(foreign_key="users.id", strict=True)`
   - Best of both worlds

**Recommendation**: **Option 3 (Configurable)** with nullable default

**Rationale**:
- Nullable default matches PostgreSQL LEFT JOIN behavior
- Strict mode available for critical relationships
- Users can choose based on their data integrity requirements

**Status**: ⏳ Pending User Confirmation

**User Input Needed**:
- Do you prefer strict or nullable by default?
- Should we match SQLAlchemy behavior (nullable)?

**Decision Deadline**: Before Phase 3b implementation

---

## Performance Concerns

### Concern #1: N+1 Queries with ForeignKeyProxy ⚠️

**Status**: ✅ Known, Mitigated in Phase 2

**Description**: (See Issue #3 above)

**Severity**: 🟡 Medium (temporary limitation)

**Mitigation Timeline**:
- **Phase 1** (CURRENT): Lazy loading only, document N+1 risk
- **Phase 2** (Week 3-4): JOIN-based eager loading via `fetch_links`
- **Future**: Automatic batched loading (DataLoader pattern)

**Performance Impact**:
- 10 rows: 11 queries (1 + 10) → ~20ms
- 100 rows: 101 queries (1 + 100) → ~200ms
- 1000 rows: 1001 queries (1 + 1000) → ~2000ms

**Acceptable For**:
- Small result sets (<10 rows)
- Rare operations
- Prototyping

**Not Acceptable For**:
- Large result sets (>50 rows)
- High-frequency endpoints
- Production APIs

**Action Items**:
- [ ] Document N+1 risk in relationship guide
- [ ] Add warning in ForeignKeyProxy docstring
- [ ] Implement Phase 2 (JOIN-based eager loading) by Week 3-4

---

### Concern #2: Migration Generation Performance 🔵

**Status**: ⏳ Speculative (not yet implemented)

**Description**:

Auto-migration generation might be slow for large databases:
- Schema introspection: Query all tables, columns, indexes, foreign keys
- 100 tables × 10 columns = 1000 rows to introspect
- Complex diff algorithm

**Potential Impact**:
- Slow CLI experience (>5 seconds)
- Timeout for very large databases (1000+ tables)

**Mitigation Strategies**:

1. **Parallel Schema Introspection** (PLANNED)
   - Use Rayon to query multiple tables in parallel
   - Target: <500ms for 100 tables

2. **Caching** (PLANNED)
   - Cache introspection results in `.migration_cache.json`
   - Invalidate on schema version change

3. **Incremental Diffing** (FUTURE)
   - Only introspect tables that changed
   - Track last introspection timestamp

4. **Progress Indicator** (NICE-TO-HAVE)
   - Show progress bar for large databases
   - "Introspecting schema (45/100 tables)..."

**Performance Targets**:
- 10 tables: <100ms
- 100 tables: <500ms
- 1000 tables: <5 seconds

**Action Items**:
- [ ] Implement parallel introspection in Phase 4
- [ ] Add benchmarks for schema introspection
- [ ] Monitor performance during Phase 4 implementation
- [ ] Add caching if targets not met

---

### Concern #3: Single-Row Insert Performance 🔴

**Status**: 🔴 Active Bottleneck (See Issue #4)

**Current**: 0.74x slower than SQLAlchemy (1.35ms vs 0.67ms)

**Impact**: Major adoption blocker

**Mitigation**: POSTGRES-01 fast-path (HIGH priority, Week 1-2)

---

### Concern #4: Bulk Upsert Performance 🔵

**Status**: ⏳ Not Yet Benchmarked

**Description**:

`upsert_many()` implementation uses Rayon parallelization, but performance has not been verified:
- Is parallel threshold (50 rows) optimal for upsert?
- Does ON CONFLICT add significant overhead?
- How does it compare to separate insert_many + update_many?

**Action Items**:
- [ ] Benchmark upsert_many with 10, 100, 1000, 10000 rows
- [ ] Compare vs insert_many performance
- [ ] Tune parallel threshold if needed
- [ ] Document performance characteristics

**Timeline**: During upsert implementation completion (Week 1)

---

## Testing Gaps

### Gap #1: Upsert Integration Tests ❌

**Status**: ❌ Not Started (0/9 tests)

**Missing Tests**:
1. Single-row upsert (insert)
2. Single-row upsert (update)
3. Bulk upsert (no conflicts)
4. Bulk upsert (all conflicts)
5. Bulk upsert (mixed insert/update)
6. Selective column update
7. Composite unique constraints
8. Error handling (invalid conflict columns)
9. Parallel processing (≥50 rows)

**Timeline**: Week 1 (during upsert completion)

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/unit/test_upsert.py` (new)

---

### Gap #2: Relationship Performance Tests ❌

**Status**: ❌ Not Started (0/5 tests)

**Missing Benchmarks**:
1. Lazy loading N+1 query time (10, 100, 1000 rows)
2. Eager loading with JOIN (Phase 2)
3. Batched relationship loading (Phase 2)
4. Memory usage: lazy vs eager
5. Complex query performance (3+ level relationships)

**Timeline**: Phase 3b (Week 3-4)

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/benchmarks/bench_relationships.py` (new)

---

### Gap #3: Migration Edge Cases ❌

**Status**: ❌ Not Started (0/10 tests)

**Missing Tests**:
1. Column rename detection (heuristic)
2. Table rename detection
3. Complex multi-step migration
4. Migration rollback (down SQL)
5. Conflicting migrations (merge conflicts)
6. Migration checksum mismatch
7. Partial migration failure (rollback)
8. Migration on non-empty table (data preservation)
9. Foreign key dependency order
10. Circular foreign key handling

**Timeline**: Phase 4 (Week 2-3)

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/unit/test_migration_edge_cases.py` (new)

---

### Gap #4: Cascade Operation Tests ❌

**Status**: ❌ Not Started (0/12 tests - Phase 3c)

**Missing Tests**:
1. ON DELETE CASCADE (delete parent → delete children)
2. ON DELETE RESTRICT (prevent delete if children exist)
3. ON DELETE SET NULL (delete parent → set FK to NULL)
4. ON UPDATE CASCADE (update parent PK → update FK)
5. BackReference query (user.posts)
6. Nested cascade (3+ levels)
7. Circular cascade detection
8. WriteRule.Deny (prevent orphan creation)
9. Soft delete cascade
10. Cascade performance (1000+ children)
11. Transaction rollback with cascade
12. Deferred constraint checking

**Timeline**: Phase 3c (Week 4-5)

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/unit/test_cascade.py` (new)

---

### Gap #5: Many-to-Many Tests ❌

**Status**: ❌ Not Started (0/10 tests - Phase 3d)

**Missing Tests**:
1. Auto-generated join table creation
2. Explicit join table (with extra columns)
3. Add relationship (single)
4. Bulk add relationships
5. Remove relationship
6. Fetch related (query through join table)
7. Symmetric relationships (User ↔ User friendships)
8. Cascade delete (delete parent → delete join table rows)
9. Many-to-many with composite keys
10. Performance: bulk operations (1000+ relations)

**Timeline**: Phase 3d (Week 5-6)

**File**: `/Users/chrischeng/projects/data-bridge/tests/postgres/unit/test_many_to_many.py` (new)

---

## Documentation Gaps

### Gap #1: Relationship Usage Guide ❌ HIGH PRIORITY

**Status**: ❌ Not Started
**Severity**: 🟡 Medium (usability)

**Missing Content**:
1. Quick start examples (basic foreign key usage)
2. Lazy loading explanation (when to use `.fetch()` vs `.ref`)
3. Eager loading guide (Phase 2 - `fetch_links` parameter)
4. Common patterns:
   - Nullable foreign keys
   - Circular relationships
   - Self-referential relationships
5. Performance best practices:
   - Avoiding N+1 queries
   - When to use lazy vs eager loading
   - Batching relationship queries
6. Troubleshooting:
   - "Foreign key not found" errors
   - Circular dependency issues
   - Performance debugging
7. Migration from SQLAlchemy:
   - relationship() vs ForeignKey[T]
   - backref vs BackReference[T]
   - lazy strategies comparison

**Timeline**: Week 3-4 (after Phase 2 completion)

**File**: `/docs/postgres/relationships.md` (new)

**Estimated Time**: 1 day

---

### Gap #2: Upsert Guide ❌

**Status**: ❌ Blocked (waiting for implementation)
**Severity**: 🟢 Low (feature not yet available)

**Missing Content**:
1. Usage examples (single and bulk)
2. Conflict resolution strategies
3. Performance tips (when to use vs insert + update)
4. Comparison with other databases (MySQL REPLACE, MongoDB upsert)
5. Error handling

**Timeline**: Week 1 (after upsert completion)

**File**: `/docs/postgres/upsert.md` (new)

**Estimated Time**: 2-3 hours

---

### Gap #3: Auto-Migration Workflow Guide ❌

**Status**: ❌ Blocked (waiting for Phase 4)
**Severity**: 🟡 Medium (major feature)

**Missing Content**:
1. CLI usage examples
2. Workflow: develop → autogenerate → review → apply
3. Best practices:
   - Reviewing generated SQL before applying
   - Testing migrations on staging
   - Rolling back migrations
4. Troubleshooting:
   - Handling column renames (heuristic limitations)
   - Resolving migration conflicts
   - Manual migration editing
5. Integration with version control (Git)
6. Team collaboration (multiple developers, migration conflicts)

**Timeline**: Week 2-3 (after Phase 4 completion)

**File**: `/docs/postgres/migrations.md` (expand existing)

**Estimated Time**: 1 day

---

### Gap #4: PostgreSQL ORM Tutorial ❌

**Status**: ❌ Not Started
**Severity**: 🟡 Medium (adoption)

**Missing Content**:
1. Getting started (installation, setup)
2. CRUD operations walkthrough
3. Query builder tutorial
4. Transaction usage
5. Schema management
6. Performance tuning
7. Best practices

**Timeline**: Week 5-6 (documentation sprint)

**File**: `/docs/postgres/tutorial.md` (new)

**Estimated Time**: 2-3 days

---

### Gap #5: Performance Tuning Guide ❌

**Status**: ❌ Not Started
**Severity**: 🟢 Low (advanced users)

**Missing Content**:
1. Relationship query optimization
2. Bulk operation best practices
3. Index usage and strategy
4. Connection pool tuning
5. Query profiling
6. Benchmarking guide

**Timeline**: Week 5-6 (documentation sprint)

**File**: `/docs/postgres/performance.md` (new)

**Estimated Time**: 1-2 days

---

### Gap #6: API Reference Completion 🟡

**Status**: ⏳ Partially Complete (~60%)
**Severity**: 🟡 Medium (usability)

**Missing Content**:
1. Incomplete docstrings for some functions
2. Missing code examples in docstrings
3. No API reference website (Sphinx/mkdocs)
4. Missing type annotations in some Python files

**Timeline**: Ongoing (incremental improvement)

**Estimated Time**: 2-3 days for completion

---

## Breaking Changes

### None Currently Planned ✅

**Status**: ✅ All new features are additive

All planned features (upsert, relationships, auto-migration) are **additive** and do not break existing APIs.

**Commitment**:
- No breaking changes until v1.0.0
- All APIs remain backward compatible
- Deprecation warnings for any future changes

---

## Compatibility Notes

### PostgreSQL Version Compatibility

**Minimum Version**: PostgreSQL 9.5+

**Reason**: `INSERT ... ON CONFLICT` (upsert) requires PostgreSQL 9.5+

**Tested Versions**:
- ✅ PostgreSQL 13 (primary testing)
- ✅ PostgreSQL 14
- ✅ PostgreSQL 15
- ✅ PostgreSQL 16

**Not Tested**:
- ⚠️ PostgreSQL 9.5-12 (should work but not verified)
- ❌ PostgreSQL <9.5 (upsert will fail)

**Action Items**:
- [ ] Add version detection in connection.py
- [ ] Warn users if PostgreSQL <9.5 detected
- [ ] Add compatibility matrix to README

---

### Python Version Compatibility

**Minimum Version**: Python 3.12+

**Reason**: PyO3 0.24+ requires Python 3.12+

**Tested Versions**:
- ✅ Python 3.12 (primary testing)
- ❌ Python 3.11 (not tested, likely incompatible)
- ❌ Python 3.10 (not compatible)

**Action Items**:
- [ ] Document Python 3.12+ requirement prominently
- [ ] Add version check in setup.py/pyproject.toml
- [ ] Consider backporting to Python 3.11 (if user demand exists)

---

### Rust Version Compatibility

**Minimum Version**: Rust 1.70+

**Reason**: data-bridge uses Rust 2021 edition features

**Tested Versions**:
- ✅ Rust 1.70
- ✅ Rust 1.75 (current)

**Action Items**:
- None (Rust version is a build-time dependency, not user-facing)

---

### Migration File Format Compatibility

**Current Format**: SQL with special comments

**Example**:
```sql
-- Migration: 20250129_120000_add_users_table
-- Description: Add users table

-- UP
CREATE TABLE users (...);

-- DOWN
DROP TABLE users;
```

**Compatibility**:
- ✅ Forward compatible (old migrations work with new code)
- ✅ Backward compatible (new migrations work with old code)
- ✅ Plain SQL (can be applied manually if needed)

**Action Items**:
- [ ] Document migration format in docs/postgres/migrations.md
- [ ] Add format version to migration files (for future changes)

---

## Security Considerations

### SQL Injection Prevention ✅

**Status**: ✅ Mitigated

**Measures**:
1. ✅ All queries use parameterized statements ($1, $2, etc.)
2. ✅ Identifier validation (table/column names)
3. ✅ No string concatenation for user input
4. ✅ QueryBuilder uses type-safe ExtractedValue enum

**Testing**:
- ✅ 28 security tests in `tests/test_security.rs`
- ✅ Fuzz testing for identifiers
- ✅ SQL injection tests (blocked)

**Remaining Work**:
- [ ] Add more fuzz tests (expand coverage)
- [ ] Security audit before v1.0.0
- [ ] Penetration testing (external)

---

### Foreign Key Reference Validation ✅

**Status**: ✅ Implemented

**Measures**:
1. ✅ Validate table and column names in `foreign_key` parameter
2. ✅ Prevent system table access (pg_*, information_schema)
3. ✅ Block dangerous patterns (DROP, DELETE, --, /*, etc.)
4. ✅ 10 unit tests for validation edge cases

**Example**:
```python
# ✅ Valid
Column(foreign_key="users.id")

# ❌ Blocked (system table)
Column(foreign_key="pg_catalog.pg_class.oid")

# ❌ Blocked (SQL injection)
Column(foreign_key="users; DROP TABLE users--")
```

**Remaining Work**:
- [ ] Add runtime validation (check if referenced table/column exists)
- [ ] Add documentation for security best practices

---

### Connection Pool Security 🟡

**Status**: ⏳ Basic (no advanced features)

**Current**:
- ✅ Connection pooling via SQLx (secure)
- ✅ Parameterized queries (no SQL injection)
- ❌ No TLS/SSL enforcement
- ❌ No connection string validation
- ❌ No credential rotation support

**Remaining Work**:
- [ ] Add TLS/SSL configuration option
- [ ] Validate connection strings (prevent malicious URIs)
- [ ] Add support for credential rotation
- [ ] Document security best practices for production

**Timeline**: Phase 5 (security hardening)

---

### Data Validation ⚠️

**Status**: ⚠️ Limited (relies on PostgreSQL constraints)

**Current**:
- ✅ Type validation (Rust ExtractedValue)
- ✅ SQL injection prevention
- ❌ No application-level constraint validation (email format, etc.)
- ❌ No sanitization of string inputs

**Remaining Work**:
- [ ] Add optional Pydantic integration for validation
- [ ] Add constraint validation (email, URL, min/max, regex)
- [ ] Add sanitization utilities (HTML escaping, etc.)

**Timeline**: Phase 6 (validation framework)

**Workaround**:
Users should use PostgreSQL constraints:
```python
class User(Table):
    email: str = Column()  # Add CHECK constraint in migration:
    # ALTER TABLE users ADD CONSTRAINT email_format CHECK (email ~ '^[^@]+@[^@]+\.[^@]+$');
```

---

## Summary of Critical Issues

### URGENT (Week 1)

1. 🔴 **Issue #1**: Complete upsert implementation (2-4 hours)
2. 🔴 **Issue #4**: Fix single-row insert performance (2-3 days)

### HIGH PRIORITY (Week 2-3)

3. 🟡 **Issue #2**: Implement auto-migration generation (3-4 days)
4. 🟡 **Issue #3**: Add JOIN-based eager loading (3-4 days)

### MEDIUM PRIORITY (Week 4-6)

5. 🟡 **Gap #1**: Document relationship usage (1 day)
6. 🟡 **Concern #2**: Benchmark migration generation performance
7. 🟡 **Gap #4**: PostgreSQL ORM tutorial (2-3 days)

### LOW PRIORITY (Future)

8. 🟢 All Phase 3c, 3d, and Phase 5 features
9. 🟢 Advanced documentation (API reference, performance tuning)
10. 🟢 Security hardening (TLS, credential rotation)

---

**End of Issues Document**
