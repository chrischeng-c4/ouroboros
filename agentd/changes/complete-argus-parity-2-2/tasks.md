# Tasks

<meta>
  <purpose>Implementation tickets derived from specs</purpose>
  <constraint>NO actual code - just file paths, actions, and references</constraint>
</meta>

## 1. Data Layer & Config

- [ ] 1.1 Reference Index Model
  - File: `crates/argus/src/semantic/index.rs` (CREATE)
  - Spec: `specs/advanced-lsp.md#data-model`
  - Do: Define the `ReferenceIndex` structure to map `SymbolId` to a list of `Location` (usage sites).
  - Depends: none

- [ ] 1.2 Module Wiring for Reference Index
  - File: `crates/argus/src/semantic/mod.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r1`
  - Do: Add `pub mod index;` and export `ReferenceIndex`.
  - Depends: 1.1

- [ ] 1.3 Unified Project Configuration
  - File: `crates/argus/src/core/config.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r4`
  - Do: Add `TypeshedConfig` fields (cache_dir, refresh_interval) to the main `ProjectConfig`.
  - Depends: none

- [ ] 1.4 Thread Config to Analyzers
  - File: `crates/argus/src/types/project.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r4`
  - Do: Update `ProjectAnalyzer` to consume `TypeshedConfig` from the unified `ProjectConfig` and pass it to `StubLoader`.
  - Depends: 1.3

- [ ] 1.5 Module Wiring for Parity Tests
  - File: `crates/argus/src/types/mod.rs` (MODIFY)
  - Spec: `specs/core-type-system.md`
  - Do: Add `#[cfg(test)] mod parity_tests;` to enable the new test module.
  - Depends: none

## 2. Logic Layer: Core Type System & Daemon

- [ ] 2.1 PEP Parity: Protocols & TypedDict
  - File: `crates/argus/src/types/check.rs` (MODIFY)
  - Spec: `specs/core-type-system.md#r1`
  - Do: Implement structural subtyping for Protocols and key-based validation for TypedDict.
  - Depends: none

- [ ] 2.2 PEP Parity: Variadic Generics & ParamSpec
  - File: `crates/argus/src/types/ty.rs` (MODIFY)
  - Spec: `specs/core-type-system.md#r4`
  - Do: Extend the `Type` enum to support `TypeVarTuple` and `ParamSpec`.
  - Depends: none

- [ ] 2.3 Reference Collection Pass
  - File: `crates/argus/src/semantic/symbols.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r1`
  - Do: Extend symbol resolution to record usage sites in the `ReferenceIndex` during semantic analysis.
  - Depends: 1.1

- [ ] 2.4 Daemon-Owned File Watcher
  - File: `crates/argus/src/server/daemon.rs` (MODIFY)
  - Spec: `specs/watch-mode.md#r1`
  - Do: Integrate the `notify` crate to monitor workspace changes and trigger `RequestHandler::on_file_change`.
  - Depends: none

- [ ] 2.5 Dynamic Stub Loader with ETag
  - File: `crates/argus/src/types/typeshed.rs` (MODIFY)
  - Spec: `specs/dynamic-typeshed.md#r2`
  - Do: Implement the HTTP downloader using `reqwest` with ETag validation and persistent caching.
  - Depends: 1.3

- [ ] 2.6 Parser Recovery Sync Points
  - File: `crates/argus/src/syntax/parser.rs` (MODIFY)
  - Spec: `specs/error-recovery.md#r1`
  - Do: Ensure `walk_with_recovery` and `synchronize_after` handle nested block synchronization correctly per requirements.
  - Depends: none

## 3. Integration: MCP & LSP

- [ ] 3.1 MCP Server: Handler & Stdio IPC
  - File: `crates/argus/src/mcp/server.rs` (MODIFY)
  - Spec: `specs/mcp-server.md#r2`
  - Do: Wire the stdio request loop to the `McpServer::handle_request` which calls the Daemon via Unix socket.
  - Depends: 2.4

- [ ] 3.2 Workspace Edit Logic
  - File: `crates/argus/src/lsp/workspace.rs` (CREATE)
  - Spec: `specs/advanced-lsp.md#r2`
  - Do: Implement `Workspace` logic to handle rename calculations and build `WorkspaceEdit` from `ReferenceIndex`.
  - Depends: 1.1

- [ ] 3.3 Workspace-wide Rename Integration
  - File: `crates/argus/src/lsp/server.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#r2`
  - Do: Update `ArgusServer` to use `workspace.rs` logic for `textDocument/rename` requests.
  - Depends: 3.2, 2.3

- [ ] 3.4 New LSP Module Declarations
  - File: `crates/argus/src/lsp/mod.rs` (MODIFY)
  - Spec: `specs/advanced-lsp.md#overview`
  - Do: Add `pub mod code_actions;`, `pub mod workspace;` and `#[cfg(test)] mod tests;`.
  - Depends: none

- [ ] 3.5 CLI Watch Command
  - File: `crates/ouroboros-cli/src/main.rs` (MODIFY)
  - Spec: `specs/watch-mode.md#r4`
  - Do: Add `watch` subcommand that establishes a long-running connection to the daemon and prints live diagnostics.
  - Depends: 2.4

- [ ] 3.6 LSP Code Actions
  - File: `crates/argus/src/lsp/code_actions.rs` (CREATE)
  - Spec: `specs/advanced-lsp.md#r5`
  - Do: Implement code action handlers for quick fixes (e.g., import missing symbol).
  - Depends: none

## 4. Testing

- [ ] 4.1 Type Parity Unit Tests
  - File: `crates/argus/src/types/parity_tests.rs` (CREATE)
  - Verify: `specs/core-type-system.md#acceptance-criteria`
  - Do: Add exhaustive tests for PEP-specific type scenarios (Protocols, Generics).
  - Depends: 1.5, 2.1, 2.2

- [ ] 4.2 Semantic Test Module Wiring
  - File: `crates/argus/src/semantic/mod.rs` (MODIFY)
  - Verify: `specs/advanced-lsp.md#acceptance-criteria`
  - Do: Add `#[cfg(test)] mod tests;` to enable semantic tests (if not already present/active).
  - Depends: 2.3

- [ ] 4.3 MCP Integration Tests
  - File: `crates/argus/src/mcp/tests.rs` (CREATE)
  - Verify: `specs/mcp-server.md#acceptance-criteria`
  - Do: Mock the Daemon and test the MCP tool call mapping.
  - Depends: 3.1

- [ ] 4.4 Reference Index Unit Tests
  - File: `crates/argus/src/semantic/tests.rs` (MODIFY)
  - Verify: `specs/advanced-lsp.md#acceptance-criteria`
  - Do: Add tests for `ReferenceIndex` collection and incremental updates to the existing semantic tests.
  - Depends: 2.3

- [ ] 4.5 LSP Unit Tests
  - File: `crates/argus/src/lsp/tests.rs` (CREATE)
  - Verify: `specs/advanced-lsp.md#acceptance-criteria`
  - Do: Add unit tests for `code_actions` and `rename` logic.
  - Depends: 3.3, 3.6