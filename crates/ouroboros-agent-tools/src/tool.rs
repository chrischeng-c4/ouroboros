use crate::error::{ToolError, ToolResult};
use async_trait::async_trait;
use ouroboros_agent_core::ToolCall;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub parameter_type: String,
}

/// Tool definition with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

/// Tool trait - all tools must implement this
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool name
    fn name(&self) -> &str;

    /// Get tool description
    fn description(&self) -> &str;

    /// Get tool parameters
    fn parameters(&self) -> Vec<ToolParameter>;

    /// Execute the tool with given arguments
    async fn execute(&self, arguments: serde_json::Value) -> ToolResult<serde_json::Value>;

    /// Get tool definition
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }

    /// Validate arguments before execution
    fn validate_arguments(&self, arguments: &serde_json::Value) -> ToolResult<()> {
        // Basic validation - check required parameters
        let args = arguments
            .as_object()
            .ok_or_else(|| ToolError::InvalidArguments("Arguments must be an object".into()))?;

        for param in self.parameters() {
            if param.required && !args.contains_key(&param.name) {
                return Err(ToolError::InvalidArguments(format!(
                    "Missing required parameter: {}",
                    param.name
                )));
            }
        }

        Ok(())
    }
}

/// Function-based tool wrapper
/// Allows creating tools from simple async functions
pub struct FunctionTool<F>
where
    F: Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
{
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
    function: F,
}

impl<F> FunctionTool<F>
where
    F: Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
{
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Vec<ToolParameter>,
        function: F,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            function,
        }
    }
}

#[async_trait]
impl<F> Tool for FunctionTool<F>
where
    F: Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        self.parameters.clone()
    }

    async fn execute(&self, arguments: serde_json::Value) -> ToolResult<serde_json::Value> {
        // Validate first
        self.validate_arguments(&arguments)?;

        // Execute function
        (self.function)(arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_function_tool() {
        let tool = FunctionTool::new(
            "test_tool",
            "A test tool",
            vec![ToolParameter {
                name: "input".to_string(),
                description: "Test input".to_string(),
                required: true,
                parameter_type: "string".to_string(),
            }],
            |args| {
                Box::pin(async move {
                    Ok(serde_json::json!({
                        "result": args.get("input").unwrap_or(&serde_json::json!("")).as_str().unwrap_or("")
                    }))
                })
            },
        );

        assert_eq!(tool.name(), "test_tool");
        assert_eq!(tool.description(), "A test tool");

        let result = tool
            .execute(serde_json::json!({"input": "hello"}))
            .await
            .unwrap();

        assert_eq!(result["result"], "hello");
    }
}
