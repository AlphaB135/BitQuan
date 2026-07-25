//! Multi-signature wallet implementation for BitQuan.
//!
//! Supports M-of-N multi-signature schemes where M signatures are required
//! from a set of N public keys to authorize a transaction.

use bitquan_types::time;
use bq_crypto::CryptoRegistry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;

/// Multi-signature wallet configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultisigConfig {
    /// Number of required signatures (M).
    pub required_sigs: u8,
    /// Total number of signers (N).
    pub total_signers: u8,
    /// List of public keys (Dilithium5 public keys as hex strings).
    pub public_keys: Vec<String>,
    /// Optional label for this multisig wallet.
    pub label: Option<String>,
    /// Creation timestamp (Unix timestamp).
    pub created_at: u64,
    /// Network this wallet belongs to (domain separator for sighash).
    ///
    /// Prevents cross-network replay: a signature for a mainnet transaction
    /// must not be valid on testnet or devnet (issue #207).
    /// Defaults to Mainnet when absent for backward compatibility.
    #[serde(default)]
    pub network_id: u8,
}

/// Represents a partial signature from one signer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialSignature {
    /// Public key of the signer (hex string).
    pub public_key: String,
    /// Signature bytes (Dilithium5 signature as hex string).
    pub signature: String,
    /// Timestamp when signed.
    pub signed_at: u64,
}

/// A pending multisig transaction awaiting signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingMultisigTx {
    /// Transaction ID or hash (hex string).
    pub tx_id: String,
    /// Raw transaction data to be signed (hex string).
    pub tx_data: String,
    /// Multisig configuration for this transaction.
    pub config: MultisigConfig,
    /// Collected signatures so far.
    pub signatures: Vec<PartialSignature>,
    /// Transaction creation timestamp.
    pub created_at: u64,
}

/// Multi-signature wallet implementation.
#[derive(Clone)]
pub struct MultisigWallet {
    config: MultisigConfig,
    crypto_registry: Arc<CryptoRegistry>,
}

impl std::fmt::Debug for MultisigWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultisigWallet")
            .field("config", &self.config)
            .field("crypto_registry", &"<CryptoRegistry>")
            .finish()
    }
}

/// Errors related to multi-signature wallet operations
#[derive(Debug, Error)]
pub enum MultisigError {
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    /// Insufficient signatures provided
    #[error("Insufficient signatures: need {required}, have {actual}")]
    InsufficientSignatures {
        /// Number of signatures required
        required: u8,
        /// Number of signatures provided
        actual: u8,
    },
    /// Duplicate signature found
    #[error("Duplicate signature from public key: {0}")]
    DuplicateSignature(String),
    /// Signer not authorized
    #[error("Unknown signer: {0}")]
    UnknownSigner(String),
    /// Signature verification failed
    #[error("Invalid signature")]
    InvalidSignature,
    /// Transaction is already fully signed
    #[error("Transaction already complete")]
    AlreadyComplete,
    /// System clock error
    #[error("system time error: {0}")]
    Clock(&'static str),
}

fn unix_timestamp() -> Result<u64, MultisigError> {
    time::unix_timestamp().map_err(|_| MultisigError::Clock("clock before epoch"))
}

impl MultisigConfig {
    /// Creates a new multisig configuration.
    pub fn new(
        required_sigs: u8,
        public_keys: Vec<String>,
        label: Option<String>,
    ) -> Result<Self, MultisigError> {
        if public_keys.len() > u8::MAX as usize {
            return Err(MultisigError::InvalidConfig(
                "Too many public keys (maximum 255)".to_string(),
            ));
        }
        let total_signers = public_keys.len() as u8;

        // Validation
        if required_sigs == 0 {
            return Err(MultisigError::InvalidConfig(
                "Required signatures must be at least 1".to_string(),
            ));
        }

        if total_signers == 0 {
            return Err(MultisigError::InvalidConfig(
                "Must have at least 1 public key".to_string(),
            ));
        }

        if required_sigs > total_signers {
            return Err(MultisigError::InvalidConfig(format!(
                "Required signatures ({}) cannot exceed total signers ({})",
                required_sigs, total_signers
            )));
        }

        // Check for duplicate public keys
        let mut seen = HashSet::new();
        for pk in &public_keys {
            if !seen.insert(pk) {
                return Err(MultisigError::InvalidConfig(format!(
                    "Duplicate public key: {}",
                    pk
                )));
            }
        }

        let created_at = unix_timestamp()?;

        Ok(Self {
            required_sigs,
            total_signers,
            public_keys,
            label,
            created_at,
            network_id: 0,
        })
    }

