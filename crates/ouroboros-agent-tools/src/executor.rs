use crate::error::{ToolError, ToolResult};
use crate::registry::ToolRegistry;
use crate::tool::Tool;
use ouroboros_agent_core::ToolResult as AgentToolResult;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, warn};

/// Tool executor with timeout and retry support
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    timeout_duration: Duration,
    max_retries: u32,
}

impl ToolExecutor {
    /// Create a new tool executor with the given registry
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            timeout_duration: Duration::from_secs(30),
            max_retries: 1,
        }
    }

    /// Set timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Execute a tool by name with the given arguments
    pub async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        debug!("Executing tool: {} with args: {:?}", tool_name, arguments);

        // Get tool from registry
        let tool = self
            .registry
            .get(tool_name)
            .ok_or_else(|| ToolError::NotFound(tool_name.to_string()))?;

        // Execute with retries
        let mut attempt = 0;
        loop {
            let result = self.execute_with_timeout(tool.clone(), arguments.clone()).await;

            match result {
                Ok(output) => {
                    debug!("Tool {} executed successfully", tool_name);
                    return Ok(output);
                }
                Err(e) if attempt < self.max_retries => {
                    warn!(
                        "Tool {} execution failed (attempt {}): {}. Retrying...",
                        tool_name,
                        attempt + 1,
                        e
                    );
                    attempt += 1;
                }
                Err(e) => {
                    error!("Tool {} execution failed: {}", tool_name, e);
                    return Err(e);
                }
            }
        }
    }

    /// Execute with timeout
    async fn execute_with_timeout(
        &self,
        tool: Arc<dyn Tool>,
        arguments: serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        timeout(self.timeout_duration, tool.execute(arguments))
            .await
            .map_err(|_| {
                ToolError::Timeout(self.timeout_duration.as_secs())
            })?
    }

    /// Execute multiple tool calls in parallel
    pub async fn execute_batch(
        &self,
        tool_calls: Vec<(String, serde_json::Value)>,
    ) -> Vec<AgentToolResult> {
        let futures: Vec<_> = tool_calls
            .into_iter()
            .enumerate()
            .map(|(idx, (name, args))| {
                let executor = self.clone();
                async move {
                    let result = executor.execute(&name, args).await;
                    AgentToolResult {
                        tool_call_id: format!("call_{}", idx),
                        output: match &result {
                            Ok(output) => output.clone(),
                            Err(_) => serde_json::json!(null),
                        },
                        error: result.err().map(|e| e.to_string()),
                    }
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }
}

impl Clone for ToolExecutor {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            timeout_duration: self.timeout_duration,
            max_retries: self.max_retries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;
    use crate::tool::FunctionTool;

    #[tokio::test]
    async fn test_executor() {
        let registry = Arc::new(ToolRegistry::new());

        let tool = Arc::new(FunctionTool::new(
            "echo",
            "Echo tool",
            vec![],
            |args| {
                Box::pin(async move {
                    Ok(args)
                })
            },
        ));

        registry.register(tool).unwrap();

        let executor = ToolExecutor::new(registry);
        let result = executor
            .execute("echo", serde_json::json!({"msg": "hello"}))
            .await
            .unwrap();

        assert_eq!(result["msg"], "hello");
    }

    #[tokio::test]
    async fn test_executor_not_found() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(registry);

        let result = executor.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
    }
}
