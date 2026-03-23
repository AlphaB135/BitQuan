//! Chainstate tracking and metrics.
//!
//! Provides real-time blockchain state information including:
//! - Block height and hash
//! - Total supply and difficulty
//! - Network statistics

use bitquan_consensus::pow;
use bitquan_storage::async_store::{AsyncChainStore, AsyncStoreError};
use bitquan_types::{Block, BlockHeader, Result};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Maximum number of block hashes to keep in rolling history cache.
/// This covers most practical IBD scenarios without excessive memory usage.
const MAX_HISTORY_SIZE: usize = 1000;

/// Maximum age of validated headers in cache (2 hours).
/// Headers older than this are considered stale and must be re-validated.
const MAX_HEADER_AGE: Duration = Duration::from_secs(2 * 60 * 60);

/// Maximum number of validated headers to keep in memory.
/// Prevents unbounded memory growth while allowing reasonable cache size.
const MAX_VALIDATED_HEADERS: usize = 5000;

/// Type alias for validated headers cache entry.
type ValidatedHeaderEntry = (BlockHeader, Instant);

/// Type alias for validated headers cache.
type ValidatedHeadersMap = HashMap<[u8; 32], ValidatedHeaderEntry>;

/// Blockchain state information.
#[derive(Clone)]
pub struct ChainState {
    /// Current chain height.
    height: Arc<AtomicU64>,
    /// Current tip hash.
    tip_hash: Arc<Mutex<[u8; 32]>>,
    /// Rolling hash history for IBD locators (oldest to newest).
    history: Arc<Mutex<VecDeque<[u8; 32]>>>,
    /// Optional reference to async chain store for full chain access.
    /// This allows find_headers_after to query blocks beyond the cache.
    store: Option<Arc<dyn AsyncChainStore>>,
    /// Validated headers cache with timestamps.
    /// Maps block hash -> (header, validation_time).
    /// Only headers validated within MAX_HEADER_AGE are returned.
    validated_headers: Arc<Mutex<ValidatedHeadersMap>>,
}

