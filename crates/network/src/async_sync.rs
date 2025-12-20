//! Async wrapper for sync operations with safe error handling and migration safety

use crate::{
    discovery::PeerBook,
    peer::PeerManager,
    sync::{ChainSync, SyncProgress},
};
use bitquan_types::{BlockHeader, NetworkId};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::task::JoinError;

/// Error type for async sync operations
#[derive(Debug, Error)]
pub enum AsyncSyncError {
    /// Underlying sync operation failed
    #[error("Sync operation failed: {0}")]
    Sync(#[from] bitquan_types::Error),

    /// Async task spawn failed
    #[error("Task spawn failed: {0}")]
    TaskSpawn(#[from] JoinError),

    /// Mutex lock acquisition failed
    #[error("Mutex lock failed: {0}")]
    MutexLock(String),

    /// No peers available for sync operation
    #[error("No peers available for sync")]
    NoPeersAvailable,

    /// Operation timed out
    #[error("Operation timed out")]
    Timeout,
}

/// Result type for async sync operations
pub type AsyncSyncResult<T> = std::result::Result<T, AsyncSyncError>;

/// Migration state for tracking async/sync transitions
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationState {
    /// Not started migration
    NotStarted,
    /// Preparing for migration
    Preparing,
    /// Migration in progress
    InProgress,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed(String),
    /// Rollback in progress
    RollingBack,
    /// Rollback completed
    RolledBack,
}

/// Safety gate configuration for migration operations
#[derive(Debug, Clone)]
pub struct MigrationSafetyConfig {
    /// Maximum time to wait for migration completion
    pub timeout: Duration,
    /// Number of retries allowed
    pub max_retries: u32,
    /// Minimum time between state checks
    pub check_interval: Duration,
    /// Whether automatic rollback is enabled
    pub auto_rollback: bool,
    /// Maximum rollback time
    pub rollback_timeout: Duration,
}

impl Default for MigrationSafetyConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300), // 5 minutes
            max_retries: 3,
            check_interval: Duration::from_secs(1),
            auto_rollback: true,
            rollback_timeout: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Migration safety gates for async operations
pub struct MigrationSafetyGates {
    state: Arc<Mutex<MigrationState>>,
    config: MigrationSafetyConfig,
    start_time: Arc<Mutex<Option<Instant>>>,
    retry_count: Arc<Mutex<u32>>,
}

impl MigrationSafetyGates {
    /// Create new migration safety gates
    pub fn new(config: MigrationSafetyConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(MigrationState::NotStarted)),
            config,
            start_time: Arc::new(Mutex::new(None)),
            retry_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Start migration with safety checks
    pub fn start_migration(&self) -> AsyncSyncResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        // Check if already in progress
        if *state != MigrationState::NotStarted {
            return Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                "Migration already in progress".to_string(),
            )));
        }

        // Reset retry count and start time
        if let Ok(mut start_time) = self.start_time.lock() {
            *start_time = Some(Instant::now());
        }
        if let Ok(mut retry_count) = self.retry_count.lock() {
            *retry_count = 0;
        }

        *state = MigrationState::Preparing;
        log::info!("Migration safety gates: Starting migration preparation");
        Ok(())
    }

    /// Transition to in-progress state
    pub fn set_in_progress(&self) -> AsyncSyncResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        match *state {
            MigrationState::Preparing => {
                *state = MigrationState::InProgress;
                log::info!("Migration safety gates: Migration in progress");
                Ok(())
            }
            _ => Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                "Cannot set in-progress from current state".to_string(),
            ))),
        }
    }

    /// Mark migration as completed
    pub fn mark_completed(&self) -> AsyncSyncResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        if *state != MigrationState::InProgress {
            return Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                "Cannot complete migration from current state".to_string(),
            )));
        }

        *state = MigrationState::Completed;
        log::info!("Migration safety gates: Migration completed successfully");
        Ok(())
    }

    /// Mark migration as failed and optionally rollback
    pub fn mark_failed(&self, error: String) -> AsyncSyncResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        if *state == MigrationState::Completed {
            return Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                "Cannot rollback completed migration".to_string(),
            )));
        }

        *state = MigrationState::Failed(error.clone());
        log::error!("Migration safety gates: Migration failed: {}", error);

        if self.config.auto_rollback {
            log::info!("Migration safety gates: Starting automatic rollback");
            *state = MigrationState::RollingBack;
            // In a real implementation, this would spawn a rollback task
        }

        Ok(())
    }

    /// Get current migration state
    pub fn get_state(&self) -> AsyncSyncResult<MigrationState> {
        let state = self
            .state
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;
        Ok(state.clone())
    }

    /// Check if migration has timed out
    pub fn check_timeout(&self) -> AsyncSyncResult<bool> {
        let state = self.get_state()?;
        let start_time = self
            .start_time
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        match (state, *start_time) {
            (MigrationState::InProgress | MigrationState::Preparing, Some(start)) => {
                Ok(start.elapsed() > self.config.timeout)
            }
            _ => Ok(false),
        }
    }

    /// Increment retry count and check if max retries exceeded
    pub fn increment_retry(&self) -> AsyncSyncResult<bool> {
        let mut retry_count = self
            .retry_count
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        *retry_count += 1;
        let exceeded = *retry_count > self.config.max_retries;

        if exceeded {
            log::warn!(
                "Migration safety gates: Max retries ({}) exceeded",
                self.config.max_retries
            );
        }

        Ok(exceeded)
    }

    /// Reset safety gates for new migration
    pub fn reset(&self) -> AsyncSyncResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

        *state = MigrationState::NotStarted;

        if let Ok(mut start_time) = self.start_time.lock() {
            *start_time = None;
        }
        if let Ok(mut retry_count) = self.retry_count.lock() {
            *retry_count = 0;
        }

        log::info!("Migration safety gates: Reset for new migration");
        Ok(())
    }

    /// Check if operation is safe to proceed
    pub fn can_proceed(&self) -> AsyncSyncResult<bool> {
        let state = self.get_state()?;
        let timed_out = self.check_timeout()?;

        match state {
            MigrationState::NotStarted | MigrationState::Completed => Ok(true),
            MigrationState::InProgress | MigrationState::Preparing => Ok(!timed_out),
            MigrationState::Failed(_)
            | MigrationState::RollingBack
            | MigrationState::RolledBack => Ok(false),
        }
    }
}

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
    safety_gates: Arc<MigrationSafetyGates>,
}

