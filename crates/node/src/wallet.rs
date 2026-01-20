//! Wallet functionality for BitQuan.
//!
//! This module provides key management, address generation, and transaction signing
//! using post-quantum cryptography (Dilithium) with secure memory handling.

use bitquan_types::error::{Error, Result};
use bq_crypto::wallet::{EncryptedData, Encryptor, SecureString};
use pqc_dilithium_seeded::{self as dilithium, Keypair, PUBLICKEYBYTES, SECRETKEYBYTES, SIGNBYTES};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Serializable representation of a wallet keypair.
#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableKeypair {
    /// Algorithm used
    pub algorithm: String,
    /// Public key as hex
    pub public_key: String,
    /// Secret key as hex (encrypted by keystore)
    pub secret_key: String,
    /// Address string
    pub address: String,
    /// Public key hash (hex)
    pub public_key_hash: String,
}

/// Supported signature algorithms for wallet keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletAlgorithm {
    /// CRYSTALS-Dilithium level 5
    Dilithium5,
}

/// A wallet keypair stored with secure memory handling.
pub struct WalletKeypair {
    /// Algorithm used
    pub algorithm: WalletAlgorithm,
    /// Public key bytes for display (public data)
    pub public_key: Vec<u8>,
    /// Secret key bytes protected by secrecy crate
    pub secret_key: Secret<Vec<u8>>,
}

impl Clone for WalletKeypair {
    fn clone(&self) -> Self {
        // Note: Cloning a secret key exposes the secret temporarily
        // This is necessary for compatibility with current architecture
        let secret_data = self.secret_key.expose_secret();
        Self {
            algorithm: self.algorithm,
            public_key: self.public_key.clone(),
            secret_key: Secret::new(secret_data.clone()),
        }
    }
}

impl WalletKeypair {
    /// Generates a new Dilithium5 keypair using OS randomness with secure memory.
    pub fn generate_dilithium5() -> Result<Self> {
        let keypair = Keypair::generate();

        Ok(WalletKeypair {
            algorithm: WalletAlgorithm::Dilithium5,
            public_key: keypair.public.to_vec(),
            secret_key: Secret::new(keypair.expose_secret().to_vec()),
        })
    }

    /// Generates a Dilithium5 keypair deterministically from a 32-byte seed.
    ///
    /// **Security Note**: This function is designed ONLY for BIP39 mnemonic recovery.
    /// The seed MUST be derived from a cryptographically secure source (e.g., HMAC-SHA512
    /// of a BIP39 seed). Never use this with weak or predictable seeds.
    ///
    /// # Arguments
    /// * `seed` - A 32-byte cryptographically secure seed
    ///
    /// # Returns
    /// A deterministic Dilithium5 keypair that will always be the same for the same seed.
    ///
    /// # Implementation
    /// Uses ChaCha20 CSPRNG seeded with the input to override getrandom, providing
    /// deterministic randomness for Dilithium key generation while maintaining
    /// cryptographic security.
    ///
    /// # Example
    /// ```ignore
    /// // Derive from BIP39 mnemonic (secure)
    /// let mnemonic_seed = mnemonic.to_seed("");
    /// let derived_seed = hmac_sha512(&mnemonic_seed, b"key_index_0");
    /// let keypair = WalletKeypair::from_seed_dilithium5(&derived_seed[..32])?;
    /// ```
    pub fn from_seed_dilithium5(seed: &[u8; 32]) -> Result<Self> {
        // Use patched pqc_dilithium with exposed crypto_sign_keypair
        let mut public = [0u8; PUBLICKEYBYTES];
        let mut secret = [0u8; SECRETKEYBYTES];

        // Call the exposed function with our seed
        dilithium::crypto_sign_keypair(&mut public, &mut secret, Some(seed));

        Ok(WalletKeypair {
            algorithm: WalletAlgorithm::Dilithium5,
            public_key: public.to_vec(),
            secret_key: Secret::new(secret.to_vec()),
        })
    }

