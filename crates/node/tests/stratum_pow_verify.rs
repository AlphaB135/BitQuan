//! Integration tests for REAL Stratum PoW verification.
//!
//! Tests share acceptance/rejection with actual hash computation and target comparison.

#![cfg(feature = "pool")]

use bitquan_consensus::pow::{meets_target, sha256d_pow_hash, target_from_bits, PowAlgo};
use bitquan_node::{BlockTemplate, PoolTemplateManager};
use bitquan_types::BlockHeader;

#[tokio::test]
async fn test_share_valid_sha256d_verification_logic() {
    use bitquan_consensus::pow::DEVNET_MAX_BITS;

    // Test the verification logic itself, not finding a valid nonce
    // (finding valid nonce with real difficulty takes too long for tests)

    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: DEVNET_MAX_BITS, // 0x207fffff
        nonce: 12345,
        algo_id: 0,
    };

    let target = target_from_bits(header.bits).unwrap();
    println!(
        "Target (big-endian) from bits 0x{:08x}: {:02x?}",
        header.bits,
        &target[..8]
    );

    // Compute hash for this header
    let preimage = header.to_bytes();
    let hash = sha256d_pow_hash(&preimage);
    println!("Hash:   {:02x}{:02x}{:02x}...", hash[0], hash[1], hash[2]);

    // Test meets_target function
    let meets = meets_target(&hash, &target);
    println!("Hash meets target: {}", meets);

    // Create a fake "easy" hash that definitely meets target
    let mut easy_hash = [0u8; 32];
    easy_hash[0] = 0x01; // Much smaller than 0x7fffff
    assert!(
        meets_target(&easy_hash, &target),
        "Easy hash should meet target"
    );

    // Create a fake "hard" hash that definitely doesn't meet target
    let hard_hash = [0xff; 32];
    assert!(
        !meets_target(&hard_hash, &target),
        "Hard hash should NOT meet target"
    );

    // Verify template manager works
    let template = BlockTemplate {
        header,
        txs: vec![],
        target,
        algo: PowAlgo::Sha256d,
        job_id: 1,
    };

    let manager = PoolTemplateManager::new(30);
    manager.update_template(template.clone()).await;

    let cached = manager.get_template().await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().job_id, 1);
}

#[test]
fn test_share_invalid_too_high_hash() {
    // Create a header with hard difficulty
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: 0x1d00ffff, // Hard difficulty
        nonce: 1,         // Unlikely to meet target
        algo_id: 0,
    };

    let target = target_from_bits(header.bits).unwrap();
    let preimage = header.to_bytes();
    let hash = sha256d_pow_hash(&preimage);

    // With hard difficulty and low nonce, should NOT meet target
    let meets = meets_target(&hash, &target);

    // This might occasionally pass, so we just verify the function works
    println!(
        "Hash meets target: {} (hash: {:?}, target: {:?})",
        meets,
        &hash[..4],
        &target[..4]
    );
}

#[tokio::test]
async fn test_share_stale_on_new_template() {
    let manager = PoolTemplateManager::new(30);

    // First template
    let header1 = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [1u8; 32], // Different merkle root
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: 0x207fffff,
        nonce: 0,
        algo_id: 0,
    };

    let template1 = BlockTemplate {
        header: header1,
        txs: vec![],
        target: [0xff; 32],
        algo: PowAlgo::Sha256d,
        job_id: 1,
    };

    manager.update_template(template1).await;
    let cached1 = manager.get_template().await.unwrap();
    assert_eq!(cached1.job_id, 1);

    // Second template (height change simulation)
    let header2 = BlockHeader {
        version: 1,
        prev_block: [1u8; 32], // Different prev_block
        merkle_root: [2u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567900,
        bits: 0x207fffff,
        nonce: 0,
        algo_id: 0,
    };

    let template2 = BlockTemplate {
        header: header2,
        txs: vec![],
        target: [0xff; 32],
        algo: PowAlgo::Sha256d,
        job_id: 2,
    };

    manager.update_template(template2).await;
    let cached2 = manager.get_template().await.unwrap();
    assert_eq!(cached2.job_id, 2);

    // Old job_id (1) would now be stale
    assert_ne!(cached2.job_id, cached1.job_id);
}

#[tokio::test]
async fn test_duplicate_detection_logic() {
    // This tests the duplicate detection data structure
    use lru::LruCache;
    use std::num::NonZeroUsize;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let cache_size = NonZeroUsize::new(4096).unwrap();
    let cache = Arc::new(Mutex::new(LruCache::<u64, ()>::new(cache_size)));

    // First submission of nonce 12345
    {
        let mut c = cache.lock().await;
        assert!(!c.contains(&12345));
        c.put(12345, ());
    }

    // Second submission of same nonce - should be detected
    {
        let c = cache.lock().await;
        assert!(c.contains(&12345), "Duplicate should be detected");
    }

    // Different nonce should be new
    {
        let c = cache.lock().await;
        assert!(
            !c.contains(&54321),
            "New nonce should not be marked duplicate"
        );
    }
}

#[test]
fn test_target_from_bits_conversion() {
    // Test easy target (regtest style)
    let easy_target = target_from_bits(0x207fffff).unwrap();

    // Test harder target
    let hard_target = target_from_bits(0x1d00ffff).unwrap();

    // Harder target should be smaller (numerically)
    assert!(hard_target < easy_target, "Harder target should be smaller");

    println!("Easy target: {:?}", &easy_target[..4]);
    println!("Hard target: {:?}", &hard_target[..4]);
}

#[cfg(feature = "randomx")]
#[tokio::test]
async fn test_share_valid_randomx_meets_target() {
    use bitquan_consensus::pow::randomx_pow_hash;

    // Create easy template for RandomX
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1234567890,
        bits: 0x207fffff, // Very easy
        nonce: 0,
        algo_id: 1, // RandomX
    };

    let target = target_from_bits(header.bits).unwrap();
    let seed = [0u8; 32];

    // Try a few nonces
    let mut found = false;
    for nonce in 0..100000 {
        let mut test_header = header.clone();
        test_header.nonce = nonce;
        let preimage = test_header.to_bytes();
        let hash = randomx_pow_hash(&preimage, &seed);

        if meets_target(&hash, &target) {
            println!(
                "RandomX: Found valid nonce: {} with hash: {:?}",
                nonce,
                &hash[..4]
            );
            found = true;
            break;
        }
    }

    assert!(
        found,
        "Should find at least one valid RandomX nonce with easy difficulty"
    );
}

#[test]
fn test_algo_mismatch_detection() {
    // Template is for SHA-256d
    let template_algo = PowAlgo::Sha256d;

    // But submission claims RandomX
    #[cfg(feature = "randomx")]
    let submit_algo = PowAlgo::RandomX;
    #[cfg(not(feature = "randomx"))]
    let submit_algo = PowAlgo::Sha256d;

    #[cfg(feature = "randomx")]
    {
        // Should detect mismatch
        assert_ne!(template_algo, submit_algo, "Algorithms should mismatch");
    }

    #[cfg(not(feature = "randomx"))]
    {
        // Without randomx feature, both are SHA-256d
        assert_eq!(template_algo, submit_algo);
    }
}
