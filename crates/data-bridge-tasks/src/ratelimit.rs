//! Rate limiting for task execution.
//!
//! Provides rate limiting to control task execution frequency.
//! Supports multiple algorithms and both in-memory and distributed (Redis) backends.
//!
//! # Example
//! ```rust,ignore
//! use data_bridge_tasks::ratelimit::{RateLimiter, RateLimitConfig, TokenBucket};
//!
//! // Create a rate limiter: 10 tasks per second
//! let limiter = TokenBucket::new(RateLimitConfig {
//!     rate: 10.0,           // 10 tokens per second
//!     capacity: 20,         // Allow burst of up to 20
//!     ..Default::default()
//! });
//!
//! // Check if we can proceed
//! if limiter.acquire().await {
//!     // Execute task
//! } else {
//!     // Rate limited, retry later
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Tokens per second (rate)
    pub rate: f64,
    /// Maximum burst capacity
    pub capacity: u32,
    /// Key for this rate limit (task name, queue, etc.)
    #[serde(default)]
    pub key: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            rate: 10.0,
            capacity: 10,
            key: "default".to_string(),
        }
    }
}

impl RateLimitConfig {
    /// Create config for N tasks per second
    pub fn per_second(n: u32) -> Self {
        Self {
            rate: n as f64,
            capacity: n,
            key: "default".to_string(),
        }
    }

    /// Create config for N tasks per minute
    pub fn per_minute(n: u32) -> Self {
        Self {
            rate: n as f64 / 60.0,
            capacity: n.min(100), // Reasonable burst
            key: "default".to_string(),
        }
    }

    /// Create config for N tasks per hour
    pub fn per_hour(n: u32) -> Self {
        Self {
            rate: n as f64 / 3600.0,
            capacity: (n / 60).clamp(1, 100),
            key: "default".to_string(),
        }
    }

    /// Set the key for this rate limit
    pub fn with_key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Time to wait before retry (if not allowed)
    pub retry_after: Option<Duration>,
    /// Remaining tokens/requests in current window
    pub remaining: u32,
    /// Total limit
    pub limit: u32,
}

impl RateLimitResult {
    /// Create an allowed result
    pub fn allowed(remaining: u32, limit: u32) -> Self {
        Self {
            allowed: true,
            retry_after: None,
            remaining,
            limit,
        }
    }

    /// Create a denied result
    pub fn denied(retry_after: Duration, limit: u32) -> Self {
        Self {
            allowed: false,
            retry_after: Some(retry_after),
            remaining: 0,
            limit,
        }
    }
}

/// Rate limiter trait
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Try to acquire a permit to execute
    async fn acquire(&self, key: &str) -> RateLimitResult;

    /// Try to acquire multiple permits
    async fn acquire_many(&self, key: &str, count: u32) -> RateLimitResult;

    /// Get current state without consuming
    async fn peek(&self, key: &str) -> RateLimitResult;

    /// Reset the rate limiter for a key
    async fn reset(&self, key: &str);
}

/// Token bucket rate limiter (in-memory)
///
/// Allows smooth rate limiting with burst capacity.
/// Tokens are added at a constant rate up to the capacity.
pub struct TokenBucket {
    config: RateLimitConfig,
    buckets: RwLock<HashMap<String, BucketState>>,
}

#[derive(Debug, Clone)]
struct BucketState {
    tokens: f64,
    last_update: Instant,
}

impl TokenBucket {
    /// Create a new token bucket rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: RwLock::new(HashMap::new()),
        }
    }

    /// Create with rate per second
    pub fn per_second(rate: u32) -> Self {
        Self::new(RateLimitConfig::per_second(rate))
    }

    /// Create with rate per minute
    pub fn per_minute(rate: u32) -> Self {
        Self::new(RateLimitConfig::per_minute(rate))
    }

    fn refill(&self, state: &mut BucketState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_update).as_secs_f64();
        let new_tokens = elapsed * self.config.rate;
        state.tokens = (state.tokens + new_tokens).min(self.config.capacity as f64);
        state.last_update = now;
    }
}

#[async_trait]
impl RateLimiter for TokenBucket {
    async fn acquire(&self, key: &str) -> RateLimitResult {
        self.acquire_many(key, 1).await
    }

