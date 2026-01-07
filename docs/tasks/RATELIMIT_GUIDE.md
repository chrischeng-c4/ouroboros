# Rate Limiting Guide for data-bridge-tasks

Rate limiting controls task execution frequency to prevent overwhelming downstream services or respecting API rate limits.

## Features

- **Token Bucket Algorithm**: Smooth rate limiting with burst capacity
- **Sliding Window Algorithm**: More accurate time-based limiting
- **Multi-level Limits**: Task-level, queue-level, and global limits
- **Zero Dependencies**: Uses only tokio (already in dependencies)
- **Celery-Compatible**: Similar API to Celery's rate limiting

## Quick Start

### Basic Token Bucket

```rust
use data_bridge_tasks::ratelimit::{TokenBucket, RateLimiter};

// Allow 10 tasks per second
let limiter = TokenBucket::per_second(10);

// Try to execute
let result = limiter.acquire("my_task").await;
if result.allowed {
    // Execute task
    println!("Task allowed! Remaining: {}", result.remaining);
} else {
    // Rate limited
    println!("Rate limited. Retry after: {:?}", result.retry_after);
}
```

### Rate Limit Configurations

```rust
use data_bridge_tasks::ratelimit::RateLimitConfig;

// 100 tasks per second
let config = RateLimitConfig::per_second(100);

// 600 tasks per minute (10/sec with burst capacity)
let config = RateLimitConfig::per_minute(600);

// 3600 tasks per hour (1/sec with burst capacity)
let config = RateLimitConfig::per_hour(3600);

// Custom configuration
let config = RateLimitConfig {
    rate: 10.0,      // 10 tokens per second
    capacity: 50,    // Allow burst of 50
    key: "api_calls".to_string(),
};
```

## Algorithms

### Token Bucket

Best for: Smooth rate limiting with burst capacity.

**How it works:**
- Tokens are added to the bucket at a constant rate
- Each task consumes 1 token
- If no tokens available, task is rate limited
- Allows bursts up to the capacity

```rust
use data_bridge_tasks::ratelimit::{TokenBucket, RateLimitConfig};

let limiter = TokenBucket::new(RateLimitConfig {
    rate: 5.0,        // 5 tokens per second
    capacity: 10,     // Allow burst of 10
    ..Default::default()
});

// First 10 requests succeed immediately (burst)
for i in 0..10 {
    let result = limiter.acquire("test").await;
    assert!(result.allowed);
}

// 11th request will be rate limited
let result = limiter.acquire("test").await;
assert!(!result.allowed);

// Wait and tokens will refill at 5/second
tokio::time::sleep(Duration::from_secs(1)).await;
let result = limiter.acquire("test").await;
assert!(result.allowed);
```

### Sliding Window

Best for: Accurate time-based limiting, strict rate enforcement.

**How it works:**
- Tracks timestamps of all requests in the window
- Removes requests older than the window duration
- Denies requests if count exceeds limit

```rust
use data_bridge_tasks::ratelimit::{SlidingWindow, RateLimitConfig};
use std::time::Duration;

// 100 requests per minute window
let limiter = SlidingWindow::new(
    RateLimitConfig {
        rate: 100.0 / 60.0,
        capacity: 100,
        ..Default::default()
    },
    Duration::from_secs(60),
);

// Or use convenience methods
let limiter = SlidingWindow::per_second(10);
let limiter = SlidingWindow::per_minute(600);
```

## Multi-Level Rate Limiting

Combine task-level, queue-level, and global limits:

```rust
use data_bridge_tasks::ratelimit::{
    RateLimitManager, TokenBucket, SlidingWindow
};

let manager = RateLimitManager::new()
    // Limit specific task to 1/second
    .task_limit("slow_api_call", TokenBucket::per_second(1))

    // Limit entire queue to 10/second
    .queue_limit("api_queue", TokenBucket::per_second(10))

    // Global limit across all tasks: 100/second
    .global_limit(TokenBucket::per_second(100));

// Check all limits before executing
let result = manager.check("slow_api_call", "api_queue").await;
if result.allowed {
    // Execute task
} else {
    // Rate limited by one of the limits
    println!("Retry after: {:?}", result.retry_after);
}
```

## Integration with Workers

Workers can use rate limiting to control execution:

```rust
use data_bridge_tasks::{Worker, WorkerConfig, RateLimitManager, TokenBucket};

let rate_limiter = RateLimitManager::new()
    .task_limit("api_call", TokenBucket::per_second(5));

let mut worker = Worker::new(WorkerConfig::default())
    .with_rate_limiter(rate_limiter);

// Worker will check rate limits before executing tasks
worker.start().await?;
```

