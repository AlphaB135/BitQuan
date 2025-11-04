//! Integration tests for hybrid PoW validation.

use bitquan_consensus::{pow::*, ConsensusParams, PowSetParams};
use bitquan_types::BlockHeader;

fn dummy_header(algo_id: u8) -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 0,
        bits: 0x207fffff,
        nonce: 0,
        algo_id,
    }
}

#[test]
fn mainnet_rejects_randomx() {
    let params = PowSetParams::mainnet();

    // Pre-activation: only SHA-256d allowed
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 0));
    #[cfg(feature = "randomx")]
    assert!(!params.is_algo_allowed(PowAlgo::RandomX, 0));

    // Post "activation" (which is at u64::MAX, so never): still only SHA-256d
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 1_000_000));
    #[cfg(feature = "randomx")]
    assert!(!params.is_algo_allowed(PowAlgo::RandomX, 1_000_000));
}

#[test]
#[cfg(feature = "randomx")]
fn testnet_hybrid_activation() {
    let params = PowSetParams::testnet_hybrid();

    // Pre-activation: only default algo (SHA-256d) allowed
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 999));
    assert!(!params.is_algo_allowed(PowAlgo::RandomX, 999));

    // Post-activation: both algorithms allowed
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 1000));
    assert!(params.is_algo_allowed(PowAlgo::RandomX, 1000));
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 1001));
    assert!(params.is_algo_allowed(PowAlgo::RandomX, 1001));
}

#[test]
#[cfg(feature = "randomx")]
fn devnet_hybrid_from_genesis() {
    let params = PowSetParams::devnet_hybrid();

    // Both algorithms allowed from height 0
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 0));
    assert!(params.is_algo_allowed(PowAlgo::RandomX, 0));
    assert!(params.is_algo_allowed(PowAlgo::Sha256d, 100));
    assert!(params.is_algo_allowed(PowAlgo::RandomX, 100));
}

#[test]
fn sha256d_header_validation() {
    let engine = Sha256dEngine;
    let header = dummy_header(0);

    // Should be able to compute hash
    let hash = engine.pow_hash(&header).unwrap();
    assert_eq!(hash.len(), 32);

    // With very easy target (0x207fffff), nonce=0 might pass
    // This is just testing the mechanism works
    let _ = engine.verify(&header);
}

#[test]
#[cfg(feature = "randomx")]
fn randomx_header_validation() {
    let config = RandomXConfig::default();
    let engine = RandomXEngine::new(config);
    let header = dummy_header(1);

    // Should be able to compute hash
    let hash = engine.pow_hash(&header).unwrap();
    assert_eq!(hash.len(), 32);

    // Verification mechanism should work
    let _ = engine.verify(&header);
}

#[test]
fn algo_id_affects_serialization() {
    let h1 = dummy_header(0);
    let h2 = dummy_header(1);

    let bytes1 = h1.to_bytes();
    let bytes2 = h2.to_bytes();

    // Last byte should be different
    assert_ne!(bytes1[bytes1.len() - 1], bytes2[bytes2.len() - 1]);
    assert_eq!(bytes1[bytes1.len() - 1], 0);
    assert_eq!(bytes2[bytes2.len() - 1], 1);
}
