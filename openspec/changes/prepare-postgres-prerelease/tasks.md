# Tasks: Prepare Postgres Crate for 0.1.0-alpha

## 1. Documentation & API Polish
- [x] ✅ 1.1 Update `crates/data-bridge-postgres/README.md`:
    - [x] ✅ Remove "IN PROGRESS" status.
    - [x] ✅ Mark Migrations, Introspection, and Joins as implemented.
    - [x] ✅ Add "Installation" and "Quick Start" sections.
- [x] ✅ 1.2 Add Rustdoc comments (`//!` and `///`) to:
    - [x] ✅ `src/lib.rs` (Crate level docs)
    - [x] ✅ `src/query.rs` (Already mostly done, verify coverage)
    - [x] ✅ `src/schema.rs` (Public structs `TableInfo`, `ColumnInfo`, etc.)
    - [x] ✅ `src/migration.rs` (`Migration`, `MigrationRunner`)
    - [x] ✅ `src/transaction.rs`
- [x] ✅ 1.3 Review public API surface:
    - [x] ✅ Ensure `data-bridge-common` error types are used consistently.
    - [x] ✅ Verify `Clone`/`Debug` implementations on public structs.

## 2. Testing & Benchmarks
- [x] ✅ 2.1 Implement Criterion benchmarks:
    - [x] ✅ Create `crates/data-bridge-postgres/benches/main.rs`.
    - [x] ✅ Benchmark: Bulk Insert (1k, 10k rows).
    - [x] ✅ Benchmark: Complex Query (Join + Filter).
    - [x] ✅ Benchmark: Serialization overhead.
- [x] ✅ 2.2 Add Migration Integration Tests:
    - [x] ✅ Test `MigrationRunner::apply` and `revert` against a real Postgres container.
    - [x] ✅ Verify checksum validation logic.
- [x] ✅ 2.3 Add Schema Introspection Tests:
    - [x] ✅ Create complex tables (enums, arrays, foreign keys).
    - [x] ✅ Assert `SchemaInspector` returns correct metadata.

## 3. Release Prep
- [x] ✅ 3.1 Update `crates/data-bridge-postgres/Cargo.toml`:
    - [x] ✅ Set version to `0.1.0-alpha.1` (or consistent workspace version).
    - [x] ✅ Ensure all dependencies use workspace versions or fixed versions.
    - [x] ✅ Check `exclude` list for packaging (exclude test assets if large).
- [x] ✅ 3.2 Verify CI:
    - [x] ✅ Run `cargo clippy -p data-bridge-postgres -- -D warnings`.
    - [x] ✅ Run `cargo doc -p data-bridge-postgres --no-deps`.
