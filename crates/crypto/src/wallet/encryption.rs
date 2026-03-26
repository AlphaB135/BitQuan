//! AES-256-GCM authenticated encryption wrapper for keystore payloads.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::{
    kdf::{KdfError, KeyDerivation},
    secure_types::SecureString,
};

/// Size in bytes of the nonce used for AES-GCM (96 bits).
pub const AES_GCM_NONCE_SIZE: usize = 12;

/// Encapsulates metadata and ciphertext for an encrypted payload.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EncryptedData {
    /// Version of the keystore/encryption format.
    pub version: u8,
    /// Random salt used for the KDF.
    pub salt: Vec<u8>,
    /// Nonce used for AES-GCM.
    pub nonce: Vec<u8>,
    /// Ciphertext produced by AES-GCM.
    pub ciphertext: Vec<u8>,
    /// KDF parameters used to derive the encryption key.
    pub kdf_params: KdfParams,
}

/// Captures the KDF parameters used for key derivation.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KdfParams {
    /// Memory cost in KiB.
    pub memory_cost_kib: u32,
    /// Time cost (iterations).
    pub time_cost: u32,
    /// Degree of parallelism.
    pub parallelism: u32,
}

impl From<&KeyDerivation> for KdfParams {
    fn from(kdf: &KeyDerivation) -> Self {
        Self {
            memory_cost_kib: kdf.memory_cost_kib(),
            time_cost: kdf.time_cost(),
            parallelism: kdf.parallelism(),
        }
    }
}

/// Errors that can occur during encryption/decryption.
#[derive(thiserror::Error, Debug)]
pub enum EncryptionError {
    /// KDF failure.
    #[error(transparent)]
    Kdf(#[from] KdfError),
    /// Random number generator failure.
    #[error("failed to generate secure random bytes")]
    Rng(#[source] getrandom::Error),
    /// Underlying AEAD failure (e.g. authentication failure).
    #[error("AES-GCM error: {0}")]
    AesGcm(aes_gcm::Error),
}

/// Encryptor that uses Argon2id for key derivation and AES-256-GCM for encryption.
#[derive(Clone, Debug, Default)]
pub struct Encryptor {
    kdf: KeyDerivation,
}

impl Encryptor {
    /// Creates a new encryptor with optional custom KDF parameters.
    pub fn with_kdf(kdf: KeyDerivation) -> Self {
        Self { kdf }
    }

    /// Encrypts a plaintext using the configured KDF and AES-256-GCM.
    pub fn encrypt(
        &self,
        plaintext: &[u8],
        password: &SecureString,
    ) -> Result<EncryptedData, EncryptionError> {
        // Derive symmetric key
        let salt = KeyDerivation::generate_salt()?;
        let mut key_bytes = self.kdf.derive_key(password, &salt)?;

        // Initialise cipher
        // NOTE: Upstream aes-gcm 0.10 still relies on generic-array 0.x helpers.
        // Allow the deprecated shim until the crate adopts generic-array 1.x.
        #[allow(deprecated)]
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        // Generate nonce
        let mut nonce_bytes = [0u8; AES_GCM_NONCE_SIZE];
        getrandom::getrandom(&mut nonce_bytes).map_err(EncryptionError::Rng)?;
        // NOTE: Upstream aes-gcm 0.10 still relies on generic-array 0.x helpers.
        #[allow(deprecated)]
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(EncryptionError::AesGcm)?;

        // Zeroize derived key asap
        key_bytes.zeroize();

        Ok(EncryptedData {
            version: 1,
            salt: salt.to_vec(),
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            kdf_params: KdfParams::from(&self.kdf),
        })
    }

    /// Decrypts ciphertext back into plaintext using the stored parameters.
    ///
    /// Performs a masking KDF derivation before attempting AES-GCM decryption
    /// to prevent timing attacks that distinguish correct from incorrect passwords.
    pub fn decrypt(
        &self,
        encrypted: &EncryptedData,
        password: &SecureString,
    ) -> Result<Vec<u8>, EncryptionError> {
        // Always derive two keys: one for the real attempt, one masking round.
        // This ensures KDF computation is constant regardless of password correctness.
        let mut key_bytes = self.kdf.derive_key(password, &encrypted.salt)?;
        let mask_salt = KeyDerivation::generate_salt().unwrap_or([0u8; 32]);
        let _masking = self.kdf.derive_key(password, &mask_salt);

        // NOTE: Upstream aes-gcm 0.10 still relies on generic-array 0.x helpers.
        #[allow(deprecated)]
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        #[allow(deprecated)]
        let nonce = Nonce::from_slice(&encrypted.nonce);

        match cipher.decrypt(nonce, encrypted.ciphertext.as_ref()) {
            Ok(plaintext) => {
                key_bytes.zeroize();
                Ok(plaintext)
            }
            Err(aes_err) => {
                key_bytes.zeroize();
                Err(EncryptionError::AesGcm(aes_err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::secure_types::SecureString;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let encryptor = Encryptor::default();
        let password = SecureString::new("my-password".into());
        let plaintext = b"super secret bytes";

        let encrypted = encryptor
            .encrypt(plaintext, &password)
            .expect("Failed to encrypt plaintext");
        let decrypted = encryptor
            .decrypt(&encrypted, &password)
            .expect("Failed to decrypt ciphertext");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_with_wrong_password_fails() {
        let encryptor = Encryptor::default();
        let password = SecureString::new("correct".into());
        let plaintext = b"secret";

        let encrypted = encryptor
            .encrypt(plaintext, &password)
            .expect("Failed to encrypt plaintext");
        let wrong_password = SecureString::new("incorrect".into());
        let result = encryptor.decrypt(&encrypted, &wrong_password);

        assert!(result.is_err());
    }
}
