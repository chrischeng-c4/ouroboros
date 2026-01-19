//! MCP Tool definitions for Argus
//!
//! This module defines the tool schemas for the Model Context Protocol (MCP).
//! These tools allow LLMs to programmatically interact with Argus for:
//! - Code analysis and type checking
//! - Python environment configuration
//! - Module discovery and resolution

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
            // Code analysis tools
            Self::argus_check(),
            Self::argus_type_at(),
            Self::argus_symbols(),
            Self::argus_diagnostics(),
            Self::argus_hover(),
            Self::argus_definition(),
            Self::argus_references(),
            Self::argus_index_status(),
            Self::argus_invalidate(),
            // Python environment configuration tools
            Self::argus_get_config(),
            Self::argus_set_python_paths(),
            Self::argus_configure_venv(),
            Self::argus_detect_environment(),
            Self::argus_list_modules(),
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

    fn argus_invalidate() -> ToolSchema {
        ToolSchema {
            name: "argus_invalidate".to_string(),
            description: "Invalidate the cache for specific files, forcing re-analysis on next access".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "List of file paths to invalidate"
                    }
                },
                "required": ["files"]
            }),
        }
    }

    // === Python Environment Configuration Tools ===

    fn argus_get_config() -> ToolSchema {
        ToolSchema {
            name: "argus_get_config".to_string(),
            description: "Get the current Python environment configuration, including search paths, active virtual environment, and detected environments.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "description": "No parameters required. Returns the merged configuration from all sources."
            }),
        }
    }

    fn argus_set_python_paths() -> ToolSchema {
        ToolSchema {
            name: "argus_set_python_paths".to_string(),
            description: "Configure additional Python module search paths. Updates the [tool.argus.python] section in pyproject.toml to persist the configuration.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "List of directory paths to search for Python modules (e.g., ['./lib', './src'])"
                    },
                    "append": {
                        "type": "boolean",
                        "description": "If true, append to existing paths instead of replacing. Default: false"
                    }
                },
                "required": ["paths"]
            }),
        }
    }

    fn argus_configure_venv() -> ToolSchema {
        ToolSchema {
            name: "argus_configure_venv".to_string(),
            description: "Configure the virtual environment to use for module resolution. Updates pyproject.toml with the new venv_path setting.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "venv_path": {
                        "type": "string",
                        "description": "Path to the virtual environment directory (e.g., '.venv', 'custom_env', or absolute path)"
                    }
                },
                "required": ["venv_path"]
            }),
        }
    }

    fn argus_detect_environment() -> ToolSchema {
        ToolSchema {
            name: "argus_detect_environment".to_string(),
            description: "Automatically detect virtual environments in the project. Returns a list of detected environments with their paths and types (venv, poetry, pipenv).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_root": {
                        "type": "string",
                        "description": "Optional project root directory to scan. Defaults to current working directory."
                    }
                }
            }),
        }
    }

    fn argus_list_modules() -> ToolSchema {
        ToolSchema {
            name: "argus_list_modules".to_string(),
            description: "List all Python modules discoverable by the import resolver. Useful for understanding what modules are available in the current environment.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "prefix": {
                        "type": "string",
                        "description": "Optional prefix to filter modules (e.g., 'django.' to list all Django submodules)"
                    },
                    "include_stubs": {
                        "type": "boolean",
                        "description": "Whether to indicate which modules have type stubs (.pyi files). Default: true"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of modules to return. Default: 100"
                    }
                }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_list_completeness() {
        let tools = ArgusTools::list();

        // Should have all tools
        assert!(tools.len() >= 14);

        // Check specific tools exist
        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();

        // Original tools
        assert!(names.contains(&"argus_check"));
        assert!(names.contains(&"argus_type_at"));
        assert!(names.contains(&"argus_symbols"));
        assert!(names.contains(&"argus_diagnostics"));
        assert!(names.contains(&"argus_hover"));
        assert!(names.contains(&"argus_definition"));
        assert!(names.contains(&"argus_references"));
        assert!(names.contains(&"argus_index_status"));
        assert!(names.contains(&"argus_invalidate"));

        // New environment tools
        assert!(names.contains(&"argus_get_config"));
        assert!(names.contains(&"argus_set_python_paths"));
        assert!(names.contains(&"argus_configure_venv"));
        assert!(names.contains(&"argus_detect_environment"));
        assert!(names.contains(&"argus_list_modules"));
    }

    #[test]
    fn test_tool_schemas_valid_json() {
        for tool in ArgusTools::list() {
            // All tools should have valid JSON schemas
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());

            // Schema should have a type
            let schema = &tool.input_schema;
            assert!(schema.get("type").is_some());
        }
    }

    #[test]
    fn test_argus_get_config_schema() {
        let tool = ArgusTools::argus_get_config();
        assert_eq!(tool.name, "argus_get_config");
        assert!(tool.description.contains("configuration"));
    }

    #[test]
    fn test_argus_set_python_paths_schema() {
        let tool = ArgusTools::argus_set_python_paths();
        assert_eq!(tool.name, "argus_set_python_paths");

        let props = tool.input_schema.get("properties").unwrap();
        assert!(props.get("paths").is_some());

        let required = tool.input_schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v == "paths"));
    }

    #[test]
    fn test_argus_detect_environment_schema() {
        let tool = ArgusTools::argus_detect_environment();
        assert_eq!(tool.name, "argus_detect_environment");
        assert!(tool.description.contains("detect"));
    }

    #[test]
    fn test_argus_list_modules_schema() {
        let tool = ArgusTools::argus_list_modules();
        assert_eq!(tool.name, "argus_list_modules");

        let props = tool.input_schema.get("properties").unwrap();
        assert!(props.get("prefix").is_some());
        assert!(props.get("limit").is_some());
    }
}