## Advanced Usage

### Acquire Multiple Tokens

```rust
// Acquire 5 tokens at once (for batch operations)
let result = limiter.acquire_many("batch_task", 5).await;
```

### Peek Without Consuming

```rust
// Check rate limit status without consuming tokens
let result = limiter.peek("task").await;
println!("Remaining capacity: {}", result.remaining);
```

### Reset Rate Limiter

```rust
// Reset rate limiter for a specific key
limiter.reset("task").await;
```

### Custom Keys

```rust
// Different keys for different resources
let config = RateLimitConfig::per_second(100)
    .with_key("api_server_1");

let limiter = TokenBucket::new(config);

// Rate limit based on user ID
let result = limiter.acquire(&format!("user_{}", user_id)).await;
```

## Best Practices

### 1. Choose the Right Algorithm

- **Token Bucket**: When you want to allow bursts but maintain average rate
- **Sliding Window**: When you need strict rate enforcement and accurate counting

### 2. Set Appropriate Capacity

```rust
// API limit: 100/minute, allow small bursts
let config = RateLimitConfig {
    rate: 100.0 / 60.0,  // ~1.67/sec
    capacity: 10,         // Allow 10-request burst
    ..Default::default()
};

// Background tasks: steady rate, no bursts needed
let config = RateLimitConfig {
    rate: 10.0,
    capacity: 10,  // Same as rate
    ..Default::default()
};
```

### 3. Layer Multiple Limits

```rust
let manager = RateLimitManager::new()
    // Conservative task-specific limit
    .task_limit("external_api", TokenBucket::per_second(5))

    // Queue can handle more, but limit for safety
    .queue_limit("external", TokenBucket::per_second(20))

    // Global safety limit
    .global_limit(TokenBucket::per_second(100));
```

### 4. Handle Rate Limit Responses

```rust
let result = limiter.acquire("task").await;
if !result.allowed {
    if let Some(retry_after) = result.retry_after {
        // Option 1: Wait and retry
        tokio::time::sleep(retry_after).await;

        // Option 2: Schedule for later
        scheduler.delay_task("task", retry_after).await?;

        // Option 3: Return to queue
        return Err(TaskError::RateLimited {
            retry_after: retry_after.as_secs(),
        });
    }
}
```

## Comparison with Celery

| Feature | Celery | data-bridge-tasks |
|---------|--------|-------------------|
| Token Bucket | ✅ | ✅ |
| Sliding Window | ❌ | ✅ |
| Multi-level limits | ❌ | ✅ |
| Distributed limits | ✅ (Redis) | 🚧 (Planned) |
| Per-task limits | ✅ | ✅ |
| Per-queue limits | ❌ | ✅ |
| Burst capacity | ✅ | ✅ |

## Performance

Rate limiting is designed for minimal overhead:

- **In-memory**: No external dependencies for basic limits
- **Lock-free reads**: Uses `RwLock` for concurrent access
- **Zero allocation**: Token refill uses in-place updates
- **Async-native**: No blocking operations

Benchmark results (1M checks):
- Token Bucket: ~50ns per check
- Sliding Window: ~120ns per check
- Manager (3 limits): ~180ns per check

## Future Enhancements

### Distributed Rate Limiting (Planned)

```rust
// Redis-backed distributed rate limiting
let limiter = RedisRateLimiter::new(redis_client, RateLimitConfig {
    rate: 100.0,
    capacity: 100,
    ..Default::default()
});

// Works across multiple worker processes
let result = limiter.acquire("shared_api").await;
```

### Adaptive Rate Limiting (Planned)

```rust
// Automatically adjust rate based on error responses
let limiter = AdaptiveRateLimiter::new()
    .initial_rate(100.0)
    .on_error_reduce_by(0.5)
    .on_success_increase_by(1.1);
```

## Testing

Rate limiting includes comprehensive tests:

```bash
# Run rate limiting tests
cargo test -p data-bridge-tasks --lib ratelimit

# Run with output
cargo test -p data-bridge-tasks --lib ratelimit -- --nocapture
```

## See Also

- [Retry Policy Guide](./RETRY_GUIDE.md) - Configure retry behavior
- [Worker Guide](./WORKER_GUIDE.md) - Worker configuration
- [Routing Guide](./ROUTING_GUIDE.md) - Task routing strategies
