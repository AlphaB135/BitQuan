//! Minimal length-prefixed wire I/O helpers for MessageEnvelope.
use std::io::{Read, Write};

use crate::protocol::{MessageEnvelope, P2pError};

/// Send a serialized MessageEnvelope with a 4-byte little-endian length prefix.
pub fn send_envelope<W: Write>(w: &mut W, env: &MessageEnvelope) -> Result<(), P2pError> {
    let bytes = env
        .serialize()
        .map_err(|e| P2pError::SerializationError(e.to_string()))?;
    let len = bytes.len() as u32;
    w.write_all(&len.to_le_bytes())
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    w.write_all(&bytes)
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    Ok(())
}

/// Receive a length-prefixed serialized MessageEnvelope.
pub fn recv_envelope<R: Read>(r: &mut R) -> Result<MessageEnvelope, P2pError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    MessageEnvelope::deserialize(&buf)
}
