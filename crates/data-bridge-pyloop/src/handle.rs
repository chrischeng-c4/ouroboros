//! Handle type for scheduled callbacks
//!
//! A Handle represents a scheduled callback that can be cancelled.
//! It's returned by `call_soon`, `call_soon_threadsafe`, and `call_later`.

use pyo3::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

/// Handle to a scheduled callback
///
/// Represents a scheduled callback in the event loop. The callback can be
/// cancelled by calling `cancel()` before it executes.
///
/// # Thread Safety
///
/// Handle is thread-safe and can be passed between threads. The cancellation
/// flag uses atomic operations for lock-free cancellation.
///
/// # Example
///
/// ```python
/// from data_bridge._pyloop import PyLoop
///
/// loop = PyLoop()
/// handle = loop.call_soon(print, "Hello")
/// handle.cancel()  # Cancel before execution
/// ```
#[pyclass]
#[derive(Clone)]
pub struct Handle {
    /// Atomic cancellation flag - thread-safe without locks
    cancelled: Arc<AtomicBool>,
}

#[pymethods]
impl Handle {
    /// Check if the callback has been cancelled
    ///
    /// Returns:
    ///     bool: True if the callback was cancelled, False otherwise
    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Cancel the callback
    ///
    /// If the callback has not yet been executed, it will be skipped.
    /// Calling cancel() on an already-executed or already-cancelled handle
    /// has no effect.
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Get debug representation
    fn __repr__(&self) -> String {
        format!("Handle(cancelled={})", self.cancelled())
    }
}

impl Handle {
    /// Create a new Handle with a fresh cancellation flag
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if this handle has been cancelled (internal use)
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Cancel the handle (internal use from Rust)
    /// This is public to allow timer_wheel to cancel handles
    pub fn cancel_internal(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Clone the handle (shares the same cancellation flag)
    #[allow(dead_code)]
    pub fn clone_handle(&self) -> Self {
        Self {
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}

impl Default for Handle {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a scheduled timer callback
///
/// Provides timer-specific functionality, including the ability
/// to cancel the underlying Tokio task that implements the delay.
///
/// # Thread Safety
///
/// TimerHandle is thread-safe and can be passed between threads.
///
/// # Example
///
/// ```python
/// from data_bridge._pyloop import PyLoop
///
/// loop = PyLoop()
/// handle = loop.call_later(1.0, print, "Hello after 1 second")
/// handle.cancel()  # Cancel before execution
/// ```
#[pyclass]
#[derive(Clone)]
pub struct TimerHandle {
    /// Base handle for cancellation flag
    base_handle: Handle,
    /// Internal join handle for the timer task (None for timer wheel timers)
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[pymethods]
impl TimerHandle {
    /// Cancel the timer
    ///
    /// Cancels both the base handle and aborts the underlying Tokio task.
    fn cancel(&self) -> PyResult<()> {
        // First cancel the base handle
        self.base_handle.cancel();

        // Then abort the Tokio task
        if let Some(handle) = self.task_handle.lock().unwrap().take() {
            handle.abort();
        }

        Ok(())
    }

    /// Check if cancelled
    fn cancelled(&self) -> bool {
        self.base_handle.cancelled()
    }

    /// Get debug representation
    fn __repr__(&self) -> String {
        if self.base_handle.cancelled() {
            "TimerHandle(cancelled)".to_string()
        } else {
            "TimerHandle(active)".to_string()
        }
    }
}

impl TimerHandle {
    /// Create a new TimerHandle with a base handle and Tokio task handle
    pub fn new(base_handle: Handle, task_handle: JoinHandle<()>) -> Self {
        Self {
            base_handle,
            task_handle: Arc::new(Mutex::new(Some(task_handle))),
        }
    }

    /// Create a new TimerHandle without a Tokio task (for timer wheel)
    pub fn new_without_task(base_handle: Handle) -> Self {
        Self {
            base_handle,
            task_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Get a reference to the base handle (for internal use)
    pub fn base_handle(&self) -> &Handle {
        &self.base_handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_creation() {
        let handle = Handle::new();
        assert!(!handle.is_cancelled(), "New handle should not be cancelled");
    }

    #[test]
    fn test_handle_cancel() {
        let handle = Handle::new();
        assert!(!handle.is_cancelled());

        handle.cancel();
        assert!(handle.is_cancelled(), "Handle should be cancelled after cancel()");
    }

    #[test]
    fn test_handle_clone_shares_state() {
        let handle1 = Handle::new();
        let handle2 = handle1.clone_handle();

        // Cancel one handle
        handle1.cancel();

        // Both should show as cancelled (shared state)
        assert!(handle1.is_cancelled());
        assert!(handle2.is_cancelled());
    }

    #[test]
    fn test_handle_double_cancel() {
        let handle = Handle::new();

        handle.cancel();
        handle.cancel(); // Should be safe to call multiple times

        assert!(handle.is_cancelled());
    }
}
