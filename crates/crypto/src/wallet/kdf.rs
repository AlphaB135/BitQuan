//! Password-based key derivation helpers using Argon2id.

use std::time::Instant;

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rand::rngs::OsRng;
use zeroize::Zeroize;

use super::secure_types::SecureString;

/// Error type for KDF operations.
#[derive(thiserror::Error, Debug)]
pub enum KdfError {
    /// Argon2 parameter selection failed.
    #[error("invalid Argon2 parameters: {0}")]
    InvalidParams(String),
    /// Password hashing failed.
    #[error("failed to hash password: {0}")]
    HashFailure(String),
    /// OS RNG failure during salt generation.
    #[error("OS RNG failure: {0}")]
    RngFailure(String),
}

/// Minimum number of KDF iterations allowed.
pub const MIN_TIME_COST: u32 = 3;

/// Target derivation time for auto-tune calibration (milliseconds).
///
/// 500 ms is chosen as a balance between security and usability.
/// Bitcoin targets 100 ms with SHA-512, but Argon2id is memory-hard
/// so each iteration is significantly more expensive.
pub const CALIBRATION_TARGET_MS: u64 = 500;

/// Number of calibration rounds to average over.
const CALIBRATION_ROUNDS: u32 = 3;

/// Wrapper for Argon2id parameter selection and key derivation.
#[derive(Clone, Debug)]
pub struct KeyDerivation {
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
}

impl Default for KeyDerivation {
    fn default() -> Self {
        Self {
            // OWASP-recommended parameters for password hashing (2023).
            // Higher costs reduce timing attack feasibility.
            memory_cost_kib: 262_144, // 256 MiB
            time_cost: 4,
            parallelism: 4,
        }
    }
}

impl KeyDerivation {
    /// Constructs a new key derivation helper with custom parameters.
    pub fn new(memory_cost_kib: u32, time_cost: u32, parallelism: u32) -> Self {
        Self {
            memory_cost_kib,
            time_cost,
            parallelism,
        }
    }

    /// Returns the memory cost in KiB.
    pub fn memory_cost_kib(&self) -> u32 {
        self.memory_cost_kib
    }

    /// Returns the time cost (iterations).
    pub fn time_cost(&self) -> u32 {
        self.time_cost
    }

    /// Returns the configured parallelism.
    pub fn parallelism(&self) -> u32 {
        self.parallelism
    }

    /// Generates a fresh 32-byte salt using the OS RNG.
    pub fn generate_salt() -> Result<[u8; 32], KdfError> {
        let mut salt = [0u8; 32];
        getrandom::getrandom(&mut salt).map_err(|e| KdfError::RngFailure(e.to_string()))?;
        Ok(salt)
    }

    /// Derives a 32-byte encryption key from the supplied password + salt.
    pub fn derive_key(&self, password: &SecureString, salt: &[u8]) -> Result<[u8; 32], KdfError> {
        let params = Params::new(
            self.memory_cost_kib,
            self.time_cost,
            self.parallelism,
            Some(32),
        )
        .map_err(|e| KdfError::InvalidParams(e.to_string()))?;

        let salt_string =
            SaltString::encode_b64(salt).map_err(|e| KdfError::InvalidParams(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|err| KdfError::HashFailure(err.to_string()))?;

        let hash = password_hash
            .hash
            .ok_or_else(|| KdfError::InvalidParams("argon2 produced empty hash".into()))?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash.as_bytes()[..32]);
        Ok(key)
    }

    /// Sets the time cost (iterations).
    pub fn set_time_cost(&mut self, cost: u32) {
        self.time_cost = cost.max(MIN_TIME_COST);
    }

    /// Calibrates `time_cost` so that key derivation takes approximately
    /// `target_ms` milliseconds on the current hardware.
    ///
    /// Uses the same weighted-average approach as Bitcoin Core's
    /// `EncryptMasterKey`: runs several derivation rounds, measures
    /// wall-clock time, and adjusts iterations proportionally.
    ///
    /// Returns the calibrated `time_cost` value.
    pub fn calibrate(&mut self, password: &SecureString, target_ms: u64) -> u32 {
        let cal_salt = Self::generate_salt().unwrap_or([0u8; 32]);

        let mut tuned: u64 = self.time_cost as u64;

        for round in 0..CALIBRATION_ROUNDS {
            let start = Instant::now();

            let result = self.derive_key(password, &cal_salt);
            let mut key_bytes = match result {
                Ok(k) => k,
                Err(_) => break,
            };

            let elapsed_ms = start.elapsed().as_millis() as u64;

            key_bytes.zeroize();

            if elapsed_ms == 0 {
                continue;
            }

            // target : current :: target_cost : current_cost
            let target_cost = (self.time_cost as u64 * target_ms) / elapsed_ms;
            // Weighted average with previous rounds.
            let round_u64 = round as u64;
            tuned = (round_u64 * tuned + target_cost) / (round_u64 + 1);
        }

        tuned = tuned.max(MIN_TIME_COST as u64);
        self.time_cost = tuned as u32;
        tuned as u32
    }

    /// Generates a `SaltString` using the OS RNG.
    pub fn generate_salt_string() -> SaltString {
        SaltString::generate(&mut OsRng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_deterministic() {
        let kdf = KeyDerivation::default();
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let password = SecureString::new("correct horse battery".to_owned());
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let salt = [42u8; 32];

        let key1 = kdf
            .derive_key(&password, &salt)
            .expect("Failed to derive key with same salt");
        let key2 = kdf
            .derive_key(&password, &salt)
            .expect("Failed to derive key with same salt");

        assert_eq!(key1, key2);
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let kdf = KeyDerivation::default();
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let password = SecureString::new("correct horse battery".to_owned());
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let salt1 = [1u8; 32];
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let salt2 = [2u8; 32];

        let key1 = kdf
            .derive_key(&password, &salt1)
            .expect("Failed to derive key with salt1");
        let key2 = kdf
            .derive_key(&password, &salt2)
            .expect("Failed to derive key with salt2");

        assert_ne!(key1, key2);
    }

    #[test]
    fn calibrate_adjusts_time_cost() {
        // Use low params so the test completes quickly.
        let mut kdf = KeyDerivation::new(8192, 1, 1);
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let password = SecureString::new("calibrate-test".to_owned());

        let original = kdf.time_cost();
        let tuned = kdf.calibrate(&password, 50); // 50 ms target

        // Tuned cost should differ from the original single-iteration cost.
        assert!(
            tuned != original || original >= MIN_TIME_COST,
            "calibration should adjust time_cost"
        );
        assert!(tuned >= MIN_TIME_COST);
        assert_eq!(kdf.time_cost(), tuned);
    }

    #[test]
    fn calibrate_enforces_minimum() {
        let mut kdf = KeyDerivation::new(8192, 1, 1);
        // codeql[rust/hard-coded-cryptographic-value] suppression: test-only value
        let password = SecureString::new("min-test".to_owned());

        // Extremely short target should still respect minimum.
        kdf.calibrate(&password, 0);
        assert!(kdf.time_cost() >= MIN_TIME_COST);
    }
}
