use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// HMR message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum HmrMessage {
    /// Module update
    Update { path: String, timestamp: u64 },

    /// Full reload required
    FullReload { reason: String },

    /// Connected confirmation
    Connected,

    /// Error occurred
    Error { message: String },
}

/// HMR manager for broadcasting updates
pub struct HmrManager {
    tx: broadcast::Sender<HmrMessage>,
}

impl HmrManager {
    /// Create a new HMR manager
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast an HMR message to all connected clients
    pub async fn broadcast(&self, message: HmrMessage) {
        let _ = self.tx.send(message);
    }

    /// Subscribe to HMR messages
    pub fn subscribe(&self) -> broadcast::Receiver<HmrMessage> {
        self.tx.subscribe()
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for HmrManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmr_manager_creation() {
        let manager = HmrManager::new();
        assert_eq!(manager.subscriber_count(), 0);
    }

    #[test]
    fn test_subscribe() {
        let manager = HmrManager::new();
        let _rx = manager.subscribe();
        assert_eq!(manager.subscriber_count(), 1);
    }
}
