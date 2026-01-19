# Tasks

<meta>
  <purpose>Implementation tickets derived from specs</purpose>
  <constraint>NO actual code - just file paths, actions, and references</constraint>
</meta>

## 1. Data Layer

- [x] 1.1 Implement `SemanticModel` struct
  - File: `crates/argus/src/types/model.rs` (CREATE)
  - Spec: `specs/argus-daemon.md#data-model`
  - Do: Define `SemanticModel`, `SymbolData`, and `TypeInfo` (owned versions) structures. Implement serialization/deserialization.
  - Depends: none

- [x] 1.2 Add `invalidate` method to `DaemonClient`
  - File: `crates/argus/src/server/daemon.rs` (MODIFY)
  - Spec: `specs/argus-daemon.md#interfaces`
  - Do: Implement convenience method for the client to send invalidate request. Note: `shutdown` method already exists.
  - Depends: none

## 2. Logic Layer

- [x] 2.1 Enhance `TypeChecker` to produce `SemanticModel`
  - File: `crates/argus/src/types/check.rs` (MODIFY)
  - Spec: `specs/argus-daemon.md#r8-deep-type-analysis`
  - Do: Update `TypeChecker` to populate and return a `SemanticModel` containing all resolved types, definitions, and references mapped to source ranges.
  - Depends: 1.1

- [x] 2.2 Implement Async FileWatcher Bridge
  - File: `crates/argus/src/server/watch_bridge.rs` (CREATE)
  - Spec: `specs/argus-daemon.md#r9-async-event-bridging`
  - Do: Create a mechanism to spawn the sync `FileWatcher` in a blocking thread and forward events to a Tokio mpsc channel.
  - Depends: none

- [x] 2.3 Implement Background Analysis Loop in `ArgusDaemon`
  - File: `crates/argus/src/server/daemon.rs` (MODIFY)
  - Spec: `specs/argus-daemon.md#r6-incremental-background-updates`
  - Do:
    1. Consume events from the Watch Bridge.
    2. Maintain a debounced queue of files to analyze.
    3. Run `TypeChecker` in a blocking task for queued files.
    4. Update the `RequestHandler` cache with the new `SemanticModel`.
  - Depends: 2.1, 2.2

- [x] 2.4 Refactor `RequestHandler` to use `SemanticModel`
  - File: `crates/argus/src/server/handler.rs` (MODIFY)
  - Spec: `specs/argus-daemon.md#interfaces`
  - Do: Update `handle_request` to query the cached `SemanticModel` for `type_at`, `hover`, `definition`, and `references` instead of re-running analysis (unless missing). Use LSP-compatible response formats where applicable. Maintain backward compatibility by falling back to `SymbolTable` when `SemanticModel` is not available.
  - Depends: 2.3

- [x] 2.5 Integrate `TypeChecker` into `PythonChecker` (Linting)
  - File: `crates/argus/src/lint/python.rs` (MODIFY)
  - Spec: `specs/argus-daemon.md#r8-deep-type-analysis`
  - Do: Update `PythonChecker::check` to run `TypeChecker` and collect its diagnostics (TCxxx rules).
  - Depends: 2.1

- [x] 2.6 Enhance `McpServer` with full tool coverage
  - File: `crates/argus/src/mcp/server.rs` (MODIFY)
  - Spec: `specs/argus-mcp.md#r3-tool-exposure`
  - Do: Ensure all tools (`argus_check`, `argus_type_at`, `argus_hover`, `argus_definition`, `argus_references`, `argus_index_status`, `argus_invalidate`, etc.) are correctly mapped to daemon calls. Adapt response formats as needed for MCP compatibility.
  - Depends: 1.2

## 3. Integration

- [x] 3.1 Wire up MCP commands in CLI
  - File: `crates/ouroboros-cli/src/main.rs` (MODIFY)
  - Spec: `specs/argus-mcp.md#r4-configuration-generation`
  - Do: Ensure `ArgusAction::Mcp` and `ArgusAction::McpServer` are correctly implemented.
  - Depends: 2.6

## 4. Testing

- [x] 4.1 Verify Semantic Model Accuracy
  - File: `crates/argus/src/server/tests.rs` (CREATE)
  - Verify: `specs/argus-daemon.md#r8-deep-type-analysis`
  - Do: Unit tests for `TypeChecker` producing correct `SemanticModel` (correct types at ranges).
  - Depends: 2.1

- [x] 4.2 Verify Background Re-analysis
  - File: `crates/argus/src/server/tests.rs` (CREATE)
  - Verify: `specs/argus-daemon.md#acceptance-criteria`
  - Do: Integration test that starts daemon, modifies a file, and asserts `type_at` returns updated result without explicit client request.
  - Depends: 2.3

- [x] 4.3 Verify Performance
  - File: `crates/argus/benches/daemon_bench.rs` (CREATE)
  - Verify: `specs/argus-daemon.md#acceptance-criteria`
  - Do: Create a criterion benchmark for `type_at` requests against a running daemon to ensure sub-5ms latency.
  - Depends: 2.4
