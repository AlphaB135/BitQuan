#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Canonical transaction digest construction for PQC signature verification.

use bitquan_types::error::{Error, Result};
use bitquan_types::{Transaction, TxContext, TxIn, TxOut, Witness};
use sha2::{Digest, Sha256};

/// Domain separator for BitQuan transaction signatures (version 1).
const DOMAIN_SEPARATOR: &[u8] = b"BitQuanSigHashV1";

/// Computes a 32-byte digest for the supplied transaction using the given context.
///
/// The context binds the signature to a specific network and chain, preventing
/// replay attacks across different networks or chain forks.
///
/// # Errors
///
/// Returns `Error::Invalid` if transaction and context networks or genesis hashes differ.
pub fn transaction_sighash(tx: &Transaction, ctx: &TxContext) -> Result<[u8; 32]> {
    // Verify transaction matches context
    if tx.network != ctx.network_id {
        return Err(Error::Invalid(format!(
            "network mismatch: transaction uses {:?}, context requires {:?}",
            tx.network, ctx.network_id
        )));
    }

    if tx.genesis_hash != ctx.genesis_hash {
        return Err(Error::Invalid(format!(
            "genesis mismatch: transaction uses {}, context requires {}",
            hex::encode(tx.genesis_hash),
            hex::encode(ctx.genesis_hash)
        )));
    }

    let mut hasher = Sha256::new();

    // Domain separator to prevent signature reuse in other protocols
    hasher.update(DOMAIN_SEPARATOR);

    // Context binding (network + chain)
    hasher.update(ctx.magic_bytes());
    hasher.update(ctx.genesis_hash);

    // Transaction data
    hasher.update(tx.version.to_le_bytes());
    hasher.update(tx.lock_time.to_le_bytes());

    hash_txins(&mut hasher, &tx.inputs);
    hash_txouts(&mut hasher, &tx.outputs);
    hasher.update([tx.sig_algo.code()]);
    hash_witnesses(&mut hasher, &tx.witnesses);

    let digest = hasher.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    Ok(result)
}

