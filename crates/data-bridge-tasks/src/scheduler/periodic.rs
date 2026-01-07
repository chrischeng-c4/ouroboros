//! Periodic task scheduler
//!
//! Supports both cron expressions and fixed intervals.

use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "scheduler")]
use cron::Schedule;
#[cfg(feature = "scheduler")]
use std::str::FromStr;

use crate::{Broker, TaskError, TaskMessage};

/// Periodic task definition
#[derive(Debug, Clone)]
pub struct PeriodicTask {
    /// Unique name for this periodic task
    pub name: String,
    /// Task name to execute
    pub task_name: String,
    /// Schedule (cron or interval)
    pub schedule: PeriodicSchedule,
    /// Arguments to pass to task
    pub args: serde_json::Value,
    /// Target queue
    pub queue: String,
    /// Whether task is enabled
    pub enabled: bool,
}

/// Schedule type for periodic tasks
#[derive(Debug, Clone)]
pub enum PeriodicSchedule {
    /// Cron expression (e.g., "0 0 * * *" for daily at midnight)
    #[cfg(feature = "scheduler")]
    Cron(String),
    /// Fixed interval in seconds
    Interval(u64),
}

impl PeriodicSchedule {
    /// Calculate next run time from given timestamp
    pub fn next_run(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            #[cfg(feature = "scheduler")]
            PeriodicSchedule::Cron(expr) => {
                let schedule = Schedule::from_str(expr).ok()?;
                schedule.after(&from).next().map(|dt| DateTime::from_naive_utc_and_offset(dt.naive_utc(), Utc))
            }
            PeriodicSchedule::Interval(seconds) => {
                Some(from + chrono::Duration::seconds(*seconds as i64))
            }
        }
    }
}

/// Scheduler for periodic tasks
pub struct PeriodicScheduler<B: Broker> {
    tasks: Vec<PeriodicTask>,
    broker: Arc<B>,
    shutdown: CancellationToken,
}

impl<B: Broker> PeriodicScheduler<B> {
    /// Create a new periodic scheduler
    pub fn new(broker: Arc<B>) -> Self {
        Self {
            tasks: Vec::new(),
            broker,
            shutdown: CancellationToken::new(),
        }
    }

    /// Add a periodic task
    pub fn add_task(&mut self, task: PeriodicTask) {
        tracing::info!("Adding periodic task: {}", task.name);
        self.tasks.push(task);
    }

    /// Remove a periodic task by name
    pub fn remove_task(&mut self, name: &str) -> Option<PeriodicTask> {
        if let Some(pos) = self.tasks.iter().position(|t| t.name == name) {
            tracing::info!("Removing periodic task: {}", name);
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<(), TaskError> {
        if self.tasks.is_empty() {
            tracing::warn!("No periodic tasks to schedule");
            return Ok(());
        }

        let broker = self.broker.clone();
        let tasks = self.tasks.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            // Calculate next run times for all tasks
            let mut next_runs: Vec<(usize, DateTime<Utc>)> = tasks
                .iter()
                .enumerate()
                .filter(|(_, task)| task.enabled)
                .filter_map(|(idx, task)| {
                    task.schedule.next_run(Utc::now()).map(|next| (idx, next))
                })
                .collect();

            tracing::info!("Periodic scheduler started with {} tasks", next_runs.len());

            loop {
                if next_runs.is_empty() {
                    tracing::warn!("No tasks scheduled, stopping");
                    break;
                }

                // Sort by next run time
                next_runs.sort_by_key(|(_, next)| *next);

                // Get the soonest task
                let (task_idx, next_run) = next_runs[0];
                let task = &tasks[task_idx];

                // Calculate sleep duration
                let now = Utc::now();
                let sleep_duration = if next_run > now {
                    (next_run - now).to_std().unwrap_or(Duration::from_secs(0))
                } else {
                    Duration::from_secs(0)
                };

                // Wait until next run or shutdown
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::info!("Periodic scheduler shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(sleep_duration) => {
                        // Time to run the task
                        tracing::debug!("Running periodic task: {}", task.name);

                        let message = TaskMessage::new(&task.task_name, task.args.clone());

                        match broker.publish(&task.queue, message).await {
                            Ok(_) => {
                                tracing::debug!("Published periodic task: {}", task.name);
                            }
                            Err(e) => {
                                tracing::error!("Failed to publish periodic task {}: {}", task.name, e);
                            }
                        }

                        // Calculate next run time
                        if let Some(next) = task.schedule.next_run(Utc::now()) {
                            next_runs[0].1 = next;
                        } else {
                            // Remove task if no next run (shouldn't happen for valid schedules)
                            tracing::warn!("Task {} has no next run time, removing", task.name);
                            next_runs.remove(0);
                        }
                    }
                }
            }

            tracing::info!("Periodic scheduler stopped");
        });

        Ok(())
    }

    /// Shutdown the scheduler
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_schedule() {
        let schedule = PeriodicSchedule::Interval(60);
        let now = Utc::now();
        let next = schedule.next_run(now).unwrap();
        assert!(next > now);
        assert!((next - now).num_seconds() >= 60);
    }

    #[cfg(feature = "scheduler")]
    #[test]
    fn test_cron_schedule() {
        use std::str::FromStr;

        // The cron crate uses extended format with seconds: "sec min hour day month dow year"
        // Every minute: "0 * * * * *"
        let expr = "0 * * * * *";
        let parsed = Schedule::from_str(expr);
        assert!(parsed.is_ok(), "Failed to parse cron expression: {:?}", parsed.err());

        let cron_schedule = parsed.unwrap();
        let now = Utc::now();

        // Test using upcoming iterator
        let mut upcoming = cron_schedule.upcoming(Utc);
        let next_time = upcoming.next();
        assert!(next_time.is_some(), "No next time from upcoming iterator");

        // Now test our wrapper
        let schedule = PeriodicSchedule::Cron("0 * * * * *".to_string());
        let next = schedule.next_run(now);
        assert!(next.is_some(), "next_run returned None for valid cron expression");
        assert!(next.unwrap() > now);
    }
}
