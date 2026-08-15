//! Async wrapper for storage operations with safe error handling

use crate::{ChainStore, StorageError};
use async_trait::async_trait;
use bitquan_types::{Block, BlockHeader, Transaction};
use std::sync::{Arc, Mutex};
use tokio::task::JoinError;

/// Error type for async storage operations
#[derive(Debug, thiserror::Error)]
pub enum AsyncStoreError {
    /// Underlying storage operation failed
    #[error("Storage operation failed: {0}")]
    Storage(#[from] StorageError),

    /// Task spawning failed in async runtime
    #[error("Task spawn failed: {0}")]
    TaskSpawn(#[from] JoinError),

    /// Mutex was poisoned due to panic
    #[error("Mutex poisoned: {0}")]
    Poisoned(&'static str),

    /// Operation was cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// No valid headers found for the given locators
    /// This indicates the peer's chain doesn't connect to ours
    #[error("No valid headers found - peer chain incompatible")]
    NoValidHeaders,
}

/// Result type for async storage operations
pub type AsyncResult<T> = std::result::Result<T, AsyncStoreError>;

/// Async wrapper around a ChainStore implementation
/// This safely runs synchronous storage operations in a blocking thread pool
pub struct AsyncStoreWrapper<T: ChainStore + Send + Sync + 'static> {
    inner: Arc<Mutex<T>>,
}

impl<T: ChainStore + Send + Sync + 'static> AsyncStoreWrapper<T> {
    /// Create a new async wrapper around a synchronous store
    pub fn new(store: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(store)),
        }
    }

    /// Get the current height of the chain
    async fn calculate_height(&self) -> std::result::Result<u64, AsyncStoreError> {
        let store = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("height calculation"))?;

            // Count blocks by height index
            let mut height = 0u64;
            loop {
                match guard.get_block_by_height(height) {
                    Ok(Some(_)) => height += 1,
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            Ok(height)
        })
        .await?
    }
}

/// Async interface for blockchain storage operations
///
/// This trait provides async versions of all ChainStore operations,
/// safely running synchronous storage operations in a blocking thread pool.
#[async_trait]
pub trait AsyncChainStore: Send + Sync {
    /// Get the current height of the chain
    async fn height(&self) -> std::result::Result<u64, AsyncStoreError>;

    /// Get the current tip of the chain
    async fn tip(&self) -> std::result::Result<Option<BlockHeader>, AsyncStoreError>;

