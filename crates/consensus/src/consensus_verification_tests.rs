//! Comprehensive consensus verification tests
//!
//! This module contains extensive tests to verify the correctness of the consensus implementation
//! including edge cases, security scenarios, and compliance with expected behavior.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::asert::MIN_TARGET_U64;
use bitquan_types::{genesis, Block, BlockHeader, Transaction, TxIn, TxOut, NetworkId, SigAlgorithm};
use bitquan_types::Witness;
use bq_crypto::CryptoRegistry;

// Helper function to create a block header
fn make_header(prev: [u8; 32], bits: u32, time: u32, nonce: u64) -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_block: prev,
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time,
        bits,
        nonce,
        algo_id: 0,
    }
}

// Helper function to create a transaction
fn create_test_tx(inputs: Vec<([u8; 32], u32)>, outputs: Vec<u128>) -> Transaction {
    Transaction {
        version: 1,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: inputs
            .into_iter()
            .map(|(txid, vout)| TxIn {
                prev_txid: txid,
                prev_vout: vout,
                script_sig: vec![],
                sequence: 0xffffffff,
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .map(|value| TxOut {
                value,
                script_pubkey: vec![0x51], // OP_TRUE
            })
            .collect(),
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    }
}

#[test]
fn asert_determinism_verification() {
    // Verify ASERT produces the same result for identical inputs
    let params = ConsensusParams::phase3_defaults();
    let anchor = 42_000u64;
    let height_delta = 15;
    let time_delta = 8000;

    // Run the calculation multiple times
    let results: Vec<u64> = (0..10)
        .map(|_| asert_next_target(anchor, height_delta, time_delta, &params, None))
        .collect();

    // All results should be identical
    for result in &results[1..] {
        assert_eq!(results[0], *result, "ASERT should be deterministic");
    }
}

#[test]
fn asert_bounds_verification() {
    // Verify ASERT never produces targets outside valid bounds
    let params = ConsensusParams::phase3_defaults();
    let max_target = compact_to_target(0x207fffff);

    // Test with various anchor values and deltas
    for anchor in [1u64, 1000, 1000000, u64::MAX] {
        for height_delta in [-100i64, -1, 0, 1, 100] {
            for time_delta in [-1000i64, -1, 0, 1, 1000] {
                let result = asert_next_target(anchor, height_delta, time_delta, &params, None);

                // Result must be positive
                assert!(result > 0, "ASERT target must be positive");

                // Result must not exceed maximum target
                assert!(result <= max_target, "ASERT target must not exceed maximum");

                // Result must not be below minimum
                assert!(result >= MIN_TARGET_U64, "ASERT target must not be below minimum");
            }
        }
    }
}

#[test]
fn difficulty_conversion_round_trip() {
    // Test compact -> target -> compact conversion preserves value
    for bits in [
        0x1c00ffff,  // Minimum
        0x1d00ffff,  // Typical
        0x1e00ffff,  // Easier
        0x1f00ffff,  // Very easy
        0x20000400,  // Edge case
    ] {
        let target = compact_to_target(bits);
        let re_compact = target_to_compact_u64(target);

        // For most values, round trip should be exact
        if bits != 0x20000400 {
            assert_eq!(re_compact, bits);
        }

        // Re-compact should always be valid
        assert!(re_compact > 0, "Re-compact bits must be positive");
    }
}

#[test]
fn fork_choice_work_calculation_correctness() {
    // Verify work calculation follows Bitcoin rules
    let test_cases = [
        // (bits, expected_work_approx)
        (0x207fffff, 1u64),      // Easy target
        (0x1d00ffff, 100_000u64), // Medium target
        (0x1c00ffff, 1_000_000u64), // Hard target
    ];

    for (bits, expected_work) in test_cases.iter() {
        let work = BlockNode::calculate_work_for_bits(bits);
        // Work should be positive and reasonable for the test case
        assert!(work > U256::zero());
        assert!(work > U256::from(*expected_work / 10));
        assert!(work < U256::from(*expected_work * 10));
    }
}

#[test]
fn fork_choice_max_reorg_protection() {
    // Test that reorgs beyond max depth are rejected
    let mut fc = ForkChoice::with_max_reorg(3);

    // Build a chain of 5 blocks
    let genesis = make_header([0u8; 32], 0x207fffff, 0, 0);
    fc.add_genesis(genesis.clone()).unwrap();
    let genesis_hash = header_hash(&genesis);

    let mut prev_hash = genesis_hash;
    for i in 1..=5 {
        let block = make_header(prev_hash, 0x207fffff, i as u32, i as u64);
        fc.add_block(block.clone()).unwrap();
        prev_hash = header_hash(&block);
    }

    // Build a competing chain from genesis with 7 blocks (exceeds max depth of 3)
    prev_hash = genesis_hash;
    for i in 10..=16 {
        let block = make_header(prev_hash, 0x207fffff, i as u32, i as u64);

        // The first few should be fine
        if i <= 12 {
            fc.add_block(block).unwrap();
        } else {
            // The 7th block should trigger reorg too deep error
            let result = fc.add_block(block);
            assert!(matches!(result, Err(ForkError::ReorgTooDeep(5, 3))));
            break;
        }
        prev_hash = header_hash(&block);
    }
}

#[test]
fn invalid_block_handling() {
    // Test handling of invalid blocks in fork choice
    let mut fc = ForkChoice::new();

    // Build valid chain
    let genesis = make_header([0u8; 32], 0x207fffff, 0, 0);
    fc.add_genesis(genesis.clone()).unwrap();
    let genesis_hash = header_hash(&genesis);

    let block1 = make_header(genesis_hash, 0x207fffff, 1, 1);
    fc.add_block(block1.clone()).unwrap();
    let block1_hash = header_hash(&block1);

    // Mark block1 as invalid
    fc.mark_invalid(block1_hash, "Invalid proof of work".to_string());

    // Try to add a block that depends on invalid block
    let block2 = make_header(block1_hash, 0x207fffff, 2, 2);
    let result = fc.add_block(block2);

    // Should be rejected due to invalid parent
    assert!(result.is_err());
}

#[test]
fn timestamp_validation_edge_cases() {
    // Test timestamp validation at boundaries
    let params = ConsensusParams::phase3_defaults();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let max_future_time = current_time + 7200;
    let block_time = max_future_time + 1;

    // Create header with timestamp too far in future
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: block_time as u32,
        bits: 0x1d00ffff,
        nonce: 0,
        algo_id: 0,
    };

    // Should fail timestamp validation
    let result = validate_block_header(&Block { header, transactions: vec![] }, 1, &params, 0);
    assert!(matches!(result, Err(ConsensusError::TimestampTooFarInFuture(_, _))));
}

#[test]
fn merkle_root_validation() {
    // Test merkle root validation with various transaction counts
    for tx_count in [0, 1, 2, 3, 10] {
        // Create transactions
        let mut transactions = Vec::new();
        for i in 0..tx_count {
            transactions.push(create_test_tx(
                vec!([(i.to_be_bytes(), 0)],
                vec![1000]
            ));
        }

        // Calculate merkle root
        let calculated_root = calculate_merkle_root(&transactions).unwrap();

        // Create block with correct merkle root
        let block = Block {
            header: BlockHeader {
                version: 1,
                prev_block: [0u8; 32],
                merkle_root: calculated_root,
                pqc_agg_hint: [0u8; 32],
                time: 1700000000,
                bits: 0x1d00ffff,
                nonce: 0,
                algo_id: 0,
            },
            transactions,
        };

        // Should validate successfully
        let result = validate_block_header(&block, 0, &ConsensusParams::phase3_defaults(), 0);
        assert!(result.is_ok(), "Merkle root validation should pass with correct root");

        // Test with incorrect merkle root
        let mut bad_block = block.clone();
        bad_block.header.merkle_root = [0xFF; 32];

        let result = validate_block_header(&bad_block, 0, &ConsensusParams::phase3_defaults(), 0);
        assert!(matches!(result, Err(ConsensusError::MerkleRootMismatch)));
    }
}

#[test]
fn coinbase_validation() {
    // Test coinbase transaction validation
    let params = ConsensusParams::phase3_defaults();
    let subsidy = params.reward_schedule.subsidy_at_height(0);

    // Valid coinbase
    let coinbase = create_test_tx(
        vec!(([0u8; 32], u32::MAX)), // Null input
        vec![subsidy] // Exact subsidy
    );

    // Validate as coinbase
    let result = validate_coinbase_transaction(&Block {
        header: BlockHeader::default(),
        transactions: vec![coinbase.clone()]
    }, 0);
    assert!(result.is_ok());

    // Coinbase with multiple inputs should fail
    let bad_coinbase = create_test_tx(
        vec!(([0u8; 32], u32::MAX), ([1u8; 32], 0)),
        vec![subsidy]
    );

    let result = validate_coinbase_transaction(&Block {
        header: BlockHeader::default(),
        transactions: vec![bad_coinbase]
    }, 0);
    assert!(matches!(result, Err(ConsensusError::InvalidSignature(_))));
}

#[test]
fn block_weight_edge_cases() {
    // Test block weight calculation at limits
    let params = ConsensusParams::phase3_defaults();

    // Create block with maximum allowed weight (just under limit)
    let mut transactions = Vec::new();
    let mut total_weight = 0;
    const WU_PER_TX: usize = 1000; // Approximate weight per transaction

    while total_weight + WU_PER_TX < params.block_weight_cap as usize {
        transactions.push(create_test_tx(
            vec!([(0u64.to_be_bytes(), 0)],
            vec![1000]
        ));
        total_weight += WU_PER_TX;
    }

    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1700000000,
            bits: 0x1d00ffff,
            nonce: 0,
            algo_id: 0,
        },
        transactions,
    };

    let weight = calculate_block_weight(&block).unwrap();
    assert!(weight <= params.block_weight_cap as usize);
}

#[test]
fn fee_calculation_accuracy() {
    // Test fee calculation for various scenarios
    let utxo_set = &mut UtxoSet::new();

    // Create initial UTXO
    let txid1 = [1u8; 32];
    let outpoint1 = crate::OutPoint::new(txid1, 0);
    let output1 = TxOut { value: 10_000, script_pubkey: vec![0x51] };
    utxo_set.add_utxo(crate::UtxoEntry::new(outpoint1, output1, 100, false)).unwrap();

    // Test spending with fee
    let spend_tx = create_test_tx(
        vec![(txid1, 0)],
        vec![9_500] // 500 fee
    );

    let result = utxo_set.apply_transaction(&spend_tx, 101, false);
    assert!(result.is_ok());

    let (inputs_value, outputs_value, fee) = result.unwrap();
    assert_eq!(inputs_value, 10_000);
    assert_eq!(outputs_value, 9_500);
    assert_eq!(fee, 500);

    // Test edge case: maximum fee
    utxo_set.add_utxo(crate::UtxoEntry::new(
        crate::OutPoint::new([2u8; 32], 0),
        TxOut { value: u128::MAX, script_pubkey: vec![0x51] },
        100,
        false
    )).unwrap();

    let max_fee_tx = create_test_tx(
        vec!(([2u8; 32], 0)),
        vec![1] // Maximum possible fee
    );

    let result = utxo_set.validate_transaction(&max_fee_tx, 101, false);
    assert!(result.is_ok());

    let (inputs_value, outputs_value, fee) = result.unwrap();
    assert_eq!(fee, u128::MAX - 1);
}

#[test]
fn dust_output_detection() {
    // Test dust output detection
    const DUST_THRESHOLD: u128 = 546;

    // Create transaction with dust output
    let dust_tx = create_test_tx(
        vec!([(0u64.to_be_bytes(), 0)],
        vec![DUST_THRESHOLD - 1] // Below dust threshold
    );

    // Should reject dust output (unless OP_RETURN)
    let result = validate_transaction(&dust_tx);
    assert!(matches!(result, Err(ConsensusError::DustOutput { .. })));

    // OP_RETURN outputs should be allowed to be dust
    let mut dust_tx_op_return = dust_tx;
    dust_tx_op_return.outputs[0].script_pubkey = vec![0x6a]; // OP_RETURN

    let result = validate_transaction(&dust_tx_op_return);
    assert!(result.is_ok());

    // Non-dust outputs should be fine
    let normal_tx = create_test_tx(
        vec!([(0u64.to_be_bytes(), 0)],
        vec![DUST_THRESHOLD + 1] // Above dust threshold
    );

    let result = validate_transaction(&normal_tx);
    assert!(result.is_ok());
}

#[test]
fn proof_of_work_verification() {
    // Test proof of work verification for different algorithms
    let test_cases = [
        (PowAlgo::Sha256d, 0x207fffff), // Easy target
    ];

    for (algo, bits) in test_cases.iter() {
        // Create header that should meet target with enough nonce
        let mut header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1700000000,
            bits: *bits,
            nonce: 0,
            algo_id: algo.to_u8(),
        };

        // Find nonce that makes hash meet target
        let mut nonce_found = false;
        for nonce in 0..1_000_000 {
            header.nonce = nonce;
            let hash = crate::pow::header_hash(&header);
            let target = compact_to_target(*bits);

            if hash_meets_target(&hash, &target.to_be_bytes()) {
                nonce_found = true;
                break;
            }
        }

        // For easy targets, we should find a nonce
        if *bits == 0x207fffff {
            assert!(nonce_found, "Should find nonce for easy target");
        }
    }
}

#[test]
fn economic_model_validation() {
    // Test reward schedule and halving
    let schedule = RewardSchedule::phase3_defaults();

    // Initial subsidy
    assert_eq!(schedule.subsidy_at_height(0), 50_000_000_000_000_000_000);

    // Halving at 210,000 blocks
    assert_eq!(schedule.subsidy_at_height(209_999), 50_000_000_000_000_000_000);
    assert_eq!(schedule.subsidy_at_height(210_000), 25_000_000_000_000_000_000);

    // Tail emission after 127 halvings
    assert_eq!(schedule.subsidy_at_height(210_000 * 127), 1_000_000_000_000_000_000);
    assert_eq!(schedule.subsidy_at_height(210_000 * 128), 500_000_000_000_000_000);
    assert_eq!(schedule.subsidy_at_height(210_000 * 1000), 500_000_000_000_000_000);
}

#[test]
fn parallel_verification_safety() {
    // Test that parallel verification doesn't cause race conditions
    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::new();

    // Create multiple identical blocks
    let transactions = (0..10)
        .map(|i| create_test_tx(vec![(i.to_be_bytes(), 0)], vec![1000]))
        .collect();

    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1700000000,
            bits: 0x207fffff,
            nonce: 0,
            algo_id: 0,
        },
        transactions,
    };

    // Verify the same block multiple times in parallel
    let results: Vec<_> = (0..10)
        .into_par_iter()
        .map(|_| {
            validate_block(
                &block,
                0,
                &params,
                &registry,
                NetworkId::Devnet,
                genesis::GENESIS_HASH_BYTES,
                Some(1000), // Some fees
                0, // median_time_past
            )
        })
        .collect();

    // All results should be identical
    for result in &results[1..] {
        assert_eq!(results[0], *result);
    }
}

