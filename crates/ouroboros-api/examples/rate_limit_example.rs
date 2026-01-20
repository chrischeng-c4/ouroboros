//! Rate Limiting Example
//!
//! This example demonstrates rate limiting in ouroboros-api,
//! including token bucket and sliding window algorithms.
//!
//! Run with:
//! ```bash
//! cargo run --example rate_limit_example -p ouroboros-api
//! ```

use ouroboros_api::rate_limit::{
    RateLimitConfig, RateLimitAlgorithm, RateLimiter, RateLimitResult,
};
use std::time::Duration;

// ============================================================================
// Rate Limit Configuration
// ============================================================================

fn demonstrate_config() {
    println!("1. Rate Limit Configuration");
    println!("---------------------------");

    // Using convenience constructors
    let per_second = RateLimitConfig::per_second(10);
    let per_minute = RateLimitConfig::per_minute(100);
    let per_hour = RateLimitConfig::per_hour(1000);

    println!("  Per second: {} requests / {:?}", per_second.max_requests, per_second.window);
    println!("  Per minute: {} requests / {:?}", per_minute.max_requests, per_minute.window);
    println!("  Per hour: {} requests / {:?}", per_hour.max_requests, per_hour.window);
    println!();

    // Custom configuration
    let custom = RateLimitConfig::new(50, Duration::from_secs(30))
        .algorithm(RateLimitAlgorithm::TokenBucket)
        .include_headers(true);

    println!("  Custom config:");
    println!("    Max requests: {}", custom.max_requests);
    println!("    Window: {:?}", custom.window);
    println!("    Algorithm: {:?}", custom.algorithm);
    println!("    Include headers: {}", custom.include_headers);
    println!();

    // Parsing from string
    let parsed_configs = [
        "10/second",
        "100/minute",
        "1000/hour",
        "5000/day",
    ];

    println!("  Parsed configs:");
    for s in parsed_configs {
        if let Some(config) = RateLimitConfig::parse(s) {
            println!("    '{}' -> {} requests / {:?}", s, config.max_requests, config.window);
        }
    }
    println!();
}

// ============================================================================
// Rate Limit Algorithms
// ============================================================================

fn demonstrate_algorithms() {
    println!("2. Rate Limit Algorithms");
    println!("------------------------");

    println!("  Fixed Window:");
    println!("    - Simplest algorithm");
    println!("    - Counts requests in fixed time windows");
    println!("    - Can allow burst at window boundaries");
    println!();

    println!("  Sliding Window (default):");
    println!("    - Smooths out traffic over time");
    println!("    - Prevents burst at window boundaries");
    println!("    - More memory intensive");
    println!();

    println!("  Token Bucket:");
    println!("    - Allows controlled bursting");
    println!("    - Tokens refill at steady rate");
    println!("    - Good for bursty traffic patterns");
    println!();
}

// ============================================================================
// Rate Limiter Usage
// ============================================================================

fn demonstrate_rate_limiter() {
    println!("3. Rate Limiter Usage");
    println!("---------------------");

    // Create a rate limiter with 5 requests per second
    let config = RateLimitConfig::per_second(5);
    let limiter = RateLimiter::new(config);

    println!("  Config: 5 requests per second");
    println!("  Simulating 10 requests from same IP...");
    println!();

    let client_ip = "192.168.1.100";

    for i in 1..=10 {
        let result = limiter.check(client_ip);
        let status = if result.allowed { "ALLOWED" } else { "DENIED" };
        println!(
            "  Request {}: {} | Remaining: {} | Reset: {:?}",
            i, status, result.remaining,
            result.reset_after
        );

        if !result.allowed {
            if let Some(retry) = result.retry_after {
                println!("             Retry after: {:?}", retry);
            }
        }
    }
    println!();
}

// ============================================================================
// Rate Limit Headers
// ============================================================================

fn demonstrate_headers() {
    println!("4. Rate Limit Headers");
    println!("---------------------");

    let result = RateLimitResult::allowed(95, 100, Duration::from_secs(45));

    println!("  Standard headers to include in response:");
    for (name, value) in result.headers() {
        println!("    {}: {}", name, value);
    }
    println!();

    let denied = RateLimitResult::denied(100, Duration::from_secs(30));
    println!("  Headers when rate limited:");
    for (name, value) in denied.headers() {
        println!("    {}: {}", name, value);
    }
    println!();
}

// ============================================================================
// Multiple Rate Limiters
// ============================================================================

fn demonstrate_tiered_limits() {
    println!("5. Tiered Rate Limits");
    println!("---------------------");

    // Different limits for different user types
    let free_tier = RateLimitConfig::per_minute(10);
    let pro_tier = RateLimitConfig::per_minute(100);
    let enterprise_tier = RateLimitConfig::per_minute(1000);

    println!("  Free tier: {} requests/minute", free_tier.max_requests);
    println!("  Pro tier: {} requests/minute", pro_tier.max_requests);
    println!("  Enterprise: {} requests/minute", enterprise_tier.max_requests);
    println!();

    // Multiple limits per endpoint
    println!("  Combined limits example:");
    println!("    - Global: 1000 requests/minute (shared across all users)");
    println!("    - Per-user: 100 requests/minute");
    println!("    - Per-endpoint: 50 requests/minute for expensive operations");
    println!();
}

// ============================================================================
// Rate Limit Middleware Pattern
// ============================================================================

fn demonstrate_middleware_pattern() {
    println!("6. Rate Limit Middleware Pattern");
    println!("--------------------------------");

    println!("  Example middleware implementation:");
    println!(r#"
    async fn rate_limit_middleware(req: &mut Request) -> ApiResult<()> {{
        let client_ip = req.client_ip().unwrap_or("unknown");
        let result = rate_limiter.check(client_ip);

        // Add rate limit headers
        for (name, value) in result.headers() {{
            req.set_header(name, value);
        }}

        if !result.allowed {{
            return Err(ApiError::TooManyRequests {{
                retry_after: result.retry_after,
            }});
        }}

        Ok(())
    }}
    "#);
    println!();
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("Rate Limiting Example");
    println!("=====================\n");

    demonstrate_config();
    demonstrate_algorithms();
    demonstrate_rate_limiter();
    demonstrate_headers();
    demonstrate_tiered_limits();
    demonstrate_middleware_pattern();

    println!("Best Practices:");
    println!("  - Choose appropriate limits for your use case");
    println!("  - Use sliding window for smoother rate limiting");
    println!("  - Implement tiered limits for different user types");
    println!("  - Always return rate limit headers for transparency");
    println!("  - Consider distributed rate limiting for multi-server setups");
}
