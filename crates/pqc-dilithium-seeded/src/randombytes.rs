use rand::rngs::OsRng;
use rand::RngCore;

/// Fills a buffer with cryptographically secure random bytes.
///
/// This function uses the OS's secure random number generator (OsRng)
/// which provides cryptographically secure randomness suitable for
/// cryptographic key generation and other security-sensitive operations.
pub fn randombytes(x: &mut [u8], len: usize) {
  OsRng.fill_bytes(&mut x[..len])
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::rngs::StdRng;
  use rand::SeedableRng;

  /// Deterministic RNG helper for testing purposes only.
  ///
  /// WARNING: This is NOT cryptographically secure and should ONLY be used in tests.
  #[cfg(test)]
  pub fn randombytes_deterministic(x: &mut [u8], len: usize, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    rng.fill_bytes(&mut x[..len]);
  }

  #[test]
  fn test_randombytes_produces_different_output() {
    let mut buf1 = [0u8; 32];
    let mut buf2 = [0u8; 32];

    randombytes(&mut buf1, 32);
    randombytes(&mut buf2, 32);

    // With overwhelming probability, two random 32-byte buffers should differ
    assert_ne!(buf1, buf2, "Two random buffers should be different");
  }

  #[test]
  fn test_randombytes_fills_correct_length() {
    let mut buf = [0u8; 64];
    randombytes(&mut buf, 32);

    // First 32 bytes should be non-zero (with overwhelming probability)
    let first_half_nonzero = buf[..32].iter().any(|&b| b != 0);
    assert!(
      first_half_nonzero,
      "First 32 bytes should contain random data"
    );

    // Last 32 bytes should still be zero
    assert_eq!(
      &buf[32..],
      &[0u8; 32],
      "Last 32 bytes should remain untouched"
    );
  }

  #[test]
  fn test_randombytes_not_all_zero() {
    let mut buf = [0u8; 32];
    randombytes(&mut buf, 32);

    // With overwhelming probability, not all bytes should be zero
    let has_nonzero = buf.iter().any(|&b| b != 0);
    assert!(has_nonzero, "Random buffer should not be all zeros");
  }

  #[test]
  fn test_randombytes_not_all_same() {
    let mut buf = [0u8; 32];
    randombytes(&mut buf, 32);

    // With overwhelming probability, not all bytes should be the same
    let first_byte = buf[0];
    let all_same = buf.iter().all(|&b| b == first_byte);
    assert!(
      !all_same,
      "Random buffer should not have all bytes the same"
    );
  }

  #[test]
  fn test_deterministic_helper_same_seed_same_output() {
    let mut buf1 = [0u8; 32];
    let mut buf2 = [0u8; 32];

    randombytes_deterministic(&mut buf1, 32, 12345);
    randombytes_deterministic(&mut buf2, 32, 12345);

    assert_eq!(buf1, buf2, "Same seed should produce same output");
  }

  #[test]
  fn test_deterministic_helper_different_seed_different_output() {
    let mut buf1 = [0u8; 32];
    let mut buf2 = [0u8; 32];

    randombytes_deterministic(&mut buf1, 32, 12345);
    randombytes_deterministic(&mut buf2, 32, 54321);

    assert_ne!(
      buf1, buf2,
      "Different seeds should produce different output"
    );
  }
}