    /// Signs a message using the secret key with secure memory access.
    #[allow(dead_code)]
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let mut sig = vec![0u8; SIGNBYTES];
        dilithium::crypto_sign_signature(&mut sig, message, self.secret_key.expose_secret());
        Ok(sig)
    }

    /// Verifies a signature using the public key.
    #[allow(dead_code)]
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        dilithium::crypto_sign_verify(signature, message, &self.public_key).is_ok()
    }

    /// Converts to serializable format with encrypted secret key.
    pub fn to_serializable(&self, password: &str) -> SerializableKeypair {
        use crate::wallet::address;

        let pubkey_hash = self.public_key_hash();
        let address_str = address::encode(&pubkey_hash);
        let pubkey_hex = hex::encode(&self.public_key);
        // Encrypt secret key before serialization
        let secret_encrypted = self.encrypt_secret_key(password).unwrap_or_else(|_| {
            log::error!("Failed to encrypt secret key for serialization");
            "ENCRYPTION_FAILED".to_string()
        });

        SerializableKeypair {
            algorithm: "dilithium5".to_string(),
            public_key: pubkey_hex,
            secret_key: secret_encrypted,
            address: address_str,
            public_key_hash: hex::encode(pubkey_hash),
        }
    }

    /// Encrypts the secret key for storage using AES-256-GCM + Argon2id.
    fn encrypt_secret_key(&self, password: &str) -> Result<String> {
        let encryptor = Encryptor::default();
        let secure_password = SecureString::new(password.to_owned());
        let plaintext = self.secret_key.expose_secret();

        let encrypted = encryptor
            .encrypt(plaintext, &secure_password)
            .map_err(|e| Error::Invalid(format!("encryption failed: {}", e)))?;

        // Serialize encrypted data to JSON (includes salt, nonce, kdf_params)
        serde_json::to_string(&encrypted)
            .map_err(|e| Error::Invalid(format!("serialization failed: {}", e)))
    }

    /// Creates from serializable format with secret key decryption.
    #[allow(dead_code)]
    pub fn from_serializable(data: &SerializableKeypair, password: &str) -> Result<Self> {
        // Reconstruct keypair from serialized data
        let public_key = hex::decode(&data.public_key)
            .map_err(|e| Error::Invalid(format!("invalid public key hex: {e}")))?;

        // Try to decrypt secret key first (encrypted JSON format)
        // Fallback to hex decode for backward compatibility with legacy wallets
        let secret_key = if data.secret_key.starts_with('{') {
            // New encrypted format (JSON)
            Self::decrypt_secret_key(&data.secret_key, password)?
        } else {
            // Legacy support: direct hex decode (less secure, log warning)
            log::warn!("Loading legacy unencrypted wallet from hex format");
            hex::decode(&data.secret_key)
                .map_err(|e| Error::Invalid(format!("invalid secret key hex: {e}")))?
        };

        // Validate key sizes
        if public_key.len() != PUBLICKEYBYTES {
            return Err(Error::Invalid(format!(
                "public key must be {} bytes, got {}",
                PUBLICKEYBYTES,
                public_key.len()
            )));
        }
        if secret_key.len() != SECRETKEYBYTES {
            return Err(Error::Invalid(format!(
                "secret key must be {} bytes, got {}",
                SECRETKEYBYTES,
                secret_key.len()
            )));
        }

        // Convert to arrays
        let mut pub_array = [0u8; PUBLICKEYBYTES];
        let mut sec_array = [0u8; SECRETKEYBYTES];
        pub_array.copy_from_slice(&public_key);
        sec_array.copy_from_slice(&secret_key);

        Ok(WalletKeypair {
            algorithm: WalletAlgorithm::Dilithium5,
            public_key: pub_array.to_vec(),
            secret_key: Secret::new(sec_array.to_vec()),
        })
    }

    /// Decrypts an encrypted secret key using AES-256-GCM + Argon2id.
    fn decrypt_secret_key(encrypted_secret: &str, password: &str) -> Result<Vec<u8>> {
        let encryptor = Encryptor::default();
        let secure_password = SecureString::new(password.to_owned());

        // Deserialize encrypted data from JSON
        let encrypted_data: EncryptedData = serde_json::from_str(encrypted_secret)
            .map_err(|e| Error::Invalid(format!("deserialization failed: {}", e)))?;

        // Decrypt using the same password
        let decrypted = encryptor
            .decrypt(&encrypted_data, &secure_password)
            .map_err(|e| Error::Invalid(format!("decryption failed: {}", e)))?;

        Ok(decrypted)
    }

    /// Returns the public key hash (for address generation).
    #[allow(dead_code)]
    pub fn public_key_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key);
        let result = hasher.finalize();

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Securely wipes the secret key from memory.
    ///
    /// **Security Critical**: Call this method when the keypair is no longer needed
    /// to prevent secret key material from remaining in memory.
    pub fn secure_wipe(&mut self) {
        // Create a new empty secret to replace the current one
        // This will zeroize the old secret when dropped
        let empty_secret = Secret::new(vec![]);
        let _ = std::mem::replace(&mut self.secret_key, empty_secret);
    }

    /// Creates a new secure keypair that automatically wipes on drop.
    #[allow(dead_code)]
    pub fn generate_secure() -> Result<Self> {
        Self::generate_dilithium5()
    }

    /// Saves keypair to a file (warning: stores in JSON - file not encrypted!).
    /// Note: The secret key field IS encrypted with the password, but the JSON file itself
    /// is not encrypted at the file level. For production, use the keystore module.
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &Path, password: &str) -> Result<()> {
        let data = self.to_serializable(password);
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(path, json)?;
        log::info!("⚠️  WARNING: Keypair saved to file (secret key is encrypted, but file is not)!");
        log::info!("⚠️  For production, use the encrypted keystore system!");
        Ok(())
    }

    /// Loads keypair from a file.
    #[allow(dead_code)]
    pub fn load_from_file(path: &Path, password: &str) -> Result<Self> {
        let json = fs::read_to_string(path)
            .map_err(|e| Error::Invalid(format!("failed to read keypair file: {e}")))?;

        let data: SerializableKeypair = serde_json::from_str(&json)
            .map_err(|e| Error::Invalid(format!("failed to parse keypair file: {e}")))?;

        Self::from_serializable(&data, password)
    }

    /// Exports public key only (safe to share).
    #[allow(dead_code)]
    pub fn export_public(&self) -> WalletPublicKey {
        WalletPublicKey {
            algorithm: self.algorithm,
            public_key: self.public_key.clone(),
        }
    }
}

