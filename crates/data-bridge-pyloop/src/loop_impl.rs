//! PyLoop: Rust-backed Python asyncio event loop implementation

use pyo3::prelude::*;
use pyo3::types::PyTuple;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::Duration;

use crate::error::PyLoopError;
use crate::get_runtime;
use crate::handle::{Handle, TimerHandle};
use crate::task::{poll_coroutine, PollResult, Task};
use crate::timer_wheel::{ScheduledCallback, TimerEntry, TimerWheel};

/// PyLoop: Rust-native Python asyncio event loop backed by Tokio
///
/// This class implements the Python asyncio event loop protocol,
/// delegating all actual work to a Tokio runtime. This provides:
///
/// - High performance through Rust's async runtime
/// - Better integration with native Rust async code
/// - Reduced GIL contention through strategic GIL release
/// - Native support for spawning Rust futures from Python
///
/// # Example
///
/// ```python
/// from data_bridge._pyloop import PyLoop
///
/// loop = PyLoop()
/// # Use like any asyncio event loop
/// ```
#[pyclass]
pub struct PyLoop {
    /// Shared reference to the global Tokio runtime
    runtime: Arc<Runtime>,

    /// Whether the loop is currently running
    running: Arc<AtomicBool>,

    /// Whether the loop has been closed
    closed: Arc<AtomicBool>,

    /// Whether stop() has been called
    stopped: Arc<AtomicBool>,

    /// Sender for scheduling tasks (thread-safe)
    task_sender: UnboundedSender<ScheduledCallback>,

    /// Receiver for scheduled tasks (wrapped in Mutex for interior mutability)
    task_receiver: Arc<Mutex<UnboundedReceiver<ScheduledCallback>>>,

    /// Loop start time (for call_at timing)
    start_time: Arc<Mutex<Option<Instant>>>,

    /// Shared timer wheel for efficient timer management
    timer_wheel: Arc<TimerWheel>,
}

#[pymethods]
impl PyLoop {
    /// Create a new PyLoop instance
    ///
    /// This creates a new event loop backed by the shared Tokio runtime.
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = get_runtime()
            .map_err(PyErr::from)?;

        let (task_sender, task_receiver) = unbounded_channel();

        // Create timer wheel that sends expired timers to main task queue
        let timer_wheel = Arc::new(TimerWheel::new(task_sender.clone()));

        // Spawn background timer processor
        let timer_wheel_clone = timer_wheel.clone();
        runtime.spawn(async move {
            timer_wheel_clone.run().await;
        });