    /// Returns the multisig address (hash of the configuration).
    ///
    /// ## Security fixes
    ///
    /// **#208 — key length prefix**: keys are now prefixed with their 4-byte
    /// little-endian length before hashing. Without this, `["ab","cdef"]` and
    /// `["abcd","ef"]` produced identical byte streams → identical addresses,
    /// allowing an attacker to craft a collision key in a legitimate M-of-N wallet.
    ///
    /// **#207 — network domain separator**: `network_id` is mixed into the hash
    /// so a mainnet address can never collide with a testnet/devnet address.
    pub fn address(&self) -> String {
        let mut hasher = Sha256::new();

        // Domain separator — prevents cross-network address collision (#207)
        hasher.update(b"BitQuan-multisig-addr-v1");
        hasher.update([self.network_id]);

        hasher.update([self.required_sigs]);
        hasher.update([self.total_signers]);

        for pk in &self.public_keys {
            // Length-prefix each key so ["ab","cdef"] ≠ ["abcd","ef"] (#208)
            let len = pk.len() as u32;
            hasher.update(len.to_le_bytes());
            hasher.update(pk.as_bytes());
        }

        let hash = hasher.finalize();
        format!("bqms1{}", hex::encode(&hash[..20]))
    }

    /// Checks if a public key is part of this multisig.
    pub fn is_signer(&self, public_key: &str) -> bool {
        self.public_keys.iter().any(|pk| pk == public_key)
    }

    /// Returns the configuration type as a string (e.g., "2-of-3").
    pub fn config_type(&self) -> String {
        format!("{}-of-{}", self.required_sigs, self.total_signers)
    }
}

impl MultisigWallet {
    /// Creates a new multisig wallet with the given configuration.
    pub fn new(config: MultisigConfig) -> Self {
        Self {
            config,
            crypto_registry: Arc::new(CryptoRegistry::default()),
        }
    }

    /// Creates a new multisig wallet with custom crypto registry (for performance optimization).
    pub fn with_registry(config: MultisigConfig, registry: Arc<CryptoRegistry>) -> Self {
        Self {
            config,
            crypto_registry: registry,
        }
    }

    /// Returns the wallet configuration.
    pub fn config(&self) -> &MultisigConfig {
        &self.config
    }

    /// Creates a new pending transaction for signing.
    ///
    /// # Fallback
    /// If system clock is unavailable, uses timestamp 0 (epoch).
    pub fn create_pending_tx(&self, tx_data: &[u8]) -> PendingMultisigTx {
        let tx_id = self.compute_tx_id(tx_data);
        // FALLBACK: Use epoch if clock unavailable (non-critical timestamp)
        let created_at = unix_timestamp().unwrap_or(0);

        PendingMultisigTx {
            tx_id: hex::encode(tx_id),
            tx_data: hex::encode(tx_data),
            config: self.config.clone(),
            signatures: Vec::new(),
            created_at,
        }
    }

