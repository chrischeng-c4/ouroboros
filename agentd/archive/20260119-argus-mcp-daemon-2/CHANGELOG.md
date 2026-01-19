# Changelog

## [Unreleased]

### Added
- **Argus Semantic Model**: Introduced an owned `SemanticModel` for persisting type information, definitions, and references independent of the AST.
- **Background Analysis**: Implemented a background analysis loop in the Argus Daemon that automatically re-analyzes files upon change detection, ensuring the semantic model is always up-to-date.
- **Deep Type Analysis**: Integrated `TypeChecker` into the main analysis pipeline, enabling deeper type inference and verification (TCxxx rules).
- **Async File Watcher**: Added an async bridge for file watcher events to support the non-blocking daemon architecture.
- **MCP Tools**: Expanded the Model Context Protocol (MCP) server with new tools: `argus_hover`, `argus_definition`, `argus_references`, and `argus_index_status`.
- **CLI Commands**: Added `ob argus mcp` and `ob argus mcp-server` commands for easy MCP integration.

### Changed
- **RequestHandler Optimization**: Refactored `RequestHandler` to utilize the cached `SemanticModel` for `type_at`, `hover`, and other queries, significantly improving response latency (targeting <5ms).
- **Python Linting**: Updated `PythonChecker` to include type checking passes by default.

### Fixed
- Addressed latency issues in interactive tools by decoupling analysis from request handling.
