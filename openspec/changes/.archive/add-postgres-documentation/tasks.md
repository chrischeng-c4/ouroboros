# Implementation Tasks

## 1. Restructure & Migration
- [x] 1.1 Create directory `docs/postgres/guides/` ✅
- [x] 1.2 Move and rename `docs/archive/postgres_inheritance.md` -> `docs/postgres/guides/inheritance.md` ✅
- [x] 1.3 Move and rename `docs/archive/postgres_migrations.md` -> `docs/postgres/guides/migrations.md` ✅
- [x] 1.4 Move and rename `docs/archive/postgres_transactions.md` -> `docs/postgres/guides/transactions.md` ✅
- [x] 1.5 Move and rename `docs/archive/postgres_raw_sql.md` -> `docs/postgres/guides/raw_sql.md` ✅
- [x] 1.6 Verify content of moved files and update internal links if broken. ✅

## 2. Quickstart Guide (`docs/postgres/quickstart.md`)
- [x] 2.1 Create "Getting Started" section with Installation (`pip install data-bridge-postgres`). ✅
- [x] 2.2 Add "Connecting" section (Async `init` with connection string). ✅
- [x] 2.3 Add "Defining Models" section (Table definition, Columns). ✅
- [x] 2.4 Add "Basic CRUD" section (Insert, Select, Update, Delete). ✅

## 3. Deep Dive Guides
- [x] 3.1 Create `docs/postgres/guides/tables_and_columns.md`: Cover types, constraints, defaults, computed columns. ✅
- [x] 3.2 Create `docs/postgres/guides/querying.md`: Cover `QueryBuilder`, filtering (`and_`, `or_`), joins, eager loading strategies. ✅
- [x] 3.3 Create `docs/postgres/guides/validation.md`: Cover `@validates`, `@validates_many`, and builtin validators. ✅
- [x] 3.4 Create `docs/postgres/guides/events.md`: Cover signal handlers (`before_insert`, etc.) and event dispatcher. ✅

## 4. API Reference (`docs/postgres/api.md`)
- [x] 4.1 Document **Models & Fields**: `Table`, `Column`, `ForeignKeyProxy`, `BackReference`, `ManyToMany`. ✅
- [x] 4.2 Document **Relationships**: `relationship`, `loading strategies` (selectinload, etc). ✅
- [x] 4.3 Document **Querying**: `QueryBuilder`, operators (`filter_by`, `and_`, etc), `QueryOption`. ✅
- [x] 4.4 Document **Connection & Session**: `init`, `Session`, `AsyncSession`, `execute`. ✅
- [x] 4.5 Document **CRUD & Utils**: `insert_one`, `upsert`, `delete_checked`, `pg_transaction`. ✅
- [x] 4.6 Document **Events & Telemetry**: Event decorators, tracing functions. ✅
- [x] 4.7 Document **Validation**: Validators and decorators. ✅

## 5. Integration
- [x] 5.1 Update `mkdocs.yml` to include the new `PostgreSQL` section with the substructure (Quickstart, Guides, API). ✅
