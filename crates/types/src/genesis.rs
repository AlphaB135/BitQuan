//! Genesis block constants and utilities for BitQuan blockchain.

use crate::{Block, BlockHeader, NetworkId, SigAlgorithm, Transaction, TxIn, TxOut};
use sha2::{Digest, Sha256};

/// Genesis block timestamp (Unix epoch)
/// Jan 1, 2025 00:00:00 UTC
pub const GENESIS_TIME: u32 = 1735689600;

/// Genesis block bits (mainnet difficulty)
pub const GENESIS_BITS: u32 = 0x1c00ffff;

/// Genesis block version
pub const GENESIS_VERSION: i32 = 1;

/// Genesis coinbase message
pub const GENESIS_MESSAGE: &[u8] =
    b"The Quantum Age Begins - 1 Jan 2025. Ownerless. Verifiable. For everyone.";

/// Genesis block reward (50 BQ)
pub const GENESIS_REWARD: u64 = 5_000_000_000; // 50 BQ in qbits

/// Genesis block nonce discovered during genesis mining
pub const GENESIS_NONCE: u64 = 2;

/// Genesis block hash (double SHA256, displayed big-endian)
/// Updated for Jan 1, 2025 launch
pub const GENESIS_HASH: &str = "cac0577af8fb0f988f64dbdd9b79c36d25f5cc7208dd66f8c97daa46bb9ec583";

/// Genesis block hash bytes (big-endian)
pub const GENESIS_HASH_BYTES: [u8; 32] = [
    0xca, 0xc0, 0x57, 0x7a, 0xf8, 0xfb, 0x0f, 0x98, 0x8f, 0x64, 0xdb, 0xdd, 0x9b, 0x79, 0xc3, 0x6d,
    0x25, 0xf5, 0xcc, 0x72, 0x08, 0xdd, 0x66, 0xf8, 0xc9, 0x7d, 0xaa, 0x46, 0xbb, 0x9e, 0xc5, 0x83,
];

/// Genesis hash value embedded in coinbase transactions for replay protection.
pub const GENESIS_EMBEDDED_HASH_BYTES: [u8; 32] = [
    0x00, 0x00, 0x00, 0x5c, 0xeb, 0x7f, 0x52, 0x7d, 0x22, 0xa5, 0xbf, 0xb5, 0xbc, 0x57, 0x8f, 0xf1,
    0x6c, 0x27, 0xb6, 0x2c, 0x75, 0xa6, 0x3b, 0x48, 0x0d, 0x7e, 0x71, 0x9c, 0xe6, 0x55, 0x35, 0xd6,
];

/// Creates the genesis block for BitQuan blockchain
pub fn create_genesis_block() -> Block {
    // Genesis coinbase transaction
    let coinbase_in = TxIn {
        prev_txid: [0u8; 32],
        prev_vout: 0xffffffff,
        script_sig: GENESIS_MESSAGE.to_vec(),
        sequence: 0xffffffff,
    };

    // Genesis output - OP_RETURN (burned, no one can spend)
    // This ensures fair launch with no premine
    let coinbase_out = TxOut {
        value: GENESIS_REWARD,
        script_pubkey: vec![0x6a], // OP_RETURN
    };

    let coinbase_tx = Transaction {
        version: 2,
        network: NetworkId::Mainnet,
        genesis_hash: GENESIS_EMBEDDED_HASH_BYTES,
        lock_time: 0,
        inputs: vec![coinbase_in],
        outputs: vec![coinbase_out],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    };

    // Calculate merkle roots
    let merkle_root = compute_merkle_root(&[coinbase_tx.txid()]);
    let witness_root = compute_merkle_root(&[coinbase_tx.wtxid()]);

    // Genesis block header
    let header = BlockHeader {
        version: GENESIS_VERSION,
        prev_block: [0u8; 32], // No previous block
        merkle_root,
        pqc_agg_hint: witness_root,
        time: GENESIS_TIME,
        bits: GENESIS_BITS,
        nonce: GENESIS_NONCE,
        algo_id: 0, // Genesis always uses SHA-256d
    };

    Block {
        header,
        transactions: vec![coinbase_tx],
    }
}