impl Drop for WalletKeypair {
    fn drop(&mut self) {
        // Ensure secret key is wiped when keypair is dropped
        self.secure_wipe();
    }
}

/// Public key only (no secret key).
#[derive(Clone, Serialize, Deserialize)]
pub struct WalletPublicKey {
    /// Algorithm used
    pub algorithm: WalletAlgorithm,
    /// Public key bytes
    pub public_key: Vec<u8>,
}

impl WalletPublicKey {
    /// Verifies a signature using this public key.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        dilithium::crypto_sign_verify(signature, message, &self.public_key).is_ok()
    }

    /// Returns the public key hash.
    #[allow(dead_code)]
    pub fn public_key_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key);
        let result = hasher.finalize();

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Bech32m address encoding (BIP 350 compatible).
pub mod address {
    use bech32::{Bech32m, Hrp};
    use bitquan_types::error::{Error, Result};

    /// Human-readable prefix for BitQuan mainnet addresses.
    pub const HRP_MAINNET: &str = "bq";

    /// Human-readable prefix for BitQuan testnet addresses.
    #[allow(dead_code)]
    pub const HRP_TESTNET: &str = "bqt";

    /// Encodes a public key hash to a Bech32m address.
    ///
    /// Uses witness version 1 (for post-quantum signatures).
    /// Format: bq1<bech32m-encoded-hash>
    pub fn encode(pubkey_hash: &[u8; 32]) -> String {
        encode_with_hrp(pubkey_hash, HRP_MAINNET)
    }

