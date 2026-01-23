use crate::agent::Agent;
use crate::context::AgentContext;
use crate::error::{AgentError, AgentResult};
use crate::types::{AgentResponse, Message};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Agent executor with GIL release support
/// Handles the execution of agent operations with proper async runtime management
pub struct AgentExecutor {
    /// Timeout for agent execution (None = no timeout)
    execution_timeout: Option<Duration>,

    /// Maximum retry attempts for retriable errors
    max_retries: u32,

    /// Delay between retries
    retry_delay: Duration,
}

impl AgentExecutor {
    /// Create a new executor with default settings
    pub fn new() -> Self {
        Self {
            execution_timeout: None,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = Some(timeout);
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set retry delay
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Execute an agent with the given input
    /// This method releases the GIL and runs the agent asynchronously
    pub async fn execute(
        &self,
        agent: Arc<dyn Agent>,
        context: &mut AgentContext,
        input: Message,
    ) -> AgentResult<AgentResponse> {
        debug!(
            "Executing agent {} with input: {}",
            agent.config().agent_id.as_str(),
            input.content
        );

        // Execute with retries
        let mut attempt = 0;
        loop {
            let result = self.execute_once(agent.clone(), context, input.clone()).await;

            match result {
                Ok(response) => {
                    info!(
                        "Agent {} execution succeeded on attempt {}",
                        agent.config().agent_id.as_str(),
                        attempt + 1
                    );
                    return Ok(response);
                }
                Err(e) if e.is_retriable() && attempt < self.max_retries => {
                    warn!(
                        "Agent {} execution failed (attempt {}): {}. Retrying...",
                        agent.config().agent_id.as_str(),
                        attempt + 1,
                        e
                    );
                    attempt += 1;
                    tokio::time::sleep(self.retry_delay).await;
                }
                Err(e) => {
                    error!(
                        "Agent {} execution failed: {}",
                        agent.config().agent_id.as_str(),
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Execute agent once (without retries)
    async fn execute_once(
        &self,
        agent: Arc<dyn Agent>,
        context: &mut AgentContext,
        input: Message,
    ) -> AgentResult<AgentResponse> {
        let execution = async {
            // Add input message to context
            context.add_message(input.clone());
            context.increment_turn();

            // Execute agent
            agent.execute(context, input).await
        };

        // Apply timeout if configured
        match self.execution_timeout {
            Some(timeout_duration) => {
                timeout(timeout_duration, execution)
                    .await
                    .map_err(|_| {
                        AgentError::ExecutionError(format!(
                            "Agent execution timed out after {:?}",
                            timeout_duration
                        ))
                    })?
            }
            None => execution.await,
        }
    }

    /// Run agent with text input (convenience method)
    pub async fn run(
        &self,
        agent: Arc<dyn Agent>,
        input: String,
    ) -> AgentResult<AgentResponse> {
        let mut context = AgentContext::new(agent.config().agent_id.clone());

        // Add system prompt if configured
        if let Some(system_prompt) = &agent.config().system_prompt {
            context.add_message(Message::system(system_prompt.clone()));
        }

        let input_message = Message::user(input);
        self.execute(agent, &mut context, input_message).await
    }

    /// Run agent with existing context
    pub async fn run_with_context(
        &self,
        agent: Arc<dyn Agent>,
        context: &mut AgentContext,
        input: String,
    ) -> AgentResult<AgentResponse> {
        let input_message = Message::user(input);
        self.execute(agent, context, input_message).await
    }
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::types::{AgentConfig, AgentId};
    use async_trait::async_trait;

    struct MockAgent {
        config: AgentConfig,
        should_fail: bool,
    }

    #[async_trait]
    impl Agent for MockAgent {
        fn config(&self) -> &AgentConfig {
            &self.config
        }

        async fn execute(
            &self,
            _context: &mut AgentContext,
            _input: Message,
        ) -> AgentResult<AgentResponse> {
            if self.should_fail {
                Err(AgentError::ExecutionError("mock failure".to_string()))
            } else {
                Ok(AgentResponse {
                    content: "mock response".to_string(),
                    tool_calls: None,
                    metadata: Default::default(),
                    message: Message::assistant("mock response"),
                    usage: Default::default(),
                })
            }
        }
    }

    #[tokio::test]
    async fn test_executor_run_success() {
        let config = AgentConfig::new(AgentId::new("test-agent"));
        let agent = Arc::new(MockAgent {
            config,
            should_fail: false,
        });

        let executor = AgentExecutor::new();
        let response = executor.run(agent, "Hello".to_string()).await.unwrap();

        assert_eq!(response.content, "mock response");
    }

    #[tokio::test]
    async fn test_executor_with_timeout() {
        let config = AgentConfig::new(AgentId::new("test-agent"));
        let agent = Arc::new(MockAgent {
            config,
            should_fail: false,
        });

        let executor = AgentExecutor::new().with_timeout(Duration::from_secs(5));
        let response = executor.run(agent, "Hello".to_string()).await.unwrap();

        assert_eq!(response.content, "mock response");
    }

    #[tokio::test]
    async fn test_executor_run_with_context() {
        let config = AgentConfig::new(AgentId::new("test-agent"));
        let agent = Arc::new(MockAgent {
            config,
            should_fail: false,
        });

        let mut context = AgentContext::new(AgentId::new("test-agent"));
        let executor = AgentExecutor::new();

        let response = executor
            .run_with_context(agent, &mut context, "Hello".to_string())
            .await
            .unwrap();

        assert_eq!(response.content, "mock response");
        assert_eq!(context.turn_count, 1);
    }
}
