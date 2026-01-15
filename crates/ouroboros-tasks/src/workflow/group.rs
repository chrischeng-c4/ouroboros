//! Group - execute tasks in parallel

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{Broker, ResultBackend, TaskError, TaskId, TaskMessage, TaskState};
use super::{TaskOptions, TaskSignature};

/// A group of tasks executed in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Unique group ID
    pub id: TaskId,
    /// Tasks to execute in parallel
    pub tasks: Vec<TaskSignature>,
    /// Group-level options
    pub options: TaskOptions,
}

impl Group {
    /// Create a new group
    pub fn new(tasks: Vec<TaskSignature>) -> Self {
        Self {
            id: TaskId::new(),
            tasks,
            options: TaskOptions::default(),
        }
    }

    /// Create a group with options
    pub fn with_options(mut self, options: TaskOptions) -> Self {
        self.options = options;
        self
    }

    /// Execute all tasks in parallel by publishing them to the broker
    pub async fn apply_async<B: Broker>(
        &self,
        broker: &B,
    ) -> Result<GroupResult, TaskError> {
        if self.tasks.is_empty() {
            return Err(TaskError::InvalidWorkflow(
                "Group must have at least one task".to_string(),
            ));
        }

        let mut task_ids = Vec::with_capacity(self.tasks.len());

        // Publish all tasks
        for task_sig in &self.tasks {
            let task_id = TaskId::new();
            let mut message = TaskMessage::new(task_sig.task_name.clone(), task_sig.args.clone())
                .with_kwargs(task_sig.kwargs.clone());

            message.id = task_id.clone();
            message.root_id = Some(self.id.clone());
            message.parent_id = Some(self.id.clone());

            // Apply options (task-level overrides group-level)
            if let Some(eta) = task_sig.options.eta.or(self.options.eta) {
                message.eta = Some(eta);
            }
            if let Some(expires) = task_sig.options.expires.or(self.options.expires) {
                message.expires = Some(expires);
            }

            // Determine target queue
            let queue = task_sig
                .options
                .queue
                .as_ref()
                .or(self.options.queue.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("default");

            // Publish task
            broker.publish(queue, message).await?;

            task_ids.push(task_id);
        }

        Ok(GroupResult {
            group_id: self.id.clone(),
            task_ids,
        })
    }
}

/// Handle to track group execution
#[derive(Debug, Clone)]
pub struct GroupResult {
    /// Group ID
    pub group_id: TaskId,
    /// Task IDs in the group
    pub task_ids: Vec<TaskId>,
}

impl GroupResult {
    /// Wait for all tasks to complete and return their results
    pub async fn get<R: ResultBackend>(
        &self,
        backend: &R,
        timeout: Option<Duration>,
    ) -> Result<Vec<serde_json::Value>, TaskError> {
        let results = backend.get_many(&self.task_ids).await?;

        let mut final_results = Vec::with_capacity(self.task_ids.len());

        for (i, result_opt) in results.iter().enumerate() {
            match result_opt {
                Some(result) => match result.state {
                    TaskState::Success => {
                        final_results.push(
                            result
                                .result
                                .clone()
                                .ok_or_else(|| {
                                    TaskError::Internal(format!(
                                        "Task {} succeeded but has no result",
                                        self.task_ids[i]
                                    ))
                                })?,
                        );
                    }
                    TaskState::Failure => {
                        return Err(TaskError::Internal(
                            result
                                .error
                                .clone()
                                .unwrap_or_else(|| format!("Task {} failed", self.task_ids[i])),
                        ));
                    }
                    other => {
                        // If not complete, wait for it
                        let poll_interval = Duration::from_millis(100);
                        let task_result = backend
                            .wait_for_result(&self.task_ids[i], timeout, poll_interval)
                            .await?;

                        match task_result.state {
                            TaskState::Success => {
                                final_results.push(
                                    task_result.result.ok_or_else(|| {
                                        TaskError::Internal(format!(
                                            "Task {} succeeded but has no result",
                                            self.task_ids[i]
                                        ))
                                    })?,
                                );
                            }
                            TaskState::Failure => {
                                return Err(TaskError::Internal(
                                    task_result.error.unwrap_or_else(|| {
                                        format!("Task {} failed", self.task_ids[i])
                                    }),
                                ));
                            }
                            _ => {
                                return Err(TaskError::Internal(format!(
                                    "Task {} in unexpected state: {:?}",
                                    self.task_ids[i], other
                                )));
                            }
                        }
                    }
                },
                None => {
                    // Task result not found, wait for it
                    let poll_interval = Duration::from_millis(100);
                    let task_result = backend
                        .wait_for_result(&self.task_ids[i], timeout, poll_interval)
                        .await?;

                    match task_result.state {
                        TaskState::Success => {
                            final_results.push(
                                task_result
                                    .result
                                    .ok_or_else(|| {
                                        TaskError::Internal(format!(
                                            "Task {} succeeded but has no result",
                                            self.task_ids[i]
                                        ))
                                    })?,
                            );
                        }
                        TaskState::Failure => {
                            return Err(TaskError::Internal(
                                task_result
                                    .error
                                    .unwrap_or_else(|| format!("Task {} failed", self.task_ids[i])),
                            ));
                        }
                        other => {
                            return Err(TaskError::Internal(format!(
                                "Task {} in unexpected state: {:?}",
                                self.task_ids[i], other
                            )));
                        }
                    }
                }
            }
        }

        Ok(final_results)
    }

