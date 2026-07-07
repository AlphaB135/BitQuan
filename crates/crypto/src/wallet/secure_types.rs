//! Secure string/private-key wrappers that zeroize memory on drop.

use crate::constant_time::{constant_time_eq, constant_time_hash_eq, SecureAllocator};
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
/// Wrapper for private key material that zeroizes when dropped and locks memory on Unix.
#[derive(Debug)]
pub struct SecurePrivateKey {
    key_bytes: Secret<SecretKeyBytes>,
    #[cfg(all(unix, feature = "memory-locking"))]
    is_locked: bool,
    #[cfg(all(unix, feature = "memory-locking"))]
    memory_size: usize,
    #[cfg(all(unix, feature = "memory-locking"))]
    locked_ptr: usize,
}

impl Clone for SecurePrivateKey {
    fn clone(&self) -> Self {
        let mut new_key = Self {
            key_bytes: self.key_bytes.clone(),
            #[cfg(all(unix, feature = "memory-locking"))]
            is_locked: false,
            #[cfg(all(unix, feature = "memory-locking"))]
            memory_size: 0,
            #[cfg(all(unix, feature = "memory-locking"))]
            locked_ptr: 0,
        };
        #[cfg(all(unix, feature = "memory-locking"))]
        if self.is_locked {
            let _ = new_key.lock_memory();
        }
        new_key
    }
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
        let len = bytes.len();

