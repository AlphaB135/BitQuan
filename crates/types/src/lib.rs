//! Core domain types for the BitQuan protocol.
#![warn(missing_docs)]

mod block;
mod compact_uint;
mod transaction;
pub mod genesis;
pub mod validation;
pub mod wire;

pub use block::{Block, BlockHeader};
pub use block::merkle_root_from_txids as compute_merkle_root_from_txids;
pub use compact_uint::CompactUint;
pub use genesis::{create_genesis_block, is_valid_genesis, GENESIS_HASH, GENESIS_TIME, GENESIS_BITS};
pub use transaction::{
    AuxiliarySignatureData, SigAlgorithm, SignaturePayload, Transaction, TxIn, TxOut, VarBytes,
    Witness,
};
pub use validation::{validate_block_structure, validate_transaction, ValidationError};
pub use wire::{WireDecode, WireEncode, WireError};

/// Alias for the number of post-quantum signatures contained in a payload.
pub type SignatureCount = u64;

/// Returns the total number of signatures across all transactions in a block.
pub fn count_signatures(block: &Block) -> SignatureCount {
    block
        .transactions
        .iter()
        .map(|tx| tx.signature_count() as SignatureCount)
        .sum()
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
}
