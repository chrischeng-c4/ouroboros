//! Lifecycle Hooks Example
//!
//! This example demonstrates startup and shutdown hooks in ouroboros-api
//! for managing application lifecycle events like database connections,
//! cache initialization, and graceful shutdown.
//!
//! Run with:
//! ```bash
//! cargo run --example lifecycle_example -p ouroboros-api
//! ```

use ouroboros_api::lifecycle::{LifecycleManager, SharedLifecycleManager};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Basic Lifecycle Manager
// ============================================================================

async fn demonstrate_basic_lifecycle() {
    println!("1. Basic Lifecycle Manager");
    println!("--------------------------");

    let mut lifecycle = LifecycleManager::new();

    // Register startup hooks
    lifecycle.on_startup(|| async {
        println!("  [Startup] Initializing configuration...");
        Ok(())
    });

    lifecycle.on_startup(|| async {
        println!("  [Startup] Connecting to database...");
        // Simulate database connection
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        println!("  [Startup] Database connected!");
        Ok(())
    });

    lifecycle.on_startup(|| async {
        println!("  [Startup] Warming up cache...");
        Ok(())
    });

    // Register shutdown hooks
    lifecycle.on_shutdown(|| async {
        println!("  [Shutdown] Flushing cache...");
    });

    lifecycle.on_shutdown(|| async {
        println!("  [Shutdown] Closing database connection...");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        println!("  [Shutdown] Database disconnected!");
    });

    lifecycle.on_shutdown(|| async {
        println!("  [Shutdown] Cleanup complete!");
    });

    println!("  Startup hooks: {}", lifecycle.startup_hook_count());
    println!("  Shutdown hooks: {}", lifecycle.shutdown_hook_count());
    println!();

    // Run startup
    println!("  Running startup...");
    match lifecycle.startup().await {
        Ok(()) => println!("  Startup successful!"),
        Err(e) => println!("  Startup failed: {:?}", e),
    }
    println!();

    // Check if started
    println!("  Is started: {}", lifecycle.is_started());
    println!();

    // Run shutdown
    println!("  Running shutdown...");
    lifecycle.shutdown().await;
    println!("  Shutdown complete!");
    println!();
}

// ============================================================================
// Named Hooks
// ============================================================================

async fn demonstrate_named_hooks() {
    println!("2. Named Hooks");
    println!("--------------");

    let mut lifecycle = LifecycleManager::new();

    // Named startup hooks for better error messages
    lifecycle.on_startup_named("config", || async {
        println!("  [config] Loading configuration...");
        Ok(())
    });

    lifecycle.on_startup_named("database", || async {
        println!("  [database] Connecting...");
        Ok(())
    });

    lifecycle.on_startup_named("cache", || async {
        println!("  [cache] Initializing...");
        Ok(())
    });

    // Named shutdown hooks
    lifecycle.on_shutdown_named("cache", || async {
        println!("  [cache] Flushing...");
    });

    lifecycle.on_shutdown_named("database", || async {
        println!("  [database] Disconnecting...");
    });

    // Run lifecycle
    lifecycle.startup().await.unwrap();
    lifecycle.shutdown().await;
    println!();
}

// ============================================================================
// Startup Error Handling
// ============================================================================

async fn demonstrate_startup_error() {
    println!("3. Startup Error Handling");
    println!("-------------------------");

    let mut lifecycle = LifecycleManager::new();

    lifecycle.on_startup_named("first", || async {
        println!("  [first] Succeeds");
        Ok(())
    });

    lifecycle.on_startup_named("failing_hook", || async {
        println!("  [failing_hook] About to fail...");
        Err("Database connection timeout".to_string())
    });

    lifecycle.on_startup_named("never_runs", || async {
        println!("  [never_runs] This should not execute");
        Ok(())
    });

    println!("  Running startup with failing hook...");
    match lifecycle.startup().await {
        Ok(()) => println!("  Startup succeeded"),
        Err(e) => {
            println!("  Startup failed!");
            println!("    Hook: {:?}", e.hook_name);
            println!("    Error: {}", e.message);
        }
    }
    println!();
}

// ============================================================================
// Shared Lifecycle Manager
// ============================================================================

