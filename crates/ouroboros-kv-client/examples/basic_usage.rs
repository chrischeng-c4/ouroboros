//! Basic usage example for KV client
//!
//! Run this example with:
//! ```
//! # Terminal 1: Start the server
//! cargo run -p ouroboros-kv-server
//!
//! # Terminal 2: Run the example
//! cargo run -p ouroboros-kv-client --example basic_usage
//! ```

use ouroboros_kv_client::{KvClient, KvValue};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== KV Client Example ===\n");

    // Connect to server
    println!("Connecting to 127.0.0.1:6380...");
    let mut client = KvClient::connect("127.0.0.1:6380").await?;
    println!("Connected!\n");

    // Ping
    println!("1. Pinging server...");
    let pong = client.ping().await?;
    println!("   Response: {}\n", pong);

    // Set string value
    println!("2. Setting string value...");
    client.set("user:1:name", KvValue::String("Alice".to_string()), None).await?;
    println!("   Set user:1:name = 'Alice'\n");

    // Get string value
    println!("3. Getting string value...");
    if let Some(value) = client.get("user:1:name").await? {
        println!("   Got user:1:name = {:?}\n", value);
    }

    // Set with TTL
    println!("4. Setting value with 5 second TTL...");
    client.set(
        "session:temp",
        KvValue::String("expires soon".to_string()),
        Some(Duration::from_secs(5))
    ).await?;
    println!("   Set session:temp (expires in 5s)\n");

    // Increment counter
    println!("5. Working with counters...");
    client.set("counter", KvValue::Int(10), None).await?;
    println!("   Initial counter = 10");

    let new_value = client.incr("counter", 5).await?;
    println!("   After INCR 5: counter = {}", new_value);

    let new_value = client.decr("counter", 3).await?;
    println!("   After DECR 3: counter = {}\n", new_value);

    // Exists check
    println!("6. Checking key existence...");
    let exists = client.exists("user:1:name").await?;
    println!("   user:1:name exists: {}", exists);

    let not_exists = client.exists("nonexistent").await?;
    println!("   nonexistent exists: {}\n", not_exists);

    // Complex data types
    println!("7. Storing complex types...");
    let list = KvValue::List(vec![
        KvValue::Int(1),
        KvValue::String("two".to_string()),
        KvValue::Float(3.14),
    ]);
    client.set("mylist", list, None).await?;

    if let Some(KvValue::List(items)) = client.get("mylist").await? {
        println!("   List items: {:?}\n", items);
    }

    // Delete
    println!("8. Deleting a key...");
    let deleted = client.delete("user:1:name").await?;
    println!("   Deleted user:1:name: {}\n", deleted);

    // Server info
    println!("9. Getting server info...");
    let info = client.info().await?;
    println!("   {}\n", info);

    println!("=== Example Complete ===");
    Ok(())
}
