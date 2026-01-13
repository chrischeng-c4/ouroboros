//! Example demonstrating Server-Sent Events (SSE) usage with data-bridge-api
//!
//! This example shows how to create SSE streams and responses.
//!
//! Run with: cargo run --package data-bridge-api --example sse_example

use data_bridge_api::sse::{SseEvent, SseStream, SseResponse, keep_alive_stream, merge_streams};
use async_stream::stream;
use futures_util::StreamExt;
use std::time::Duration;
use std::pin::Pin;

#[tokio::main]
async fn main() {
    println!("=== Server-Sent Events (SSE) Examples ===\n");

    // Example 1: Basic SSE event
    println!("1. Basic SSE Event:");
    let event = SseEvent::new("Hello, World!");
    println!("{}", event.to_string());

    // Example 2: SSE event with all fields
    println!("2. SSE Event with all fields:");
    let event = SseEvent::new("Important update")
        .with_event("notification")
        .with_id("12345")
        .with_retry(3000);
    println!("{}", event.to_string());

    // Example 3: Multi-line data
    println!("3. Multi-line data:");
    let event = SseEvent::new("Line 1\nLine 2\nLine 3");
    println!("{}", event.to_string());

    // Example 4: Simple event stream
    println!("4. Simple event stream:");
    let event_stream = stream! {
        for i in 1..=3 {
            yield SseEvent::new(format!("Event {}", i))
                .with_id(i.to_string());
        }
    };

    let mut sse_stream = SseStream::new(event_stream);
    while let Some(event) = sse_stream.next().await {
        print!("{}", event.to_string());
    }

    // Example 5: Time-based event stream
    println!("\n5. Time-based event stream (3 seconds):");
    let timed_stream = stream! {
        for i in 1..=3 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            yield SseEvent::new(format!("Timed event {}", i))
                .with_event("timer");
        }
    };

    let mut sse_stream = SseStream::new(timed_stream);
    while let Some(event) = sse_stream.next().await {
        print!("  {}", event.to_string());
    }

    // Example 6: Keep-alive stream
    println!("\n6. Keep-alive stream (first 3 events):");
    let keep_alive = keep_alive_stream(Duration::from_millis(500));
    tokio::pin!(keep_alive);

    for i in 0..3 {
        if let Some(event) = keep_alive.next().await {
            println!("  Keep-alive event {}: event={:?}", i + 1, event.event);
        }
    }

    // Example 7: Merged streams
    println!("\n7. Merged streams:");
    let stream1: Pin<Box<dyn futures_util::Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
        yield SseEvent::new("From stream 1").with_event("stream1");
    });

    let stream2: Pin<Box<dyn futures_util::Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
        yield SseEvent::new("From stream 2").with_event("stream2");
    });

    let stream3: Pin<Box<dyn futures_util::Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
        yield SseEvent::new("From stream 3").with_event("stream3");
    });

    let merged = merge_streams(vec![stream1, stream2, stream3]);
    let events: Vec<_> = merged.collect().await;

    println!("  Received {} events:", events.len());
    for event in events {
        println!("    - Event type: {:?}, Data: {}", event.event, event.data);
    }

    // Example 8: SSE Response headers
    println!("\n8. SSE Response headers:");
    let headers = SseResponse::headers();
    for (name, value) in headers {
        println!("  {}: {}", name, value);
    }

    // Example 9: Convert stream to bytes
    println!("\n9. Convert stream to bytes:");
    let event_stream = stream! {
        yield SseEvent::new("Test message").with_event("test");
    };

    let sse_stream = SseStream::new(event_stream);
    let bytes_stream = sse_stream.into_bytes_stream();
    tokio::pin!(bytes_stream);

    if let Some(Ok(bytes)) = bytes_stream.next().await {
        let text = String::from_utf8_lossy(&bytes);
        println!("  Bytes representation:\n{}", text);
    }

    println!("=== All examples completed ===");
}
