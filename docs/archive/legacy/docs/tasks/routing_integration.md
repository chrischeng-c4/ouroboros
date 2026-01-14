# Router Integration with TaskRegistry

The TaskRegistry now supports automatic task routing based on task names and arguments. This allows you to direct tasks to specific queues automatically when they are published.

## Overview

When a router is attached to the TaskRegistry, the `route_task()` method will automatically determine which queue a task should be sent to based on:

1. **Exact matches** - Direct task name to queue mapping
2. **Glob patterns** - Pattern-based routing (e.g., `email.*` routes all email tasks)
3. **Regex patterns** - Complex pattern matching
4. **Custom functions** - Dynamic routing based on task name and arguments

## Usage

### Basic Setup

```rust
use data_bridge_tasks::{TaskRegistry, RouterConfig};

// Create a router with routing rules
let router = RouterConfig::new()
    .route("math.add", "math")           // Exact match: math.add -> math queue
    .route_glob("email.*", "email")      // Glob: email.* -> email queue
    .route("urgent_task", "high-priority") // Exact match
    .default_queue("default")            // Fallback queue
    .build();

// Create registry with router
let registry = TaskRegistry::new().with_router(router);

// Now routing is automatic
let queue = registry.route_task("math.add", &serde_json::json!({}));
// Returns: "math"

let queue = registry.route_task("email.send", &serde_json::json!({}));
// Returns: "email"

let queue = registry.route_task("unknown_task", &serde_json::json!({}));
// Returns: "default"
```

### Setting Router After Creation

```rust
let mut registry = TaskRegistry::new();

// Register some tasks first
registry.register(MathTask);
registry.register(EmailTask);

// Add router later
let router = RouterConfig::new()
    .route("math.add", "math")
    .route("email.send", "email")
    .build();

registry.set_router(router);
```

### Advanced Routing with Custom Logic

```rust
use data_bridge_tasks::{TaskRegistry, RouterConfig};

let router = RouterConfig::new()
    // Static routes
    .route("process_payment", "payments")

    // Glob patterns
    .route_glob("tasks.analytics.*", "analytics")

    // Custom routing based on arguments
    .route_fn("priority_router", |task_name, args| {
        // Route based on priority in args
        if let Some(priority) = args.get("priority").and_then(|v| v.as_str()) {
            match priority {
                "high" => return Some("high-priority".to_string()),
                "low" => return Some("low-priority".to_string()),
                _ => {}
            }
        }

        // Route based on task name patterns
        if task_name.starts_with("urgent_") {
            return Some("urgent".to_string());
        }

        None // Let other rules handle it
    })
    .default_queue("default")
    .build();

let registry = TaskRegistry::new().with_router(router);

// Route based on args
let queue = registry.route_task(
    "process_data",
    &serde_json::json!({"priority": "high"})
);
// Returns: "high-priority"

// Route based on task name
let queue = registry.route_task("urgent_backup", &serde_json::json!({}));
// Returns: "urgent"
```

## Integration with Worker

When a worker publishes a task, it can use the registry's routing:

```rust
use data_bridge_tasks::{TaskRegistry, RouterConfig, TaskSignature};

let router = RouterConfig::new()
    .route("send_email", "email")
    .build();

let registry = TaskRegistry::new().with_router(router);

// When creating a task signature, use registry routing
let task_name = "send_email";
let args = serde_json::json!({"to": "user@example.com"});
let queue = registry.route_task(task_name, &args);

// Create signature with the routed queue
let signature = TaskSignature::new(task_name, vec![args])
    .set_queue(&queue);

// Now publish to the correct queue
```

## Routing Priority

The router evaluates rules in this order:

1. **Custom functions** - Checked first (most flexible)
2. **Pattern routes** - Checked in order added (exact, glob, regex)
3. **Default queue** - Used if no routes match

## Benefits

1. **Centralized Configuration** - All routing logic in one place
2. **Automatic Queue Selection** - No manual queue specification needed
3. **Flexible Rules** - Combine static and dynamic routing
4. **Type Safety** - Routing happens at the Rust layer
5. **Performance** - Pattern matching is cached (regex)

## Example: Email Service

```rust
use data_bridge_tasks::{Task, TaskRegistry, RouterConfig};
use async_trait::async_trait;

// Define tasks
struct SendEmailTask;

#[async_trait]
impl Task for SendEmailTask {
    fn name(&self) -> &'static str {
        "email.send"
    }

    async fn execute(&self, _ctx: TaskContext, _args: serde_json::Value) -> TaskOutcome {
        // Send email logic
        TaskOutcome::Success(serde_json::json!({"sent": true}))
    }
}

// Setup registry with routing
let router = RouterConfig::new()
    .route_glob("email.*", "email-workers")
    .route_glob("sms.*", "sms-workers")
    .route_glob("push.*", "push-workers")
    .default_queue("default")
    .build();

let registry = TaskRegistry::new().with_router(router);

// Register tasks
registry.register(SendEmailTask);

// All email.* tasks automatically go to "email-workers" queue
assert_eq!(
    registry.route_task("email.send", &serde_json::json!({})),
    "email-workers"
);
```

## Testing

The integration includes comprehensive tests:

```rust
#[test]
fn test_registry_with_router() {
    use data_bridge_tasks::{TaskRegistry, RouterConfig};

    let router = RouterConfig::new()
        .route("math.add", "math")
        .route_glob("email.*", "email")
        .route("test_task", "testing")
        .build();

    let registry = TaskRegistry::new().with_router(router);

    // Verify router is set
    assert!(registry.router().is_some());

    // Test routing
    assert_eq!(registry.route_task("math.add", &serde_json::json!({})), "math");
    assert_eq!(registry.route_task("email.send", &serde_json::json!({})), "email");
    assert_eq!(registry.route_task("test_task", &serde_json::json!({})), "testing");
    assert_eq!(registry.route_task("unknown", &serde_json::json!({})), "default");
}
```

## See Also

- [Task Routing Documentation](./routing.md)
- [Task Registry API](./task_registry.md)
- [Worker Configuration](./worker.md)
