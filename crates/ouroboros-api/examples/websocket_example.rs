//! WebSocket Example
//!
//! This example demonstrates WebSocket concepts in ouroboros-api,
//! including message types and close frame handling.
//!
//! Run with:
//! ```bash
//! cargo run --example websocket_example -p ouroboros-api
//! ```

use ouroboros_api::websocket::{
    is_websocket_upgrade, WebSocketMessage, CloseFrame,
};
use ouroboros_api::request::{HttpMethod, SerializableRequest, SerializableValue};

// ============================================================================
// WebSocket Upgrade Check
// ============================================================================

/// Example of checking WebSocket upgrade request
fn check_websocket_request() {
    println!("1. WebSocket Upgrade Request Check");
    println!("-----------------------------------");

    // Create a mock WebSocket upgrade request
    let req = SerializableRequest::new(HttpMethod::Get, "/ws")
        .with_header("upgrade", "websocket")
        .with_header("connection", "upgrade")
        .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .with_header("sec-websocket-version", "13");

    // Check if this is a WebSocket upgrade request
    if is_websocket_upgrade(&req) {
        println!("  Valid WebSocket upgrade request detected!");
    } else {
        println!("  Not a WebSocket request");
    }

    // Test invalid request
    let invalid_req = SerializableRequest::new(HttpMethod::Get, "/ws");
    if !is_websocket_upgrade(&invalid_req) {
        println!("  Regular HTTP request (not WebSocket)");
    }
    println!();
}

// ============================================================================
// Message Types Demo
// ============================================================================

/// Example of WebSocket message types
fn demonstrate_message_types() {
    println!("2. WebSocket Message Types");
    println!("--------------------------");

    // Text message
    let text_msg = WebSocketMessage::Text("Hello, WebSocket!".to_string());
    println!("  Text: {:?}", text_msg);
    println!("    is_text: {}, is_binary: {}", text_msg.is_text(), text_msg.is_binary());

    // Binary message
    let binary_msg = WebSocketMessage::Binary(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    println!("  Binary: {:?}", binary_msg);
    println!("    is_binary: {}", binary_msg.is_binary());

    // Ping/Pong for keep-alive
    let ping = WebSocketMessage::Ping(vec![1, 2, 3, 4]);
    let pong = WebSocketMessage::Pong(vec![1, 2, 3, 4]);
    println!("  Ping: {:?}", ping);
    println!("  Pong: {:?}", pong);
    println!("    is_ping: {}, is_pong: {}", ping.is_ping(), pong.is_pong());

    // Close message with reason
    let close = WebSocketMessage::Close(Some(CloseFrame::normal()));
    println!("  Close: {:?}", close);
    println!("    is_close: {}", close.is_close());
    println!();
}

// ============================================================================
// Close Frame Examples
// ============================================================================

fn demonstrate_close_frames() {
    println!("3. Close Frame Types");
    println!("--------------------");

    // Normal closure (1000)
    let normal = CloseFrame::normal();
    println!("  Normal (1000): code={}, reason='{}'", normal.code, normal.reason);

    // Going away (1001)
    let going_away = CloseFrame::going_away("Server restarting");
    println!("  Going Away (1001): code={}, reason='{}'", going_away.code, going_away.reason);

    // Protocol error (1002)
    let protocol_error = CloseFrame::protocol_error("Invalid frame format");
    println!("  Protocol Error (1002): code={}, reason='{}'", protocol_error.code, protocol_error.reason);

    // Internal error (1011)
    let internal = CloseFrame::internal_error("Database connection lost");
    println!("  Internal Error (1011): code={}, reason='{}'", internal.code, internal.reason);

    // Custom close frame
    let custom = CloseFrame::new(4000, "Custom application error");
    println!("  Custom (4000): code={}, reason='{}'", custom.code, custom.reason);
    println!();
}

// ============================================================================
// Echo Handler Example
// ============================================================================

/// Example of a simple echo handler logic
fn echo_message(msg: &WebSocketMessage) -> Option<WebSocketMessage> {
    match msg {
        WebSocketMessage::Text(text) => {
            Some(WebSocketMessage::Text(format!("Echo: {}", text)))
        }
        WebSocketMessage::Binary(data) => {
            Some(WebSocketMessage::Binary(data.clone()))
        }
        WebSocketMessage::Ping(payload) => {
            Some(WebSocketMessage::Pong(payload.clone()))
        }
        WebSocketMessage::Close(_) => {
            Some(WebSocketMessage::Close(Some(CloseFrame::normal())))
        }
        _ => None,
    }
}

fn demonstrate_echo_handler() {
    println!("4. Echo Handler Pattern");
    println!("-----------------------");

    let messages = [
        WebSocketMessage::Text("Hello, World!".to_string()),
        WebSocketMessage::Binary(vec![1, 2, 3]),
        WebSocketMessage::Ping(vec![4, 5, 6]),
    ];

    for msg in messages {
        if let Some(response) = echo_message(&msg) {
            println!("  Input:  {:?}", msg);
            println!("  Output: {:?}", response);
            println!();
        }
    }
}

// ============================================================================
// Chat Message Handler Example
// ============================================================================

fn handle_chat_message(msg: &str) -> SerializableValue {
    if msg.starts_with("/") {
        let parts: Vec<&str> = msg.splitn(2, ' ').collect();
        match parts.first() {
            Some(&"/help") => {
                SerializableValue::Object(vec![
                    ("type".to_string(), SerializableValue::String("help".to_string())),
                    ("commands".to_string(), SerializableValue::List(vec![
                        SerializableValue::String("/help - Show this help".to_string()),
                        SerializableValue::String("/nick <name> - Set nickname".to_string()),
                        SerializableValue::String("/quit - Disconnect".to_string()),
                    ])),
                ])
            }
            Some(&"/nick") if parts.len() > 1 => {
                SerializableValue::Object(vec![
                    ("type".to_string(), SerializableValue::String("nick_change".to_string())),
                    ("new_nick".to_string(), SerializableValue::String(parts[1].to_string())),
                ])
            }
            Some(&"/quit") => {
                SerializableValue::Object(vec![
                    ("type".to_string(), SerializableValue::String("quit".to_string())),
                ])
            }
            _ => {
                SerializableValue::Object(vec![
                    ("type".to_string(), SerializableValue::String("error".to_string())),
                    ("message".to_string(), SerializableValue::String("Unknown command".to_string())),
                ])
            }
        }
    } else {
        SerializableValue::Object(vec![
            ("type".to_string(), SerializableValue::String("message".to_string())),
            ("content".to_string(), SerializableValue::String(msg.to_string())),
        ])
    }
}

fn demonstrate_chat_handler() {
    println!("5. Chat Message Handler Pattern");
    println!("--------------------------------");

    let messages = ["/help", "/nick Alice", "Hello everyone!", "/unknown"];
    for msg in messages {
        let result = handle_chat_message(msg);
        println!("  Input:  '{}'", msg);
        println!("  Output: {:?}", result);
        println!();
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("WebSocket Example");
    println!("=================\n");

    check_websocket_request();
    demonstrate_message_types();
    demonstrate_close_frames();
    demonstrate_echo_handler();
    demonstrate_chat_handler();

    println!("Note: For actual WebSocket connections, run a full server");
    println!("and connect using a WebSocket client like wscat or websocat.");
}
