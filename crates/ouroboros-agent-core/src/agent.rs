use crate::context::AgentContext;
use crate::error::{AgentError, AgentResult};
use crate::types::{AgentConfig, AgentResponse, Message};
use async_trait::async_trait;
use std::sync::Arc;

/// Core agent trait
/// All agents must implement this trait to participate in the agent execution framework
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent configuration
    fn config(&self) -> &AgentConfig;

    /// Execute a single turn with the given input message
    /// Returns the agent's response
    async fn execute(&self, context: &mut AgentContext, input: Message) -> AgentResult<AgentResponse>;

    /// Validate agent configuration
    fn validate_config(&self) -> AgentResult<()> {
        // Basic validation - subclasses can override
        Ok(())
    }
}

/// Type alias for shared agent reference
pub type AgentRef = Arc<dyn Agent>;

/// Base agent implementation
/// Provides common functionality for all agent types
pub struct BaseAgent {
    config: AgentConfig,
}

impl BaseAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Validate turn count against max_turns limit
    pub fn check_max_turns(&self, context: &AgentContext) -> AgentResult<()> {
        if self.config.max_turns > 0 && context.turn_count >= self.config.max_turns {
            return Err(AgentError::MaxTurnsReached(self.config.max_turns));
        }
        Ok(())
    }

    /// Add system prompt to context if not already present
    pub fn ensure_system_prompt(&self, context: &mut AgentContext) {
        if let Some(system_prompt) = &self.config.system_prompt {
            if context.messages.is_empty() {
                context.add_message(Message::system(system_prompt.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentId, AgentConfig};

    #[test]
    fn test_base_agent_creation() {
        let config = AgentConfig::new(AgentId::new("test-agent"));
        let base = BaseAgent::new(config);
        assert_eq!(base.config.agent_id.as_str(), "test-agent");
    }

    #[test]
    fn test_base_agent_max_turns() {
        let config = AgentConfig::new(AgentId::new("test-agent")).with_max_turns(2);
        let base = BaseAgent::new(config);

        let mut context = AgentContext::new(AgentId::new("test-agent"));
        context.turn_count = 1;
        assert!(base.check_max_turns(&context).is_ok());

        context.turn_count = 2;
        assert!(base.check_max_turns(&context).is_err());
    }
}
