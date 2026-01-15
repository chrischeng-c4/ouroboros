## 1. Refactor & Rename
- [x] 1.1 Rename crate `crates/ouroboros-test` to `crates/ouroboros-qc`.
- [x] 1.2 Update `Cargo.toml` members and dependencies in other crates to point to `ouroboros-qc`.
- [x] 1.3 Rename Python module usages from `ouroboros.test` to `ouroboros.qc`.

## 2. CLI Implementation
- [ ] 2.1 Create new crate `crates/ouroboros-cli` with `clap`.
- [ ] 2.2 Implement main entry point `ob`.
- [ ] 2.3 Implement `ob qc` subcommand structure.
- [ ] 2.4 Wire `ob qc run` to the test runner logic.
- [ ] 2.5 Wire `ob qc collect` to the discovery logic.

## 3. QC Features
- [ ] 3.1 Implement automatic `TestSuite` discovery (scan classes inheriting from TestSuite).
- [ ] 3.2 Fix async hook execution (`setup_method`, `teardown_method`) to ensure DB connections.
- [ ] 3.3 Implement `-k` pattern filtering logic in the runner.

## 4. Integration
- [ ] 4.1 Expose `ouroboros_cli` as a binary or Python entry point.
- [ ] 4.2 Verify `ob qc run` executes existing tests correctly.
