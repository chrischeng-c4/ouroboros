---
change: argus-mcp-daemon
date: 2026-01-19
phase: H1
---

# Clarifications: Phase H1 - MCP Server + Daemon Mode

## Context

This is **Phase H1** of a multi-phase roadmap to achieve Python type checking parity with mypy/pyright.

**Full Roadmap**:
- **Phase H1** (this change): MCP Server + Daemon Mode
- Phase H2 (future): Core Type System Enhancement (80-90% parity)
- Phase H3 (future): Advanced LSP + Global References
- Phase H4 (future): Validation & Performance

## Scope: Phase H1 Only

This change focuses **exclusively** on building the infrastructure for LLM integration:
- MCP Server with 10 core tools
- Daemon Mode for persistent, fast analysis
- File watcher for incremental updates

**Out of Scope for H1**:
- New type checking features (use existing capabilities)
- 100% PEP parity (defer to Phase H2)
- Advanced LSP features like workspace-wide references (defer to Phase H3)
- Comprehensive benchmark suite (defer to Phase H4)

---

## Requirements Clarification

### Q1: MCP Server Scope
- **Question**: Which MCP tools should Phase H1 implement?
- **Answer**: **10 core tools** for LLM usage:
  1. `check` - Type check files, return diagnostics as JSON
  2. `type_at` - Query type at specific position
  3. `symbols` - Extract symbol table (functions, classes, variables)
  4. `hover` - Get type information at position
  5. `goto_definition` - Find symbol definition
  6. `find_references` - Find symbol usages (local file only for H1)
  7. `quick_fix` - Get auto-fix suggestions
  8. `analyze_file` - Single file deep analysis
  9. `lint` - Run linting rules on files
  10. `format` - Get formatting suggestions

- **Rationale**: These 10 tools cover the most common LLM workflows. Advanced tools like `analyze_project` and `infer_types` can be added in Phase H2+.

### Q2: Daemon Mode Requirements
- **Question**: What are the core daemon features for Phase H1?
- **Answer**:
  - **Unix socket server** (JSON-RPC protocol)
  - **In-memory code index** (AST cache for fast re-analysis)
  - **File watcher** (debounced, 100ms interval)
  - **Session management** (multiple clients can connect)
  - **Graceful shutdown** (save cache, close connections)

- **Performance Target**:
  - First request: <2s (includes daemon startup)
  - Warm requests: <100ms
  - Incremental update: <50ms per file

- **Rationale**: Focus on performance infrastructure. Advanced features like config hot-reload can wait.

### Q3: Type Checking Capabilities
- **Question**: Should Phase H1 implement new type checking features?
- **Answer**: **No - Use existing capabilities**
  - Phase G already provides:
    - Basic type inference
    - Generics, Union, Optional
    - Protocol types (structural subtyping)
    - TypedDict, NamedTuple
    - Dataclass and property support
    - Configuration system (pyproject.toml)

- **What's Good Enough**: Current Argus can handle ~60-70% of common Python codebases. This is sufficient for Phase H1 to demonstrate MCP value to LLMs.

- **Rationale**: Decouple infrastructure (MCP/Daemon) from type system improvements. This reduces scope and makes Phase H1 achievable in 1-2 weeks.

### Q4: Integration with Existing Code
- **Question**: How should MCP tools use existing Argus functionality?
- **Answer**: **Leverage existing modules**:
  - Use `crates/argus/src/types/check.rs` for type checking
  - Use `crates/argus/src/lsp/server.rs` logic for hover/definition
  - Use `crates/argus/src/lint/mod.rs` for linting
  - Use `crates/argus/src/semantic/symbols.rs` for symbol extraction

- **New Code**:
  - `crates/argus/src/mcp/server.rs` - MCP protocol handler
  - `crates/argus/src/mcp/tools.rs` - Tool implementations (thin wrappers)
  - `crates/argus/src/server/daemon.rs` - Daemon process
  - `crates/argus/src/server/protocol.rs` - JSON-RPC protocol
  - `crates/argus/src/watch.rs` - File watcher (already exists, needs integration)

- **Rationale**: Minimize new code by reusing existing functionality. MCP tools are "adapters" that expose Argus capabilities via MCP protocol.

