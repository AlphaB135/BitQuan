use bitquan_types::*;

fn main() {
    let tx_in = TxIn {
        prev_txid: [0u8; 32],
        prev_vout: 0,
        sequence: 0xffff_fffe,
        script_sig: vec![0x51],
    };
    let tx_out = TxOut {
        value: 123_456_789,
        script_pubkey: vec![0x76, 0xa9, 0x14, 0x00, 0x88, 0xac],
    };
    let sig = SignaturePayload {
        signer_index: 0,
        signature: vec![0xAB; 8],
        public_key: vec![0xCD; 4],
        aux: None,
    };
    let witness = Witness {
        signatures: vec![sig],
    };
    let tx = Transaction {
        version: 2,
        lock_time: 0,
        inputs: vec![tx_in],
        outputs: vec![tx_out],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![witness],
    };

    let base = tx.to_bytes_base();
    let full = tx.to_bytes_with_witness();
    let txid = tx.txid();
    let wtxid = tx.wtxid();

    println!("tx_base: {}", hex::encode(&base));
    println!("tx_full: {}", hex::encode(&full));
    println!("txid: {}", hex::encode(txid));
    println!("wtxid: {}", hex::encode(wtxid));
}
