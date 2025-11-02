//! BitQuan cryptography utilities.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;

use bitquan_types::{SigAlgorithm, SignaturePayload, Transaction, TxContext};
use pqc_dilithium_seeded as dilithium;
use thiserror::Error;

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
    /// Signature hash computation failed.
    #[error("sighash error: {0}")]
    Sighash(String),
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

    /// Verifies all signatures in a transaction using the configured provider.
    ///
    /// This method computes the transaction sighash using the provided context
    /// and verifies all witness signatures against that hash.
    pub fn verify_transaction(&self, tx: &Transaction, ctx: &TxContext) -> Result<(), CryptoError> {
        // Compute sighash using consensus algorithm
        let message_digest = bitquan_consensus::transaction_sighash(tx, ctx)
            .map_err(|e| CryptoError::Sighash(e.to_string()))?;

        let provider = self
            .provider_for(tx.sig_algo)
            .ok_or(CryptoError::NotImplemented(tx.sig_algo))?;

        for witness in &tx.witnesses {
            for payload in &witness.signatures {
                provider.verify(payload, &message_digest)?;
            }
        }

        Ok(())
    }

    /// Verifies all signatures in a transaction using a pre-computed digest (legacy).
    ///
    /// DEPRECATED: Use verify_transaction with TxContext instead.
    /// This method is kept for backward compatibility.
    #[deprecated(note = "Use verify_transaction with TxContext")]
    pub fn verify_transaction_with_digest(
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

/// Dilithium3 signature scheme implementation with constant-time verification.
#[derive(Default)]
pub struct DilithiumProvider;

impl SignatureScheme for DilithiumProvider {
    fn algorithm(&self) -> SigAlgorithm {
        SigAlgorithm::Dilithium3
    }

    fn verify(&self, payload: &SignaturePayload, message: &[u8]) -> Result<(), CryptoError> {
        // Dilithium signature sizes (level 3)
        const DILITHIUM3_SIG_SIZE: usize = 3293;
        const DILITHIUM3_PK_SIZE: usize = 1952;

        // Validate sizes
        if payload.signature.len() != DILITHIUM3_SIG_SIZE {
            return Err(CryptoError::Malformed("invalid signature length"));
        }
        if payload.public_key.len() != DILITHIUM3_PK_SIZE {
            return Err(CryptoError::Malformed("invalid public key length"));
        }

        // Limit message size to prevent DoS
        if message.len() > 1_000_000 {
            return Err(CryptoError::Malformed("message too large"));
        }

        // Convert to fixed arrays
        let mut sig_bytes = [0u8; DILITHIUM3_SIG_SIZE];
        let mut pk_bytes = [0u8; DILITHIUM3_PK_SIZE];
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
    use bitquan_types::{genesis::GENESIS_HASH_BYTES, NetworkId, SigAlgorithm, TxIn, TxOut};

    fn create_test_transaction(network: NetworkId, genesis_hash: [u8; 32]) -> Transaction {
        Transaction {
            version: 1,
            network,
            genesis_hash,
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
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = CryptoRegistry::with_default_providers();
        assert!(registry.provider_for(SigAlgorithm::Dilithium3).is_some());
    }

    #[test]
    fn test_verify_transaction_with_context() {
        let registry = CryptoRegistry::with_default_providers();
        let ctx = TxContext::new(NetworkId::Devnet, GENESIS_HASH_BYTES);
        let tx = create_test_transaction(NetworkId::Devnet, GENESIS_HASH_BYTES);

        // Transaction with no witnesses should verify successfully (no signatures to check)
        let result = registry.verify_transaction(&tx, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_context_network_mismatch() {
        let registry = CryptoRegistry::with_default_providers();

        // Create transaction for devnet
        let tx = create_test_transaction(NetworkId::Devnet, GENESIS_HASH_BYTES);

        // Try to verify with mainnet context
        let ctx = TxContext::new(NetworkId::Mainnet, GENESIS_HASH_BYTES);

        let result = registry.verify_transaction(&tx, &ctx);

        // Should fail due to network mismatch
        assert!(result.is_err());
        if let Err(CryptoError::Sighash(msg)) = result {
            assert!(msg.contains("network mismatch") || msg.contains("Network"));
        } else {
            panic!("Expected Sighash error with network mismatch");
        }
    }

    #[test]
    fn test_context_genesis_mismatch() {
        let registry = CryptoRegistry::with_default_providers();

        // Create transaction with genesis A
        let genesis_a = [0xAA; 32];
        let tx = create_test_transaction(NetworkId::Devnet, genesis_a);

        // Try to verify with genesis B
        let genesis_b = [0xBB; 32];
        let ctx = TxContext::new(NetworkId::Devnet, genesis_b);

        let result = registry.verify_transaction(&tx, &ctx);

        // Should fail due to genesis mismatch
        assert!(result.is_err());
        if let Err(CryptoError::Sighash(msg)) = result {
            assert!(msg.contains("genesis mismatch") || msg.contains("genesis"));
        } else {
            panic!("Expected Sighash error with genesis mismatch");
        }
    }

    #[test]
    fn test_different_networks_different_verification() {
        let registry = CryptoRegistry::with_default_providers();

        let tx_mainnet = create_test_transaction(NetworkId::Mainnet, GENESIS_HASH_BYTES);
        let tx_testnet = create_test_transaction(NetworkId::Testnet, GENESIS_HASH_BYTES);

        let ctx_mainnet = TxContext::new(NetworkId::Mainnet, GENESIS_HASH_BYTES);
        let ctx_testnet = TxContext::new(NetworkId::Testnet, GENESIS_HASH_BYTES);

        // Each transaction should verify with its own context
        assert!(registry
            .verify_transaction(&tx_mainnet, &ctx_mainnet)
            .is_ok());
        assert!(registry
            .verify_transaction(&tx_testnet, &ctx_testnet)
            .is_ok());

        // Cross-network verification should fail
        assert!(registry
            .verify_transaction(&tx_mainnet, &ctx_testnet)
            .is_err());
        assert!(registry
            .verify_transaction(&tx_testnet, &ctx_mainnet)
            .is_err());
    }

    #[test]
    fn test_not_implemented_algorithm() {
        let registry = CryptoRegistry::new(); // No providers registered

        let ctx = TxContext::new(NetworkId::Devnet, GENESIS_HASH_BYTES);
        let tx = create_test_transaction(NetworkId::Devnet, GENESIS_HASH_BYTES);

        let result = registry.verify_transaction(&tx, &ctx);

        // Should fail with NotImplemented
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::NotImplemented(_)
        ));
    }
}
