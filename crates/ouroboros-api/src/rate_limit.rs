//! Request rate limiting
//!
//! Provides rate limiting middleware with token bucket and sliding window algorithms.
//! Supports per-IP, per-user, and custom key-based limiting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// ============================================================================
// Rate Limit Configuration
// ============================================================================

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window: Duration,
    /// Algorithm to use
    pub algorithm: RateLimitAlgorithm,
    /// Whether to include headers in response
    pub include_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            algorithm: RateLimitAlgorithm::SlidingWindow,
            include_headers: true,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            ..Default::default()
        }
    }

    /// Set requests per second
    pub fn per_second(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(1))
    }

    /// Set requests per minute
    pub fn per_minute(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(60))
    }

    /// Set requests per hour
    pub fn per_hour(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(3600))
    }

    /// Set algorithm
    pub fn algorithm(mut self, algorithm: RateLimitAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set include headers
    pub fn include_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }

    /// Parse from string like "10/minute" or "100/hour"
    pub fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let count: u32 = parts[0].parse().ok()?;
        let window = match parts[1].trim().to_lowercase().as_str() {
            "second" | "sec" | "s" => Duration::from_secs(1),
            "minute" | "min" | "m" => Duration::from_secs(60),
            "hour" | "hr" | "h" => Duration::from_secs(3600),
            "day" | "d" => Duration::from_secs(86400),
            _ => return None,
        };

        Some(Self::new(count, window))
    }
}

/// Rate limit algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateLimitAlgorithm {
    /// Fixed window counter
    FixedWindow,
    /// Sliding window log
    #[default]
    SlidingWindow,
    /// Token bucket
    TokenBucket,
}

// ============================================================================
// Rate Limit Result
// ============================================================================

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in current window
    pub remaining: u32,
    /// Total requests allowed
    pub limit: u32,
    /// When the current window resets (seconds from now)
    pub reset_after: Duration,
    /// Retry after duration (only if not allowed)
    pub retry_after: Option<Duration>,
}

impl RateLimitResult {
    /// Create an allowed result
    pub fn allowed(remaining: u32, limit: u32, reset_after: Duration) -> Self {
        Self {
            allowed: true,
            remaining,
            limit,
            reset_after,
            retry_after: None,
        }
    }

    /// Create a denied result
    pub fn denied(limit: u32, retry_after: Duration) -> Self {
        Self {
            allowed: false,
            remaining: 0,
            limit,
            reset_after: retry_after,
            retry_after: Some(retry_after),
        }
    }

    /// Get headers to include in response
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            ("X-RateLimit-Limit", self.limit.to_string()),
            ("X-RateLimit-Remaining", self.remaining.to_string()),
            ("X-RateLimit-Reset", self.reset_after.as_secs().to_string()),
        ];

        if let Some(retry) = self.retry_after {
            headers.push(("Retry-After", retry.as_secs().to_string()));
        }

        headers
    }
}

// ============================================================================
// Rate Limiter
// ============================================================================

