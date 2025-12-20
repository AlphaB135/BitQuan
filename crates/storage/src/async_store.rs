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
    async fn calculate_height(&self) -> AsyncResult<u64> {
        let store = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            let guard = store
                .lock()
                .map_err(|_| AsyncStoreError::Poisoned("height calculation"))?;

            // For InMemoryChainStore, try to use height method if available
            // For other stores, approximate by counting blocks
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

    /// Get a transaction by its ID
    async fn get_transaction(
        &self,
        txid: &[u8; 32],
    ) -> std::result::Result<Option<Transaction>, AsyncStoreError>;

    /// Insert a block into the store
    async fn insert_block(&mut self, block: Block) -> std::result::Result<(), AsyncStoreError>;

    /// Check if a block exists
    async fn has_block(&self, hash: &[u8; 32]) -> std::result::Result<bool, AsyncStoreError>;

    /// Get block header by hash
    async fn get_header(
        &self,
        hash: &[u8; 32],
    ) -> std::result::Result<Option<BlockHeader>, AsyncStoreError>;
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

    async fn insert_block(&mut self, block: Block) -> std::result::Result<(), AsyncStoreError> {
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
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.db");
        let rocks_store = RocksDBStore::open(path).unwrap();
        let async_store = AsyncStoreWrapper::new(rocks_store);

        test_async_store(&async_store).await.unwrap();
    }
}
