//! MCP Tool definitions for Argus

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool schema for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Argus MCP tools
pub struct ArgusTools;

impl ArgusTools {
    /// Get all available tools
    pub fn list() -> Vec<ToolSchema> {
        vec![
            Self::argus_check(),
            Self::argus_type_at(),
            Self::argus_symbols(),
            Self::argus_diagnostics(),
            Self::argus_hover(),
            Self::argus_definition(),
            Self::argus_references(),
            Self::argus_index_status(),
        ]
    }

    fn argus_check() -> ToolSchema {
        ToolSchema {
            name: "argus_check".to_string(),
            description: "Check files or directories for code issues (linting + type analysis)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to file or directory to check"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn argus_type_at() -> ToolSchema {
        ToolSchema {
            name: "argus_type_at".to_string(),
            description: "Get the type at a specific position in a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number (0-indexed)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (0-indexed)"
                    }
                },
                "required": ["file", "line", "column"]
            }),
        }
    }

    fn argus_symbols() -> ToolSchema {
        ToolSchema {
            name: "argus_symbols".to_string(),
            description: "List all symbols (functions, classes, variables) in a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the file"
                    }
                },
                "required": ["file"]
            }),
        }
    }

    fn argus_diagnostics() -> ToolSchema {
        ToolSchema {
            name: "argus_diagnostics".to_string(),
            description: "Get all diagnostics (errors, warnings) for a file or the entire project".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Optional path to a specific file. If omitted, returns all diagnostics."
                    }
                }
            }),
        }
    }

    fn argus_hover() -> ToolSchema {
        ToolSchema {
            name: "argus_hover".to_string(),
            description: "Get hover information (type, documentation) at a position".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number (0-indexed)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (0-indexed)"
                    }
                },
                "required": ["file", "line", "column"]
            }),
        }
    }

    fn argus_definition() -> ToolSchema {
        ToolSchema {
            name: "argus_definition".to_string(),
            description: "Go to the definition of a symbol at a position".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number (0-indexed)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (0-indexed)"
                    }
                },
                "required": ["file", "line", "column"]
            }),
        }
    }

    fn argus_references() -> ToolSchema {
        ToolSchema {
            name: "argus_references".to_string(),
            description: "Find all references to a symbol at a position".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number (0-indexed)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (0-indexed)"
                    },
                    "include_declaration": {
                        "type": "boolean",
                        "description": "Whether to include the declaration in results"
                    }
                },
                "required": ["file", "line", "column"]
            }),
        }
    }

    fn argus_index_status() -> ToolSchema {
        ToolSchema {
            name: "argus_index_status".to_string(),
            description: "Get the current status of the code index (files indexed, symbols, etc.)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }
}
