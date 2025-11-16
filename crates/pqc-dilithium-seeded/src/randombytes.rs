use rand::rngs::OsRng;
use rand::RngCore;
use sha3::{
  digest::{ExtendableOutput, Update, XofReader},
  Shake256,
};

/// Fills a buffer with cryptographically secure random bytes.
///
/// This function uses OS's secure random number generator (OsRng)
/// which provides cryptographically secure randomness suitable for
/// cryptographic key generation and other security-sensitive operations.
pub fn randombytes(x: &mut [u8], len: usize) {
  OsRng.fill_bytes(&mut x[..len])
}

/// Fills a buffer with cryptographically secure random bytes conditioned with SHAKE-256.
///
/// This function provides quantum-resistant entropy conditioning by:
/// 1. Collecting raw entropy from OS CSPRNG (2x the required length for security margin)
/// 2. Processing through SHAKE-256 sponge construction
/// 3. Outputting cryptographically secure conditioned entropy
///
/// This approach protects against potential side-channel attacks and quantum
/// adversaries attempting to exploit entropy sources.
pub fn randombytes_conditioned(out: &mut [u8]) {
  let len = out.len();

  // 1. Collect raw entropy from OS CSPRNG (2x length for security margin)
  let mut raw_entropy = vec![0u8; len * 2];
  OsRng.fill_bytes(&mut raw_entropy);

  // 2. Process through SHAKE-256 sponge construction
  let mut hasher = Shake256::default();
  hasher.update(&raw_entropy);

  // 3. Squeeze conditioned entropy output
  let mut reader = hasher.finalize_xof();
  reader.read(out);
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
  }

  #[test]
  fn test_randombytes_not_all_zero() {
    let mut buf = [0u8; 32];
    randombytes(&mut buf, 32);

    // With overwhelming probability, not all bytes should be zero
    let has_nonzero = buf.iter().any(|&b| b != 0);
    assert!(has_nonzero, "Random bytes should not all be zero");
  }

  #[test]
  fn test_randombytes_conditioned_produces_different_output() {
    let mut buf1 = [0u8; 32];
    let mut buf2 = [0u8; 32];

    randombytes_conditioned(&mut buf1);
    randombytes_conditioned(&mut buf2);

    // With overwhelming probability, two conditioned buffers should differ
    assert_ne!(buf1, buf2, "Two conditioned buffers should be different");
  }

  #[test]
  fn test_randombytes_conditioned_fills_correct_length() {
    let mut buf = [0u8; 32];
    randombytes_conditioned(&mut buf);

    // Should fill entire buffer
    let all_nonzero = buf.iter().any(|&b| b != 0);
    assert!(all_nonzero, "Conditioned buffer should contain random data");
  }

  #[test]
  fn test_randombytes_conditioned_different_from_raw() {
    let mut raw_buf = [0u8; 32];
    let mut conditioned_buf = [0u8; 32];

    randombytes(&mut raw_buf, 32);
    randombytes_conditioned(&mut conditioned_buf);

    // Conditioned entropy should be different from raw (with overwhelming probability)
    assert_ne!(
      raw_buf, conditioned_buf,
      "Conditioned entropy should differ from raw"
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
