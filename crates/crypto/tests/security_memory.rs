//! Security tests for memory locking and secure key handling.

use bq_crypto::wallet::secure_types::SecurePrivateKey;

#[test]
fn test_memory_locking_unix() {
    let key_data = vec![0x42; 32];
    let secure_key = SecurePrivateKey::new(key_data);

    // Verify the key contains the correct data
    assert_eq!(secure_key.as_slice(), &[0x42; 32]);

    // On Unix systems, memory should be locked
    #[cfg(unix)]
    {
        assert!(
            secure_key.is_locked(),
            "Private key memory should be locked on Unix"
        );
    }

    // On non-Unix systems, memory locking is not available
    #[cfg(not(unix))]
    {
        assert!(
            !secure_key.is_locked(),
            "Memory locking not available on this platform"
        );
    }
}

#[test]
fn test_multiple_keys_memory_locking() {
    let key1 = SecurePrivateKey::new(vec![1; 32]);
    let key2 = SecurePrivateKey::new(vec![2; 32]);
    let key3 = SecurePrivateKey::new(vec![3; 32]);

    // All keys should contain their respective data
    assert_eq!(key1.as_slice(), &[1; 32]);
    assert_eq!(key2.as_slice(), &[2; 32]);
    assert_eq!(key3.as_slice(), &[3; 32]);

    // All keys should be locked on Unix systems
    #[cfg(unix)]
    {
        assert!(key1.is_locked());
        assert!(key2.is_locked());
        assert!(key3.is_locked());
    }
}

#[test]
fn test_key_zeroization_on_drop() {
    // This test verifies that keys are properly zeroized when dropped
    // The actual zeroization is handled by the Drop implementation and zeroize crate

    {
        let _key = SecurePrivateKey::new(vec![0xFF; 64]);
        // Key goes out of scope here and should be zeroized
    }

    // We can't directly test that memory was zeroized (since it's freed),
    // but this test ensures the Drop implementation runs without panicking
}

#[test]
fn test_empty_key_handling() {
    let empty_key = SecurePrivateKey::new(vec![]);

    assert_eq!(empty_key.as_slice(), &[0u8; 0]);

    #[cfg(unix)]
    {
        // Empty keys should still attempt to lock memory (may fail gracefully)
        // The important thing is that it doesn't panic
        let _is_locked = empty_key.is_locked();
    }
}

#[test]
fn test_large_key_memory_locking() {
    // Test with a larger key (4KB) to test memory locking with larger allocations
    let large_key_data = vec![0xAB; 4096];
    let large_key = SecurePrivateKey::new(large_key_data);

    assert_eq!(large_key.as_slice(), &[0xAB; 4096]);

    #[cfg(unix)]
    {
        assert!(
            large_key.is_locked(),
            "Large key memory should also be locked"
        );
    }
}
