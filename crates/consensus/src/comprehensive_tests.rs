//! Comprehensive test suite for consensus crate edge cases and security scenarios
//!
//! This module contains tests for:
//! - Edge cases in difficulty adjustment
//! - Reorg scenarios with invalid blocks
//! - Invalid block rejection patterns
//! - Fee calculation accuracy
//! - ASERT corner cases
//! - Fork choice edge cases

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::asert::MIN_TARGET_U64;
use bitquan_types::{genesis, Block, BlockHeader, Transaction, TxIn, TxOut, NetworkId, SigAlgorithm};
use bitquan_types::Witness;
use bq_crypto::CryptoRegistry;

#[test]
fn asert_extreme_time_delta() {
    // Test ASERT with maximum time deltas
    let params = ConsensusParams::phase3_defaults();
    let anchor = 1000u64;

    // Maximum positive delta (should give max target)
    let result_max = asert_next_target(anchor, 1, i64::MAX, &params, None);
    assert_eq!(result_max, compact_to_target(DEVNET_MAX_BITS));

    // Maximum negative delta (should give min target)
    let result_min = asert_next_target(anchor, 1, i64::MIN, &params, None);
    assert_eq!(result_min, MIN_TARGET_U64);
}

#[test]
fn asert_zero_height_delta() {
    // Test ASERT with zero height delta
    let params = ConsensusParams::phase3_defaults();
    let anchor = 1000u64;

    // Zero height delta should maintain anchor target
    let result = asert_next_target(anchor, 0, 6000, &params, None);
    assert_eq!(result, anchor); // Should be unchanged
}

#[test]
fn asert_negative_height_delta() {
    // Test ASERT with negative height delta (going backwards)
    let params = ConsensusParams::phase3_defaults();
    let anchor = 1000u64;

    // Going backwards in height should increase difficulty
    let result = asert_next_target(anchor, -10, 6000, &params, None);
    assert!(result < anchor); // Harder difficulty
}

#[test]
fn burst_guard_multiple_fast_blocks() {
    // Test burst guard with multiple fast blocks in sequence
    let params = ConsensusParams::phase3_defaults();
    let anchor = 10000u64;
    let window = params.difficulty.burst_guard_window as i64;
    let floor_ratio = params.difficulty.burst_guard_floor_ratio_fp as f64 / FP_SCALE as f64;
    let fast_time = ((params.difficulty.target_block_time as i64 * window) as f64 * floor_ratio * 0.8) as i64;

    let mut guard_state = BurstGuardState::default();

    // First fast block should trigger guard
    let result1 = asert_next_target(anchor, window, fast_time, &params,
        Some(GuardContext {
            state: &mut guard_state,
            current_height: window as u64,
            activation_height: 0,
        }));
    assert_eq!(result1, compact_to_target(DEVNET_MAX_BITS));
    assert!(guard_state.is_active());

    // Second fast block during cooldown should not trigger guard again
    let result2 = asert_next_target(anchor, window, fast_time, &params,
        Some(GuardContext {
            state: &mut guard_state,
            current_height: window as u64 + 1,
            activation_height: 0,
        }));
    assert_ne!(result2, compact_to_target(DEVNET_MAX_BITS));
    assert!(guard_state.is_active()); // Still active
}

#[test]
fn reorg_with_invalid_blocks() {
    let mut fc = ForkChoice::with_max_reorg(10);

    // Build main chain with an invalid block
    let genesis = make_header([0u8; 32], 0x207fffff, 0, 0);
    fc.add_genesis(genesis.clone()).unwrap();
    let genesis_hash = header_hash(&genesis);

    let block1 = make_header(genesis_hash, 0x207fffff, 1, 1);
    fc.add_block(block1.clone()).unwrap();
    let block1_hash = header_hash(&block1);

    // Mark block1 as invalid (simulating malicious behavior)
    fc.mark_invalid(block1_hash, "malicious double-spend".to_string());

    // Build competing chain from genesis
    let comp1 = make_header(genesis_hash, 0x207fffff, 10, 10);
    let comp2 = make_header(header_hash(&comp1), 0x207fffff, 11, 11);

    // Add competing blocks
    let (_is_tip1, _reorg1) = fc.add_block(comp1).unwrap();
    let (is_tip2, reorg2) = fc.add_block(comp2).unwrap();

    // Should reorg despite competing chain having less work
    // because main chain contains invalid blocks
    assert!(is_tip2);
    assert!(reorg2.is_some());

    let reorg_info = reorg2.unwrap();
    assert_eq!(reorg_info.disconnected_blocks.len(), 2); // Genesis + block1
}

