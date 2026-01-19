# Tasks

<meta>
  <purpose>Implementation tickets derived from specs</purpose>
  <constraint>NO actual code - just file paths, actions, and references</constraint>
</meta>

## 1. Data Layer

- [ ] 1.0 Update Cargo.toml Dependencies
  - File: `crates/argus/Cargo.toml` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r1-stub-downloader`, `specs/watch-mode.md#r1-file-system-monitoring`
  - Do: Add `reqwest` (with blocking feature) and `notify` crates as dependencies.
  - Depends: none

- [ ] 1.1 Implement Stub Cache and Downloader
  - File: `crates/argus/src/types/typeshed.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r1-stub-downloader`
  - Do: Replace hardcoded stubs with a manager that handles local storage and HTTP fetching. Use background thread for downloads to avoid blocking LSP/analysis.
  - Depends: 1.0

- [ ] 1.2 Extend Type and Symbol Data Models
  - File: `crates/argus/src/types/ty.rs` (MODIFY)
  - Spec: `specs/variance-checking.md#r1-typevar-variance-declaration`
  - Do: Add Variance metadata to TypeVar and implement subtyping logic for generic instances.
  - Depends: none

- [ ] 1.2b Parse TypeVar Variance Declarations
  - File: `crates/argus/src/types/infer.rs` (MODIFY)
  - Spec: `specs/variance-checking.md#r1-typevar-variance-declaration`
  - Do: Parse `TypeVar(...)` calls to extract `covariant=True`/`contravariant=True` flags, bounds, and constraints. Register variance info in TypeInferencer.
  - Depends: 1.2

- [ ] 1.3 Add Dependency and Config Support
  - File: `crates/argus/src/core/config.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r5-configuration`
  - Do: Add configuration fields for typeshed path, cache dir, offline mode, and update `Cargo.toml` with `reqwest` and `notify` crates.
  - Depends: none

- [ ] 1.4 Implement Class Metadata Extensions
  - File: `crates/argus/src/types/class_info.rs` (MODIFY)
  - Spec: `specs/generic-inference.md#r4-class-metadata-extensions`
  - Do: Update ClassInfo struct to store generic parameters and variance info.
  - Depends: none

- [ ] 1.5 Extend Python Type Configuration
  - File: `crates/argus/src/types/config.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r5-configuration`
  - Do: Add typeshed settings to Python-specific config and thread through to StubLoader.
  - Depends: 1.3

- [ ] 1.6 Implement Assignment Variance Validation
  - File: `crates/argus/src/types/check.rs` (MODIFY)
  - Spec: `specs/variance-checking.md#r3-assignment-validation`
  - Do: Update assignment checking logic to enforce covariance and contravariance rules.
  - Depends: 1.2

- [ ] 1.7 Validate Variance Usage in Declarations
  - File: `crates/argus/src/types/check.rs` (MODIFY)
  - Spec: `specs/variance-checking.md#acceptance-criteria`
  - Do: Implement validation to ensure covariant/contravariant TypeVars are used in valid positions (e.g., return types vs arguments).
  - Depends: 1.2

- [ ] 1.8 Implement Stub Precedence and Resolution
  - File: `crates/argus/src/types/stubs.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r4-conflict-resolution`
  - Do: Implement logic to resolve stub conflicts based on configuration (Local > Typeshed > Bundled) during import resolution.
  - Depends: 1.5

- [ ] 1.9 Implement Versioned Cache Management
  - File: `crates/argus/src/types/typeshed.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r2-local-cache-management`
  - Do: Structure cache keys to include Python target version and implement cache invalidation.
  - Depends: 1.1

- [ ] 1.10 Implement Deferred Analysis for Unused Imports
  - File: `crates/argus/src/types/check.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r3-on-demand-loading`
  - Do: Update type checker to skip full analysis of imported modules until a symbol is accessed.
  - Depends: 1.5

- [ ] 1.11 Implement Typeshed Index and TTL
  - File: `crates/argus/src/types/typeshed.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r6-typeshed-source-and-resolution`
  - Do: Download typeshed tree index on first use, implement cache TTL refresh, and support commit pin configuration.
  - Depends: 1.1, 1.5

- [ ] 1.12 Add Stub Precedence Configuration
  - File: `crates/argus/src/types/config.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r5-configuration`
  - Do: Add `stub_precedence` field to config and wire through to StubLoader conflict resolution.
  - Depends: 1.5, 1.8

- [ ] 1.13 Add Typeshed TTL and Commit Pin Configuration
  - File: `crates/argus/src/types/config.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r6-typeshed-source-and-resolution`
  - Do: Add `typeshed_ttl_days` (default: 7) and `typeshed_commit` (optional pin) config fields.
  - Depends: 1.5

- [ ] 1.14 Implement Offline Fallback to Any with Warning
  - File: `crates/argus/src/types/stubs.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#acceptance-criteria`
  - Do: When offline and no stub exists (cache, bundled), fall back to `Any` and emit a warning diagnostic.
  - Depends: 1.8

## 2. Logic Layer

- [ ] 2.1 Implement Parser Synchronization
  - File: `crates/argus/src/syntax/parser.rs` (MODIFY)
  - Spec: `specs/error-recovery.md#r1-parser-synchronization`
  - Do: Add logic to skip to the next statement or block when tree-sitter reports an ERROR node.
  - Depends: none

- [ ] 2.2 Enhance Semantic Analysis for Error Recovery
  - File: `crates/argus/src/semantic/mod.rs` (MODIFY)
  - Spec: `specs/error-recovery.md#r2-partial-symbol-table-construction`
  - Do: Ensure symbol table construction continues past syntax errors and uses placeholder types.
  - Depends: 2.1