async fn demonstrate_shared_lifecycle() {
    println!("4. Shared Lifecycle Manager");
    println!("---------------------------");

    // SharedLifecycleManager can be cloned and shared across threads
    let lifecycle = SharedLifecycleManager::new();

    // Counter to verify hooks run
    let counter = Arc::new(AtomicU32::new(0));

    // Register from multiple "components"
    let c1 = counter.clone();
    lifecycle.on_startup(move || {
        let c1 = c1.clone();
        async move {
            c1.fetch_add(1, Ordering::SeqCst);
            println!("  [Component 1] Initialized");
            Ok(())
        }
    }).await;

    let c2 = counter.clone();
    lifecycle.on_startup(move || {
        let c2 = c2.clone();
        async move {
            c2.fetch_add(1, Ordering::SeqCst);
            println!("  [Component 2] Initialized");
            Ok(())
        }
    }).await;

    // Run startup
    println!("  Running startup on shared manager...");
    lifecycle.startup().await.unwrap();
    println!("  Components initialized: {}", counter.load(Ordering::SeqCst));
    println!("  Is started: {}", lifecycle.is_started().await);
    println!();
}

// ============================================================================
// Shutdown Order
// ============================================================================

async fn demonstrate_shutdown_order() {
    println!("5. Shutdown Order (Reverse of Registration)");
    println!("-------------------------------------------");

    let mut lifecycle = LifecycleManager::new();
    let order = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Register in order: 1, 2, 3
    for i in 1..=3 {
        let o = order.clone();
        lifecycle.on_shutdown_named(&format!("hook_{}", i), move || {
            let o = o.clone();
            async move {
                o.lock().await.push(i);
                println!("  [hook_{}] Executed", i);
            }
        });
    }

    lifecycle.startup().await.unwrap();
    println!("  Running shutdown...");
    lifecycle.shutdown().await;

    let execution_order = order.lock().await;
    println!("  Execution order: {:?} (reverse of registration)", *execution_order);
    println!();
}

// ============================================================================
// Real-World Pattern
// ============================================================================

async fn demonstrate_real_world_pattern() {
    println!("6. Real-World Pattern");
    println!("---------------------");

    println!("  Typical application lifecycle:");
    println!();
    println!("  Startup (in order):");
    println!("    1. Load configuration");
    println!("    2. Initialize logging");
    println!("    3. Connect to database");
    println!("    4. Connect to cache (Redis)");
    println!("    5. Start background workers");
    println!("    6. Ready for traffic");
    println!();
    println!("  Shutdown (reverse order):");
    println!("    1. Stop accepting new requests");
    println!("    2. Wait for in-flight requests (graceful)");
    println!("    3. Stop background workers");
    println!("    4. Flush cache");
    println!("    5. Close database connection");
    println!("    6. Flush logs");
    println!();

    println!("  Code pattern:");
    println!(r#"
    let mut lifecycle = LifecycleManager::new();

    // Database (early startup, late shutdown)
    lifecycle.on_startup_named("database", || async {{
        DB_POOL.connect().await
    }});
    lifecycle.on_shutdown_named("database", || async {{
        DB_POOL.close().await;
    }});

    // Cache (after DB, before workers)
    lifecycle.on_startup_named("cache", || async {{
        CACHE.connect().await
    }});
    lifecycle.on_shutdown_named("cache", || async {{
        CACHE.flush().await;
        CACHE.close().await;
    }});

    // Background workers (last startup, first shutdown)
    lifecycle.on_startup_named("workers", || async {{
        WORKERS.start().await
    }});
    lifecycle.on_shutdown_named("workers", || async {{
        WORKERS.stop_gracefully().await;
    }});
    "#);
    println!();
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    println!("Lifecycle Hooks Example");
    println!("=======================\n");

    demonstrate_basic_lifecycle().await;
    demonstrate_named_hooks().await;
    demonstrate_startup_error().await;
    demonstrate_shared_lifecycle().await;
    demonstrate_shutdown_order().await;
    demonstrate_real_world_pattern().await;

    println!("Best Practices:");
    println!("  - Use named hooks for better error messages");
    println!("  - Register critical hooks (DB, cache) early");
    println!("  - Shutdown is automatic reverse order");
    println!("  - Use SharedLifecycleManager for multi-component apps");
    println!("  - Handle startup errors gracefully");
}
