#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Proof-of-Work helpers: header hashing and target checks.

use bitquan_types::BlockHeader;
use sha2::{Digest, Sha256};

/// Computes double-SHA256 hash of the block header (Bitcoin-style), big-endian bytes.
pub fn header_hash(header: &BlockHeader) -> [u8; 32] {
    let bytes = header.to_bytes();
    let h1 = Sha256::digest(&bytes);
    let h2 = Sha256::digest(&h1);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h2);
    out
}

/// Converts compact `bits` to a 32-byte big-endian target.
pub fn compact_to_target_bytes(bits: u32) -> [u8; 32] {
    let exponent = (bits >> 24) as i32;
    let mantissa = bits & 0x007f_ffff; // 23-bit mantissa per Bitcoin-style rule
    let mut target = [0u8; 32];
    if exponent <= 3 {
        // mantissa << (8*(3-exponent))
        let shift = (3 - exponent) as usize;
        let m = (mantissa as u64) << (8 * shift);
        target[24..32].copy_from_slice(&m.to_be_bytes());
    } else {
        let byte_pos = exponent as usize - 3; // number of bytes mantissa occupies from the left
        // place mantissa at the leftmost bytes (big-endian)
        let mut m_bytes = [0u8; 4];
        m_bytes.copy_from_slice(&(mantissa as u32).to_be_bytes());
        // mantissa is 3 bytes; take the last 3 bytes of m_bytes
        let mantissa_bytes = &m_bytes[1..4];
        let start = 32 - byte_pos;
        let end = start + 3;
        if end <= 32 {
            target[start..end].copy_from_slice(mantissa_bytes);
        } else {
            // overflow -> saturate to max
            target = [0xFF; 32];
        }
    }
    target
}

/// Returns true if `hash` (big-endian) <= `target` (big-endian).
pub fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    // Lexicographic compare on big-endian byte arrays
    hash <= target
}

/// Checks Proof-of-Work for the header using its `bits`.
pub fn check_header_pow(header: &BlockHeader) -> bool {
    let hash = header_hash(header);
    let target = compact_to_target_bytes(header.bits);
    hash_meets_target(&hash, &target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_hash_changes_with_nonce() {
        let mut hdr = dummy_header();
        let h1 = header_hash(&hdr);
        hdr.nonce = 1;
        let h2 = header_hash(&hdr);
        assert_ne!(h1, h2);
    }

    #[test]
    fn target_bytes_monotonic_with_bits() {
        // Larger exponent -> larger target (easier)
        let t1 = compact_to_target_bytes(0x1d00ffff);
        let t2 = compact_to_target_bytes(0x1f00ffff);
        assert!(t1 < t2);
    }

    fn dummy_header() -> BlockHeader {
        BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 0,
            bits: 0x207fffff, // very easy target (like regtest/testnet style)
            nonce: 0,
        }
    }
}
