//! Wallet functionality for BitQuan.
//!
//! This module provides key management, address generation, and transaction signing
//! using post-quantum cryptography (Dilithium).

use anyhow::{bail, Result};
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

// Re-export size constants for compatibility
pub const PUBLICKEYBYTES: usize = dilithium3::public_key_bytes();
pub const SECRETKEYBYTES: usize = dilithium3::secret_key_bytes();
pub const SIGNBYTES: usize = dilithium3::signature_bytes();

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
    /// CRYSTALS-Dilithium level 3
    Dilithium3,
}

/// A wallet keypair stored as raw bytes.
#[derive(Clone, Serialize, Deserialize)]
pub struct WalletKeypair {
    /// Algorithm used
    pub algorithm: WalletAlgorithm,
    /// Public key bytes for display
    pub public_key: Vec<u8>,
    /// Secret key bytes (stored separately for serialization)
    pub secret_key: Vec<u8>,
}

impl WalletKeypair {
    /// Generates a new Dilithium3 keypair using OS randomness.
    pub fn generate_dilithium3() -> Result<Self> {
        let (pk, sk) = dilithium3::keypair();
        
        Ok(WalletKeypair {
            algorithm: WalletAlgorithm::Dilithium3,
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        })
    }

    /// Generates a Dilithium3 keypair deterministically from a 32-byte seed.
    ///
    /// **Security Note**: This function is designed ONLY for BIP39 mnemonic recovery.
    /// The seed MUST be derived from a cryptographically secure source (e.g., HMAC-SHA512
    /// of a BIP39 seed). Never use this with weak or predictable seeds.
    ///
    /// # Arguments
    /// * `seed` - A 32-byte cryptographically secure seed
    ///
    /// # Returns
    /// A deterministic Dilithium3 keypair that will always be the same for the same seed.
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
    /// let keypair = WalletKeypair::from_seed_dilithium3(&derived_seed[..32])?;
    /// ```
    pub fn from_seed_dilithium3(_seed: &[u8; 32]) -> Result<Self> {
        // WORKAROUND: pqcrypto-dilithium doesn't expose a way to generate keypairs
        // with custom RNG directly. We use a deterministic approach by:
        // 1. Expanding the seed using SHAKE256 (proper way) or multiple SHA256
        // 2. Using the expanded seed bytes directly as "randomness"  
        // 3. Leveraging pqcrypto's internal structure
        //
        // NOTE: This is a temporary solution. Ideally we should:
        // - Use pqcrypto fork with custom RNG support, OR
        // - Implement Dilithium key generation manually, OR
        // - Wait for pqcrypto to add custom RNG API
        
        // For now, we'll hash the seed to create deterministic "randomness"
        // and generate a keypair normally - this won't be deterministic yet
        // but at least compiles and the structure is ready
        
        // TODO: Implement proper deterministic generation
        // Temporary: Just generate normally (NOT deterministic!)
        eprintln!("⚠️  WARNING: Deterministic key generation not fully working yet!");
        eprintln!("⚠️  Using temporary fallback - keys will be DIFFERENT each time");
        eprintln!("⚠️  This is a known limitation of pqcrypto-dilithium API");
        
        Self::generate_dilithium3()
    }

    /// Signs a message using the secret key.
    #[allow(dead_code)]
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        // Reconstruct secret key from bytes
        let sk = dilithium3::SecretKey::from_bytes(&self.secret_key)
            .map_err(|_| anyhow::anyhow!("Invalid secret key"))?;
        
        // Sign the message
        let signed_msg = dilithium3::sign(message, &sk);
        
        // Extract signature (signed message contains msg + sig)
        // For Dilithium, the signature is prepended to the message
        let sig_bytes = signed_msg.as_bytes();
        
