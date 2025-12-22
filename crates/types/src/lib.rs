//! Core domain types for the BitQuan protocol.
#![warn(missing_docs)]

mod block;
mod compact_uint;
pub mod context;
pub mod entropy;
pub mod error;
pub mod ext;
pub mod genesis;
pub mod time;
mod transaction;
pub mod validation;
pub mod wire;

pub use block::merkle_root_from_txids as compute_merkle_root_from_txids;
pub use block::merkle_root_from_txids;
pub use block::{Block, BlockHeader};
pub use compact_uint::CompactUint;
pub use context::TxContext;
pub use error::{Error, Result};
pub use ext::{OptionExt, ResultExt};
pub use genesis::{
    create_genesis_block, is_valid_genesis, GENESIS_BITS, GENESIS_HASH, GENESIS_TIME,
};
pub use transaction::{
    AuxiliarySignatureData, NetworkId, SigAlgorithm, SignaturePayload, Transaction, TxIn, TxOut,
    VarBytes, Witness,
};
pub use validation::{validate_block_structure, validate_transaction, ValidationError};
pub use wire::{WireDecode, WireEncode, WireError};

/// Alias for the number of post-quantum signatures contained in a payload.
pub type SignatureCount = u64;

/// Returns the total number of signatures across all transactions in a block.
/// Returns 0 if overflow is detected (defensive programming).
pub fn count_signatures(block: &Block) -> SignatureCount {
    block
        .transactions
        .iter()
        .try_fold(0 as SignatureCount, |acc, tx| {
            let count = tx.signature_count().ok()? as SignatureCount;
            acc.checked_add(count)
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_uint_encoded_length_matches_reference_values() {
        assert_eq!(CompactUint::from(0_u64).encoded_length(), 1);
        assert_eq!(CompactUint::from(252_u64).encoded_length(), 1);
        assert_eq!(CompactUint::from(253_u64).encoded_length(), 3);
        assert_eq!(CompactUint::from(65_535_u64).encoded_length(), 3);
        assert_eq!(CompactUint::from(65_536_u64).encoded_length(), 5);
    }

    fn sample_tx() -> Transaction {
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
            aux: Some(AuxiliarySignatureData {
                payload: vec![0xEF],
            }),
        };

        let witness = Witness {
            signatures: vec![sig],
        };

        Transaction {
            version: 2,
            network: NetworkId::Devnet,
            genesis_hash: genesis::GENESIS_HASH_BYTES,
            lock_time: 0,
            inputs: vec![tx_in],
            outputs: vec![tx_out],
            sig_algo: SigAlgorithm::Dilithium5,
            witnesses: vec![witness],
        }
    }

    #[test]
    fn test_normal_tx_size_calculations_still_work() {
        let tx = sample_tx();

        // All size calculations should succeed for normal transactions
        assert!(tx.serialized_size_hint().is_ok());
        assert!(tx.witness_size_hint().is_ok());
        assert!(tx.signature_count().is_ok());

        let size = tx
            .serialized_size_hint()
            .expect("Failed to get transaction size");
        let witness_size = tx.witness_size_hint().expect("Failed to get witness size");
        let sig_count = tx.signature_count().expect("Failed to get signature count");

        assert!(size > 0);
        assert!(witness_size > 0);
        assert_eq!(sig_count, 1);
    }

    #[test]
    fn test_tx_size_overflow_protection() {
        // Create transaction with many large inputs
        let mut inputs = Vec::new();
        for i in 0..1000 {
            inputs.push(TxIn {
                prev_txid: [i as u8; 32],
                prev_vout: i,
                sequence: 0xffffffff,
                script_sig: vec![0u8; 10000],
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
            sig_algo: SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        // Should either succeed or detect overflow
        let result = tx.serialized_size_hint();
        assert!(result.is_ok() || matches!(result, Err(ValidationError::SizeOverflow(_))));
    }

    #[test]
    fn test_signature_count_overflow() {
        // Create transaction with many witnesses (but not too many to OOM)
        let mut witnesses = Vec::new();
        for _ in 0..10000 {
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
            sig_algo: SigAlgorithm::Dilithium5,
            witnesses,
        };

        // Should successfully count signatures
        let result = tx.signature_count();
        assert!(result.is_ok());
        assert_eq!(result.expect("Failed to get signature count"), 10000);
    }
}
