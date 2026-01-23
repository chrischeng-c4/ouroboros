use crate::types::{AgentId, Message, SharedState};
use std::collections::HashMap;
use std::sync::Arc;

/// Agent execution context
/// Contains all the runtime information needed during agent execution
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Agent identifier
    pub agent_id: AgentId,

    /// Conversation history
    pub messages: Vec<Message>,

    /// Shared state across turns (Copy-on-Write)
    pub state: SharedState,

    /// Number of turns executed so far
    pub turn_count: u32,

    /// Additional context metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentContext {
    pub fn new(agent_id: impl Into<AgentId>) -> Self {
        Self {
            agent_id: agent_id.into(),
            messages: Vec::new(),
            state: Arc::new(HashMap::new()),
            turn_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the conversation history
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Update the shared state (Copy-on-Write)
    pub fn update_state(&mut self, key: String, value: serde_json::Value) {
        let mut new_state = (*self.state).clone();
        new_state.insert(key, value);
        self.state = Arc::new(new_state);
    }

    /// Get a value from the state
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Increment turn counter
    pub fn increment_turn(&mut self) {
        self.turn_count += 1;
    }

    /// Get the last N messages
    pub fn last_messages(&self, n: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    /// Clear conversation history (keep system messages)
    pub fn clear_history(&mut self) {
        self.messages.retain(|msg| msg.role == crate::types::Role::System);
        self.turn_count = 0;
    }

    /// Add metadata
    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[test]
    fn test_context_creation() {
        let ctx = AgentContext::new("test-agent");
        assert_eq!(ctx.agent_id.as_str(), "test-agent");
        assert_eq!(ctx.messages.len(), 0);
        assert_eq!(ctx.turn_count, 0);
    }

    #[test]
    fn test_add_message() {
        let mut ctx = AgentContext::new("test-agent");
        ctx.add_message(Message::user("Hello"));
        assert_eq!(ctx.messages.len(), 1);
    }

    #[test]
    fn test_state_update() {
        let mut ctx = AgentContext::new("test-agent");
        ctx.update_state("key".to_string(), serde_json::json!("value"));
        assert_eq!(
            ctx.get_state("key"),
            Some(&serde_json::json!("value"))
        );
    }

    #[test]
    fn test_turn_increment() {
        let mut ctx = AgentContext::new("test-agent");
        assert_eq!(ctx.turn_count, 0);
        ctx.increment_turn();
        assert_eq!(ctx.turn_count, 1);
    }

    #[test]
    fn test_last_messages() {
        let mut ctx = AgentContext::new("test-agent");
        ctx.add_message(Message::user("1"));
        ctx.add_message(Message::user("2"));
        ctx.add_message(Message::user("3"));

        let last_2 = ctx.last_messages(2);
        assert_eq!(last_2.len(), 2);
        assert_eq!(last_2[0].content, "2");
        assert_eq!(last_2[1].content, "3");
    }
}
