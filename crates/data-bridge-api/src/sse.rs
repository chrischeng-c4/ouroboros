//! Server-Sent Events (SSE) support for data-bridge-api
//!
//! This module provides Server-Sent Events functionality for real-time
//! server-to-client streaming. It follows the data-bridge architecture principles:
//! - GIL-free event processing
//! - Async/await based on Tokio runtime
//! - Type-safe event handling
//! - Proper error handling with `ApiError`
//!
//! # Architecture
//!
//! ```text
//! HTTP Request → SSE Response → Event Stream
//!     ↓                              ↓
//! Headers set                   Send Events
//!     ↓                              ↓
//! Content-Type: text/event-stream   Format per SSE spec
//!     ↓
//! Stream body
//! ```
//!
//! # SSE Wire Format
//!
//! Per the SSE specification, each event is formatted as:
//! ```text
//! event: eventname
//! id: 123
//! retry: 3000
//! data: line1
//! data: line2
//!
//! ```
//!
//! # Example
//!
//! ```rust
//! use data_bridge_api::sse::{SseEvent, SseStream, SseResponse};
//! use async_stream::stream;
//!
//! # async fn example() {
//! // Create an event stream
//! let event_stream = stream! {
//!     for i in 0..10 {
//!         yield SseEvent::new(format!("Event {}", i))
//!             .with_event("counter")
//!             .with_id(i.to_string());
//!         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!     }
//! };
//!
//! // Create SSE response
//! let sse_stream = SseStream::new(event_stream);
//! let response = SseResponse::new(sse_stream);
//! # }
//! ```

use bytes::Bytes;
use futures_util::stream::Stream;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::debug;

// ============================================================================
// Core Types
// ============================================================================

/// Represents a single Server-Sent Event
///
/// An SSE event can contain:
/// - Data (required): The actual event data
/// - Event type (optional): A custom event name
/// - ID (optional): A unique identifier for client reconnection
/// - Retry (optional): How long the client should wait before reconnecting (milliseconds)
#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    /// The event data (required)
    pub data: String,
    /// Event type/name (optional)
    pub event: Option<String>,
    /// Event ID for client reconnection (optional)
    pub id: Option<String>,
    /// Retry interval in milliseconds (optional)
    pub retry: Option<u64>,
}

impl SseEvent {
    /// Create a new SSE event with the given data
    ///
    /// # Example
    /// ```
    /// use data_bridge_api::sse::SseEvent;
    ///
    /// let event = SseEvent::new("Hello, World!");
    /// ```
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            event: None,
            id: None,
            retry: None,
        }
    }

    /// Set the event type (builder pattern)
    ///
    /// # Example
    /// ```
    /// use data_bridge_api::sse::SseEvent;
    ///
    /// let event = SseEvent::new("Hello")
    ///     .with_event("greeting");
    /// ```
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Set the event ID (builder pattern)
    ///
    /// # Example
    /// ```
    /// use data_bridge_api::sse::SseEvent;
    ///
    /// let event = SseEvent::new("Hello")
    ///     .with_id("123");
    /// ```
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the retry interval (builder pattern)
    ///
    /// # Example
    /// ```
    /// use data_bridge_api::sse::SseEvent;
    ///
    /// let event = SseEvent::new("Hello")
    ///     .with_retry(3000); // 3 seconds
    /// ```
    pub fn with_retry(mut self, retry: u64) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Format this event according to the SSE specification
    ///
    /// The format is:
    /// Convert to bytes for transmission
    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(format_sse_event(self))
    }
}

impl fmt::Display for SseEvent {
    /// Format as SSE event string:
    /// ```text
    /// event: eventname
    /// id: 123
    /// retry: 3000
    /// data: line1
    /// data: line2
    ///
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_sse_event(self))
    }
}

// ============================================================================
// Stream Types
// ============================================================================

/// A stream of SSE events
///
/// Wraps an async stream of `SseEvent`s and provides methods to convert
/// to HTTP response bodies.
pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = SseEvent> + Send + 'static>>,
}

