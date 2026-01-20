//! Timer wheel for efficient timer management
//!
//! This module provides a shared timer wheel that manages all scheduled timers
//! in a single background task, replacing the per-timer Tokio task approach.
//!
//! # Architecture
//!
//! Instead of spawning a separate task for each timer:
//! ```text
//! OLD: call_later(delay, callback) → tokio::spawn(sleep(delay)) → schedule callback
//! ```
//!
//! We use a shared timer wheel with a single background processor:
//! ```text
//! NEW: call_later(delay, callback) → register in TimerWheel
//!                                  ↓
//!      Background processor (1ms tick) → check expired timers → schedule callbacks
//! ```
//!
//! # Performance
//!
//! This approach reduces overhead by:
//! - Eliminating per-timer task spawning
//! - Batching timer checks every 1ms
//! - Using efficient BTreeMap for time-based lookups
//! - Lock-free channels for timer registration/cancellation

use pyo3::prelude::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::handle::{Handle, TimerHandle};

/// Entry in the timer wheel representing a scheduled timer
pub struct TimerEntry {
    /// Python callback to execute when timer fires
    pub callback: PyObject,
    /// Arguments to pass to the callback
    pub args: PyObject,
    /// Handle for cancellation tracking
    pub handle: Handle,
}

/// Message sent to task queue when timer expires
/// This is defined here to avoid circular dependencies,
/// but is structurally identical to loop_impl::ScheduledCallback
pub struct ScheduledCallback {
    /// Python callback to execute
    pub callback: PyObject,
    /// Arguments to pass to the callback
    pub args: PyObject,
    /// Handle for cancellation tracking
    pub handle: Handle,
}

/// Shared timer wheel for efficient timer management
///
/// The timer wheel maintains a sorted collection of timers organized by
/// expiration time. A single background task processes all timers,
/// waking up when timers expire or new timers are registered.
///
/// # Thread Safety
///
/// All operations are thread-safe using Arc<Mutex<>> for shared state
/// and lock-free channels for communication.
pub struct TimerWheel {
    /// Timers organized by expiration time
    /// BTreeMap provides O(log n) insertion and efficient range queries
    timers: Arc<Mutex<BTreeMap<Instant, Vec<TimerEntry>>>>,

    /// Channel to send expired timers to main task queue
    callback_sender: UnboundedSender<ScheduledCallback>,

    /// Channel to register new timers (lock-free communication)
    register_sender: UnboundedSender<(Instant, TimerEntry, TimerHandle)>,
    register_receiver: Arc<Mutex<UnboundedReceiver<(Instant, TimerEntry, TimerHandle)>>>,

    /// Channel to cancel timers (lock-free communication)
    cancel_sender: UnboundedSender<TimerHandle>,
    cancel_receiver: Arc<Mutex<UnboundedReceiver<TimerHandle>>>,
}

impl TimerWheel {
    /// Create a new timer wheel
    ///
    /// # Arguments
    ///
    /// * `callback_sender` - Channel to send expired callbacks to main event loop
    ///
    /// # Returns
    ///
    /// A new TimerWheel instance ready for use
    pub fn new(callback_sender: UnboundedSender<ScheduledCallback>) -> Self {
        let (register_sender, register_receiver) = unbounded_channel();
        let (cancel_sender, cancel_receiver) = unbounded_channel();

        Self {
            timers: Arc::new(Mutex::new(BTreeMap::new())),
            callback_sender,
            register_sender,
            register_receiver: Arc::new(Mutex::new(register_receiver)),
            cancel_sender,
            cancel_receiver: Arc::new(Mutex::new(cancel_receiver)),
        }
    }

    /// Register a new timer
    ///
    /// This is a non-blocking operation that sends the timer to the background
    /// processor via a lock-free channel.
    ///
    /// # Arguments
    ///
    /// * `when` - Absolute time when the timer should fire
    /// * `entry` - Timer entry containing callback and arguments
    /// * `handle` - Timer handle for cancellation
    pub fn register(&self, when: Instant, entry: TimerEntry, handle: TimerHandle) {
        // Send to background processor - non-blocking
        let _ = self.register_sender.send((when, entry, handle));
    }

