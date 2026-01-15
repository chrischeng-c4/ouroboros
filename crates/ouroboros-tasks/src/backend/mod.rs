//! Result backend implementations
//!
//! Provides traits and implementations for storing task results.

use async_trait::async_trait;
use std::time::Duration;

use crate::{TaskError, TaskId, TaskResult, TaskState};

/// Trait for result backend implementations
#[async_trait]
pub trait ResultBackend: Send + Sync + 'static {
    /// Store task state
    async fn set_state(&self, task_id: &TaskId, state: TaskState) -> Result<(), TaskError>;

    /// Get task state
    async fn get_state(&self, task_id: &TaskId) -> Result<Option<TaskState>, TaskError>;

    /// Store task result
    async fn set_result(
        &self,
        task_id: &TaskId,
        result: TaskResult,
        ttl: Option<Duration>,
    ) -> Result<(), TaskError>;

    /// Get task result
    async fn get_result(&self, task_id: &TaskId) -> Result<Option<TaskResult>, TaskError>;

    /// Wait for task to complete (blocking poll)
    async fn wait_for_result(
        &self,
        task_id: &TaskId,
        timeout: Option<Duration>,
        poll_interval: Duration,
    ) -> Result<TaskResult, TaskError>;

    /// Delete result (cleanup)
    async fn delete(&self, task_id: &TaskId) -> Result<(), TaskError>;

    /// Get multiple results
    async fn get_many(&self, task_ids: &[TaskId]) -> Result<Vec<Option<TaskResult>>, TaskError>;

    /// Health check
    async fn health_check(&self) -> Result<(), TaskError>;
}

// Redis backend implementation
#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "redis")]
pub use redis::{RedisBackend, RedisBackendConfig};