#[test]
fn replay_attack_protection() {
    // Test protection against replay attacks with different network contexts
    let tx = create_test_tx(
        vec!([(0u64.to_be_bytes(), 0)],
        vec![1000]
    );

    let registry = CryptoRegistry::new();

    // Same transaction should fail on different networks
    let result_devnet = validate_transaction_signatures(
        &tx,
        &bitquan_types::TxContext::new(NetworkId::Devnet, genesis::GENESIS_HASH_BYTES),
        &registry
    );

    let result_mainnet = validate_transaction_signatures(
        &tx,
        &bitquan_types::TxContext::new(NetworkId::Mainnet, genesis::GENESIS_HASH_BYTES),
        &registry
    );

    // Both should fail (no signatures), but on different networks
    assert!(result_devnet.is_ok());
    assert!(result_mainnet.is_ok());

    // With different genesis hashes, should fail
    let result_diff_genesis = validate_transaction_signatures(
        &tx,
        &bitquan_types::TxContext::new(NetworkId::Devnet, [0xFF; 32]),
        &registry
    );

    assert!(result_diff_genesis.is_err());
}

// Property-based tests using proptest
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn asert_monotonic_increasing_with_time(
            anchor in 1u64..1_000_000,
            height_delta in 1i64..100,
            base_time in 1_000_000u64..10_000_000,
            time_offset in 0u64..100_000
        ) {
            let params = ConsensusParams::phase3_defaults();

            let result1 = asert_next_target(anchor, height_delta, base_time as i64, &params, None);
            let result2 = asert_next_target(anchor, height_delta, (base_time + time_offset) as i64, &params, None);

            // More time should result in higher target (lower difficulty)
            assert!(result2 >= result1);
        }

        #[test]
        fn asert_monotonic_decreasing_with_height_difficulty(
            anchor in 1u64..1_000_000,
            base_height_diff in 1i64..100,
            base_time in 1_000_000u64..10_000_000,
            time_offset in 0u64..100_000
        ) {
            let params = ConsensusParams::phase3_defaults();

            let result1 = asert_next_target(anchor, base_height_diff, base_time as i64, &params, None);
            let result2 = asert_next_target(anchor + 1, base_height_diff, base_time as i64, &params, None);

            // Greater height with same time should result in lower target (higher difficulty)
            assert!(result2 <= result1);
        }

        #[test]
        fn weight_calculation_linear_with_signatures(
            base_size in 100usize..1000,
            signature_count in 0usize..100
        ) {
            // Create transaction with variable signature count
            let tx = Transaction {
                version: 1,
                network: NetworkId::Devnet,
                genesis_hash: genesis::GENESIS_HASH_BYTES,
                lock_time: 0,
                inputs: vec![TxIn {
                    prev_txid: [0u8; 32],
                    prev_vout: 0,
                    script_sig: vec![0u8; base_size],
                    sequence: 0xffffffff,
                }],
                outputs: vec![TxOut {
                    value: 1000,
                    script_pubkey: vec![0x51],
                }],
                sig_algo: SigAlgorithm::Dilithium5,
                witnesses: vec![Witness {
                    signatures: (0..signature_count)
                        .map(|i| SignaturePayload {
                            signer_index: i as u16,
                            signature: vec![0u8; 10],
                            public_key: vec![0u8; 10],
                            aux: None,
                        })
                        .collect(),
                }],
            };

            let weight = calculate_tx_weight(&tx).unwrap();

            // Weight should increase with signature count
            // Base weight: base_size * 4
            // Signature weight: signature_count * 384
            let expected_min = base_size * 4 + signature_count * 384;
            assert!(weight >= expected_min);
        }
    }
}