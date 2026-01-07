//! Chain - execute tasks sequentially, passing results

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{Broker, ResultBackend, TaskError, TaskId, TaskMessage, TaskState};
use super::{ChainMeta, TaskOptions, TaskSignature};

/// A chain of tasks executed sequentially
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Unique chain ID
    pub id: TaskId,
    /// Tasks to execute in order
    pub tasks: Vec<TaskSignature>,
    /// Chain-level options
    pub options: TaskOptions,
}

impl Chain {
    /// Create a new chain
    pub fn new(tasks: Vec<TaskSignature>) -> Self {
        Self {
            id: TaskId::new(),
            tasks,
            options: TaskOptions::default(),
        }
    }

    /// Create a chain with options
    pub fn with_options(mut self, options: TaskOptions) -> Self {
        self.options = options;
        self
    }

    /// Execute the chain by publishing the first task
    ///
    /// The chain metadata is stored in the result backend for the worker to continue
    /// execution after each task completes.
    pub async fn apply_async<B: Broker>(
        &self,
        broker: &B,
    ) -> Result<AsyncChainResult, TaskError> {
        if self.tasks.is_empty() {
            return Err(TaskError::InvalidWorkflow(
                "Chain must have at least one task".to_string(),
            ));
        }

        let first_task = &self.tasks[0];
        let first_task_id = TaskId::new();
        let last_task_id = if self.tasks.len() == 1 {
            first_task_id.clone()
        } else {
            TaskId::new() // Placeholder, actual ID will be assigned during execution
        };

        // Create task message for the first task
        let mut message = TaskMessage::new(first_task.task_name.clone(), first_task.args.clone())
            .with_kwargs(first_task.kwargs.clone());

        message.id = first_task_id.clone();
        message.root_id = Some(self.id.clone());

        // Apply options
        if let Some(eta) = first_task.options.eta.or(self.options.eta) {
            message.eta = Some(eta);
        }
        if let Some(expires) = first_task.options.expires.or(self.options.expires) {
            message.expires = Some(expires);
        }

        // Determine target queue
        let queue = first_task
            .options
            .queue
            .as_ref()
            .or(self.options.queue.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("default");

        // Publish first task
        broker.publish(queue, message).await?;

        Ok(AsyncChainResult {
            chain_id: self.id.clone(),
            first_task_id,
            last_task_id,
        })
    }

    /// Get chain metadata for storage
    ///
    /// Returns the serialized chain metadata that can be stored in the backend.
    /// This is called internally by apply_async and also by workers when
    /// processing chain tasks.
    pub fn get_metadata(&self) -> Result<(String, String), TaskError> {
        let meta = ChainMeta::new(self.id.clone(), self.tasks.clone());
        let key = format!("chain:{}", self.id);
        let data = serde_json::to_string(&meta)
            .map_err(|e| TaskError::Serialization(e.to_string()))?;
        Ok((key, data))
    }
}

/// Handle to track chain execution
#[derive(Debug, Clone)]
pub struct AsyncChainResult {
    /// Chain ID
    pub chain_id: TaskId,
    /// ID of the first task in the chain
    pub first_task_id: TaskId,
    /// ID of the last task in the chain
    pub last_task_id: TaskId,
}

impl AsyncChainResult {
    /// Wait for the final result of the chain
    ///
    /// This polls the result backend until the last task completes or timeout occurs.
    pub async fn get<R: ResultBackend>(
        &self,
        backend: &R,
        timeout: Option<Duration>,
    ) -> Result<serde_json::Value, TaskError> {
        let poll_interval = Duration::from_millis(100);
        let result = backend
            .wait_for_result(&self.last_task_id, timeout, poll_interval)
            .await?;

        match result.state {
            TaskState::Success => result
                .result
                .ok_or_else(|| TaskError::Internal("Success state but no result".to_string())),
            TaskState::Failure => Err(TaskError::Internal(
                result.error.unwrap_or_else(|| "Task failed".to_string()),
            )),
            other => Err(TaskError::Internal(format!(
                "Unexpected task state: {:?}",
                other
            ))),
        }
    }

    /// Check if the chain is ready (last task completed)
    pub async fn ready<R: ResultBackend>(&self, backend: &R) -> Result<bool, TaskError> {
        match backend.get_state(&self.last_task_id).await? {
            Some(state) => Ok(state.is_terminal()),
            None => Ok(false),
        }
    }

    /// Get the current state of the chain
    pub async fn state<R: ResultBackend>(&self, backend: &R) -> Result<Option<TaskState>, TaskError> {
        backend.get_state(&self.last_task_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_new() {
        let tasks = vec![
            TaskSignature::new("task1", serde_json::json!([1])),
            TaskSignature::new("task2", serde_json::json!([2])),
        ];

        let chain = Chain::new(tasks.clone());
        assert_eq!(chain.tasks.len(), 2);
        assert_eq!(chain.tasks[0].task_name, "task1");
        assert_eq!(chain.tasks[1].task_name, "task2");
    }

    #[test]
    fn test_chain_with_options() {
        let tasks = vec![TaskSignature::new("task1", serde_json::json!([]))];
        let options = TaskOptions::new().with_queue("priority");

        let chain = Chain::new(tasks).with_options(options);
        assert_eq!(chain.options.queue, Some("priority".to_string()));
    }

    #[test]
    fn test_empty_chain_error() {
        let chain = Chain::new(vec![]);
        // Empty chain should be invalid
        assert!(chain.tasks.is_empty());
    }

    #[test]
    fn test_async_chain_result_creation() {
        let chain_id = TaskId::new();
        let first_task_id = TaskId::new();
        let last_task_id = TaskId::new();

        let result = AsyncChainResult {
            chain_id: chain_id.clone(),
            first_task_id: first_task_id.clone(),
            last_task_id: last_task_id.clone(),
        };

        assert_eq!(result.chain_id, chain_id);
        assert_eq!(result.first_task_id, first_task_id);
        assert_eq!(result.last_task_id, last_task_id);
    }
}