    /// Adds a signature to a pending transaction.
    ///
    /// # Security
    /// The signature is cryptographically verified against the transaction data
    /// before being accepted. Invalid or forged signatures are rejected.
    pub fn add_signature(
        &self,
        pending: &mut PendingMultisigTx,
        public_key: &str,
        signature: &[u8],
    ) -> Result<(), MultisigError> {
        // Check if transaction is already complete
        if pending.is_complete() {
            return Err(MultisigError::AlreadyComplete);
        }

        // Verify signer is authorized
        if !self.config.is_signer(public_key) {
            return Err(MultisigError::UnknownSigner(public_key.to_string()));
        }

        // Check for duplicate signature
        if pending.has_signature_from(public_key) {
            return Err(MultisigError::DuplicateSignature(public_key.to_string()));
        }

        // SECURITY FIX (C-3): Cryptographically verify the signature before accepting it.
        // This prevents collection of invalid/forged signatures that would only be caught
        // at finalization time, reducing the attack window.
        let tx_data = hex::decode(&pending.tx_data).map_err(|_| MultisigError::InvalidSignature)?;
        let tx_hash = {
            let mut hasher = Sha256::new();
            // Domain separator — prevents cross-network replay (#207)
            hasher.update(b"BitQuan-multisig-sighash-v1");
            hasher.update([self.config.network_id]);
            hasher.update(&tx_data);
            hasher.finalize()
        };
        let pubkey_bytes = hex::decode(public_key).map_err(|_| MultisigError::InvalidSignature)?;
        if !self.verify_pqc_signature(signature, &pubkey_bytes, &tx_hash)? {
            return Err(MultisigError::InvalidSignature);
        }

        // Signature verified — add it
        let signed_at = unix_timestamp()?;

        pending.signatures.push(PartialSignature {
            public_key: public_key.to_string(),
            signature: hex::encode(signature),
            signed_at,
        });

        Ok(())
    }

    /// Verifies that a pending transaction has enough valid signatures.
    pub fn verify_signatures(&self, pending: &PendingMultisigTx) -> Result<(), MultisigError> {
        let sig_count = pending.signatures.len().try_into().unwrap_or(u8::MAX);

        if sig_count < self.config.required_sigs {
            return Err(MultisigError::InsufficientSignatures {
                required: self.config.required_sigs,
                actual: sig_count,
            });
        }

        // Check all signers are authorized
        for sig in &pending.signatures {
            if !self.config.is_signer(&sig.public_key) {
                return Err(MultisigError::UnknownSigner(sig.public_key.clone()));
            }
        }

        // Check for duplicate signatures
        let mut seen = HashSet::new();
        for sig in &pending.signatures {
            if !seen.insert(&sig.public_key) {
                return Err(MultisigError::DuplicateSignature(sig.public_key.clone()));
            }
        }

        // CRITICAL: Verify cryptographic validity of each signature
        // First decode the raw transaction data
        let tx_data = hex::decode(&pending.tx_data).map_err(|_| MultisigError::InvalidSignature)?;

        // CRITICAL FIX: Always hash the transaction data before signature verification
        // Dilithium5 should sign a digest, not raw data (for security and performance)
        let tx_hash = {
            let mut hasher = Sha256::new();
            // Domain separator — prevents cross-network replay (#207)
            hasher.update(b"BitQuan-multisig-sighash-v1");
            hasher.update([self.config.network_id]);
            hasher.update(&tx_data);
            hasher.finalize()
        };

        for sig in &pending.signatures {
            let signature_bytes =
                hex::decode(&sig.signature).map_err(|_| MultisigError::InvalidSignature)?;

            let pubkey_bytes =
                hex::decode(&sig.public_key).map_err(|_| MultisigError::InvalidSignature)?;

            // Verify PQC signature using the transaction HASH (not raw data)
            if !self.verify_pqc_signature(&signature_bytes, &pubkey_bytes, &tx_hash)? {
                return Err(MultisigError::InvalidSignature);
            }
        }

        Ok(())
    }

