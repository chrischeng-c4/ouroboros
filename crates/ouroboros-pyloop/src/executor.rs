//! Executor integration for CPU-bound work
//!
//! Provides asyncio-compatible executor APIs for offloading blocking
//! operations to thread pools.

use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

// ============================================================================
// Executor Types
// ============================================================================

/// Thread pool executor for CPU-bound tasks
pub struct ThreadPoolExecutor {
    /// Maximum concurrent tasks
    max_workers: usize,
    /// Semaphore for limiting concurrency
    semaphore: Arc<Semaphore>,
}

impl ThreadPoolExecutor {
    /// Create a new thread pool executor
    ///
    /// # Arguments
    /// * `max_workers` - Maximum number of concurrent tasks (None = CPU count)
    pub fn new(max_workers: Option<usize>) -> Self {
        let max_workers = max_workers.unwrap_or_else(num_cpus::get);
        Self {
            max_workers,
            semaphore: Arc::new(Semaphore::new(max_workers)),
        }
    }

    /// Get the maximum number of workers
    pub fn max_workers(&self) -> usize {
        self.max_workers
    }

    /// Get the current number of available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Run a blocking function in the thread pool
    ///
    /// This is equivalent to `loop.run_in_executor(executor, func, *args)`
    /// in Python's asyncio.
    pub async fn run<F, R>(&self, func: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed");

        let result = tokio::task::spawn_blocking(move || {
            let result = func();
            drop(permit); // Release permit after task completes
            result
        })
        .await
        .expect("spawn_blocking panicked");

        result
    }

    /// Submit a task and return a handle
    pub fn submit<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let semaphore = Arc::clone(&self.semaphore);

        tokio::spawn(async move {
            let permit = semaphore
                .acquire_owned()
                .await
                .expect("semaphore closed");

            let result = tokio::task::spawn_blocking(move || {
                let result = func();
                drop(permit);
                result
            })
            .await
            .expect("spawn_blocking panicked");

            result
        })
    }

    /// Map a function over an iterator of items concurrently
    pub async fn map<I, T, F, R>(&self, items: I, func: F) -> Vec<R>
    where
        I: IntoIterator<Item = T>,
        T: Send + 'static,
        F: Fn(T) -> R + Send + Sync + Clone + 'static,
        R: Send + 'static,
    {
        let func = Arc::new(func);
        let mut handles = Vec::new();

        for item in items {
            let func = Arc::clone(&func);
            let semaphore = Arc::clone(&self.semaphore);

            let handle = tokio::spawn(async move {
                let permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("semaphore closed");

                let result = tokio::task::spawn_blocking(move || {
                    let result = func(item);
                    drop(permit);
                    result
                })
                .await
                .expect("spawn_blocking panicked");

                result
            });

            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(handle.await.expect("task panicked"));
        }

        results
    }
}

impl Default for ThreadPoolExecutor {
    fn default() -> Self {
        Self::new(None)
    }
}

// ============================================================================
// Global Default Executor
// ============================================================================

use std::sync::OnceLock;
use tokio::sync::RwLock;

static DEFAULT_EXECUTOR: OnceLock<RwLock<Option<Arc<ThreadPoolExecutor>>>> = OnceLock::new();

fn get_executor_lock() -> &'static RwLock<Option<Arc<ThreadPoolExecutor>>> {
    DEFAULT_EXECUTOR.get_or_init(|| RwLock::new(None))
}

/// Set the default executor for `run_in_executor(None, ...)`
pub async fn set_default_executor(executor: ThreadPoolExecutor) {
    let mut lock = get_executor_lock().write().await;
    *lock = Some(Arc::new(executor));
}

/// Get the default executor
pub async fn get_default_executor() -> Arc<ThreadPoolExecutor> {
    let lock = get_executor_lock().read().await;
    lock.clone()
        .unwrap_or_else(|| Arc::new(ThreadPoolExecutor::default()))
}

/// Run a function in the default executor
///
/// This is equivalent to `loop.run_in_executor(None, func, *args)` in asyncio.
pub async fn run_in_executor<F, R>(func: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let executor = get_default_executor().await;
    executor.run(func).await
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Run a blocking function in the thread pool without executor management
///
/// This is a simpler API that uses Tokio's spawn_blocking directly.
pub async fn spawn_blocking<F, R>(func: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(func)
        .await
        .expect("spawn_blocking panicked")
}

/// Run multiple blocking functions concurrently
pub async fn spawn_blocking_many<F, R>(funcs: Vec<F>) -> Vec<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let handles: Vec<_> = funcs
        .into_iter()
        .map(|f| tokio::task::spawn_blocking(f))
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await.expect("spawn_blocking panicked"));
    }
    results
}

/// Block the current thread on a future
///
/// This is for calling async code from sync code.
/// Should not be used inside an async context.
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Handle::current().block_on(future)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_thread_pool_executor_basic() {
        let executor = ThreadPoolExecutor::new(Some(2));
        assert_eq!(executor.max_workers(), 2);

        let result = executor.run(|| 42).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_thread_pool_executor_concurrency() {
        let executor = Arc::new(ThreadPoolExecutor::new(Some(2)));
        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..4 {
            let executor = Arc::clone(&executor);
            let counter = Arc::clone(&counter);

            let handle = tokio::spawn(async move {
                executor
                    .run(move || {
                        counter.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(10));
                    })
                    .await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_executor_map() {
        let executor = ThreadPoolExecutor::new(Some(4));
        let items = vec![1, 2, 3, 4, 5];

        let results = executor.map(items, |x| x * 2).await;
        assert_eq!(results, vec![2, 4, 6, 8, 10]);
    }

    #[tokio::test]
    async fn test_spawn_blocking() {
        let result = spawn_blocking(|| {
            std::thread::sleep(Duration::from_millis(10));
            "done"
        })
        .await;

        assert_eq!(result, "done");
    }

    #[tokio::test]
    async fn test_spawn_blocking_many() {
        let funcs: Vec<Box<dyn FnOnce() -> i32 + Send>> = vec![
            Box::new(|| 1),
            Box::new(|| 2),
            Box::new(|| 3),
        ];

        let results = spawn_blocking_many(funcs).await;
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_default_executor() {
        // Set custom default
        set_default_executor(ThreadPoolExecutor::new(Some(4))).await;

        let executor = get_default_executor().await;
        assert_eq!(executor.max_workers(), 4);

        // Test run_in_executor
        let result = run_in_executor(|| 100).await;
        assert_eq!(result, 100);
    }
}
