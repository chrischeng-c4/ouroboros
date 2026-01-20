use crate::types::SharedState;
use std::collections::HashMap;
use std::sync::Arc;

/// State manager with Copy-on-Write semantics
/// Provides efficient state management using Arc for sharing and cloning
#[derive(Debug, Clone)]
pub struct StateManager {
    state: SharedState,
}

impl StateManager {
    /// Create a new empty state manager
    pub fn new() -> Self {
        Self {
            state: Arc::new(HashMap::new()),
        }
    }

    /// Create a state manager from existing state
    pub fn from_state(state: SharedState) -> Self {
        Self { state }
    }

    /// Get the current state (immutable reference)
    pub fn state(&self) -> &SharedState {
        &self.state
    }

    /// Get a value from the state
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Update a value in the state (Copy-on-Write)
    /// Returns a new StateManager with the updated state
    pub fn set(&self, key: String, value: serde_json::Value) -> Self {
        let mut new_state = (*self.state).clone();
        new_state.insert(key, value);
        Self {
            state: Arc::new(new_state),
        }
    }

    /// Update multiple values in the state (Copy-on-Write)
    pub fn update(&self, updates: HashMap<String, serde_json::Value>) -> Self {
        let mut new_state = (*self.state).clone();
        for (key, value) in updates {
            new_state.insert(key, value);
        }
        Self {
            state: Arc::new(new_state),
        }
    }

    /// Remove a key from the state (Copy-on-Write)
    pub fn remove(&self, key: &str) -> Self {
        let mut new_state = (*self.state).clone();
        new_state.remove(key);
        Self {
            state: Arc::new(new_state),
        }
    }

    /// Check if a key exists in the state
    pub fn contains(&self, key: &str) -> bool {
        self.state.contains_key(key)
    }

    /// Get the number of keys in the state
    pub fn len(&self) -> usize {
        self.state.len()
    }

    /// Check if the state is empty
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// Clear all state (returns new empty StateManager)
    pub fn clear(&self) -> Self {
        Self::new()
    }

    /// Get all keys in the state
    pub fn keys(&self) -> Vec<&String> {
        self.state.keys().collect()
    }

    /// Merge with another state (Copy-on-Write)
    /// Values from other state take precedence
    pub fn merge(&self, other: &StateManager) -> Self {
        let mut new_state = (*self.state).clone();
        for (key, value) in other.state.iter() {
            new_state.insert(key.clone(), value.clone());
        }
        Self {
            state: Arc::new(new_state),
        }
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_creation() {
        let manager = StateManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_state_set_get() {
        let manager = StateManager::new();
        let manager = manager.set("key".to_string(), serde_json::json!("value"));

        assert_eq!(manager.get("key"), Some(&serde_json::json!("value")));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_state_update() {
        let manager = StateManager::new();
        let mut updates = HashMap::new();
        updates.insert("key1".to_string(), serde_json::json!("value1"));
        updates.insert("key2".to_string(), serde_json::json!("value2"));

        let manager = manager.update(updates);

        assert_eq!(manager.get("key1"), Some(&serde_json::json!("value1")));
        assert_eq!(manager.get("key2"), Some(&serde_json::json!("value2")));
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_state_remove() {
        let manager = StateManager::new();
        let manager = manager.set("key".to_string(), serde_json::json!("value"));
        assert!(manager.contains("key"));

        let manager = manager.remove("key");
        assert!(!manager.contains("key"));
        assert!(manager.is_empty());
    }

    #[test]
    fn test_state_merge() {
        let manager1 = StateManager::new();
        let manager1 = manager1.set("key1".to_string(), serde_json::json!("value1"));

        let manager2 = StateManager::new();
        let manager2 = manager2.set("key2".to_string(), serde_json::json!("value2"));

        let merged = manager1.merge(&manager2);

        assert_eq!(merged.get("key1"), Some(&serde_json::json!("value1")));
        assert_eq!(merged.get("key2"), Some(&serde_json::json!("value2")));
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_state_cow() {
        let manager1 = StateManager::new();
        let manager1 = manager1.set("key".to_string(), serde_json::json!("value1"));

        let manager2 = manager1.set("key".to_string(), serde_json::json!("value2"));

        // Original state unchanged
        assert_eq!(manager1.get("key"), Some(&serde_json::json!("value1")));
        // New state updated
        assert_eq!(manager2.get("key"), Some(&serde_json::json!("value2")));
    }
}
