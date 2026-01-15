//! Task message format for broker communication

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::TaskId;

/// Task message sent through the broker
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TaskMessage {
    /// Unique task identifier
    pub id: TaskId,
    /// Name of the task to execute
    pub task_name: String,
    /// Positional arguments (JSON array)
    pub args: serde_json::Value,
    /// Keyword arguments (JSON object)
    pub kwargs: serde_json::Value,
    /// Number of retry attempts so far
    pub retries: u32,
    /// Earliest time to execute (for delayed tasks)
    pub eta: Option<DateTime<Utc>>,
    /// Task expiration time
    pub expires: Option<DateTime<Utc>>,
    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
    /// Parent task ID (for chains)
    pub parent_id: Option<TaskId>,
    /// Root task ID (for workflows)
    pub root_id: Option<TaskId>,
}

impl TaskMessage {
    /// Create a new task message
    pub fn new(task_name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: TaskId::new(),
            task_name: task_name.into(),
            args,
            kwargs: serde_json::Value::Null,
            retries: 0,
            eta: None,
            expires: None,
            correlation_id: None,
            parent_id: None,
            root_id: None,
        }
    }

    /// Set keyword arguments
    pub fn with_kwargs(mut self, kwargs: serde_json::Value) -> Self {
        self.kwargs = kwargs;
        self
    }

    /// Set ETA for delayed execution
    pub fn with_eta(mut self, eta: DateTime<Utc>) -> Self {
        self.eta = Some(eta);
        self
    }

    /// Set expiration time
    pub fn with_expires(mut self, expires: DateTime<Utc>) -> Self {
        self.expires = Some(expires);
        self
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set parent task ID
    pub fn with_parent(mut self, parent_id: TaskId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set root task ID
    pub fn with_root(mut self, root_id: TaskId) -> Self {
        self.root_id = Some(root_id);
        self
    }

    /// Check if the task has expired
    pub fn is_expired(&self) -> bool {
        self.expires.map(|e| e < Utc::now()).unwrap_or(false)
    }

    /// Check if the task is ready to execute (ETA passed)
    pub fn is_ready(&self) -> bool {
        self.eta.map(|e| e <= Utc::now()).unwrap_or(true)
    }

    /// Increment retry count and return new message
    pub fn for_retry(mut self) -> Self {
        self.retries += 1;
        self.eta = None; // Clear ETA for immediate retry (delay handled separately)
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_message() {
        let msg = TaskMessage::new("my_task", serde_json::json!([1, 2, 3]));
        assert_eq!(msg.task_name, "my_task");
        assert_eq!(msg.args, serde_json::json!([1, 2, 3]));
        assert_eq!(msg.retries, 0);
        assert!(msg.is_ready());
        assert!(!msg.is_expired());
    }

    #[test]
    fn test_delayed_message() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let msg = TaskMessage::new("delayed_task", serde_json::json!([])).with_eta(future);
        assert!(!msg.is_ready());
    }

    #[test]
    fn test_retry() {
        let msg = TaskMessage::new("retry_task", serde_json::json!([]));
        assert_eq!(msg.retries, 0);
        let msg = msg.for_retry();
        assert_eq!(msg.retries, 1);
        let msg = msg.for_retry();
        assert_eq!(msg.retries, 2);
    }
}
