#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_mempool::Mempool;
use bitquan_types::{Transaction, TxIn, TxOut, SigAlgorithm};

fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
        return;
    }
    
    let Ok(mut mempool) = Mempool::new() else {
        return;
    };
    
    // Fuzz adding transactions
    let num_txs = (data[0] as usize % 10) + 1;
    
    for i in 0..num_txs {
        if data.len() < (i + 1) * 10 {
            break;
        }
        
        let offset = i * 10;
        let value = u64::from_le_bytes([
            data[offset], data[offset+1], data[offset+2], data[offset+3],
            data[offset+4], data[offset+5], data[offset+6], data[offset+7]
        ]);
        
        let tx = Transaction {
            version: 1,
            inputs: vec![TxIn {
                prev_txid: {
                    let mut txid = [0u8; 32];
                    txid[0] = data[offset+8];
                    txid[1] = i as u8;
                    txid
                },
                prev_vout: 0,
                script_sig: vec![],
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                value,
                script_pubkey: vec![],
            }],
            lock_time: 0,
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![],
        };
        
        let fee = (data[offset+9] as u64) * 1000;
        
        // Test adding transaction doesn't panic
        let _ = mempool.insert(tx, fee);
    }
    
    // Test len/empty checks
    let _ = mempool.len();
    let _ = mempool.is_empty();
    
    // Test size calculation
    let _ = mempool.size_bytes();
    
    // Test min fee rate
    let _ = mempool.min_fee_rate();
});
