//! Multi-signature wallet support for BitQuan.
//!
//! Implements M-of-N threshold signatures using Dilithium (quantum-resistant).
//! Supports various multisig schemes: 2-of-3, 3-of-5, etc.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Multi-signature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultisigConfig {
    /// Required number of signatures (M)
    pub threshold: usize,
    /// Total number of signers (N)
    pub total: usize,
    /// List of public keys (Dilithium)
    pub public_keys: Vec<Vec<u8>>,
    /// Optional labels for each signer
    pub labels: Vec<String>,
    /// Creation timestamp
    pub created_at: u64,
}

impl MultisigConfig {
    /// Create a new multisig configuration
    pub fn new(threshold: usize, public_keys: Vec<Vec<u8>>, labels: Vec<String>) -> Result<Self> {
        let total = public_keys.len();
        
        // Validate parameters
        if threshold == 0 {
            bail!("Threshold must be at least 1");
        }
        if threshold > total {
            bail!("Threshold ({}) cannot exceed total signers ({})", threshold, total);
        }
        if total < 2 {
            bail!("Multisig requires at least 2 signers");
        }
        if !labels.is_empty() && labels.len() != total {
            bail!("Labels count must match signers count");
        }
        
        // Validate public key sizes (Dilithium3 = 1952 bytes)
        for (i, pk) in public_keys.iter().enumerate() {
            if pk.len() != 1952 {
                bail!("Invalid public key size for signer {}: expected 1952, got {}", i, pk.len());
            }
        }
        
        let labels = if labels.is_empty() {
            (0..total).map(|i| format!("Signer {}", i + 1)).collect()
        } else {
            labels
        };
        
        Ok(Self {
            threshold,
            total,
            public_keys,
            labels,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    
    /// Generate multisig address from this configuration
    pub fn to_address(&self) -> String {
        // Create deterministic hash from configuration
        let mut hasher = Sha256::new();
        
        // Add threshold and total
        hasher.update(&self.threshold.to_le_bytes());
        hasher.update(&self.total.to_le_bytes());
        
        // Add all public keys in order
        for pk in &self.public_keys {
            hasher.update(pk);
        }
        
        let hash = hasher.finalize();
        
        // Use first 20 bytes for address (same as Bitcoin)
        let addr_hash = &hash[..20];
        
        // Encode as Bech32m with "bqm" (BitQuan Multisig) prefix
        crate::address::encode_bech32m_with_prefix(addr_hash, "bqm")
    }
    
    /// Validate that a set of public keys matches this config
    pub fn validate_signers(&self, signers: &[Vec<u8>]) -> Result<Vec<usize>> {
        let mut indices = Vec::new();
        
        for signer_pk in signers {
            match self.public_keys.iter().position(|pk| pk == signer_pk) {
                Some(idx) => indices.push(idx),
                None => bail!("Signer public key not found in multisig configuration"),
            }
        }
        
        Ok(indices)
    }
}

/// Partial signature from one signer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSignature {
    /// Index of the signer in the multisig config
    pub signer_index: usize,
    /// Public key of the signer
    pub public_key: Vec<u8>,
    /// Dilithium signature
    pub signature: Vec<u8>,
    /// Transaction hash that was signed
    pub tx_hash: Vec<u8>,
    /// Timestamp
    pub signed_at: u64,
}

impl PartialSignature {
    /// Create a new partial signature
    pub fn new(
        signer_index: usize,
        public_key: Vec<u8>,
        signature: Vec<u8>,
        tx_hash: Vec<u8>,
    ) -> Self {
        Self {
            signer_index,
            public_key,
            signature,
            tx_hash,
            signed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    /// Verify this partial signature
    pub fn verify(&self, message: &[u8]) -> bool {
        // TODO: Implement Dilithium signature verification
        // For now, basic validation
        !self.signature.is_empty() && self.signature.len() == 3293 // Dilithium3 sig size
    }
}

/// Collection of partial signatures for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureCollection {
    /// Multisig configuration
    pub config: MultisigConfig,
    /// Collected partial signatures
    pub signatures: HashMap<usize, PartialSignature>,
    /// Transaction hash
    pub tx_hash: Vec<u8>,
}

impl SignatureCollection {
    /// Create a new signature collection
    pub fn new(config: MultisigConfig, tx_hash: Vec<u8>) -> Self {
        Self {
            config,
            signatures: HashMap::new(),
            tx_hash,
        }
    }
    
    /// Add a partial signature
    pub fn add_signature(&mut self, sig: PartialSignature) -> Result<()> {
        // Validate signer index
        if sig.signer_index >= self.config.total {
            bail!("Invalid signer index: {}", sig.signer_index);
        }
        
        // Validate transaction hash matches
        if sig.tx_hash != self.tx_hash {
            bail!("Transaction hash mismatch");
        }
        
        // Validate public key matches config
        if self.config.public_keys[sig.signer_index] != sig.public_key {
            bail!("Public key does not match signer index");
        }
        
        // Check for duplicate
        if self.signatures.contains_key(&sig.signer_index) {
            bail!("Signature from signer {} already exists", sig.signer_index);
        }
        
        // Verify signature
        if !sig.verify(&self.tx_hash) {
            bail!("Invalid signature from signer {}", sig.signer_index);
        }
        
        self.signatures.insert(sig.signer_index, sig);
        Ok(())
    }
    
    /// Check if we have enough signatures
    pub fn is_complete(&self) -> bool {
        self.signatures.len() >= self.config.threshold
    }
    
    /// Get the number of signatures collected
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }
    
    /// Get list of signer indices who have signed
    pub fn signed_by(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.signatures.keys().copied().collect();
        indices.sort();
        indices
    }
    
    /// Get list of signer indices who haven't signed yet
    pub fn pending_signers(&self) -> Vec<usize> {
        (0..self.config.total)
            .filter(|i| !self.signatures.contains_key(i))
            .collect()
    }
    
    /// Combine signatures into final transaction signature
    pub fn combine_signatures(&self) -> Result<Vec<u8>> {
        if !self.is_complete() {
            bail!(
                "Not enough signatures: have {}, need {}",
                self.signature_count(),
                self.config.threshold
            );
        }
        
        // Collect signatures in deterministic order
        let mut combined = Vec::new();
        let signed_by = self.signed_by();
        
        for idx in signed_by.iter().take(self.config.threshold) {
            if let Some(sig) = self.signatures.get(idx) {
                combined.extend_from_slice(&sig.signature);
            }
        }
        
        Ok(combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_mock_public_key() -> Vec<u8> {
        vec![0u8; 1952] // Dilithium3 public key size
    }
    
    #[test]
    fn test_multisig_config_creation() {
        let pk1 = create_mock_public_key();
        let pk2 = create_mock_public_key();
        let pk3 = create_mock_public_key();
        
        let config = MultisigConfig::new(
            2,
            vec![pk1, pk2, pk3],
            vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()],
        )
        .unwrap();
        
        assert_eq!(config.threshold, 2);
        assert_eq!(config.total, 3);
        assert_eq!(config.public_keys.len(), 3);
    }
    
    #[test]
    fn test_multisig_validation() {
        let pk1 = create_mock_public_key();
        let pk2 = create_mock_public_key();
        
        // Invalid: threshold too high
        assert!(MultisigConfig::new(3, vec![pk1.clone(), pk2.clone()], vec![]).is_err());
        
        // Invalid: threshold zero
        assert!(MultisigConfig::new(0, vec![pk1.clone(), pk2.clone()], vec![]).is_err());
        
        // Invalid: only one signer
        assert!(MultisigConfig::new(1, vec![pk1.clone()], vec![]).is_err());
    }
    
    #[test]
    fn test_signature_collection() {
        let pk1 = create_mock_public_key();
        let pk2 = create_mock_public_key();
        let pk3 = create_mock_public_key();
        
        let config = MultisigConfig::new(
            2,
            vec![pk1.clone(), pk2.clone(), pk3.clone()],
            vec![],
        )
        .unwrap();
        
        let tx_hash = vec![1, 2, 3, 4];
        let mut collection = SignatureCollection::new(config, tx_hash.clone());
        
        assert!(!collection.is_complete());
        assert_eq!(collection.signature_count(), 0);
        
        // Add first signature
        let sig1 = PartialSignature::new(0, pk1, vec![0u8; 3293], tx_hash.clone());
        collection.add_signature(sig1).unwrap();
        
        assert!(!collection.is_complete());
        assert_eq!(collection.signature_count(), 1);
        
        // Add second signature
        let sig2 = PartialSignature::new(1, pk2, vec![0u8; 3293], tx_hash);
        collection.add_signature(sig2).unwrap();
        
        assert!(collection.is_complete());
        assert_eq!(collection.signature_count(), 2);
    }
}
