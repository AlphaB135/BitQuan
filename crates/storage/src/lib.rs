//! Storage primitives for maintaining chain state.
#![warn(missing_docs)]

use std::collections::HashMap;

use bitquan_types::{Block, BlockHeader, Transaction};
use log::error;
use thiserror::Error;

#[cfg(feature = "rocksdb-backend")]
pub mod rocksdb_store;

#[cfg(feature = "rocksdb-backend")]
pub use rocksdb_store::{serialize, DatabaseStats, RecoveryOptions, RocksDBStore, StoredUtxoEntry};

/// Undo block functionality for rolling back blockchain state
pub mod undo_block;
pub use undo_block::{SpentOutput, UndoBlock};

pub mod async_store;
pub use async_store::{AsyncChainStore, AsyncResult, AsyncStoreError, AsyncStoreWrapper};

/// Errors produced by chain storage backends.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A requested block was not present in the storage backend.
    #[error("block not found")]
    BlockNotFound,
    /// Transaction not found
    #[error("transaction not found")]
    TxNotFound,
    /// Database I/O error
    #[error("database error: {0}")]
    DatabaseError(String),
    /// Serialization error
    #[error("serialization error: {0}")]
    SerializationError(String),
    /// Pruning error - cannot prune blocks needed for reorg safety
    #[error("cannot prune below minimum depth of {0} blocks")]
    PruningDepthError(u64),
    /// Block data has been pruned and is unavailable
    #[error("block data pruned at height {height}, only headers available")]
    BlockDataPruned {
        /// The block height that was pruned
        height: u64,
    },
}

/// Pruning mode for blockchain storage.
///
/// Controls how much historical block data is retained on disk.
/// Headers are always kept for SPV verification, but full block
/// data (transactions, witnesses) can be pruned to save space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PruningMode {
    /// Keep all blocks (full node).
    ///
    /// No pruning is performed. All historical block data is retained.
    #[default]
    Full,
    /// Keep only the last N blocks of full data.
    ///
    /// Blocks older than `keep_blocks` are pruned, keeping only headers.
    /// The minimum safe value is 1000 to protect against reorgs.
    Pruned {
        /// Number of recent blocks to keep (minimum 1000)
        keep_blocks: u64,
    },
    /// Keep only headers and UTXO set (minimum storage mode).
    ///
    /// All block bodies are pruned, leaving only headers and the UTXO set.
    /// This is the most space-efficient mode but cannot serve historical
    /// block data to other nodes.
    UtxoOnly,
}

impl PruningMode {
    /// Returns the minimum safe depth for pruning (1000 blocks).
    ///
    /// This protects against chain reorganizations up to 1000 blocks deep.
    /// At 15s block time, this represents approximately 4 hours of chain history.
    pub const MIN_SAFE_DEPTH: u64 = 1000;

    /// Returns the number of blocks to keep based on the pruning mode.
    ///
    /// - `Full`: Returns `u64::MAX` (keep everything)
    /// - `Pruned { keep_blocks }`: Returns the configured value
    /// - `UtxoOnly`: Returns 0 (keep only headers)
    pub fn keep_blocks(&self) -> u64 {
        match self {
            PruningMode::Full => u64::MAX,
            PruningMode::Pruned { keep_blocks } => *keep_blocks,
            PruningMode::UtxoOnly => 0,
        }
    }

    /// Returns true if this mode prunes any block data.
    pub fn is_pruned(&self) -> bool {
        !matches!(self, PruningMode::Full)
    }

    /// Validates that the pruning mode configuration is safe.
    ///
    /// Returns an error if `keep_blocks` is less than `MIN_SAFE_DEPTH`.
    pub fn validate(&self) -> Result<(), StorageError> {
        if let PruningMode::Pruned { keep_blocks } = self {
            if *keep_blocks < Self::MIN_SAFE_DEPTH {
                return Err(StorageError::PruningDepthError(Self::MIN_SAFE_DEPTH));
            }
        }
        Ok(())
    }
}

/// Metadata about the current pruning state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PruningMetadata {
    /// The current pruning mode.
    pub mode: PruningMode,
    /// The height below which blocks have been pruned (if any).
    pub pruning_height: Option<u64>,
    /// Unix timestamp of last pruning operation.
    pub last_pruned: u64,
    /// Number of blocks that have been pruned total.
    pub total_pruned: u64,
}

impl PruningMetadata {
    /// Creates a new pruning metadata for the given mode.
    pub fn new(mode: PruningMode) -> Self {
        Self {
            mode,
            pruning_height: None,
            last_pruned: 0,
            total_pruned: 0,
        }
    }

    /// Returns true if any blocks have been pruned.
    pub fn is_pruned(&self) -> bool {
        self.pruning_height.is_some()
    }

    /// Updates the pruning metadata after a pruning operation.
    pub fn record_pruning(&mut self, new_pruning_height: u64, blocks_pruned: u64) {
        self.pruning_height = Some(new_pruning_height);
        self.last_pruned = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|e| {
                // System time went backwards - use current timestamp as fallback
                eprintln!("System clock error in record_pruning: {}", e);
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
            })
            .as_secs();
        self.total_pruned = self.total_pruned.saturating_add(blocks_pruned);
    }
}

