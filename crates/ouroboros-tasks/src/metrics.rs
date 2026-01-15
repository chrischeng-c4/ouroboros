//! Prometheus metrics for task queue observability

#[cfg(feature = "metrics")]
use prometheus::{
    CounterVec, HistogramVec, GaugeVec,
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    Opts, HistogramOpts,
};
#[cfg(feature = "metrics")]
use once_cell::sync::Lazy;

/// Task execution metrics
#[cfg(feature = "metrics")]
pub struct TaskMetrics {
    /// Total tasks published
    pub tasks_published: CounterVec,
    /// Total tasks executed
    pub tasks_executed: CounterVec,
    /// Task execution duration in seconds
    pub task_duration_seconds: HistogramVec,
    /// Tasks currently in progress
    pub tasks_in_progress: GaugeVec,
    /// Task retries count
    pub task_retries: CounterVec,
    /// Task failures count
    pub task_failures: CounterVec,
}

#[cfg(feature = "metrics")]
impl TaskMetrics {
    pub fn new() -> Self {
        Self {
            tasks_published: register_counter_vec!(
                Opts::new("tasks_published_total", "Total number of tasks published"),
                &["task_name", "queue"]
            ).unwrap(),

            tasks_executed: register_counter_vec!(
                Opts::new("tasks_executed_total", "Total number of tasks executed"),
                &["task_name", "queue", "status"]
            ).unwrap(),

            task_duration_seconds: register_histogram_vec!(
                HistogramOpts::new("task_duration_seconds", "Task execution duration")
                    .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
                &["task_name", "queue"]
            ).unwrap(),

            tasks_in_progress: register_gauge_vec!(
                Opts::new("tasks_in_progress", "Number of tasks currently being executed"),
                &["task_name", "queue"]
            ).unwrap(),

            task_retries: register_counter_vec!(
                Opts::new("task_retries_total", "Total number of task retries"),
                &["task_name", "queue"]
            ).unwrap(),

            task_failures: register_counter_vec!(
                Opts::new("task_failures_total", "Total number of task failures"),
                &["task_name", "queue", "error_type"]
            ).unwrap(),
        }
    }

    pub fn record_published(&self, task_name: &str, queue: &str) {
        self.tasks_published.with_label_values(&[task_name, queue]).inc();
    }

    pub fn record_started(&self, task_name: &str, queue: &str) {
        self.tasks_in_progress.with_label_values(&[task_name, queue]).inc();
    }

    pub fn record_completed(&self, task_name: &str, queue: &str, duration_secs: f64, success: bool) {
        self.tasks_in_progress.with_label_values(&[task_name, queue]).dec();
        self.task_duration_seconds.with_label_values(&[task_name, queue]).observe(duration_secs);
        let status = if success { "success" } else { "failure" };
        self.tasks_executed.with_label_values(&[task_name, queue, status]).inc();
    }

    pub fn record_retry(&self, task_name: &str, queue: &str) {
        self.task_retries.with_label_values(&[task_name, queue]).inc();
    }

    pub fn record_failure(&self, task_name: &str, queue: &str, error_type: &str) {
        self.task_failures.with_label_values(&[task_name, queue, error_type]).inc();
    }
}

#[cfg(feature = "metrics")]
impl Default for TaskMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "metrics")]
pub static METRICS: Lazy<TaskMetrics> = Lazy::new(TaskMetrics::new);

/// Get metrics in Prometheus text format
#[cfg(feature = "metrics")]
pub fn gather_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