    /// Cancel a timer
    ///
    /// This is a non-blocking operation that sends the cancellation request
    /// to the background processor via a lock-free channel.
    ///
    /// # Arguments
    ///
    /// * `handle` - Timer handle to cancel
    pub fn cancel_timer(&self, handle: &TimerHandle) {
        // Mark handle as cancelled (through base handle)
        handle.base_handle().cancel_internal();

        // Also send cancellation message for cleanup
        let _ = self.cancel_sender.send(handle.clone());
    }

    /// Get registration sender for cloning (used by PyLoop)
    pub fn get_register_sender(&self) -> UnboundedSender<(Instant, TimerEntry, TimerHandle)> {
        self.register_sender.clone()
    }

    /// Get cancellation sender for cloning (used by PyLoop)
    pub fn get_cancel_sender(&self) -> UnboundedSender<TimerHandle> {
        self.cancel_sender.clone()
    }

    /// Process pending timer registrations
    ///
    /// This is called by the background processor to add newly registered
    /// timers to the timer wheel.
    fn process_registrations(&self) {
        let mut receiver = self.register_receiver.lock().unwrap();

        // Process all pending registrations
        while let Ok((when, entry, _handle)) = receiver.try_recv() {
            let mut timers = self.timers.lock().unwrap();
            timers.entry(when).or_default().push(entry);
        }
    }

    /// Process pending cancellations
    ///
    /// Note: We rely on the atomic cancellation flag in the Handle.
    /// The actual timer entries will be filtered out when they expire.
    /// This method just drains the cancellation channel.
    fn process_cancellations(&self) {
        let mut receiver = self.cancel_receiver.lock().unwrap();

        // Drain the cancellation queue
        // The handles are already marked as cancelled (atomic flag)
        while receiver.try_recv().is_ok() {
            // Nothing to do - cancellation is tracked in the Handle itself
        }
    }

    /// Process expired timers
    ///
    /// This is the core of the timer wheel - it finds all timers that have
    /// expired and sends their callbacks to the main event loop queue.
    fn process_expired(&self) {
        let now = Instant::now();
        let mut timers = self.timers.lock().unwrap();

        // Find all expired time slots (BTreeMap range query is efficient)
        let expired_keys: Vec<Instant> = timers
            .range(..=now)
            .map(|(k, _)| *k)
            .collect();

        // Process each expired time slot
        for key in expired_keys {
            if let Some(entries) = timers.remove(&key) {
                for entry in entries {
                    // Skip cancelled timers
                    if entry.handle.is_cancelled() {
                        continue;
                    }

                    // Send to main task queue
                    let scheduled = ScheduledCallback {
                        callback: entry.callback,
                        args: entry.args,
                        handle: entry.handle,
                    };

                    // If send fails, event loop is shutting down
                    let _ = self.callback_sender.send(scheduled);
                }
            }
        }
    }

    /// Get the next expiration time
    ///
    /// Returns the instant when the next timer will expire, or None if no timers.
    fn get_next_expiration(&self) -> Option<Instant> {
        let timers = self.timers.lock().unwrap();
        timers.keys().next().copied()
    }

    /// Calculate the optimal sleep duration until the next timer expires
    ///
    /// This method is used by the event loop to implement adaptive sleeping,
    /// which reduces CPU usage and improves timer precision.
    ///
    /// # Returns
    ///
    /// - `Some(Duration)` if there are pending timers
    /// - `None` if there are no pending timers
    ///
    /// # Algorithm
    ///
    /// - If next timer expires in the future: sleep for that duration
    /// - If next timer already expired: return Duration::ZERO (process immediately)
    /// - If no timers: return None (caller should use default sleep)
    ///
    /// # Example
    ///
    /// ```rust
    /// let sleep_duration = timer_wheel.calculate_sleep_duration()
    ///     .unwrap_or(Duration::from_millis(1))  // Default to 1ms
    ///     .min(Duration::from_millis(1));        // Cap at 1ms
    /// ```
    pub fn calculate_sleep_duration(&self) -> Option<Duration> {
        match self.get_next_expiration() {
            Some(next_expiry) => {
                let now = Instant::now();
                if next_expiry > now {
                    // Timer expires in the future - sleep until then
                    Some(next_expiry - now)
                } else {
                    // Timer already expired - process immediately
                    Some(Duration::ZERO)
                }
            }
            None => {
                // No timers pending
                None
            }
        }
    }

