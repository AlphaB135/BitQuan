#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Canonical transaction digest construction for PQC signature verification.

use bitquan_types::{NetworkId, Transaction, TxIn, TxOut, Witness};
use sha2::{Digest, Sha256};

/// Computes a 32-byte digest for the supplied transaction.
/// Includes network_id for cross-chain replay protection (BQIP-0002).
pub fn transaction_sighash(tx: &Transaction, network_id: NetworkId) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([network_id.as_u8()]);
    hasher.update(tx.version.to_le_bytes());
    hasher.update(tx.lock_time.to_le_bytes());

    hash_txins(&mut hasher, &tx.inputs);
    hash_txouts(&mut hasher, &tx.outputs);
    hasher.update([tx.sig_algo.code()]);
    hash_witnesses(&mut hasher, &tx.witnesses);

    let digest = hasher.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

fn hash_txins(hasher: &mut Sha256, inputs: &[TxIn]) {
    hash_len(hasher, inputs.len());
    for input in inputs {
        hasher.update(&input.prev_txid);
        hasher.update(input.prev_vout.to_le_bytes());
        hasher.update(input.sequence.to_le_bytes());
        hash_bytes(hasher, &input.script_sig);
    }
}

fn hash_txouts(hasher: &mut Sha256, outputs: &[TxOut]) {
    hash_len(hasher, outputs.len());
    for output in outputs {
        hasher.update(output.value.to_le_bytes());
        hash_bytes(hasher, &output.script_pubkey);
    }
}

fn hash_witnesses(hasher: &mut Sha256, witnesses: &[Witness]) {
    hash_len(hasher, witnesses.len());
    for witness in witnesses {
        hash_len(hasher, witness.signatures.len());
        for sig in &witness.signatures {
            hasher.update(sig.signer_index.to_le_bytes());
            hash_bytes(hasher, &sig.signature);
            hash_bytes(hasher, &sig.public_key);
            match &sig.aux {
                Some(aux) => {
                    hasher.update([1]);
                    hash_bytes(hasher, &aux.payload);
                }
                None => hasher.update([0]),
            }
        }
    }
}

fn hash_bytes(hasher: &mut Sha256, data: &[u8]) {
    hash_len(hasher, data.len());
    hasher.update(data);
}

fn hash_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::{
        AuxiliarySignatureData, NetworkId, SigAlgorithm, SignaturePayload, Transaction, TxIn,
        TxOut, Witness,
    };

    #[test]
    fn digest_changes_with_witness() {
        let mut tx = sample_tx();
        let original = transaction_sighash(&tx, NetworkId::Devnet);
        tx.witnesses[0].signatures[0].signature[0] ^= 0xFF;
        let mutated = transaction_sighash(&tx, NetworkId::Devnet);
        assert_ne!(original, mutated);
    }

    #[test]
    fn digest_changes_with_lock_time() {
        let mut tx = sample_tx();
        let original = transaction_sighash(&tx, NetworkId::Devnet);
        tx.lock_time = 42;
        let mutated = transaction_sighash(&tx, NetworkId::Devnet);
        assert_ne!(original, mutated);
    }

    #[test]
    fn digest_changes_with_network_id() {
        let tx = sample_tx();
        let mainnet = transaction_sighash(&tx, NetworkId::Mainnet);
        let testnet = transaction_sighash(&tx, NetworkId::Testnet);
        let devnet = transaction_sighash(&tx, NetworkId::Devnet);
        let regtest = transaction_sighash(&tx, NetworkId::Regtest);

        assert_ne!(mainnet, testnet);
        assert_ne!(mainnet, devnet);
        assert_ne!(mainnet, regtest);
        assert_ne!(testnet, devnet);
        assert_ne!(testnet, regtest);
        assert_ne!(devnet, regtest);
    }

    #[test]
    fn digest_deterministic_same_network() {
        let tx = sample_tx();
        let hash1 = transaction_sighash(&tx, NetworkId::Mainnet);
        let hash2 = transaction_sighash(&tx, NetworkId::Mainnet);
        assert_eq!(hash1, hash2);
    }

    fn sample_tx() -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            inputs: vec![TxIn {
                prev_txid: [1u8; 32],
                prev_vout: 0,
                sequence: 0xffff_fffe,
                script_sig: vec![0xaa, 0xbb],
            }],
            outputs: vec![TxOut {
                value: 1_000,
                script_pubkey: vec![0x51],
            }],
            sig_algo: SigAlgorithm::Dilithium3,
            witnesses: vec![Witness {
                signatures: vec![SignaturePayload {
                    signer_index: 0,
                    signature: vec![0x11; 2],
                    public_key: vec![0x22; 2],
                    aux: Some(AuxiliarySignatureData {
                        payload: vec![0x33, 0x44],
                    }),
                }],
            }],
        }
    }
}