        Ok(Self {
            runtime,
            running: Arc::new(AtomicBool::new(false)),
            closed: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
            task_sender,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
            start_time: Arc::new(Mutex::new(None)),
            timer_wheel,
        })
    }

    /// Check if the event loop is running
    fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Check if the event loop is closed
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Close the event loop
    ///
    /// This marks the loop as closed but does not shut down the shared
    /// Tokio runtime (which may be used by other PyLoop instances).
    fn close(&mut self) -> PyResult<()> {
        if self.running.load(Ordering::Acquire) {
            return Err(PyLoopError::InvalidState(
                "Cannot close a running event loop".to_string()
            ).into());
        }

        self.closed.store(true, Ordering::Release);
        Ok(())
    }

    /// Schedule a callback to be called soon
    ///
    /// Arrange for `callback(*args)` to be called on the next iteration
    /// of the event loop. Callbacks are called in the order in which they
    /// are registered. Each callback will be called exactly once.
    ///
    /// Any positional arguments after the callback will be passed to the
    /// callback when it is called.
    ///
    /// An instance of `Handle` is returned, which can be used to cancel
    /// the callback.
    ///
    /// This method is not thread-safe. Use `call_soon_threadsafe` to
    /// schedule callbacks from other threads.
    ///
    /// Args:
    ///     callback: The function to call
    ///     *args: Positional arguments to pass to the callback
    ///
    /// Returns:
    ///     Handle: A handle that can be used to cancel the callback
    ///
    /// Raises:
    ///     RuntimeError: If the event loop is closed
    ///
    /// # Example
    ///
    /// ```python
    /// from data_bridge._pyloop import PyLoop
    ///
    /// loop = PyLoop()
    /// handle = loop.call_soon(print, "Hello, World!")
    /// ```
    #[pyo3(signature = (callback, *args))]
    fn call_soon(
        &self,
        py: Python<'_>,
        callback: PyObject,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<Handle> {
        if self.closed.load(Ordering::Acquire) {
            return Err(PyLoopError::InvalidState(
                "Event loop is closed".to_string()
            ).into());
        }

        let handle = Handle::new();
        #[allow(deprecated)] // PyO3 API transition - to_object will be replaced by IntoPyObject
        let scheduled_callback = ScheduledCallback {
            callback,
            args: args.to_object(py),
            handle: handle.clone_handle(),
        };

        self.task_sender
            .send(scheduled_callback)
            .map_err(|_| PyLoopError::TaskSpawn(
                "Failed to schedule callback".to_string()
            ))?;

        Ok(handle)
    }

    /// Schedule a callback to be called soon (thread-safe)
    ///
    /// Like `call_soon`, but thread-safe. This method can be called from
    /// any thread to schedule a callback in the event loop's thread.
    ///
    /// Args:
    ///     callback: The function to call
    ///     *args: Positional arguments to pass to the callback
    ///
    /// Returns:
    ///     Handle: A handle that can be used to cancel the callback
    ///
    /// Raises:
    ///     RuntimeError: If the event loop is closed
    ///
    /// # Example
    ///
    /// ```python
    /// from data_bridge._pyloop import PyLoop
    /// import threading
    ///
    /// loop = PyLoop()
    ///
    /// def worker():
    ///     loop.call_soon_threadsafe(print, "From thread!")
    ///
    /// thread = threading.Thread(target=worker)
    /// thread.start()
    /// ```
    #[pyo3(signature = (callback, *args))]
    fn call_soon_threadsafe(
        &self,
        py: Python<'_>,
        callback: PyObject,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<Handle> {
        // UnboundedSender is already thread-safe, so we can just call call_soon
        self.call_soon(py, callback, args)
    }

    /// Schedule a callback to be called after a delay
    ///
    /// Arrange for `callback(*args)` to be called approximately `delay` seconds
    /// in the future. The delay is relative to the current time.
    ///
    /// Args:
    ///     delay: Delay in seconds (float, must be non-negative)
    ///     callback: The function to call
    ///     *args: Positional arguments to pass to the callback
    ///
    /// Returns:
    ///     TimerHandle: A handle that can be used to cancel the callback
    ///
    /// Raises:
    ///     RuntimeError: If the event loop is closed
    ///     ValueError: If delay is negative
    ///
    /// # Example
    ///
    /// ```python
    /// from data_bridge._pyloop import PyLoop
    ///
    /// loop = PyLoop()
    /// handle = loop.call_later(1.0, print, "Hello after 1 second")
    /// ```
    #[pyo3(signature = (delay, callback, *args))]
    fn call_later(
        &self,
        py: Python<'_>,
        delay: f64,
        callback: PyObject,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<TimerHandle> {
        if self.closed.load(Ordering::Acquire) {
            return Err(PyLoopError::InvalidState(
                "Event loop is closed".to_string()
            ).into());
        }

        if delay < 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "delay must be non-negative"
            ));
        }

        // Calculate expiration time
        let when = Instant::now() + Duration::from_secs_f64(delay);

        // Create handle for cancellation tracking
        let handle = Handle::new();
        let timer_handle = TimerHandle::new_without_task(handle.clone());

        // Create timer entry
        #[allow(deprecated)] // PyO3 API transition
        let entry = TimerEntry {
            callback,
            args: args.to_object(py),
            handle,
        };

        // Register with timer wheel (non-blocking, lock-free)
        self.timer_wheel.register(when, entry, timer_handle.clone());

        Ok(timer_handle)
    }

    /// Schedule a callback to be called at an absolute time
    ///
    /// Arrange for `callback(*args)` to be called at the given absolute
    /// timestamp `when` (a float using the same time reference as `time()`).
    ///
    /// Args:
    ///     when: Absolute time in seconds (float)
    ///     callback: The function to call
    ///     *args: Positional arguments to pass to the callback
    ///
    /// Returns:
    ///     TimerHandle: A handle that can be used to cancel the callback
    ///
    /// Raises:
    ///     RuntimeError: If the event loop is closed
    ///
    /// # Example
    ///
    /// ```python
    /// from data_bridge._pyloop import PyLoop
    ///
    /// loop = PyLoop()
    /// when = loop.time() + 1.0  # 1 second from now
    /// handle = loop.call_at(when, print, "Hello")
    /// ```
    #[pyo3(signature = (when, callback, *args))]
    fn call_at(
        &self,
        py: Python<'_>,
        when: f64,
        callback: PyObject,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<TimerHandle> {
        // Initialize start time if not set
        self.init_start_time();

        // Calculate delay from current loop time
        let current_time = self.loop_time();
        let delay = (when - current_time).max(0.0);

        // Delegate to call_later
        self.call_later(py, delay, callback, args)
    }

    /// Get the loop's internal time
    ///
    /// Returns the current time according to the event loop's internal clock.
    /// The time is a float representing seconds since an arbitrary reference point.
    ///
    /// Returns:
    ///     float: The current loop time in seconds
    ///
    /// # Example
    ///
    /// ```python
    /// from data_bridge._pyloop import PyLoop
    ///
    /// loop = PyLoop()
    /// now = loop.time()
    /// ```
    fn time(&self) -> f64 {
        self.init_start_time();
        self.loop_time()
    }

    /// Create a task from a coroutine
    ///
    /// Wrap a coroutine in a Task and schedule it for execution. The coroutine
    /// will start executing on the next iteration of the event loop.
    ///
    /// # Arguments
    ///
    /// * `coro` - A Python coroutine object (must have a `send` method)
    /// * `name` - Optional task name for debugging
    ///
    /// # Returns
    ///
    /// A Task object that wraps the coroutine
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If the event loop is closed
    /// * `TypeError` - If the argument is not a coroutine
    ///
    /// # Example
    ///
    /// ```python
    /// async def my_coro():
    ///     await asyncio.sleep(1)
    ///     return 42
    ///
    /// task = loop.create_task(my_coro())
    /// result = await task  # Returns 42
    /// ```
    #[pyo3(signature = (coro, *, name=None))]
    fn create_task(
        &self,
        py: Python<'_>,
        coro: PyObject,
        name: Option<String>,
    ) -> PyResult<Task> {
        if self.closed.load(Ordering::Acquire) {
            return Err(PyLoopError::InvalidState("Event loop is closed".to_string()).into());
        }

        // Verify it's a coroutine (must have a send method)
        let coro_bound = coro.bind(py);
        if !coro_bound.hasattr("send")? {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "create_task() requires a coroutine object",
            ));
        }

        // Clone what we need for the task
        let coro_clone = coro.clone_ref(py);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();
        let exception = Arc::new(Mutex::new(None));
        let exception_clone = exception.clone();

        // Spawn a Tokio task to run the coroutine
        let task_handle = self.runtime.spawn(async move {
            Python::with_gil(|py| {
                let coro_bound = coro_clone.bind(py);

                // Poll the coroutine to completion
                loop {
                    // Check if cancelled
                    if cancelled_clone.load(Ordering::Acquire) {
                        break;
                    }

                    // Poll once
                    match poll_coroutine(py, coro_bound) {
                        Ok(PollResult::Ready(value)) => {
                            // Coroutine finished successfully
                            *result_clone.lock().unwrap() = Some(value);
                            done_clone.store(true, Ordering::Release);
                            break;
                        }
                        Ok(PollResult::Pending(_awaitable)) => {
                            // TODO: Handle awaitable (for now, just yield)
                            // In a full implementation, we'd schedule the awaitable
                            // For now, just sleep a bit to avoid busy-waiting
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(e) => {
                            // Exception raised
                            *exception_clone.lock().unwrap() = Some(e.value(py).clone().unbind());
                            done_clone.store(true, Ordering::Release);
                            break;
                        }
                    }
                }
            });
        });

        // Create the Task object
        let task = Task::new(coro, name, task_handle);

        Ok(task)
    }

    /// Stop the event loop
    ///
    /// This will cause `run_forever` to exit after the current iteration.
    ///
    /// # Example
    ///
    /// ```python
    /// loop = PyLoop()
    ///
    /// def stop_soon():
    ///     loop.stop()
    ///
    /// loop.call_later(1.0, stop_soon)
    /// loop.run_forever()  # Will stop after 1 second
    /// ```
    fn stop(&self) {
        if self.running.load(Ordering::Acquire) {
            self.stopped.store(true, Ordering::Release);
        }
    }

    /// Run the event loop until stop() is called
    ///
    /// Processes all scheduled callbacks (from call_soon, call_later, etc.)
    /// in a continuous loop until stop() is called.
    ///
    /// # Example
    ///
    /// ```python
    /// loop = PyLoop()
    ///
    /// def hello():
    ///     print("Hello from event loop!")
    ///     loop.stop()
    ///
    /// loop.call_soon(hello)
    /// loop.run_forever()  # Prints "Hello from event loop!" and exits
    /// ```
    fn run_forever(&self, py: Python) -> PyResult<()> {
        if self.is_closed() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Event loop is closed"));
        }

        if self.running.load(Ordering::Acquire) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Event loop is already running"));
        }

        // Mark as running
        self.running.store(true, Ordering::Release);
        self.stopped.store(false, Ordering::Release);
        self.init_start_time();

        // Clone Arc references for thread safety
        let running = self.running.clone();
        let stopped = self.stopped.clone();
        let closed = self.closed.clone();
        let receiver = self.task_receiver.clone();

        // Main event loop - release GIL for better concurrency
        py.allow_threads(|| {
            loop {
                // Check if we should stop
                if stopped.load(Ordering::Acquire) {
                    break;
                }

                // Check if closed
                if closed.load(Ordering::Acquire) {
                    break;
                }

                // Process pending tasks (reacquire GIL for Python callbacks)
                let has_tasks = Python::with_gil(|py| {
                    Self::process_tasks_internal(py, &receiver)
                });

                // If no tasks, sleep briefly to avoid busy-waiting
                if !has_tasks {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        });

        // Mark as not running
        running.store(false, Ordering::Release);

        Ok(())
    }

    /// Run the event loop until a future completes
    ///
    /// # Arguments
    ///
    /// * `future` - A Task or coroutine to run until completion
    ///
    /// # Returns
    ///
    /// The result of the future
    ///
    /// # Example
    ///
    /// ```python
    /// loop = PyLoop()
    ///
    /// async def my_coro():
    ///     await asyncio.sleep(1)
    ///     return 42
    ///
    /// result = loop.run_until_complete(my_coro())
    /// print(result)  # 42
    /// ```
    fn run_until_complete(&self, py: Python, future: PyObject) -> PyResult<PyObject> {
        if self.is_closed() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Event loop is closed"));
        }

        if self.running.load(Ordering::Acquire) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Event loop is already running"));
        }

        // Check if it's already a Task or a coroutine
        let future_bound = future.bind(py);
        let task = if future_bound.is_instance_of::<Task>() {
            // It's already a Task, extract it
            future_bound.extract::<Task>()?
        } else if future_bound.hasattr("send")? {
            // It's a coroutine, wrap it in a Task
            self.create_task(py, future.clone_ref(py), None)?
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "run_until_complete() requires a coroutine or Task"
            ));
        };

        // Run the loop until the task is done
        self.running.store(true, Ordering::Release);
        self.init_start_time();

        // Clone Arc references and Task for thread safety
        let running = self.running.clone();
        let closed = self.closed.clone();
        let receiver = self.task_receiver.clone();
        let task_clone = task.clone();

        // Release GIL and run loop
        py.allow_threads(|| {
            loop {
                // Check if task is done
                if task_clone.is_done() {
                    break;
                }

                // Check if closed
                if closed.load(Ordering::Acquire) {
                    break;
                }

                // Process pending tasks (reacquire GIL)
                Python::with_gil(|py| {
                    Self::process_tasks_internal(py, &receiver);
                });

                // Brief sleep to avoid busy-waiting
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        // Mark as not running
        running.store(false, Ordering::Release);

        // Return the task result
        task.get_result(py)
    }

    /// Get debug representation
    fn __repr__(&self) -> String {
        format!(
            "PyLoop(running={}, closed={})",
            self.running.load(Ordering::Acquire),
            self.closed.load(Ordering::Acquire)
        )
    }
}

impl PyLoop {
    /// Get time since loop start (for call_at)
    fn loop_time(&self) -> f64 {
        match *self.start_time.lock().unwrap() {
            Some(start) => start.elapsed().as_secs_f64(),
            None => 0.0,
        }
    }

    /// Initialize start time (called when loop starts or first time() call)
    fn init_start_time(&self) {
        let mut start = self.start_time.lock().unwrap();
        if start.is_none() {
            *start = Some(Instant::now());
        }
    }

    /// Process all pending tasks (internal helper)
    ///
    /// This is called by run_forever and run_until_complete to execute
    /// scheduled callbacks. It processes all currently pending tasks,
    /// skipping any that have been cancelled.
    ///
    /// # GIL Management
    ///
    /// This method assumes the GIL is held when called, as it needs to
    /// invoke Python callbacks.
    ///
    /// # Returns
    ///
    /// Returns `true` if any tasks were processed, `false` otherwise.
    #[allow(dead_code)]
    pub(crate) fn process_tasks(&self, py: Python<'_>) -> bool {
        Self::process_tasks_internal(py, &self.task_receiver)
    }

    /// Process all pending tasks (static helper for use in threads)
    ///
    /// This static method allows processing tasks when we don't have
    /// a direct reference to `self` (e.g., in `py.allow_threads()`).
    ///
    /// Returns `true` if any tasks were processed, `false` otherwise.
    fn process_tasks_internal(
        py: Python<'_>,
        receiver: &Arc<Mutex<UnboundedReceiver<ScheduledCallback>>>,
    ) -> bool {
        let mut receiver_guard = match receiver.lock() {
            Ok(guard) => guard,
            Err(_) => return false, // Lock poisoned, no tasks processed
        };

        let mut processed = false;
        let mut batch_count = 0;

        // Maximum number of callbacks to process per iteration
        // This prevents GIL starvation and ensures fairness.
        // Inspired by uvloop's batch processing strategy.
        const MAX_BATCH_SIZE: usize = 128;

        // Process up to MAX_BATCH_SIZE tasks per iteration (non-blocking)
        while batch_count < MAX_BATCH_SIZE {
            match receiver_guard.try_recv() {
                Ok(scheduled_callback) => {
                    batch_count += 1;
                    processed = true;

                    // Skip cancelled tasks
                    if scheduled_callback.handle.is_cancelled() {
                        continue;
                    }

                    // Call the Python callback with its arguments
                    match scheduled_callback.args.downcast_bound::<PyTuple>(py) {
                        Ok(args) => {
                            // Invoke callback, print exception but don't crash loop
                            if let Err(e) = scheduled_callback.callback.call1(py, args) {
                                e.print(py);
                            }
                        }
                        Err(_e) => {
                            // Print type error - create a proper error and print it
                            let type_err = pyo3::exceptions::PyTypeError::new_err(
                                "Callback arguments must be a tuple"
                            );
                            type_err.print(py);
                        }
                    }
                }
                Err(_) => break, // No more tasks available
            }
        }

        processed
    }

    /// Create a new PyLoop instance (for Rust tests)
    #[cfg(test)]
    pub fn new_for_test() -> Result<Self, crate::error::PyLoopError> {
        let runtime = crate::get_runtime()?;
        let (task_sender, task_receiver) = unbounded_channel();

        // Create timer wheel
        let timer_wheel = Arc::new(TimerWheel::new(task_sender.clone()));

        // Spawn background timer processor
        let timer_wheel_clone = timer_wheel.clone();
        runtime.spawn(async move {
            timer_wheel_clone.run().await;
        });

        Ok(Self {
            runtime,
            running: Arc::new(AtomicBool::new(false)),
            closed: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
            task_sender,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
            start_time: Arc::new(Mutex::new(None)),
            timer_wheel,
        })
    }

    /// Set the running state (internal use)
    #[allow(dead_code)]
    pub(crate) fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::Release);
    }

    /// Get a reference to the Tokio runtime
    #[allow(dead_code)]
    pub(crate) fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Check if running (for Rust tests)
    #[cfg(test)]
    pub fn test_is_running(&self) -> bool {
        self.is_running()
    }

    /// Check if closed (for Rust tests)
    #[cfg(test)]
    pub fn test_is_closed(&self) -> bool {
        self.is_closed()
    }

    /// Close the loop (for Rust tests)
    #[cfg(test)]
    pub fn test_close(&self) -> Result<(), crate::error::PyLoopError> {
        if self.running.load(Ordering::Acquire) {
            return Err(crate::error::PyLoopError::InvalidState(
                "Cannot close a running event loop".to_string()
            ));
        }
        self.closed.store(true, Ordering::Release);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyloop_creation() {
        let loop_inst = PyLoop::new_for_test();
        assert!(loop_inst.is_ok(), "PyLoop should be created successfully");

        let loop_inst = loop_inst.unwrap();
        assert!(!loop_inst.test_is_running(), "New loop should not be running");
        assert!(!loop_inst.test_is_closed(), "New loop should not be closed");
    }

    #[test]
    fn test_pyloop_close() {
        let loop_inst = PyLoop::new_for_test().unwrap();

        let result = loop_inst.test_close();
        assert!(result.is_ok(), "Closing a stopped loop should succeed");
        assert!(loop_inst.test_is_closed(), "Loop should be closed");
    }

    #[test]
    fn test_cannot_close_running_loop() {
        let loop_inst = PyLoop::new_for_test().unwrap();
        loop_inst.set_running(true);

        let result = loop_inst.test_close();
        assert!(result.is_err(), "Cannot close a running loop");
    }
}