/// Rate limiter with configurable algorithm
pub struct RateLimiter {
    config: RateLimitConfig,
    store: RwLock<HashMap<String, RateLimitEntry>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            store: RwLock::new(HashMap::new()),
        }
    }

    /// Create with requests per second
    pub fn per_second(requests: u32) -> Self {
        Self::new(RateLimitConfig::per_second(requests))
    }

    /// Create with requests per minute
    pub fn per_minute(requests: u32) -> Self {
        Self::new(RateLimitConfig::per_minute(requests))
    }

    /// Create with requests per hour
    pub fn per_hour(requests: u32) -> Self {
        Self::new(RateLimitConfig::per_hour(requests))
    }

    /// Check if a request is allowed for a key
    pub fn check(&self, key: &str) -> RateLimitResult {
        match self.config.algorithm {
            RateLimitAlgorithm::FixedWindow => self.check_fixed_window(key),
            RateLimitAlgorithm::SlidingWindow => self.check_sliding_window(key),
            RateLimitAlgorithm::TokenBucket => self.check_token_bucket(key),
        }
    }

    /// Check and consume if allowed
    pub fn acquire(&self, key: &str) -> RateLimitResult {
        let result = self.check(key);
        if result.allowed {
            self.record(key);
        }
        result
    }

    /// Record a request for a key
    pub fn record(&self, key: &str) {
        let mut store = self.store.write();
        let entry = store
            .entry(key.to_string())
            .or_insert_with(|| RateLimitEntry::new(&self.config));
        entry.record();
    }

    /// Reset limit for a key
    pub fn reset(&self, key: &str) {
        self.store.write().remove(key);
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.store.write().clear();
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        let mut store = self.store.write();
        let now = Instant::now();
        store.retain(|_, entry| {
            entry.window_start + self.config.window * 2 > now
        });
    }

    fn check_fixed_window(&self, key: &str) -> RateLimitResult {
        let store = self.store.read();
        let now = Instant::now();

        if let Some(entry) = store.get(key) {
            let window_elapsed = now.duration_since(entry.window_start);

            if window_elapsed >= self.config.window {
                // Window expired, allow
                RateLimitResult::allowed(
                    self.config.max_requests - 1,
                    self.config.max_requests,
                    self.config.window,
                )
            } else if entry.count >= self.config.max_requests {
                // Limit exceeded
                let reset_after = self.config.window - window_elapsed;
                RateLimitResult::denied(self.config.max_requests, reset_after)
            } else {
                // Within limits
                let reset_after = self.config.window - window_elapsed;
                RateLimitResult::allowed(
                    self.config.max_requests - entry.count - 1,
                    self.config.max_requests,
                    reset_after,
                )
            }
        } else {
            // New key
            RateLimitResult::allowed(
                self.config.max_requests - 1,
                self.config.max_requests,
                self.config.window,
            )
        }
    }

    fn check_sliding_window(&self, key: &str) -> RateLimitResult {
        let store = self.store.read();
        let now = Instant::now();

        if let Some(entry) = store.get(key) {
            // Count requests within the window
            let cutoff = now - self.config.window;
            let count: u32 = entry
                .timestamps
                .iter()
                .filter(|&&t| t > cutoff)
                .count() as u32;

            if count >= self.config.max_requests {
                // Find when oldest request expires
                let oldest = entry
                    .timestamps
                    .iter()
                    .filter(|&&t| t > cutoff)
                    .min()
                    .copied()
                    .unwrap_or(now);
                let retry_after = self.config.window - now.duration_since(oldest);
                RateLimitResult::denied(self.config.max_requests, retry_after)
            } else {
                // Within limits
                let remaining = self.config.max_requests - count - 1;
                RateLimitResult::allowed(
                    remaining,
                    self.config.max_requests,
                    self.config.window,
                )
            }
        } else {
            // New key
            RateLimitResult::allowed(
                self.config.max_requests - 1,
                self.config.max_requests,
                self.config.window,
            )
        }
    }

    fn check_token_bucket(&self, key: &str) -> RateLimitResult {
        let store = self.store.read();
        let now = Instant::now();

        if let Some(entry) = store.get(key) {
            let elapsed = now.duration_since(entry.last_update);
            let refill_rate = self.config.max_requests as f64 / self.config.window.as_secs_f64();
            let new_tokens = (elapsed.as_secs_f64() * refill_rate) as u32;
            let tokens = (entry.tokens + new_tokens).min(self.config.max_requests);

            if tokens > 0 {
                RateLimitResult::allowed(
                    tokens - 1,
                    self.config.max_requests,
                    Duration::from_secs_f64(1.0 / refill_rate),
                )
            } else {
                let retry_after = Duration::from_secs_f64(1.0 / refill_rate);
                RateLimitResult::denied(self.config.max_requests, retry_after)
            }
        } else {
            RateLimitResult::allowed(
                self.config.max_requests - 1,
                self.config.max_requests,
                self.config.window,
            )
        }
    }
}

/// Shared rate limiter
pub type SharedRateLimiter = Arc<RateLimiter>;

/// Create a shared rate limiter
pub fn shared_rate_limiter(config: RateLimitConfig) -> SharedRateLimiter {
    Arc::new(RateLimiter::new(config))
}

// ============================================================================
// Rate Limit Entry
// ============================================================================

struct RateLimitEntry {
    /// Request count (fixed window)
    count: u32,
    /// Window start time (fixed window)
    window_start: Instant,
    /// Timestamps of recent requests (sliding window)
    timestamps: Vec<Instant>,
    /// Available tokens (token bucket)
    tokens: u32,
    /// Last update time (token bucket)
    last_update: Instant,
}