#[test]
fn invalid_coinbase_rejection() {
    let mut block = create_valid_block();
    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::new();

    // Test 1: Coinbase missing entirely
    block.transactions = vec![];
    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, Some(0), 0);
    assert!(matches!(result, Err(ConsensusError::InvalidSignature(_))));

    // Test 2: Coinbase with wrong input
    let coinbase = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [1u8; 32], // Invalid - should be [0;32]
            prev_vout: 0xffffffff,
            script_sig: vec![0x01, 0x00],
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: params.reward_schedule.subsidy_at_height(0),
            script_pubkey: vec![0x76, 0xa9],
        }],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    };
    block.transactions = vec![coinbase];

    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, Some(0), 0);
    assert!(matches!(result, Err(ConsensusError::InvalidSignature(_))));

    // Test 3: Coinbase with invalid script length
    let mut coinbase = create_valid_coinbase();
    coinbase.inputs[0].script_sig = vec![0x01]; // Too short
    block.transactions = vec![coinbase];

    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, Some(0), 0);
    assert!(matches!(result, Err(ConsensusError::InvalidSignature(_))));
}

#[test]
fn test_fee_calculation_precision() {
    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::new();

    // Test with maximum fee values
    let total_fees = u128::MAX - 1; // Maximum without overflow
    let block_subsidy = params.reward_schedule.subsidy_at_height(0);

    // Create block with exact subsidy + fees
    let mut coinbase = create_valid_coinbase();
    coinbase.outputs[0].value = block_subsidy + total_fees;

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
        transactions: vec![coinbase],
    };

    // Should succeed with exact fee calculation
    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, Some(total_fees), 0);
    assert!(result.is_ok());
}

#[test]
fn test_fee_overflow_protection() {
    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::new();

    // Test fee overflow detection
    let total_fees = u128::MAX;
    let block_subsidy = params.reward_schedule.subsidy_at_height(0);

    // Create block that would cause overflow
    let mut coinbase = create_valid_coinbase();
    coinbase.outputs[0].value = block_subsidy + total_fees;

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
        transactions: vec![coinbase],
    };

    // Should detect potential overflow during fee validation
    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, Some(total_fees), 0);
    assert!(matches!(result, Err(ConsensusError::CoinbaseExceedsSubsidy)));
}

#[test]
fn test_validate_block_with_fees() {
    let params = ConsensusParams::phase3_defaults();
    let mut engine = ConsensusEngine::new(params.clone(), CryptoRegistry::new());
    let registry = CryptoRegistry::new();

    // Test exact fee calculation with validate_block_with_fees
    let total_fees = 1000; // 1 BQ fee
    let block_subsidy = params.reward_schedule.subsidy_at_height(0);

    // Create block with exact subsidy + fees
    let mut coinbase = create_valid_coinbase();
    coinbase.outputs[0].value = block_subsidy + total_fees;

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
        transactions: vec![coinbase],
    };

    // Should succeed with exact fee calculation using validate_block_with_fees
    let result = engine.validate_block_with_fees(&block, 0, total_fees, 0);
    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.block_subsidy, block_subsidy);
    assert_eq!(report.signature_count, 0);
}

