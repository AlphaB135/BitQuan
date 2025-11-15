//! BitQuan Blockchain SDK
//! 
//! A comprehensive Rust SDK for interacting with the BitQuan blockchain,
//! featuring post-quantum cryptography support and TypeScript bindings.

pub mod client;
pub mod transaction;
pub mod wallet;
pub mod crypto;
pub mod types;
pub mod error;

pub use client::BitQuanClient;
pub use transaction::{Transaction, TransactionBuilder};
pub use wallet::WalletManager;
pub use error::{BitQuanError, Result};

#[cfg(feature = "bindings")]
pub mod bindings;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default RPC endpoint
pub const DEFAULT_RPC_URL: &str = "https://api.bitquan.network/rpc";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_rpc_url() {
        assert!(!DEFAULT_RPC_URL.is_empty());
        assert!(DEFAULT_RPC_URL.starts_with("http"));
    }
}
