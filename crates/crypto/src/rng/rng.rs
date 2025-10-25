//! RNG façade ensuring reproducible derivations with strong entropy guarantees.

use hex::FromHexError;
#[cfg(not(feature = "deterministic_tests"))]
use rand::rngs::OsRng;
use rand::{Error as RandError, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use thiserror::Error;
use zeroize::Zeroize;

use super::hkdf;

/// Errors that can arise while instantiating or using the RNG service.
#[derive(Debug, Error)]
pub enum RngError {
    /// The operating system could not source cryptographically secure entropy.
    #[error("failed to obtain entropy from OS CSPRNG: {0}")]
    EntropyUnavailable(#[from] RandError),
    /// Environment variable `BQ_TEST_SEED` is required when deterministic testing is enabled.
    #[error("BQ_TEST_SEED environment variable is required for deterministic RNG tests")]
    MissingTestSeed,
    /// The provided deterministic seed has an invalid hex encoding.
    #[error("BQ_TEST_SEED must be a 64-character hex string: {0}")]
    InvalidTestSeedHex(#[from] FromHexError),
    /// The provided deterministic seed is not the expected length.
    #[error("BQ_TEST_SEED must decode to exactly 32 bytes, found {0}")]
    InvalidTestSeedLength(usize),
}

/// Holds secret key material and ensures it is wiped on drop.
struct KeyMaterial {
    bytes: [u8; 32],
}

impl KeyMaterial {
    /// Constructs a new key container.
    fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Borrows the key bytes for derivation.
    fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl Drop for KeyMaterial {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

/// Stateful deterministic random byte generator backed by ChaCha20.
pub struct RngService {
    key: KeyMaterial,
    rng: ChaCha20Rng,
}

impl RngService {
    /// Creates a new RNG stream from the supplied 32-byte seed.
    pub(crate) fn from_seed(seed: [u8; 32]) -> Self {
        let key = KeyMaterial::new(seed);
        let rng = ChaCha20Rng::from_seed(*key.as_bytes());
        Self { key, rng }
    }

    /// Returns `n` cryptographically secure random bytes.
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut buffer = vec![0_u8; n];
        self.fill(&mut buffer);
        buffer
    }

    /// Fills the provided slice with random bytes.
    pub fn fill(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }

    /// Draws a uniformly random `u64`.
    pub fn u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// Derives a new RNG stream using HKDF-SHA256 domain separation.
    pub fn derive_stream(&self, label: &str) -> Self {
        let seed = hkdf::derive_seed(self.key.as_bytes(), label);
        Self::from_seed(seed)
    }
}

/// Primary random source seeded from the operating system.
pub struct RandomSource {
    inner: RngService,
}

impl RandomSource {
    /// Creates a new random source, seeding from `OsRng` (or deterministic env seed in tests).
    pub fn new() -> Result<Self, RngError> {
        let seed = master_seed()?;
        Ok(Self {
            inner: RngService::from_seed(seed),
        })
    }

    /// Returns `n` random bytes from the master stream.
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        self.inner.bytes(n)
    }

    /// Fills the provided slice with bytes from the master stream.
    pub fn fill(&mut self, dest: &mut [u8]) {
        self.inner.fill(dest);
    }

    /// Draws a uniformly random `u64` from the master stream.
    pub fn u64(&mut self) -> u64 {
        self.inner.u64()
    }

    /// Derives a new labeled stream using HKDF-SHA256 domain separation.
    pub fn derive_stream(&self, label: &str) -> RngService {
        self.inner.derive_stream(label)
    }
}

#[cfg(feature = "deterministic_tests")]
fn master_seed() -> Result<[u8; 32], RngError> {
    use std::env;

    let value = env::var("BQ_TEST_SEED").map_err(|_| RngError::MissingTestSeed)?;
    let value = value.trim();
    let bytes = hex::decode(value)?;
    if bytes.len() != 32 {
        return Err(RngError::InvalidTestSeedLength(bytes.len()));
    }

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(seed)
}

#[cfg(not(feature = "deterministic_tests"))]
fn master_seed() -> Result<[u8; 32], RngError> {
    let mut seed = [0u8; 32];
    OsRng.try_fill_bytes(&mut seed)?;
    Ok(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn test_source() -> RandomSource {
        #[cfg(feature = "deterministic_tests")]
        {
            const DEFAULT_TEST_SEED: &str =
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
            if std::env::var("BQ_TEST_SEED").is_err() {
                std::env::set_var("BQ_TEST_SEED", DEFAULT_TEST_SEED);
            }
        }

        RandomSource::new().expect("entropy available")
    }

    #[test]
    fn random_bytes_not_all_zero() {
        let mut source = test_source();
        let sample = source.bytes(64);
        assert!(sample.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn derived_streams_are_domain_separated() {
        let source = test_source();
        let mut wallet = source.derive_stream("wallet-seed");
        let mut tx = source.derive_stream("tx-sig");

        let wallet_bytes = wallet.bytes(32);
        let tx_bytes = tx.bytes(32);
        assert_ne!(wallet_bytes, tx_bytes);
    }

    #[test]
    fn no_collisions_in_small_sample() {
        let mut source = test_source();
        let mut set: HashSet<[u8; 16]> = HashSet::with_capacity(10_000);

        for _ in 0..10_000 {
            let mut block = [0u8; 16];
            source.fill(&mut block);
            assert!(set.insert(block), "collision detected in 10k sample");
        }
    }

    proptest! {
        #[test]
        fn byte_length_matches_request(len in 1usize..1024) {
            let mut source = test_source();
            let data = source.bytes(len);
            prop_assert_eq!(data.len(), len);
        }
    }

    #[cfg(feature = "deterministic_tests")]
    #[test]
    fn deterministic_feature_enforces_reproducibility() {
        std::env::set_var(
            "BQ_TEST_SEED",
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        );
        let mut first = test_source();
        std::env::set_var(
            "BQ_TEST_SEED",
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        );
        let mut second = test_source();

        let sample_one = first.bytes(128);
        let sample_two = second.bytes(128);
        assert_eq!(sample_one, sample_two);
    }
}