impl SseStream {
    /// Create a new SSE stream from an async stream of events
    ///
    /// # Example
    /// ```
    /// use data_bridge_api::sse::{SseEvent, SseStream};
    /// use async_stream::stream;
    ///
    /// let event_stream = stream! {
    ///     yield SseEvent::new("Event 1");
    ///     yield SseEvent::new("Event 2");
    /// };
    ///
    /// let sse_stream = SseStream::new(event_stream);
    /// ```
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = SseEvent> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Convert to a stream of `Bytes` for HTTP body
    ///
    /// This is used internally by `SseResponse` to create the HTTP response body.
    pub fn into_bytes_stream(self) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
        futures_util::stream::unfold(self.inner, |mut stream| async move {
            match futures_util::stream::StreamExt::next(&mut stream).await {
                Some(event) => {
                    let bytes = event.to_bytes();
                    debug!(
                        event_type = ?event.event,
                        event_id = ?event.id,
                        data_len = bytes.len(),
                        "Sending SSE event"
                    );
                    Some((Ok(bytes), stream))
                }
                None => None,
            }
        })
    }
}

impl Stream for SseStream {
    type Item = SseEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// HTTP response for Server-Sent Events
///
/// Sets the correct headers for SSE:
/// - `Content-Type: text/event-stream`
/// - `Cache-Control: no-cache`
/// - `Connection: keep-alive`
/// - `X-Accel-Buffering: no` (disable nginx buffering)
pub struct SseResponse {
    stream: SseStream,
}

impl SseResponse {
    /// Create a new SSE response from a stream
    ///
    /// # Example
    /// ```
    /// use data_bridge_api::sse::{SseEvent, SseStream, SseResponse};
    /// use async_stream::stream;
    ///
    /// # async fn example() {
    /// let event_stream = stream! {
    ///     yield SseEvent::new("Hello");
    /// };
    ///
    /// let sse_stream = SseStream::new(event_stream);
    /// let response = SseResponse::new(sse_stream);
    /// # }
    /// ```
    pub fn new(stream: SseStream) -> Self {
        Self { stream }
    }

