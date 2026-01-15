//! Task state machine

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::TaskId;

/// Task state in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    /// Task is waiting to be picked up
    #[default]
    Pending,
    /// Task has been received by a worker
    Received,
    /// Task is being executed
    Started,
    /// Task completed successfully
    Success,
    /// Task failed permanently
    Failure,
    /// Task is being retried
    Retry,
    /// Task was revoked/cancelled
    Revoked,
    /// Task was rejected (invalid)
    Rejected,
}

impl TaskState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Failure | Self::Revoked | Self::Rejected)
    }

    /// Check if task is in progress
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Received | Self::Started | Self::Retry)
    }

    /// Valid state transitions
    pub fn can_transition_to(&self, next: TaskState) -> bool {
        match (self, next) {
            // From PENDING
            (Self::Pending, Self::Received) => true,
            (Self::Pending, Self::Revoked) => true,

            // From RECEIVED
            (Self::Received, Self::Started) => true,
            (Self::Received, Self::Revoked) => true,
            (Self::Received, Self::Rejected) => true,

            // From STARTED
            (Self::Started, Self::Success) => true,
            (Self::Started, Self::Failure) => true,
            (Self::Started, Self::Retry) => true,
            (Self::Started, Self::Revoked) => true,

            // From RETRY
            (Self::Retry, Self::Pending) => true,
            (Self::Retry, Self::Received) => true,
            (Self::Retry, Self::Failure) => true,

            _ => false,
        }
    }
}

/// Full task result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TaskResult {
    pub task_id: TaskId,
    pub state: TaskState,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub traceback: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub runtime_ms: Option<u64>,
    pub retries: u32,
    pub worker_id: Option<String>,
}

impl TaskResult {
    /// Create a new pending result
    pub fn pending(task_id: TaskId) -> Self {
        Self {
            task_id,
            state: TaskState::Pending,
            result: None,
            error: None,
            traceback: None,
            started_at: None,
            completed_at: None,
            runtime_ms: None,
            retries: 0,
            worker_id: None,
        }
    }

    /// Create a success result
    pub fn success(task_id: TaskId, value: serde_json::Value) -> Self {
        Self {
            task_id,
            state: TaskState::Success,
            result: Some(value),
            error: None,
            traceback: None,
            started_at: None,
            completed_at: Some(Utc::now()),
            runtime_ms: None,
            retries: 0,
            worker_id: None,
        }
    }

    /// Create a failure result
    pub fn failure(task_id: TaskId, error: String) -> Self {
        Self {
            task_id,
            state: TaskState::Failure,
            result: None,
            error: Some(error),
            traceback: None,
            started_at: None,
            completed_at: Some(Utc::now()),
            runtime_ms: None,
            retries: 0,
            worker_id: None,
        }
    }
}
