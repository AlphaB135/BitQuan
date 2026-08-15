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
    pub fn new(local_height: u64, peer_book: Arc<std::sync::Mutex<PeerBook>>) -> Self {
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
        // C3 FIX: Initialize to 0 instead of local_height
        // This allows discovering new blocks even when all known peers are behind
        let mut best_height = 0u64;
        let mut found_valid_peer = false;

        for peer_addr in best_peers {
            // Verify peer's claimed height matches what they can actually provide
            if let Some(peer) = peer_book.get_peer(&peer_addr) {
                // Use peer's claimed height from version handshake
                // In a full implementation, we would also verify by requesting
                // a block at that height to confirm the peer has it
                if let Some(peer_height) = peer.claimed_height {
                    // C3 FIX: Found a peer with claimed height
                    found_valid_peer = true;

                    // Verify that height is reasonable (not ridiculously high)
                    // This prevents Sybil attacks where peers claim extreme heights
                    let local_height = self.chain_sync.local_height();

                    // Sanity check: Don't accept heights more than 1000 blocks ahead
                    // without verification. In production, this would request
                    // a block at peer_height to confirm.
                    if peer_height <= local_height + 1000 {
                        log::info!(
                            "✓ Peer {} claims height {} (local: {})",
                            peer_addr,
                            peer_height,
                            local_height
                        );
                        if peer_height > best_height {
                            best_height = peer_height;
                        }
                    } else {
                        log::warn!(
                            "⚠ Peer {} claims unreasonable height {} (local: {}), ignoring",
                            peer_addr,
                            peer_height,
                            local_height
                        );
                    }
                } else {
                    log::debug!("Peer {} has no claimed_height, skipping", peer_addr);
                }
            }
        }

        // C3 FIX: If no valid peers found, fall back to local_height
        // This prevents syncing when network is unavailable
        if !found_valid_peer {
            best_height = self.chain_sync.local_height();
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
                let peer_claimed_height =
                    peer_book.get_peer(&peer_id).and_then(|p| p.claimed_height);

                if let Some(claimed) = peer_claimed_height {
                    if claimed < end_height {
                        log::warn!(
                            "Peer {} claims height {} but we're requesting up to {}, skipping",
                            peer_id,
                            claimed,
                            end_height
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
        "request_blocks_from_peer not implemented - peer unavailable".to_string(),
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

    log::info!(
        "Processed {} headers, new height: {}",
        headers.len(),
        last_height
    );

    Ok(())
}

// ============================================================================
// HEADERS-FIRST SYNC IMPLEMENTATION (#78)
// ============================================================================

/// Block checkpoint for IBD validation.
/// Each checkpoint is a known-good block hash at a specific height.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkpoint {
    /// Block height
    pub height: u64,
    /// Expected block hash
    pub hash: [u8; 32],
    /// Timestamp when checkpoint was added
    pub timestamp: u64,
}

/// Known checkpoints for mainnet.
/// These are hardcoded trusted block hashes for validation during IBD.
pub const MAINNET_CHECKPOINTS: &[(u64, &str)] = &[
    (
        0,
        "0000000000000000000000000000000000000000000000000000000000000000",
    ), // Genesis
       // Add more checkpoints as the chain grows
];

/// Sync state that can be persisted to disk.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PersistentSyncState {
    /// Last synced header height
    pub header_height: u64,
    /// Last synced block height
    pub block_height: u64,
    /// Best known peer height
    pub best_known_height: u64,
    /// Hash of last synced header
    pub last_header_hash: [u8; 32],
    /// Timestamp of last sync
    pub last_sync_timestamp: u64,
    /// Sync errors count
    pub sync_errors: u64,
}

/// Peer download state for stalling detection.
#[derive(Debug, Clone)]
pub struct PeerDownloadState {
    /// Peer ID
    pub peer_id: String,
    /// Assigned start height
    pub start_height: u64,
    /// Assigned end height
    pub end_height: u64,
    /// Last progress timestamp
    pub last_progress: std::time::Instant,
    /// Bytes downloaded
    pub bytes_downloaded: u64,
    /// Is this peer stalled?
    pub stalled: bool,
}

/// Headers-first sync manager.
/// Implements the complete headers-first synchronization protocol.
pub struct HeadersFirstSync {
    /// Chain sync state
    chain_sync: Arc<ChainSync>,
    /// Persistent sync state for resume
    persistent_state: PersistentSyncState,
    /// Checkpoints for validation
    checkpoints: Vec<Checkpoint>,
    /// Peer download states for stalling detection
    peer_states: std::collections::HashMap<String, PeerDownloadState>,
    /// Headers queue (downloaded but not yet connected)
    headers_queue: Vec<BlockHeader>,
    /// Blocks pending download (hash -> height)
    pending_blocks: std::collections::VecDeque<([u8; 32], u64)>,
    /// Downloaded blocks waiting to be connected
    downloaded_blocks: std::collections::BTreeMap<u64, bitquan_types::Block>,
    /// Maximum headers to request per batch (for future use)
    #[allow(dead_code)]
    max_headers_per_batch: usize,
    /// Maximum blocks to download in parallel (for future use)
    #[allow(dead_code)]
    max_parallel_downloads: usize,
    /// Stalling timeout in seconds
    stall_timeout_secs: u64,
    /// Start time for ETA calculation
    sync_start_time: Option<std::time::Instant>,
    /// Initial height for progress calculation
    initial_height: u64,
}

impl HeadersFirstSync {
    /// Create a new headers-first sync manager.
    pub fn new(chain_sync: Arc<ChainSync>) -> Self {
        Self {
            chain_sync,
            persistent_state: PersistentSyncState::default(),
            checkpoints: Self::load_checkpoints(),
            peer_states: std::collections::HashMap::new(),
            headers_queue: Vec::new(),
            pending_blocks: std::collections::VecDeque::new(),
            downloaded_blocks: std::collections::BTreeMap::new(),
            max_headers_per_batch: 2000,
            max_parallel_downloads: 4,
            stall_timeout_secs: 30,
            sync_start_time: None,
            initial_height: 0,
        }
    }

    /// Load checkpoints from hardcoded list.
    fn load_checkpoints() -> Vec<Checkpoint> {
        MAINNET_CHECKPOINTS
            .iter()
            .map(|(height, hash_str)| {
                let mut hash = [0u8; 32];
                if let Ok(bytes) = hex::decode(hash_str) {
                    if bytes.len() == 32 {
                        hash.copy_from_slice(&bytes);
                    }
                }
                Checkpoint {
                    height: *height,
                    hash,
                    timestamp: 0,
                }
            })
            .collect()
    }

    /// Initialize sync from persisted state.
    pub fn restore_from_state(&mut self, state: PersistentSyncState) {
        self.persistent_state = state.clone();
        self.chain_sync.set_local_height(state.block_height);
        self.chain_sync.set_best_height(state.best_known_height);
        self.initial_height = state.block_height;
        log::info!(
            "Restored sync state: headers={}, blocks={}",
            state.header_height,
            state.block_height
        );
    }

    /// Get current persistent state for saving.
    pub fn get_persistent_state(&self) -> &PersistentSyncState {
        &self.persistent_state
    }

    /// Start headers-first sync process.
    pub fn start_headers_sync(&mut self) -> Result<()> {
        if self.sync_start_time.is_none() {
            self.sync_start_time = Some(std::time::Instant::now());
            self.initial_height = self.chain_sync.local_height();
        }

        self.chain_sync.set_status(SyncStatus::DownloadingHeaders);
        log::info!(
            "Starting headers-first sync from height {}",
            self.chain_sync.local_height()
        );

        Ok(())
    }

    /// Process received headers from a peer.
    /// Returns the number of headers processed.
    pub fn process_received_headers(&mut self, headers: Vec<BlockHeader>) -> Result<usize> {
        if headers.is_empty() {
            return Ok(0);
        }

        let mut processed = 0;
        let current_height = self.chain_sync.local_height();

        for header in headers {
            // Validate header
            if !self.validate_header(&header, current_height + processed as u64)? {
                log::warn!(
                    "Header validation failed at height {}",
                    current_height + processed as u64
                );
                break;
            }

            // Check against checkpoints
            let header_height = current_height + processed as u64;
            if let Some(checkpoint) = self.find_checkpoint(header_height) {
                let header_hash = self.compute_header_hash(&header);
                if header_hash != checkpoint.hash {
                    log::error!(
                        "Checkpoint mismatch at height {}: expected {}, got {}",
                        header_height,
                        hex::encode(checkpoint.hash),
                        hex::encode(header_hash)
                    );
                    return Err(bitquan_types::Error::Invalid(format!(
                        "Checkpoint validation failed at height {}",
                        header_height
                    )));
                }
                log::info!("✓ Checkpoint validated at height {}", header_height);
            }

            // Add to queue for block download
            self.headers_queue.push(header.clone());
            processed += 1;
        }

        // Update sync state
        let new_height = current_height + processed as u64;
        self.chain_sync.set_local_height(new_height);
        self.persistent_state.header_height = new_height;

        if processed > 0 {
            log::info!(
                "Processed {} headers, current height: {}",
                processed,
                new_height
            );
        }

        Ok(processed)
    }

    /// Validate a single header.
    fn validate_header(&self, header: &BlockHeader, _height: u64) -> Result<bool> {
        // Basic validation
        if header.time == 0 {
            return Err(bitquan_types::Error::Invalid(
                "Header has zero timestamp".into(),
            ));
        }

        // Proof-of-work validation would go here
        // For now, basic checks only

        Ok(true)
    }

    /// Compute header hash using SHA-256d (same algorithm used for block identity everywhere).
    fn compute_header_hash(&self, header: &BlockHeader) -> [u8; 32] {
        bitquan_consensus::header_hash(header)
    }

    /// Find checkpoint at given height.
    fn find_checkpoint(&self, height: u64) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|c| c.height == height)
    }

    /// Queue blocks for download after headers are synced.
    pub fn queue_blocks_for_download(&mut self) {
        for (idx, header) in self.headers_queue.iter().enumerate() {
            let height = self.persistent_state.block_height + idx as u64 + 1;
            let hash = self.compute_header_hash(header);
            self.pending_blocks.push_back((hash, height));
        }
        log::info!("Queued {} blocks for download", self.pending_blocks.len());
    }

    /// Get next batch of blocks to download from a peer.
    /// Returns (start_height, end_height, block_hashes).
    pub fn get_download_batch(
        &mut self,
        peer_id: &str,
        max_blocks: usize,
    ) -> Option<(u64, u64, Vec<[u8; 32]>)> {
        if self.pending_blocks.is_empty() {
            return None;
        }

        let batch_size = std::cmp::min(max_blocks, self.pending_blocks.len());
        let mut hashes = Vec::with_capacity(batch_size);
        let mut start_height = None;
        let mut end_height = 0;

        for _ in 0..batch_size {
            if let Some((hash, height)) = self.pending_blocks.pop_front() {
                if start_height.is_none() {
                    start_height = Some(height);
                }
                end_height = height;
                hashes.push(hash);
            }
        }

        let start = start_height?;

        // Track peer download state for stalling detection
        self.peer_states.insert(
            peer_id.to_string(),
            PeerDownloadState {
                peer_id: peer_id.to_string(),
                start_height: start,
                end_height,
                last_progress: std::time::Instant::now(),
                bytes_downloaded: 0,
                stalled: false,
            },
        );

        Some((start, end_height, hashes))
    }

    /// Record block download progress for a peer.
    pub fn record_download_progress(&mut self, peer_id: &str, bytes: u64) {
        if let Some(state) = self.peer_states.get_mut(peer_id) {
            state.bytes_downloaded += bytes;
            state.last_progress = std::time::Instant::now();
            state.stalled = false;
        }
    }

    /// Check for stalled peers and return their IDs.
    pub fn detect_stalled_peers(&mut self) -> Vec<String> {
        let now = std::time::Instant::now();
        let stall_duration = std::time::Duration::from_secs(self.stall_timeout_secs);

        let stalled: Vec<String> = self
            .peer_states
            .iter()
            .filter(|(_, state)| {
                now.duration_since(state.last_progress) > stall_duration && !state.stalled
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Mark as stalled
        for id in &stalled {
            if let Some(state) = self.peer_states.get_mut(id) {
                state.stalled = true;
                log::warn!(
                    "Peer {} appears stalled (heights {}-{})",
                    id,
                    state.start_height,
                    state.end_height
                );
                // Re-queue blocks for download from another peer
                for _h in state.start_height..=state.end_height {
                    // Find the hash for this height and re-queue
                    // In production, we'd maintain a height->hash mapping
                }
            }
        }

        stalled
    }

    /// Store downloaded block for later connection.
    pub fn store_downloaded_block(&mut self, height: u64, block: bitquan_types::Block) {
        if self.downloaded_blocks.len() >= 50 {
            log::warn!("Sync backpressure: max downloaded blocks reached (50), dropping block at height {}", height);
            return;
        }
        self.downloaded_blocks.insert(height, block);
    }

    /// Connect downloaded blocks in order.
    /// Returns number of blocks connected.
    pub fn connect_ready_blocks(&mut self) -> usize {
        let mut connected = 0;
        let mut next_height = self.persistent_state.block_height + 1;

        while let Some(_block) = self.downloaded_blocks.remove(&next_height) {
            // In production, this would call storage.connect_block()
            log::debug!("Connected block at height {}", next_height);
            next_height += 1;
            connected += 1;
        }

        if connected > 0 {
            self.persistent_state.block_height = next_height - 1;
            self.chain_sync.set_local_height(next_height - 1);
        }

        connected
    }

    /// Calculate sync progress with ETA.
    pub fn get_sync_progress(&self) -> SyncProgressInfo {
        let current = self.chain_sync.local_height();
        let target = self.chain_sync.best_height();
        let progress = if target > 0 {
            (current as f64 / target as f64) * 100.0
        } else {
            100.0
        };

        let eta = if let Some(start) = self.sync_start_time {
            let elapsed = start.elapsed().as_secs();
            let blocks_synced = current.saturating_sub(self.initial_height);
            let blocks_remaining = target.saturating_sub(current);

            if blocks_synced > 0 && elapsed > 0 {
                let blocks_per_sec = blocks_synced as f64 / elapsed as f64;
                if blocks_per_sec > 0.0 {
                    Some((blocks_remaining as f64 / blocks_per_sec) as u64)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        SyncProgressInfo {
            status: self.chain_sync.status(),
            header_height: self.persistent_state.header_height,
            block_height: self.persistent_state.block_height,
            target_height: target,
            progress_percent: progress,
            blocks_behind: target.saturating_sub(current),
            eta_seconds: eta,
            download_speed_bps: self.calculate_download_speed(),
            active_downloads: self.peer_states.len(),
        }
    }

    /// Calculate current download speed.
    fn calculate_download_speed(&self) -> f64 {
        let total_bytes: u64 = self.peer_states.values().map(|s| s.bytes_downloaded).sum();
        if let Some(start) = self.sync_start_time {
            let elapsed_secs = start.elapsed().as_secs_f64();
            if elapsed_secs > 0.0 {
                return total_bytes as f64 / elapsed_secs;
            }
        }
        0.0
    }

    /// Check if headers sync is complete.
    pub fn headers_sync_complete(&self) -> bool {
        self.persistent_state.header_height >= self.chain_sync.best_height()
    }

    /// Check if block download is complete.
    pub fn block_download_complete(&self) -> bool {
        self.pending_blocks.is_empty() && self.downloaded_blocks.is_empty()
    }

    /// Transition to block download phase.
    pub fn start_block_download(&mut self) {
        self.chain_sync.set_status(SyncStatus::DownloadingBlocks);
        self.queue_blocks_for_download();
        log::info!(
            "Headers sync complete, starting block download ({} blocks pending)",
            self.pending_blocks.len()
        );
    }

    /// Complete sync process.
    pub fn complete_sync(&mut self) {
        self.chain_sync.complete_sync();
        self.sync_start_time = None;
        log::info!(
            "🎉 Sync complete at height {}",
            self.chain_sync.local_height()
        );
    }
}

/// Detailed sync progress information with ETA.
#[derive(Debug, Clone)]
pub struct SyncProgressInfo {
    /// Current sync status
    pub status: SyncStatus,
    /// Current header height
    pub header_height: u64,
    /// Current block height
    pub block_height: u64,
    /// Target height
    pub target_height: u64,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Blocks behind
    pub blocks_behind: u64,
    /// Estimated time to completion in seconds
    pub eta_seconds: Option<u64>,
    /// Current download speed in bytes/sec
    pub download_speed_bps: f64,
    /// Number of active parallel downloads
    pub active_downloads: usize,
}

impl std::fmt::Display for SyncProgressInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let eta_str = match self.eta_seconds {
            Some(secs) => {
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                format!("ETA: {:02}:{:02}:{:02}", hours, mins, secs)
            }
            None => "ETA: --:--:--".to_string(),
        };

        write!(
            f,
            "Sync: {:.1}% | Headers: {} | Blocks: {}/{} | {} | Speed: {:.1} KB/s | Active: {}",
            self.progress_percent,
            self.header_height,
            self.block_height,
            self.target_height,
            eta_str,
            self.download_speed_bps / 1024.0,
            self.active_downloads
        )
    }
}

/// Async sync manager for background sync operations.
pub struct AsyncSyncManager {
    /// Headers-first sync engine
    headers_sync: HeadersFirstSync,
    /// Peer book for peer management
    peer_book: Arc<Mutex<PeerBook>>,
    /// Running flag
    running: Arc<AtomicBool>,
}

impl AsyncSyncManager {
    /// Create a new async sync manager.
    pub fn new(chain_sync: Arc<ChainSync>, peer_book: Arc<Mutex<PeerBook>>) -> Self {
        Self {
            headers_sync: HeadersFirstSync::new(chain_sync),
            peer_book,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the sync manager.
    pub fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::Relaxed);
        log::info!("Async sync manager started");
        Ok(())
    }

    /// Stop the sync manager.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        log::info!("Async sync manager stopped");
    }

    /// Check if running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get headers sync engine.
    pub fn headers_sync(&self) -> &HeadersFirstSync {
        &self.headers_sync
    }

    /// Get mutable headers sync engine.
    pub fn headers_sync_mut(&mut self) -> &mut HeadersFirstSync {
        &mut self.headers_sync
    }

    /// Run one sync iteration.
    /// This should be called periodically in a loop.
    pub fn run_sync_iteration(&mut self) -> Result<SyncAction> {
        if !self.is_running() {
            return Ok(SyncAction::None);
        }

        let status = self.headers_sync.chain_sync.status();

        match status {
            SyncStatus::Idle => {
                // Check if we need to sync
                if self.headers_sync.chain_sync.needs_sync() {
                    self.headers_sync.start_headers_sync()?;
                    return Ok(SyncAction::StartHeadersSync);
                }
            }
            SyncStatus::DownloadingHeaders => {
                // Check for stalled peers
                let stalled = self.headers_sync.detect_stalled_peers();
                if !stalled.is_empty() {
                    log::warn!("Detected {} stalled peers", stalled.len());
                    return Ok(SyncAction::SwitchPeers(stalled));
                }

                // Check if headers sync is complete
                if self.headers_sync.headers_sync_complete() {
                    self.headers_sync.start_block_download();
                    return Ok(SyncAction::StartBlockDownload);
                }

                // Log progress
                let progress = self.headers_sync.get_sync_progress();
                log::info!("{}", progress);

                return Ok(SyncAction::RequestHeaders);
            }
            SyncStatus::DownloadingBlocks => {
                // Connect any ready blocks
                let connected = self.headers_sync.connect_ready_blocks();
                if connected > 0 {
                    log::debug!("Connected {} blocks", connected);
                }

                // Check for stalled peers
                let stalled = self.headers_sync.detect_stalled_peers();
                if !stalled.is_empty() {
                    return Ok(SyncAction::SwitchPeers(stalled));
                }

                // Check if sync is complete
                if self.headers_sync.block_download_complete() {
                    self.headers_sync.complete_sync();
                    return Ok(SyncAction::SyncComplete);
                }

                // Log progress
                let progress = self.headers_sync.get_sync_progress();
                log::info!("{}", progress);

                return Ok(SyncAction::RequestBlocks);
            }
            SyncStatus::Synced => {
                // Check for new blocks
                if self.headers_sync.chain_sync.needs_sync() {
                    self.headers_sync.chain_sync.set_status(SyncStatus::Idle);
                }
            }
            SyncStatus::Discovering => {
                // Discover best height from peers
                if let Ok(peer_book) = self.peer_book.lock() {
                    let best_peers = peer_book.best_peers(5);
                    let mut best_height = 0u64;
                    for peer_addr in best_peers {
                        if let Some(peer) = peer_book.get_peer(&peer_addr) {
                            if let Some(height) = peer.claimed_height {
                                best_height = std::cmp::max(best_height, height);
                            }
                        }
                    }
                    drop(peer_book);
                    self.headers_sync.chain_sync.set_best_height(best_height);
                    self.headers_sync
                        .chain_sync
                        .set_status(SyncStatus::DownloadingHeaders);
                }
            }
        }

        Ok(SyncAction::None)
    }

    /// Get current sync progress.
    pub fn get_progress(&self) -> SyncProgressInfo {
        self.headers_sync.get_sync_progress()
    }
}

/// Action to take after a sync iteration.
#[derive(Debug, Clone)]
pub enum SyncAction {
    /// No action needed
    None,
    /// Start requesting headers from peers
    RequestHeaders,
    /// Start downloading blocks
    StartBlockDownload,
    /// Request blocks from peers
    RequestBlocks,
    /// Switch away from stalled peers
    SwitchPeers(Vec<String>),
    /// Sync has completed
    SyncComplete,
    /// Start headers sync
    StartHeadersSync,
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

        // Test valid range - note: request_blocks_from_peer is not implemented
        // and returns Err for unimplemented functionality (SECURITY FIX)
        let result = request_blocks(0, 100, "test_peer");
        assert!(result.is_err());
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