    /// Check if all tasks are ready (completed)
    pub async fn ready<R: ResultBackend>(&self, backend: &R) -> Result<bool, TaskError> {
        let states = backend.get_many(&self.task_ids).await?;

        for state_opt in states {
            match state_opt {
                Some(result) => {
                    if !result.state.is_terminal() {
                        return Ok(false);
                    }
                }
                None => return Ok(false),
            }
        }

        Ok(true)
    }

    /// Get all results that are currently available (non-blocking)
    pub async fn get_ready<R: ResultBackend>(
        &self,
        backend: &R,
    ) -> Result<Vec<Option<serde_json::Value>>, TaskError> {
        let results = backend.get_many(&self.task_ids).await?;

        let mut ready_results = Vec::with_capacity(self.task_ids.len());

        for result_opt in results {
            match result_opt {
                Some(result) => match result.state {
                    TaskState::Success => ready_results.push(result.result),
                    _ => ready_results.push(None),
                },
                None => ready_results.push(None),
            }
        }

        Ok(ready_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_new() {
        let tasks = vec![
            TaskSignature::new("task1", serde_json::json!([1])),
            TaskSignature::new("task2", serde_json::json!([2])),
            TaskSignature::new("task3", serde_json::json!([3])),
        ];

        let group = Group::new(tasks.clone());
        assert_eq!(group.tasks.len(), 3);
        assert_eq!(group.tasks[0].task_name, "task1");
    }

    #[test]
    fn test_group_with_options() {
        let tasks = vec![TaskSignature::new("task1", serde_json::json!([]))];
        let options = TaskOptions::new().with_queue("bulk");

        let group = Group::new(tasks).with_options(options);
        assert_eq!(group.options.queue, Some("bulk".to_string()));
    }

    #[test]
    fn test_empty_group_error() {
        let group = Group::new(vec![]);
        assert!(group.tasks.is_empty());
    }

    #[test]
    fn test_group_result_creation() {
        let group_id = TaskId::new();
        let task_ids = vec![TaskId::new(), TaskId::new()];

        let result = GroupResult {
            group_id: group_id.clone(),
            task_ids: task_ids.clone(),
        };

        assert_eq!(result.group_id, group_id);
        assert_eq!(result.task_ids.len(), 2);
    }
}
