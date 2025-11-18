//! # BitQuan SDK
//!
//! Comprehensive SDK for BitQuan blockchain with post-quantum security.
//!
//! ## Features
//!
//! - **Post-Quantum Security**: Dilithium3 signatures for quantum resistance
//! - **PQ-PSBT**: Post-Quantum Partially Signed Bitcoin Transactions
//! - **Address Management**: Bech32m address encoding/decoding
//! - **HD Wallets**: BIP32/BIP39 with quantum-resistant enhancements
//! - **Hardware Wallets**: Standardized hardware wallet integration
//!
//! ## Quick Start
//!
//! ```rust
//! use bq_sdk::{Wallet, WalletConfig, Network};
//!
//! // Create new wallet
//! let config = WalletConfig::new(Network::Mainnet);
//! let mut wallet = Wallet::generate(&config)?;
//!
//! // Get address
//! let address = wallet.get_address(&DerivationPath::default())?;
//! println!("Address: {}", address);
//!
//! // Build transaction
//! let psbt = PQPSBT::builder()
//!     .add_input("txid...", 0)?
//!     .add_output("bq1...", 1000000)?
//!     .build()?;
//!
//! // Sign transaction
//! wallet.sign_psbt(&mut psbt)?;
//! let tx = psbt.finalize()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub mod address;
pub mod crypto;
pub mod hardware;
pub mod psbt;
pub mod wallet;

// Re-export commonly used types
pub use address::{Address, AddressError, AddressType, Network};
pub use psbt::{PQPSBT, PSBTError, PSBTInput, PSBTOutput};
pub use wallet::{
    DerivationPath, Mnemonic, Wallet, WalletConfig, WalletError, SignatureAlgorithm
};

pub use hardware::{DeviceCapabilities, HardwareWallet, HardwareError};

/// Result type for SDK operations
pub type Result<T> = std::result::Result<T, SDKError>;

/// Main SDK error type
#[derive(Debug, thiserror::Error)]
pub enum SDKError {
    /// Address-related errors
    #[error("Address error: {0}")]
    Address(#[from] AddressError),
    
    /// PSBT-related errors
    #[error("PSBT error: {0}")]
    PSBT(#[from] PSBTError),
    
    /// Wallet-related errors
    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
    
    /// Hardware wallet errors
    #[error("Hardware wallet error: {0}")]
    Hardware(#[from] HardwareError),
    
    /// Cryptographic errors
    #[error("Cryptographic error: {0}")]
    Crypto(String),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// I/O errors
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sdk_version() {
        // Basic test to ensure SDK compiles
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }
}