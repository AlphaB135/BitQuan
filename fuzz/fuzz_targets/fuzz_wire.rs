#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_types::{Transaction, NetworkId, SigAlgorithm};
use serde_json;

fuzz_target!(|data: &[u8]| {
    // Test raw transaction deserialization from wire format
    // This simulates network message parsing with malformed data

    // Try to deserialize as Transaction directly (most common wire format)
    let _result: Result<Transaction, _> = serde_json::from_slice(data);

    // Test partial transaction structures that might appear in wire format
    if data.len() >= 4 {
        // Test version parsing
        let version_bytes = &data[..4.min(data.len())];
        if version_bytes.len() == 4 {
            let version = i32::from_le_bytes([version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3]]);
            // Test that version doesn't cause issues in validation
            if version >= 0 && version <= 1000 {
                // Create minimal transaction with this version
                let tx = Transaction {
                    version,
                    network: NetworkId::Devnet,
                    genesis_hash: [0u8; 32],
                    inputs: vec![],
                    outputs: vec![],
                    lock_time: 0,
                    sig_algo: SigAlgorithm::Dilithium3,
                    witnesses: vec![],
                };

                // Test serialization doesn't panic
                let _ = tx.to_bytes_base();
                let _ = tx.to_bytes_with_witness();
            }
        }
    }

    // Test network ID parsing from single byte
    if data.len() >= 1 {
        let network_byte = data[0];
        let _network = NetworkId::from_u8(network_byte);
    }

    // Test signature algorithm parsing
    if data.len() >= 1 {
        let algo_byte = data[0];
        let _algo = SigAlgorithm::from_code(algo_byte);
    }

    // Test malformed JSON that might come from RPC endpoints
    if data.len() >= 2 && data[0] == b'{' && data[data.len()-1] == b'}' {
        let _result: Result<Transaction, _> = serde_json::from_slice(data);
    }

    // Test compact uint parsing (common in wire format)
    if !data.is_empty() {
        let first_byte = data[0];
        match first_byte {
            0xFD => {
                if data.len() >= 3 {
                    let _value = u16::from_le_bytes([data[1], data[2]]) as u64;
                }
            }
            0xFE => {
                if data.len() >= 5 {
                    let _value = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as u64;
                }
            }
            0xFF => {
                if data.len() >= 9 {
                    let _value = u64::from_le_bytes([
                        data[1], data[2], data[3], data[4],
                        data[5], data[6], data[7], data[8]
                    ]);
                }
            }
            _ => {
                let _value = first_byte as u64;
            }
        }
    }

    // Test transaction ID calculation with various data
    if data.len() >= 32 {
        let mut txid_bytes = [0u8; 32];
        txid_bytes.copy_from_slice(&data[..32]);

        // Create transaction with this as previous txid
        let tx = Transaction {
            version: 1,
            network: NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            inputs: vec![bitquan_types::TxIn {
                prev_txid: txid_bytes,
                prev_vout: 0,
                sequence: 0xffffffff,
                script_sig: vec![],
            }],
            outputs: vec![],
            lock_time: 0,
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };

        // Test txid calculation doesn't panic
        let _txid = tx.txid();
        let _wtxid = tx.wtxid();
    }
});
