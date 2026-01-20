# Tasks

<meta>
  <purpose>Implementation tickets derived from specs</purpose>
  <constraint>NO actual code - just file paths, actions, and references</constraint>
</meta>

## 1. Data Layer

- [ ] 1.1 Update test environment documentation
  - File: `crates/ouroboros-postgres/README.md` (MODIFY)
  - Spec: `specs/obpg-transaction-tests.md#overview`
  - Do: Add detailed instructions for local PostgreSQL setup using `brew services` and environment variables to the Development section.
  - Depends: none

## 2. Logic Layer

- [ ] 2.1 Refactor existing transaction tests
  - File: `crates/ouroboros-postgres/tests/test_transaction.rs` (MODIFY)
  - Spec: `specs/obpg-transaction-tests.md#R1`
  - Do: Remove `#[ignore]` from existing tests and ensure they use a consistent connection setup and cleanup.
  - Depends: none

## 3. Integration

- [ ] 3.1 Implement Isolation Level verification
  - File: `crates/ouroboros-postgres/tests/test_transaction.rs` (MODIFY)
  - Spec: `specs/obpg-transaction-tests.md#R2`
  - Do: Implement tests for all 4 PostgreSQL isolation levels and verify them using `SHOW transaction_isolation` within the transaction context.
  - Depends: 2.1

- [ ] 3.2 Implement Transaction Options support verification
  - File: `crates/ouroboros-postgres/tests/test_transaction.rs` (MODIFY)
  - Spec: `specs/obpg-transaction-tests.md#R5`
  - Do: Verify `AccessMode::ReadOnly` enforcement (blocking INSERTs) and ensure the `DEFERRABLE` flag can be set for serializable transactions.
  - Depends: 3.1

## 4. Testing

- [ ] 4.1 Implement Savepoint and Nested Simulation tests
  - File: `crates/ouroboros-postgres/tests/test_transaction.rs` (MODIFY)
  - Spec: `specs/obpg-transaction-tests.md#R3`, `specs/obpg-transaction-tests.md#R6`
  - Verify: `specs/obpg-transaction-tests.md#acceptance-criteria`
  - Do: Implement named savepoints and verify partial rollbacks and nested simulation scenarios (savepoint stack).
  - Depends: 2.1

- [ ] 4.2 Implement Auto-Rollback on Drop tests
  - File: `crates/ouroboros-postgres/tests/test_transaction.rs` (MODIFY)
  - Spec: `specs/obpg-transaction-tests.md#R4`
  - Verify: `specs/obpg-transaction-tests.md#acceptance-criteria`
  - Do: Verify that a `Transaction` object dropped without an explicit commit or rollback triggers a database-level rollback.
  - Depends: 2.1

- [ ] 4.3 Full Transaction Integration Verification
  - File: `crates/ouroboros-postgres/tests/test_transaction.rs` (MODIFY)
  - Verify: `specs/obpg-transaction-tests.md#acceptance-criteria`
  - Do: Run the complete suite of transaction tests and ensure all scenarios (Happy Path, Savepoints, Rollbacks, Isolation Levels, Read-Only mode) pass successfully against a live database.
  - Depends: 3.2, 4.1, 4.2