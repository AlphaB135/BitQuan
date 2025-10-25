//! Storage primitives for maintaining chain state.
#![warn(missing_docs)]

use std::collections::HashMap;

use bitquan_types::{Block, BlockHeader};
use thiserror::Error;

/// Errors produced by chain storage backends.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A requested block was not present in the storage backend.
    #[error("block not found")]
    BlockNotFound,
}

/// Interface describing basic blockchain storage operations.
pub trait ChainStore {
    /// Inserts a fully validated block.
    fn insert_block(&mut self, block: Block);
    /// Fetches a block by its header hash (placeholder identifier).
    fn get_block(&self, id: &[u8; 32]) -> Option<&Block>;
    /// Returns the latest known block header.
    fn tip(&self) -> Option<&BlockHeader>;
}

/// In-memory chain store for prototyping and tests.
pub struct InMemoryChainStore {
    blocks: HashMap<[u8; 32], Block>,
    tip: Option<BlockHeader>,
    times: Vec<u32>,
    height: u64,
}

impl InMemoryChainStore {
    /// Creates a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
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
    fn insert_block(&mut self, block: Block) {
        // Use SHA256d(header) as the block id
        let id = header_id(&block.header);
        self.times.push(block.header.time);
        if self.times.len() > 11 { self.times.remove(0); }
        self.height = self.height.saturating_add(1);
        self.tip = Some(block.header.clone());
        self.blocks.insert(id, block);
    }

    fn get_block(&self, id: &[u8; 32]) -> Option<&Block> {
        self.blocks.get(id)
    }

    fn tip(&self) -> Option<&BlockHeader> {
        self.tip.as_ref()
    }
}

impl InMemoryChainStore {
    /// Returns median-time-past of the last up to 11 blocks.
    pub fn mtp(&self) -> Option<u32> {
        if self.times.is_empty() { return None; }
        let mut v = self.times.clone();
        v.sort_unstable();
        Some(v[v.len()/2])
    }
    /// Returns the current height (number of blocks inserted).
    pub fn height(&self) -> u64 { self.height }
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
