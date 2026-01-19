# Challenge Report: complete-argus-parity-2-2

## Summary
The proposal is ambitious but not internally consistent with the tasks/specs. Several core requirements (full PEP parity, daemon IPC/session management, global references, and MCP toolset scope) are not represented in tasks or are mismatched with existing code, so it is not ready for implementation.

## Internal Consistency Issues
[Check if proposal files are consistent with each other]
[Examples: Does proposal.md match tasks.md? Do Mermaid diagrams in specs/ match descriptions? Do task spec refs exist?]
[These are HIGH priority - must fix before implementation]

### Issue: 100% parity claim not reflected in tasks
- **Severity**: High
- **Category**: Completeness
- **Description**: The proposal and core-type-system spec require broad PEP support (484, 526, 544, 585, 586, 589, 591, 593, 604, 612, 613, 646, 647, 655, 673, 742), but tasks only cover Protocol/TypedDict and ParamSpec/TypeVarTuple. The remaining PEP features (e.g., Literals, Annotated, TypeAlias/TypeAliasType, TypeIs, TypeGuard behavior, ParamSpec/Concatenate semantics, variance rules, etc.) are not represented.
- **Location**: `agentd/changes/complete-argus-parity-2-2/proposal.md`, `agentd/changes/complete-argus-parity-2-2/specs/core-type-system.md`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Recommendation**: Expand tasks to cover the full PEP list or narrow the proposal/spec to the subset actually planned.

### Issue: Daemon-mode requirements missing from tasks
- **Severity**: High
- **Category**: Completeness
- **Description**: The daemon spec requires a persistent in-memory index, JSON-RPC over Unix socket, debounced incremental re-indexing, and session/config hot-reload. Tasks only mention adding a watcher and MCP wiring, with no explicit work for IPC protocol, session management, or index lifecycle.
- **Location**: `agentd/changes/complete-argus-parity-2-2/specs/daemon-mode.md`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Recommendation**: Add tasks for IPC protocol definition, session handling, index lifecycle, and config reload behavior; or update the spec to match what is already implemented.

### Issue: Advanced LSP requirements not fully covered
- **Severity**: High
- **Category**: Completeness
- **Description**: The advanced-lsp spec requires workspace-wide find-references and incremental reference index updates. Tasks add a ReferenceIndex and rename/code actions but do not include `textDocument/references` wiring against the global index nor incremental update logic tied to file changes. The acceptance criterion for reindex on daemon restart is also not addressed.
- **Location**: `agentd/changes/complete-argus-parity-2-2/specs/advanced-lsp.md`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Recommendation**: Add explicit tasks for global references handler, incremental index updates on file changes, and daemon restart re-indexing.

### Issue: MCP toolset scope inconsistent across proposal and spec
- **Severity**: High
- **Category**: Consistency
- **Description**: The proposal promises 10+ tools, but the MCP spec lists 7 tools and uses different names than the existing code. The tasks do not reconcile this mismatch or specify which tool catalog is authoritative.
- **Location**: `agentd/changes/complete-argus-parity-2-2/proposal.md`, `agentd/changes/complete-argus-parity-2-2/specs/mcp-server.md`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Recommendation**: Align proposal/spec with an explicit, versioned tool list and naming scheme, and add tasks for any missing tools.

## Code Alignment Issues
[Check if proposal aligns with existing codebase]
[Examples: Do file paths exist? Do APIs exist? Does architecture match patterns?]
[IMPORTANT: If proposal mentions refactoring or BREAKING changes, deviations are EXPECTED]

### Issue: Type system tasks duplicate existing implementations
- **Severity**: Medium
- **Category**: Conflict
- **Description**: `Type::ParamSpec`, `Type::TypeVarTuple`, `Type::Protocol`, and `Type::TypedDict` already exist, and `check.rs` already contains Protocol/TypedDict assignability logic. The tasks as written risk duplicating or re-implementing existing behavior without clarifying the missing gaps.
- **Location**: `crates/argus/src/types/ty.rs`, `crates/argus/src/types/check.rs`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Note**: No explicit refactor note in proposal.
- **Recommendation**: Reframe tasks to target specific missing semantics (variance, TypeGuard/TypeIs narrowing, exhaustive match, etc.) instead of adding types already present.

### Issue: Daemon watcher task does not match current architecture
- **Severity**: Medium
- **Category**: Conflict
- **Description**: The daemon already uses a `FileWatcher` built on `notify`, and `daemon.rs` has a TODO placeholder. The task to "integrate notify crate" is outdated and ignores the existing watcher module. The real gap is wiring watcher events to `RequestHandler::on_file_change` and aligning debounce timing with spec (100ms).
- **Location**: `crates/argus/src/server/daemon.rs`, `crates/argus/src/watch.rs`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Note**: No refactor callout in proposal.
- **Recommendation**: Update the task to use `FileWatcher` events and implement targeted invalidation + debounce per spec.

### Issue: MCP tool naming conflicts with existing implementation
- **Severity**: Medium
- **Category**: Conflict
- **Description**: MCP spec tools are named `check`, `type_at`, `symbols`, etc., while the existing MCP tools are `argus_check`, `argus_type_at`, etc. This affects interoperability with MCP clients and acceptance criteria.
- **Location**: `crates/argus/src/mcp/tools.rs`, `agentd/changes/complete-argus-parity-2-2/specs/mcp-server.md`
- **Note**: No refactor note in proposal.
- **Recommendation**: Standardize naming in spec and code; add compatibility or aliasing if needed.

### Issue: LSP code actions and references already implemented in server
- **Severity**: Low
- **Category**: Conflict
- **Description**: `textDocument/references` and `textDocument/codeAction` are already implemented in `lsp/server.rs`, so tasks that create new modules for these features may be refactors rather than net-new functionality.
- **Location**: `crates/argus/src/lsp/server.rs`, `agentd/changes/complete-argus-parity-2-2/tasks.md`
- **Note**: No refactor note in proposal.
- **Recommendation**: Clarify whether this is a refactor; if so, update tasks/specs to reflect relocation and shared logic.

## Quality Suggestions
[Missing tests, error handling, edge cases, documentation]
[These are LOW priority - nice to have improvements]

### Issue: Acceptance criteria do not cover key edge cases
- **Severity**: Low
- **Category**: Completeness
- **Description**: The acceptance criteria are narrow relative to the claimed 100% parity and daemon behavior. There are no criteria for TypeIs narrowing, variance, overload resolution, TypeAlias/Annotated behaviors, or daemon session/config reload edge cases.
- **Recommendation**: Add WHEN/THEN scenarios for missing PEP features and daemon lifecycle events.

### Issue: Typeshed config surface area is unclear
- **Severity**: Low
- **Category**: Other
- **Description**: The spec references cache directories, enabled packages, and custom paths, but tasks only mention cache_dir and refresh_interval. This makes it unclear what the user-facing config contract is.
- **Recommendation**: Define a single canonical config schema and map it to both `ProjectConfig` and stub loading order.

## Verdict
- [ ] APPROVED - Ready for implementation
- [x] NEEDS_REVISION - Address issues above (specify which severity levels)
- [ ] REJECTED - Fundamental problems, needs rethinking

**Next Steps**: Resolve all HIGH severity consistency gaps (scope vs tasks), then update tasks/specs to align with existing code and the desired tool/daemon behavior.