/// Compute merkle root from transaction IDs
fn compute_merkle_root(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }
    if txids.len() == 1 {
        return txids[0];
    }

    let mut level = txids.to_vec();
    while level.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in level.chunks(2) {
            let hash = if chunk.len() == 2 {
                // Hash pair
                let mut hasher = Sha256::new();
                hasher.update(chunk[0]);
                hasher.update(chunk[1]);
                let result = hasher.finalize();
                let mut out = [0u8; 32];
                out.copy_from_slice(&result);
                out
            } else {
                // Odd number, hash with itself
                let mut hasher = Sha256::new();
                hasher.update(chunk[0]);
                hasher.update(chunk[0]);
                let result = hasher.finalize();
                let mut out = [0u8; 32];
                out.copy_from_slice(&result);
                out
            };
            next_level.push(hash);
        }

        level = next_level;
    }

    level[0]
}

/// Validates that a block is the correct genesis block
pub fn is_valid_genesis(block: &Block) -> bool {
    let genesis = create_genesis_block();

    // Compare all core header fields and ensure the canonical nonce is used
    block.header.version == genesis.header.version
        && block.header.prev_block == genesis.header.prev_block
        && block.header.merkle_root == genesis.header.merkle_root
        && block.header.pqc_agg_hint == genesis.header.pqc_agg_hint
        && block.header.time == genesis.header.time
        && block.header.bits == genesis.header.bits
        && block.header.nonce == GENESIS_NONCE
        && block.transactions.len() == 1
        && block.transactions[0].inputs[0].script_sig == GENESIS_MESSAGE
}

/// Gets the genesis block hash (after mining)
pub fn genesis_hash() -> [u8; 32] {
    GENESIS_HASH_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block_creation() {
        let genesis = create_genesis_block();

        assert_eq!(genesis.header.version, GENESIS_VERSION);
        assert_eq!(genesis.header.time, GENESIS_TIME);
        assert_eq!(genesis.header.bits, GENESIS_BITS);
        assert_eq!(genesis.header.prev_block, [0u8; 32]);
        assert_eq!(genesis.transactions.len(), 1);
    }

    #[test]
    fn test_genesis_coinbase() {
        let genesis = create_genesis_block();
        let coinbase = &genesis.transactions[0];

        assert_eq!(coinbase.inputs.len(), 1);
        assert_eq!(coinbase.outputs.len(), 1);
        assert_eq!(coinbase.outputs[0].value, GENESIS_REWARD);
        assert_eq!(coinbase.inputs[0].script_sig, GENESIS_MESSAGE);
    }

    #[test]
    fn test_genesis_validation() {
        let genesis = create_genesis_block();
        assert!(is_valid_genesis(&genesis));
    }

    #[test]
    fn test_genesis_hash_matches_constant() {
        let genesis = create_genesis_block();
        let bytes = genesis.header.to_bytes();
        let first = Sha256::digest(&bytes);
        let second = Sha256::digest(first);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&second);
        assert_eq!(hex::encode(hash), GENESIS_HASH);
    }

    #[test]
    fn test_merkle_root_single() {
        let txid = [0x42u8; 32];
        let root = compute_merkle_root(&[txid]);
        assert_eq!(root, txid);
    }

    #[test]
    fn test_merkle_root_pair() {
        let txid1 = [0x01u8; 32];
        let txid2 = [0x02u8; 32];
        let root = compute_merkle_root(&[txid1, txid2]);

        // Should be SHA256(SHA256(txid1 || txid2))
        assert_ne!(root, [0u8; 32]);
        assert_ne!(root, txid1);
        assert_ne!(root, txid2);
    }
}
