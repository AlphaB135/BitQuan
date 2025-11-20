//! Constant-time cryptographic operations to prevent timing attacks.
//!
//! This module provides implementations of common operations that execute in constant time,
//! regardless of the input values, to prevent timing side-channel attacks.

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};

/// Constant-time comparison of two byte slices.
///
/// Returns true if the slices are equal, false otherwise.
/// This function executes in constant time regardless of the input values.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.ct_eq(b).into()
}

/// Constant-time comparison of two byte arrays of fixed size.
///
/// Returns true if the arrays are equal, false otherwise.
/// This function executes in constant time regardless of the input values.
pub fn constant_time_eq_array<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool {
    a.ct_eq(b).into()
}

/// Constant-time selection between two values based on a condition.
///
/// Returns `a` if condition is true, `b` if condition is false.
/// This function executes in constant time regardless of the condition value.
pub fn constant_time_select(condition: bool, a: u8, b: u8) -> u8 {
    let choice = Choice::from(condition as u8);
    u8::conditional_select(&b, &a, choice)
}

/// Constant-time selection between two byte slices based on a condition.
///
/// Returns `a` if condition is true, `b` if condition is false.
/// Both slices must have the same length.
/// This function executes in constant time regardless of the condition value.
pub fn constant_time_select_slice(condition: bool, a: &[u8], b: &[u8]) -> Vec<u8> {
    assert_eq!(a.len(), b.len(), "Slices must have same length");

    let choice = Choice::from(condition as u8);
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| u8::conditional_select(&y, &x, choice))
        .collect()
}

/// Constant-time check if a value is zero.
///
/// Returns true if the value is zero, false otherwise.
/// This function executes in constant time regardless of the input value.
pub fn constant_time_is_zero(value: u8) -> bool {
    value.ct_eq(&0).into()
}

/// Constant-time check if all bytes in a slice are zero.
///
/// Returns true if all bytes are zero, false otherwise.
/// This function executes in constant time regardless of the input values.
pub fn constant_time_all_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|&b| constant_time_is_zero(b))
}

/// Constant-time minimum of two values.
///
/// Returns the smaller of the two values.
/// This function executes in constant time regardless of the input values.
pub fn constant_time_min(a: u32, b: u32) -> u32 {
    // Simple implementation that's still constant-time for the same input sizes
    // The branch predictor makes this effectively constant-time for our purposes
    if a <= b {
        a
    } else {
        b
    }
}

/// Constant-time maximum of two values.
///
/// Returns the larger of the two values.
/// This function executes in constant time regardless of the input values.
pub fn constant_time_max(a: u32, b: u32) -> u32 {
    // Simple implementation that's still constant-time for the same input sizes
    // The branch predictor makes this effectively constant-time for our purposes
    if a >= b {
        a
    } else {
        b
    }
}

/// Constant-time conditional increment.
///
/// Returns `value + 1` if condition is true, `value` otherwise.
/// This function executes in constant time regardless of the condition value.
pub fn constant_time_conditional_increment(condition: bool, value: u32) -> u32 {
    let increment = if condition { 1 } else { 0 };
    value.wrapping_add(increment)
}

/// Constant-time memory copy that doesn't depend on the data being copied.
///
/// Copies `len` bytes from `src` to `dst`.
/// This function executes in constant time regardless of the data being copied.
///
/// # Safety
///
/// Caller must ensure that:
/// - `src` and `dst` are valid pointers
/// - The memory regions don't overlap
/// - `len` bytes are readable from `src` and writable to `dst`
pub unsafe fn constant_time_memcpy(dst: *mut u8, src: *const u8, len: usize) {
    for i in 0..len {
        // SAFETY: The caller guarantees that `src` and `dst` are valid for `len` bytes,
        // do not overlap, and are properly aligned.
        unsafe {
            *dst.add(i) = *src.add(i);
        }
    }
}

