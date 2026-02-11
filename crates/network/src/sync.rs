//! Chain synchronization with peers.

use crate::{discovery::PeerBook, protocol::Message};
use bitquan_types::{BlockHeader, Result};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
    /// Last sync attempt timestamp.
    last_sync_attempt: Arc<AtomicU64>,
    /// Sync errors count.
    sync_errors: Arc<AtomicU64>,
}

impl ChainSync {
    /// Create a new chain sync manager.
    pub fn new(local_height: u64) -> Self {
        Self {
            status: Arc::new(AtomicU64::new(SyncStatus::Idle as u64)),
            local_height: Arc::new(AtomicU64::new(local_height)),
            best_height: Arc::new(AtomicU64::new(local_height)),
            syncing: Arc::new(AtomicBool::new(false)),
            last_sync_attempt: Arc::new(AtomicU64::new(0)),
            sync_errors: Arc::new(AtomicU64::new(0)),
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
            self.update_last_sync_attempt();
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

    /// Get last sync attempt timestamp.
    pub fn last_sync_attempt(&self) -> u64 {
        self.last_sync_attempt.load(Ordering::Relaxed)
    }

    /// Update last sync attempt timestamp.
    pub fn update_last_sync_attempt(&self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_sync_attempt.store(now, Ordering::Relaxed);
    }

    /// Get sync errors count.
    pub fn sync_errors(&self) -> u64 {
        self.sync_errors.load(Ordering::Relaxed)
    }

    /// Increment sync errors count.
    pub fn increment_sync_errors(&self) {
        self.sync_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset sync errors count.
    pub fn reset_sync_errors(&self) {
        self.sync_errors.store(0, Ordering::Relaxed);
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
    /// Last sync attempt timestamp.
    pub last_sync_attempt: u64,
    /// Sync errors count.
    pub sync_errors: u64,
}

impl From<&ChainSync> for SyncProgress {
    fn from(sync: &ChainSync) -> Self {
        Self {
            status: sync.status(),
            local_height: sync.local_height(),
            best_height: sync.best_height(),
            blocks_behind: sync.blocks_behind(),
            progress: sync.progress(),
            last_sync_attempt: sync.last_sync_attempt(),
            sync_errors: sync.sync_errors(),
        }
    }
}

/// Enhanced sync manager that integrates with peer management.
pub struct SyncManager {
    /// Chain sync instance
    chain_sync: Arc<ChainSync>,
    /// Peer book for peer discovery
    peer_book: Arc<Mutex<PeerBook>>,
}

impl SyncManager {
    /// Create a new sync manager.
    pub fn new(
        local_height: u64,
        peer_book: Arc<std::sync::Mutex<PeerBook>>,
    ) -> Self {
        Self {
            chain_sync: Arc::new(ChainSync::new(local_height)),
            peer_book,
        }
    }

    /// Get the chain sync state.
    pub fn chain_sync(&self) -> &Arc<ChainSync> {
        &self.chain_sync
    }

    /// Discover best peers and update best height.
    pub fn discover_best_height(&self) -> Result<u64> {
        let peer_book = self
            .peer_book
            .lock()
            .map_err(|_| bitquan_types::Error::Fatal("peer book lock poisoned"))?;

        let best_peers = peer_book.best_peers(5);
        let mut best_height = self.chain_sync.local_height();

        for peer_addr in best_peers {
            // Verify peer's claimed height matches what they can actually provide
            if let Some(peer) = peer_book.get_peer(&peer_addr) {
                // Use peer's claimed height from version handshake
                // In a full implementation, we would also verify by requesting
                // a block at that height to confirm the peer has it
                if let Some(peer_height) = peer.claimed_height {
                    // Verify that height is reasonable (not ridiculously high)
                    // This prevents Sybil attacks where peers claim extreme heights
                    let local_height = self.chain_sync.local_height();

                    // Sanity check: Don't accept heights more than 1000 blocks ahead
                    // without verification. In production, this would request
                    // a block at peer_height to confirm.
                    if peer_height > local_height && peer_height <= local_height + 1000 {
                        log::info!(
                            "✓ Peer {} claims height {} (local: {})",
                            peer_addr, peer_height, local_height
                        );
                        if peer_height > best_height {
                            best_height = peer_height;
                        }
                    } else if peer_height > local_height + 1000 {
                        log::warn!(
                            "⚠ Peer {} claims unreasonable height {} (local: {}), ignoring",
                            peer_addr, peer_height, local_height
                        );
                    }
                } else {
                    log::debug!("Peer {} has no claimed_height, skipping", peer_addr);
                }
            }
        }

        self.chain_sync.set_best_height(best_height);
        Ok(best_height)
    }

    /// Start the sync process if needed.
    pub fn start_sync_if_needed(&self) -> Result<bool> {
        if !self.chain_sync.needs_sync() {
            return Ok(false);
        }

        if !self.chain_sync.start_sync() {
            return Ok(false); // Already syncing
        }

        // Discover best height
        self.discover_best_height()?;

        if self.chain_sync.needs_sync() {
            self.chain_sync
                .set_status(crate::sync::SyncStatus::DownloadingHeaders);
            self.sync_headers()?;
        }

        Ok(true)
    }

    /// Sync headers from peers.
    fn sync_headers(&self) -> Result<()> {
        let local_height = self.chain_sync.local_height();
        let best_height = self.chain_sync.best_height();

        let mut current_height = local_height;

        while current_height < best_height {
            let batch_size = std::cmp::min(2000, (best_height - current_height) as usize);
            let end_height = current_height + batch_size as u64 - 1;

            // Get best peer for this batch
            let mut peer_book = self
                .peer_book
                .lock()
                .map_err(|_| bitquan_types::Error::Fatal("peer book lock poisoned"))?;

            let best_peers = peer_book.best_peers(1);
            if best_peers.is_empty() {
                self.chain_sync.increment_sync_errors();
                break;
            }

            let peer_id = best_peers[0].clone();

            // Validate peer's claimed height before requesting blocks
            // This prevents requesting blocks that peer doesn't actually have
            let should_skip_peer = {
                let peer_claimed_height = peer_book.get_peer(&peer_id)
                    .and_then(|p| p.claimed_height);

                if let Some(claimed) = peer_claimed_height {
                    if claimed < end_height {
                        log::warn!(
                            "Peer {} claims height {} but we're requesting up to {}, skipping",
                            peer_id, claimed, end_height
                        );
                        peer_book.mark_peer_failure(&peer_id);
                        true
                    } else {
                        false
                    }
                } else {
                    log::debug!("Peer {} has no claimed_height, trying anyway", peer_id);
                    false
                }
            };

            // Release peer_book lock before network call
            drop(peer_book);

            // Skip to next peer if current one is unsuitable
            if should_skip_peer {
                continue;
            }

            match request_blocks_from_peer(current_height, end_height, &peer_id) {
                Ok(headers) => {
                    if headers.is_empty() {
                        // SECURITY FIX: Don't break - try next peer instead
                        // A malicious peer returning empty headers should not halt sync
                        log::warn!(
                            "Peer {} returned no headers for range {}-{}, trying next peer",
                            peer_id,
                            current_height,
                            end_height
                        );

                        // Mark peer as unreliable and continue with next peer
                        if let Ok(mut peer_book) = self.peer_book.lock() {
                            peer_book.mark_peer_failure(&peer_id);
                        }

                        // Continue to try other peers instead of breaking
                        continue;
                    }

                    process_headers(headers, &self.chain_sync)?;
                    current_height = self.chain_sync.local_height();
                }
                Err(e) => {
                    log::error!("Failed to request blocks from {}: error={}", peer_id, e);
                    self.chain_sync.increment_sync_errors();

                    // Mark peer failure
                    if let Ok(mut peer_book) = self.peer_book.lock() {
                        peer_book.mark_peer_failure(&peer_id);
                    }

                    // Try next peer
                    break;
                }
            }
        }

        if self.chain_sync.local_height() >= self.chain_sync.best_height() {
            self.chain_sync.complete_sync();
        } else {
            self.chain_sync
                .set_status(crate::sync::SyncStatus::DownloadingBlocks);
        }

        Ok(())
    }
}

/// Request missing blocks from a specific peer.
///
/// Sends a getheaders message to request block headers in specified range.
pub fn request_blocks_from_peer(
    start_height: u64,
    end_height: u64,
    peer_id: &str,
) -> Result<Vec<BlockHeader>> {
    use crate::protocol::PROTOCOL_VERSION;

    // Validate input parameters
    if start_height > end_height {
        return Err(bitquan_types::Error::Invalid(
            "start_height cannot be greater than end_height".to_string(),
        ));
    }

    if end_height - start_height > 2000 {
        return Err(bitquan_types::Error::Invalid(
            "cannot request more than 2000 blocks at once".to_string(),
        ));
    }

    // Create block locator hashes (simplified - in real implementation would use chain state)
    // For now, we'll use a simple approach - in production this would use actual chain tips
    let locator_hashes = vec![[0u8; 32]]; // Genesis hash placeholder

    // Create getheaders message
    let _getheaders_msg = Message::GetHeaders {
        version: PROTOCOL_VERSION,
        locator_hashes,
        stop_hash: [0u8; 32], // Stop at tip (placeholder)
    };

    // In a real implementation, this would:
    // 1. Connect to peer if not already connected
    // 2. Send getheaders message
    // 3. Wait for headers response with timeout
    // 4. Parse and return headers

    // TODO: Implement actual network communication
    // For now, this is a stub that returns empty headers
    log::debug!(
        "Requesting blocks {} to {} from peer: {}",
        start_height,
        end_height,
        peer_id
    );

    // SECURITY FIX: Return error instead of empty headers
    // This prevents infinite retry loops in the caller when peers
    // cannot provide the requested blocks
    //
    // In a full implementation, this function would:
    // - Connect to peer if not already connected
    // - Send GetHeaders message
    // - Wait for and parse Headers response
    // - Validate and return the headers
    Err(bitquan_types::Error::Net(
        "request_blocks_from_peer not implemented - peer unavailable".to_string()
    ))
}

/// Legacy function for backward compatibility.
pub fn request_blocks(
    start_height: u64,
    end_height: u64,
    peer_id: &str,
) -> Result<Vec<BlockHeader>> {
    request_blocks_from_peer(start_height, end_height, peer_id)
}

/// Process received headers and update sync state.
pub fn process_headers(headers: Vec<BlockHeader>, sync: &ChainSync) -> Result<()> {
    if headers.is_empty() {
        return Ok(());
    }

    // Validate headers
    for header in &headers {
        // Basic validation - in production would include full consensus validation
        if header.time == 0 {
            return Err(bitquan_types::Error::Invalid(
                "header has invalid timestamp".to_string(),
            ));
        }
    }

    // Update local height based on received headers
    let last_height = sync.local_height() + headers.len() as u64;
    sync.set_local_height(last_height);

    println!(
        "Processed {} headers, new height: {}",
        headers.len(),
        last_height
    );

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
        assert_eq!(progress.sync_errors, 0);
    }

    #[test]
    fn test_request_blocks_validation() {
        // Test invalid range
        let result = request_blocks(100, 50, "test_peer");
        assert!(result.is_err());

        // Test too large range
        let result = request_blocks(0, 3000, "test_peer");
        assert!(result.is_err());

        // Test valid range
        let result = request_blocks(0, 100, "test_peer");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_error_tracking() {
        let sync = ChainSync::new(100);

        assert_eq!(sync.sync_errors(), 0);

        sync.increment_sync_errors();
        assert_eq!(sync.sync_errors(), 1);

        sync.increment_sync_errors();
        sync.increment_sync_errors();
        assert_eq!(sync.sync_errors(), 3);

        sync.reset_sync_errors();
        assert_eq!(sync.sync_errors(), 0);
    }

    #[test]
    fn test_last_sync_attempt() {
        let sync = ChainSync::new(100);

        let initial = sync.last_sync_attempt();
        sync.start_sync();
        let after = sync.last_sync_attempt();

        // Should have updated timestamp
        assert!(after > initial);
    }
}
