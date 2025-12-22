//! BitQuan cryptography utilities.
#![cfg_attr(
    not(any(feature = "memory-locking", feature = "memory-security")),
    forbid(unsafe_code)
)]
#![deny(missing_docs)]

use std::collections::HashMap;

use bitquan_types::{SigAlgorithm, SignaturePayload, Transaction};
use pqc_dilithium_seeded as dilithium;
use thiserror::Error;

pub mod constant_time;
pub mod rng;
pub mod wallet;

/// Error type returned when cryptographic operations fail.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// The requested signature scheme has not been implemented yet.
    #[error("signature scheme {0:?} is not implemented")]
    NotImplemented(SigAlgorithm),
    /// The signature payload is malformed or inconsistent.
    #[error("malformed signature payload: {0}")]
    Malformed(&'static str),
}

/// Trait implemented by concrete signature scheme providers.
pub trait SignatureScheme: Send + Sync {
    /// Returns the algorithm identifier.
    fn algorithm(&self) -> SigAlgorithm;

    /// Verifies a signature payload against the supplied message digest.
    fn verify(&self, payload: &SignaturePayload, message: &[u8]) -> Result<(), CryptoError>;
}

/// Shared pointer type for signature scheme implementations.
pub type SignatureSchemeRef = Box<dyn SignatureScheme>;

/// Registry responsible for dispatching signature verification requests.
pub struct CryptoRegistry {
    schemes: HashMap<u8, SignatureSchemeRef>,
}

impl CryptoRegistry {
    /// Constructs an empty registry.
    pub fn new() -> Self {
        Self {
            schemes: HashMap::new(),
        }
    }

    /// Installs the default set of signature providers (Dilithium only for now).
    pub fn with_default_providers() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(DilithiumProvider));
        registry
    }

    /// Registers a new signature scheme provider.
    pub fn register(&mut self, provider: SignatureSchemeRef) {
        self.schemes.insert(provider.algorithm().code(), provider);
    }

    /// Fetches the provider for a given algorithm.
    pub fn provider_for(&self, algorithm: SigAlgorithm) -> Option<&dyn SignatureScheme> {
        self.schemes
            .get(&algorithm.code())
            .map(|provider| provider.as_ref())
    }

    /// Verifies all signatures in a transaction using a pre-computed message digest.
    ///
    /// Note: The caller is responsible for computing the correct sighash.
    /// For block validation, use the helpers in the consensus crate.
    pub fn verify_transaction(
        &self,
        tx: &Transaction,
        message_digest: &[u8],
    ) -> Result<(), CryptoError> {
        let provider = self
            .provider_for(tx.sig_algo)
            .ok_or(CryptoError::NotImplemented(tx.sig_algo))?;

        for witness in &tx.witnesses {
            for payload in &witness.signatures {
                provider.verify(payload, message_digest)?;
            }
        }

        Ok(())
    }
}

impl Default for CryptoRegistry {
    fn default() -> Self {
        Self::with_default_providers()
    }
}

/// Dilithium5 signature scheme implementation with constant-time verification.
#[derive(Default)]
pub struct DilithiumProvider;

impl SignatureScheme for DilithiumProvider {
    fn algorithm(&self) -> SigAlgorithm {
        SigAlgorithm::Dilithium5
    }

    fn verify(&self, payload: &SignaturePayload, message: &[u8]) -> Result<(), CryptoError> {
        // Dilithium signature sizes (level 5)
        const DILITHIUM5_SIG_SIZE: usize = 4595;
        const DILITHIUM5_PK_SIZE: usize = 2592;

        // Validate sizes
        if payload.signature.len() != DILITHIUM5_SIG_SIZE {
            return Err(CryptoError::Malformed("invalid signature length"));
        }
        if payload.public_key.len() != DILITHIUM5_PK_SIZE {
            return Err(CryptoError::Malformed("invalid public key length"));
        }

        // Limit message size to prevent DoS
        if message.len() > 1_000_000 {
            return Err(CryptoError::Malformed("message too large"));
        }

        // Convert to fixed arrays
        let mut sig_bytes = [0u8; DILITHIUM5_SIG_SIZE];
        let mut pk_bytes = [0u8; DILITHIUM5_PK_SIZE];
        sig_bytes.copy_from_slice(&payload.signature);
        pk_bytes.copy_from_slice(&payload.public_key);

        // Verify signature using patched pqc_dilithium
        dilithium::crypto_sign_verify(&sig_bytes, message, &pk_bytes)
            .map_err(|_| CryptoError::Malformed("signature verification failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::{SigAlgorithm, TxIn, TxOut};

    fn create_test_transaction() -> Transaction {
        Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
            lock_time: 0,
            inputs: vec![TxIn {
                prev_txid: [0x42; 32],
                prev_vout: 0,
                sequence: 0xffffffff,
                script_sig: vec![],
            }],
            outputs: vec![TxOut {
                value: 1000,
                script_pubkey: vec![0x51],
            }],
            sig_algo: SigAlgorithm::Dilithium5,
            witnesses: vec![],
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = CryptoRegistry::with_default_providers();
        assert!(registry.provider_for(SigAlgorithm::Dilithium5).is_some());
    }

    #[test]
    fn test_verify_transaction_with_digest() {
        let registry = CryptoRegistry::with_default_providers();
        let tx = create_test_transaction();
        let dummy_digest = [0u8; 32];

        // Transaction with no witnesses should verify successfully (no signatures to check)
        let result = registry.verify_transaction(&tx, &dummy_digest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_not_implemented_algorithm() {
        let registry = CryptoRegistry::new(); // No providers registered
        let tx = create_test_transaction();
        let dummy_digest = [0u8; 32];

        let result = registry.verify_transaction(&tx, &dummy_digest);

        // Should fail with NotImplemented
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::NotImplemented(_)
        ));
    }

    #[test]
    fn test_provider_lookup() {
        let registry = CryptoRegistry::with_default_providers();

        // Dilithium5 should be available
        assert!(registry.provider_for(SigAlgorithm::Dilithium5).is_some());
    }

    #[test]
    fn test_empty_registry() {
        let registry = CryptoRegistry::new();

        // No providers registered
        assert!(registry.provider_for(SigAlgorithm::Dilithium5).is_none());
    }
}