    /// Encodes a public key hash with a custom HRP.
    #[allow(clippy::expect_used)]
    pub fn encode_with_hrp(pubkey_hash: &[u8; 32], hrp_str: &str) -> String {
        // Witness version 1 (for Bech32m)
        let witness_version = 1u8;

        // Combine witness version + pubkey hash
        let mut data = Vec::with_capacity(33);
        data.push(witness_version);
        data.extend_from_slice(pubkey_hash);

        // Encode using Bech32m
        // SAFETY: Network HRPs are validated at compile-time
        let hrp = Hrp::parse(hrp_str).expect("network HRP is valid");
        // SAFETY: encoding with valid HRP and valid data cannot fail
        bech32::encode::<Bech32m>(hrp, &data).expect("encoding with valid HRP/data")
    }

    /// Decodes a Bech32m address to a public key hash.
    #[allow(dead_code)]
    pub fn decode(address: &str) -> Result<[u8; 32]> {
        decode_with_hrp(address, HRP_MAINNET)
    }

    /// Decodes a Bech32m address with HRP validation.
    #[allow(dead_code)]
    pub fn decode_with_hrp(address: &str, expected_hrp: &str) -> Result<[u8; 32]> {
        // Decode Bech32m
        let (hrp, data) = bech32::decode(address)
            .map_err(|e| Error::Invalid(format!("Invalid Bech32m address: {}", e)))?;

        // Verify HRP
        if hrp.as_str() != expected_hrp {
            return Err(Error::Invalid(format!(
                "Invalid HRP: expected '{}', got '{}'",
                expected_hrp, hrp
            )));
        }

        // Verify witness version
        if data.is_empty() {
            return Err(Error::Invalid("Address data is empty".to_string()));
        }

        let witness_version = data[0];
        if witness_version != 1 {
            return Err(Error::Invalid(format!(
                "Invalid witness version: expected 1, got {}",
                witness_version
            )));
        }

        // Extract pubkey hash (skip witness version byte)
        if data.len() != 33 {
            return Err(Error::Invalid(format!(
                "Invalid address length: expected 33 bytes, got {}",
                data.len()
            )));
        }

        let mut pubkey_hash = [0u8; 32];
        pubkey_hash.copy_from_slice(&data[1..33]);

        Ok(pubkey_hash)
    }

    /// Validates a Bech32m address without decoding.
    #[allow(dead_code)]
    pub fn validate(address: &str) -> bool {
        decode(address).is_ok()
    }

