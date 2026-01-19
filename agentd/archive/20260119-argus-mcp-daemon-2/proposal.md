# Change: argus-mcp-daemon-2

<meta>
  <purpose>PRD - Product Requirements Document</purpose>
  <constraint>Describes WHAT and WHY, not HOW</constraint>
</meta>

<section id="summary" required="true">
## Summary

Implement a high-performance Model Context Protocol (MCP) server and a long-running Daemon for the Argus code analysis engine to provide LLMs with deep, type-aware code understanding.
</section>

<section id="why" required="true">
## Why

LLMs need fast and accurate access to code semantics (types, definitions, references) to provide high-quality coding assistance. The current CLI-based linting is too slow for interactive use by LLMs because it re-initializes for every request. A daemon provides:
1. **Performance**: Sub-millisecond response times via in-memory caching and background updates.
2. **Context**: Deep type analysis via a persistent Semantic Model that standard linters miss.
3. **Connectivity**: Standardized MCP interface for tools like Claude Desktop.
</section>

<section id="what-changes" required="true">
## What Changes

### Core Analysis (Argus)
- **Semantic Model**: Introduce an owned `SemanticModel` struct to store resolved types, symbols, and references independent of the AST.
- **Deep Type Analysis**: Update `TypeChecker` to populate the `SemanticModel` during analysis.
- **Background Analysis Loop**: Implement a debounced loop that consumes file watcher events, re-analyzes changed files, and updates the cache asynchronously.

### Daemon Server
- **Async Architecture**: Bridge synchronous file watcher events into the async daemon loop.
- **Protocol Extensions**: Add support for `hover`, `definition`, `references`, and `index_status`.

### MCP Integration
- **MCP Server**: Implement an MCP-compliant server that communicates via stdio.
- **MCP Tools**: Expose `argus_check`, `argus_type_at`, `argus_hover`, `argus_definition`, and `argus_references`.
- **Auto-Config**: Add a command to generate Claude Desktop configuration.

### CLI (ob argus)
- `ob argus server`: Command to start/manage the daemon.
- `ob argus mcp-server`: Command to start the MCP server.
- `ob argus mcp`: Command to output MCP configuration.
</section>

<section id="impact" required="true">
## Impact

- Affected specs: `specs/argus-daemon.md`, `specs/argus-mcp.md`
- Affected code: 
  - `crates/argus/src/server/*` (Daemon implementation)
  - `crates/argus/src/mcp/*` (MCP implementation)
  - `crates/argus/src/types/*` (Type checking and Semantic Model)
  - `crates/ouroboros-cli/src/main.rs` (CLI commands)
- Breaking changes: No. The daemon and MCP server are additive features.
</section>
