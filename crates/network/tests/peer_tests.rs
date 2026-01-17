//! Tests for peer I/O operations
//!
//! Note: The `handshake()` function now requires a NoiseTransport (encrypted channel).
//! Timeout behavior is tested at the Noise handshake level in noise.rs tests.

use std::io::Cursor;

use bitquan_network::peer::{read_frame, MAX_MSG_BYTES};
use bitquan_types::error::Error;

#[test]
fn oversized_message_rejected_without_panic() {
    let len = (MAX_MSG_BYTES + 1) as u32;
    let mut data = Vec::with_capacity(4 + len as usize);
    data.extend_from_slice(&len.to_le_bytes());
    data.resize(4 + len as usize, 0xAA);
    let mut cursor = Cursor::new(data);
    let result = read_frame(&mut cursor);
    assert!(matches!(result, Err(Error::Invalid(msg)) if msg.contains("message too large")));
}

#[test]
fn empty_message_rejected() {
    let mut cursor = Cursor::new(0u32.to_le_bytes().to_vec());
    let result = read_frame(&mut cursor);
    assert!(matches!(result, Err(Error::Invalid(msg)) if msg == "empty frame"));
}