### Q5: Validation for Phase H1
- **Question**: How to validate Phase H1 is successful?
- **Answer**: **Functional validation only** (no performance benchmarks yet):
  1. **MCP Server Tests**: Each of the 10 tools responds correctly to valid requests
  2. **Daemon Tests**:
     - Daemon starts and accepts connections
     - Multiple clients can connect simultaneously
     - File changes trigger re-analysis
     - Graceful shutdown works
  3. **Integration Test**: LLM can use MCP tools to analyze a real Python file (e.g., FastAPI example)

- **Success Criteria**:
  - All 10 MCP tools working
  - Daemon responds <100ms (warm)
  - File watcher triggers updates within 100ms
  - No crashes or memory leaks in 30min test

- **Rationale**: Defer comprehensive benchmarking to Phase H4. Focus on "does it work?" not "is it fast enough?".

### Q6: Configuration
- **Question**: What configuration options for Phase H1?
- **Answer**: **Minimal config** (extend in later phases):
  - `[tool.argus.daemon]` in pyproject.toml:
    - `socket_path` (default: `/tmp/argus.sock`)
    - `watch_debounce_ms` (default: `100`)
    - `max_memory_mb` (default: `500`)

  - `[tool.argus.mcp]`:
    - `enabled` (default: `true`)
    - `stdio` (default: `true` - use stdio transport)

- **Rationale**: Keep config simple. Advanced options (custom typeshed paths, strict mode, etc.) already exist from Phase G.

---

## Implementation Scope Summary

### What Phase H1 WILL Deliver

1. **MCP Server**:
   - 10 core tools fully implemented
   - Stdio transport for CLI integration
   - Unix socket transport (via daemon)
   - JSON-RPC 2.0 protocol compliance

2. **Daemon Mode**:
   - Background process (persistent)
   - Unix socket IPC
   - In-memory AST cache
   - File watcher integration
   - Session management
   - Graceful shutdown

3. **CLI Integration**:
   - `ob argus daemon start` - Start daemon
   - `ob argus daemon stop` - Stop daemon
   - `ob argus daemon status` - Check daemon
   - `ob argus mcp` - Run MCP server (stdio mode)

4. **Testing**:
   - Unit tests for each MCP tool
   - Integration tests for daemon
   - Example LLM workflow (documented)

### What Phase H1 WILL NOT Deliver

1. **Type System Enhancements**:
   - No new PEP features
   - No TypeGuard/TypeIs narrowing
   - No variance checking
   - No exhaustive match checking

2. **Advanced LSP**:
   - No workspace-wide references
   - No cross-file rename
   - No persistent reference index

3. **Dynamic Typeshed**:
   - No automatic stub downloading
   - Use bundled stubs only

4. **Performance Optimization**:
   - No benchmark suite
   - No mypy/pyright comparison
   - No performance tuning (beyond basic caching)

5. **Advanced MCP Tools**:
   - No `analyze_project` (project-wide stats)
   - No `infer_types` (suggest annotations)
   - Defer to Phase H2+

---

## Dependencies on Future Phases

- **Phase H2** will add:
  - Advanced type checking (TypeGuard, Literals, etc.)
  - Dynamic typeshed integration
  - MCP tools: `analyze_project`, `infer_types`

- **Phase H3** will add:
  - Workspace-wide references
  - Cross-file rename
  - Global reference index

- **Phase H4** will add:
  - Comprehensive benchmarks
  - Performance tuning
  - Comparison with mypy/pyright

---

## Timeline

**Estimated effort**: 1-2 weeks

**Breakdown**:
- MCP Server implementation: 3-4 days
- Daemon infrastructure: 2-3 days
- File watcher integration: 1 day
- Testing and documentation: 2-3 days

---

## Success Criteria (Phase H1 Only)

1. ✅ **Functional**: All 10 MCP tools work correctly
2. ✅ **Performance**: Daemon warm response <100ms
3. ✅ **Stability**: No crashes in 30min test session
4. ✅ **Usability**: LLM can analyze a FastAPI example file via MCP
5. ✅ **Documentation**: MCP protocol documented with examples
