//! Wallet-related utilities: secure types, KDF, encryption, keystore helpers.

pub mod encryption;
pub mod kdf;
pub mod keystore;
pub mod secure_types;

pub use encryption::{EncryptedData, Encryptor};
pub use keystore::{Keystore, KeystoreError};
pub use secure_types::{SecurePrivateKey, SecureString};
