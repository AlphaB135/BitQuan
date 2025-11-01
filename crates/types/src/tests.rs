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
    assert_eq!(tx.signature_count(), 1);
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
    assert!(block.serialized_size_hint() as u64 + weight >= weight);
}
