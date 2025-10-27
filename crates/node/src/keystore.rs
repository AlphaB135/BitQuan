//! Encrypted keystore for secure wallet key storage.
//!
//! Uses Argon2id for password-based key derivation and ChaCha20-Poly1305
//! for authenticated encryption.

use anyhow::{bail, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use zeroize::Zeroize;

/// Keystore file format version.
const KEYSTORE_VERSION: u32 = 1;

/// Nonce size for ChaCha20-Poly1305 (96 bits).
const NONCE_SIZE: usize = 12;

/// Encrypted keystore file format.
#[derive(Serialize, Deserialize)]
pub struct KeystoreFile {
    /// Format version.
    pub version: u32,

    /// Argon2 salt (base64).
    pub salt: String,

    /// ChaCha20-Poly1305 nonce (base64).
    pub nonce: String,

    /// Encrypted keypair data (base64).
    pub ciphertext: String,

    /// Argon2 parameters.
    pub crypto: CryptoParams,
}

/// Cryptographic parameters for the keystore.
#[derive(Serialize, Deserialize)]
pub struct CryptoParams {
    /// Algorithm identifier.
    pub cipher: String,

    /// Key derivation function.
    pub kdf: String,

    /// Argon2 memory cost (KB).
    pub mem_cost: u32,

    /// Argon2 time cost (iterations).
    pub time_cost: u32,
}

impl Default for CryptoParams {
    fn default() -> Self {
        CryptoParams {
            cipher: "chacha20poly1305".to_string(),
            kdf: "argon2id".to_string(),
            mem_cost: 19456, // 19 MB
            time_cost: 2,
        }
    }
}

/// Encrypts keypair data with a password.
pub fn encrypt_keypair(keypair_json: &str, password: &str) -> Result<KeystoreFile> {
    // Generate random salt for Argon2
    let salt = SaltString::generate(&mut OsRng);

    // Derive encryption key from password using Argon2id
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    // Extract the hash bytes as the encryption key (32 bytes)
    let hash = password_hash
        .hash
        .ok_or_else(|| anyhow::anyhow!("No hash generated"))?;
    let key_bytes = hash.as_bytes();

    if key_bytes.len() < 32 {
        bail!("Derived key too short");
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes[..32]);

    // Create cipher
    let cipher = ChaCha20Poly1305::new(&key.into());

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    getrandom::getrandom(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the keypair data
    let ciphertext = cipher
        .encrypt(nonce, keypair_json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Zeroize sensitive data
    key.zeroize();

    Ok(KeystoreFile {
        version: KEYSTORE_VERSION,
        salt: salt.to_string(),
        nonce: base64::encode(&nonce_bytes),
        ciphertext: base64::encode(&ciphertext),
        crypto: CryptoParams::default(),
    })
}

/// Decrypts keypair data with a password.
pub fn decrypt_keypair(keystore: &KeystoreFile, password: &str) -> Result<String> {
    // Verify version
    if keystore.version != KEYSTORE_VERSION {
        bail!("Unsupported keystore version: {}", keystore.version);
    }

    // Parse salt
    let salt =
        SaltString::from_b64(&keystore.salt).map_err(|e| anyhow::anyhow!("Invalid salt: {}", e))?;

    // Derive encryption key from password
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let hash = password_hash
        .hash
        .ok_or_else(|| anyhow::anyhow!("No hash generated"))?;
    let key_bytes = hash.as_bytes();

    if key_bytes.len() < 32 {
        bail!("Derived key too short");
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes[..32]);

    // Create cipher
    let cipher = ChaCha20Poly1305::new(&key.into());

    // Decode nonce and ciphertext
    let nonce_bytes = base64::decode(&keystore.nonce)?;
    let ciphertext = base64::decode(&keystore.ciphertext)?;

    if nonce_bytes.len() != NONCE_SIZE {
        bail!("Invalid nonce size");
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed - wrong password or corrupted file"))?;

    // Zeroize sensitive data
    key.zeroize();

    String::from_utf8(plaintext)
        .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in decrypted data: {}", e))
}

/// Saves an encrypted keystore to a file.
pub fn save_keystore(keystore: &KeystoreFile, path: &Path) -> Result<()> {
    // Serialize to JSON
    let json = serde_json::to_string_pretty(keystore)?;

    // Write to temporary file first
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, json)?;

    // Set restrictive permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o600); // rw------- (owner only)
        fs::set_permissions(&temp_path, perms)?;
    }

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Loads an encrypted keystore from a file.
pub fn load_keystore(path: &Path) -> Result<KeystoreFile> {
    let json = fs::read_to_string(path)?;
    let keystore: KeystoreFile = serde_json::from_str(&json)?;
    Ok(keystore)
}

// Add base64 encoding helpers
mod base64 {
    use anyhow::Result;

    pub fn encode(data: &[u8]) -> String {
        use base64ct::{Base64, Encoding};
        Base64::encode_string(data)
    }

    pub fn decode(s: &str) -> Result<Vec<u8>> {
        use base64ct::{Base64, Encoding};
        Base64::decode_vec(s).map_err(|e| anyhow::anyhow!("Base64 decode error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original = r#"{"test": "data", "secret": "key"}"#;
        let password = "my-secure-password-123";

        // Encrypt
        let keystore = encrypt_keypair(original, password).unwrap();

        // Verify fields are present
        assert_eq!(keystore.version, KEYSTORE_VERSION);
        assert!(!keystore.salt.is_empty());
        assert!(!keystore.nonce.is_empty());
        assert!(!keystore.ciphertext.is_empty());

        // Decrypt
        let decrypted = decrypt_keypair(&keystore, password).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_wrong_password_fails() {
        let original = r#"{"secret": "data"}"#;
        let password = "correct-password";

        let keystore = encrypt_keypair(original, password).unwrap();

        // Try with wrong password
        let result = decrypt_keypair(&keystore, "wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_load_keystore() {
        use std::env;

        let original = r#"{"test": "keystore"}"#;
        let password = "test-password";

        let keystore = encrypt_keypair(original, password).unwrap();

        // Save to temp file
        let temp_dir = env::temp_dir();
        let path = temp_dir.join("test_keystore.json");

        save_keystore(&keystore, &path).unwrap();

        // Load and verify
        let loaded = load_keystore(&path).unwrap();
        let decrypted = decrypt_keypair(&loaded, password).unwrap();
        assert_eq!(decrypted, original);

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_different_passwords_produce_different_ciphertexts() {
        let original = "secret data";

        let ks1 = encrypt_keypair(original, "password1").unwrap();
        let ks2 = encrypt_keypair(original, "password2").unwrap();

        // Different salts and ciphertexts
        assert_ne!(ks1.salt, ks2.salt);
        assert_ne!(ks1.ciphertext, ks2.ciphertext);
    }
}
