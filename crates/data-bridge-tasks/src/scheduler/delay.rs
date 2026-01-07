//! Delayed task scheduler
//!
//! Polls scheduled messages and republishes them when ETA is reached.

#[cfg(feature = "nats")]
use std::sync::Arc;
#[cfg(feature = "nats")]
use std::time::Duration;
#[cfg(feature = "nats")]
use chrono::{DateTime, Utc};
#[cfg(feature = "nats")]
use tokio_util::sync::CancellationToken;

#[cfg(feature = "nats")]
use crate::{BrokerMessage, MessageHandler, NatsBroker, PullBroker, TaskError};
#[cfg(feature = "nats")]
use crate::Broker;

/// Configuration for delayed task scheduler
#[cfg(feature = "nats")]
#[derive(Debug, Clone)]
pub struct DelayedTaskConfig {
    /// Poll interval for checking scheduled tasks
    pub poll_interval: Duration,
    /// Batch size for fetching scheduled messages
    pub batch_size: usize,
}

#[cfg(feature = "nats")]
impl Default for DelayedTaskConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            batch_size: 100,
        }
    }
}

/// Scheduler for delayed tasks
///
/// Polls the `tasks.scheduled.*` subjects and republishes
/// messages to their target queues when ETA is reached.
#[cfg(feature = "nats")]
pub struct DelayedTaskScheduler {
    #[allow(dead_code)]
    config: DelayedTaskConfig,
    broker: Arc<NatsBroker>,
    shutdown: CancellationToken,
}

#[cfg(feature = "nats")]
impl DelayedTaskScheduler {
    /// Create a new delayed task scheduler
    pub fn new(broker: Arc<NatsBroker>, config: DelayedTaskConfig) -> Self {
        Self {
            config,
            broker,
            shutdown: CancellationToken::new(),
        }
    }

    /// Start the scheduler (spawns background task)
    pub async fn start(&self) -> Result<(), TaskError> {
        let broker = self.broker.clone();
        let shutdown = self.shutdown.clone();

        // Create handler for scheduled messages
        struct ScheduledMessageHandler {
            broker: Arc<NatsBroker>,
        }

        #[async_trait::async_trait]
        impl MessageHandler for ScheduledMessageHandler {
            async fn handle(&self, message: BrokerMessage) -> Result<(), TaskError> {
                // Check if ETA header exists and if it's time to execute
                if let Some(eta_str) = message.headers.get("eta") {
                    let eta = DateTime::parse_from_rfc3339(eta_str)
                        .map_err(|e| TaskError::Internal(format!("Invalid ETA: {}", e)))?
                        .with_timezone(&Utc);

                    if eta <= Utc::now() {
                        // ETA reached, republish to target queue
                        if let Some(target_queue) = message.headers.get("target-queue") {
                            tracing::debug!(
                                "Republishing scheduled task {} to queue {}",
                                message.payload.id,
                                target_queue
                            );

                            self.broker.publish(target_queue, message.payload.clone()).await?;
                        } else {
                            tracing::error!("Scheduled message missing target-queue header");
                            return Err(TaskError::Internal("Missing target-queue header".to_string()));
                        }
                    } else {
                        // Not ready yet, nack to requeue
                        tracing::trace!("Task {} not ready yet (ETA: {})", message.payload.id, eta);
                        return Err(TaskError::Internal("Not ready yet".to_string()));
                    }
                } else {
                    tracing::error!("Scheduled message missing ETA header");
                    return Err(TaskError::Internal("Missing ETA header".to_string()));
                }

                Ok(())
            }
        }

        let handler = Arc::new(ScheduledMessageHandler {
            broker: broker.clone(),
        });

        // Subscribe to all scheduled queues
        let subscription = broker.subscribe("scheduled.*", handler).await?;

        // Spawn monitoring task
        tokio::spawn(async move {
            shutdown.cancelled().await;
            tracing::info!("Delayed task scheduler shutting down");
            subscription.cancel();
        });

        Ok(())
    }

    /// Shutdown the scheduler
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }
}

#[cfg(all(test, feature = "nats"))]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = DelayedTaskConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(1));
        assert_eq!(config.batch_size, 100);
    }
}
