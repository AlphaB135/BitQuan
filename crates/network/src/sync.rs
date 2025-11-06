//! Chain synchronization with peers.

use bitquan_types::{BlockHeader, Result};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[allow(unused_variables)]
/// Chain sync status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Not syncing, up to date.
    Idle,
    /// Discovering best peer height.
    Discovering,
    /// Downloading headers.
    DownloadingHeaders,
    /// Downloading blocks.
    DownloadingBlocks,
    /// Synced and caught up.
    Synced,
}

/// Chain synchronization state.
pub struct ChainSync {
    /// Current sync status.
    status: Arc<AtomicU64>,
    /// Local chain height.
    local_height: Arc<AtomicU64>,
    /// Best known height from peers.
    best_height: Arc<AtomicU64>,
    /// Sync in progress flag.
    syncing: Arc<AtomicBool>,
}

impl ChainSync {
    /// Create a new chain sync manager.
    pub fn new(local_height: u64) -> Self {
        Self {
            status: Arc::new(AtomicU64::new(SyncStatus::Idle as u64)),
            local_height: Arc::new(AtomicU64::new(local_height)),
            best_height: Arc::new(AtomicU64::new(local_height)),
            syncing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get current sync status.
    pub fn status(&self) -> SyncStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => SyncStatus::Idle,
            1 => SyncStatus::Discovering,
            2 => SyncStatus::DownloadingHeaders,
            3 => SyncStatus::DownloadingBlocks,
            4 => SyncStatus::Synced,
            _ => SyncStatus::Idle,
        }
    }

    /// Set sync status.
    pub fn set_status(&self, status: SyncStatus) {
        self.status.store(status as u64, Ordering::Relaxed);
    }

    /// Get local chain height.
    pub fn local_height(&self) -> u64 {
        self.local_height.load(Ordering::Relaxed)
    }

    /// Update local chain height.
    pub fn set_local_height(&self, height: u64) {
        self.local_height.store(height, Ordering::Relaxed);

        // If we caught up, mark as synced
        if height >= self.best_height() {
            self.set_status(SyncStatus::Synced);
            self.syncing.store(false, Ordering::Relaxed);
        }
    }

    /// Get best known height from peers.
    pub fn best_height(&self) -> u64 {
        self.best_height.load(Ordering::Relaxed)
    }

    /// Update best known height.
    pub fn set_best_height(&self, height: u64) {
        let current = self.best_height.load(Ordering::Relaxed);
        if height > current {
            self.best_height.store(height, Ordering::Relaxed);
        }
    }

    /// Check if we need to sync.
    pub fn needs_sync(&self) -> bool {
        self.local_height() < self.best_height()
    }

    /// Check if sync is in progress.
    pub fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::Relaxed)
    }

    /// Start sync process.
    pub fn start_sync(&self) -> bool {
        // Try to set syncing flag
        if self
            .syncing
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            self.set_status(SyncStatus::Discovering);
            true
        } else {
            false // Already syncing
        }
    }

    /// Complete sync process.
    pub fn complete_sync(&self) {
        self.syncing.store(false, Ordering::Relaxed);
        self.set_status(SyncStatus::Synced);
    }

    /// Calculate blocks behind.
    pub fn blocks_behind(&self) -> u64 {
        self.best_height().saturating_sub(self.local_height())
    }

    /// Calculate sync progress percentage.
    pub fn progress(&self) -> f64 {
        let best = self.best_height();
        if best == 0 {
            return 100.0;
        }

        let local = self.local_height();
        (local as f64 / best as f64) * 100.0
    }
}

/// Sync progress info.
#[derive(Debug, Clone)]
pub struct SyncProgress {
    /// Current status.
    pub status: SyncStatus,
    /// Local chain height.
    pub local_height: u64,
    /// Best known height.
    pub best_height: u64,
    /// Blocks behind.
    pub blocks_behind: u64,
    /// Progress percentage.
    pub progress: f64,
}

impl From<&ChainSync> for SyncProgress {
    fn from(sync: &ChainSync) -> Self {
        Self {
            status: sync.status(),
            local_height: sync.local_height(),
            best_height: sync.best_height(),
            blocks_behind: sync.blocks_behind(),
            progress: sync.progress(),
        }
    }
}

/// Request missing blocks from a peer.
///
/// This is a placeholder for actual block request logic.
#[allow(unused_variables)]
pub fn request_blocks(
    start_height: u64,
    end_height: u64,
    peer_id: &str,
) -> Result<Vec<BlockHeader>> {
    // TODO: Implement actual block request via network protocol
    // For now, return empty list
    Ok(vec![])
}

/// Process received headers and update sync state.
pub fn process_headers(headers: Vec<BlockHeader>, sync: &ChainSync) -> Result<()> {
    if headers.is_empty() {
        return Ok(());
    }

    // Update local height based on received headers
    let last_height = sync.local_height() + headers.len() as u64;
    sync.set_local_height(last_height);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_sync_initialization() {
        let sync = ChainSync::new(100);

        assert_eq!(sync.local_height(), 100);
        assert_eq!(sync.best_height(), 100);
        assert_eq!(sync.status(), SyncStatus::Idle);
        assert!(!sync.is_syncing());
    }

    #[test]
    fn test_sync_needs_sync() {
        let sync = ChainSync::new(100);

        // Initially no sync needed
        assert!(!sync.needs_sync());

        // Update best height
        sync.set_best_height(150);

        // Now sync is needed
        assert!(sync.needs_sync());
        assert_eq!(sync.blocks_behind(), 50);
    }

    #[test]
    fn test_sync_start_and_complete() {
        let sync = ChainSync::new(100);
        sync.set_best_height(150);

        // Start sync
        assert!(sync.start_sync());
        assert!(sync.is_syncing());
        assert_eq!(sync.status(), SyncStatus::Discovering);

        // Cannot start again while syncing
        assert!(!sync.start_sync());

        // Complete sync
        sync.complete_sync();
        assert!(!sync.is_syncing());
        assert_eq!(sync.status(), SyncStatus::Synced);
    }

    #[test]
    fn test_sync_progress() {
        let sync = ChainSync::new(50);
        sync.set_best_height(100);

        // 50% progress
        assert_eq!(sync.progress(), 50.0);

        // Update local height
        sync.set_local_height(75);

        // 75% progress
        assert_eq!(sync.progress(), 75.0);
    }

    #[test]
    fn test_auto_sync_completion() {
        let sync = ChainSync::new(90);
        sync.set_best_height(100);

        sync.start_sync();
        assert!(sync.is_syncing());

        // Catch up to best height
        sync.set_local_height(100);

        // Should auto-complete
        assert!(!sync.is_syncing());
        assert_eq!(sync.status(), SyncStatus::Synced);
    }

    #[test]
    fn test_sync_progress_struct() {
        let sync = ChainSync::new(75);
        sync.set_best_height(100);

        let progress = SyncProgress::from(&sync);

        assert_eq!(progress.local_height, 75);
        assert_eq!(progress.best_height, 100);
        assert_eq!(progress.blocks_behind, 25);
        assert_eq!(progress.progress, 75.0);
    }
}
