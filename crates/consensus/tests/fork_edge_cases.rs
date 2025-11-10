//! Fork choice edge case tests

use bitquan_consensus::fork::{ForkChoice, ForkError};
use bitquan_consensus::pow::header_hash;
use bitquan_types::BlockHeader;

fn make_header(prev: [u8; 32], bits: u32, nonce: u64) -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_block: prev,
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 0,
        bits,
        nonce,
        algo_id: 0,
    }
}

#[test]
fn test_tie_breaking_by_timestamp() {
    let mut fc = ForkChoice::new();

    let genesis = make_header([0u8; 32], 0x207fffff, 0);
    fc.add_genesis(genesis.clone()).expect("Failed to add genesis block");
    let genesis_hash = header_hash(&genesis);

    // Chain A: timestamp 100
    let mut a1 = make_header(genesis_hash, 0x207fffff, 1);
    a1.time = 100;
    fc.add_block(a1).expect("Failed to add block a1");

    // Chain B: earlier timestamp 50 - should win
    let mut b1 = make_header(genesis_hash, 0x207fffff, 2);
    b1.time = 50;
    let (is_new_tip, reorg) = fc.add_block(b1).expect("Failed to add block b1");

    assert!(is_new_tip, "Earlier timestamp should win tie-break");
    assert!(reorg.is_some());
}

#[test]
fn test_deep_reorg_rejected() {
    let mut fc = ForkChoice::with_max_reorg(5);

    let genesis = make_header([0u8; 32], 0x207fffff, 0);
    fc.add_genesis(genesis.clone()).expect("Failed to add genesis block");
    let mut prev = header_hash(&genesis);

    // Build chain A with 10 blocks
    for i in 1..=10 {
        let header = make_header(prev, 0x207fffff, i);
        fc.add_block(header.clone()).expect("Failed to add block to chain A");
        prev = header_hash(&header);
    }

    // Try competing chain B from genesis with 11 blocks (would need 10-block reorg)
    prev = header_hash(&genesis);
    for i in 100..=110 {
        let header = make_header(prev, 0x207fffff, i);
        prev = header_hash(&header);

        if i == 110 {
            // This should be rejected as reorg depth would be 10 > max 5
            let result = fc.add_block(header);
            assert!(matches!(result, Err(ForkError::ReorgTooDeep(_, _))));
        } else {
            // Earlier blocks should add fine as alternative chain
            let _ = fc.add_block(header);
        }
    }
}

#[test]
fn test_reorg_depth_tracking() {
    let mut fc = ForkChoice::new();

    let genesis = make_header([0u8; 32], 0x207fffff, 0);
    fc.add_genesis(genesis.clone()).expect("Failed to add genesis block");
    let genesis_hash = header_hash(&genesis);

    // Chain A: 3 blocks
    let mut prev_a = genesis_hash;
    for i in 1..=3 {
        let header = make_header(prev_a, 0x207fffff, i);
        fc.add_block(header.clone()).expect("Failed to add block to chain A");
        prev_a = header_hash(&header);
    }

    assert_eq!(fc.last_reorg_depth, 0);

    // Chain B: 4 blocks from genesis (triggers reorg of depth 3)
    let mut prev_b = genesis_hash;
    for i in 10..=13 {
        let header = make_header(prev_b, 0x207fffff, i);
        let (is_tip, _reorg) = fc.add_block(header.clone()).expect("Failed to add block to chain B");
        prev_b = header_hash(&header);

        if is_tip && i == 13 {
            assert_eq!(fc.last_reorg_depth, 3);
        }
    }
}

#[test]
fn test_invalid_block_marking() {
    let mut fc = ForkChoice::new();

    let hash = [42u8; 32];
    assert!(fc.is_invalid(&hash).is_none());

    fc.mark_invalid(hash, "Invalid PoW".to_string());
    assert_eq!(fc.is_invalid(&hash), Some(&"Invalid PoW".to_string()));
}

#[test]
fn test_reorg_over_100_blocks() {
    // Test that default max reorg (100) works
    let mut fc = ForkChoice::new();

    let genesis = make_header([0u8; 32], 0x207fffff, 0);
    fc.add_genesis(genesis.clone()).expect("Failed to add genesis block");
    let genesis_hash = header_hash(&genesis);

    // Chain A: 150 blocks
    let mut prev_a = genesis_hash;
    for i in 1..=150 {
        let header = make_header(prev_a, 0x207fffff, i);
        fc.add_block(header.clone()).expect("Failed to add block to chain A");
        prev_a = header_hash(&header);
    }

    // Try chain B: 160 blocks (would need 150-block reorg, exceeds 100)
    let mut prev_b = genesis_hash;
    for i in 1000..=1159 {
        let header = make_header(prev_b, 0x207fffff, i);
        prev_b = header_hash(&header);

        if i == 1159 {
            // This should be rejected as reorg depth would be 150 > max 100
            let result = fc.add_block(header);
            assert!(matches!(result, Err(ForkError::ReorgTooDeep(_, _))));
        } else {
            // Earlier blocks should add fine as alternative chain
            let _ = fc.add_block(header);
        }
    }
}
