//! Chainstate tracking and metrics.
//!
//! Provides real-time blockchain state information including:
//! - Block height and hash
//! - Total supply and difficulty
//! - Network statistics

use bitquan_types::{Block, Result};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Maximum number of block hashes to keep in rolling history cache.
/// This covers most practical IBD scenarios without excessive memory usage.
const MAX_HISTORY_SIZE: usize = 1000;

/// Blockchain state information.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for Phase 8 pool/RPC metrics integration
pub struct ChainState {
    /// Current chain height.
    height: Arc<AtomicU64>,
    /// Current tip hash.
    tip_hash: Arc<Mutex<[u8; 32]>>,
    /// Rolling hash history for IBD locators (oldest to newest).
    history: Arc<Mutex<VecDeque<[u8; 32]>>>,
}

#[allow(dead_code)] // Phase 8 pool/RPC metrics integration
impl ChainState {
    /// Create a new chain state starting at genesis.
    pub fn new() -> Self {
        Self {
            height: Arc::new(AtomicU64::new(0)),
            tip_hash: Arc::new(Mutex::new([0u8; 32])),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
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

        // Add to history with rolling cache (remove oldest if at capacity)
        let mut history = self
            .history
            .lock()
            .map_err(|_| bitquan_types::Error::Invalid("lock poisoned".into()))?;
        if history.len() >= MAX_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(block_hash);

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
    /// The locator follows Bitcoin BIP-37 pattern:
    /// - Start with tip
    /// - Then tip-1, tip-2, tip-4, tip-8, tip-16, ... (double step each time)
    /// - Always include genesis block
    /// - Limit to ~10-12 entries total
    ///
    /// # Returns
    /// Vector of block hashes, newest first. Empty if chain is empty.
    ///
    /// # Implementation
    /// Uses rolling hash cache of last MAX_HISTORY_SIZE blocks. For chains
    /// longer than the cache, the locator will include the oldest cached hash.
    pub fn get_locator(&self) -> Vec<[u8; 32]> {
        let mut locator = Vec::new();
        let height = self.get_height();

        if height == 0 {
            return locator;
        }

        // Get history snapshot (minimize lock time)
        let Ok(history) = self.history.lock() else {
            // Lock poisoned - return empty locator as fallback
            return locator;
        };
        let history_len = history.len();

        if history_len == 0 {
            return locator;
        }

        // Always start with tip (most recent block)
        locator.push(history[history_len - 1]);

        // Exponential backoff: 1, 2, 4, 8, 16, 32, ...
        let mut step = 1u64;
        let mut index = height as i64 - 1 - step as i64;

        // Limit to ~10 entries to prevent excessive locators
        while locator.len() < 10 && index >= 0 {
            let idx = index as usize;
            if idx < history_len {
                locator.push(history[idx]);
            } else {
                // Index outside our history cache - use oldest cached hash
                // This happens when chain is longer than MAX_HISTORY_SIZE
                locator.push(history[0]);
                break;
            }

            step = step.saturating_mul(2);
            index = height as i64 - 1 - step as i64;
        }

        // Always include genesis block (hash[0]) if we have it
        // and it's not already the last entry
        if let Some(&genesis) = history.front() {
            if locator.last() != Some(&genesis) {
                locator.push(genesis);
            }
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

    #[test]
    fn test_locator_empty_chain() {
        let state = ChainState::new();
        let locator = state.get_locator();
        assert_eq!(locator.len(), 0);
    }

    #[test]
    fn test_locator_single_block() {
        let state = ChainState::new();
        let block = dummy_block();
        let hash = [1u8; 32];

        state
            .append_block(&block, hash)
            .unwrap_or_else(|e| panic!("Failed to append block: {}", e));

        let locator = state.get_locator();
        assert_eq!(locator.len(), 1);
        assert_eq!(locator[0], hash);
    }

    #[test]
    fn test_locator_exponential_backoff() {
        let state = ChainState::new();
        let block = dummy_block();

        // Add 20 blocks
        for i in 0..20 {
            let hash = [i as u8; 32];
            state
                .append_block(&block, hash)
                .unwrap_or_else(|e| panic!("Failed to append block: {}", e));
        }

        let locator = state.get_locator();

        // Should have: tip(19), 18, 17, 15, 11, 3, 0(genesis)
        // Or similar exponential backoff pattern
        assert!(locator.len() >= 2);
        assert_eq!(locator[0], [19u8; 32]); // Tip is most recent

        // Genesis should be included
        assert_eq!(locator.last(), Some(&[0u8; 32]));
    }

    #[test]
    fn test_locator_includes_genesis() {
        let state = ChainState::new();
        let block = dummy_block();

        // Add a few blocks
        for i in 0..5 {
            let hash = [i as u8; 32];
            state
                .append_block(&block, hash)
                .unwrap_or_else(|e| panic!("Failed to append block: {}", e));
        }

        let locator = state.get_locator();

        // Genesis (hash[0]) should always be included
        assert!(locator.contains(&[0u8; 32]));
    }
}
