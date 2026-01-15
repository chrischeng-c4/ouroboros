# Change: Unified CLI and Quality Control

## Why
The current tooling is fragmented (`dbtest`, `ouroboros-test`), leading to a disjointed developer experience. Renaming `ouroboros-test` to `ouroboros-qc` (Quality Control) clarifies its scope beyond just "testing" to include benchmarking, security checks, and linting, while avoiding naming conflicts with standard `test` modules. A unified `ob` CLI will serve as the single entry point for all developer tasks.

## What Changes
- **New Crate**: `ouroboros-cli` providing the `ob` command.
- **Rename**: `ouroboros-test` crate becomes `ouroboros-qc`.
- **New Capability**: `cli-core` for managing CLI commands and subcommands.
- **Enhanced Capability**: `quality-control` (replaces `test-framework`) with:
    - `ob qc run` command structure.
    - Automatic `TestSuite` subclass discovery (no `if __name__ == "__main__"` needed).
    - Fixed async `setup_method`/`teardown_method` hooks.
    - Pattern filtering via `-k`.

## Impact
- **Affected Specs**: `cli-core` (new), `quality-control` (new), `test-framework` (removed).
- **Affected Code**:
    - `crates/ouroboros-cli` (new)
    - `crates/ouroboros-test` -> `crates/ouroboros-qc` (renamed & modified)
    - `python/ouroboros` (updated bindings)
