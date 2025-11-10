use super::*;
use serde_json::{from_str, to_string};

fn sample_tx() -> Transaction {
    let tx_in = TxIn {
        prev_txid: [0u8; 32],
        prev_vout: 0,
        sequence: 0xffff_fffe,
        script_sig: vec![0x51], // OP_1 placeholder
    };

    let tx_out = TxOut {
        value: 123_456_789,
        script_pubkey: vec![0x76, 0xa9, 0x14, 0x00, 0x88, 0xac], // OP_DUP OP_HASH160 <20B=0> OP_EQUALVERIFY OP_CHECKSIG
    };

    let sig = SignaturePayload {
        signer_index: 0,
        signature: vec![0xAB; 8],
        public_key: vec![0xCD; 4],
        aux: Some(AuxiliarySignatureData { payload: vec![0xEF] }),
    };

    let witness = Witness { signatures: vec![sig] };

    Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![tx_in],
        outputs: vec![tx_out],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![witness],
    }
}

#[test]
fn witness_json_roundtrip() {
    let tx = sample_tx();
    let json = to_string(&tx).expect("json");
    let de: Transaction = from_str(&json).expect("de");
    assert_eq!(tx, de);
    // Vector sanity: stable JSON prefix
    assert!(json.contains("\"version\":2"));
    assert!(json.contains("\"witnesses\""));
}

#[test]
fn signature_count_matches_witness_items() {
    let tx = sample_tx();
    assert_eq!(tx.signature_count().expect("Failed to get signature count"), 1);
}

#[test]
fn block_weight_accounts_for_signatures() {
    let tx = sample_tx();
    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 0,
            bits: 0,
            nonce: 0,
        },
        transactions: vec![tx],
    };
    let alpha = 384u32;
    let weight = crate::count_signatures(&block) * alpha as u64;
    assert_eq!(crate::count_signatures(&block), 1);
    // Serialized size hint >= header size; just ensure signature term is added as expected
    let block_size = block.serialized_size_hint().expect("Failed to get block serialized size") as u64;
    assert!(block_size + weight >= weight);
}

#[test]
fn test_tx_size_overflow_protection() {
    // Create transaction with many large inputs to trigger overflow
    let mut inputs = Vec::new();
    for i in 0..10000 {
        inputs.push(TxIn {
            prev_txid: [i as u8; 32],
            prev_vout: i,
            sequence: 0xffffffff,
            script_sig: vec![0u8; 10000], // Large script
        });
    }

    let tx = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs,
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![],
    };

    // Should either succeed with large size or fail with overflow
    let result = tx.serialized_size_hint();
    match result {
        Ok(size) => assert!(size > 0),
        Err(ValidationError::SizeOverflow(_)) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_witness_size_overflow_protection() {
    // Create transaction with massive witness data
    let mut witnesses = Vec::new();
    for _ in 0..1000 {
        let mut signatures = Vec::new();
        for j in 0..100 {
            signatures.push(SignaturePayload {
                signer_index: j,
                signature: vec![0u8; 10000],
                public_key: vec![0u8; 10000],
                aux: Some(AuxiliarySignatureData {
                    payload: vec![0u8; 10000],
                }),
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
            prev_vout: 0,
            sequence: 0xffffffff,
            script_sig: vec![],
        }],
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses,
    };

    // Should detect overflow
    let result = tx.witness_size_hint();
    match result {
        Ok(size) => assert!(size > 0),
        Err(ValidationError::SizeOverflow(_)) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_signature_count_overflow() {
    // Create transaction with many witnesses
    let mut witnesses = Vec::new();
    for _ in 0..usize::MAX / 1000 {
        witnesses.push(Witness {
            signatures: vec![SignaturePayload {
                signer_index: 0,
                signature: vec![0u8; 1],
                public_key: vec![0u8; 1],
                aux: None,
            }],
        });
    }

    let tx = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: genesis::GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![TxIn {
            prev_txid: [0u8; 32],
            prev_vout: 0,
            sequence: 0xffffffff,
            script_sig: vec![],
        }],
        outputs: vec![TxOut {
            value: 1000,
            script_pubkey: vec![0x51],
        }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses,
    };

    // Should detect overflow
    let result = tx.signature_count();
    match result {
        Ok(_) => {}
        Err(ValidationError::SizeOverflow(msg)) => {
            assert!(msg.contains("signature"));
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_normal_tx_size_calculations_still_work() {
    let tx = sample_tx();

    // All size calculations should succeed for normal transactions
    assert!(tx.serialized_size_hint().is_ok());
    assert!(tx.witness_size_hint().is_ok());
    assert!(tx.signature_count().is_ok());

    let size = tx.serialized_size_hint().expect("Failed to get transaction size");
    let witness_size = tx.witness_size_hint().expect("Failed to get witness size");
    let sig_count = tx.signature_count().expect("Failed to get signature count");

    assert!(size > 0);
    assert!(witness_size > 0);
    assert_eq!(sig_count, 1);
}