    /// Get the SSE headers
    ///
    /// Returns the headers that should be set on the HTTP response:
    /// - `Content-Type: text/event-stream`
    /// - `Cache-Control: no-cache`
    /// - `Connection: keep-alive`
    /// - `X-Accel-Buffering: no`
    pub fn headers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "keep-alive"),
            ("X-Accel-Buffering", "no"), // Disable nginx buffering
        ]
    }

    /// Convert to a stream of bytes for HTTP body
    ///
    /// This consumes the response and returns a stream that can be used
    /// as an HTTP response body.
    pub fn into_bytes_stream(self) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
        self.stream.into_bytes_stream()
    }

    /// Convert to a Hyper body
    ///
    /// This is used to integrate with Hyper-based HTTP servers.
    pub fn into_hyper_body(
        self,
    ) -> http_body_util::StreamBody<
        impl Stream<Item = Result<hyper::body::Frame<Bytes>, std::io::Error>>,
    > {
        use http_body_util::StreamBody;
        use hyper::body::Frame;

        let stream = self.stream.into_bytes_stream();
        let frame_stream =
            futures_util::stream::StreamExt::map(stream, |result| {
                result.map(Frame::data)
            });
        StreamBody::new(frame_stream)
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Format an SSE event according to the specification
///
/// The format is:
/// ```text
/// event: eventname
/// id: 123
/// retry: 3000
/// data: line1
/// data: line2
///
/// ```
///
/// Note: The trailing blank line is required by the SSE spec.
pub fn format_sse_event(event: &SseEvent) -> String {
    let mut output = String::new();

    // Event type (optional)
    if let Some(ref event_type) = event.event {
        output.push_str(&format!("event: {}\n", event_type));
    }

    // Event ID (optional)
    if let Some(ref id) = event.id {
        output.push_str(&format!("id: {}\n", id));
    }

    // Retry interval (optional)
    if let Some(retry) = event.retry {
        output.push_str(&format!("retry: {}\n", retry));
    }

    // Data (required) - support multi-line data
    for line in event.data.lines() {
        output.push_str(&format!("data: {}\n", line));
    }

    // Blank line to end the event
    output.push('\n');

    output
}

/// Create a keep-alive comment
///
/// Returns a comment line that can be sent to keep the connection alive.
/// Comments are lines starting with `:` and are ignored by the client.
///
/// # Example
/// ```
/// use data_bridge_api::sse::keep_alive_comment;
///
/// let comment = keep_alive_comment();
/// assert_eq!(comment, ": keep-alive\n\n");
/// ```
pub fn keep_alive_comment() -> String {
    ": keep-alive\n\n".to_string()
}

/// Create a keep-alive comment as bytes
pub fn keep_alive_comment_bytes() -> Bytes {
    Bytes::from_static(b": keep-alive\n\n")
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a stream that sends keep-alive comments at regular intervals
///
/// This is useful to prevent proxies and firewalls from closing idle connections.
///
/// # Example
/// ```
/// use data_bridge_api::sse::keep_alive_stream;
/// use std::time::Duration;
///
/// # async fn example() {
/// let keep_alive = keep_alive_stream(Duration::from_secs(15));
/// # }
/// ```
pub fn keep_alive_stream(
    interval: std::time::Duration,
) -> impl Stream<Item = SseEvent> {
    async_stream::stream! {
        let mut interval = tokio::time::interval(interval);
        loop {
            interval.tick().await;
            // Send a comment as a special "keep-alive" event
            // We use an empty data field since comments don't have a formal representation in SseEvent
            yield SseEvent {
                data: String::new(),
                event: Some("keep-alive".to_string()),
                id: None,
                retry: None,
            };
        }
    }
}

/// Merge multiple event streams into a single stream
///
/// This is useful when you want to combine multiple event sources into
/// a single SSE connection.
///
/// # Example
/// ```
/// use data_bridge_api::sse::{SseEvent, merge_streams};
/// use async_stream::stream;
///
/// # async fn example() {
/// let stream1: std::pin::Pin<Box<dyn futures_util::Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
///     yield SseEvent::new("Event 1");
/// });
///
/// let stream2: std::pin::Pin<Box<dyn futures_util::Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
///     yield SseEvent::new("Event 2");
/// });
///
/// let merged = merge_streams(vec![stream1, stream2]);
/// # }
/// ```
pub fn merge_streams(
    streams: Vec<Pin<Box<dyn Stream<Item = SseEvent> + Send>>>,
) -> impl Stream<Item = SseEvent> {
    futures_util::stream::select_all(streams)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;
    use futures_util::StreamExt;

    #[test]
    fn test_sse_event_new() {
        let event = SseEvent::new("Hello, World!");
        assert_eq!(event.data, "Hello, World!");
        assert_eq!(event.event, None);
        assert_eq!(event.id, None);
        assert_eq!(event.retry, None);
    }

    #[test]
    fn test_sse_event_builder() {
        let event = SseEvent::new("Test data")
            .with_event("test-event")
            .with_id("123")
            .with_retry(3000);

        assert_eq!(event.data, "Test data");
        assert_eq!(event.event, Some("test-event".to_string()));
        assert_eq!(event.id, Some("123".to_string()));
        assert_eq!(event.retry, Some(3000));
    }

    #[test]
    fn test_format_sse_event_minimal() {
        let event = SseEvent::new("Hello");
        let formatted = format_sse_event(&event);
        assert_eq!(formatted, "data: Hello\n\n");
    }

    #[test]
    fn test_format_sse_event_full() {
        let event = SseEvent::new("Test data")
            .with_event("test")
            .with_id("42")
            .with_retry(5000);

        let formatted = format_sse_event(&event);
        assert_eq!(
            formatted,
            "event: test\nid: 42\nretry: 5000\ndata: Test data\n\n"
        );
    }

    #[test]
    fn test_format_sse_event_multiline() {
        let event = SseEvent::new("Line 1\nLine 2\nLine 3");
        let formatted = format_sse_event(&event);
        assert_eq!(formatted, "data: Line 1\ndata: Line 2\ndata: Line 3\n\n");
    }

    #[test]
    fn test_keep_alive_comment() {
        let comment = keep_alive_comment();
        assert_eq!(comment, ": keep-alive\n\n");
    }

    #[tokio::test]
    async fn test_sse_stream() {
        let event_stream = stream! {
            yield SseEvent::new("Event 1");
            yield SseEvent::new("Event 2");
            yield SseEvent::new("Event 3");
        };

        let mut sse_stream = SseStream::new(event_stream);

        let event1 = sse_stream.next().await.unwrap();
        assert_eq!(event1.data, "Event 1");

        let event2 = sse_stream.next().await.unwrap();
        assert_eq!(event2.data, "Event 2");

        let event3 = sse_stream.next().await.unwrap();
        assert_eq!(event3.data, "Event 3");

        let end = sse_stream.next().await;
        assert!(end.is_none());
    }

    #[tokio::test]
    async fn test_sse_stream_into_bytes() {
        let event_stream = stream! {
            yield SseEvent::new("Hello").with_event("greeting");
        };

        let sse_stream = SseStream::new(event_stream);
        let bytes_stream = sse_stream.into_bytes_stream();
        tokio::pin!(bytes_stream);

        let bytes = bytes_stream.next().await.unwrap().unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(text, "event: greeting\ndata: Hello\n\n");
    }

    #[test]
    fn test_sse_response_headers() {
        let headers = SseResponse::headers();
        assert_eq!(headers.len(), 4);
        assert!(headers.contains(&("Content-Type", "text/event-stream")));
        assert!(headers.contains(&("Cache-Control", "no-cache")));
        assert!(headers.contains(&("Connection", "keep-alive")));
        assert!(headers.contains(&("X-Accel-Buffering", "no")));
    }

    #[tokio::test]
    async fn test_sse_response_into_bytes_stream() {
        let event_stream = stream! {
            yield SseEvent::new("Event 1");
            yield SseEvent::new("Event 2");
        };

        let sse_stream = SseStream::new(event_stream);
        let response = SseResponse::new(sse_stream);
        let bytes_stream = response.into_bytes_stream();
        tokio::pin!(bytes_stream);

        let bytes1 = bytes_stream.next().await.unwrap().unwrap();
        let text1 = String::from_utf8(bytes1.to_vec()).unwrap();
        assert_eq!(text1, "data: Event 1\n\n");

        let bytes2 = bytes_stream.next().await.unwrap().unwrap();
        let text2 = String::from_utf8(bytes2.to_vec()).unwrap();
        assert_eq!(text2, "data: Event 2\n\n");

        let end = bytes_stream.next().await;
        assert!(end.is_none());
    }

    #[tokio::test]
    async fn test_keep_alive_stream() {
        use std::time::Duration;

        let stream = keep_alive_stream(Duration::from_millis(10));
        tokio::pin!(stream);

        // Get first keep-alive event
        let event = tokio::time::timeout(Duration::from_millis(50), stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(event.data, "");
        assert_eq!(event.event, Some("keep-alive".to_string()));
    }

    #[tokio::test]
    async fn test_merge_streams() {
        let stream1: Pin<Box<dyn Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
            yield SseEvent::new("A");
        });

        let stream2: Pin<Box<dyn Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
            yield SseEvent::new("B");
        });

        let stream3: Pin<Box<dyn Stream<Item = SseEvent> + Send>> = Box::pin(stream! {
            yield SseEvent::new("C");
        });

        let merged = merge_streams(vec![stream1, stream2, stream3]);

        let events: Vec<_> = merged.collect().await;
        assert_eq!(events.len(), 3);

        // Events may arrive in any order due to select_all
        let data: Vec<_> = events.iter().map(|e| e.data.as_str()).collect();
        assert!(data.contains(&"A"));
        assert!(data.contains(&"B"));
        assert!(data.contains(&"C"));
    }

    #[test]
    fn test_sse_event_to_bytes() {
        let event = SseEvent::new("Hello").with_event("greeting");
        let bytes = event.to_bytes();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(text, "event: greeting\ndata: Hello\n\n");
    }

    #[test]
    fn test_keep_alive_comment_bytes() {
        let bytes = keep_alive_comment_bytes();
        assert_eq!(bytes.as_ref(), b": keep-alive\n\n");
    }
}
