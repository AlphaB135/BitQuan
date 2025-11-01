use super::*;

#[test]
fn subsidy_initial_and_halving() {
    let rs = RewardSchedule::phase3_defaults();
    assert_eq!(rs.subsidy_at_height(0), 5_000_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 - 1), 5_000_000_000);
    assert_eq!(rs.subsidy_at_height(210_000), 2_500_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 * 6), 78_125_000);
}

#[test]
fn subsidy_tail_emission_after_seven_halvings() {
    let rs = RewardSchedule::phase3_defaults();
    assert_eq!(rs.subsidy_at_height(210_000 * 7), 50_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 * 1000), 50_000_000);
}

fn mtp(timestamps: &[u64]) -> u64 {
    let mut v = timestamps.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

#[test]
fn difficultystate_with_mtp_anchor_chain() {
    let params = ConsensusParams::phase3_defaults();
    // Simulate 11 previous block timestamps spaced by 600s
    let base: u64 = 1_700_000_000;
    let prev_times: Vec<u64> = (0..11).map(|i| base + i * 600).collect();
    let anchor_height = 1000;
    let anchor_bits = 0x1d00ffff; // classic initial target
    let mut state = DifficultyState::new(anchor_height, prev_times[10], anchor_bits, 0);

    // Next block time uses MTP of the previous 11
    let next_time = mtp(&prev_times);
    let next_bits = state.update(anchor_height + 1, next_time, &params);
    assert!(next_bits > 0);
}

#[test]
fn difficultystate_with_chainstore_mtp_anchor() {
    use bitquan_storage::{ChainStore, InMemoryChainStore};
    use bitquan_types::{Block, BlockHeader, Transaction};

    let params = ConsensusParams::phase3_defaults();
    let mut store = InMemoryChainStore::new();
    let base: u64 = 1_700_100_000;
    let bits: u32 = 0x1d00ffff;

    // Build a chain of 11 headers
    let mut headers: Vec<BlockHeader> = Vec::new();
    for i in 0..11u64 {
        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: (base + i * params.target_block_time) as u32,
            bits,
            nonce: 0,
        };
        headers.push(header.clone());
        let _ = store.insert_block(Block {
            header,
            transactions: Vec::<Transaction>::new(),
        });
    }

    let tip = store.tip().expect("tip").expect("tip block").clone();
    let anchor_height = 10; // zero-based index of tip in this test chain
    let mut state = DifficultyState::new(anchor_height, tip.time as u64, tip.bits, 0);

    let times: Vec<u64> = headers.iter().map(|h| h.time as u64).collect();
    let next_time = mtp(&times);
    let next_bits = state.update(anchor_height + 1, next_time, &params);
    assert!(next_bits > 0);
}

#[test]
fn test_calculate_tx_weight_bqip0002() {
    use bitquan_types::{SigAlgorithm, SignaturePayload, Transaction, TxIn, TxOut, Witness};

    // Create transaction with 1 input, 2 outputs, 1 signature
    let tx = Transaction {
        version: 2,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0,
            script_sig: vec![],
            sequence: 0xffffffff,
        }],
        outputs: vec![
            TxOut {
                value: 1000,
                script_pubkey: vec![0x76, 0xa9],
            },
            TxOut {
                value: 2000,
                script_pubkey: vec![0x76, 0xa9],
            },
        ],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![Witness {
            signatures: vec![SignaturePayload {
                signer_index: 0,
                signature: vec![0u8; 10],
                public_key: vec![0u8; 10],
                aux: None,
            }],
        }],
    };

    let weight = calculate_tx_weight(&tx);

    // Weight should be: base_size*4 + 1*384
    // At minimum: 384 (1 signature weight)
    assert!(weight >= 384);

    // Should be deterministic
    assert_eq!(weight, calculate_tx_weight(&tx));
}

#[test]
fn test_block_weight_calculation() {
    use bitquan_types::{Block, BlockHeader, SigAlgorithm, Transaction, TxIn, TxOut};

    let params = ConsensusParams::phase3_defaults();

    // Create a block with single coinbase transaction
    let coinbase = Transaction {
        version: 2,
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
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![],
    };

    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1700000000,
            bits: 0x1d00ffff,
            nonce: 0,
        },
        transactions: vec![coinbase],
    };

    let weight = calculate_block_weight(&block);

    // Verify weight is within limit
    assert!(weight < params.block_weight_cap as usize);

    // Weight should be deterministic
    assert_eq!(weight, calculate_block_weight(&block));
}

