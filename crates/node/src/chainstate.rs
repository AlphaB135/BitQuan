//! Chainstate tracking and metrics.
//!
//! Provides real-time blockchain state information including:
//! - Block height and hash
//! - Total supply and difficulty
//! - Network statistics

use bitquan_types::{Block, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Blockchain state information.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for Phase 8 pool/RPC metrics integration
pub struct ChainState {
    /// Current chain height.
    height: Arc<AtomicU64>,
    /// Current tip hash.
    tip_hash: Arc<Mutex<[u8; 32]>>,
}

#[allow(dead_code)] // Phase 8 pool/RPC metrics integration
impl ChainState {
    /// Create a new chain state starting at genesis.
    pub fn new() -> Self {
        Self {
            height: Arc::new(AtomicU64::new(0)),
            tip_hash: Arc::new(Mutex::new([0u8; 32])),
        }
    }

    /// Append a new block to the chain.
    pub fn append_block(&self, _block: &Block, block_hash: [u8; 32]) -> Result<u64> {
        // Increment height
        let new_height = self.height.fetch_add(1, Ordering::SeqCst) + 1;

        // Update tip hash
        *self
            .tip_hash
            .lock()
            .map_err(|_| bitquan_types::Error::Invalid("lock poisoned".into()))? = block_hash;

        Ok(new_height)
    }

    /// Get current chain height.
    pub fn get_height(&self) -> u64 {
        self.height.load(Ordering::SeqCst)
    }

    /// Get current tip hash.
    pub fn get_tip(&self) -> [u8; 32] {
        self.tip_hash.lock().map(|g| *g).unwrap_or([0u8; 32])
    }

    /// Set height (for testing or initialization).
    pub fn set_height(&self, height: u64) {
        self.height.store(height, Ordering::SeqCst);
    }

    /// Get block locator hashes for IBD (Initial Block Download).
    ///
    /// Returns a list of block hashes from tip backwards (exponentially spaced)
    /// to genesis. This is used by clients to say "this is what I have" when
    /// requesting blocks from a peer.
    ///
    /// # Bitcoin-style Exponential Backoff Pattern
    /// The locator should follow Bitcoin BIP-37 pattern:
    /// - Start with tip
    /// - Then tip-1, tip-2, tip-4, tip-8, tip-16, ... (double step each time)
    /// - Always include genesis block
    /// - Limit to ~10-12 entries total
    ///
    /// # Returns
    /// Vector of block hashes, newest first. Empty if chain is empty.
    ///
    /// # Implementation Note
    /// **STUB**: This implementation only returns the current tip hash.
    ///
    /// Proper exponential backoff requires access to full block history:
    /// - Option 1: Store block hash history in ChainState (memory overhead)
    /// - Option 2: Integrate with ChainStore to query historical blocks
    /// - Option 3: Implement rolling hash cache (last N blocks)
    ///
    /// When implementing, consider using a rolling cache of last 1000 blocks
    /// to cover most practical sync scenarios without storing entire history.
    pub fn get_locator(&self) -> Vec<[u8; 32]> {
        let mut locator = Vec::new();
        let height = self.get_height();

        if height > 0 {
            // Stub: return only the current tip hash
            // Proper implementation requires block history access
            locator.push(self.get_tip());
        }

        locator
    }

    /// Find block headers after a given locator point.
    ///
    /// Used by servers to answer GetBlocks requests. Finds the first hash
    /// in `locators` that matches our chain, then returns headers *after*
    /// that point (up to `limit`).
    ///
    /// # Arguments
    /// * `locators` - List of hashes to search for (client's known blocks)
    /// * `limit` - Maximum number of headers to return
    ///
    /// # Returns
    /// Vector of block headers that come after the locator point.
    ///
    /// # Note
    /// This is a stub implementation. Once ChainState is integrated with
    /// ChainStore, this will query actual blocks from the chain.
    /// For now, returns empty vec.
    pub fn find_headers_after(
        &self,
        _locators: &[[u8; 32]],
        _limit: usize,
    ) -> Vec<bitquan_types::BlockHeader> {
        // TODO: Implement once ChainStore is integrated
        // 1. Find first locator that exists in our chain
        // 2. Fetch blocks after that point (up to limit)
        // 3. Return their headers
        Vec::new()
    }
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use bitquan_types::{BlockHeader, NetworkId, SigAlgorithm, Transaction};

    fn dummy_block() -> Block {
        Block {
            header: BlockHeader {
                version: 1,
                prev_block: [0u8; 32],
                merkle_root: [0u8; 32],
                pqc_agg_hint: [0u8; 32],
                time: 1234567890,
                bits: 0x207fffff,
                nonce: 0,
                algo_id: 0,
            },
            transactions: vec![Transaction {
                version: 1,
                network: NetworkId::Testnet,
                genesis_hash: [0u8; 32],
                lock_time: 0,
                inputs: vec![],
                outputs: vec![],
                sig_algo: SigAlgorithm::Dilithium5,
                witnesses: vec![],
            }],
        }
    }

    #[test]
    fn test_chainstate_initialization() {
        let state = ChainState::new();
        assert_eq!(state.get_height(), 0);
    }

    #[test]
    fn test_append_block_increments_height() {
        let state = ChainState::new();
        let block = dummy_block();
        let hash = [1u8; 32];

        let height = state
            .append_block(&block, hash)
            .unwrap_or_else(|e| panic!("Failed to append block: {}", e));
        assert_eq!(height, 1);
        assert_eq!(state.get_height(), 1);
        assert_eq!(state.get_tip(), hash);
    }

    #[test]
    fn test_multiple_block_appends() {
        let state = ChainState::new();
        let block = dummy_block();

        for i in 0..10 {
            let hash = [i as u8; 32];
            state
                .append_block(&block, hash)
                .unwrap_or_else(|e| panic!("Failed to append block: {}", e));
        }

        assert_eq!(state.get_height(), 10);
    }
}