    /// Finalizes a pending transaction if it has enough signatures.
    pub fn finalize_transaction(
        &self,
        pending: &PendingMultisigTx,
    ) -> Result<FinalizedMultisigTx, MultisigError> {
        self.verify_signatures(pending)?;

        Ok(FinalizedMultisigTx {
            tx_id: pending.tx_id.clone(),
            tx_data: pending.tx_data.clone(),
            config: pending.config.clone(),
            signatures: pending.signatures.clone(),
            finalized_at: unix_timestamp()?,
        })
    }

    /// Verifies a PQC signature using the same logic as ScriptInterpreter.
    /// PERFORMANCE FIX: Uses cached registry instead of creating new one each time.
    fn verify_pqc_signature(
        &self,
        sig: &[u8],
        pubkey: &[u8],
        message: &[u8],
    ) -> Result<bool, MultisigError> {
        use bitquan_types::SigAlgorithm;
        use bitquan_types::SignaturePayload;

        // Create signature payload
        let payload = SignaturePayload {
            signer_index: 0,
            signature: sig.to_vec(),
            public_key: pubkey.to_vec(),
            aux: None,
        };

        // PERFORMANCE FIX: Use cached registry instead of creating new one
        let provider = self
            .crypto_registry
            .provider_for(SigAlgorithm::Dilithium5)
            .ok_or(MultisigError::InvalidSignature)?;

        // Verify signature
        match provider.verify(&payload, message) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn compute_tx_id(&self, tx_data: &[u8]) -> [u8; 32] {
        // Include a network domain separator so tx IDs (and therefore signatures)
        // are network-specific. Without this, a signature for a mainnet transaction
        // is byte-for-byte valid on testnet/devnet — a cross-network replay attack
        // (issue #207). The domain tag, network_id, and protocol version are mixed
        // in before the transaction data.
        let mut hasher = Sha256::new();
        hasher.update(b"BitQuan-multisig-sighash-v1");
        hasher.update([self.config.network_id]);
        hasher.update(tx_data);
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        id
    }
}

impl PendingMultisigTx {
    /// Checks if the transaction has enough signatures.
    pub fn is_complete(&self) -> bool {
        self.signatures.len() >= self.config.required_sigs as usize
    }

    /// Checks if a specific public key has already signed.
    pub fn has_signature_from(&self, public_key: &str) -> bool {
        self.signatures
            .iter()
            .any(|sig| sig.public_key == public_key)
    }

    /// Returns the number of signatures collected.
    pub fn signature_count(&self) -> u8 {
        self.signatures.len().try_into().unwrap_or(u8::MAX)
    }

    /// Returns the number of signatures still needed.
    pub fn signatures_needed(&self) -> u8 {
        self.config
            .required_sigs
            .saturating_sub(self.signature_count())
    }

    /// Returns a list of signers who haven't signed yet.
    pub fn pending_signers(&self) -> Vec<String> {
        self.config
            .public_keys
            .iter()
            .filter(|pk| !self.has_signature_from(pk))
            .cloned()
            .collect()
    }