#[test]
fn test_block_weight_exceeds_limit() {
    use bitquan_types::{
        Block, BlockHeader, SigAlgorithm, SignaturePayload, Transaction, TxIn, TxOut, Witness,
    };

    // Create a block that would exceed weight limit
    let mut transactions = vec![];

    // Add many transactions with signatures
    for i in 0..15000 {
        transactions.push(Transaction {
            version: 2,
            lock_time: 0,
            inputs: vec![TxIn {
                prev_txid: [i as u8; 32],
                prev_vout: 0,
                script_sig: vec![],
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                value: 1000,
                script_pubkey: vec![0x76, 0xa9],
            }],
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![Witness {
                signatures: vec![SignaturePayload {
                    signer_index: 0,
                    signature: vec![0u8; 10],
                    public_key: vec![0u8; 10],
                    aux: None,
                }],
            }],
        });
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
        },
        transactions,
    };

    let weight = calculate_block_weight(&block);
    let params = ConsensusParams::phase3_defaults();

    // Weight should exceed limit
    assert!(weight as u64 > params.block_weight_cap);
}

#[test]
fn test_transaction_and_block_hash_determinism() {
    use crate::pow::header_hash;
    use crate::transaction_sighash;
    use bitquan_types::{
        Block, BlockHeader, NetworkId, SigAlgorithm, SignaturePayload, Transaction, TxIn, TxOut,
        Witness,
    };

    // Sample transaction with witness payload.
    let tx = Transaction {
        version: 2,
        lock_time: 42,
        inputs: vec![TxIn {
            prev_txid: [0x10; 32],
            prev_vout: 7,
            script_sig: vec![0xaa, 0xbb, 0xcc],
            sequence: 0xffff_fffe,
        }],
        outputs: vec![TxOut {
            value: 42_000,
            script_pubkey: vec![0x51, 0x20, 0x99],
        }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![Witness {
            signatures: vec![SignaturePayload {
                signer_index: 0,
                signature: vec![0xde, 0xad, 0xbe, 0xef],
                public_key: vec![0x01, 0x02],
                aux: None,
            }],
        }],
    };

    // Transaction sighash must be deterministic across repeated invocations.
    let expected_tx_hash = transaction_sighash(&tx, NetworkId::Mainnet);
    for _ in 0..32 {
        assert_eq!(
            transaction_sighash(&tx, NetworkId::Mainnet),
            expected_tx_hash
        );
    }

    // Build a block and ensure header hashing is deterministic as well.
    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_block: [0x55; 32],
            merkle_root: [0x33; 32],
            pqc_agg_hint: [0x44; 32],
            time: 1_700_000_123,
            bits: 0x1d00ffff,
            nonce: 99,
        },
        transactions: vec![tx.clone()],
    };

    let expected_header_hash = header_hash(&block.header);
    for _ in 0..32 {
        assert_eq!(header_hash(&block.header), expected_header_hash);
    }

    // Recomputing via freshly constructed block components must stay stable.
    let expected_again = transaction_sighash(&block.transactions[0], NetworkId::Mainnet);
    assert_eq!(expected_again, expected_tx_hash);
}

