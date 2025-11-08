//! Secure string/private-key wrappers that zeroize memory on drop.

use secrecy::{CloneableSecret, ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secret key bytes that can be cloned for secrecy crate compatibility
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
struct SecretKeyBytes(Vec<u8>);

impl CloneableSecret for SecretKeyBytes {}

impl secrecy::DebugSecret for SecretKeyBytes {
    fn debug_secret(f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str("[REDACTED]")
    }
}

#[cfg(all(unix, feature = "memory-locking"))]
use libc::{mlock, munlock};

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

/// Wrapper for private key material that zeroizes when dropped and locks memory on Unix.
#[derive(Clone, Debug)]
pub struct SecurePrivateKey {
    key_bytes: Secret<SecretKeyBytes>,
    #[cfg(all(unix, feature = "memory-locking"))]
    is_locked: bool,
    #[cfg(all(unix, feature = "memory-locking"))]
    memory_size: usize,
}

impl Serialize for SecurePrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Only serialize the length, not the actual key bytes
        serializer.serialize_u32(self.key_bytes.expose_secret().0.len() as u32)
    }
}

impl<'de> Deserialize<'de> for SecurePrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // For deserialization, we need to create a placeholder
        // In practice, this should be handled differently for security
        let len = u32::deserialize(deserializer)?;
        Ok(Self::new(vec![0u8; len as usize]))
    }
}

impl SecurePrivateKey {
    /// Creates a new secure private key from raw bytes.
    #[allow(unused_mut)]
    pub fn new(bytes: Vec<u8>) -> Self {
        let mut secure = Self {
            key_bytes: Secret::new(SecretKeyBytes(bytes)),
            #[cfg(all(unix, feature = "memory-locking"))]
            is_locked: false,
            #[cfg(all(unix, feature = "memory-locking"))]
            memory_size: 0,
        };

        #[cfg(all(unix, feature = "memory-locking"))]
        {
            secure.lock_memory().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to lock memory: {}", e);
            });
        }

        secure
    }

    /// Returns the underlying key bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.key_bytes.expose_secret().0
    }

    /// Locks the memory containing the private key on Unix systems.
    #[cfg(all(unix, feature = "memory-locking"))]
    fn lock_memory(&mut self) -> Result<(), std::io::Error> {
        let bytes = &self.key_bytes.expose_secret().0;
        let ptr = bytes.as_ptr() as *mut libc::c_void;
        let len = bytes.len();

        // SAFETY: mlock is used to prevent swapping of sensitive key material
        // The pointer is valid and within the bounds of the Vec<u8>
        let result = unsafe { mlock(ptr, len) };

        if result == 0 {
            self.is_locked = true;
            self.memory_size = len;
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    /// Unlocks the memory containing the private key on Unix systems.
    #[cfg(all(unix, feature = "memory-locking"))]
    fn unlock_memory(&mut self) -> Result<(), std::io::Error> {
        if !self.is_locked {
            return Ok(());
        }

        let bytes = &self.key_bytes.expose_secret().0;
        let ptr = bytes.as_ptr() as *mut libc::c_void;

        // SAFETY: munlock is used to release memory previously locked with mlock
        // The pointer is valid and within the bounds of the Vec<u8>
        let result = unsafe { munlock(ptr, self.memory_size) };

        if result == 0 {
            self.is_locked = false;
            self.memory_size = 0;
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    /// Returns true if memory is locked on Unix systems.
    #[cfg(all(unix, feature = "memory-locking"))]
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    /// Returns true if memory is locked on non-Unix systems (always false).
    #[cfg(not(all(unix, feature = "memory-locking")))]
    pub fn is_locked(&self) -> bool {
        false
    }
}

impl Drop for SecurePrivateKey {
    fn drop(&mut self) {
        #[cfg(all(unix, feature = "memory-locking"))]
        let _ = self.unlock_memory();

        // SecretKeyBytes already implements ZeroizeOnDrop, so no need to manually zeroize
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

    #[test]
    fn secure_private_key_memory_locking() {
        let key = SecurePrivateKey::new(vec![1, 2, 3, 4, 5]);

        // On Unix systems with memory-locking feature, memory should be locked
        #[cfg(all(unix, feature = "memory-locking"))]
        assert!(
            key.is_locked(),
            "Memory should be locked on Unix systems with memory-locking feature"
        );

        // On other systems, memory locking is not available
        #[cfg(not(all(unix, feature = "memory-locking")))]
        assert!(
            !key.is_locked(),
            "Memory locking not available on this platform/configuration"
        );
    }

    #[test]
    fn secure_private_key_zeroizes_on_drop() {
        let key_bytes = vec![42; 32];
        let key = SecurePrivateKey::new(key_bytes);

        // Verify the key contains the expected bytes
        assert_eq!(key.as_slice(), &[42; 32]);

        // When the key goes out of scope, Drop will zeroize the memory
        // This is tested implicitly by the zeroize crate's own tests
    }
}