    /// Returns signing progress as a percentage.
    pub fn progress_percentage(&self) -> f64 {
        (self.signature_count() as f64 / self.config.required_sigs as f64) * 100.0
    }
}

/// A finalized multisig transaction with all required signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizedMultisigTx {
    /// Transaction ID.
    pub tx_id: String,
    /// Raw transaction data.
    pub tx_data: String,
    /// Multisig configuration.
    pub config: MultisigConfig,
    /// All collected signatures.
    pub signatures: Vec<PartialSignature>,
    /// Finalization timestamp.
    pub finalized_at: u64,
}

impl FinalizedMultisigTx {
    /// Verifies the **structural** validity of this finalized transaction.
    ///
    /// This checks:
    /// - Sufficient number of signatures (M-of-N threshold)
    /// - All signers are authorized members of the multisig config
    /// - No duplicate signatures from the same signer
    ///
    /// # Security Warning
    /// This method does **NOT** perform cryptographic signature verification.
    /// It only validates the structural constraints of the multisig scheme.
    /// Callers **MUST** use [`MultisigWallet::verify_signatures()`] (which holds
    /// a `CryptoRegistry`) for full cryptographic verification before trusting
    /// the transaction.
    ///
    /// # Renamed from `verify()`
    /// Previously named `verify()`, which implied cryptographic verification.
    /// Renamed to `verify_structure()` to prevent false trust (security fix C-4).
    pub fn verify_structure(&self) -> Result<(), MultisigError> {
        let sig_count = self.signatures.len().try_into().unwrap_or(u8::MAX);

        if sig_count < self.config.required_sigs {
            return Err(MultisigError::InsufficientSignatures {
                required: self.config.required_sigs,
                actual: sig_count,
            });
        }

        // Check all signers are authorized
        let mut seen = HashSet::new();
        for sig in &self.signatures {
            if !self.config.is_signer(&sig.public_key) {
                return Err(MultisigError::UnknownSigner(sig.public_key.clone()));
            }
            if !seen.insert(&sig.public_key) {
                return Err(MultisigError::DuplicateSignature(sig.public_key.clone()));
            }
        }

        Ok(())
    }
}

/// Helper for managing multiple multisig wallets.
#[derive(Clone, Debug, Default)]
pub struct MultisigWalletManager {
    wallets: HashMap<String, MultisigWallet>,
    pending_txs: HashMap<String, PendingMultisigTx>,
}

impl MultisigWalletManager {
    /// Creates a new manager.
    pub fn new() -> Self {
        Self {
            wallets: HashMap::new(),
            pending_txs: HashMap::new(),
        }
    }

    /// Adds a multisig wallet.
    pub fn add_wallet(&mut self, wallet: MultisigWallet) {
        let address = wallet.config.address();
        self.wallets.insert(address, wallet);
    }

    /// Gets a wallet by address.
    pub fn get_wallet(&self, address: &str) -> Option<&MultisigWallet> {
        self.wallets.get(address)
    }

