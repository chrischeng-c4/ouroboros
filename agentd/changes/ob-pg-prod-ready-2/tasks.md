# Tasks

<meta>
  <purpose>Implementation tickets for Postgres Production Readiness</purpose>
  <constraint>Focus on P0 (Fixes/Safety), P1 (Features), P2 (Opt)</constraint>
</meta>

## 1. P0: Fixes & Safety

- [ ] 1.1 Fix INSERT RETURNING logic
  - File: `crates/ouroboros-postgres/src/query/mod.rs` (MODIFY)
  - Spec: `specs/correctness.md#r1-insert-with-returning`
  - Do: Update execution logic to return `Vec<Row>` when `RETURNING` is present, `u64` otherwise.
  - Depends: none

- [ ] 1.2 Fix DECIMAL Serialization
  - File: `crates/ouroboros-postgres/src/types.rs` (MODIFY)
  - Spec: `specs/correctness.md#r2-decimal-type-serialization`
  - Do: Ensure `rust_decimal` types are correctly converted to strings/floats for Python.
  - Depends: none

- [ ] 1.3 Critical Path Panic Audit
  - File: `crates/ouroboros-postgres/src/connection.rs`, `crates/ouroboros-postgres/src/transaction.rs`, `crates/ouroboros-postgres/src/query/`, `crates/ouroboros-postgres/src/row.rs` (MODIFY)
  - Spec: `specs/safety.md`
  - Do: Audit `unwrap`/`expect` usage in `connection.rs`, `transaction.rs`, `query/`, and `row.rs`. Replace with `?` or explicit error handling.
  - Depends: none

- [ ] 1.4 Connection Resilience
  - File: `crates/ouroboros-postgres/src/connection.rs` (MODIFY)
  - Spec: `specs/robustness.md#r1-connection-retries`
  - Do: Implement exponential backoff in `Connection::new`.
  - Depends: 1.3

- [ ] 1.5 Error Classification
  - File: `crates/ouroboros-postgres/src/lib.rs`, `crates/ouroboros-postgres/src/connection.rs` (MODIFY)
  - Spec: `specs/robustness.md#r3-error-classification`
  - Do: Map PostgreSQL error codes (23505 Unique, 23503 FK, 40P01 Deadlock) to `DataBridgeError` variants (Conflict, ForeignKey, Deadlock).
  - Depends: 1.3

## 2. P1: Essential Features

- [ ] 2.1 Implement `any_` and `has` filters
  - File: `crates/ouroboros-postgres/src/query/builder.rs` (MODIFY)
  - Spec: `specs/features.md`
  - Do: Add support for `Any` and `Has` operators in `WhereClause`.
  - Depends: 1.1

- [ ] 2.2 Transaction Options
  - File: `crates/ouroboros-postgres/src/transaction.rs` (MODIFY)
  - Spec: `specs/features.md`
  - Do: Add `read_only` and `deferrable` parameters to `begin()`.
  - Depends: 1.3

- [ ] 2.3 Advanced Query Support (Deferred/Joins/Subqueries)
  - File: `crates/ouroboros-postgres/src/query/builder.rs` (MODIFY)
  - Spec: `specs/advanced_query.md`
  - Do: Implement `defer()`/`only()` logic and enhance join/subquery generation.
  - Depends: 2.1

- [ ] 2.4 Expose Configuration
  - File: `crates/ouroboros/src/postgres/connection.rs` (MODIFY)
  - Spec: `specs/robustness.md`
  - Do: Expose full pool config options to Python.
  - Depends: 1.4

## 3. P2: Optimization & Observability

- [ ] 3.1 Enable Statement Caching
  - File: `crates/ouroboros-postgres/src/connection.rs` (MODIFY)
  - Spec: `specs/performance.md#r1-statement-caching`
  - Do: Configure `sqlx` to use prepared statement caching.
  - Depends: 1.4

- [ ] 3.2 Transient Error Retries
  - File: `crates/ouroboros-postgres/src/query/mod.rs` (MODIFY)
  - Spec: `specs/performance.md#r2-transient-error-retries`
  - Do: Implement automatic retry for deadlock errors (40P01) with configurable attempts.
  - Depends: 1.5

- [ ] 3.3 Query Tracing Spans
  - File: `crates/ouroboros-postgres/src/query/mod.rs` (MODIFY)
  - Spec: `specs/observability.md#r1-tracing`
  - Do: Add `tracing` spans for all database query executions with SQL statement as attribute.
  - Depends: none

- [ ] 3.4 Error Context Logging
  - File: `crates/ouroboros-postgres/src/query/mod.rs` (MODIFY)
  - Spec: `specs/observability.md#r2-error-logging`
  - Do: Log failed queries with full SQL context and error details.
  - Depends: 3.3

- [ ] 3.5 Add Slow Query Logging
  - File: `crates/ouroboros-postgres/src/query/mod.rs` (MODIFY)
  - Spec: `specs/observability.md`
  - Do: Add tracing event for queries exceeding a configurable threshold.
  - Depends: 3.3

## 4. Verification

- [ ] 4.1 Verify Fixes
  - File: `python/tests/postgres/integration/test_execute_integration.py` (RUN)
  - Verify: `specs/correctness.md`
  - Do: Run existing tests and ensure they pass.
  - Depends: 1.1, 1.2