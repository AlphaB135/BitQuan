//! Async wrapper for sync operations with safe error handling

use crate::{
    discovery::PeerBook,
    peer::PeerManager,
    sync::{ChainSync, SyncProgress},
};
use bitquan_types::{BlockHeader, NetworkId};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::task::JoinError;

/// Error type for async sync operations
#[derive(Debug, Error)]
pub enum AsyncSyncError {
    #[error("Sync operation failed: {0}")]
    Sync(#[from] bitquan_types::Error),

    #[error("Task spawn failed: {0}")]
    TaskSpawn(#[from] JoinError),

    #[error("Peer manager lock poisoned")]
    PeerManagerPoisoned,

    #[error("Peer book lock poisoned")]
    PeerBookPoisoned,

    #[error("No peers available for sync")]
    NoPeersAvailable,

    #[error("Operation timed out")]
    Timeout,
}

/// Result type for async sync operations
pub type AsyncSyncResult<T> = std::result::Result<T, AsyncSyncError>;

/// Async wrapper for ChainSync operations
pub struct AsyncChainSync {
    inner: Arc<ChainSync>,
}

impl AsyncChainSync {
    /// Create a new async chain sync wrapper
    pub fn new(local_height: u64) -> Self {
        Self {
            inner: Arc::new(ChainSync::new(local_height)),
        }
    }

    /// Get current sync progress safely
    pub async fn get_progress(&self) -> std::result::Result<SyncProgress, AsyncSyncError> {
        let sync = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || Ok(SyncProgress::from(sync.as_ref()))).await?
    }

    /// Set local height safely
    pub async fn set_local_height(&self, height: u64) -> std::result::Result<(), AsyncSyncError> {
        let sync = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            sync.set_local_height(height);
            Ok(())
        })
        .await?
    }

    /// Update best height safely
    pub async fn set_best_height(&self, height: u64) -> std::result::Result<(), AsyncSyncError> {
        let sync = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            sync.set_best_height(height);
            Ok(())
        })
        .await?
    }

    /// Get the inner ChainSync reference for read-only operations
    pub fn inner(&self) -> &Arc<ChainSync> {
        &self.inner
    }
}

/// Async wrapper for SyncManager operations
pub struct AsyncSyncManager {
    chain_sync: Arc<AsyncChainSync>,
    peer_manager: Arc<PeerManager>,
    peer_book: Arc<Mutex<PeerBook>>,
    network_id: NetworkId,
}

impl AsyncSyncManager {
    /// Create a new async sync manager
    pub fn new(
        local_height: u64,
        peer_manager: Arc<PeerManager>,
        peer_book: Arc<Mutex<PeerBook>>,
        network_id: NetworkId,
    ) -> Self {
        Self {
            chain_sync: Arc::new(AsyncChainSync::new(local_height)),
            peer_manager,
            peer_book,
            network_id,
        }
    }

    /// Get current sync progress
    pub async fn get_sync_progress(&self) -> AsyncSyncResult<SyncProgress> {
        self.chain_sync.get_progress().await
    }

    /// Check if sync is needed
    pub async fn needs_sync(&self) -> AsyncSyncResult<bool> {
        let progress = self.get_sync_progress().await?;
        Ok(progress.local_height < progress.best_height)
    }

    /// Discover best height from peers asynchronously
    pub async fn discover_best_height(&self) -> std::result::Result<u64, AsyncSyncError> {
        let peer_book = self
            .peer_book
            .lock()
            .map_err(|_| AsyncSyncError::PeerBookPoisoned)?;

        let best_peers = peer_book.best_peers(5);

        if best_peers.is_empty() {
            return Err(AsyncSyncError::NoPeersAvailable);
        }

        // Get current height as fallback
        let current_progress = self.get_sync_progress().await?;
        let mut best_height = current_progress.local_height;

        // Query peers asynchronously with timeout
        let peer_heights = futures::future::join_all(best_peers.iter().map(|peer_addr| {
            let peer_addr = peer_addr.clone();
            async move {
                // In a real implementation, this would query the peer
                // For now, simulate with a delay and increment
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Ok::<u64, AsyncSyncError>(current_progress.local_height + 10)
            }
        }))
        .await;

        for height_result in peer_heights {
            if let Ok(height) = height_result {
                best_height = best_height.max(height);
            }
        }

        self.chain_sync.set_best_height(best_height).await?;
        Ok(best_height)
    }

    /// Start sync if needed (non-blocking)
    pub async fn start_sync_if_needed(&self) -> std::result::Result<bool, AsyncSyncError> {
        if !self.needs_sync().await? {
            return Ok(false);
        }

        // Try to start sync
        let sync = self.chain_sync.inner();
        let started = sync.start_sync();

        if !started {
            return Ok(false); // Already syncing
        }

        // Update status to discovering
        sync.set_status(crate::sync::SyncStatus::Discovering);

        // Discover best height
        let best_height = self.discover_best_height().await?;

        if sync.needs_sync() {
            sync.set_status(crate::sync::SyncStatus::DownloadingHeaders);

            // Spawn sync headers task in background
            let sync_manager = self.clone();
            tokio::spawn(async move {
                if let Err(e) = sync_manager.sync_headers_background().await {
                    log::error!("Background sync failed: {}", e);
                }
            });
        }

        Ok(true)
    }

