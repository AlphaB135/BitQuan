#![no_main]

use bitquan_consensus::difficulty::compact_to_target;
use bitquan_consensus::{ForkChoice, ForkError};
use bitquan_types::BlockHeader;
use libfuzzer_sys::fuzz_target;

// Fuzz fork choice logic for difficulty calculations and reorg handling
fuzz_target!(|data: &[u8]| {
    // Ensure minimum data for block headers
    if data.len() < 117 {
        return;
    }

    let mut fork_choice = ForkChoice::new();

    // Add genesis block first
    let genesis_header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 1231006505, // Bitcoin genesis timestamp
        bits: 0x1d00ffff, // Easy difficulty for genesis
        nonce: 0,
        algo_id: 0,
    };

    let _ = fork_choice.add_genesis(genesis_header.clone());

    // Parse multiple block headers from fuzz data
    let num_blocks = (data.len() / 117).min(10); // Limit to prevent excessive computation
    let mut prev_hash = [0u8; 32]; // Will be set after first block

    for i in 0..num_blocks {
        let start = i * 117;
        let end = (start + 117).min(data.len());
        if end - start < 117 {
            break;
        }

        let block_data = &data[start..end];

        // Create block header from fuzz data
        let header = BlockHeader {
            version: i32::from_le_bytes([
                block_data[0],
                block_data[1],
                block_data[2],
                block_data[3],
            ]),
            prev_block: if i == 0 {
                [0u8; 32] // Genesis child
            } else {
                prev_hash // Chain from previous block
            },
            merkle_root: {
                let mut root = [0u8; 32];
                root.copy_from_slice(&block_data[4..36]);
                root
            },
            pqc_agg_hint: {
                let mut hint = [0u8; 32];
                hint.copy_from_slice(&block_data[36..68]);
                hint
            },
            time: u32::from_le_bytes([
                block_data[68],
                block_data[69],
                block_data[70],
                block_data[71],
            ]),
            bits: u32::from_le_bytes([
                block_data[72],
                block_data[73],
                block_data[74],
                block_data[75],
            ]),
            nonce: u64::from_le_bytes([
                block_data[76],
                block_data[77],
                block_data[78],
                block_data[79],
                block_data[80],
                block_data[81],
                block_data[82],
                block_data[83],
            ]),
            algo_id: block_data[84],
        };

        // Test fork choice with this block
        match fork_choice.add_block(header.clone()) {
            Ok((is_new_tip, reorg_info)) => {
                // If this became new tip, update prev_hash for next block
                if is_new_tip {
                    prev_hash = bitquan_consensus::pow::header_hash(&header);
                }

                // Test reorg info if present
                if let Some(reorg) = reorg_info {
                    // Verify reorg depth is reasonable
                    assert!(reorg.depth() <= 100, "Reorg too deep: {}", reorg.depth());
                    assert!(
                        reorg.new_blocks() <= 1000,
                        "Too many new blocks: {}",
                        reorg.new_blocks()
                    );
                }
            }
            Err(ForkError::OrphanBlock(_)) => {
                // Expected for some blocks - continue
            }
            Err(ForkError::DuplicateBlock(_)) => {
                // Expected for some blocks - continue
            }
            Err(ForkError::ReorgTooDeep(depth, max)) => {
                // Expected for deep reorgs - verify bounds
                assert!(depth > max, "Reorg depth error logic wrong");
            }
            Err(ForkError::InvalidWork) => {
                // Expected for invalid blocks - continue
            }
        }

        // Test difficulty calculations directly
        let target = compact_to_target(header.bits);

        // Test extreme target values
        if target == 0 {
            // Should handle zero target gracefully
            continue;
        }

        // Test work calculation (should not overflow)
        let work = if target != 0 {
            u64::MAX.checked_div(target.saturating_add(1)).unwrap_or(1)
        } else {
            1
        };

        // Work should be reasonable
        assert!(work > 0, "Work should be positive");
    }

    // Test fork choice state queries
    let _ = fork_choice.best_tip();
    let _ = fork_choice.height();
    let _ = fork_choice.best_hash();
    let _ = fork_choice.get_main_chain();

    // Test edge cases
    if data.len() > 500 {
        // Test with maximum reorg depth
        let mut limited_fc = ForkChoice::with_max_reorg(1);
        let _ = limited_fc.add_genesis(genesis_header.clone());

        // Try to create deep reorg
        for i in 0..5 {
            let header = BlockHeader {
                version: 1,
                prev_block: if i == 0 { [0u8; 32] } else { [1u8; 32] },
                merkle_root: [i as u8; 32],
                pqc_agg_hint: [0u8; 32],
                time: 1231006505 + i as u32,
                bits: 0x207fffff, // Easy difficulty
                nonce: i as u64,
                algo_id: 0,
            };

            let _ = limited_fc.add_block(header);
        }
    }

    // Test concurrent access patterns
    if data.len() > 700 {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let fc_arc = Arc::new(Mutex::new(ForkChoice::new()));
        let handles: Vec<_> = (0..3)
            .map(|thread_id| {
                let fc_clone = Arc::clone(&fc_arc);
                let test_header = BlockHeader {
                    version: 1,
                    prev_block: [thread_id as u8; 32],
                    merkle_root: [thread_id as u8; 32],
                    pqc_agg_hint: [0u8; 32],
                    time: 1231006505 + thread_id as u32,
                    bits: 0x207fffff,
                    nonce: thread_id as u64,
                    algo_id: 0,
                };

                thread::spawn(move || {
                    let mut fc = fc_clone.lock().unwrap();
                    let _ = fc.add_genesis(test_header.clone());
                    let _ = fc.add_block(test_header);
                    let _ = fc.best_tip();
                    let _ = fc.height();
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            let _ = handle.join();
        }
    }
});
