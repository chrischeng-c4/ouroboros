//! Background Tasks for post-response processing
//!
//! This module provides a FastAPI-like `BackgroundTasks` API for queuing
//! work to be executed after the HTTP response is sent.
//!
//! # Example
//!
//! ```rust,ignore
//! use ouroboros_api::background_tasks::BackgroundTasks;
//!
//! async fn send_email(to: String, subject: String) {
//!     // Email sending logic
//! }
//!
//! async fn handler(tasks: &mut BackgroundTasks) -> Response {
//!     tasks.add(|| async move {
//!         send_email("user@example.com".into(), "Welcome!".into()).await;
//!     });
//!     Response::ok()
//! }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Boxed task future type
pub type BoxedTask = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Background task collection for post-response execution
///
/// Tasks are queued during request handling and executed after
/// the response is sent to the client.
#[derive(Default)]
pub struct BackgroundTasks {
    /// Queued tasks
    tasks: Vec<BoxedTask>,
}

impl BackgroundTasks {
    /// Create a new empty task collection
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Add a task to be executed after the response
    ///
    /// The task will run asynchronously and won't block the response.
    pub fn add<F, Fut>(&mut self, task: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.tasks.push(Box::pin(async move {
            task().await;
        }));
    }

    /// Add a pre-boxed task
    pub fn add_boxed(&mut self, task: BoxedTask) {
        self.tasks.push(task);
    }

    /// Check if there are any tasks queued
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get the number of queued tasks
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Execute all queued tasks
    ///
    /// This should be called after the response is sent.
    /// Tasks are spawned as Tokio tasks and run concurrently.
    pub fn execute(self) -> Vec<JoinHandle<()>> {
        self.tasks
            .into_iter()
            .map(|task| tokio::spawn(task))
            .collect()
    }

    /// Execute all tasks and wait for completion
    ///
    /// Useful for testing or when you need to ensure all tasks complete.
    pub async fn execute_and_wait(self) {
        let handles = self.execute();
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Execute tasks with error handling
    ///
    /// Returns a list of task results (Ok or panic message).
    pub async fn execute_with_results(self) -> Vec<Result<(), String>> {
        let handles = self.execute();
        let mut results = Vec::with_capacity(handles.len());

        for handle in handles {
            match handle.await {
                Ok(()) => results.push(Ok(())),
                Err(e) => results.push(Err(format!("Task panicked: {}", e))),
            }
        }

        results
    }

    /// Take all tasks out of the collection
    pub fn take(&mut self) -> Vec<BoxedTask> {
        std::mem::take(&mut self.tasks)
    }
}

impl std::fmt::Debug for BackgroundTasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundTasks")
            .field("task_count", &self.tasks.len())
            .finish()
    }
}

// ============================================================================
// Thread-safe Background Tasks
// ============================================================================

/// Thread-safe background task collection
///
/// Use this when tasks need to be added from multiple threads/handlers.
#[derive(Clone, Default)]
pub struct SharedBackgroundTasks {
    tasks: Arc<Mutex<Vec<BoxedTask>>>,
}

impl SharedBackgroundTasks {
    /// Create a new shared task collection
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a task to the collection
    pub async fn add<F, Fut>(&self, task: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self.tasks.lock().await;
        tasks.push(Box::pin(async move {
            task().await;
        }));
    }

    /// Check if there are any tasks
    pub async fn is_empty(&self) -> bool {
        self.tasks.lock().await.is_empty()
    }

    /// Get task count
    pub async fn len(&self) -> usize {
        self.tasks.lock().await.len()
    }

    /// Execute all tasks
    pub async fn execute(&self) -> Vec<JoinHandle<()>> {
        let tasks = std::mem::take(&mut *self.tasks.lock().await);
        tasks
            .into_iter()
            .map(|task| tokio::spawn(task))
            .collect()
    }

    /// Execute and wait for all tasks
    pub async fn execute_and_wait(&self) {
        let handles = self.execute().await;
        for handle in handles {
            let _ = handle.await;
        }
    }
}

impl std::fmt::Debug for SharedBackgroundTasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedBackgroundTasks").finish()
    }
}

// ============================================================================
// Task Builder
// ============================================================================

/// Builder for creating background tasks with configuration
pub struct TaskBuilder {
    name: Option<String>,
    retry_count: u32,
    timeout_ms: Option<u64>,
}

impl TaskBuilder {
    /// Create a new task builder
    pub fn new() -> Self {
        Self {
            name: None,
            retry_count: 0,
            timeout_ms: None,
        }
    }

    /// Set task name for logging/debugging
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set number of retry attempts on failure
    pub fn retries(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }

    /// Build the task with the given future
    ///
    /// Note: Retries are not supported for FnOnce closures.
    /// Use timeout for basic deadline enforcement.
    pub fn build<F, Fut>(self, task: F) -> BoxedTask
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let name = self.name.clone();
        let timeout_ms = self.timeout_ms;

        Box::pin(async move {
            let task_name = name.as_deref().unwrap_or("unnamed");

            let result = if let Some(timeout) = timeout_ms {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(timeout),
                    task(),
                )
                .await
                {
                    Ok(()) => Ok(()),
                    Err(_) => Err(format!("Task '{}' timed out", task_name)),
                }
            } else {
                // Execute without timeout
                task().await;
                Ok(())
            };

            if let Err(e) = result {
                tracing::error!(
                    task = task_name,
                    error = %e,
                    "Background task failed"
                );
            }
        })
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_background_tasks_basic() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut tasks = BackgroundTasks::new();

        let c1 = counter.clone();
        tasks.add(move || async move {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let c2 = counter.clone();
        tasks.add(move || async move {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(tasks.len(), 2);

        tasks.execute_and_wait().await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_shared_background_tasks() {
        let counter = Arc::new(AtomicU32::new(0));
        let tasks = SharedBackgroundTasks::new();

        let c1 = counter.clone();
        tasks.add(move || async move {
            c1.fetch_add(1, Ordering::SeqCst);
        }).await;

        let c2 = counter.clone();
        tasks.add(move || async move {
            c2.fetch_add(1, Ordering::SeqCst);
        }).await;

        assert_eq!(tasks.len().await, 2);

        tasks.execute_and_wait().await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_task_builder() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut tasks = BackgroundTasks::new();

        let c = counter.clone();
        let task = TaskBuilder::new()
            .name("test_task")
            .build(move || async move {
                c.fetch_add(1, Ordering::SeqCst);
            });

        tasks.add_boxed(task);
        tasks.execute_and_wait().await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_empty_tasks() {
        let tasks = BackgroundTasks::new();
        assert!(tasks.is_empty());
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_execute_with_results() {
        let mut tasks = BackgroundTasks::new();

        tasks.add(|| async {
            // Success
        });

        let results = tasks.execute_with_results().await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }
}
