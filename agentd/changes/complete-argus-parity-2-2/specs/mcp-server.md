# Specification: MCP Server

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

This specification defines the Model Context Protocol (MCP) server for Argus. It allows LLMs (like Claude) to directly interact with the Argus engine to analyze code, find symbols, and perform type-safe refactoring through a standardized tool interface.

## Requirements

### R1: MCP Protocol Support
The server SHALL implement the MCP specification over stdio, supporting `initialize`, `tools/list`, and `tools/call` methods.

### R2: Daemon Integration
The MCP server SHALL act as a thin client to the Argus Daemon, forwarding analysis requests to the daemon's Unix socket to ensure low latency.

### R3: Comprehensive Toolset
The server SHALL expose the following tools to the LLM:
- `check`: Run type checking on a file or project.
- `type_at`: Get the type of an expression at a specific location.
- `symbols`: List all symbols in a file.
- `goto_definition`: Find the definition of a symbol.
- `find_references`: Find all usages of a symbol.
- `analyze_project`: Provide a high-level summary of the project structure and types.
- `infer_types`: Suggest type annotations for unannotated code.

## Interfaces

### Tool: check
```json
{
  "name": "check",
  "description": "Type check a Python file or the whole project",
  "inputSchema": {
    "type": "object",
    "properties": {
      "file": { "type": "string", "description": "Optional file path to check" }
    }
  }
}
```

### Tool: type_at
```json
{
  "name": "type_at",
  "description": "Get the type of the expression at the given cursor position",
  "inputSchema": {
    "type": "object",
    "required": ["file", "line", "column"],
    "properties": {
      "file": { "type": "string" },
      "line": { "type": "integer" },
      "column": { "type": "integer" }
    }
  }
}
```

## Acceptance Criteria

### Scenario: LLM Type Inquiry
- **WHEN** an LLM calls `type_at` for a variable in a project
- **THEN** the MCP server should return the precise type string as inferred by the Argus engine.

### Scenario: Project-wide Symbol Search
- **WHEN** the `symbols` tool is called for a specific module
- **THEN** it should return a list of classes, functions, and variables with their respective types.

### Scenario: Error Handling
- **WHEN** a tool is called for a non-existent file
- **THEN** the MCP server should return a clear error message via the protocol's error handling mechanism.
