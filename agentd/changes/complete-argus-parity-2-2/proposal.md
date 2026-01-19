# Change: complete-argus-parity-2-2

<meta>
  <purpose>PRD - Product Requirements Document</purpose>
  <constraint>Describes WHAT and WHY, not HOW</constraint>
</meta>

<section id="summary" required="true">
## Summary

Achieve 100% Python type checking parity with mypy and pyright, while introducing an integrated Model Context Protocol (MCP) Server and high-performance Daemon Mode to provide instant, AI-friendly code analysis.
</section>

<section id="why" required="true">
## Why

Argus aims to be the high-performance backbone for Python development, but currently lacks advanced type system features (generics, protocols, variance) and full PEP compliance required for large-scale production codebases. Furthermore, existing AI integrations are slow and lack deep context. By reaching parity with industry standards and providing a warm daemon with MCP support, we enable a new class of ultra-fast, context-aware AI development tools.
</section>

<section id="what-changes" required="true">
## What Changes

- **Core Type System (100% Parity)**:
    - Implement full support for PEP 484, 544 (Protocols), 586 (Literals), 589 (TypedDict), 612 (ParamSpec), and 646 (Variadic Generics).
    - Advanced type narrowing (TypeGuard, TypeIs) and exhaustive match checking.
    - Bidirectional type inference and generic parameter resolution.
- **Daemon Mode**:
    - Background process with in-memory workspace indexing for sub-100ms analysis.
    - Unix socket communication for fast local IPC.
    - Integrated file system watcher with debounced re-indexing.
- **MCP Server**:
    - Implementation of Model Context Protocol over stdio.
    - 10+ AI-friendly tools including `check`, `type_at`, `symbols`, `analyze_project`, and `infer_types`.
    - Leverages the warm daemon for low-latency responses to LLM requests.
- **Dynamic Typeshed & Module System**:
    - Automated downloading, caching, and loading of typeshed stubs.
    - Full cross-file import resolution and circular dependency detection.
- **Enhanced LSP & Semantic Analysis**:
    - Implementation of `textDocument/rename`, `textDocument/references`, and `textDocument/codeAction` (quick fixes).
    - New `ReferenceIndex` for global usage tracking.
    - Robust AST error recovery to maintain analysis integrity during active editing.
</section>

<section id="impact" required="true">
## Impact

- Affected specs:
    - `specs/core-type-system.md` (New)
    - `specs/daemon-mode.md` (New)
    - `specs/mcp-server.md` (New)
    - `specs/dynamic-typeshed.md` (Modified)
    - `specs/error-recovery.md` (Modified)
    - `specs/watch-mode.md` (Modified)
    - `specs/advanced-lsp.md` (Modified)
- Affected code:
    - `crates/argus/src/core/` (Config, Errors)
    - `crates/argus/src/types/` (Type system, Inference, Typeshed)
    - `crates/argus/src/semantic/` (Symbols, Reference indexing)
    - `crates/argus/src/syntax/` (Parser recovery)
    - `crates/argus/src/server/` (Daemon, IPC)
    - `crates/argus/src/mcp/` (MCP protocol and tools)
    - `crates/argus/src/lsp/` (Server, Workspace, CodeActions)
- Breaking changes: No. The changes are additive or corrective to existing internal APIs.
</section>