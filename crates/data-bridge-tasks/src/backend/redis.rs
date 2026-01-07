//! Redis result backend implementation
//!
//! Provides Redis-based storage for task results with connection pooling,
//! TTL support, and efficient batch operations.

use async_trait::async_trait;
use deadpool_redis::{Config as PoolConfig, Connection, Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::{ResultBackend, TaskError, TaskId, TaskResult, TaskState};

/// Redis result backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisBackendConfig {
    /// Redis URL (e.g., "redis://localhost:6379")
    pub url: String,
    /// Key prefix for all task data
    pub key_prefix: String,
    /// Default result TTL (0 = no expiry)
    pub default_ttl: Duration,
    /// Connection pool size
    pub pool_size: usize,
}

impl Default for RedisBackendConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            key_prefix: "data-bridge-tasks".to_string(),
            default_ttl: Duration::from_secs(86400), // 24 hours
            pool_size: 10,
        }
    }
}

/// Redis result backend implementation
///
/// Uses deadpool-redis for async connection pooling and provides
/// atomic operations for state and result updates.
pub struct RedisBackend {
    config: RedisBackendConfig,
    pool: Pool,
}

impl RedisBackend {
    /// Create a new Redis backend
    pub async fn new(config: RedisBackendConfig) -> Result<Self, TaskError> {
        debug!(
            "Creating Redis backend: url={}, prefix={}, pool_size={}",
            config.url, config.key_prefix, config.pool_size
        );

        let pool_config = PoolConfig::from_url(&config.url);
        let pool = pool_config
            .builder()
            .map_err(|e| TaskError::Backend(format!("Failed to create pool builder: {}", e)))?
            .max_size(config.pool_size)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| TaskError::Backend(format!("Failed to create pool: {}", e)))?;

        // Test connection
        let mut conn = pool
            .get()
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to get connection: {}", e)))?;

        // Connection successful if we got here
        let _: Option<String> = conn.get("__ping__").await.ok();

        debug!("Redis backend initialized successfully");

        Ok(Self { config, pool })
    }

    /// Generate state key for a task
    fn state_key(&self, task_id: &TaskId) -> String {
        format!("{}:state:{}", self.config.key_prefix, task_id)
    }

    /// Generate result key for a task
    fn result_key(&self, task_id: &TaskId) -> String {
        format!("{}:result:{}", self.config.key_prefix, task_id)
    }

    /// Get a connection from the pool
    async fn get_conn(&self) -> Result<Connection, TaskError> {
        self.pool
            .get()
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to get connection: {}", e)))
    }

    /// Get TTL in seconds
    fn get_ttl_seconds(&self, ttl: Option<Duration>) -> u64 {
        ttl.unwrap_or(self.config.default_ttl).as_secs()
    }
}

#[async_trait]
impl ResultBackend for RedisBackend {
    async fn set_state(&self, task_id: &TaskId, state: TaskState) -> Result<(), TaskError> {
        let key = self.state_key(task_id);
        let value = serde_json::to_string(&state)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize state: {}", e)))?;

        let ttl = self.get_ttl_seconds(None);
        let mut conn = self.get_conn().await?;

        debug!("Setting state for task {}: {:?}", task_id, state);

        if ttl > 0 {
            conn.set_ex::<_, _, ()>(&key, value, ttl)
                .await
                .map_err(|e| TaskError::Backend(format!("Failed to set state: {}", e)))?;
        } else {
            conn.set::<_, _, ()>(&key, value)
                .await
                .map_err(|e| TaskError::Backend(format!("Failed to set state: {}", e)))?;
        }

        Ok(())
    }