#[test]
fn test_validate_block_with_fees_invalid_fees() {
    let params = ConsensusParams::phase3_defaults();
    let mut engine = ConsensusEngine::new(params.clone(), CryptoRegistry::new());

    // Test with incorrect fee amount
    let total_fees = 2000; // Incorrect fee
    let block_subsidy = params.reward_schedule.subsidy_at_height(0);

    // Create block with incorrect total
    let mut coinbase = create_valid_coinbase();
    coinbase.outputs[0].value = block_subsidy + 1000; // Only 1000 fees, not 2000

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
        transactions: vec![coinbase],
    };

    // Should fail due to fee mismatch
    let result = engine.validate_block_with_fees(&block, 0, total_fees, 0);
    assert!(result.is_err());
}

#[test]
fn dust_output_rejection() {
    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::new();

    // Create transaction with dust output
    let dust_tx = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0xffffffff,
            script_sig: vec![0x01, 0x00],
            sequence: 0xffffffff,
        }],
        outputs: vec![
            // Dust output (below threshold)
            TxOut {
                value: 500, // Below 546 threshold
                script_pubkey: vec![0x76, 0xa9],
            },
            // Valid output
            TxOut {
                value: 1000,
                script_pubkey: vec![0x51, 0x20, 0x99], // OP_RETURN
            },
        ],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    };

    // Create block with dust transaction
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
        transactions: vec![dust_tx],
    };

    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, None, 0);

    assert!(matches!(result, Err(ConsensusError::DustOutput { index: 0, value: 500, .. })));
}

#[test]
fn op_return_dust_allowed() {
    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::new();

    // Create transaction with OP_RETURN dust (should be allowed)
    let op_return_tx = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0xffffffff,
            script_sig: vec![0x01, 0x00],
            sequence: 0xffffffff,
        }],
        outputs: vec![
            // OP_RETURN dust (should be allowed)
            TxOut {
                value: 500, // Below threshold
                script_pubkey: vec![0x6a, 0x00], // OP_RETURN
            },
        ],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    };

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
        transactions: vec![op_return_tx],
    };

    // Should allow OP_RETURN dust
    let result = validate_block(&block, 0, &params, &registry,
                               NetworkId::Devnet, genesis::GENESIS_HASH_BYTES, None, 0);
    assert!(result.is_ok());
}

#[test]
fn script_execution_limits() {
    use crate::script::{ScriptInterpreter, OpCode, ScriptError};
    use bq_crypto::CryptoRegistry;

    let registry = CryptoRegistry::new();
    let mut interpreter = ScriptInterpreter::new(registry);

    // Test maximum script size
    let large_script = vec![0x51u8; 10_001]; // Exceeds 10,000 byte limit
    let result = interpreter.execute(&large_script, b"message");
    assert!(matches!(result, Err(ScriptError::ScriptTooLarge(10001))));

    // Test maximum operation limit
    let mut ops_script = vec![];
    for _ in 0..202 { // Exceeds 201 op limit
        ops_script.push(OpCode::True as u8);
    }
    let result = interpreter.execute(&ops_script, b"message");
    assert!(matches!(result, Err(ScriptError::TooManyOps(202))));
}

#[test]
fn fork_choice_equal_work_with_different_timestamps() {
    let mut fc = ForkChoice::new();

    let genesis = make_header([0u8; 32], 0x207fffff, 0, 0);
    fc.add_genesis(genesis.clone()).unwrap();
    let genesis_hash = header_hash(&genesis);

    // Chain A: earlier timestamp
    let a1 = make_header(genesis_hash, 0x207fffff, 1, 1);
    let a2 = make_header(header_hash(&a1), 0x207fffff, 2, 2);
    fc.add_block(a1.clone()).unwrap();
    let (is_tip_a2, _reorg_a2) = fc.add_block(a2).unwrap();
    assert!(is_tip_a2);

    // Chain B: same work, later timestamp (should not win)
    let b1 = make_header(genesis_hash, 0x207fffff, 10, 10);
    let b2 = make_header(header_hash(&b1), 0x207fffff, 11, 11);
    fc.add_block(b1).unwrap();
    let (is_tip_b2, reorg_b2) = fc.add_block(b2).unwrap();

    // Chain B should not reorg because Chain A has earlier timestamp
    assert!(!is_tip_b2);
    assert!(reorg_b2.is_none());
}