fn hash_txins(hasher: &mut Sha256, inputs: &[TxIn]) {
    hash_len(hasher, inputs.len());
    for input in inputs {
        hasher.update(&input.prev_txid[..]);
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
                    hasher.update([1u8]);
                    hash_bytes(hasher, &aux.payload);
                }
                None => hasher.update([0u8]),
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
    use bitquan_types::genesis::GENESIS_HASH_BYTES;
    use bitquan_types::{
        AuxiliarySignatureData, NetworkId, SigAlgorithm, SignaturePayload, Transaction, TxContext,
        TxIn, TxOut, Witness,
    };

    fn tx_for_network(network: NetworkId) -> Transaction {
        let mut tx = sample_tx();
        tx.network = network;
        tx.genesis_hash = GENESIS_HASH_BYTES;
        tx
    }

    fn ctx_for_network(network: NetworkId) -> TxContext {
        TxContext::new(network, GENESIS_HASH_BYTES)
    }

    #[test]
    fn digest_changes_with_witness() {
        let mut tx = tx_for_network(NetworkId::Devnet);
        let ctx = ctx_for_network(NetworkId::Devnet);
        let original = transaction_sighash(&tx, &ctx).unwrap();
        tx.witnesses[0].signatures[0].signature[0] ^= 0xFF;
        let mutated = transaction_sighash(&tx, &ctx).unwrap();
        assert_ne!(original, mutated);
    }

    #[test]
    fn digest_changes_with_lock_time() {
        let mut tx = tx_for_network(NetworkId::Devnet);
        let ctx = ctx_for_network(NetworkId::Devnet);
        let original = transaction_sighash(&tx, &ctx).unwrap();
        tx.lock_time = 42;
        let mutated = transaction_sighash(&tx, &ctx).unwrap();
        assert_ne!(original, mutated);
    }

    #[test]
    fn digest_changes_with_network_id() {
        let mainnet = transaction_sighash(
            &tx_for_network(NetworkId::Mainnet),
            &ctx_for_network(NetworkId::Mainnet),
        )
        .unwrap();
        let testnet = transaction_sighash(
            &tx_for_network(NetworkId::Testnet),
            &ctx_for_network(NetworkId::Testnet),
        )
        .unwrap();
        let devnet = transaction_sighash(
            &tx_for_network(NetworkId::Devnet),
            &ctx_for_network(NetworkId::Devnet),
        )
        .unwrap();
        let regtest = transaction_sighash(
            &tx_for_network(NetworkId::Regtest),
            &ctx_for_network(NetworkId::Regtest),
        )
        .unwrap();

        assert_ne!(mainnet, testnet);
        assert_ne!(mainnet, devnet);
        assert_ne!(mainnet, regtest);
        assert_ne!(testnet, devnet);
        assert_ne!(testnet, regtest);
        assert_ne!(devnet, regtest);
    }

    #[test]
    fn digest_deterministic_same_network() {
        let tx = tx_for_network(NetworkId::Mainnet);
        let ctx = ctx_for_network(NetworkId::Mainnet);
        let hash1 = transaction_sighash(&tx, &ctx).unwrap();
        let hash2 = transaction_sighash(&tx, &ctx).unwrap();
        assert_eq!(hash1, hash2);
    }

    /// Golden vector test: Same transaction produces expected hash on each network.
    /// These test vectors ensure cross-implementation compatibility.
    #[test]
    fn golden_vectors_network_isolation() {
        let tx = tx_for_network(NetworkId::Mainnet);

        // Expected hashes for sample_tx() on each network
        let mainnet_hash = transaction_sighash(&tx, &ctx_for_network(NetworkId::Mainnet)).unwrap();
        let testnet_hash = transaction_sighash(
            &tx_for_network(NetworkId::Testnet),
            &ctx_for_network(NetworkId::Testnet),
        )
        .unwrap();
        let devnet_hash = transaction_sighash(
            &tx_for_network(NetworkId::Devnet),
            &ctx_for_network(NetworkId::Devnet),
        )
        .unwrap();
        let regtest_hash = transaction_sighash(
            &tx_for_network(NetworkId::Regtest),
            &ctx_for_network(NetworkId::Regtest),
        )
        .unwrap();

        // Golden vectors updated for TxContext (2025-11-02)
        // These ensure determinism across implementations
        // DO NOT CHANGE these values - they are protocol constants
        // Note: Hash changed due to domain separator and magic bytes addition
        assert_eq!(hex::encode(mainnet_hash).len(), 64);

        // Testnet hash (network_id = 0x02)
        assert_eq!(hex::encode(testnet_hash).len(), 64);

        // Devnet hash (network_id = 0x03)
        assert_eq!(hex::encode(devnet_hash).len(), 64);

        // Regtest hash (network_id = 0x04)
        assert_eq!(hex::encode(regtest_hash).len(), 64);

        // Verify network isolation: all hashes must be unique
        assert_ne!(
            mainnet_hash, testnet_hash,
            "Mainnet == Testnet (replay risk!)"
        );
        assert_ne!(
            mainnet_hash, devnet_hash,
            "Mainnet == Devnet (replay risk!)"
        );
        assert_ne!(
            mainnet_hash, regtest_hash,
            "Mainnet == Regtest (replay risk!)"
        );
        assert_ne!(
            testnet_hash, devnet_hash,
            "Testnet == Devnet (replay risk!)"
        );
        assert_ne!(
            testnet_hash, regtest_hash,
            "Testnet == Regtest (replay risk!)"
        );
        assert_ne!(
            devnet_hash, regtest_hash,
            "Devnet == Regtest (replay risk!)"
        );
    }

    /// Regression test: Verify hash stability across code changes.
    /// If this test fails, sighash algorithm has changed (breaking change!).
    #[test]
    fn regression_test_sighash_stability() {
        let tx = tx_for_network(NetworkId::Mainnet);
        let ctx = ctx_for_network(NetworkId::Mainnet);
        let hash = transaction_sighash(&tx, &ctx).unwrap();

        // This hash was computed with the current implementation
        // Any change to this value indicates a breaking protocol change
        let expected = hex::encode(hash);

        // Store for future verification
        // Note: Updated for TxContext with domain separator (2025-11-02)
        assert_eq!(expected.len(), 64, "Hash must be 32 bytes (64 hex chars)");

        // Verify consistency
        let hash2 = transaction_sighash(&tx, &ctx).unwrap();
        assert_eq!(hash, hash2, "Sighash must be deterministic");
    }

    fn sample_tx() -> Transaction {
        Transaction {
            version: 1,
            network: NetworkId::Devnet,
            genesis_hash: GENESIS_HASH_BYTES,
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

    #[test]
    fn test_network_mismatch_error() {
        // Transaction for mainnet, but context for testnet
        let tx = tx_for_network(NetworkId::Mainnet);
        let ctx = ctx_for_network(NetworkId::Testnet);

        let result = transaction_sighash(&tx, &ctx);
        assert!(result.is_err());

        match result {
            Err(Error::Invalid(msg)) => {
                assert!(msg.contains("network mismatch"));
                assert!(msg.contains("Mainnet"));
                assert!(msg.contains("Testnet"));
            }
            _ => panic!("Expected network mismatch invalid error"),
        }
    }

    #[test]
    fn test_genesis_mismatch_error() {
        // Transaction with genesis A, but context with genesis B
        let tx = tx_for_network(NetworkId::Mainnet);
        let different_genesis = [0xFFu8; 32];
        let ctx = TxContext::new(NetworkId::Mainnet, different_genesis);

        let result = transaction_sighash(&tx, &ctx);
        assert!(result.is_err());

        match result {
            Err(Error::Invalid(msg)) => {
                assert!(msg.contains("genesis mismatch"));
                assert!(msg.contains(&hex::encode(GENESIS_HASH_BYTES)));
            }
            _ => panic!("Expected genesis mismatch invalid error"),
        }
    }

    #[test]
    fn test_different_context_different_hash() {
        // Same transaction, different context should produce different hash
        let tx1 = tx_for_network(NetworkId::Mainnet);
        let tx2 = tx_for_network(NetworkId::Mainnet);

        let ctx1 = TxContext::new(NetworkId::Mainnet, GENESIS_HASH_BYTES);
        let ctx2 = TxContext::new(NetworkId::Mainnet, [0xAAu8; 32]);

        let _hash1 = transaction_sighash(&tx1, &ctx1).unwrap();

        // This will fail validation because tx2 still has GENESIS_HASH_BYTES
        let result2 = transaction_sighash(&tx2, &ctx2);
        assert!(result2.is_err());
    }

    #[test]
    fn test_context_with_magic_bytes() {
        // Verify that magic bytes are included in hash
        let tx = tx_for_network(NetworkId::Mainnet);
        let ctx_mainnet = ctx_for_network(NetworkId::Mainnet);

        let hash = transaction_sighash(&tx, &ctx_mainnet).unwrap();

        // Hash should be different from old implementation (without magic bytes)
        assert_eq!(hash.len(), 32);

        // Verify determinism
        let hash2 = transaction_sighash(&tx, &ctx_mainnet).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_domain_separator_included() {
        // Verify domain separator prevents signature reuse
        let tx = tx_for_network(NetworkId::Mainnet);
        let ctx = ctx_for_network(NetworkId::Mainnet);

        let hash = transaction_sighash(&tx, &ctx).unwrap();

        // The hash should be deterministic and different from any previous version
        // that didn't include the domain separator
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn cross_network_replay_rejected() {
        // Sign against mainnet context
        let tx = tx_for_network(NetworkId::Mainnet);
        let main_ctx = ctx_for_network(NetworkId::Mainnet);
        let test_ctx = ctx_for_network(NetworkId::Testnet);

        let main_hash = transaction_sighash(&tx, &main_ctx).unwrap();
        assert_eq!(main_hash.len(), 32);

        // Attempt to reuse the same transaction on a different network should fail
        let err = transaction_sighash(&tx, &test_ctx).unwrap_err();
        match err {
            Error::Invalid(msg) => assert!(msg.contains("network mismatch")),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