impl RateLimitEntry {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            timestamps: Vec::new(),
            tokens: config.max_requests,
            last_update: Instant::now(),
        }
    }

    fn record(&mut self) {
        let now = Instant::now();

        // Fixed window
        self.count += 1;

        // Sliding window
        self.timestamps.push(now);

        // Token bucket
        if self.tokens > 0 {
            self.tokens -= 1;
        }
        self.last_update = now;

        // Cleanup old timestamps
        if self.timestamps.len() > 1000 {
            let cutoff = now - Duration::from_secs(3600);
            self.timestamps.retain(|&t| t > cutoff);
        }
    }
}

// ============================================================================
// Key Extractors
// ============================================================================

/// Extract rate limit key from request
pub trait KeyExtractor: Send + Sync {
    /// Extract the key for rate limiting
    fn extract(&self, ip: Option<&str>, user_id: Option<&str>, path: &str) -> String;
}

/// IP-based key extractor
#[derive(Debug, Clone, Default)]
pub struct IpKeyExtractor;

impl KeyExtractor for IpKeyExtractor {
    fn extract(&self, ip: Option<&str>, _user_id: Option<&str>, _path: &str) -> String {
        ip.unwrap_or("unknown").to_string()
    }
}

/// User-based key extractor
#[derive(Debug, Clone, Default)]
pub struct UserKeyExtractor;

impl KeyExtractor for UserKeyExtractor {
    fn extract(&self, ip: Option<&str>, user_id: Option<&str>, _path: &str) -> String {
        user_id.or(ip).unwrap_or("unknown").to_string()
    }
}

/// Path-based key extractor (per IP per path)
#[derive(Debug, Clone, Default)]
pub struct PathKeyExtractor;

impl KeyExtractor for PathKeyExtractor {
    fn extract(&self, ip: Option<&str>, _user_id: Option<&str>, path: &str) -> String {
        format!("{}:{}", ip.unwrap_or("unknown"), path)
    }
}

/// Custom key extractor using a closure
pub struct FnKeyExtractor<F>
where
    F: Fn(Option<&str>, Option<&str>, &str) -> String + Send + Sync,
{
    f: F,
}

impl<F> FnKeyExtractor<F>
where
    F: Fn(Option<&str>, Option<&str>, &str) -> String + Send + Sync,
{
    /// Create a new function-based key extractor
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> KeyExtractor for FnKeyExtractor<F>
where
    F: Fn(Option<&str>, Option<&str>, &str) -> String + Send + Sync,
{
    fn extract(&self, ip: Option<&str>, user_id: Option<&str>, path: &str) -> String {
        (self.f)(ip, user_id, path)
    }
}

// ============================================================================
// Rate Limit Tiers
// ============================================================================

/// Rate limit tier for different user types
#[derive(Debug, Clone)]
pub struct RateLimitTier {
    /// Tier name
    pub name: String,
    /// Rate limit configuration
    pub config: RateLimitConfig,
}

impl RateLimitTier {
    /// Create a new tier
    pub fn new(name: impl Into<String>, config: RateLimitConfig) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }

    /// Create anonymous tier
    pub fn anonymous(requests_per_minute: u32) -> Self {
        Self::new("anonymous", RateLimitConfig::per_minute(requests_per_minute))
    }

    /// Create authenticated tier
    pub fn authenticated(requests_per_minute: u32) -> Self {
        Self::new("authenticated", RateLimitConfig::per_minute(requests_per_minute))
    }

    /// Create premium tier
    pub fn premium(requests_per_minute: u32) -> Self {
        Self::new("premium", RateLimitConfig::per_minute(requests_per_minute))
    }
}

/// Tiered rate limiter
pub struct TieredRateLimiter {
    tiers: HashMap<String, RateLimiter>,
    default_tier: String,
}

impl TieredRateLimiter {
    /// Create a new tiered rate limiter
    pub fn new(tiers: Vec<RateLimitTier>, default_tier: impl Into<String>) -> Self {
        let tier_map = tiers
            .into_iter()
            .map(|t| (t.name.clone(), RateLimiter::new(t.config)))
            .collect();

        Self {
            tiers: tier_map,
            default_tier: default_tier.into(),
        }
    }

    /// Check rate limit for a key in a specific tier
    pub fn check(&self, tier: &str, key: &str) -> RateLimitResult {
        let limiter = self.tiers.get(tier)
            .or_else(|| self.tiers.get(&self.default_tier));

        match limiter {
            Some(l) => l.check(key),
            None => RateLimitResult::allowed(u32::MAX, u32::MAX, Duration::from_secs(0)),
        }
    }

