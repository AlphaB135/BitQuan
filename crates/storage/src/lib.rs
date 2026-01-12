//! Storage primitives for maintaining chain state.
#![warn(missing_docs)]

use std::collections::HashMap;

use bitquan_types::{Block, BlockHeader, Transaction};
use thiserror::Error;

#[cfg(feature = "rocksdb-backend")]
pub mod rocksdb_store;

#[cfg(feature = "rocksdb-backend")]
pub use rocksdb_store::{DatabaseStats, RecoveryOptions, RocksDBStore};

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
    by_height: Vec<Block>,  // Track blocks by height for IBD
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