impl AsyncSyncManager {
    /// Create a new async sync manager with minimal setup
    pub fn new(local_height: u64) -> Self {
        // Create mock components for testing
        let peer_manager = Arc::new(PeerManager::new(125, bitquan_types::NetworkId::Testnet));
        let peer_book = Arc::new(Mutex::new(PeerBook::new()));
        let safety_config = MigrationSafetyConfig::default();

        Self {
            chain_sync: Arc::new(AsyncChainSync::new(local_height)),
            peer_manager,
            peer_book,
            network_id: bitquan_types::NetworkId::Testnet,
            safety_gates: Arc::new(MigrationSafetyGates::new(safety_config)),
        }
    }

    /// Create a new async sync manager with full components
    pub fn new_with_components(
        local_height: u64,
        peer_manager: Arc<PeerManager>,
        peer_book: Arc<Mutex<PeerBook>>,
        network_id: NetworkId,
    ) -> Self {
        let safety_config = MigrationSafetyConfig::default();

        Self {
            chain_sync: Arc::new(AsyncChainSync::new(local_height)),
            peer_manager,
            peer_book,
            network_id,
            safety_gates: Arc::new(MigrationSafetyGates::new(safety_config)),
        }
    }

    /// Create a new async sync manager with custom safety configuration
    pub fn new_with_safety_config(
        local_height: u64,
        peer_manager: Arc<PeerManager>,
        peer_book: Arc<Mutex<PeerBook>>,
        network_id: NetworkId,
        safety_config: MigrationSafetyConfig,
    ) -> Self {
        Self {
            chain_sync: Arc::new(AsyncChainSync::new(local_height)),
            peer_manager,
            peer_book,
            network_id,
            safety_gates: Arc::new(MigrationSafetyGates::new(safety_config)),
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
        // Collect peer addresses before any await to avoid holding lock across await
        let best_peers = {
            let peer_book = self
                .peer_book
                .lock()
                .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;

            let peers = peer_book.best_peers(5);
            if peers.is_empty() {
                return Err(AsyncSyncError::NoPeersAvailable);
            }
            peers
        };

        // Get current height as fallback
        let current_progress = self.get_sync_progress().await?;
        let mut best_height = current_progress.local_height;

        // Query peers asynchronously with timeout
        let peer_heights = futures::future::join_all(best_peers.iter().map(|_peer_addr| {
            async move {
                // In a real implementation, this would query the peer
                // For now, simulate with a delay and increment
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Ok::<u64, AsyncSyncError>(current_progress.local_height + 10)
            }
        }))
        .await;

        for height in peer_heights.into_iter().flatten() {
            best_height = best_height.max(height);
        }

        self.chain_sync.set_best_height(best_height).await?;
        Ok(best_height)
    }

    /// Start sync if needed (non-blocking) with migration safety checks
    pub async fn start_sync_if_needed(&self) -> std::result::Result<bool, AsyncSyncError> {
        // Check if migration allows proceeding
        if !self.safety_gates.can_proceed()? {
            let state = self.safety_gates.get_state()?;
            return Err(AsyncSyncError::Sync(bitquan_types::Error::Invalid(
                format!("Migration in progress: {:?}", state),
            )));
        }

        if !self.needs_sync().await? {
            return Ok(false);
        }

        // Start migration for safety tracking
        self.safety_gates.start_migration()?;
        self.safety_gates.set_in_progress()?;

        // Try to start sync
        let sync = self.chain_sync.inner();
        let started = sync.start_sync();

        if !started {
            self.safety_gates
                .mark_failed("Sync already in progress".to_string())?;
            return Ok(false); // Already syncing
        }

        // Update status to discovering
        sync.set_status(crate::sync::SyncStatus::Discovering);

        // Discover best height
        let _best_height = self.discover_best_height().await?;

        if sync.needs_sync() {
            sync.set_status(crate::sync::SyncStatus::DownloadingHeaders);

            // Spawn sync headers task in background
            let sync_manager = self.clone();
            tokio::spawn(async move {
                if let Err(e) = sync_manager.sync_headers_background().await {
                    log::error!("Background sync failed: {}", e);
                    // Mark migration as failed
                    let _ = sync_manager.safety_gates.mark_failed(e.to_string());
                }
            });
        } else {
            // Mark migration as completed if no sync needed
            self.safety_gates.mark_completed()?;
        }

        Ok(true)
    }

    /// Get current migration state
    pub fn get_migration_state(&self) -> AsyncSyncResult<MigrationState> {
        self.safety_gates.get_state()
    }

    /// Reset migration safety gates
    pub fn reset_migration(&self) -> AsyncSyncResult<()> {
        self.safety_gates.reset()
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
                    .map_err(|e| AsyncSyncError::MutexLock(e.to_string()))?;
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
            // Mark migration as completed successfully
            let _ = self.safety_gates.mark_completed();
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
            safety_gates: Arc::clone(&self.safety_gates),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::PeerBook;

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
        let peer_manager = Arc::new(PeerManager::new(10, NetworkId::Regtest));
        let peer_book = Arc::new(Mutex::new(PeerBook::new()));
        let network_id = NetworkId::Regtest;

        let sync_manager =
            AsyncSyncManager::new_with_components(100, peer_manager, peer_book, network_id);

        let progress = sync_manager.get_sync_progress().await.unwrap();
        assert_eq!(progress.local_height, 100);

        let needs_sync = sync_manager.needs_sync().await.unwrap();
        assert!(!needs_sync);
    }
}
