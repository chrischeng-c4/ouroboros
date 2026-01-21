//! MCP Server implementation for Argus
//!
//! Implements the Model Context Protocol over stdio, connecting to the Argus daemon.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::server::DaemonClient;

use super::tools::ArgusTools;

/// MCP Server that communicates via stdio
pub struct McpServer {
    client: DaemonClient,
}

/// MCP JSON-RPC request
#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

/// MCP error
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(workspace_root: PathBuf) -> Self {
        let client = DaemonClient::for_workspace(&workspace_root);
        Self { client }
    }

    /// Run the MCP server (blocking, reads from stdin, writes to stdout)
    pub fn run(&self) -> Result<(), String> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();

        let reader = stdin.lock();
        let mut writer = stdout.lock();

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;

            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_request(&line);
            let response_json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;

            writeln!(writer, "{}", response_json)
                .map_err(|e| format!("Failed to write response: {}", e))?;
            writer.flush()
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        Ok(())
    }

    /// Run the MCP server asynchronously
    pub async fn run_async(&self) -> Result<(), String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let reader = BufReader::new(stdin);
        let mut writer = stdout;
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await.map_err(|e| format!("Read error: {}", e))? {
            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_request_async(&line).await;
            let response_json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;

            writer.write_all(response_json.as_bytes()).await
                .map_err(|e| format!("Failed to write response: {}", e))?;
            writer.write_all(b"\n").await
                .map_err(|e| format!("Failed to write newline: {}", e))?;
            writer.flush().await
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        Ok(())
    }

    /// Handle a single request (sync version)
    fn handle_request(&self, line: &str) -> McpResponse {
        // Parse request
        let request: McpRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(McpError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
            }
        };

        // Create tokio runtime for async operations
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: format!("Internal error: {}", e),
                    }),
                };
            }
        };

        rt.block_on(self.handle_method(&request))
    }

    /// Handle a single request (async version)
    async fn handle_request_async(&self, line: &str) -> McpResponse {
        // Parse request
        let request: McpRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(McpError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
            }
        };

        self.handle_method(&request).await
    }

    /// Handle a specific method
    async fn handle_method(&self, request: &McpRequest) -> McpResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize().await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params.clone()).await,
            _ => Err(McpError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
            }),
        };

        match result {
            Ok(value) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: Some(value),
                error: None,
            },
            Err(error) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: None,
                error: Some(error),
            },
        }
    }

    /// Handle initialize
    async fn handle_initialize(&self) -> Result<Value, McpError> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "argus",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }

    /// Handle tools/list
    async fn handle_tools_list(&self) -> Result<Value, McpError> {
        let tools = ArgusTools::list();
        Ok(serde_json::json!({
            "tools": tools
        }))
    }

    /// Handle tools/call
    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, McpError> {
        let params = params.ok_or_else(|| McpError {
            code: -32602,
            message: "Missing params".to_string(),
        })?;

        let name = params.get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing tool name".to_string(),
            })?;

        let arguments = params.get("arguments").cloned();

        // Check if daemon is running
        if !self.client.is_daemon_running().await {
            return Err(McpError {
                code: -32000,
                message: "Argus daemon is not running. Start it with: ob argus server".to_string(),
            });
        }

        // Map MCP tool calls to daemon methods
        let result = match name {
            "argus_check" => {
                let path = arguments.as_ref()
                    .and_then(|a| a.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or(".");
                self.client.check(path).await
            }
            "argus_type_at" => {
                let args = arguments.as_ref().ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                })?;
                let file = args.get("file").and_then(|f| f.as_str()).ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing file argument".to_string(),
                })?;
                let line = args.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32;
                let column = args.get("column").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
                self.client.type_at(file, line, column).await
            }
            "argus_symbols" => {
                let file = arguments.as_ref()
                    .and_then(|a| a.get("file"))
                    .and_then(|f| f.as_str())
                    .ok_or_else(|| McpError {
                        code: -32602,
                        message: "Missing file argument".to_string(),
                    })?;
                self.client.symbols(file).await
            }
            "argus_diagnostics" => {
                let file = arguments.as_ref()
                    .and_then(|a| a.get("file"))
                    .and_then(|f| f.as_str());
                self.client.diagnostics(file).await
            }
            "argus_hover" => {
                let args = arguments.as_ref().ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                })?;
                let file = args.get("file").and_then(|f| f.as_str()).ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing file argument".to_string(),
                })?;
                let line = args.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32;
                let column = args.get("column").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
                self.client.request("hover", Some(serde_json::json!({
                    "file": file,
                    "line": line,
                    "column": column
                }))).await
            }
            "argus_definition" => {
                let args = arguments.as_ref().ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                })?;
                let file = args.get("file").and_then(|f| f.as_str()).ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing file argument".to_string(),
                })?;
                let line = args.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32;
                let column = args.get("column").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
                self.client.request("definition", Some(serde_json::json!({
                    "file": file,
                    "line": line,
                    "column": column
                }))).await
            }
            "argus_references" => {
                let args = arguments.as_ref().ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                })?;
                let file = args.get("file").and_then(|f| f.as_str()).ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing file argument".to_string(),
                })?;
                let line = args.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as u32;
                let column = args.get("column").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
                let include_decl = args.get("include_declaration").and_then(|i| i.as_bool()).unwrap_or(false);
                self.client.request("references", Some(serde_json::json!({
                    "file": file,
                    "line": line,
                    "column": column,
                    "include_declaration": include_decl
                }))).await
            }
            "argus_index_status" => {
                self.client.index_status().await
            }
            "argus_invalidate" => {
                let args = arguments.as_ref().ok_or_else(|| McpError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                })?;
                let files = args.get("files")
                    .and_then(|f| f.as_array())
                    .ok_or_else(|| McpError {
                        code: -32602,
                        message: "Missing files argument".to_string(),
                    })?;
                let file_strs: Vec<&str> = files.iter()
                    .filter_map(|v| v.as_str())
                    .collect();
                self.client.invalidate(&file_strs).await
            }
            _ => {
                return Err(McpError {
                    code: -32601,
                    message: format!("Unknown tool: {}", name),
                });
            }
        };

        result.map_err(|e| McpError {
            code: -32000,
            message: e,
        }).map(|v| serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
            }]
        }))
    }
}

/// Output MCP configuration for Claude Desktop
pub fn print_mcp_config() {
    let config = serde_json::json!({
        "mcpServers": {
            "argus": {
                "command": "ob",
                "args": ["argus", "mcp-server"],
                "env": {}
            }
        }
    });

    match serde_json::to_string_pretty(&config) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing config: {}", e),
    }
}
