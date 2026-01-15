//! Task-specific error types

use ouroboros_common::DataBridgeError;
use std::time::Duration;
use thiserror::Error;

/// Task-specific error types
#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Broker error: {0}")]
    Broker(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid task ID: {0}")]
    InvalidTaskId(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Task revoked: {0}")]
    Revoked(String),

    #[error("Max retries exceeded: {0}")]
    MaxRetriesExceeded(String),

    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Rate limited, retry after {0:?}")]
    RateLimited(Duration),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<TaskError> for DataBridgeError {
    fn from(err: TaskError) -> Self {
        match err {
            TaskError::Broker(s) => DataBridgeError::Connection(s),
            TaskError::Backend(s) => DataBridgeError::Connection(s),
            TaskError::Connection(s) => DataBridgeError::Connection(s),
            TaskError::Serialization(s) => DataBridgeError::Serialization(s),
            TaskError::Deserialization(s) => DataBridgeError::Deserialization(s),
            TaskError::Configuration(s) => DataBridgeError::Internal(s),
            TaskError::NotConnected => DataBridgeError::Connection("Not connected".to_string()),
            _ => DataBridgeError::Internal(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for TaskError {
    fn from(err: serde_json::Error) -> Self {
        TaskError::Serialization(err.to_string())
    }
}

impl From<uuid::Error> for TaskError {
    fn from(err: uuid::Error) -> Self {
        TaskError::InvalidTaskId(err.to_string())
    }
}
