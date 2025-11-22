#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_types::{Block, BlockHeader};

fuzz_target!(|data: &[u8]| {
    // Fuzz block header parsing
    if data.len() >= 117 {
        let mut version = [0u8; 4];
        let mut prev_block = [0u8; 32];
        let mut merkle_root = [0u8; 32];
        let mut pqc_agg_hint = [0u8; 32];
        let mut time = [0u8; 4];
        let mut bits = [0u8; 4];
        let mut nonce = [0u8; 8];
        let mut algo_id = [0u8; 1];

        version.copy_from_slice(&data[0..4]);
        prev_block.copy_from_slice(&data[4..36]);
        merkle_root.copy_from_slice(&data[36..68]);
        pqc_agg_hint.copy_from_slice(&data[68..100]);
        time.copy_from_slice(&data[100..104]);
        bits.copy_from_slice(&data[104..108]);
        nonce.copy_from_slice(&data[108..116]);
        algo_id.copy_from_slice(&data[116..117]);

        let header = BlockHeader {
            version: i32::from_le_bytes(version),
            prev_block,
            merkle_root,
            pqc_agg_hint,
            time: u32::from_le_bytes(time),
            bits: u32::from_le_bytes(bits),
            nonce: u64::from_le_bytes(nonce),
            algo_id: algo_id[0],
        };

        // Test serialization doesn't panic
        let _ = header.to_bytes();
        let _ = header.serialized_size();

        // Test block with header
        let block = Block {
            header,
            transactions: vec![],
        };

        // Test merkle root calculation
        let _ = block.compute_merkle_root();

        // Test validation doesn't panic (needs timestamp)
        let current_time = u32::from_le_bytes(time);
        let _ = bitquan_types::validate_block_structure(&block, current_time);

        // Test signature count
        let _ = bitquan_types::count_signatures(&block);
    }
});
