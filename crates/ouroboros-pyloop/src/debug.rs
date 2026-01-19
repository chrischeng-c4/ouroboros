//! Event loop debugging tools
//!
//! Provides debugging, profiling, and monitoring capabilities
//! for the event loop.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// ============================================================================
// Debug Configuration
// ============================================================================

/// Debug mode configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Enable debug mode
    pub enabled: bool,
    /// Slow callback threshold
    pub slow_callback_duration: Duration,
    /// Maximum number of slow callbacks to track
    pub max_slow_callbacks: usize,
    /// Enable stack trace capture
    pub capture_stack_traces: bool,
    /// Log slow callbacks
    pub log_slow_callbacks: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            slow_callback_duration: Duration::from_millis(100),
            max_slow_callbacks: 100,
            capture_stack_traces: false,
            log_slow_callbacks: true,
        }
    }
}

impl DebugConfig {
    /// Create a new debug configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable debug mode
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set slow callback threshold
    pub fn slow_callback_duration(mut self, duration: Duration) -> Self {
        self.slow_callback_duration = duration;
        self
    }

    /// Set max slow callbacks to track
    pub fn max_slow_callbacks(mut self, max: usize) -> Self {
        self.max_slow_callbacks = max;
        self
    }

    /// Enable stack trace capture
    pub fn capture_stack_traces(mut self, capture: bool) -> Self {
        self.capture_stack_traces = capture;
        self
    }

    /// Enable slow callback logging
    pub fn log_slow_callbacks(mut self, log: bool) -> Self {
        self.log_slow_callbacks = log;
        self
    }
}

// ============================================================================
// Slow Callback Record
// ============================================================================

/// Record of a slow callback
#[derive(Debug, Clone)]
pub struct SlowCallback {
    /// Callback name/description
    pub name: String,
    /// Duration the callback took
    pub duration: Duration,
    /// When the callback started
    pub started_at: Instant,
    /// Stack trace (if captured)
    pub stack_trace: Option<String>,
}

impl SlowCallback {
    /// Create a new slow callback record
    pub fn new(name: impl Into<String>, duration: Duration, started_at: Instant) -> Self {
        Self {
            name: name.into(),
            duration,
            started_at,
            stack_trace: None,
        }
    }

    /// Add stack trace
    pub fn with_stack_trace(mut self, trace: String) -> Self {
        self.stack_trace = Some(trace);
        self
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration.as_secs_f64() * 1000.0
    }
}

// ============================================================================
// Loop Statistics
// ============================================================================

/// Event loop statistics
#[derive(Debug, Clone)]
pub struct LoopStatistics {
    /// Total callbacks executed
    pub callbacks_executed: u64,
    /// Total time spent in callbacks
    pub total_callback_time: Duration,
    /// Number of slow callbacks detected
    pub slow_callbacks_count: u64,
    /// Number of tasks created
    pub tasks_created: u64,
    /// Number of tasks completed
    pub tasks_completed: u64,
    /// Number of tasks cancelled
    pub tasks_cancelled: u64,
    /// Current pending tasks
    pub pending_tasks: u64,
    /// Loop iterations
    pub iterations: u64,
}

impl Default for LoopStatistics {
    fn default() -> Self {
        Self {
            callbacks_executed: 0,
            total_callback_time: Duration::ZERO,
            slow_callbacks_count: 0,
            tasks_created: 0,
            tasks_completed: 0,
            tasks_cancelled: 0,
            pending_tasks: 0,
            iterations: 0,
        }
    }
}

impl LoopStatistics {
    /// Get average callback duration
    pub fn average_callback_duration(&self) -> Duration {
        if self.callbacks_executed == 0 {
            Duration::ZERO
        } else {
            self.total_callback_time / self.callbacks_executed as u32
        }
    }

    /// Get slow callback percentage
    pub fn slow_callback_percentage(&self) -> f64 {
        if self.callbacks_executed == 0 {
            0.0
        } else {
            (self.slow_callbacks_count as f64 / self.callbacks_executed as f64) * 100.0
        }
    }
}

