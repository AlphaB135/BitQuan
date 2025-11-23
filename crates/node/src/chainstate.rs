//! Chainstate tracking and metrics.
//!
//! Provides real-time blockchain state information including:
//! - Block height and hash
//! - Total supply and difficulty
//! - Network statistics

#[cfg(feature = "pool")]
use crate::pool_db::PoolDatabase;
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
    /// Pool database for persistence.
    db: Option<PoolDatabase>,
}

#[allow(dead_code)] // Phase 8 pool/RPC metrics integration
impl ChainState {
    /// Create a new chain state starting at genesis.
    pub fn new() -> Self {
        Self {
            height: Arc::new(AtomicU64::new(0)),
            tip_hash: Arc::new(Mutex::new([0u8; 32])),
            db: None,
        }
    }

    /// Create chain state with database backend.
    pub fn with_db(db: PoolDatabase) -> Result<Self> {
        let state = Self {
            height: Arc::new(AtomicU64::new(0)),
            tip_hash: Arc::new(Mutex::new([0u8; 32])),
            db: Some(db),
        };

        // Load from database
        state.load_from_db()?;
        Ok(state)
    }

    /// Load chain state from database.
    fn load_from_db(&self) -> Result<()> {
        if let Some(ref db) = self.db {
            if let Some(latest) = db
                .get_latest_block()
                .map_err(|e| bitquan_types::Error::Invalid(format!("DB error: {}", e)))?
            {
                self.height.store(latest.height, Ordering::SeqCst);

                // Parse hash from hex
                let hash_bytes = hex::decode(&latest.hash)
                    .map_err(|e| bitquan_types::Error::Invalid(format!("Invalid hash: {}", e)))?;

                if hash_bytes.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&hash_bytes);
                    *self
                        .tip_hash
                        .lock()
                        .map_err(|_| bitquan_types::Error::Invalid("lock poisoned".into()))? = hash;
                }
            }
        }
        Ok(())
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
                sig_algo: SigAlgorithm::Dilithium3,
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
