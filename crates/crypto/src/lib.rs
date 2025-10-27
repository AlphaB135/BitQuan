//! BitQuan cryptography utilities.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;

use bitquan_types::{SigAlgorithm, SignaturePayload, Transaction};
use thiserror::Error;

pub mod rng;

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

    /// Verifies all signatures in a transaction using the configured provider.
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

        // Verify signature
        match pqc_dilithium::verify(&sig_bytes, message, &pk_bytes) {
            Ok(_) => Ok(()),
            Err(_) => Err(CryptoError::Malformed("signature verification failed")),
        }
    }
}