#[test]
fn test_signature_weight_scaling() {
    use bitquan_types::{SigAlgorithm, SignaturePayload, Transaction, TxIn, TxOut, Witness};

    // Transaction with 1 signature
    let tx1 = Transaction {
        version: 2,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0,
            script_sig: vec![],
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0x76, 0xa9],
        }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![Witness {
            signatures: vec![SignaturePayload {
                signer_index: 0,
                signature: vec![0u8; 10],
                public_key: vec![0u8; 10],
                aux: None,
            }],
        }],
    };

    // Transaction with 3 signatures
    let tx3 = Transaction {
        version: 2,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0,
            script_sig: vec![],
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0x76, 0xa9],
        }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![Witness {
            signatures: vec![
                SignaturePayload {
                    signer_index: 0,
                    signature: vec![0u8; 10],
                    public_key: vec![0u8; 10],
                    aux: None,
                },
                SignaturePayload {
                    signer_index: 1,
                    signature: vec![0u8; 10],
                    public_key: vec![0u8; 10],
                    aux: None,
                },
                SignaturePayload {
                    signer_index: 2,
                    signature: vec![0u8; 10],
                    public_key: vec![0u8; 10],
                    aux: None,
                },
            ],
        }],
    };

    let weight1 = calculate_tx_weight(&tx1);
    let weight3 = calculate_tx_weight(&tx3);

    // Weight difference should be approximately 2 * 384 = 768
    let diff = weight3 - weight1;
    assert!(
        (768..=800).contains(&diff),
        "Expected ~768 WU difference, got {}",
        diff
    );
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use bitquan_types::{SigAlgorithm, Transaction, TxIn, TxOut};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn weight_calculation_deterministic(
            num_inputs in 1usize..10,
            num_outputs in 1usize..10
        ) {
            let tx = Transaction {
                version: 2,
                lock_time: 0,
                inputs: (0..num_inputs).map(|i| TxIn {
                    prev_txid: [i as u8; 32],
                    prev_vout: i as u32,
                    script_sig: vec![],
                    sequence: 0xffffffff,
                }).collect(),
                outputs: (0..num_outputs).map(|i| TxOut {
                    value: 1000 * (i as u64 + 1),
                    script_pubkey: vec![0x76, 0xa9],
                }).collect(),
                sig_algo: SigAlgorithm::Dilithium3,
                witnesses: vec![],
            };

            // Weight should be deterministic
            let w1 = calculate_tx_weight(&tx);
            let w2 = calculate_tx_weight(&tx);
            prop_assert_eq!(w1, w2);

            // Weight should be positive
            prop_assert!(w1 > 0);
        }

        #[test]
        fn signature_weight_linear(
            sig_count in 0usize..20
        ) {
            use bitquan_types::{Witness, SignaturePayload};

            let tx = Transaction {
                version: 2,
                lock_time: 0,
                inputs: vec![TxIn {
                    prev_txid: [0u8; 32],
                    prev_vout: 0,
                    script_sig: vec![],
                    sequence: 0xffffffff,
                }],
                outputs: vec![TxOut {
                    value: 1000,
                    script_pubkey: vec![0x76, 0xa9],
                }],
                sig_algo: SigAlgorithm::Dilithium3,
                witnesses: vec![Witness {
                    signatures: (0..sig_count).map(|i| SignaturePayload {
                        signer_index: i as u16,
                        signature: vec![0u8; 10],
                        public_key: vec![0u8; 10],
                        aux: None,
                    }).collect(),
                }],
            };

            let weight = calculate_tx_weight(&tx);

            // Weight should include signature_count * 384
            let expected_sig_weight = sig_count * 384;
            prop_assert!(weight >= expected_sig_weight);
        }

        #[test]
        fn block_weight_is_sum_of_txs(
            tx_count in 1usize..10
        ) {
            use bitquan_types::{Block, BlockHeader};

            let txs: Vec<Transaction> = (0..tx_count).map(|i| Transaction {
                version: 2,
                lock_time: 0,
                inputs: vec![TxIn {
                    prev_txid: [i as u8; 32],
                    prev_vout: 0,
                    script_sig: vec![],
                    sequence: 0xffffffff,
                }],
                outputs: vec![TxOut {
                    value: 1000,
                    script_pubkey: vec![0x76, 0xa9],
                }],
                sig_algo: SigAlgorithm::Dilithium3,
                witnesses: vec![],
            }).collect();

            let block = Block {
                header: BlockHeader {
                    version: 1,
                    prev_block: [0u8; 32],
                    merkle_root: [0u8; 32],
                    pqc_agg_hint: [0u8; 32],
                    time: 1700000000,
                    bits: 0x1d00ffff,
                    nonce: 0,
                },
                transactions: txs.clone(),
            };

            let block_weight = calculate_block_weight(&block);
            let sum_weights: usize = txs.iter().map(calculate_tx_weight).sum();

            prop_assert_eq!(block_weight, sum_weights);
        }
    }
}
