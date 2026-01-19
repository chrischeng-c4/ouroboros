# Change: ob-pg-prod-ready-2

## Summary

Production readiness roadmap for `ouroboros-postgres` focusing on fixing critical test failures, auditing panic safety, implementing missing query features, and enhancing connection robustness.

## Why

To support small/secondary project production deployments, the Postgres ORM must be reliable and feature-complete. Currently, there are:
- 2 known integration test failures (RETURNING clause, DECIMAL types).
- Significant panic risks (506 unwrap calls).
- Missing query filters (`any_`, `has`) and transaction options required by the application layer.
- Unimplemented query strategies (Deferred loading, Subqueries) that limit ORM capabilities.
- Lack of robust connection handling for production environments.

## What Changes

### P0: Critical Fixes & Safety (High Priority)
- **Correctness**: Fix `test_execute_insert_with_returning` (ensure INSERT with RETURNING returns rows, not count).
- **Type Support**: Fix `test_execute_aggregate_query` (proper handling of DECIMAL/NUMERIC serialization).
- **Safety Audit**: Audit and replace `unwrap()`/`expect()` with proper error handling in critical paths (CRUD, connection, transaction).
- **Connection Resilience**: Implement exponential backoff retries and map specific Postgres errors (Conflict, Transient).

### P1: Essential Features (Medium Priority)
- **Query Filters**: Implement `any_()` (array containment) and `has()` (JSON/Map existence) filters.
- **Transactions**: Add support for `read_only` and `deferrable` transaction options.
- **Advanced Query Support**: Implement deferred column loading, join strategies, and subquery support.
- **Documentation**: Create basic operational docs (configuration, monitoring).

### P2: Optimization (Low Priority)
- **Performance**: Enable prepared statement caching.
- **Observability**: Implement slow query logging.

## Impact

- Affected specs:
    - `specs/correctness.md` (New)
    - `specs/safety.md` (New)
    - `specs/features.md` (New)
    - `specs/advanced_query.md` (New)
    - `specs/robustness.md` (Modified)
    - `specs/performance.md` (Modified)
    - `specs/observability.md` (Modified)
- Affected code:
    - `crates/ouroboros-postgres/src/query/`
    - `crates/ouroboros-postgres/src/types.rs`
    - `crates/ouroboros-postgres/src/connection.rs`
    - `python/tests/postgres/integration/`
- Breaking changes: No public API breaking changes, but internal behavior for INSERT/RETURNING will change to match expectations.