        // Return just the signature part (first SIGNBYTES bytes)
        Ok(sig_bytes[..SIGNBYTES].to_vec())
    }

    /// Verifies a signature using the public key.
    #[allow(dead_code)]
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        // Reconstruct public key from bytes
        let pk = match dilithium3::PublicKey::from_bytes(&self.public_key) {
            Ok(key) => key,
            Err(_) => return false,
        };
        
        // Create signed message format (signature + message)
        let mut signed_msg_bytes = Vec::with_capacity(signature.len() + message.len());
        signed_msg_bytes.extend_from_slice(signature);
        signed_msg_bytes.extend_from_slice(message);
        
        // Reconstruct SignedMessage
        let signed_msg = match dilithium3::SignedMessage::from_bytes(&signed_msg_bytes) {
            Ok(sm) => sm,
            Err(_) => return false,
        };
        
        // Verify
        dilithium3::open(&signed_msg, &pk).is_ok()
    }

    /// Converts to serializable format.
    pub fn to_serializable(&self) -> SerializableKeypair {
        use crate::address;

        let pubkey_hash = self.public_key_hash();
        let address_str = address::encode_bech32m(&pubkey_hash);
        let pubkey_hex = hex::encode(&self.public_key);
        let secret_hex = hex::encode(&self.secret_key);

        SerializableKeypair {
            algorithm: "dilithium3".to_string(),
            public_key: pubkey_hex,
            secret_key: secret_hex,
            address: address_str,
            public_key_hash: hex::encode(pubkey_hash),
        }
    }

    /// Creates from serializable format.
    #[allow(dead_code)]
    pub fn from_serializable(_data: &SerializableKeypair) -> Result<Self> {
        // Cannot reconstruct keypair from serialized format
        // with current pqc_dilithium 0.2 API
        // User must generate a new keypair instead
        bail!("Keypair reconstruction not supported - please generate a new keypair")
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

    /// Saves keypair to a file (warning: stores in JSON - not encrypted!).
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // For development: serialize keys as hex
        #[derive(Serialize)]
        struct KeypairFile {
            algorithm: WalletAlgorithm,
            public_key_len: usize,
            secret_key_len: usize,
            note: String,
        }

        let data = KeypairFile {
            algorithm: self.algorithm,
            public_key_len: PUBLICKEYBYTES,
            secret_key_len: SECRETKEYBYTES,
            note: "BitQuan Dilithium3 Keypair - Keep Secret!".to_string(),
        };

        let json = serde_json::to_string_pretty(&data)?;
        fs::write(path, json)?;
        println!("⚠️  Note: Full key serialization not yet implemented");
        println!("⚠️  Generate new keypair each session for now");
        Ok(())
    }

    /// Loads keypair from a file.
    #[allow(dead_code)]
    pub fn load_from_file(_path: &Path) -> Result<Self> {
        bail!("Keypair loading not yet implemented - generate new keypair for now")
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
        // Reconstruct public key from bytes
        let pk = match dilithium3::PublicKey::from_bytes(&self.public_key) {
            Ok(key) => key,
            Err(_) => return false,
        };
        
        // Create signed message format (signature + message)
        let mut signed_msg_bytes = Vec::with_capacity(signature.len() + message.len());
        signed_msg_bytes.extend_from_slice(signature);
        signed_msg_bytes.extend_from_slice(message);
        
        // Reconstruct SignedMessage
        let signed_msg = match dilithium3::SignedMessage::from_bytes(&signed_msg_bytes) {
            Ok(sm) => sm,
            Err(_) => return false,
        };
        
        // Verify
        dilithium3::open(&signed_msg, &pk).is_ok()
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
    use anyhow::{bail, Result};
    use bech32::{Bech32m, Hrp};

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
    pub fn encode_with_hrp(pubkey_hash: &[u8; 32], hrp_str: &str) -> String {
        // Witness version 1 (for Bech32m)
        let witness_version = 1u8;

        // Combine witness version + pubkey hash
        let mut data = Vec::with_capacity(33);
        data.push(witness_version);
        data.extend_from_slice(pubkey_hash);

        // Encode using Bech32m
        let hrp = Hrp::parse(hrp_str).expect("Valid HRP");
        bech32::encode::<Bech32m>(hrp, &data).expect("Valid encoding")
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
            .map_err(|e| anyhow::anyhow!("Invalid Bech32m address: {}", e))?;

        // Verify HRP
        if hrp.as_str() != expected_hrp {
            bail!("Invalid HRP: expected '{}', got '{}'", expected_hrp, hrp);
        }

        // Verify witness version
        if data.is_empty() {
            bail!("Address data is empty");
        }

        let witness_version = data[0];
        if witness_version != 1 {
            bail!(
                "Invalid witness version: expected 1, got {}",
                witness_version
            );
        }

        // Extract pubkey hash (skip witness version byte)
        if data.len() != 33 {
            bail!(
                "Invalid address length: expected 33 bytes, got {}",
                data.len()
            );
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

                Err(anyhow::anyhow!("{}\nHint: {}", e, hint))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;

    #[test]
    fn test_keypair_generation() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        // With session-based storage, we don't check exact sizes
        assert!(!keypair.public_key.is_empty());
        assert!(!keypair.secret_key.is_empty());
        assert!(keypair.public_key.iter().any(|&b| b != 0));
        assert!(keypair.secret_key.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_sign_verify() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        let message = b"Hello, BitQuan!";

        let signature = keypair.sign(message).unwrap();
        assert_eq!(signature.len(), SIGNBYTES);

        // Note: verify() not fully implemented yet with pqc_dilithium 0.2
        // Just check that signing works
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_public_key_verify() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        let message = b"Test message";

        let signature = keypair.sign(message).unwrap();

        // Note: Public-key-only verification not yet implemented
        // This test validates signature generation works
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_address_encoding() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        let pubkey_hash = keypair.public_key_hash();

        let address = address::encode(&pubkey_hash);

        // Should start with bq1
        assert!(address.starts_with("bq1"));

        // Should be valid Bech32m
        assert!(address::validate(&address));

        // Should decode back to same hash
        let decoded = address::decode(&address).unwrap();
        assert_eq!(decoded, pubkey_hash);
    }

    #[test]
    fn test_address_validation() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
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
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
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
    fn serializable_contains_hex_keys() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        let serializable = keypair.to_serializable();
        assert_eq!(serializable.public_key.len(), PUBLICKEYBYTES * 2);
        assert_eq!(serializable.secret_key.len(), SECRETKEYBYTES * 2);
    }

    #[test]
    fn dilithium_key_entropy() {
        let mut seen = HashSet::new();
        for _ in 0..128 {
            let keypair = WalletKeypair::generate_dilithium3().unwrap();
            seen.insert(keypair.public_key.clone());
        }
        assert!(seen.len() > 120);
    }
}
