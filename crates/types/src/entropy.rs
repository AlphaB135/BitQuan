//! Secure entropy generation utilities.
//!
//! All randomness in BitQuan MUST use cryptographically secure sources.
//! This module provides helpers that ensure OsRng is used consistently.

use rand::rngs::OsRng;
use rand::RngCore;

/// Generates cryptographically secure random bytes.
///
/// Uses the operating system's secure random number generator (OsRng).
/// This is suitable for:
/// - Cryptographic key generation
/// - Nonce generation
/// - Salt generation
/// - Any security-sensitive randomness
///
/// # Example
/// ```
/// use bitquan_types::entropy::secure_bytes;
///
/// let salt = secure_bytes(32);
/// assert_eq!(salt.len(), 32);
/// ```
pub fn secure_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    OsRng.fill_bytes(&mut buf);
    buf
}

/// Fills an existing buffer with cryptographically secure random bytes.
///
/// # Example
/// ```
/// use bitquan_types::entropy::fill_secure;
///
/// let mut buffer = [0u8; 32];
/// fill_secure(&mut buffer);
/// assert!(buffer.iter().any(|&b| b != 0)); // Should not be all zeros
/// ```
pub fn fill_secure(buf: &mut [u8]) {
    OsRng.fill_bytes(buf);
}

/// Generates a secure random u64.
///
/// Useful for nonces, tie-breakers, and other numeric randomness needs.
///
/// # Example
/// ```
/// use bitquan_types::entropy::secure_u64;
///
/// let random_value = secure_u64();
/// assert!(random_value != 0); // Overwhelmingly likely
/// ```
pub fn secure_u64() -> u64 {
    OsRng.next_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_bytes_correct_length() {
        let bytes = secure_bytes(32);
        assert_eq!(bytes.len(), 32);

        let bytes2 = secure_bytes(64);
        assert_eq!(bytes2.len(), 64);
    }

    #[test]
    fn test_secure_bytes_different_outputs() {
        let a = secure_bytes(32);
        let b = secure_bytes(32);
        assert_ne!(a, b, "Two random outputs should differ");
    }

    #[test]
    fn test_secure_bytes_not_all_zeros() {
        let bytes = secure_bytes(32);
        assert!(
            bytes.iter().any(|&b| b != 0),
            "Random bytes should not be all zeros"
        );
    }

    #[test]
    fn test_fill_secure() {
        let mut buf = [0u8; 32];
        fill_secure(&mut buf);
        assert!(
            buf.iter().any(|&b| b != 0),
            "Buffer should be filled with random data"
        );
    }

    #[test]
    fn test_fill_secure_different_calls() {
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        
        fill_secure(&mut buf1);
        fill_secure(&mut buf2);
        
        assert_ne!(buf1, buf2, "Two fills should produce different data");
    }

    #[test]
    fn test_secure_u64_different_values() {
        let v1 = secure_u64();
        let v2 = secure_u64();
        assert_ne!(v1, v2, "Two random u64s should differ");
    }

    #[test]
    fn test_secure_u64_nonzero() {
        let v = secure_u64();
        assert_ne!(v, 0, "Random u64 should be nonzero (overwhelmingly likely)");
    }
}