// ============================================================================
// Debug Monitor
// ============================================================================

/// Event loop debug monitor
pub struct DebugMonitor {
    config: RwLock<DebugConfig>,
    enabled: AtomicBool,
    slow_callbacks: RwLock<VecDeque<SlowCallback>>,
    stats: RwLock<LoopStatistics>,
    // Atomic counters for fast updates
    callbacks_counter: AtomicU64,
    slow_counter: AtomicU64,
    tasks_created: AtomicU64,
    tasks_completed: AtomicU64,
    tasks_cancelled: AtomicU64,
    iterations: AtomicU64,
}

impl DebugMonitor {
    /// Create a new debug monitor
    pub fn new(config: DebugConfig) -> Self {
        let enabled = config.enabled;
        Self {
            config: RwLock::new(config),
            enabled: AtomicBool::new(enabled),
            slow_callbacks: RwLock::new(VecDeque::new()),
            stats: RwLock::new(LoopStatistics::default()),
            callbacks_counter: AtomicU64::new(0),
            slow_counter: AtomicU64::new(0),
            tasks_created: AtomicU64::new(0),
            tasks_completed: AtomicU64::new(0),
            tasks_cancelled: AtomicU64::new(0),
            iterations: AtomicU64::new(0),
        }
    }

    /// Enable debug mode
    pub fn set_debug(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.config.write().enabled = enabled;
    }

    /// Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Set slow callback duration threshold
    pub fn set_slow_callback_duration(&self, duration: Duration) {
        self.config.write().slow_callback_duration = duration;
    }

    /// Get slow callback duration threshold
    pub fn slow_callback_duration(&self) -> Duration {
        self.config.read().slow_callback_duration
    }

    /// Start timing a callback
    pub fn start_callback(&self) -> Option<CallbackTimer> {
        if self.enabled.load(Ordering::Relaxed) {
            Some(CallbackTimer {
                start: Instant::now(),
            })
        } else {
            None
        }
    }

    /// Record callback completion
    pub fn end_callback(&self, timer: Option<CallbackTimer>, name: &str) {
        self.callbacks_counter.fetch_add(1, Ordering::Relaxed);

        if let Some(timer) = timer {
            let duration = timer.start.elapsed();
            let config = self.config.read();

            // Update stats
            {
                let mut stats = self.stats.write();
                stats.callbacks_executed += 1;
                stats.total_callback_time += duration;
            }

            // Check for slow callback
            if duration > config.slow_callback_duration {
                self.slow_counter.fetch_add(1, Ordering::Relaxed);

                let mut record = SlowCallback::new(name, duration, timer.start);

                if config.capture_stack_traces {
                    record.stack_trace = Some(capture_stack_trace());
                }

                if config.log_slow_callbacks {
                    eprintln!(
                        "[WARNING] Slow callback '{}' took {:.2}ms (threshold: {:.2}ms)",
                        name,
                        record.duration_ms(),
                        config.slow_callback_duration.as_secs_f64() * 1000.0
                    );
                }

                let mut slow_callbacks = self.slow_callbacks.write();
                slow_callbacks.push_back(record);

                // Trim to max size
                while slow_callbacks.len() > config.max_slow_callbacks {
                    slow_callbacks.pop_front();
                }
            }
        }
    }