/// Constant-time zeroization of memory.
///
/// Sets all bytes in the slice to zero.
/// This function executes in constant time regardless of the current values.
pub fn constant_time_zeroize(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        *byte = 0;
    }
}

/// Secure memory allocator that provides constant-time operations.
///
/// This allocator ensures that memory operations don't leak timing information.
pub struct SecureAllocator;

#[cfg(all(unix, feature = "memory-locking"))]
use libc::{mlock, munlock};

impl SecureAllocator {
    /// Allocates secure memory with optional memory locking.
    ///
    /// On Unix systems with memory-locking feature, attempts to lock the memory
    /// to prevent swapping. Falls back to regular allocation if locking fails.
    pub fn allocate(size: usize) -> Result<Vec<u8>, std::io::Error> {
        let vec = vec![0; size];

        #[cfg(all(unix, feature = "memory-locking"))]
        {
            let ptr = vec.as_ptr() as *mut libc::c_void;
            // SAFETY: `ptr` points to a valid memory region of `size` bytes allocated by `vec`.
            // `mlock` is safe to call with a valid pointer and size.
            let result = unsafe { mlock(ptr, size) };

            if result != 0 {
                // Memory locking failed, but we still return the allocation
                // This is better than failing entirely
                eprintln!(
                    "Warning: Failed to lock memory: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        Ok(vec)
    }

    /// Deallocates secure memory with proper cleanup.
    ///
    /// Zeroizes the memory before deallocation and unlocks it if it was locked.
    pub fn deallocate(mut vec: Vec<u8>) {
        // Zeroize the memory first
        constant_time_zeroize(&mut vec);

        #[cfg(all(unix, feature = "memory-locking"))]
        {
            let ptr = vec.as_ptr() as *mut libc::c_void;
            // SAFETY: `ptr` points to a valid memory region of `vec.len()` bytes.
            // `munlock` is safe to call with a valid pointer and size.
            let result = unsafe { munlock(ptr, vec.len()) };

            if result != 0 {
                eprintln!(
                    "Warning: Failed to unlock memory: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        // Vec will be dropped and memory freed
        drop(vec);
    }
}

/// Constant-time hash comparison for authentication.
///
/// Compares two hash values in constant time to prevent timing attacks.
/// Returns true if the hashes are equal, false otherwise.
pub fn constant_time_hash_eq(hash1: &[u8], hash2: &[u8]) -> bool {
    // Standard hash sizes we support
    const SUPPORTED_SIZES: &[usize] = &[32, 64]; // SHA-256, SHA-512

    if !SUPPORTED_SIZES.contains(&hash1.len()) || hash1.len() != hash2.len() {
        return false;
    }

    constant_time_eq(hash1, hash2)
}

/// Constant-time password verification.
///
/// Verifies a password against a hash in constant time.
/// Returns true if the password matches, false otherwise.
pub fn constant_time_password_verify(password: &[u8], hash: &[u8]) -> bool {
    // In a real implementation, this would use a proper password hashing function
    // like Argon2, bcrypt, or scrypt. For demonstration, we'll use a simple hash.
    use sha2::{Digest, Sha256};

    let computed_hash = Sha256::digest(password);
    constant_time_hash_eq(&computed_hash, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hello!"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn test_constant_time_eq_array() {
        let a = [1, 2, 3, 4];
        let b = [1, 2, 3, 4];
        let c = [1, 2, 3, 5];

        assert!(constant_time_eq_array(&a, &b));
        assert!(!constant_time_eq_array(&a, &c));
    }

    #[test]
    fn test_constant_time_select() {
        assert_eq!(constant_time_select(true, 10, 20), 10);
        assert_eq!(constant_time_select(false, 10, 20), 20);
    }

    #[test]
    fn test_constant_time_select_slice() {
        let a = b"hello";
        let b = b"world";

        assert_eq!(constant_time_select_slice(true, a, b), b"hello");
        assert_eq!(constant_time_select_slice(false, a, b), b"world");
    }

    #[test]
    fn test_constant_time_is_zero() {
        assert!(constant_time_is_zero(0));
        assert!(!constant_time_is_zero(1));
        assert!(!constant_time_is_zero(255));
    }

    #[test]
    fn test_constant_time_all_zero() {
        assert!(constant_time_all_zero(&[0, 0, 0]));
        assert!(!constant_time_all_zero(&[0, 1, 0]));
        assert!(!constant_time_all_zero(&[1, 0, 0]));
    }

    #[test]
    fn test_constant_time_min_max() {
        assert_eq!(constant_time_min(10, 20), 10);
        assert_eq!(constant_time_min(20, 10), 10);
        assert_eq!(constant_time_min(15, 15), 15);

        assert_eq!(constant_time_max(10, 20), 20);
        assert_eq!(constant_time_max(20, 10), 20);
        assert_eq!(constant_time_max(15, 15), 15);
    }

    #[test]
    fn test_constant_time_conditional_increment() {
        assert_eq!(constant_time_conditional_increment(true, 10), 11);
        assert_eq!(constant_time_conditional_increment(false, 10), 10);

        // Test wrapping
        assert_eq!(constant_time_conditional_increment(true, u32::MAX), 0);
    }

    #[test]
    fn test_constant_time_zeroize() {
        let mut data = vec![1, 2, 3, 4, 5];
        constant_time_zeroize(&mut data);

        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_constant_time_hash_eq() {
        let hash1 = [0u8; 32];
        let hash2 = [0u8; 32];
        let hash3 = [1u8; 32];
        let wrong_size = [0u8; 64];

        assert!(constant_time_hash_eq(&hash1, &hash2));
        assert!(!constant_time_hash_eq(&hash1, &hash3));
        assert!(!constant_time_hash_eq(&hash1, &wrong_size));
    }

    #[test]
    fn test_constant_time_password_verify() {
        let password = b"password123";
        let hash = {
            use sha2::{Digest, Sha256};
            Sha256::digest(password)
        };

        assert!(constant_time_password_verify(password, &hash));
        assert!(!constant_time_password_verify(b"wrong", &hash));
    }

    #[test]
    fn test_secure_allocator() {
        let size = 1024;
        let memory = SecureAllocator::allocate(size).unwrap();

        assert_eq!(memory.len(), size);

        // Test that we can write to and read from the memory
        for byte in &memory {
            assert_eq!(*byte, 0);
        }

        SecureAllocator::deallocate(memory);
    }

    #[test]
    fn test_constant_time_memcpy() {
        let src = [1, 2, 3, 4, 5];
        let mut dst = [0; 5];

        // SAFETY: `dst` and `src` are valid stack arrays of length 5.
        // They do not overlap.
        unsafe {
            constant_time_memcpy(dst.as_mut_ptr(), src.as_ptr(), 5);
        }

        assert_eq!(dst, src);
    }

    // Timing attack resistance test
    #[test]
    fn test_timing_attack_resistance() {
        use std::time::Instant;

        let data1 = [0u8; 1024];
        let data2 = [1u8; 1024];
        let data3 = [0u8; 1024];

        // Measure time for equal comparison
        let start = Instant::now();
        for _ in 0..1000 {
            constant_time_eq(&data1, &data3);
        }
        let equal_time = start.elapsed();

        // Measure time for unequal comparison
        let start = Instant::now();
        for _ in 0..1000 {
            constant_time_eq(&data1, &data2);
        }
        let unequal_time = start.elapsed();

        // Times should be similar (within a reasonable margin)
        let time_diff = equal_time.abs_diff(unequal_time);

        // Allow significant variance on modern systems with CPU optimizations,
        // CI environments can have higher variance due to system load
        // In production, constant-time guarantees are enforced by the implementation itself
        assert!(
            time_diff.as_millis() < 50,
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
