//! Post-quantum cryptography abstractions for BitQuan.
#![warn(missing_docs)]

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
        registry.register(Box::new(DilithiumProvider::default()));
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
            .ok_or_else(|| CryptoError::NotImplemented(tx.sig_algo))?;

        for payload in &tx.signatures {
            provider.verify(payload, message_digest)?;
        }

        Ok(())
    }
}

impl Default for CryptoRegistry {
    fn default() -> Self {
        Self::with_default_providers()
    }
}

/// Placeholder implementation for the Dilithium signature scheme.
#[derive(Default)]
pub struct DilithiumProvider;

impl SignatureScheme for DilithiumProvider {
    fn algorithm(&self) -> SigAlgorithm {
        SigAlgorithm::Dilithium3
    }

    fn verify(&self, _payload: &SignaturePayload, _message: &[u8]) -> Result<(), CryptoError> {
        Err(CryptoError::NotImplemented(SigAlgorithm::Dilithium3))
    }
}
