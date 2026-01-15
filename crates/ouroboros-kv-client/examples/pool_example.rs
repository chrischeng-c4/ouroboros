//! Example: Using KvPool for connection pooling
//!
//! Run the server first:
//!   cargo run -p ouroboros-kv-server
//!
//! Then run this example:
//!   cargo run -p ouroboros-kv-client --example pool_example

use ouroboros_kv_client::{KvPool, PoolConfig, KvValue, ClientError};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== KvPool Example ===\n");

    // 1. Create a connection pool
    println!("Creating pool with min_size=3, max_size=10...");
    let pool = KvPool::connect(
        PoolConfig::new("127.0.0.1:6380")
            .min_size(3)
            .max_size(10)
            .idle_timeout(Duration::from_secs(300))
            .acquire_timeout(Duration::from_secs(5))
    ).await?;

    // Show initial pool stats
    let stats = pool.stats().await;
    println!("Initial pool stats - Idle: {}, Active: {}, Max: {}\n",
        stats.idle, stats.active, stats.max_size);

    // 2. Single connection usage
    println!("--- Single Connection Example ---");
    {
        let mut conn = pool.acquire().await?;
        let stats = pool.stats().await;
        println!("After acquire - Idle: {}, Active: {}", stats.idle, stats.active);

        conn.client().set("greeting", KvValue::String("Hello, Pool!".to_string()), None).await?;
        let value = conn.client().get("greeting").await?;
        println!("Retrieved: {:?}", value);
    } // Connection automatically returned here
    tokio::time::sleep(Duration::from_millis(50)).await;  // Give async return time

    let stats = pool.stats().await;
    println!("After return - Idle: {}, Active: {}\n", stats.idle, stats.active);

    // 3. Concurrent usage
    println!("--- Concurrent Usage Example ---");
    let pool = Arc::new(pool);
    let mut handles = vec![];

    for i in 0..20 {
        let pool = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            let mut conn = pool.acquire().await?;

            // Set a counter value
            conn.client().set(
                &format!("counter_{}", i),
                KvValue::Int(i as i64),
                Some(Duration::from_secs(60))
            ).await?;

            // Increment it
            let new_val = conn.client().incr(&format!("counter_{}", i), 10).await?;
            println!("Task {}: counter_{} = {}", i, i, new_val);

            Ok::<_, ClientError>(())
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;  // Give connections time to return

    // Show final stats
    let stats = pool.stats().await;
    println!("\nFinal pool stats - Idle: {}, Active: {}, Max: {}",
        stats.idle, stats.active, stats.max_size);

    // 4. Namespace example
    println!("\n--- Namespace Example ---");
    let ns_pool = KvPool::connect(
        PoolConfig::new("127.0.0.1:6380/myapp")
            .min_size(2)
            .max_size(5)
    ).await?;

    println!("Pool namespace: {:?}", ns_pool.namespace());

    let mut conn = ns_pool.acquire().await?;
    conn.client().set("config", KvValue::String("production".to_string()), None).await?;
    let value = conn.client().get("config").await?;
    println!("Namespaced value: {:?}", value);

    // Clean up
    conn.client().delete("config").await?;

    println!("\n=== Example Complete ===");
    Ok(())
}
