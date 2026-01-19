# Changelog

All notable changes to the Argus specifications will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [2026-01-19] - argus-mcp-daemon-2

### Added
- **Argus Daemon Specification** (`argus-daemon.md`)
  - Live code indexing with SemanticModel
  - Incremental background updates with file watching
  - Fast JSON-RPC interface over Unix Domain Socket
  - Deep type analysis with TypeChecker integration
  - Async event bridging for file watcher

- **Argus MCP Server Specification** (`argus-mcp.md`)
  - MCP server with stdio transport
  - Daemon bridge for tool forwarding
  - Full tool exposure (9 tools):
    - `argus_check` - Run full analysis
    - `argus_type_at` - Get type at position
    - `argus_symbols` - List symbols in file
    - `argus_diagnostics` - Get diagnostics
    - `argus_hover` - Get hover information
    - `argus_definition` - Go to definition
    - `argus_references` - Find references
    - `argus_index_status` - Get index status
    - `argus_invalidate` - Invalidate cache
  - MCP configuration generation

### Implementation
- 18 comprehensive tests (all passing)
- Performance benchmarks for daemon operations
- SemanticModel integration with RequestHandler
- Backward compatibility with existing SymbolTable

### Notes
- Queue size tracking deferred to future release
- Last updated timestamp tracking deferred
