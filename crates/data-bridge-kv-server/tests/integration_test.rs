//! Integration tests for KV server

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Helper to encode a simple PING command
fn encode_ping() -> Vec<u8> {
    // Command: PING (0x08), Payload length: 0
    vec![0x08, 0x00, 0x00, 0x00, 0x00]
}

#[tokio::test]
#[ignore] // Run with --ignored when server is running
async fn test_server_ping() {
    // This test requires a running server on localhost:6380
    // Run with: cargo test -p data-bridge-kv-server -- --ignored
    let mut stream = timeout(
        Duration::from_secs(2),
        TcpStream::connect("127.0.0.1:6380")
    )
    .await
    .expect("Server not running on 127.0.0.1:6380")
    .expect("Failed to connect");

    // Send PING
    let ping_cmd = encode_ping();
    stream.write_all(&ping_cmd).await.unwrap();

    // Read response header (5 bytes)
    let mut header = [0u8; 5];
    timeout(Duration::from_secs(1), stream.read_exact(&mut header))
        .await
        .expect("Timeout reading header")
        .expect("Failed to read header");

    // Status should be OK (0x00)
    assert_eq!(header[0], 0x00);

    // Payload length
    let payload_len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    assert_eq!(payload_len, 4); // "PONG" = 4 bytes

    // Read payload
    let mut payload = vec![0u8; payload_len as usize];
    timeout(Duration::from_secs(1), stream.read_exact(&mut payload))
        .await
        .expect("Timeout reading payload")
        .expect("Failed to read payload");
    assert_eq!(&payload, b"PONG");
}

#[test]
fn test_protocol_encoding() {
    // Test that we can encode commands correctly
    let ping_cmd = encode_ping();
    assert_eq!(ping_cmd.len(), 5);
    assert_eq!(ping_cmd[0], 0x08); // PING command
    assert_eq!(&ping_cmd[1..], &[0, 0, 0, 0]); // Zero payload length
}