    /// Record task creation
    pub fn task_created(&self) {
        self.tasks_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Record task completion
    pub fn task_completed(&self) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record task cancellation
    pub fn task_cancelled(&self) {
        self.tasks_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    /// Record loop iteration
    pub fn iteration(&self) {
        self.iterations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get recent slow callbacks
    pub fn get_slow_callbacks(&self) -> Vec<SlowCallback> {
        self.slow_callbacks.read().iter().cloned().collect()
    }

    /// Clear slow callback history
    pub fn clear_slow_callbacks(&self) {
        self.slow_callbacks.write().clear();
    }

    /// Get current statistics
    pub fn get_statistics(&self) -> LoopStatistics {
        let mut stats = self.stats.read().clone();
        stats.callbacks_executed = self.callbacks_counter.load(Ordering::Relaxed);
        stats.slow_callbacks_count = self.slow_counter.load(Ordering::Relaxed);
        stats.tasks_created = self.tasks_created.load(Ordering::Relaxed);
        stats.tasks_completed = self.tasks_completed.load(Ordering::Relaxed);
        stats.tasks_cancelled = self.tasks_cancelled.load(Ordering::Relaxed);
        stats.iterations = self.iterations.load(Ordering::Relaxed);
        stats.pending_tasks = stats.tasks_created - stats.tasks_completed - stats.tasks_cancelled;
        stats
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        self.callbacks_counter.store(0, Ordering::Relaxed);
        self.slow_counter.store(0, Ordering::Relaxed);
        self.tasks_created.store(0, Ordering::Relaxed);
        self.tasks_completed.store(0, Ordering::Relaxed);
        self.tasks_cancelled.store(0, Ordering::Relaxed);
        self.iterations.store(0, Ordering::Relaxed);
        *self.stats.write() = LoopStatistics::default();
    }
}

impl Default for DebugMonitor {
    fn default() -> Self {
        Self::new(DebugConfig::default())
    }
}

/// Shared debug monitor
pub type SharedDebugMonitor = Arc<DebugMonitor>;

/// Create a shared debug monitor
pub fn shared_debug_monitor(config: DebugConfig) -> SharedDebugMonitor {
    Arc::new(DebugMonitor::new(config))
}

// ============================================================================
// Callback Timer
// ============================================================================

/// Timer for measuring callback duration
pub struct CallbackTimer {
    start: Instant,
}

impl CallbackTimer {
    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Capture current stack trace
fn capture_stack_trace() -> String {
    // Simplified - in a real implementation, use backtrace crate
    "Stack trace not available".to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_config() {
        let config = DebugConfig::new()
            .enabled(true)
            .slow_callback_duration(Duration::from_millis(50))
            .log_slow_callbacks(true);

        assert!(config.enabled);
        assert_eq!(config.slow_callback_duration, Duration::from_millis(50));
        assert!(config.log_slow_callbacks);
    }

    #[test]
    fn test_debug_monitor() {
        let monitor = DebugMonitor::new(DebugConfig::new().enabled(true));

        let timer = monitor.start_callback();
        assert!(timer.is_some());

        // Simulate callback
        std::thread::sleep(Duration::from_millis(10));
        monitor.end_callback(timer, "test_callback");

        let stats = monitor.get_statistics();
        assert_eq!(stats.callbacks_executed, 1);
    }

    #[test]
    fn test_slow_callback_detection() {
        let config = DebugConfig::new()
            .enabled(true)
            .slow_callback_duration(Duration::from_millis(10))
            .log_slow_callbacks(false);

        let monitor = DebugMonitor::new(config);

        let timer = monitor.start_callback();
        std::thread::sleep(Duration::from_millis(20));
        monitor.end_callback(timer, "slow_callback");

        let slow = monitor.get_slow_callbacks();
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].name, "slow_callback");
    }

    #[test]
    fn test_loop_statistics() {
        let monitor = DebugMonitor::default();

        monitor.task_created();
        monitor.task_created();
        monitor.task_completed();
        monitor.iteration();

        let stats = monitor.get_statistics();
        assert_eq!(stats.tasks_created, 2);
        assert_eq!(stats.tasks_completed, 1);
        assert_eq!(stats.pending_tasks, 1);
        assert_eq!(stats.iterations, 1);
    }

    #[test]
    fn test_statistics_reset() {
        let monitor = DebugMonitor::default();
        monitor.task_created();
        monitor.iteration();

        let stats = monitor.get_statistics();
        assert!(stats.tasks_created > 0);

        monitor.reset_statistics();
        let stats = monitor.get_statistics();
        assert_eq!(stats.tasks_created, 0);
    }
}