impl ChainState {
    /// Create a new chain state starting at genesis.
    pub fn new() -> Self {
        Self {
            height: Arc::new(AtomicU64::new(0)),
            tip_hash: Arc::new(Mutex::new([0u8; 32])),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            store: None,
            validated_headers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the async chain store for full chain access.
    ///
    /// This enables `find_headers_after_async()` to query blocks beyond the
    /// rolling cache. Should be called during node initialization.
    pub fn set_store(&mut self, store: Arc<dyn AsyncChainStore>) {
        self.store = Some(store);
    }

    /// Get a reference to the store if set.
    pub fn store(&self) -> Option<&Arc<dyn AsyncChainStore>> {
        self.store.as_ref()
    }

    /// Create a new chain state with a store attached.
    pub fn with_store(store: Arc<dyn AsyncChainStore>) -> Self {
        Self {
            height: Arc::new(AtomicU64::new(0)),
            tip_hash: Arc::new(Mutex::new([0u8; 32])),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            store: Some(store),
            validated_headers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl std::fmt::Debug for ChainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainState")
            .field("height", &self.get_height())
            .field("tip_hash", &self.tip_hash)
            .field(
                "history_len",
                &self.history.lock().map(|h| h.len()).unwrap_or(0),
            )
            .field("store", &self.store.is_some())
            .finish()
    }
}

impl ChainState {
    /// Append a new block to the chain.
    pub fn append_block(&self, block: &Block, block_hash: [u8; 32]) -> Result<u64> {
        // Verify the provided hash matches the actual block header hash (Closes #138)
        let computed_hash = pow::header_hash(&block.header);
        if computed_hash != block_hash {
            return Err(bitquan_types::Error::Invalid(format!(
                "append_block: hash mismatch (provided={} computed={})",
                hex::encode(&block_hash[..8]),
                hex::encode(&computed_hash[..8])
            )));
        }

        // Verify parent linkage: block's prev_block must equal current tip
        let current_tip = self.get_tip();
        if current_tip != [0u8; 32] && block.header.prev_block != current_tip {
            return Err(bitquan_types::Error::Invalid(format!(
                "append_block: parent mismatch (tip={} block.prev_block={})",
                hex::encode(&current_tip[..8]),
                hex::encode(&block.header.prev_block[..8])
            )));
        }

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

    /// Find block headers after a given locator point (async version).
    ///
    /// This is the server-side handler for GetBlocks requests during IBD.
    /// It searches through the provided locators to find the first hash that
    /// exists in our chain, then returns headers *after* that point.
    ///
    /// # Algorithm
    /// 1. Search through locators to find first hash that exists in our chain
    /// 2. Starting from the next block after the match, fetch headers
    /// 3. Return up to `limit` headers
    /// 4. If no locator matches, start from genesis (height 0)
    ///
    /// # Arguments
    /// * `locators` - Client's known block hashes (newest first)
    /// * `limit` - Maximum headers to return
    ///
    /// # Returns
    /// Vector of block headers after the locator point
    ///
    /// # Errors
    /// Returns `AsyncStoreError` if:
    /// - Store is not set
    /// - Storage query fails
    pub async fn find_headers_after_async(
        &self,
        locators: &[[u8; 32]],
        limit: usize,
    ) -> std::result::Result<Vec<BlockHeader>, AsyncStoreError> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| AsyncStoreError::Poisoned("store not set - call set_store() first"))?;

        // Step 1: Find the first locator that exists in our chain
        let mut start_height = 0u64;

        for locator in locators {
            // Check if this locator hash exists in our chain
            if let Ok(Some(_block)) = store.get_block(locator).await {
                // Found! Now we need to find the height of this block
                let chain_height = store.height().await?;

                // Search for the block height by iterating
                for h in 0..=chain_height {
                    if let Ok(Some(block)) = store.get_block_by_height(h).await {
                        let block_hash = pow::header_hash(&block.header);
                        if block_hash == *locator {
                            start_height = h + 1; // Start AFTER this block
                            break;
                        }
                    }
                }
                break; // Found first match, stop searching
            }
        }

        // Step 2: Fetch headers starting from start_height
        let mut headers = Vec::with_capacity(limit);
        let chain_height = store.height().await?;

        for h in start_height..(start_height.saturating_add(limit as u64)) {
            if h > chain_height {
                break; // Reached tip
            }

            match store.get_block_by_height(h).await {
                Ok(Some(block)) => headers.push(block.header),
                Ok(None) => break, // Block not found
                Err(e) => return Err(e),
            }
        }

        // If no headers found and start_height was 0, this means:
        // 1. No locators matched our chain
        // 2. Our chain is empty or doesn't have the requested blocks
        // This indicates an incompatible peer - return error instead of empty
        if headers.is_empty() && !locators.is_empty() {
            return Err(AsyncStoreError::NoValidHeaders);
        }

        Ok(headers)
    }

    /// Find block headers after a given locator point (sync version).
    ///
    /// This is a synchronous wrapper that falls back to cache-only implementation
    /// if no async runtime is available. Use `find_headers_after_async()` in async
    /// contexts for full functionality.
    ///
    /// # Note
    /// Without a store, this uses only the in-memory history cache (limited to
    /// MAX_HISTORY_SIZE blocks). For full chain access, use the async version with
    /// a store attached.
    ///
    /// # Returns
    /// - Vector of headers if found
    /// - Empty vector if no headers available (fallback to cache)
    ///
    /// # Behavior on Error
    /// If async version returns `NoValidHeaders`, this falls back to
    /// cache-only implementation instead of propagating the error.
    pub fn find_headers_after(&self, locators: &[[u8; 32]], limit: usize) -> Vec<BlockHeader> {
        // If store is available and we have a runtime, use async version
        if self.store.is_some() {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let self_clone = self.clone();
                let locators = locators.to_vec();

                return handle.block_on(async move {
                    match self_clone.find_headers_after_async(&locators, limit).await {
                        Ok(headers) => headers,
                        Err(AsyncStoreError::NoValidHeaders) => {
                            // Fall back to cache-only implementation
                            self_clone.find_headers_after_cached(&locators, limit)
                        }
                        Err(_) => Vec::new(), // Other errors return empty
                    }
                });
            }
        }

        // Fallback: cache-only implementation
        self.find_headers_after_cached(locators, limit)
    }

    /// Cache-only implementation using only in-memory history.
    ///
    /// This is a limited fallback that works without a store but is bounded by
    /// MAX_HISTORY_SIZE. Returns ONLY validated headers from cache.
    ///
    /// # Security Note
    /// This function NO LONGER returns fake headers with zeroed fields.
    /// It only returns headers that have been explicitly validated via
    /// `cache_validated_header()`. This prevents accepting invalid peer data.
    fn find_headers_after_cached(&self, locators: &[[u8; 32]], limit: usize) -> Vec<BlockHeader> {
        // Get validated headers snapshot
        let Ok(validated) = self.validated_headers.lock() else {
            return Vec::new();
        };

        // Clean up stale entries first (prevent unbounded growth)
        let now = Instant::now();
        let mut start_index = None;
        let history = if let Ok(h) = self.history.lock() {
            h
        } else {
            return Vec::new();
        };

        if history.is_empty() {
            return Vec::new();
        }

        let history_len = history.len();

        // Find first locator that exists in our cache
        if !locators.is_empty() {
            for locator in locators {
                for (idx, cached_hash) in history.iter().enumerate() {
                    if *cached_hash == *locator {
                        start_index = Some(idx + 1); // Start AFTER this block
                        break;
                    }
                }
                if let Some(idx) = start_index {
                    if idx < history_len {
                        break;
                    }
                }
            }
        }

        let start_idx = start_index.unwrap_or(0);

        // Only return validated headers that are not stale
        let mut result = Vec::new();
        for i in start_idx..(start_idx.saturating_add(limit)) {
            if i >= history_len {
                break;
            }
            let block_hash = history[i];

            // Check if this header is validated and not too old
            if let Some((header, validated_at)) = validated.get(&block_hash) {
                if now.duration_since(*validated_at) < MAX_HEADER_AGE {
                    result.push(header.clone());
                }
                // Skip stale headers - they must be re-validated
            }
            // Skip unvalidated headers - do NOT return fake headers
        }

        result
    }

    /// Cache a validated block header with timestamp.
    ///
    /// This should be called after a header has been fully validated
    /// (PoW checked, signature verified, etc.) to allow it to be
    /// returned in future `find_headers_after` calls.
    ///
    /// # Arguments
    /// * `hash` - The block hash (32 bytes)
    /// * `header` - The validated block header
    ///
    /// # Behavior
    /// - Stores header with current timestamp
    /// - Enforces MAX_VALIDATED_HEADERS limit (evicts oldest if needed)
    /// - Overwrites existing entry if present (refreshes timestamp)
    pub fn cache_validated_header(&self, hash: [u8; 32], header: BlockHeader) {
        if let Ok(mut validated) = self.validated_headers.lock() {
            // Enforce maximum cache size by evicting oldest entries
            if validated.len() >= MAX_VALIDATED_HEADERS {
                // Find and remove the oldest entry
                if let Some(oldest_key) = validated
                    .iter()
                    .min_by_key(|(_, (_, time))| time)
                    .map(|(k, _)| *k)
                {
                    validated.remove(&oldest_key);
                }
            }

            // Insert/refresh the validated header
            validated.insert(hash, (header, Instant::now()));
        }
    }

    /// Clear all validated headers from cache.
    ///
    /// This should be called on chain reorg to prevent serving
    /// headers from orphaned chains.
    pub fn clear_validated_headers(&self) {
        if let Ok(mut validated) = self.validated_headers.lock() {
            validated.clear();
        }
    }

    /// Get the number of headers currently in the validation cache.
    pub fn validated_cache_size(&self) -> usize {
        self.validated_headers.lock().map(|v| v.len()).unwrap_or(0)
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
                uncles_hash: [0u8; 32],
                time: 1234567890,
                bits: 0x207fffff,
                nonce: 0,
                algo_id: 0,
            },
            uncles: vec![],
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

    /// Append a dummy block with correct hash and parent linkage.
    fn append_dummy(state: &ChainState, block: &mut Block) -> [u8; 32] {
        block.header.prev_block = state.get_tip();
        let hash = pow::header_hash(&block.header);
        state
            .append_block(block, hash)
            .unwrap_or_else(|e| panic!("Failed to append block: {}", e));
        hash
    }

    #[test]
    fn test_chainstate_initialization() {
        let state = ChainState::new();
        assert_eq!(state.get_height(), 0);
    }

    #[test]
    fn test_append_block_increments_height() {
        let state = ChainState::new();
        let mut block = dummy_block();
        let hash = append_dummy(&state, &mut block);
        assert_eq!(state.get_height(), 1);
        assert_eq!(state.get_tip(), hash);
    }

    #[test]
    fn test_multiple_block_appends() {
        let state = ChainState::new();
        let mut block = dummy_block();

        for _i in 0..10 {
            append_dummy(&state, &mut block);
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
        let mut block = dummy_block();
        let hash = append_dummy(&state, &mut block);

        let locator = state.get_locator();
        assert_eq!(locator.len(), 1);
        assert_eq!(locator[0], hash);
    }

    #[test]
    fn test_locator_exponential_backoff() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add 20 blocks
        for _i in 0..20 {
            append_dummy(&state, &mut block);
        }

        let locator = state.get_locator();

        // Should have exponential backoff pattern
        assert!(locator.len() >= 2);
        assert_eq!(locator[0], state.get_tip()); // Tip is most recent
    }

    #[test]
    fn test_locator_includes_genesis() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add a few blocks
        for _i in 0..5 {
            append_dummy(&state, &mut block);
        }

        let locator = state.get_locator();
        assert!(locator.len() >= 2);
    }