    /// Run the timer wheel processor
    ///
    /// This is the main loop that should be spawned as a background task.
    /// It processes timers dynamically, checking for:
    /// - New registrations
    /// - Cancellations
    /// - Expired timers
    ///
    /// # Performance
    ///
    /// - Dynamic sleep based on next expiration time (low CPU usage)
    /// - Maximum 1ms sleep to handle new registrations
    /// - Lock contention is minimal (locks held briefly)
    /// - BTreeMap operations are O(log n)
    /// - Channel operations are lock-free
    pub async fn run(&self) {
        loop {
            // Process new registrations first
            self.process_registrations();

            // Process cancellations
            self.process_cancellations();

            // Process expired timers
            self.process_expired();

            // Calculate sleep duration based on next timer
            let sleep_duration = match self.get_next_expiration() {
                Some(next_expiration) => {
                    let now = Instant::now();
                    if next_expiration <= now {
                        // Timer already expired, process immediately
                        continue;
                    } else {
                        // Sleep until next timer, but max 1ms to check for new registrations
                        let until_expiration = next_expiration - now;
                        until_expiration.min(Duration::from_millis(1))
                    }
                }
                None => {
                    // No timers, sleep for 1ms and check for new registrations
                    Duration::from_millis(1)
                }
            };

            tokio::time::sleep(sleep_duration).await;
        }
    }

    /// Get the number of pending timers (for debugging/testing)
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.timers
            .lock()
            .unwrap()
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Check if timer wheel is empty (for debugging/testing)
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::Python;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_python() {
        INIT.call_once(|| {
            pyo3::prepare_freethreaded_python();
        });
    }

    #[test]
    fn test_timer_wheel_creation() {
        let (sender, _receiver) = unbounded_channel();
        let wheel = TimerWheel::new(sender);

        assert_eq!(wheel.len(), 0, "New timer wheel should be empty");
    }

    #[test]
    fn test_timer_registration() {
        init_python();
        Python::with_gil(|py| {
            let (sender, _receiver) = unbounded_channel();
            let wheel = TimerWheel::new(sender);

            // Create a dummy callback
            let callback = py.None().into();
            let args = py.None().into();

            let entry = TimerEntry {
                callback,
                args,
                handle: Handle::new(),
            };

            let when = Instant::now() + Duration::from_secs(1);
            let timer_handle = TimerHandle::new_without_task(Handle::new());

            wheel.register(when, entry, timer_handle);

            // Process registrations
            wheel.process_registrations();

            assert_eq!(wheel.len(), 1, "Should have 1 registered timer");
        });
    }

    #[test]
    fn test_timer_expiration() {
        init_python();
        Python::with_gil(|py| {
            let (sender, mut receiver) = unbounded_channel();
            let wheel = TimerWheel::new(sender);

            // Create a dummy callback that should expire immediately
            let callback = py.None().into();
            let args = py.None().into();

            let entry = TimerEntry {
                callback,
                args,
                handle: Handle::new(),
            };

            // Register timer that expired in the past
            let when = Instant::now() - Duration::from_secs(1);
            let timer_handle = TimerHandle::new_without_task(Handle::new());

            wheel.register(when, entry, timer_handle);

            // Process registrations and expirations
            wheel.process_registrations();
            wheel.process_expired();

            // Should have sent callback to main queue
            assert!(receiver.try_recv().is_ok(), "Should have expired timer");
            assert_eq!(wheel.len(), 0, "Timer wheel should be empty after expiration");
        });
    }

    #[test]
    fn test_timer_cancellation() {
        init_python();
        Python::with_gil(|py| {
            let (sender, mut receiver) = unbounded_channel();
            let wheel = TimerWheel::new(sender);

            // Create a dummy callback
            let callback = py.None().into();
            let args = py.None().into();

            let handle = Handle::new();
            let entry = TimerEntry {
                callback,
                args,
                handle: handle.clone(),
            };

            // Register timer that should expire immediately
            let when = Instant::now() - Duration::from_secs(1);
            let timer_handle = TimerHandle::new_without_task(handle);

            wheel.register(when, entry, timer_handle.clone());

            // Cancel before processing
            wheel.cancel_timer(&timer_handle);

            // Process registrations and expirations
            wheel.process_registrations();
            wheel.process_cancellations();
            wheel.process_expired();

            // Should not have sent callback (was cancelled)
            assert!(receiver.try_recv().is_err(), "Should not have expired cancelled timer");
        });
    }
}
