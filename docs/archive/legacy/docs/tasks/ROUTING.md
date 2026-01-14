# Task Routing

Task routing allows you to direct tasks to specific queues based on task names, patterns, or custom logic. This is similar to Celery's `CELERY_ROUTES` configuration.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Routing Strategies](#routing-strategies)
  - [Exact Match](#exact-match)
  - [Glob Patterns](#glob-patterns)
  - [Regex Patterns](#regex-patterns)
  - [Custom Functions](#custom-functions)
- [Configuration](#configuration)
- [Priority and Fallback](#priority-and-fallback)
- [JSON Configuration](#json-configuration)
- [Environment Variables](#environment-variables)
- [Examples](#examples)

## Overview

The routing system provides flexible task-to-queue mapping through:

- **Exact match routing**: Direct task names to specific queues
- **Glob pattern routing**: Use wildcards to match task names (e.g., `email.*`)
- **Regex pattern routing**: Advanced pattern matching with regular expressions
- **Custom function routing**: Programmatic routing based on task name and arguments

## Quick Start

```rust
use data_bridge_tasks::routing::RouterConfig;
use serde_json::json;

// Create a router
let router = RouterConfig::new()
    .route("send_email", "email")           // Exact match
    .route_glob("tasks.math.*", "math")     // Glob pattern
    .default_queue("default")               // Fallback queue
    .build();

// Route a task
let queue = router.route("send_email", &json!({}));
println!("Task routed to: {}", queue);  // "email"
```

## Routing Strategies

### Exact Match

Route specific task names to queues:

```rust
let router = RouterConfig::new()
    .route("send_email", "email")
    .route("process_payment", "payments")
    .route("generate_report", "reports")
    .build();

assert_eq!(router.route("send_email", &json!({})), "email");
assert_eq!(router.route("process_payment", &json!({})), "payments");
```

### Glob Patterns

Use wildcards for flexible matching:

- `*` matches any sequence of characters
- `?` matches a single character

```rust
let router = RouterConfig::new()
    .route_glob("email.*", "email")              // email.send, email.receive
    .route_glob("tasks.math.*", "math")          // tasks.math.add, tasks.math.multiply
    .route_glob("tasks.*.urgent", "high-priority") // tasks.email.urgent, tasks.payment.urgent
    .build();

assert_eq!(router.route("email.send", &json!({})), "email");
assert_eq!(router.route("tasks.math.add", &json!({})), "math");
assert_eq!(router.route("tasks.email.urgent", &json!({})), "high-priority");
```

### Regex Patterns

Advanced pattern matching with regular expressions:

```rust
let router = RouterConfig::new()
    .route_regex(r"^user_\d+$", "users")           // user_123, user_456
    .route_regex(r"^report_.*_monthly$", "reports") // report_sales_monthly
    .route_regex(r"^(critical|urgent)_", "high-priority")
    .build();

assert_eq!(router.route("user_123", &json!({})), "users");
assert_eq!(router.route("report_sales_monthly", &json!({})), "reports");
assert_eq!(router.route("critical_alert", &json!({})), "high-priority");
```

### Custom Functions

Route based on task name and arguments:

```rust
let router = RouterConfig::new()
    .route_fn("priority_router", |task_name, args| {
        // Route based on priority argument
        if let Some(priority) = args.get("priority").and_then(|v| v.as_str()) {
            match priority {
                "high" => return Some("high-priority".to_string()),
                "low" => return Some("low-priority".to_string()),
                _ => {}
            }
        }

        // Route based on task name
        if task_name.starts_with("urgent_") {
            return Some("urgent".to_string());
        }

        None  // Fall through to other routes
    })
    .build();

// Route based on args
let queue = router.route("process_order", &json!({"priority": "high"}));
assert_eq!(queue, "high-priority");

// Route based on task name
let queue = router.route("urgent_notification", &json!({}));
assert_eq!(queue, "urgent");
```

## Configuration

### Builder Pattern

```rust
use data_bridge_tasks::routing::RouterConfig;

let router = RouterConfig::new()
    // Add routes
    .route("send_email", "email")
    .route_glob("tasks.math.*", "math")
    .route_regex(r"^user_\d+$", "users")

    // Set default queue
    .default_queue("worker")

    // Build the router
    .build();
```

### Programmatic Configuration

```rust
use data_bridge_tasks::routing::{Router, Route, PatternType};

let routes = vec![
    Route {
        pattern: "email.*".to_string(),
        queue: "email".to_string(),
        pattern_type: PatternType::Glob,
    },
    Route {
        pattern: "process_payment".to_string(),
        queue: "payments".to_string(),
        pattern_type: PatternType::Exact,
    },
];

let router = Router::from_routes(routes, "default".to_string());
```

## Priority and Fallback

Routes are checked in this order:

1. **Custom functions** (added via `route_fn`)
2. **Pattern routes** (exact, glob, regex - in order added)
3. **Default queue** (fallback)

```rust
let router = RouterConfig::new()
    // Custom routes checked first
    .route_fn("vip", |_, args| {
        if args.get("user_type")?.as_str()? == "vip" {
            Some("vip".to_string())
        } else {
            None
        }
    })
    // Pattern routes checked second
    .route("send_email", "email")
    // Default queue used last
    .default_queue("worker")
    .build();

// Custom route wins
assert_eq!(
    router.route("send_email", &json!({"user_type": "vip"})),
    "vip"
);

// Pattern route used when custom returns None
assert_eq!(
    router.route("send_email", &json!({})),
    "email"
);

// Default used when no routes match
assert_eq!(
    router.route("unknown_task", &json!({})),
    "worker"
);
```

## JSON Configuration

Routes can be loaded from JSON:

```rust
use data_bridge_tasks::routing::RoutesConfig;

let json = r#"{
    "routes": [
        {
            "pattern": "email.*",
            "queue": "email",
            "pattern_type": "glob"
        },
        {
            "pattern": "^user_\\d+$",
            "queue": "users",
            "pattern_type": "regex"
        },
        {
            "pattern": "process_payment",
            "queue": "payments",
            "pattern_type": "exact"
        }
    ],
    "default_queue": "worker"
}"#;

let config: RoutesConfig = serde_json::from_str(json)?;
let router = config.into_router();
```

### Pattern Types

In JSON configuration, `pattern_type` can be:
- `"exact"` - Exact string match (default)
- `"glob"` - Glob pattern with wildcards
- `"regex"` - Regular expression

## Environment Variables

Load routes from the `TASK_ROUTES` environment variable:

```rust
use data_bridge_tasks::routing::RoutesConfig;

// Set environment variable
std::env::set_var("TASK_ROUTES", r#"{
    "routes": [
        {"pattern": "email.*", "queue": "email", "pattern_type": "glob"}
    ],
    "default_queue": "worker"
}"#);

// Load from environment
let config = RoutesConfig::from_env()?;
let router = config.into_router();
```

## Examples

### Example 1: Organize by Service

```rust
let router = RouterConfig::new()
    .route_glob("email.*", "email-service")
    .route_glob("payment.*", "payment-service")
    .route_glob("notification.*", "notification-service")
    .default_queue("general")
    .build();
```

### Example 2: Priority-Based Routing

```rust
let router = RouterConfig::new()
    .route_fn("priority", |task_name, args| {
        if task_name.starts_with("critical_") {
            return Some("critical".to_string());
        }

        if let Some(priority) = args.get("priority").and_then(|v| v.as_u64()) {
            if priority >= 8 {
                return Some("high".to_string());
            } else if priority <= 3 {
                return Some("low".to_string());
            }
        }

        None  // Use normal queue
    })
    .default_queue("normal")
    .build();
```

### Example 3: User-Type Based Routing

```rust
let router = RouterConfig::new()
    .route_fn("user_router", |_, args| {
        match args.get("user_type").and_then(|v| v.as_str())? {
            "vip" => Some("vip-queue".to_string()),
            "premium" => Some("premium-queue".to_string()),
            "enterprise" => Some("enterprise-queue".to_string()),
            _ => None,
        }
    })
    .default_queue("free-tier")
    .build();
```

### Example 4: Time-Sensitive Routing

```rust
let router = RouterConfig::new()
    .route_fn("time_router", |task_name, args| {
        // Route time-sensitive tasks to fast queue
        if task_name.contains("realtime") || task_name.contains("live") {
            return Some("fast".to_string());
        }

        // Route batch/scheduled tasks to slow queue
        if let Some(scheduled) = args.get("scheduled").and_then(|v| v.as_bool()) {
            if scheduled {
                return Some("batch".to_string());
            }
        }

        None
    })
    .route_glob("*.batch", "batch")
    .route_glob("*.urgent", "fast")
    .default_queue("normal")
    .build();
```

### Example 5: Geographic Routing

```rust
let router = RouterConfig::new()
    .route_fn("geo_router", |_, args| {
        let region = args.get("region").and_then(|v| v.as_str())?;
        Some(format!("{}-region", region))
    })
    .default_queue("global")
    .build();

// Route to regional queues
let queue = router.route("process_order", &json!({"region": "us-east"}));
assert_eq!(queue, "us-east-region");
```

## Thread Safety

The `Router` type is thread-safe and can be shared across multiple threads:

```rust
use std::sync::Arc;

let router = Arc::new(RouterConfig::new()
    .route("send_email", "email")
    .build());

// Clone the Arc to share across threads
let router_clone = Arc::clone(&router);
std::thread::spawn(move || {
    let queue = router_clone.route("send_email", &json!({}));
    println!("Queue: {}", queue);
});
```

## Performance

- **Exact matches**: O(n) where n is the number of routes (typically very small)
- **Glob patterns**: Compiled to regex and cached
- **Regex patterns**: Compiled on first use and cached in thread-safe RwLock
- **Custom functions**: Performance depends on your implementation

For optimal performance:
- Put most common routes first
- Use exact matches when possible
- Avoid complex regex patterns if simple globs suffice
- Keep custom routing logic lightweight

## Best Practices

1. **Order matters**: List more specific routes before general ones
2. **Use appropriate patterns**: Don't use regex when glob is sufficient
3. **Cache the router**: Create once, use many times
4. **Test your routes**: Verify routing logic with unit tests
5. **Document custom logic**: Comment complex routing functions
6. **Monitor queue distribution**: Ensure balanced workload

## Comparison with Celery

| Feature | Celery | data-bridge-tasks |
|---------|--------|-------------------|
| Exact routes | ✓ | ✓ |
| Glob patterns | ✓ | ✓ |
| Regex patterns | ✗ | ✓ |
| Custom functions | ✓ | ✓ |
| JSON config | ✗ | ✓ |
| Env variables | ✗ | ✓ |
| Thread-safe | N/A (Python) | ✓ |
| Regex caching | N/A | ✓ |

## See Also

- [Worker Configuration](./WORKER.md)
- [Task Signatures](./SIGNATURES.md)
- [Broker Configuration](./BROKER.md)
