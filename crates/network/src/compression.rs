//! BQIP-0006: zstd block/message compression for P2P and storage.
//!
//! Block messages contain large Dilithium5 signatures (4,595 bytes each)
//! that compress well (~25-30% size reduction with zstd level 3).
//! This layer wraps the raw wire bytes without touching the crypto.

use std::io::Read as _;

use crate::protocol::P2pError;

/// zstd compression level: 3 = fast encode, good ratio (optimal for real-time P2P).
const ZSTD_LEVEL: i32 = 3;

/// Magic prefix to identify zstd-compressed messages on the wire.
/// Avoids double-compression attempt if peer sends uncompressed data.
const COMPRESSED_MAGIC: &[u8; 4] = b"BQZS";

/// Maximum number of bytes accepted after decompression.
///
/// Prevents decompression-bomb DoS: a malicious peer could send a ~1 MB
/// zstd payload that expands to gigabytes, OOM-killing the node before any
/// authentication or rate limiting runs. 32 MB matches Bitcoin Core's
/// MAX_PROTOCOL_MESSAGE_LENGTH and is well above any legitimate block size
/// at current Dilithium5 signature sizes (~4.6 KB × max transactions).
///
/// Fixes issue #201.
const MAX_DECOMPRESSED_SIZE: usize = 32 * 1024 * 1024; // 32 MB

/// Compresses raw block bytes using zstd.
///
/// Prepends a 4-byte [`COMPRESSED_MAGIC`] so receivers can detect compression.
/// Returns the original bytes untouched if compression somehow increases size.
pub fn compress_block(raw: &[u8]) -> Result<Vec<u8>, P2pError> {
    let compressed = zstd::encode_all(raw, ZSTD_LEVEL)
        .map_err(|e| P2pError::SerializationError(format!("zstd compress: {e}")))?;

    // Only use compressed form if it's actually smaller
    if compressed.len() + COMPRESSED_MAGIC.len() < raw.len() {
        let mut out = Vec::with_capacity(COMPRESSED_MAGIC.len() + compressed.len());
        out.extend_from_slice(COMPRESSED_MAGIC);
        out.extend_from_slice(&compressed);
        Ok(out)
    } else {
        // Passthrough: skip compression for already-small messages
        Ok(raw.to_vec())
    }
}

/// Decompresses block bytes. Detects magic prefix; passthrough if uncompressed.
///
/// Enforces [`MAX_DECOMPRESSED_SIZE`] to prevent decompression-bomb DoS attacks
/// where a small compressed payload expands to a gigabyte allocation.
pub fn decompress_block(data: &[u8]) -> Result<Vec<u8>, P2pError> {
    if data.starts_with(COMPRESSED_MAGIC) {
        let payload = &data[COMPRESSED_MAGIC.len()..];
        let decoder = zstd::Decoder::new(payload)
            .map_err(|e| P2pError::SerializationError(format!("zstd init: {e}")))?;
        let mut output = Vec::with_capacity(payload.len().min(MAX_DECOMPRESSED_SIZE));
        // Read at most MAX_DECOMPRESSED_SIZE + 1 bytes.
        // If we get more than the limit, the payload is a decompression bomb.
        decoder
            .take((MAX_DECOMPRESSED_SIZE + 1) as u64)
            .read_to_end(&mut output)
            .map_err(|e| P2pError::SerializationError(format!("zstd decompress: {e}")))?;
        if output.len() > MAX_DECOMPRESSED_SIZE {
            return Err(P2pError::SerializationError(format!(
                "decompressed size exceeds limit ({} bytes > {} bytes max)",
                output.len(),
                MAX_DECOMPRESSED_SIZE,
            )));
        }
        Ok(output)
    } else {
        // Legacy / uncompressed peer — passthrough
        Ok(data.to_vec())
    }
}

/// Returns true if the byte slice is zstd-compressed by this implementation.
pub fn is_compressed(data: &[u8]) -> bool {
    data.starts_with(COMPRESSED_MAGIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_decompress_roundtrip() {
        // Simulate a block with many repeated Dilithium5 signature bytes (very compressible)
        let fake_block = vec![0xABu8; 100_000];
        let compressed = compress_block(&fake_block).expect("compress should succeed");

        // Should actually be compressed (repeated bytes compress very well)
        assert!(is_compressed(&compressed));
        assert!(
            compressed.len() < fake_block.len(),
            "compressed should be smaller: {} vs {}",
            compressed.len(),
            fake_block.len()
        );

        let decompressed = decompress_block(&compressed).expect("decompress should succeed");
        assert_eq!(decompressed, fake_block);
    }

    #[test]
    fn passthrough_if_compression_does_not_help() {
        // Random-like data that doesn't compress well — should pass through
        let random_data: Vec<u8> = (0..100).map(|i| i as u8).collect();
        let result = compress_block(&random_data).expect("should not error");
        // Small data — either passthrough or compressed, both valid
        let decompressed = decompress_block(&result).expect("should decompress");
        assert_eq!(decompressed, random_data);
    }

    #[test]
    fn decompression_bomb_is_rejected() {
        // Regression test for issue #201.
        // Build a highly compressible payload > MAX_DECOMPRESSED_SIZE and verify
        // that decompress_block() rejects it before allocating unbounded memory.
        //
        // We create a payload that would decompress to MAX + 1 bytes.
        // Because we use .take(MAX + 1), the actual allocation stays bounded.
        let bomb_raw = vec![0xABu8; MAX_DECOMPRESSED_SIZE + 1];
        let mut compressed = Vec::new();
        compressed.extend_from_slice(COMPRESSED_MAGIC);
        let zstd_bytes = zstd::encode_all(bomb_raw.as_slice(), ZSTD_LEVEL)
            .expect("test compression should succeed");
        compressed.extend_from_slice(&zstd_bytes);

        let result = decompress_block(&compressed);
        assert!(
            result.is_err(),
            "decompression bomb must be rejected, got {} bytes",
            result.unwrap().len()
        );
    }

    #[test]
    fn legitimate_large_block_within_limit_is_accepted() {
        // A block at exactly MAX_DECOMPRESSED_SIZE bytes must pass.
        let large_block = vec![0xCDu8; MAX_DECOMPRESSED_SIZE];
        let compressed = compress_block(&large_block).expect("compress should succeed");
        let decompressed = decompress_block(&compressed).expect("within limit must succeed");
        assert_eq!(decompressed, large_block);
    }

    #[test]
    fn uncompressed_passthrough_works() {
        let raw = b"raw block data without magic prefix";
        let decompressed = decompress_block(raw).expect("passthrough should work");
        assert_eq!(decompressed, raw);
    }
}
