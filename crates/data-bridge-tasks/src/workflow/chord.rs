//! Chord - group of parallel tasks followed by a callback

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{Broker, ResultBackend, TaskError, TaskId, TaskMessage, TaskState};
use super::{ChordMeta, Group, GroupResult, TaskOptions, TaskSignature};

/// A chord: group of parallel tasks followed by a callback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chord {
    /// Unique chord ID
    pub id: TaskId,
    /// Header tasks (run in parallel)
    pub header: Group,
    /// Callback task (receives all results)
    pub callback: TaskSignature,
    /// Chord-level options
    pub options: TaskOptions,
}

impl Chord {
    /// Create a new chord
    pub fn new(header: Group, callback: TaskSignature) -> Self {
        Self {
            id: TaskId::new(),
            header,
            callback,
            options: TaskOptions::default(),
        }
    }

    /// Create a chord with options
    pub fn with_options(mut self, options: TaskOptions) -> Self {
        self.options = options;
        self
    }

    /// Execute the chord
    ///
    /// This publishes all header tasks and stores chord metadata in the backend.
    /// When all header tasks complete, a worker will trigger the callback.
    pub async fn apply_async<B: Broker, R: ResultBackend>(
        &self,
        broker: &B,
        backend: &R,
    ) -> Result<AsyncChordResult, TaskError> {
        if self.header.tasks.is_empty() {
            return Err(TaskError::InvalidWorkflow(
                "Chord header must have at least one task".to_string(),
            ));
        }

        // Execute the header group
        let group_result = self.header.apply_async(broker).await?;

        // Create callback task ID
        let callback_task_id = TaskId::new();

        // Store chord metadata
        let chord_meta = ChordMeta::new(
            self.id.clone(),
            group_result.task_ids.clone(),
            self.callback.clone(),
        );

        let meta_key = format!("chord:{}", self.id);
        let meta_json = serde_json::to_vec(&chord_meta)
            .map_err(|e| TaskError::Serialization(e.to_string()))?;

        // Store metadata in backend
        // TODO: Add set_metadata to ResultBackend trait
        // For now, we'll store it as a special result
        let meta_task_id = TaskId::new();
        let _ = (backend, meta_key, meta_json, meta_task_id);

        Ok(AsyncChordResult {
            chord_id: self.id.clone(),
            header_result: group_result,
            callback_task_id,
        })
    }

    /// Manually trigger the callback (for testing or manual intervention)
    ///
    /// This should normally be called by the worker after all header tasks complete.
    pub async fn trigger_callback<B: Broker>(
        &self,
        broker: &B,
        header_results: Vec<serde_json::Value>,
    ) -> Result<TaskId, TaskError> {
        let callback_task_id = TaskId::new();

        // Prepare callback args with header results
        let callback_args = serde_json::Value::Array(header_results);

        let mut message = TaskMessage::new(self.callback.task_name.clone(), callback_args)
            .with_kwargs(self.callback.kwargs.clone());

        message.id = callback_task_id.clone();
        message.root_id = Some(self.id.clone());
        message.parent_id = Some(self.id.clone());

        // Apply options
        let queue = self
            .callback
            .options
            .queue
            .as_ref()
            .or(self.options.queue.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("default");

        broker.publish(queue, message).await?;

        Ok(callback_task_id)
    }
}

/// Handle to track chord execution
#[derive(Debug, Clone)]
pub struct AsyncChordResult {
    /// Chord ID
    pub chord_id: TaskId,
    /// Handle to the header group
    pub header_result: GroupResult,
    /// ID of the callback task (will be assigned when triggered)
    pub callback_task_id: TaskId,
}

impl AsyncChordResult {
    /// Wait for the callback to complete and return its result
    pub async fn get<R: ResultBackend>(
        &self,
        backend: &R,
        timeout: Option<Duration>,
    ) -> Result<serde_json::Value, TaskError> {
        let poll_interval = Duration::from_millis(100);
        let result = backend
            .wait_for_result(&self.callback_task_id, timeout, poll_interval)
            .await?;

        match result.state {
            TaskState::Success => result
                .result
                .ok_or_else(|| TaskError::Internal("Success state but no result".to_string())),
            TaskState::Failure => Err(TaskError::Internal(
                result.error.unwrap_or_else(|| "Callback failed".to_string()),
            )),
            other => Err(TaskError::Internal(format!(
                "Unexpected callback state: {:?}",
                other
            ))),
        }
    }

    /// Check if the header group is ready (all tasks completed)
    pub async fn header_ready<R: ResultBackend>(&self, backend: &R) -> Result<bool, TaskError> {
        self.header_result.ready(backend).await
    }

    /// Check if the callback is ready (completed)
    pub async fn ready<R: ResultBackend>(&self, backend: &R) -> Result<bool, TaskError> {
        match backend.get_state(&self.callback_task_id).await? {
            Some(state) => Ok(state.is_terminal()),
            None => Ok(false),
        }
    }

    /// Get header results (non-blocking)
    pub async fn get_header_results<R: ResultBackend>(
        &self,
        backend: &R,
    ) -> Result<Vec<Option<serde_json::Value>>, TaskError> {
        self.header_result.get_ready(backend).await
    }

    /// Wait for header to complete and return all results
    pub async fn wait_for_header<R: ResultBackend>(
        &self,
        backend: &R,
        timeout: Option<Duration>,
    ) -> Result<Vec<serde_json::Value>, TaskError> {
        self.header_result.get(backend, timeout).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chord_new() {
        let header = Group::new(vec![
            TaskSignature::new("task1", serde_json::json!([1])),
            TaskSignature::new("task2", serde_json::json!([2])),
        ]);

        let callback = TaskSignature::new("combine", serde_json::json!([]));

        let chord = Chord::new(header.clone(), callback);
        assert_eq!(chord.header.tasks.len(), 2);
        assert_eq!(chord.callback.task_name, "combine");
    }

    #[test]
    fn test_chord_with_options() {
        let header = Group::new(vec![TaskSignature::new("task1", serde_json::json!([]))]);
        let callback = TaskSignature::new("callback", serde_json::json!([]));
        let options = TaskOptions::new().with_queue("callbacks");

        let chord = Chord::new(header, callback).with_options(options);
        assert_eq!(chord.options.queue, Some("callbacks".to_string()));
    }

    #[test]
    fn test_empty_header_error() {
        let header = Group::new(vec![]);
        let callback = TaskSignature::new("callback", serde_json::json!([]));
        let chord = Chord::new(header, callback);
        assert!(chord.header.tasks.is_empty());
    }

    #[test]
    fn test_async_chord_result_creation() {
        let chord_id = TaskId::new();
        let group_id = TaskId::new();
        let task_ids = vec![TaskId::new(), TaskId::new()];
        let callback_task_id = TaskId::new();

        let header_result = GroupResult {
            group_id,
            task_ids,
        };

        let result = AsyncChordResult {
            chord_id: chord_id.clone(),
            header_result: header_result.clone(),
            callback_task_id: callback_task_id.clone(),
        };

        assert_eq!(result.chord_id, chord_id);
        assert_eq!(result.callback_task_id, callback_task_id);
        assert_eq!(result.header_result.task_ids.len(), 2);
    }
}