    /// Lists all wallet addresses.
    pub fn list_addresses(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    /// Adds a pending transaction.
    pub fn add_pending_tx(&mut self, tx: PendingMultisigTx) {
        self.pending_txs.insert(tx.tx_id.clone(), tx);
    }

    /// Gets a pending transaction by ID.
    pub fn get_pending_tx(&self, tx_id: &str) -> Option<&PendingMultisigTx> {
        self.pending_txs.get(tx_id)
    }

    /// Gets a mutable pending transaction by ID.
    pub fn get_pending_tx_mut(&mut self, tx_id: &str) -> Option<&mut PendingMultisigTx> {
        self.pending_txs.get_mut(tx_id)
    }

    /// Removes a pending transaction.
    pub fn remove_pending_tx(&mut self, tx_id: &str) -> Option<PendingMultisigTx> {
        self.pending_txs.remove(tx_id)
    }

    /// Lists all pending transaction IDs.
    pub fn list_pending_txs(&self) -> Vec<String> {
        self.pending_txs.keys().cloned().collect()
    }

    /// Lists complete pending transactions.
    pub fn list_complete_txs(&self) -> Vec<String> {
        self.pending_txs
            .iter()
            .filter(|(_, tx)| tx.is_complete())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Lists incomplete pending transactions.
    pub fn list_incomplete_txs(&self) -> Vec<String> {
        self.pending_txs
            .iter()
            .filter(|(_, tx)| !tx.is_complete())
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pubkeys() -> Vec<String> {
        vec![
            "pubkey1".to_string(),
            "pubkey2".to_string(),
            "pubkey3".to_string(),
        ]
    }

    #[test]
    fn test_multisig_config_creation() {
        let config = MultisigConfig::new(2, sample_pubkeys(), Some("Test".to_string()))
            .expect("Failed to create multisig config");
        assert_eq!(config.required_sigs, 2);
        assert_eq!(config.total_signers, 3);
        assert_eq!(config.config_type(), "2-of-3");
    }

    #[test]
    fn test_multisig_config_validation() {
        // Required > total should fail
        let result = MultisigConfig::new(4, sample_pubkeys(), None);
        assert!(result.is_err());

        // Zero required should fail
        let result = MultisigConfig::new(0, sample_pubkeys(), None);
        assert!(result.is_err());

        // Empty keys should fail
        let result = MultisigConfig::new(1, vec![], None);
        assert!(result.is_err());

        // Duplicate keys should fail
        let dup_keys = vec!["key1".to_string(), "key1".to_string()];
        let result = MultisigConfig::new(2, dup_keys, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_multisig_address_generation() {
        let config1 =
            MultisigConfig::new(2, sample_pubkeys(), None).expect("Failed to create config1");
        let config2 =
            MultisigConfig::new(2, sample_pubkeys(), None).expect("Failed to create config2");

        // Same config should generate same address
        assert_eq!(config1.address(), config2.address());

        // Different config should generate different address
        let config3 =
            MultisigConfig::new(3, sample_pubkeys(), None).expect("Failed to create config3");
        assert_ne!(config1.address(), config3.address());
    }

    #[test]
    fn test_is_signer() {
        let config =
            MultisigConfig::new(2, sample_pubkeys(), None).expect("Failed to create config");
        assert!(config.is_signer("pubkey1"));
        assert!(config.is_signer("pubkey2"));
        assert!(config.is_signer("pubkey3"));
        assert!(!config.is_signer("pubkey4"));
    }

    #[test]
    fn test_create_pending_tx() {
        let config =
            MultisigConfig::new(2, sample_pubkeys(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"sample transaction data";
        let pending = wallet.create_pending_tx(tx_data);

        assert_eq!(pending.tx_data, hex::encode(tx_data));
        assert_eq!(pending.signatures.len(), 0);
        assert!(!pending.is_complete());
    }

    fn setup_keys() -> (Vec<String>, Vec<pqc_dilithium_seeded::Keypair>) {
        let mut hex_keys = Vec::new();
        let mut keypairs = Vec::new();
        for _ in 0..3 {
            let kp = pqc_dilithium_seeded::Keypair::generate();
            hex_keys.push(hex::encode(&kp.public));
            keypairs.push(kp);
        }
        (hex_keys, keypairs)
    }

    fn sign_tx_data(kp: &pqc_dilithium_seeded::Keypair, tx_data: &[u8]) -> Vec<u8> {
        let tx_hash = {
            let mut hasher = Sha256::new();
            hasher.update(tx_data);
            hasher.finalize()
        };
        kp.sign(&tx_hash).to_vec()
    }

    #[test]
    fn test_add_signature() {
        let (hex_keys, keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys.clone(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"tx data";
        let mut pending = wallet.create_pending_tx(tx_data);

        // Add first signature
        let sig1 = sign_tx_data(&keypairs[0], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[0], &sig1)
            .expect("Failed to add first signature");
        assert_eq!(pending.signature_count(), 1);
        assert!(!pending.is_complete());

        // Add second signature
        let sig2 = sign_tx_data(&keypairs[1], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[1], &sig2)
            .expect("Failed to add second signature");
        assert_eq!(pending.signature_count(), 2);
        assert!(pending.is_complete());
    }

    #[test]
    fn test_duplicate_signature_rejected() {
        let (hex_keys, keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys.clone(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"tx data";
        let mut pending = wallet.create_pending_tx(tx_data);

        let sig1 = sign_tx_data(&keypairs[0], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[0], &sig1)
            .expect("Failed to add signature");
        let result = wallet.add_signature(&mut pending, &hex_keys[0], &sig1);

        assert!(matches!(result, Err(MultisigError::DuplicateSignature(_))));
    }

    #[test]
    fn test_unknown_signer_rejected() {
        let (hex_keys, _keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys, None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let mut pending = wallet.create_pending_tx(b"tx data");
        let unknown_kp = pqc_dilithium_seeded::Keypair::generate();
        let unknown_pub = hex::encode(&unknown_kp.public);
        let sig = unknown_kp.sign(b"tx data");
        let result = wallet.add_signature(&mut pending, &unknown_pub, &sig);

        assert!(matches!(result, Err(MultisigError::UnknownSigner(_))));
    }

    #[test]
    fn test_finalize_transaction() {
        let (hex_keys, keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys.clone(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"tx data";
        let mut pending = wallet.create_pending_tx(tx_data);

        // Add required signatures
        let sig1 = sign_tx_data(&keypairs[0], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[0], &sig1)
            .expect("Failed to add first signature");
        let sig2 = sign_tx_data(&keypairs[1], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[1], &sig2)
            .expect("Failed to add second signature");
        assert_eq!(pending.progress_percentage(), 100.0);
    }

    #[test]
    fn test_invalid_signature_rejected() {
        let (hex_keys, keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys.clone(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"test transaction data";
        let mut pending = wallet.create_pending_tx(tx_data);

        // Add valid signature
        let sig1 = sign_tx_data(&keypairs[0], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[0], &sig1)
            .expect("Failed to add first signature");

        // Add invalid signature (wrong signature bytes)
        let invalid_sig = b"invalid_signature".to_vec();
        let result = wallet.add_signature(&mut pending, &hex_keys[1], &invalid_sig);
        assert!(matches!(result, Err(MultisigError::InvalidSignature)));
    }

    #[test]
    fn test_malformed_signature_hex_rejected() {
        let (hex_keys, keypairs) = setup_keys();
        let config =
            MultisigConfig::new(1, hex_keys.clone(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"test data";
        let mut pending = wallet.create_pending_tx(tx_data);

        // Add a signature first
        let sig1 = sign_tx_data(&keypairs[0], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[0], &sig1)
            .expect("Failed to add signature");

        // Set invalid hex in tx_data
        pending.tx_data = "invalid_hex_data".to_string();

        let result = wallet.verify_signatures(&pending);
        assert!(matches!(result, Err(MultisigError::InvalidSignature)));
    }

    #[test]
    fn test_manager_pending_txs() {
        let mut manager = MultisigWalletManager::new();
        let (hex_keys, _keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys, None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let pending = wallet.create_pending_tx(b"tx data");
        let tx_id = pending.tx_id.clone();

        manager.add_pending_tx(pending);

        assert!(manager.get_pending_tx(&tx_id).is_some());
        assert_eq!(manager.list_pending_txs().len(), 1);
        assert_eq!(manager.list_incomplete_txs().len(), 1);
        assert_eq!(manager.list_complete_txs().len(), 0);
    }

    #[test]
    fn test_already_complete_error() {
        let (hex_keys, keypairs) = setup_keys();
        let config =
            MultisigConfig::new(2, hex_keys.clone(), None).expect("Failed to create config");
        let wallet = MultisigWallet::new(config);

        let tx_data = b"tx data";
        let mut pending = wallet.create_pending_tx(tx_data);

        // Add required signatures
        let sig1 = sign_tx_data(&keypairs[0], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[0], &sig1)
            .expect("Failed to add first signature");
        let sig2 = sign_tx_data(&keypairs[1], tx_data);
        wallet
            .add_signature(&mut pending, &hex_keys[1], &sig2)
            .expect("Failed to add second signature");

        // Try to add another signature
        let sig3 = sign_tx_data(&keypairs[2], tx_data);
        let result = wallet.add_signature(&mut pending, &hex_keys[2], &sig3);
        assert!(matches!(result, Err(MultisigError::AlreadyComplete)));
    }
}
