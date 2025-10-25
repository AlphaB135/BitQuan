//! Block data structures encapsulating transactions and headers.

use crate::{compact_uint::CompactUint, transaction::Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Block header committed to by miners or validators.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block version.
    pub version: i32,
    /// Hash of the previous block header.
    pub prev_block: [u8; 32],
    /// Merkle root of the block's transactions.
    pub merkle_root: [u8; 32],
    /// Reserved commitment for aggregate PQ signatures or future extensions.
    pub pqc_agg_hint: [u8; 32],
    /// Unix timestamp when the block was produced.
    pub time: u32,
    /// Compact representation of the PoW difficulty target.
    pub bits: u32,
    /// Expanded nonce size (64-bit) to support larger search spaces.
    pub nonce: u64,
}

impl BlockHeader {
    /// Returns the serialized size of the header (always fixed length).
    pub const fn serialized_size(&self) -> usize {
        4 + 32 + 32 + 32 + 4 + 4 + 8
    }

    /// Serializes the header to bytes (little-endian fields per wire format).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self {
            version: 0,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 0,
            bits: 0,
            nonce: 0,
        }
        .serialized_size());
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.prev_block);
        out.extend_from_slice(&self.merkle_root);
        out.extend_from_slice(&self.pqc_agg_hint);
        out.extend_from_slice(&self.time.to_le_bytes());
        out.extend_from_slice(&self.bits.to_le_bytes());
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out
    }
}

/// Full block containing a header and ordered transactions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Block header.
    pub header: BlockHeader,
    /// Transactions included in this block.
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Returns the number of transactions.
    pub fn tx_count(&self) -> usize {
        self.transactions.len()
    }

    /// Computes the merkle root from txids of the transactions in this block.
    pub fn compute_merkle_root(&self) -> [u8; 32] {
        let txids: Vec<[u8; 32]> = self.transactions.iter().map(|tx| tx.txid()).collect();
        merkle_root_from_txids(&txids)
    }

    /// Returns a best-effort serialized size hint including all transactions.
    pub fn serialized_size_hint(&self) -> usize {
        let tx_count = self.tx_count();
        let tx_count_len = CompactUint::from_usize(tx_count).encoded_length();

        let txs_len = self
            .transactions
            .iter()
            .map(Transaction::serialized_size_hint)
            .sum::<usize>();

        self.header.serialized_size() + tx_count_len + txs_len
    }
}
