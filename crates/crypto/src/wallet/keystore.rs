//! High-level keystore management (encrypt/decrypt/save/load).

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::{
    encryption::{EncryptedData, EncryptionError, Encryptor},
    secure_types::{SecurePrivateKey, SecureString},
};

/// Error type for keystore operations.
#[derive(thiserror::Error, Debug)]
pub enum KeystoreError {
    /// Encryption-related failure.
    #[error(transparent)]
    Encryption(#[from] super::encryption::EncryptionError),
    /// Serialization/deserialization failure.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    /// I/O failure.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// Password mismatch / authentication failure.
    #[error("invalid password or corrupted keystore")]
    InvalidPassword,
}

/// On-disk keystore representation.
#[derive(Clone, Serialize, Deserialize)]
pub struct Keystore {
    /// Format version.
    pub version: u8,
    /// User-facing address.
    pub address: String,
    /// Encrypted private-key payload.
    pub encrypted_private_key: EncryptedData,
    /// Unix timestamp (seconds) when created.
    pub created_at: i64,
}

impl Keystore {
    /// Constructs a new keystore from a plaintext private key and password.
    pub fn new(
        private_key: &SecurePrivateKey,
        password: &SecureString,
        address: String,
    ) -> Result<Self, KeystoreError> {
        let encryptor = Encryptor::default();
        let encrypted_private_key = encryptor.encrypt(private_key.as_slice(), password)?;

        Ok(Self {
            version: 1,
            address,
            encrypted_private_key,
            created_at: Utc::now().timestamp(),
        })
    }

    /// Decrypts the private key using the supplied password.
    pub fn unlock(&self, password: &SecureString) -> Result<SecurePrivateKey, KeystoreError> {
        let encryptor = Encryptor::default();
        let plaintext = encryptor
            .decrypt(&self.encrypted_private_key, password)
            .map_err(|err| match err {
                EncryptionError::AesGcm(_) => KeystoreError::InvalidPassword,
                other => KeystoreError::Encryption(other),
            })?;

        let mut key_bytes = plaintext;
        if key_bytes.is_empty() {
            return Err(KeystoreError::InvalidPassword);
        }

        let secure = SecurePrivateKey::new(key_bytes.clone());
        key_bytes.zeroize();
        Ok(secure)
    }

    /// Serialises and writes the keystore to disk atomically.
    pub fn save_to_file(&self, path: &Path) -> Result<(), KeystoreError> {
        let json = serde_json::to_string_pretty(self)?;

        let temp = path.with_extension("tmp");
        std::fs::write(&temp, json)?;

        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&temp, perms)?;
        }

        std::fs::rename(temp, path)?;
        Ok(())
    }

    /// Loads a keystore from disk.
    pub fn load_from_file(path: &Path) -> Result<Self, KeystoreError> {
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn keystore_roundtrip() {
        let temp = tempdir().expect("Failed to create temporary directory");
        let path = temp.path().join("wallet.keystore");

        let private = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let password = SecureString::new("s3cr3t".into());
        let keystore = Keystore::new(&private, &password, "bq1testaddr".into()).expect("Failed to create keystore");
        keystore.save_to_file(&path).expect("Failed to save keystore");

        let loaded = Keystore::load_from_file(&path).expect("Failed to load keystore");
        assert_eq!(loaded.address, "bq1testaddr");

        let unlocked = loaded.unlock(&password).expect("Failed to unlock keystore");
        assert_eq!(unlocked.as_slice(), private.as_slice());
    }

    #[test]
    fn wrong_password_fails() {
        let private = SecurePrivateKey::new(vec![9, 9, 9]);
        let password = SecureString::new("goodpass".into());
        let keystore = Keystore::new(&private, &password, "addr".into()).expect("Failed to create keystore");

        let wrong = SecureString::new("badpass".into());
        let err = keystore.unlock(&wrong).unwrap_err();
        assert!(matches!(err, KeystoreError::InvalidPassword));
    }
}
