//! Secure string/private-key wrappers that zeroize memory on drop.

use serde::{Deserialize, Serialize};
use std::ops::Deref;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Password string kept in heap memory that is zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct SecureString(String);

impl SecureString {
    /// Constructs a new secure string.
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns the string as raw bytes (borrowed).
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Deref for SecureString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wrapper for private key material that zeroizes when dropped.
#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct SecurePrivateKey {
    key_bytes: Vec<u8>,
}

impl SecurePrivateKey {
    /// Creates a new secure private key from raw bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { key_bytes: bytes }
    }

    /// Returns the underlying key bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.key_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_private_key_exposes_bytes() {
        let key = SecurePrivateKey::new(vec![1, 2, 3]);
        assert_eq!(key.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn secure_string_access() {
        let secret = SecureString::new("hello".to_owned());
        assert_eq!(secret.as_bytes(), b"hello");
        assert_eq!(&*secret, "hello");
    }
}