    async fn acquire_many(&self, key: &str, count: u32) -> RateLimitResult {
        let mut buckets = self.buckets.write().await;
        let state = buckets.entry(key.to_string()).or_insert_with(|| BucketState {
            tokens: self.config.capacity as f64,
            last_update: Instant::now(),
        });

        self.refill(state);

        let count_f64 = count as f64;
        if state.tokens >= count_f64 {
            state.tokens -= count_f64;
            RateLimitResult::allowed(state.tokens as u32, self.config.capacity)
        } else {
            // Calculate wait time
            let tokens_needed = count_f64 - state.tokens;
            let wait_secs = tokens_needed / self.config.rate;
            RateLimitResult::denied(
                Duration::from_secs_f64(wait_secs),
                self.config.capacity,
            )
        }
    }

    async fn peek(&self, key: &str) -> RateLimitResult {
        let mut buckets = self.buckets.write().await;
        let state = buckets.entry(key.to_string()).or_insert_with(|| BucketState {
            tokens: self.config.capacity as f64,
            last_update: Instant::now(),
        });

        self.refill(state);
        RateLimitResult::allowed(state.tokens as u32, self.config.capacity)
    }

    async fn reset(&self, key: &str) {
        let mut buckets = self.buckets.write().await;
        buckets.insert(
            key.to_string(),
            BucketState {
                tokens: self.config.capacity as f64,
                last_update: Instant::now(),
            },
        );
    }
}

/// Sliding window rate limiter (in-memory)
///
/// More accurate than fixed windows, tracks requests in a sliding time window.
pub struct SlidingWindow {
    config: RateLimitConfig,
    window_duration: Duration,
    windows: RwLock<HashMap<String, WindowState>>,
}

#[derive(Debug, Clone)]
struct WindowState {
    /// Timestamps of requests in current window
    requests: Vec<Instant>,
}

impl SlidingWindow {
    /// Create a new sliding window rate limiter
    pub fn new(config: RateLimitConfig, window: Duration) -> Self {
        Self {
            config,
            window_duration: window,
            windows: RwLock::new(HashMap::new()),
        }
    }

    /// Create with rate per second (1 second window)
    pub fn per_second(rate: u32) -> Self {
        Self::new(
            RateLimitConfig {
                rate: rate as f64,
                capacity: rate,
                key: "default".to_string(),
            },
            Duration::from_secs(1),
        )
    }

    /// Create with rate per minute (1 minute window)
    pub fn per_minute(rate: u32) -> Self {
        Self::new(
            RateLimitConfig {
                rate: rate as f64 / 60.0,
                capacity: rate,
                key: "default".to_string(),
            },
            Duration::from_secs(60),
        )
    }

    fn cleanup(&self, state: &mut WindowState) {
        let cutoff = Instant::now() - self.window_duration;
        state.requests.retain(|&t| t > cutoff);
    }
}

#[async_trait]
impl RateLimiter for SlidingWindow {
    async fn acquire(&self, key: &str) -> RateLimitResult {
        self.acquire_many(key, 1).await
    }

    async fn acquire_many(&self, key: &str, count: u32) -> RateLimitResult {
        let mut windows = self.windows.write().await;
        let state = windows.entry(key.to_string()).or_insert_with(|| WindowState {
            requests: Vec::new(),
        });

        self.cleanup(state);

        let current_count = state.requests.len() as u32;
        if current_count + count <= self.config.capacity {
            let now = Instant::now();
            for _ in 0..count {
                state.requests.push(now);
            }
            RateLimitResult::allowed(
                self.config.capacity - current_count - count,
                self.config.capacity,
            )
        } else {
            // Calculate when oldest request will expire
            let retry_after = if let Some(&oldest) = state.requests.first() {
                let expires_at = oldest + self.window_duration;
                expires_at.saturating_duration_since(Instant::now())
            } else {
                Duration::from_millis(100)
            };
            RateLimitResult::denied(retry_after, self.config.capacity)
        }
    }

    async fn peek(&self, key: &str) -> RateLimitResult {
        let mut windows = self.windows.write().await;
        let state = windows.entry(key.to_string()).or_insert_with(|| WindowState {
            requests: Vec::new(),
        });

        self.cleanup(state);
        let current_count = state.requests.len() as u32;
        RateLimitResult::allowed(
            self.config.capacity.saturating_sub(current_count),
            self.config.capacity,
        )
    }

    async fn reset(&self, key: &str) {
        let mut windows = self.windows.write().await;
        windows.insert(key.to_string(), WindowState { requests: Vec::new() });
    }
}

