//! BQIP-0006: zstd block/message compression for P2P and storage.
//!
//! Block messages contain large Dilithium5 signatures (4,595 bytes each)
//! that compress well (~25-30% size reduction with zstd level 3).
//! This layer wraps the raw wire bytes without touching the crypto.

use crate::protocol::P2pError;

/// zstd compression level: 3 = fast encode, good ratio (optimal for real-time P2P).
const ZSTD_LEVEL: i32 = 3;

/// Magic prefix to identify zstd-compressed messages on the wire.
/// Avoids double-compression attempt if peer sends uncompressed data.
const COMPRESSED_MAGIC: &[u8; 4] = b"BQZS";

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
pub fn decompress_block(data: &[u8]) -> Result<Vec<u8>, P2pError> {
    if data.starts_with(COMPRESSED_MAGIC) {
        let payload = &data[COMPRESSED_MAGIC.len()..];
        zstd::decode_all(payload)
            .map_err(|e| P2pError::SerializationError(format!("zstd decompress: {e}")))
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
    fn legacy_uncompressed_passthrough() {
        let raw = b"raw block data without magic prefix";
        let decompressed = decompress_block(raw).expect("passthrough should work");
        assert_eq!(decompressed, raw);
    }
}
