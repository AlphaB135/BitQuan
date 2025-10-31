#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Proof-of-Work helpers: header hashing and target checks with bounded difficulty.

use bitquan_types::BlockHeader;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Minimum compact bits permitted on dev/test networks (hardest difficulty).
pub const DEVNET_MIN_BITS: u32 = 0x1d00ffff;
/// Maximum compact bits permitted on dev/test networks (easiest difficulty).
pub const DEVNET_MAX_BITS: u32 = 0x207fffff;

/// Errors returned by Proof-of-Work helpers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PowError {
    /// The encoded target bits exceed the allowed min/max envelope.
    #[error("pow target bits {bits:#010x} out of range [{min:#010x}, {max:#010x}]")]
    PowTargetOutOfRange {
        /// Encoded compact bits that violated the bounds.
        bits: u32,
        /// Minimum permitted bits (hardest difficulty).
        min: u32,
        /// Maximum permitted bits (easiest difficulty).
        max: u32,
    },
}

/// Computes double-SHA256 hash of the block header (Bitcoin-style), big-endian bytes.
pub fn header_hash(header: &BlockHeader) -> [u8; 32] {
    let bytes = header.to_bytes();
    let h1 = Sha256::digest(&bytes);
    let h2 = Sha256::digest(h1);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h2);
    out
}

/// Converts compact `bits` to a 32-byte big-endian target.
pub fn compact_to_target_bytes(bits: u32) -> Result<[u8; 32], PowError> {
    let bits = clamp_bits(bits)?;
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
        m_bytes.copy_from_slice(&mantissa.to_be_bytes());
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
    Ok(target)
}

/// Returns true if `hash` (big-endian) <= `target` (big-endian).
pub fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    // Lexicographic compare on big-endian byte arrays
    hash <= target
}

/// Checks Proof-of-Work for the header using its `bits`.
pub fn check_header_pow(header: &BlockHeader) -> Result<bool, PowError> {
    let hash = header_hash(header);
    let target = compact_to_target_bytes(header.bits)?;
    Ok(hash_meets_target(&hash, &target))
}

/// Ensures a compact bits value stays within devnet bounds.
pub fn clamp_bits_within_bounds(bits: u32) -> u32 {
    bits.clamp(DEVNET_MIN_BITS, DEVNET_MAX_BITS)
}

fn clamp_bits(bits: u32) -> Result<u32, PowError> {
    if (DEVNET_MIN_BITS..=DEVNET_MAX_BITS).contains(&bits) {
        Ok(bits)
    } else {
        Err(PowError::PowTargetOutOfRange {
            bits,
            min: DEVNET_MIN_BITS,
            max: DEVNET_MAX_BITS,
        })
    }
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
        let t1 = compact_to_target_bytes(0x1d00ffff).unwrap();
        let t2 = compact_to_target_bytes(0x1f00ffff).unwrap();
        assert!(t1 < t2);
    }

    #[test]
    fn target_clamped_at_bounds() {
        assert!(compact_to_target_bytes(DEVNET_MIN_BITS).is_ok());
        assert!(compact_to_target_bytes(DEVNET_MAX_BITS).is_ok());
        let high = DEVNET_MAX_BITS.saturating_add(0x0100_0000);
        let low = DEVNET_MIN_BITS.saturating_sub(0x0100_0000);
        assert_eq!(
            compact_to_target_bytes(high),
            Err(PowError::PowTargetOutOfRange {
                bits: high,
                min: DEVNET_MIN_BITS,
                max: DEVNET_MAX_BITS
            })
        );
        assert_eq!(
            compact_to_target_bytes(low),
            Err(PowError::PowTargetOutOfRange {
                bits: low,
                min: DEVNET_MIN_BITS,
                max: DEVNET_MAX_BITS
            })
        );
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