#[test]
fn proof_of_work_boundary_values() {
    use crate::pow::{check_header_pow, clamp_bits_within_bounds};

    // Test minimum bits
    let mut header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1700000000,
        bits: DEVNET_MIN_BITS,
        nonce: 0,
        algo_id: 0,
    };

    // Find nonce that satisfies minimum difficulty
    let mut nonce = 0;
    loop {
        header.nonce = nonce;
        if check_header_pow(&header).unwrap_or(false) {
            break;
        }
        nonce += 1;
        if nonce > 1_000_000 {
            panic!("Failed to find valid nonce for minimum bits");
        }
    }

    // Test maximum bits (should be easy to satisfy)
    header.bits = DEVNET_MAX_BITS;
    header.nonce = 0;
    let max_result = check_header_pow(&header);
    assert!(max_result.unwrap_or(false));

    // Test bit clamping
    let high_bits = 0x30000000;
    let clamped = clamp_bits_within_bounds(high_bits);
    assert_eq!(clamped, DEVNET_MAX_BITS);

    let low_bits = 0x0c000000;
    let clamped = clamp_bits_within_bounds(low_bits);
    assert_eq!(clamped, DEVNET_MIN_BITS);
}

#[test]
fn subsidy_halving_precision() {
    let rs = RewardSchedule::phase3_defaults();

    // Test halving at exact intervals
    for i in 0..127 {
        let height = i * 210_000;
        let subsidy = rs.subsidy_at_height(height);

        // Verify exact halving
        if i > 0 {
            let prev_subsidy = rs.subsidy_at_height(height - 1);
            assert_eq!(subsidy, prev_subsidy / 2);
        }

        // Verify no fractional subunits
        assert_eq!(subsidy % 1_000_000_000_000_000_000, 0); // No fractional qbits
    }

    // Test tail emission
    let height = 210_000 * 1000;
    let subsidy = rs.subsidy_at_height(height);
    assert_eq!(subsidy, rs.tail_emission_per_block);
}

#[test]
fn weight_calculation_with_max_values() {
    use bitquan_types::{SigAlgorithm, SignaturePayload};

    // Create transaction with maximum reasonable signature count
    let mut witnesses = Vec::new();
    const MAX_WITNESSES: usize = 100;
    const MAX_SIGS_PER_WITNESS: usize = 100;

    for _ in 0..MAX_WITNESSES {
        let mut signatures = Vec::new();
        for i in 0..MAX_SIGS_PER_WITNESS {
            signatures.push(SignaturePayload {
                signer_index: i as u16,
                signature: vec![0u8; 10],
                public_key: vec![0u8; 10],
                aux: None,
            });
        }
        witnesses.push(Witness { signatures });
    }

    let tx = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0xffffffff,
            script_sig: vec![0x01, 0x00],
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0x76, 0xa9],
        }],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses,
    };

    // Should calculate weight or detect overflow
    let result = calculate_tx_weight(&tx);
    match result {
        Ok(weight) => {
            // Total expected signatures: 100 * 100 = 10,000
            let expected_sig_weight = 10_000 * 384;
            assert!(weight >= expected_sig_weight);
        }
        Err(ConsensusError::WeightOverflow(_)) => {
            // Overflow detection is acceptable for extreme cases
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

// Helper functions for tests
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

fn create_valid_block() -> Block {
    let coinbase = create_valid_coinbase();
    Block {
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
        transactions: vec![coinbase],
    }
}

fn create_valid_coinbase() -> Transaction {
    let params = ConsensusParams::phase3_defaults();
    Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0xffffffff,
            script_sig: vec![0x01, 0x00],
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: params.reward_schedule.subsidy_at_height(0),
            script_pubkey: vec![0x76, 0xa9],
        }],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    }
}