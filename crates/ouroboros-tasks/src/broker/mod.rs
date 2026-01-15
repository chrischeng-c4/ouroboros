//! Message broker implementations
//!
//! Provides traits and implementations for task message brokers.

pub mod config;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::{TaskError, TaskMessage};

/// Delivery model for broker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryModel {
    /// Worker pulls messages from broker (NATS, Pub/Sub pull)
    Pull,
    /// Broker pushes messages to worker via HTTP (Cloud Tasks, Pub/Sub push)
    Push,
}

/// Broker feature capabilities
#[derive(Debug, Clone, Default)]
pub struct BrokerCapabilities {
    /// Supports native delayed/scheduled tasks
    pub delayed_tasks: bool,
    /// Supports dead-letter queues
    pub dead_letter: bool,
    /// Supports message priority
    pub priority: bool,
    /// Supports message batching
    pub batching: bool,
    /// Maximum delay duration (if delayed_tasks is true)
    pub max_delay: Option<Duration>,
}

/// Trait for message broker implementations
#[async_trait]
pub trait Broker: Send + Sync + 'static {
    /// Connect to the broker
    async fn connect(&self) -> Result<(), TaskError>;

    /// Disconnect from the broker
    async fn disconnect(&self) -> Result<(), TaskError>;

    /// Publish a task message to a queue
    async fn publish(&self, queue: &str, message: TaskMessage) -> Result<(), TaskError>;

    /// Check if broker is healthy
    async fn health_check(&self) -> Result<(), TaskError>;

    /// Get the delivery model of this broker
    fn delivery_model(&self) -> DeliveryModel;

    /// Get the capabilities of this broker
    fn capabilities(&self) -> BrokerCapabilities;
}

/// Trait for pull-based brokers (worker fetches messages)
#[async_trait]
pub trait PullBroker: Broker {
    /// Subscribe to a queue and receive messages
    async fn subscribe<H: MessageHandler + 'static>(
        &self,
        queue: &str,
        handler: Arc<H>,
    ) -> Result<SubscriptionHandle, TaskError>;

    /// Acknowledge a message
    async fn ack(&self, delivery_tag: &str) -> Result<(), TaskError>;

    /// Negative acknowledge (requeue or DLQ)
    async fn nack(&self, delivery_tag: &str, requeue: bool) -> Result<(), TaskError>;
}

/// Trait for push-based brokers (broker sends HTTP to worker)
pub trait PushBroker: Broker {
    /// Parse an incoming HTTP request into a BrokerMessage
    fn parse_push_request(&self, headers: &HashMap<String, String>, body: &[u8])
        -> Result<BrokerMessage, TaskError>;

    /// Get HTTP status code for successful ack
    fn ack_status_code(&self) -> u16 { 200 }

    /// Get HTTP status code for nack (retry)
    fn nack_status_code(&self) -> u16 { 500 }

    /// Get the expected endpoint path pattern
    fn endpoint_path(&self) -> &str;
}

/// Trait for brokers with native delayed task support
#[async_trait]
pub trait DelayedBroker: Broker {
    /// Publish with native delay support
    async fn publish_delayed(
        &self,
        queue: &str,
        message: TaskMessage,
        delay: Duration,
    ) -> Result<(), TaskError>;

    /// Publish at specific time
    async fn publish_at(
        &self,
        queue: &str,
        message: TaskMessage,
        eta: DateTime<Utc>,
    ) -> Result<(), TaskError> {
        let now = Utc::now();
        if eta <= now {
            // Execute immediately
            self.publish(queue, message).await
        } else {
            let delay = (eta - now).to_std().unwrap_or(Duration::ZERO);
            self.publish_delayed(queue, message, delay).await
        }
    }
}

/// Message received from the broker
#[derive(Debug, Clone)]
pub struct BrokerMessage {
    /// Delivery tag for acknowledgment
    pub delivery_tag: String,
    /// Task message payload
    pub payload: TaskMessage,
    /// Message headers
    pub headers: HashMap<String, String>,
    /// Timestamp when message was received
    pub timestamp: DateTime<Utc>,
    /// Whether this is a redelivery
    pub redelivered: bool,
}

/// Handler for incoming messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle(&self, message: BrokerMessage) -> Result<(), TaskError>;
}

/// Handle for managing subscriptions
pub struct SubscriptionHandle {
    /// Queue name
    pub queue: String,
    /// Cancellation token
    cancel_token: CancellationToken,
}

impl SubscriptionHandle {
    /// Create a new subscription handle
    pub fn new(queue: String, cancel_token: CancellationToken) -> Self {
        Self { queue, cancel_token }
    }

    /// Cancel the subscription
    pub fn cancel(&self) {
        tracing::info!("Cancelling subscription for queue: {}", self.queue);
        self.cancel_token.cancel();
    }
}

// NATS JetStream broker implementation
#[cfg(feature = "nats")]
pub mod nats;

#[cfg(feature = "nats")]
pub use nats::{NatsBroker, NatsBrokerConfig};

// Google Cloud Pub/Sub broker implementation
#[cfg(feature = "pubsub")]
pub mod pubsub;

#[cfg(feature = "pubsub")]
pub use pubsub::{PubSubPullBroker, PubSubPullConfig};

// Unified broker configuration
pub use config::BrokerConfig;

#[cfg(any(feature = "nats", feature = "pubsub"))]
pub use config::BrokerInstance;
