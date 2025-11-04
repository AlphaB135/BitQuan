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
    /// Proof-of-Work algorithm identifier (0=SHA-256d, 1=RandomX).
    /// Added for hybrid PoW support (testnet-only).
    pub algo_id: u8,
}

impl BlockHeader {
    /// Returns the serialized size of the header (always fixed length).
    pub const fn serialized_size(&self) -> usize {
        4 + 32 + 32 + 32 + 4 + 4 + 8 + 1 // version + prev + merkle + pqc + time + bits + nonce + algo_id
    }

    /// Serializes the header to bytes (little-endian fields per wire format).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            Self {
                version: 0,
                prev_block: [0u8; 32],
                merkle_root: [0u8; 32],
                pqc_agg_hint: [0u8; 32],
                time: 0,
                bits: 0,
                nonce: 0,
                algo_id: 0,
            }
            .serialized_size(),
        );
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.prev_block);
        out.extend_from_slice(&self.merkle_root);
        out.extend_from_slice(&self.pqc_agg_hint);
        out.extend_from_slice(&self.time.to_le_bytes());
        out.extend_from_slice(&self.bits.to_le_bytes());
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out.push(self.algo_id);
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

    /// Computes the witness root (merkle over wtxids) of the transactions in this block.
    pub fn compute_witness_root(&self) -> [u8; 32] {
        let wtxids: Vec<[u8; 32]> = self.transactions.iter().map(|tx| tx.wtxid()).collect();
        merkle_root_from_txids(&wtxids)
    }

    /// Returns a best-effort serialized size hint including all transactions.
    pub fn serialized_size_hint(&self) -> Result<usize, crate::ValidationError> {
        let tx_count = self.tx_count();
        let tx_count_len = CompactUint::from_usize(tx_count).encoded_length();

        let txs_len = self.transactions.iter().try_fold(0usize, |acc, tx| {
            let tx_size = tx.serialized_size_hint()?;
            acc.checked_add(tx_size)
                .ok_or(crate::ValidationError::SizeOverflow("block transactions"))
        })?;

        self.header
            .serialized_size()
            .checked_add(tx_count_len)
            .and_then(|v| v.checked_add(txs_len))
            .ok_or(crate::ValidationError::SizeOverflow("block total size"))
    }
}

/// Computes merkle root (Bitcoin-style) from a slice of txids.
///
/// Security: Prevents CVE-2012-2459 style duplicate attacks by rejecting
/// duplicate internal nodes and odd-length layers without duplication.
pub fn merkle_root_from_txids(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }

    let mut layer: Vec<[u8; 32]> = txids.to_vec();

    // Detect duplicates in the input layer (invalid block)
    for i in 0..layer.len() {
        for j in (i + 1)..layer.len() {
            if layer[i] == layer[j] {
                // Duplicate transaction IDs are not allowed
                return [0u8; 32];
            }
        }
    }

    while layer.len() > 1 {
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;

        while i < layer.len() {
            let a = layer[i];
            let b = if i + 1 < layer.len() {
                layer[i + 1]
            } else {
                // Odd length: hash with itself but mark this condition
                // For safety, we enforce that there must be at least 2 elements
                // or we're at the final merge
                if layer.len() > 1 {
                    // This is a security risk - reject odd-sized internal layers
                    // to prevent merkle tree manipulation
                    return [0u8; 32];
                }
                a
            };

            let mut data = [0u8; 64];
            data[..32].copy_from_slice(&a);
            data[32..].copy_from_slice(&b);
            let h1 = Sha256::digest(data);
            let h2 = Sha256::digest(h1);
            let mut out = [0u8; 32];
            out.copy_from_slice(&h2);
            next.push(out);
            i += 2;
        }
        layer = next;
    }
    layer[0]
}
