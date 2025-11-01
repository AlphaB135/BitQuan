#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_types::{genesis::GENESIS_HASH_BYTES, NetworkId, SigAlgorithm, Transaction, TxIn, TxOut};

fuzz_target!(|data: &[u8]| {
    // Try to parse as individual components
    if data.len() >= 8 {
        let version = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let lock_time = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        
        // Create minimal transaction
        let tx = Transaction {
            version,
            network: NetworkId::Devnet,
            genesis_hash: GENESIS_HASH_BYTES,
            inputs: vec![],
            outputs: vec![],
            lock_time,
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };
        
        // Test validation doesn't panic
        let _ = bitquan_types::validate_transaction(&tx);
    }

    // Fuzz transaction weight calculation
    if data.len() >= 12 {
        let num_inputs = (data[8] as usize).min(100);
        let num_outputs = (data[9] as usize).min(100);
        
        let inputs: Vec<TxIn> = (0..num_inputs)
            .map(|_| TxIn {
                prev_txid: [0u8; 32],
                prev_vout: 0,
                script_sig: vec![],
                sequence: 0xffffffff,
            })
            .collect();
            
        let outputs: Vec<TxOut> = (0..num_outputs)
            .map(|_| TxOut {
                value: 0,
                script_pubkey: vec![],
            })
            .collect();
            
        let tx = Transaction {
            version: 1,
            network: NetworkId::Devnet,
            genesis_hash: GENESIS_HASH_BYTES,
            inputs,
            outputs,
            lock_time: 0,
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };
        
        // Test serialization size hint doesn't panic
        let _ = tx.serialized_size_hint();
        
        // Test signature count doesn't panic
        let _ = tx.signature_count();
    }
});