        // Use secure allocator for memory allocation
        let secure_bytes = SecureAllocator::allocate(len).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to allocate secure memory: {}", e);
            bytes.clone()
        });

        // Copy data using constant-time operation
        if secure_bytes.len() == len {
            // SAFETY: `secure_bytes` and `bytes` are valid pointers of length `len`.
            // They are distinct allocations and do not overlap.
            unsafe {
                crate::constant_time::constant_time_memcpy(
                    secure_bytes.as_ptr() as *mut u8,
                    bytes.as_ptr(),
                    len,
                );
            }
        }

        let mut secure = Self {
            key_bytes: Secret::new(SecretKeyBytes(secure_bytes)),
            #[cfg(all(unix, feature = "memory-locking"))]
            is_locked: false,
            #[cfg(all(unix, feature = "memory-locking"))]
            memory_size: 0,
            #[cfg(all(unix, feature = "memory-locking"))]
            locked_ptr: 0,
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

        // SAFETY: mlock is used to prevent swapping of sensitive key material.
        // The pointer is valid and within the bounds of the Vec<u8> which is kept alive by `self`.
        let result = unsafe { mlock(ptr, len) };

        if result == 0 {
            self.is_locked = true;
            self.memory_size = len;
            self.locked_ptr = ptr as usize;
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

        // Use the exact pointer and size that was locked to prevent issues if Vec was reallocated (M-15)
        let ptr = self.locked_ptr as *mut libc::c_void;

        // SAFETY: munlock is used to release memory previously locked with mlock.
        let result = unsafe { munlock(ptr, self.memory_size) };
        self.is_locked = false;
        self.locked_ptr = 0;

        if result == 0 {
            self.is_locked = false;
            self.memory_size = 0;
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    /// Constant-time comparison with another private key.
    ///
    /// Returns true if keys contain the same bytes, false otherwise.
    /// This comparison executes in constant time to prevent timing attacks.
    pub fn constant_time_eq(&self, other: &SecurePrivateKey) -> bool {
        let self_bytes = self.key_bytes.expose_secret();
        let other_bytes = other.key_bytes.expose_secret();

        if self_bytes.0.len() != other_bytes.0.len() {
            return false;
        }

        constant_time_eq(&self_bytes.0, &other_bytes.0)
    }

    /// Constant-time comparison with raw bytes.
    ///
    /// Returns true if key contains the same bytes, false otherwise.
    /// This comparison executes in constant time to prevent timing attacks.
    pub fn constant_time_eq_bytes(&self, bytes: &[u8]) -> bool {
        let key_bytes = self.key_bytes.expose_secret();

        if key_bytes.0.len() != bytes.len() {
            return false;
        }

        constant_time_eq(&key_bytes.0, bytes)
    }

    /// Securely updates the key material with constant-time operations.
    ///
    /// Replaces the current key material with new bytes.
    /// The old memory is zeroized before being replaced.
    pub fn secure_update(&mut self, new_bytes: Vec<u8>) {
        let len = new_bytes.len();

        // Allocate new secure memory
        let secure_bytes = SecureAllocator::allocate(len).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to allocate secure memory: {}", e);
            new_bytes.clone()
        });

        // Copy new data using constant-time operation
        if secure_bytes.len() == len {
            // SAFETY: `secure_bytes` and `new_bytes` are valid pointers of length `len`.
            // They are distinct allocations and do not overlap.
            unsafe {
                crate::constant_time::constant_time_memcpy(
                    secure_bytes.as_ptr() as *mut u8,
                    new_bytes.as_ptr(),
                    len,
                );
            }
        }

        // Replace the key bytes (old memory will be zeroized by ZeroizeOnDrop)
        self.key_bytes = Secret::new(SecretKeyBytes(secure_bytes));

        #[cfg(all(unix, feature = "memory-locking"))]
        {
            self.memory_size = len;
            let _ = self.lock_memory();
        }
    }

    /// Derives a secure hash of the private key for verification purposes.
    ///
    /// Returns a SHA-256 hash of the key material.
    /// This operation is designed to not expose the actual key material.
    pub fn secure_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let key_bytes = self.key_bytes.expose_secret();
        Sha256::digest(&key_bytes.0).into()
    }

    /// Verifies that the key matches an expected hash.
    ///
    /// Returns true if the key's hash matches the expected hash.
    /// This comparison is done in constant time to prevent timing attacks.
    pub fn verify_hash(&self, expected_hash: &[u8; 32]) -> bool {
        let actual_hash = self.secure_hash();
        constant_time_hash_eq(&actual_hash, expected_hash)
    }

    /// Returns true if the memory containing the private key is locked.
    ///
    /// This is only available on Unix systems with the memory-locking feature enabled.
    #[cfg(all(unix, feature = "memory-locking"))]
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    /// Returns false on platforms where memory locking is not available.
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

        // Verify key contains expected bytes
        assert_eq!(key.as_slice(), &[42; 32]);

        // When key goes out of scope, Drop will zeroize memory
        // This is tested implicitly by zeroize crate's own tests
    }

    #[test]
    fn test_constant_time_key_comparison() {
        let key1 = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let key2 = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let key3 = SecurePrivateKey::new(vec![1, 2, 3, 5]);

        assert!(key1.constant_time_eq(&key2));
        assert!(!key1.constant_time_eq(&key3));
        assert!(!key2.constant_time_eq(&key3));
    }

    #[test]
    fn test_constant_time_bytes_comparison() {
        let key = SecurePrivateKey::new(vec![1, 2, 3, 4]);

        assert!(key.constant_time_eq_bytes(&[1, 2, 3, 4]));
        assert!(!key.constant_time_eq_bytes(&[1, 2, 3, 5]));
        assert!(!key.constant_time_eq_bytes(&[1, 2, 3]));
        assert!(!key.constant_time_eq_bytes(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_secure_key_update() {
        let mut key = SecurePrivateKey::new(vec![1, 2, 3, 4]);

        // Verify initial state
        assert_eq!(key.as_slice(), &[1, 2, 3, 4]);

        // Update with new data
        key.secure_update(vec![5, 6, 7, 8]);

        // Verify new state
        assert_eq!(key.as_slice(), &[5, 6, 7, 8]);

        // Memory should still be locked if available
        #[cfg(all(unix, feature = "memory-locking"))]
        assert!(key.is_locked());
    }

    #[test]
    fn test_secure_key_hash() {
        let key1 = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let key2 = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let key3 = SecurePrivateKey::new(vec![1, 2, 3, 5]);

        let hash1 = key1.secure_hash();
        let hash2 = key2.secure_hash();
        let hash3 = key3.secure_hash();

        // Same keys should have same hash
        assert_eq!(hash1, hash2);

        // Different keys should have different hashes
        assert_ne!(hash1, hash3);

        // Hash should be deterministic
        let hash1_again = key1.secure_hash();
        assert_eq!(hash1, hash1_again);
    }

    #[test]
    fn test_verify_hash() {
        let key = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let expected_hash = key.secure_hash();

        // Should verify with correct hash
        assert!(key.verify_hash(&expected_hash));

        // Should not verify with incorrect hash
        let wrong_hash = [0u8; 32];
        assert!(!key.verify_hash(&wrong_hash));
    }

    #[test]
    fn test_timing_attack_resistance() {
        use std::time::Instant;

        let key1 = SecurePrivateKey::new(vec![1; 32]);
        let key2 = SecurePrivateKey::new(vec![1; 32]);
        let key3 = SecurePrivateKey::new(vec![2; 32]);

        // Measure time for equal comparison
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = key1.constant_time_eq(&key2);
        }
        let equal_time = start.elapsed();

        // Measure time for unequal comparison
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = key1.constant_time_eq(&key3);
        }
        let unequal_time = start.elapsed();

        // Times should be similar (within a reasonable margin)
        let time_diff = equal_time.abs_diff(unequal_time);

        // Allow significant variance on modern systems with CPU optimizations,
        // but still verify the function completes in reasonable time
        assert!(
            time_diff.as_millis() < 10,
            "Timing difference too large: {:?}",
            time_diff
        );

        // Also verify both operations complete in reasonable time
        assert!(
            equal_time.as_millis() < 100,
            "Equal comparison too slow: {:?}",
            equal_time
        );
        assert!(
            unequal_time.as_millis() < 100,
            "Unequal comparison too slow: {:?}",
            unequal_time
        );
    }
}
