# Change: Prepare Postgres Crate for 0.1.0-alpha

## Why
The `data-bridge-postgres` crate has reached feature parity with the initial requirements (including Migration, Schema Introspection, and Advanced Queries) but is still marked as "IN PROGRESS". To enable downstream usage and release, we need to stabilize the API, complete documentation, and verify performance/correctness.

## What Changes
- **Documentation**:
    - Update `README.md` to reflect current feature set (remove outdated TODOs).
    - Add Rustdoc comments to all public modules (`lib.rs`, `query`, `transaction`, etc.).
    - Add a `GUIDE.md` or expanded examples in `README.md` showing complex usage (joins, upserts).
- **API Stabilization**:
    - Final review of `query.rs` and `schema.rs` public interfaces.
    - Ensure all public types implement `Debug`, `Clone` (where appropriate), and `Serialize`/`Deserialize` if needed.
- **Testing**:
    - Add a proper Criterion benchmark suite in `benches/` (replacing or augmenting `tests/test_benchmark.rs`).
    - Add integration tests for the Migration system (`migration.rs`).
    - Add integration tests for Schema Introspection (`schema.rs`).
- **CI/CD**:
    - Verify crate is included in workspace `cargo test` and `cargo doc`.
    - Ensure `package.metadata` in `Cargo.toml` is ready for publishing.

## Impact
- **Affected Specs**: `postgres-orm`
- **Affected Code**: `crates/data-bridge-postgres/`