impl From<crate::async_store::AsyncStoreError> for StorageError {
    fn from(err: crate::async_store::AsyncStoreError) -> Self {
        match err {
            crate::async_store::AsyncStoreError::Storage(s) => s,
            crate::async_store::AsyncStoreError::TaskSpawn(_) => {
                StorageError::DatabaseError("Task spawn failed".to_string())
            }
            crate::async_store::AsyncStoreError::Poisoned(s) => {
                StorageError::DatabaseError(format!("Poisoned mutex: {}", s))
            }
            crate::async_store::AsyncStoreError::Cancelled => {
                StorageError::DatabaseError("Operation cancelled".to_string())
            }
            crate::async_store::AsyncStoreError::NoValidHeaders => StorageError::DatabaseError(
                "No valid headers found - peer chain incompatible".to_string(),
            ),
        }
    }
}

/// Interface describing basic blockchain storage operations.
pub trait ChainStore {
    /// Inserts a fully validated block.
    fn insert_block(&mut self, block: Block) -> Result<(), StorageError>;
    /// Disconnects a block, rolling back its changes.
    fn disconnect_block(&mut self, block: &Block) -> Result<(), StorageError>;
    /// Fetches a block by its header hash.
    fn get_block(&self, id: &[u8; 32]) -> Result<Option<Block>, StorageError>;
    /// Returns the latest known block header.
    fn tip(&self) -> Result<Option<BlockHeader>, StorageError>;
    /// Get block by height
    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>, StorageError>;
    /// Get transaction by txid
    fn get_transaction(&self, txid: &[u8; 32]) -> Result<Option<Transaction>, StorageError>;
    /// Store UTXO entry
    fn put_utxo(&mut self, outpoint: &[u8], data: &[u8]) -> Result<(), StorageError>;
    /// Get UTXO entry
    fn get_utxo(&self, outpoint: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    /// Delete UTXO entry
    fn delete_utxo(&mut self, outpoint: &[u8]) -> Result<(), StorageError>;
}

/// In-memory chain store for prototyping and tests.
pub struct InMemoryChainStore {
    blocks: HashMap<[u8; 32], Block>,
    by_height: Vec<Block>, // Track blocks by height for IBD
    tip: Option<BlockHeader>,
    times: Vec<u32>,
    height: u64,
}

impl InMemoryChainStore {
    /// Creates a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            by_height: Vec::new(),
            tip: None,
            times: Vec::new(),
            height: 0,
        }
    }
}

impl Default for InMemoryChainStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainStore for InMemoryChainStore {
    fn insert_block(&mut self, block: Block) -> Result<(), StorageError> {
        let id = header_id(&block.header);
        self.times.push(block.header.time);
        if self.times.len() > 11 {
            self.times.remove(0);
        }
        self.height = self.height.saturating_add(1);
        self.tip = Some(block.header.clone());

        // Store by hash and by height
        self.blocks.insert(id, block.clone());
        self.by_height.push(block);

        Ok(())
    }

    fn disconnect_block(&mut self, _block: &Block) -> Result<(), StorageError> {
        // Not implemented for in-memory store, but required for trait
        Err(StorageError::DatabaseError(
            "disconnect_block is not supported in InMemoryChainStore".into(),
        ))
    }

    fn get_block(&self, id: &[u8; 32]) -> Result<Option<Block>, StorageError> {
        Ok(self.blocks.get(id).cloned())
    }

    fn tip(&self) -> Result<Option<BlockHeader>, StorageError> {
        Ok(self.tip.clone())
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>, StorageError> {
        // by_height is 0-indexed (height 0 is at index 0)
        if height < self.by_height.len() as u64 {
            Ok(Some(self.by_height[height as usize].clone()))
        } else {
            Ok(None)
        }
    }

    fn get_transaction(&self, _txid: &[u8; 32]) -> Result<Option<Transaction>, StorageError> {
        // Not implemented for in-memory store
        Ok(None)
    }

    fn put_utxo(&mut self, _outpoint: &[u8], _data: &[u8]) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_utxo(&self, _outpoint: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(None)
    }

    fn delete_utxo(&mut self, _outpoint: &[u8]) -> Result<(), StorageError> {
        Ok(())
    }
}

impl InMemoryChainStore {
    /// Returns median-time-past of the last up to 11 blocks.
    pub fn mtp(&self) -> Option<u32> {
        if self.times.is_empty() {
            return None;
        }
        let mut v = self.times.clone();
        v.sort_unstable();
        Some(v[v.len() / 2])
    }
    /// Returns the current height (number of blocks inserted).
    pub fn height(&self) -> u64 {
        self.height
    }
}

fn header_id(h: &bitquan_types::BlockHeader) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let bytes = h.to_bytes();
    let first = Sha256::digest(bytes);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}
