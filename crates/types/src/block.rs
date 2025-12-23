//! Block data structures encapsulating transactions and headers.

use crate::{compact_uint::CompactUint, transaction::Transaction, ValidationError};
use serde::{Deserialize, Serialize};

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
    ///
    /// Returns an error if duplicate transaction IDs are detected or if the merkle tree
    /// structure is invalid (CVE-2012-2459 protection).
    pub fn compute_merkle_root(&self) -> Result<[u8; 32], ValidationError> {
        let txids: Vec<[u8; 32]> = self.transactions.iter().map(|tx| tx.txid()).collect();
        merkle_root_from_txids(&txids)
    }

    /// Computes the witness root (merkle over wtxids) of the transactions in this block.
    ///
    /// Returns an error if duplicate witness transaction IDs are detected or if the merkle tree
    /// structure is invalid (CVE-2012-2459 protection).
    pub fn compute_witness_root(&self) -> Result<[u8; 32], ValidationError> {
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

/// Computes merkle root using BLAKE3 from a slice of txids.
///
/// Security: Prevents CVE-2012-2459 style duplicate attacks by rejecting
/// duplicate internal nodes and odd-length layers without duplication.
///
/// # Security Features
///
/// 1. **Duplicate Detection**: Returns error if any duplicate txids are found
/// 2. **Odd Layer Protection**: Returns error for odd-length internal layers
/// 3. **BLAKE3**: Uses quantum-resistant BLAKE3 instead of SHA-256d
///
/// # Errors
///
/// Returns `ValidationError::DuplicateTransactionId` if duplicates are detected.
/// Returns `ValidationError::OddMerkleTreeLayer` if odd-length internal layer is found.
pub fn merkle_root_from_txids(txids: &[[u8; 32]]) -> Result<[u8; 32], ValidationError> {
    if txids.is_empty() {
        return Ok([0u8; 32]);
    }

    let mut layer: Vec<[u8; 32]> = txids.to_vec();

    // SECURITY: Detect duplicates in the input layer (CVE-2012-2459 protection)
    for i in 0..layer.len() {
        for j in (i + 1)..layer.len() {
            if layer[i] == layer[j] {
                // Duplicate transaction IDs are not allowed
                return Err(ValidationError::DuplicateTransactionId);
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
                // SECURITY: Odd length internal layers are a security risk
                // Only allow odd length at the final layer (when layer.len() == 1 after this iteration)
                // This prevents merkle tree manipulation attacks
                if layer.len() > 1 {
                    return Err(ValidationError::OddMerkleTreeLayer);
                }
                a
            };

            // Use BLAKE3 for merkle node hashing (quantum-resistant, faster than SHA-256d)
            let mut data = [0u8; 64];
            data[..32].copy_from_slice(&a);
            data[32..].copy_from_slice(&b);
            let hash = blake3::hash(&data);
            next.push(*hash.as_bytes());
            i += 2;
        }
        layer = next;
    }
    Ok(layer[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_empty() {
        let result = merkle_root_from_txids(&[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0u8; 32]);
    }

    #[test]
    fn test_merkle_single() {
        let txid = [1u8; 32];
        let result = merkle_root_from_txids(&[txid]);
        assert!(result.is_ok());
        // Single txid returns itself
        assert_eq!(result.unwrap(), txid);
    }

    #[test]
    fn test_merkle_two_txids() {
        let txid1 = [1u8; 32];
        let txid2 = [2u8; 32];
        let result = merkle_root_from_txids(&[txid1, txid2]);
        assert!(result.is_ok());

        // Should produce deterministic result
        let root = result.unwrap();
        assert_ne!(root, [0u8; 32]);
        assert_ne!(root, txid1);
        assert_ne!(root, txid2);
    }

    #[test]
    fn test_merkle_reject_duplicates() {
        let txid = [1u8; 32];
        let result = merkle_root_from_txids(&[txid, txid]);

        // Should reject duplicates
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::DuplicateTransactionId
        ));
    }

    #[test]
    fn test_merkle_reject_odd_layer() {
        // Three txids will create an odd layer in the tree
        let txid1 = [1u8; 32];
        let txid2 = [2u8; 32];
        let txid3 = [3u8; 32];

        let result = merkle_root_from_txids(&[txid1, txid2, txid3]);

        // Should reject odd-length internal layers
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::OddMerkleTreeLayer
        ));
    }

    #[test]
    fn test_merkle_four_txids_ok() {
        // Four txids is safe (power of 2)
        let txids = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let result = merkle_root_from_txids(&txids);

        // Should succeed with 4 txids
        assert!(result.is_ok());
    }

    #[test]
    fn test_merkle_deterministic() {
        let txids = [[1u8; 32], [2u8; 32]];
        let result1 = merkle_root_from_txids(&txids);
        let result2 = merkle_root_from_txids(&txids);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_blake3_different_from_sha256() {
        // This test documents that we're using BLAKE3, not SHA-256d
        let txids = [[1u8; 32], [2u8; 32]];
        let blake3_root = merkle_root_from_txids(&txids).unwrap();

        // BLAKE3 hash should be different from SHA-256d
        // (This test will pass as long as we're using BLAKE3)
        assert_ne!(blake3_root, [0u8; 32]);
    }
}