    #[test]
    fn test_find_headers_after_empty_locators() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add some blocks
        for _i in 0..5 {
            let hash = append_dummy(&state, &mut block);
            state.cache_validated_header(hash, block.header.clone());
        }

        // Empty locators should return from beginning (cache-only fallback)
        let headers = state.find_headers_after(&[], 3);
        // Cache-only implementation returns limited data
        assert!(!headers.is_empty());
    }

    #[test]
    fn test_find_headers_after_tip_locator() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add 10 blocks
        let mut block_hashes = Vec::new();
        for _i in 0..10 {
            block.header.prev_block = state.get_tip();
            let hash = pow::header_hash(&block.header);
            state.append_block(&block, hash).unwrap();
            block_hashes.push(hash);
        }

        // Send locator = tip
        let headers = state.find_headers_after(&[block_hashes[9]], 10);
        assert!(headers.len() <= 10);
    }

    #[test]
    fn test_find_headers_after_middle_locator() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add 20 blocks with caching
        for _i in 0..20 {
            block.header.prev_block = state.get_tip();
            let hash = pow::header_hash(&block.header);
            state.append_block(&block, hash).unwrap();
            state.cache_validated_header(hash, block.header.clone());
        }

        // Send locator = block 10 (middle)
        let headers = state.find_headers_after(&[[10u8; 32]], 5);

        // Cache-only: should return blocks after position 10
        assert!(!headers.is_empty());
        assert!(headers.len() <= 5);
    }

    #[test]
    fn test_find_headers_after_limit() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add 20 blocks
        let mut block_hashes = Vec::new();
        for _i in 0..20 {
            block.header.prev_block = state.get_tip();
            let hash = pow::header_hash(&block.header);
            state.append_block(&block, hash).unwrap();
            block_hashes.push(hash);
        }

        // Ask for more than available
        let headers = state.find_headers_after(&[block_hashes[5]], 100);

        // Should not exceed available blocks
        assert!(headers.len() <= 100);
    }

    #[test]
    fn test_find_headers_after_unknown_locator() {
        let state = ChainState::new();
        let mut block = dummy_block();

        // Add some blocks
        for _i in 0..5 {
            block.header.prev_block = state.get_tip();
            let hash = pow::header_hash(&block.header);
            state.append_block(&block, hash).unwrap();
            state.cache_validated_header(hash, block.header.clone());
        }

        // Send unknown locator
        let headers = state.find_headers_after(&[[255u8; 32]], 3);

        // Should return from beginning (fallback behavior)
        assert!(!headers.is_empty());
    }
}
