//! Minimal length-prefixed wire I/O helpers for MessageEnvelope.
//! BQIP-0006: zstd compression applied transparently on the wire.
use std::io::{Read, Write};

use crate::compression::{compress_block, decompress_block};
use crate::protocol::{MessageEnvelope, P2pError, MAX_MESSAGE_SIZE};

/// Send a serialized MessageEnvelope with a 4-byte little-endian length prefix.
/// BQIP-0006: zstd-compresses the payload before sending.
pub fn send_envelope<W: Write>(w: &mut W, env: &MessageEnvelope) -> Result<(), P2pError> {
    let bytes = env
        .serialize()
        .map_err(|e| P2pError::SerializationError(e.to_string()))?;
    let bytes = compress_block(&bytes)?;
    let len = bytes.len() as u32;
    w.write_all(&len.to_le_bytes())
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    w.write_all(&bytes)
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    Ok(())
}

/// Receive a length-prefixed serialized MessageEnvelope.
pub fn recv_envelope<R: Read>(
    r: &mut R,
    expected_magic: [u8; 4],
) -> Result<MessageEnvelope, P2pError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Err(P2pError::InvalidMessage);
    }
    if len > MAX_MESSAGE_SIZE {
        return Err(P2pError::MessageTooLarge(len));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    // BQIP-0006: decompress if peer sent compressed payload
    let buf = decompress_block(&buf)?;
    MessageEnvelope::deserialize(&buf, expected_magic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    struct LenOnlyReader {
        bytes: [u8; 4],
        pos: usize,
    }

    impl LenOnlyReader {
        fn new(len: usize) -> Self {
            Self {
                bytes: (len as u32).to_le_bytes(),
                pos: 0,
            }
        }
    }

    impl Read for LenOnlyReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.pos >= self.bytes.len() {
                return Ok(0);
            }
            let remaining = self.bytes.len() - self.pos;
            let to_copy = remaining.min(buf.len());
            buf[..to_copy].copy_from_slice(&self.bytes[self.pos..self.pos + to_copy]);
            self.pos += to_copy;
            Ok(to_copy)
        }
    }

    #[test]
    fn rejects_oversized_length_prefix() {
        let mut reader = LenOnlyReader::new(MAX_MESSAGE_SIZE + 1);
        let err = recv_envelope(&mut reader, [0u8; 4]).expect_err("should reject oversize");
        match err {
            P2pError::MessageTooLarge(size) => assert_eq!(size, MAX_MESSAGE_SIZE + 1),
            other => unreachable!("unexpected error: {:?}", other),
        }
    }
}