- [ ] 2.2b Implement Placeholder Types for Error Recovery
  - File: `crates/argus/src/types/infer.rs` (MODIFY)
  - Spec: `specs/error-recovery.md#r3-placeholder-types`
  - Do: Assign `ErrorType` or `Any` to unresolved expressions in error contexts to prevent cascading type errors.
  - Depends: 2.2

- [ ] 2.3 Implement Generic Inference Engine
  - File: `crates/argus/src/types/infer.rs` (MODIFY)
  - Spec: `specs/generic-inference.md#r1-constructor-based-inference`
  - Do: Implement TypeVar constraint solving, including nested generic inference and bounds enforcement. Default to `Any` when inference fails and no defaults are provided.
  - Depends: 1.4

- [ ] 2.4 Implement File System Watcher
  - File: `crates/argus/src/lib.rs` (MODIFY)
  - Spec: `specs/watch-mode.md#r1-file-system-monitoring`
  - Do: Add background thread using `notify` crate to watch for file events and trigger debounced re-analysis.
  - Depends: none

- [ ] 2.5 Implement Workspace Index
  - File: `crates/argus/src/lsp/workspace.rs` (CREATE)
  - Spec: `specs/advanced-lsp.md#r4-workspace-indexing`
  - Do: Create a global index structure with fully-qualified symbol IDs (module_path + scope_path + name). Integrate with ModuleGraph for import resolution. Track file URIs alongside ranges.
  - Depends: 2.2

- [ ] 2.5b Implement Open-Document Precedence
  - File: `crates/argus/src/lsp/workspace.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r4-workspace-indexing`
  - Do: LSP in-memory text takes precedence for open documents. Index updates from didChange/didSave for open files; watcher events only affect non-open files.
  - Depends: 2.5

- [ ] 2.6 Extend Module Dependency Graph
  - File: `crates/argus/src/types/modules.rs` (MODIFY)
  - Spec: `specs/watch-mode.md#r2-incremental-re-analysis`
  - Do: Expose public API for reverse-dependency lookup to support incremental invalidation.
  - Depends: none

- [ ] 2.6b Integrate Dependency Lookup into Watch-Mode Re-analysis
  - File: `crates/argus/src/lib.rs` (MODIFY)
  - Spec: `specs/watch-mode.md#r2-incremental-re-analysis`
  - Do: When a file changes, use reverse-dependency lookup to schedule re-analysis of dependent files.
  - Depends: 2.4, 2.6

- [ ] 2.7 Implement Initial Workspace Scan
  - File: `crates/argus/src/lsp/server.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r4-workspace-indexing`
  - Do: Perform a background scan of all files in the root directory on initialization to populate the workspace index.
  - Depends: 2.5

- [ ] 2.8 Hook Workspace Index Updates
  - File: `crates/argus/src/lsp/server.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r4-workspace-indexing`
  - Do: Update the workspace index on `didChange`/`didSave` notifications and file watcher events.
  - Depends: 2.5, 2.4

- [ ] 2.9 Handle File Deletion and Rename in Index
  - File: `crates/argus/src/lsp/server.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#acceptance-criteria`
  - Do: Implement index cleanup on file deletion and URI updates on rename events.
  - Depends: 2.5, 2.4

## 3. Integration

- [ ] 3.0 Wire New Modules into Crate
  - File: `crates/argus/src/lsp/mod.rs` (MODIFY), `crates/argus/src/semantic/mod.rs` (MODIFY)
  - Spec: N/A (build requirement)
  - Do: Add `mod workspace;`, `mod code_actions;`, `#[cfg(test)] mod tests;` declarations to lsp/mod.rs and `#[cfg(test)] mod tests;` to semantic/mod.rs.
  - Depends: 2.5, 3.2, 4.2, 4.4

- [ ] 3.1 Implement Advanced LSP Request Handlers
  - File: `crates/argus/src/lsp/server.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r1-global-rename`
  - Do: Implement rename, find references, and code action handlers, integrating the new Workspace Index.
  - Depends: 2.5

- [ ] 3.2 Implement Extensible Code Action Registry
  - File: `crates/argus/src/lsp/code_actions.rs` (CREATE)
  - Spec: `specs/advanced-lsp.md#r3-extensible-code-actions`
  - Do: Create a registry for code actions and implement "Add Type Hint" action.
  - Depends: 3.1

## 4. Testing

- [ ] 4.1 Integration Tests for Parity Features
  - File: `crates/argus/src/types/check_tests.rs` (MODIFY)
  - Verify: `specs/variance-checking.md#acceptance-criteria`, `specs/generic-inference.md#acceptance-criteria`
  - Depends: 2.3

- [ ] 4.2 LSP End-to-End Tests
  - File: `crates/argus/src/lsp/tests.rs` (CREATE)
  - Verify: `specs/advanced-lsp.md#acceptance-criteria`
  - Depends: 3.1

- [ ] 4.3 Watch Mode and Offline Tests
  - File: `crates/argus/src/types/stubs.rs` (MODIFY)
  - Verify: `specs/dynamic-typeshed.md#acceptance-criteria`, `specs/watch-mode.md#acceptance-criteria`
  - Do: Add unit tests for offline fallback and local override; add integration test for watch mode debounce.
  - Depends: 3.2, 2.4

- [ ] 4.4 Error Recovery Tests
  - File: `crates/argus/src/semantic/tests.rs` (CREATE)
  - Verify: `specs/error-recovery.md#acceptance-criteria`
  - Do: Add tests for partial symbol table construction and parser recovery.
  - Depends: 2.2