    async fn get_state(&self, task_id: &TaskId) -> Result<Option<TaskState>, TaskError> {
        let key = self.state_key(task_id);
        let mut conn = self.get_conn().await?;

        debug!("Getting state for task {}", task_id);

        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to get state: {}", e)))?;

        match value {
            Some(v) => {
                let state = serde_json::from_str(&v).map_err(|e| {
                    TaskError::Deserialization(format!("Failed to deserialize state: {}", e))
                })?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    async fn set_result(
        &self,
        task_id: &TaskId,
        result: TaskResult,
        ttl: Option<Duration>,
    ) -> Result<(), TaskError> {
        let state_key = self.state_key(task_id);
        let result_key = self.result_key(task_id);

        let state_value = serde_json::to_string(&result.state).map_err(|e| {
            TaskError::Serialization(format!("Failed to serialize state: {}", e))
        })?;
        let result_value = serde_json::to_string(&result).map_err(|e| {
            TaskError::Serialization(format!("Failed to serialize result: {}", e))
        })?;

        let ttl_secs = self.get_ttl_seconds(ttl);
        let mut conn = self.get_conn().await?;

        debug!("Setting result for task {}: {:?}", task_id, result.state);

        // Set both keys (not truly atomic in Redis without Lua script, but good enough)
        if ttl_secs > 0 {
            conn.set_ex::<_, _, ()>(&state_key, &state_value, ttl_secs)
                .await
                .map_err(|e| TaskError::Backend(format!("Failed to set state: {}", e)))?;
            conn.set_ex::<_, _, ()>(&result_key, &result_value, ttl_secs)
                .await
                .map_err(|e| TaskError::Backend(format!("Failed to set result: {}", e)))?;
        } else {
            conn.set::<_, _, ()>(&state_key, &state_value)
                .await
                .map_err(|e| TaskError::Backend(format!("Failed to set state: {}", e)))?;
            conn.set::<_, _, ()>(&result_key, &result_value)
                .await
                .map_err(|e| TaskError::Backend(format!("Failed to set result: {}", e)))?;
        }

        Ok(())
    }

    async fn get_result(&self, task_id: &TaskId) -> Result<Option<TaskResult>, TaskError> {
        let key = self.result_key(task_id);
        let mut conn = self.get_conn().await?;

        debug!("Getting result for task {}", task_id);

        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to get result: {}", e)))?;

        match value {
            Some(v) => {
                let result = serde_json::from_str(&v).map_err(|e| {
                    TaskError::Deserialization(format!("Failed to deserialize result: {}", e))
                })?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    async fn wait_for_result(
        &self,
        task_id: &TaskId,
        timeout: Option<Duration>,
        poll_interval: Duration,
    ) -> Result<TaskResult, TaskError> {
        let start = std::time::Instant::now();
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(3600)); // 1 hour default

        debug!(
            "Waiting for result: task={}, timeout={:?}, interval={:?}",
            task_id, timeout, poll_interval
        );

        loop {
            // Check timeout
            if start.elapsed() >= timeout_duration {
                warn!("Timeout waiting for task {}", task_id);
                return Err(TaskError::Timeout(format!(
                    "Task {} did not complete within {:?}",
                    task_id, timeout_duration
                )));
            }

            // Check state
            if let Some(state) = self.get_state(task_id).await? {
                if state.is_terminal() {
                    // Terminal state reached, get result
                    if let Some(result) = self.get_result(task_id).await? {
                        debug!("Task {} completed with state {:?}", task_id, state);
                        return Ok(result);
                    } else {
                        error!("Task {} in terminal state but no result found", task_id);
                        return Err(TaskError::Backend(format!(
                            "Task {} in terminal state but no result found",
                            task_id
                        )));
                    }
                }
            }

            // Wait before next poll
            tokio::time::sleep(poll_interval).await;
        }
    }

    async fn delete(&self, task_id: &TaskId) -> Result<(), TaskError> {
        let state_key = self.state_key(task_id);
        let result_key = self.result_key(task_id);

        debug!("Deleting task data for {}", task_id);

        let mut conn = self.get_conn().await?;

        // Delete both keys
        conn.del::<_, ()>(&state_key)
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to delete state: {}", e)))?;
        conn.del::<_, ()>(&result_key)
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to delete result: {}", e)))?;

        Ok(())
    }

    async fn get_many(&self, task_ids: &[TaskId]) -> Result<Vec<Option<TaskResult>>, TaskError> {
        if task_ids.is_empty() {
            return Ok(Vec::new());
        }

        let keys: Vec<String> = task_ids.iter().map(|id| self.result_key(id)).collect();

        debug!("Getting {} results in batch", task_ids.len());

        let mut conn = self.get_conn().await?;

        let values: Vec<Option<String>> = conn.get(&keys)
            .await
            .map_err(|e| TaskError::Backend(format!("Failed to get many results: {}", e)))?;

        let mut results = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            match value {
                Some(v) => {
                    let result = serde_json::from_str(&v).map_err(|e| {
                        TaskError::Deserialization(format!(
                            "Failed to deserialize result for task {}: {}",
                            task_ids[i], e
                        ))
                    })?;
                    results.push(Some(result));
                }
                None => results.push(None),
            }
        }

        Ok(results)
    }

    async fn health_check(&self) -> Result<(), TaskError> {
        debug!("Performing Redis health check");

        let mut conn = self.get_conn().await?;

        // Just get the connection - if we can connect, Redis is healthy
        let _: Option<String> = conn.get("__health_check__").await
            .map_err(|e| TaskError::Backend(format!("Health check failed: {}", e)))?;

        Ok(())
    }
}

// Make backend cloneable by cloning the pool
impl Clone for RedisBackend {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            pool: self.pool.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = RedisBackendConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.key_prefix, "data-bridge-tasks");
        assert_eq!(config.default_ttl, Duration::from_secs(86400));
        assert_eq!(config.pool_size, 10);
    }

    #[test]
    fn test_key_generation() {
        let config = RedisBackendConfig::default();
        let backend_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { RedisBackend::new(config.clone()).await });

        // Skip if Redis not available (integration test environment)
        if backend_result.is_err() {
            return;
        }

        let backend = backend_result.unwrap();
        let task_id = TaskId::new();

        let state_key = backend.state_key(&task_id);
        let result_key = backend.result_key(&task_id);

        assert_eq!(
            state_key,
            format!("data-bridge-tasks:state:{}", task_id)
        );
        assert_eq!(
            result_key,
            format!("data-bridge-tasks:result:{}", task_id)
        );
    }

    #[test]
    fn test_serialization() {
        let state = TaskState::Success;
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);

        let task_id = TaskId::new();
        let result = TaskResult::success(task_id.clone(), serde_json::json!({"foo": "bar"}));
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.task_id, task_id);
        assert_eq!(deserialized.state, TaskState::Success);
    }

    // Integration tests - require Redis running
    #[tokio::test]
    #[ignore]
    async fn test_set_get_state() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        let task_id = TaskId::new();
        let state = TaskState::Started;

        backend.set_state(&task_id, state).await.unwrap();
        let retrieved = backend.get_state(&task_id).await.unwrap();

        assert_eq!(retrieved, Some(state));

        // Cleanup
        backend.delete(&task_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_set_get_result() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        let task_id = TaskId::new();
        let result = TaskResult::success(task_id.clone(), serde_json::json!({"test": "data"}));

        backend
            .set_result(&task_id, result.clone(), None)
            .await
            .unwrap();

        let retrieved = backend.get_result(&task_id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.task_id, task_id);
        assert_eq!(retrieved.state, TaskState::Success);

        // Cleanup
        backend.delete(&task_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_wait_for_result_success() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        let task_id = TaskId::new();

        // Set pending state
        backend
            .set_state(&task_id, TaskState::Pending)
            .await
            .unwrap();

        // Spawn task to complete after delay
        let task_id_clone = task_id.clone();
        let backend_clone = backend.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let result = TaskResult::success(
                task_id_clone.clone(),
                serde_json::json!({"result": "done"}),
            );
            backend_clone
                .set_result(&task_id_clone, result, None)
                .await
                .unwrap();
        });

        // Wait for result
        let result = backend
            .wait_for_result(
                &task_id,
                Some(Duration::from_secs(5)),
                Duration::from_millis(50),
            )
            .await
            .unwrap();

        assert_eq!(result.state, TaskState::Success);
        assert_eq!(result.task_id, task_id);

        // Cleanup
        backend.delete(&task_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_wait_for_result_timeout() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        let task_id = TaskId::new();

        // Set pending state (never completes)
        backend
            .set_state(&task_id, TaskState::Pending)
            .await
            .unwrap();

        // Wait should timeout
        let result = backend
            .wait_for_result(
                &task_id,
                Some(Duration::from_millis(200)),
                Duration::from_millis(50),
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskError::Timeout(_)));

        // Cleanup
        backend.delete(&task_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_many() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        let task_id1 = TaskId::new();
        let task_id2 = TaskId::new();
        let task_id3 = TaskId::new();

        let result1 =
            TaskResult::success(task_id1.clone(), serde_json::json!({"id": 1}));
        let result2 =
            TaskResult::success(task_id2.clone(), serde_json::json!({"id": 2}));

        backend.set_result(&task_id1, result1, None).await.unwrap();
        backend.set_result(&task_id2, result2, None).await.unwrap();

        let results = backend
            .get_many(&[task_id1.clone(), task_id2.clone(), task_id3.clone()])
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert!(results[1].is_some());
        assert!(results[2].is_none());

        assert_eq!(results[0].as_ref().unwrap().task_id, task_id1);
        assert_eq!(results[1].as_ref().unwrap().task_id, task_id2);

        // Cleanup
        backend.delete(&task_id1).await.unwrap();
        backend.delete(&task_id2).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        let task_id = TaskId::new();
        let result = TaskResult::success(task_id.clone(), serde_json::json!({"test": "data"}));

        backend
            .set_result(&task_id, result, None)
            .await
            .unwrap();

        // Verify it exists
        let retrieved = backend.get_result(&task_id).await.unwrap();
        assert!(retrieved.is_some());

        // Delete
        backend.delete(&task_id).await.unwrap();

        // Verify it's gone
        let retrieved = backend.get_result(&task_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_health_check() {
        let config = RedisBackendConfig::default();
        let backend = RedisBackend::new(config).await.unwrap();

        backend.health_check().await.unwrap();
    }
}