    /// Background sync headers task
    async fn sync_headers_background(&self) -> std::result::Result<(), AsyncSyncError> {
        let sync = self.chain_sync.inner();
        let local_height = sync.local_height();
        let best_height = sync.best_height();

        let mut current_height = local_height;
        let max_batch_size = 2000u64;

        while current_height < best_height {
            let batch_size = max_batch_size.min(best_height - current_height);
            let end_height = current_height + batch_size - 1;

            // Get peers for this batch
            let best_peers = {
                let peer_book = self
                    .peer_book
                    .lock()
                    .map_err(|_| AsyncSyncError::PeerBookPoisoned)?;
                peer_book.best_peers(3)
            };

            if best_peers.is_empty() {
                sync.increment_sync_errors();
                break;
            }

            // Try each peer until one succeeds
            let mut headers_received = false;
            for peer_id in best_peers {
                match self
                    .request_headers_from_peer(current_height, end_height, &peer_id)
                    .await
                {
                    Ok(headers) => {
                        if !headers.is_empty() {
                            headers_received = true;
                            current_height += headers.len() as u64;
                            sync.set_local_height(current_height);

                            // Reset error count on success
                            if sync.sync_errors() > 0 {
                                sync.reset_sync_errors();
                            }
                            break;
                        } else {
                            log::warn!(
                                "Peer {} returned no headers for range {}-{}",
                                peer_id,
                                current_height,
                                end_height
                            );
                            // Mark peer failure
                            if let Ok(mut peer_book) = self.peer_book.lock() {
                                peer_book.mark_peer_failure(&peer_id);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to request headers from {}: {}", peer_id, e);
                        sync.increment_sync_errors();

                        // Mark peer failure
                        if let Ok(mut peer_book) = self.peer_book.lock() {
                            peer_book.mark_peer_failure(&peer_id);
                        }
                    }
                }
            }

            if !headers_received {
                // All peers failed for this batch
                break;
            }
        }

        // Update final status
        if sync.local_height() >= sync.best_height() {
            sync.complete_sync();
        } else {
            sync.set_status(crate::sync::SyncStatus::DownloadingBlocks);
        }

        Ok(())
    }

    /// Request headers from a specific peer
    async fn request_headers_from_peer(
        &self,
        start_height: u64,
        end_height: u64,
        peer_id: &str,
    ) -> std::result::Result<Vec<BlockHeader>, AsyncSyncError> {
        // Validate input
        if start_height > end_height {
            return Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                "start_height cannot be greater than end_height".to_string(),
            )));
        }

        if end_height - start_height > 2000 {
            return Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                "cannot request more than 2000 headers at once".to_string(),
            )));
        }

        // In a real implementation, this would:
        // 1. Connect to the peer if not already connected
        // 2. Send a getheaders message
        // 3. Wait for response with timeout
        // 4. Parse and return headers

        // For now, simulate with empty response
        log::debug!(
            "Requesting headers {} to {} from peer {}",
            start_height,
            end_height,
            peer_id
        );

        // Simulate network latency
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Ok(vec![])
    }

    /// Get sync status as string
    pub async fn sync_status(&self) -> std::result::Result<String, AsyncSyncError> {
        let progress = self.get_sync_progress().await?;
        Ok(format!("{:?}", progress.status))
    }
}

impl Clone for AsyncSyncManager {
    fn clone(&self) -> Self {
        Self {
            chain_sync: Arc::clone(&self.chain_sync),
            peer_manager: Arc::clone(&self.peer_manager),
            peer_book: Arc::clone(&self.peer_book),
            network_id: self.network_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::PeerBook;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_async_chain_sync() {
        let sync = AsyncChainSync::new(100);

        let progress = sync.get_progress().await.unwrap();
        assert_eq!(progress.local_height, 100);
        assert_eq!(progress.best_height, 100);

        sync.set_best_height(150).await.unwrap();

        let progress = sync.get_progress().await.unwrap();
        assert_eq!(progress.best_height, 150);
    }

    #[tokio::test]
    async fn test_async_sync_manager() {
        let peer_manager = Arc::new(PeerManager::new());
        let peer_book = Arc::new(Mutex::new(PeerBook::new()));
        let network_id = NetworkId::Regtest;

        let sync_manager = AsyncSyncManager::new(100, peer_manager, peer_book, network_id);

        let progress = sync_manager.get_sync_progress().await.unwrap();
        assert_eq!(progress.local_height, 100);

        let needs_sync = sync_manager.needs_sync().await.unwrap();
        assert!(!needs_sync);
    }
}