    /// Acquire for a key in a specific tier
    pub fn acquire(&self, tier: &str, key: &str) -> RateLimitResult {
        let limiter = self.tiers.get(tier)
            .or_else(|| self.tiers.get(&self.default_tier));

        match limiter {
            Some(l) => l.acquire(key),
            None => RateLimitResult::allowed(u32::MAX, u32::MAX, Duration::from_secs(0)),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_from_str() {
        let config = RateLimitConfig::from_str("10/minute").unwrap();
        assert_eq!(config.max_requests, 10);
        assert_eq!(config.window, Duration::from_secs(60));

        let config = RateLimitConfig::from_str("100/hour").unwrap();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window, Duration::from_secs(3600));

        assert!(RateLimitConfig::from_str("invalid").is_none());
    }

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::per_minute(3);

        // First 3 requests should pass
        let result = limiter.acquire("test");
        assert!(result.allowed);
        assert_eq!(result.remaining, 2);

        let result = limiter.acquire("test");
        assert!(result.allowed);
        assert_eq!(result.remaining, 1);

        let result = limiter.acquire("test");
        assert!(result.allowed);
        assert_eq!(result.remaining, 0);

        // 4th request should be denied
        let result = limiter.acquire("test");
        assert!(!result.allowed);
        assert!(result.retry_after.is_some());
    }

    #[test]
    fn test_rate_limiter_different_keys() {
        let limiter = RateLimiter::per_minute(2);

        // Different keys have separate limits
        let result = limiter.acquire("key1");
        assert!(result.allowed);

        let result = limiter.acquire("key2");
        assert!(result.allowed);

        let result = limiter.acquire("key1");
        assert!(result.allowed);

        let result = limiter.acquire("key2");
        assert!(result.allowed);
    }

    #[test]
    fn test_rate_limit_result_headers() {
        let result = RateLimitResult::allowed(5, 10, Duration::from_secs(30));
        let headers = result.headers();

        assert!(headers.iter().any(|(k, v)| *k == "X-RateLimit-Limit" && v == "10"));
        assert!(headers.iter().any(|(k, v)| *k == "X-RateLimit-Remaining" && v == "5"));
        assert!(headers.iter().any(|(k, v)| *k == "X-RateLimit-Reset" && v == "30"));
    }

    #[test]
    fn test_ip_key_extractor() {
        let extractor = IpKeyExtractor;
        assert_eq!(extractor.extract(Some("192.168.1.1"), None, "/api"), "192.168.1.1");
        assert_eq!(extractor.extract(None, Some("user1"), "/api"), "unknown");
    }

    #[test]
    fn test_user_key_extractor() {
        let extractor = UserKeyExtractor;
        assert_eq!(extractor.extract(Some("192.168.1.1"), Some("user1"), "/api"), "user1");
        assert_eq!(extractor.extract(Some("192.168.1.1"), None, "/api"), "192.168.1.1");
    }

    #[test]
    fn test_path_key_extractor() {
        let extractor = PathKeyExtractor;
        assert_eq!(extractor.extract(Some("192.168.1.1"), None, "/api/users"), "192.168.1.1:/api/users");
    }

    #[test]
    fn test_tiered_rate_limiter() {
        let limiter = TieredRateLimiter::new(
            vec![
                RateLimitTier::anonymous(2),
                RateLimitTier::authenticated(5),
            ],
            "anonymous",
        );

        // Anonymous gets 2 requests
        let result = limiter.acquire("anonymous", "user1");
        assert!(result.allowed);
        let result = limiter.acquire("anonymous", "user1");
        assert!(result.allowed);
        let result = limiter.acquire("anonymous", "user1");
        assert!(!result.allowed);

        // Authenticated gets 5 requests
        for _ in 0..5 {
            let result = limiter.acquire("authenticated", "user2");
            assert!(result.allowed);
        }
        let result = limiter.acquire("authenticated", "user2");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rate_limit_reset() {
        let limiter = RateLimiter::per_minute(1);

        // Use up the limit
        limiter.acquire("test");
        let result = limiter.acquire("test");
        assert!(!result.allowed);

        // Reset
        limiter.reset("test");

        // Should work again
        let result = limiter.acquire("test");
        assert!(result.allowed);
    }
}
