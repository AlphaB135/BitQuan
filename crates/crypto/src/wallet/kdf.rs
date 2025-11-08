//! Password-based key derivation helpers using Argon2id.

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rand::rngs::OsRng;

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
            memory_cost_kib: 65_536, // 64 MiB
            time_cost: 3,
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
        getrandom::getrandom(&mut salt)
            .map_err(|e| KdfError::RngFailure(e.to_string()))?;
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
        let password = SecureString::new("correct horse battery".to_owned());
        let salt = [42u8; 32];

        let key1 = kdf.derive_key(&password, &salt).unwrap();
        let key2 = kdf.derive_key(&password, &salt).unwrap();

        assert_eq!(key1, key2);
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let kdf = KeyDerivation::default();
        let password = SecureString::new("correct horse battery".to_owned());
        let salt1 = [1u8; 32];
        let salt2 = [2u8; 32];

        let key1 = kdf.derive_key(&password, &salt1).unwrap();
        let key2 = kdf.derive_key(&password, &salt2).unwrap();

        assert_ne!(key1, key2);
    }
}