    /// Get a block by its hash
    async fn get_block(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<Block>, AsyncStoreError>;

    /// Get a block by its height
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> std::result::Result<Option<Block>, AsyncStoreError>;

    /// Get the height of a block given its hash.
    ///
    /// Returns `Ok(Some(height))` when the hash is part of this chain.
    /// Returns `Ok(None)` when the hash is unknown.
    ///
    /// # Complexity
    /// Implementations MUST be O(1) — do NOT perform a full-chain scan.
    /// This is called once per locator in GetHeaders processing; an O(height)
    /// implementation here is a single-message DoS vector.
    async fn get_height_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<u64>, AsyncStoreError>;

    /// Get a transaction by its ID
    async fn get_transaction(
        &self,
        txid: &[u8; 32],
    ) -> std::result::Result<Option<Transaction>, AsyncStoreError>;

    /// Insert a block into the store
    async fn insert_block(&self, block: Block) -> std::result::Result<(), AsyncStoreError>;

    /// Check if a block exists
    async fn has_block(&self, hash: &[u8; 32]) -> std::result::Result<bool, AsyncStoreError>;

    /// Get block header by hash
    async fn get_header(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<BlockHeader>, AsyncStoreError>;

    /// Get a UTXO entry by outpoint
    async fn get_utxo(
        &self,
        outpoint: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, AsyncStoreError>;

    /// Disconnect a block, rolling back its changes (for chain reorg).
    async fn disconnect_block(&self, block: &Block) -> std::result::Result<(), AsyncStoreError>;

    /// Calculate the Median Time Past (MTP) from the last 11 blocks
    /// Returns median timestamp for timestamp validation
    async fn median_time_past(&self) -> std::result::Result<u64, AsyncStoreError>;

    /// Get pruning metadata if available.
    ///
    /// Returns None for stores that don't support pruning or if metadata is not available.
    async fn get_pruning_metadata(
        &self,
    ) -> std::result::Result<Option<crate::PruningMetadata>, AsyncStoreError>;
}

#[async_trait]
impl<T: ChainStore + Send + Sync + 'static> AsyncChainStore for AsyncStoreWrapper<T> {
    async fn height(&self) -> std::result::Result<u64, AsyncStoreError> {
        self.calculate_height().await
    }

    async fn tip(&self) -> std::result::Result<Option<BlockHeader>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("tip operation"))?;
            guard.tip()
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn get_block(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<Block>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let hash = *hash;

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("get_block operation"))?;
            guard.get_block(&hash)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn get_height_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<u64>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let hash = *hash;

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("get_height_by_hash operation"))?;
            guard.get_height_by_hash(&hash)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> std::result::Result<Option<Block>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("get_block_by_height operation"))?;
            guard.get_block_by_height(height)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn get_transaction(
        &self,
        txid: &[u8; 32],
    ) -> std::result::Result<Option<Transaction>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let txid = *txid;

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("get_transaction operation"))?;
            guard.get_transaction(&txid)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn insert_block(&self, block: Block) -> std::result::Result<(), AsyncStoreError> {
        let store = Arc::clone(&self.inner);

        Ok(tokio::task::spawn_blocking(move || {
            let mut guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("insert_block operation"))?;
            guard.insert_block(block)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn has_block(&self, hash: &[u8; 32]) -> std::result::Result<bool, AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let hash = *hash;

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("has_block operation"))?;
            // Check if block exists by trying to get it
            match guard.get_block(&hash) {
                Ok(Some(_)) => Ok(true),
                Ok(None) => Ok(false),
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn get_header(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<BlockHeader>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let hash = *hash;

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("get_header operation"))?;
            // Get the block and return just the header
            match guard.get_block(&hash) {
                Ok(Some(block)) => Ok(Some(block.header)),
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn get_utxo(
        &self,
        outpoint: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let outpoint = outpoint.to_vec();

        Ok(tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("get_utxo operation"))?;
            guard.get_utxo(&outpoint)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)??)
    }

    async fn disconnect_block(&self, block: &Block) -> std::result::Result<(), AsyncStoreError> {
        let store = Arc::clone(&self.inner);
        let block = block.clone();

        // Spawn blocking task because RocksDB disconnect_block is synchronous
        tokio::task::spawn_blocking(move || {
            let mut guard = store
                .lock()
                .map_err(|_e| AsyncStoreError::Poisoned("disconnect_block operation"))?;
            guard.disconnect_block(&block)
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)?
        .map_err(AsyncStoreError::Storage)?;

        Ok(())
    }

    async fn median_time_past(&self) -> std::result::Result<u64, AsyncStoreError> {
        let store = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("median_time_past calculation"))?;

            // Count blocks to get current height (same pattern as calculate_height)
            let mut current_height = 0u64;
            loop {
                match guard.get_block_by_height(current_height) {
                    Ok(Some(_)) => current_height += 1,
                    Ok(None) => break,
                    Err(_) => break,
                }
            }

            // If less than 11 blocks, return 0 as fallback
            if current_height < 11 {
                return Ok(0);
            }

            // Collect last 11 block timestamps
            let mut timestamps = Vec::with_capacity(11);
            for i in 0u64..11 {
                let height = current_height.saturating_sub(i);
                if let Ok(Some(block)) = guard.get_block_by_height(height) {
                    timestamps.push(u64::from(block.header.time));
                }
            }

            // Sort and return median (middle element)
            timestamps.sort_unstable();
            Ok(timestamps[5]) // 11 elements, index 5 is median
        })
        .await
        .map_err(AsyncStoreError::TaskSpawn)?
    }

    async fn get_pruning_metadata(
        &self,
    ) -> std::result::Result<Option<crate::PruningMetadata>, AsyncStoreError> {
        // For generic ChainStore, we can't get pruning metadata
        // This is only available for RocksDBStore
        // Return None for non-RocksDB stores
        Ok(
            tokio::task::spawn_blocking(|| {
                Ok::<Option<crate::PruningMetadata>, StorageError>(None)
            })
            .await
            .map_err(AsyncStoreError::TaskSpawn)??,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rocksdb_store::RocksDBStore;
    use tempfile::TempDir;

    async fn test_async_store<T: AsyncChainStore>(store: &T) -> AsyncResult<()> {
        // Test height
        let height = store.height().await?;
        assert_eq!(height, 0);

        // Test tip
        let tip = store.tip().await?;
        assert!(tip.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_async_store_wrapper() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().join("test.db");
        let rocks_store = RocksDBStore::open(path).expect("Failed to open RocksDB store");
        let async_store = AsyncStoreWrapper::new(rocks_store);

        test_async_store(&async_store).await.unwrap();
    }
}