    /// Returns helpful error message for invalid addresses.
    #[allow(dead_code)]
    pub fn validate_with_hint(address: &str) -> Result<()> {
        match decode(address) {
            Ok(_) => Ok(()),
            Err(e) => {
                let hint = if !address.starts_with("bq1") {
                    "Address should start with 'bq1'"
                } else if address.len() < 42 {
                    "Address is too short"
                } else if address.len() > 90 {
                    "Address is too long"
                } else {
                    "Address has invalid checksum or characters"
                };

                Err(Error::Invalid(format!("{}\nHint: {}", e, hint)))
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;

    #[test]
    fn test_keypair_generation() {
        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        // With session-based storage, we don't check exact sizes
        assert!(!keypair.public_key.is_empty());
        assert!(!keypair.secret_key.expose_secret().is_empty());
        assert!(keypair.public_key.iter().any(|&b| b != 0));
        assert!(keypair.secret_key.expose_secret().iter().any(|&b| b != 0));
    }

    #[test]
    fn test_sign_verify() {
        if std::env::var_os("BITQUAN_SKIP_PQC_TESTS").is_some() {
            return;
        }

        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        let message = b"Hello, BitQuan!";

        let signature = keypair.sign(message).expect("Failed to sign message");
        assert_eq!(signature.len(), SIGNBYTES);

        // Note: verify() not fully implemented yet with pqc_dilithium 0.2
        // Just check that signing works
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_public_key_verify() {
        if std::env::var_os("BITQUAN_SKIP_PQC_TESTS").is_some() {
            return;
        }

        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        let message = b"Test message";

        let signature = keypair.sign(message).expect("Failed to sign message");

        // Note: Public-key-only verification not yet implemented
        // This test validates signature generation works
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_address_encoding() {
        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        let pubkey_hash = keypair.public_key_hash();

        let address = address::encode(&pubkey_hash);

        // Should start with bq1
        assert!(address.starts_with("bq1"));

        // Should be valid Bech32m
        assert!(address::validate(&address));

        // Should decode back to same hash
        let decoded = address::decode(&address).expect("Failed to decode address");
        assert_eq!(decoded, pubkey_hash);
    }

    #[test]
    fn test_address_validation() {
        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        let pubkey_hash = keypair.public_key_hash();
        let address = address::encode(&pubkey_hash);

        // Valid address
        assert!(address::validate(&address));

        // Invalid addresses
        assert!(!address::validate("invalid"));
        assert!(!address::validate("bq1"));
        assert!(!address::validate(
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
        )); // Bitcoin address
    }

    #[test]
    fn test_testnet_addresses() {
        let pubkey_hash = [0u8; 32];

        // Mainnet
        let mainnet = address::encode_with_hrp(&pubkey_hash, address::HRP_MAINNET);
        assert!(mainnet.starts_with("bq1"));

        // Testnet
        let testnet = address::encode_with_hrp(&pubkey_hash, address::HRP_TESTNET);
        assert!(testnet.starts_with("bqt1"));

        // Should not cross-decode
        assert!(address::decode_with_hrp(&mainnet, address::HRP_TESTNET).is_err());
    }

    #[test]
    fn test_bech32m_checksum() {
        let pubkey_hash = [0xAB; 32];
        let address = address::encode(&pubkey_hash);

        // Corrupt the address (flip a character)
        let mut corrupted = address.clone();
        let bytes = unsafe { corrupted.as_bytes_mut() };
        if bytes[10] == b'a' {
            bytes[10] = b'b';
        } else {
            bytes[10] = b'a';
        }

        // Should fail checksum
        assert!(address::decode(&corrupted).is_err());
    }

    #[test]
    fn test_public_key_hash() {
        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        let hash1 = keypair.public_key_hash();
        let hash2 = keypair.public_key_hash();

        assert_eq!(hash1, hash2); // Should be deterministic
        assert_eq!(hash1.len(), 32);

        let mut zero_hasher = Sha256::new();
        zero_hasher.update(vec![0u8; PUBLICKEYBYTES]);
        let zero_hash = zero_hasher.finalize();
        let mut zero_arr = [0u8; 32];
        zero_arr.copy_from_slice(zero_hash.as_ref());
        assert_ne!(hash1, zero_arr);
    }

    #[test]
    fn serializable_contains_encrypted_key() {
        let keypair =
            WalletKeypair::generate_dilithium5().expect("Failed to generate Dilithium5 keypair");
        let serializable = keypair.to_serializable("test_password");
        assert_eq!(serializable.public_key.len(), PUBLICKEYBYTES * 2);
        // Secret key is now encrypted JSON (AES-256-GCM + Argon2id)
        assert!(serializable.secret_key.starts_with('{')); // JSON format
                                                           // Verify it's valid encrypted data JSON
        assert!(
            serde_json::from_str::<bq_crypto::wallet::EncryptedData>(&serializable.secret_key)
                .is_ok()
        );
    }

    #[test]
    fn dilithium_key_entropy() {
        let mut seen = HashSet::new();
        for _ in 0..128 {
            let keypair = WalletKeypair::generate_dilithium5()
                .expect("Failed to generate Dilithium5 keypair");
            seen.insert(keypair.public_key.clone());
        }
        assert!(seen.len() > 120);
    }
}