/// Composite rate limiter that manages multiple rate limits
pub struct RateLimitManager {
    /// Per-task rate limits
    task_limits: HashMap<String, Arc<dyn RateLimiter>>,
    /// Per-queue rate limits
    queue_limits: HashMap<String, Arc<dyn RateLimiter>>,
    /// Global rate limit
    global_limit: Option<Arc<dyn RateLimiter>>,
}

impl Default for RateLimitManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitManager {
    /// Create a new rate limit manager
    pub fn new() -> Self {
        Self {
            task_limits: HashMap::new(),
            queue_limits: HashMap::new(),
            global_limit: None,
        }
    }

    /// Add a per-task rate limit
    pub fn task_limit<R: RateLimiter + 'static>(mut self, task_name: &str, limiter: R) -> Self {
        self.task_limits.insert(task_name.to_string(), Arc::new(limiter));
        self
    }

    /// Add a per-queue rate limit
    pub fn queue_limit<R: RateLimiter + 'static>(mut self, queue: &str, limiter: R) -> Self {
        self.queue_limits.insert(queue.to_string(), Arc::new(limiter));
        self
    }

    /// Set global rate limit
    pub fn global_limit<R: RateLimiter + 'static>(mut self, limiter: R) -> Self {
        self.global_limit = Some(Arc::new(limiter));
        self
    }

    /// Check if a task can be executed
    pub async fn check(&self, task_name: &str, queue: &str) -> RateLimitResult {
        // Check global limit first
        if let Some(global) = &self.global_limit {
            let result = global.acquire("global").await;
            if !result.allowed {
                return result;
            }
        }

        // Check queue limit
        if let Some(limiter) = self.queue_limits.get(queue) {
            let result = limiter.acquire(queue).await;
            if !result.allowed {
                return result;
            }
        }

        // Check task limit
        if let Some(limiter) = self.task_limits.get(task_name) {
            return limiter.acquire(task_name).await;
        }

        // All checks passed
        RateLimitResult::allowed(u32::MAX, u32::MAX)
    }

    /// Check without consuming (for preview)
    pub async fn peek(&self, task_name: &str, queue: &str) -> RateLimitResult {
        if let Some(global) = &self.global_limit {
            let result = global.peek("global").await;
            if !result.allowed {
                return result;
            }
        }

        if let Some(limiter) = self.queue_limits.get(queue) {
            let result = limiter.peek(queue).await;
            if !result.allowed {
                return result;
            }
        }

        if let Some(limiter) = self.task_limits.get(task_name) {
            return limiter.peek(task_name).await;
        }

        RateLimitResult::allowed(u32::MAX, u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let limiter = TokenBucket::per_second(5);

        // Should allow first 5 requests
        for i in 0..5 {
            let result = limiter.acquire("test").await;
            assert!(result.allowed, "Request {} should be allowed", i);
        }

        // 6th request should be denied
        let result = limiter.acquire("test").await;
        assert!(!result.allowed, "6th request should be denied");
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let limiter = TokenBucket::new(RateLimitConfig {
            rate: 100.0, // 100 per second
            capacity: 5,
            key: "test".to_string(),
        });

        // Use all tokens
        for _ in 0..5 {
            limiter.acquire("test").await;
        }

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should have some tokens now
        let result = limiter.acquire("test").await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_sliding_window_basic() {
        let limiter = SlidingWindow::per_second(3);

        // Should allow first 3 requests
        for _ in 0..3 {
            let result = limiter.acquire("test").await;
            assert!(result.allowed);
        }

        // 4th should be denied
        let result = limiter.acquire("test").await;
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_manager() {
        let manager = RateLimitManager::new()
            .task_limit("slow_task", TokenBucket::per_second(1))
            .queue_limit("limited", SlidingWindow::per_second(2))
            .global_limit(TokenBucket::per_second(100));

        // Task limit
        let result = manager.check("slow_task", "default").await;
        assert!(result.allowed);

        let result = manager.check("slow_task", "default").await;
        assert!(!result.allowed);

        // Queue limit (different task, same queue)
        let result = manager.check("fast_task", "limited").await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_per_minute_config() {
        let config = RateLimitConfig::per_minute(60);
        assert_eq!(config.rate, 1.0); // 1 per second
    }

    #[tokio::test]
    async fn test_reset() {
        let limiter = TokenBucket::per_second(1);

        // Use the token
        limiter.acquire("test").await;

        // Should be denied
        let result = limiter.acquire("test").await;
        assert!(!result.allowed);

        // Reset
        limiter.reset("test").await;

        // Should be allowed again
        let result = limiter.acquire("test").await;
        assert!(result.allowed);
    }
}
