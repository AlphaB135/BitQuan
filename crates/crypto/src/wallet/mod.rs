//! Wallet-related utilities: secure types, KDF, encryption, keystore helpers.

pub mod encryption;
pub mod kdf;
pub mod keystore;
pub mod secure_memory_pool;
pub mod secure_types;
pub mod session;

pub use encryption::{EncryptedData, Encryptor};
pub use keystore::{Keystore, KeystoreError};
pub use secure_memory_pool::{MemoryPoolStats, SecureMemoryManager, SecureMemoryPool};
pub use secure_types::{SecurePrivateKey, SecureString};
pub use session::{SessionError, WalletSession};
