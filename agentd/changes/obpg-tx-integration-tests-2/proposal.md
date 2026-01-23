---
change_id: obpg-tx-integration-tests-2
type: proposal
---

<proposal>
# Change: obpg-tx-integration-tests-2

## Summary

Add comprehensive transaction integration tests for the `ouroboros-postgres` crate, covering all isolation levels, savepoints, nested rollbacks, and auto-rollback on drop.

## Why

Transaction management is a critical component of the database adapter. While some basic tests exist in the codebase, they are currently ignored and lack full coverage of PostgreSQL-specific features like isolation levels and complex savepoint scenarios. Ensuring these work correctly is essential for maintaining ACID properties and reliability in the Ouroboros PostgreSQL implementation. This addresses issue #72.

## What Changes

- Enable and update existing transaction integration tests in `crates/ouroboros-postgres/tests/test_transaction.rs`.
- Add new test cases for all PostgreSQL isolation levels: `READ UNCOMMITTED`, `READ COMMITTED`, `REPEATABLE READ`, and `SERIALIZABLE`.
- Implement detailed tests for savepoints and "nested transaction" behavior (savepoint-based nesting) to verify partial rollback propagation.
- Verify auto-rollback on drop behavior for incomplete transactions to ensure connection health.
- Add tests for `AccessMode` (READ ONLY vs READ WRITE) and `DEFERRABLE` transaction options.
- Document and provide guidance for local PostgreSQL test environment setup (e.g., via `brew services`).

## Impact

- Affected specs: `obpg-transaction-tests`
- Affected code:
  - `crates/ouroboros-postgres/README.md`
  - `crates/ouroboros-postgres/tests/test_transaction.rs`
- Breaking changes: No
</proposal>
