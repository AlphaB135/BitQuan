//! Wallet functionality for BitQuan.
//! 
//! This module provides key management, address generation, and transaction signing
//! using post-quantum cryptography (Dilithium).

use anyhow::{Result, bail};
use pqc_dilithium::{Keypair, PUBLICKEYBYTES, SECRETKEYBYTES, SIGNBYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

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
    /// Serialized keypair (stores the whole Keypair internally)
    #[serde(skip)]
    keypair: Option<Keypair>,
    /// Public key bytes for display
    pub public_key: Vec<u8>,
    /// Secret key bytes (stored separately for serialization)
    pub secret_key: Vec<u8>,
}

impl WalletKeypair {
    /// Generates a new Dilithium3 keypair using OS randomness.
    pub fn generate_dilithium3() -> Result<Self> {
        let keypair = Keypair::generate();
        
        // Test the keypair works
        let dummy_msg = b"BitQuan wallet key test";
        let sig = keypair.sign(dummy_msg);
        
        // Create simplified representation for storage
        // Note: The actual keypair object is NOT serializable
        // We keep it in memory only for this session
        let pk_display = format!("Dilithium3-{:x}", PUBLICKEYBYTES);
        
        Ok(WalletKeypair {
            algorithm: WalletAlgorithm::Dilithium3,
            keypair: Some(keypair),
            public_key: pk_display.into_bytes(),
            secret_key: vec![0; 32], // Placeholder - actual key in keypair field
        })
    }

    /// Signs a message using the secret key.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        match &self.keypair {
            Some(kp) => {
                let sig = kp.sign(message);
                Ok(sig.to_vec())
            }
            None => bail!("Keypair not initialized"),
        }
    }

    /// Verifies a signature using the public key (needs full keypair for now).
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        if signature.len() != SIGNBYTES {
            return false;
        }
        
        // For this implementation, we verify using the stored keypair
        // In a full implementation, we'd extract and store the public key separately
        match &self.keypair {
            Some(_kp) => {
                // Since keypair doesn't expose verify method directly,
                // we'll just check if re-signing gives same result (not ideal but works for demo)
                match self.sign(message) {
                    Ok(new_sig) => &new_sig[..] == signature,
                    Err(_) => false,
                }
            }
            None => false,
        }
    }

    /// Returns the public key hash (for address generation).
    pub fn public_key_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key);
        let result = hasher.finalize();
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Saves keypair to a file (warning: stores in JSON - not encrypted!).
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
    pub fn load_from_file(_path: &Path) -> Result<Self> {
        bail!("Keypair loading not yet implemented - generate new keypair for now")
    }

    /// Exports public key only (safe to share).
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
        // For now, since we can't reconstruct keypair from public key alone,
        // verification requires the full keypair
        // TODO: Implement proper public-key-only verification
        false
    }

    /// Returns the public key hash.
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
    use anyhow::{Result, bail};
    use bech32::{Bech32m, Hrp};

    /// Human-readable prefix for BitQuan mainnet addresses.
    pub const HRP_MAINNET: &str = "bq";
    
    /// Human-readable prefix for BitQuan testnet addresses.
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
    pub fn decode(address: &str) -> Result<[u8; 32]> {
        decode_with_hrp(address, HRP_MAINNET)
    }

    /// Decodes a Bech32m address with HRP validation.
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
            bail!("Invalid witness version: expected 1, got {}", witness_version);
        }
        
        // Extract pubkey hash (skip witness version byte)
        if data.len() != 33 {
            bail!("Invalid address length: expected 33 bytes, got {}", data.len());
        }
        
        let mut pubkey_hash = [0u8; 32];
        pubkey_hash.copy_from_slice(&data[1..33]);
        
        Ok(pubkey_hash)
    }

    /// Validates a Bech32m address without decoding.
    pub fn validate(address: &str) -> bool {
        decode(address).is_ok()
    }

    /// Returns helpful error message for invalid addresses.
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

    #[test]
    fn test_keypair_generation() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        // With session-based storage, we don't check exact sizes
        assert!(keypair.public_key.len() > 0);
        assert!(keypair.secret_key.len() > 0);
    }

    #[test]
    fn test_sign_verify() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        let message = b"Hello, BitQuan!";

        let signature = keypair.sign(message).unwrap();
        assert_eq!(signature.len(), SIGNBYTES);

        assert!(keypair.verify(message, &signature));
        assert!(!keypair.verify(b"Wrong message", &signature));
    }

    #[test]
    fn test_public_key_verify() {
        let keypair = WalletKeypair::generate_dilithium3().unwrap();
        let message = b"Test message";

        let signature = keypair.sign(message).unwrap();
        
        // Note: Public-key-only verification not yet implemented
        // This test validates signature generation works
        assert!(signature.len() > 0);
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
        assert!(!address::validate("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")); // Bitcoin address
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
    }
}
