//! Entropy sanity tests to verify secure randomness properties.

use bitquan_types::entropy::{fill_secure, secure_bytes, secure_u64};

#[test]
fn test_entropy_is_random() {
    let a = secure_bytes(32);
    let b = secure_bytes(32);
    assert_ne!(
        a, b,
        "Two sequential secure_bytes calls should produce different output"
    );
}

#[test]
fn test_secure_bytes_not_all_zeros() {
    let bytes = secure_bytes(32);
    assert!(
        bytes.iter().any(|&b| b != 0),
        "Secure random bytes should not be all zeros"
    );
}

#[test]
fn test_secure_bytes_not_all_same() {
    let bytes = secure_bytes(32);
    let first = bytes[0];
    let all_same = bytes.iter().all(|&b| b == first);
    assert!(
        !all_same,
        "Secure random bytes should not all be the same value"
    );
}

#[test]
fn test_fill_secure_changes_buffer() {
    let mut buf = [0u8; 32];
    fill_secure(&mut buf);

    assert!(
        buf.iter().any(|&b| b != 0),
        "fill_secure should change buffer from all zeros"
    );
}

#[test]
fn test_fill_secure_different_calls() {
    let mut buf1 = [0u8; 32];
    let mut buf2 = [0u8; 32];

    fill_secure(&mut buf1);
    fill_secure(&mut buf2);

    assert_ne!(
        buf1, buf2,
        "Two fill_secure calls should produce different output"
    );
}

#[test]
fn test_secure_u64_different_values() {
    let v1 = secure_u64();
    let v2 = secure_u64();
    assert_ne!(
        v1, v2,
        "Two secure_u64 calls should produce different values"
    );
}

#[test]
fn test_secure_u64_nonzero() {
    let v = secure_u64();
    // With 2^64 possible values, getting 0 is 1/2^64 probability
    // This test will pass with overwhelming probability
    assert_ne!(
        v, 0,
        "secure_u64 should produce non-zero value (overwhelmingly likely)"
    );
}

#[test]
fn test_secure_bytes_various_lengths() {
    for len in [1, 8, 16, 32, 64, 128, 256] {
        let bytes = secure_bytes(len);
        assert_eq!(
            bytes.len(),
            len,
            "secure_bytes should produce correct length"
        );
        // Only check non-zero for lengths > 1, since a single byte has a 1/256 chance of being 0
        if len > 1 {
            assert!(
                bytes.iter().any(|&b| b != 0),
                "secure_bytes of length {} should not be all zeros",
                len
            );
        }
    }
}

#[test]
fn test_entropy_independence() {
    // Generate multiple random values and ensure they're independent
    let samples: Vec<_> = (0..10).map(|_| secure_bytes(32)).collect();

    // Check that not all samples are the same
    let first = &samples[0];
    let all_same = samples.iter().all(|s| s == first);
    assert!(!all_same, "Multiple entropy samples should be different");

    // Check pairwise differences
    for i in 0..samples.len() {
        for j in (i + 1)..samples.len() {
            assert_ne!(
                samples[i], samples[j],
                "Samples {} and {} should be different",
                i, j
            );
        }
    }
}

#[test]
fn test_entropy_quality_basic_stats() {
    // Generate a large sample and check basic statistical properties
    let sample = secure_bytes(1000);

    // Count zeros
    let zero_count = sample.iter().filter(|&&b| b == 0).count();

    // Expect roughly 4 zeros out of 1000 bytes (1000/256 ≈ 3.9)
    // Allow wide margin for statistical variance
    assert!(
        zero_count < 20,
        "Too many zeros in random sample: {} (expected ~4)",
        zero_count
    );

    // Check that we have reasonable variety of byte values
    let mut byte_counts = [0usize; 256];
    for &byte in &sample {
        byte_counts[byte as usize] += 1;
    }

    let unique_values = byte_counts.iter().filter(|&&count| count > 0).count();
    assert!(
        unique_values > 200,
        "Insufficient byte value variety: {} unique values out of 256",
        unique_values
    );
}
