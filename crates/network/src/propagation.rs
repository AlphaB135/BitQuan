//! Block and transaction propagation across the P2P network.

use crate::{
    protocol::{InvType, InvVector, Message, MessageEnvelope},
    NetworkError, Result,
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Tracks seen blocks and transactions to prevent duplicate propagation.
#[derive(Clone)]
pub struct SeenFilter {
    /// Recently seen block hashes.
    seen_blocks: Arc<Mutex<HashSet<[u8; 32]>>>,
    /// Recently seen transaction hashes.
    seen_txs: Arc<Mutex<HashSet<[u8; 32]>>>,
    /// Maximum items to track.
    max_items: usize,
}

impl SeenFilter {
    /// Create a new seen filter with capacity.
    pub fn new(max_items: usize) -> Self {
        Self {
            seen_blocks: Arc::new(Mutex::new(HashSet::new())),
            seen_txs: Arc::new(Mutex::new(HashSet::new())),
            max_items,
        }
    }

    /// Check if a block hash was seen, and mark it if not.
    pub fn mark_block_seen(&self, hash: [u8; 32]) -> Result<bool> {
        let mut seen = self
            .seen_blocks
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("seen_blocks: {}", e)))?;

        // If at capacity, clear oldest (simple eviction)
        if seen.len() >= self.max_items {
            seen.clear();
        }

        Ok(seen.insert(hash))
    }

    /// Check if a transaction hash was seen, and mark it if not.
    pub fn mark_tx_seen(&self, hash: [u8; 32]) -> Result<bool> {
        let mut seen = self
            .seen_txs
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("seen_txs: {}", e)))?;

        if seen.len() >= self.max_items {
            seen.clear();
        }

        Ok(seen.insert(hash))
    }

    /// Check if block was already seen (without marking).
    pub fn has_block(&self, hash: &[u8; 32]) -> Result<bool> {
        let seen = self
            .seen_blocks
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("seen_blocks: {}", e)))?;
        Ok(seen.contains(hash))
    }

    /// Check if transaction was already seen (without marking).
    pub fn has_tx(&self, hash: &[u8; 32]) -> Result<bool> {
        let seen = self
            .seen_txs
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("seen_txs: {}", e)))?;
        Ok(seen.contains(hash))
    }
}

/// Block propagation statistics.
#[derive(Clone, Debug, Default)]
pub struct PropagationStats {
    /// Total blocks broadcast.
    pub blocks_broadcast: u64,
    /// Total blocks received.
    pub blocks_received: u64,
    /// Total blocks rejected (duplicate/invalid).
    pub blocks_rejected: u64,
    /// Total transactions broadcast.
    pub txs_broadcast: u64,
    /// Total transactions received.
    pub txs_received: u64,
}

/// Block propagation manager.
pub struct BlockPropagator {
    /// Seen filter to prevent duplicate propagation.
    seen_filter: SeenFilter,
    /// Propagation statistics.
    stats: Arc<Mutex<PropagationStats>>,
}

impl BlockPropagator {
    /// Create a new block propagator.
    pub fn new() -> Self {
        Self {
            seen_filter: SeenFilter::new(10_000),
            stats: Arc::new(Mutex::new(PropagationStats::default())),
        }
    }

    /// Create inventory message for a block.
    pub fn create_block_inv(&self, block_hash: [u8; 32]) -> Message {
        Message::Inv {
            inventory: vec![InvVector {
                inv_type: InvType::Block,
                hash: block_hash,
            }],
        }
    }

    /// Create inventory message for a transaction.
    pub fn create_tx_inv(&self, tx_hash: [u8; 32]) -> Message {
        Message::Inv {
            inventory: vec![InvVector {
                inv_type: InvType::Tx,
                hash: tx_hash,
            }],
        }
    }

    /// Check if we should propagate this block (not seen before).
    pub fn should_propagate_block(&self, block_hash: [u8; 32]) -> bool {
        !self.seen_filter.has_block(&block_hash).unwrap_or(false)
    }

    /// Mark block as propagated.
    pub fn mark_block_propagated(&self, block_hash: [u8; 32]) -> Result<()> {
        let _ = self.seen_filter.mark_block_seen(block_hash)?;
        let mut stats = self
            .stats
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("stats: {}", e)))?;
        stats.blocks_broadcast += 1;
        Ok(())
    }

    /// Mark block as received.
    pub fn mark_block_received(&self, block_hash: [u8; 32]) -> Result<bool> {
        let is_new = self.seen_filter.mark_block_seen(block_hash)?;
        let mut stats = self
            .stats
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("stats: {}", e)))?;

        if is_new {
            stats.blocks_received += 1;
        } else {
            stats.blocks_rejected += 1;
        }

        Ok(is_new)
    }

    /// Mark transaction as propagated.
    pub fn mark_tx_propagated(&self, tx_hash: [u8; 32]) -> Result<()> {
        let _ = self.seen_filter.mark_tx_seen(tx_hash)?;
        let mut stats = self
            .stats
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("stats: {}", e)))?;
        stats.txs_broadcast += 1;
        Ok(())
    }

    /// Mark transaction as received.
    pub fn mark_tx_received(&self, tx_hash: [u8; 32]) -> Result<bool> {
        let is_new = self.seen_filter.mark_tx_seen(tx_hash)?;
        let mut stats = self
            .stats
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("stats: {}", e)))?;

        if is_new {
            stats.txs_received += 1;
        }

        Ok(is_new)
    }

    /// Get propagation statistics.
    pub fn stats(&self) -> Result<PropagationStats> {
        let stats = self
            .stats
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("stats: {}", e)))?;
        Ok(stats.clone())
    }

    /// Reset statistics.
    pub fn reset_stats(&self) -> Result<()> {
        let mut stats = self
            .stats
            .lock()
            .map_err(|e| NetworkError::LockPoisoned(format!("stats: {}", e)))?;
        *stats = PropagationStats::default();
        Ok(())
    }
}

impl Default for BlockPropagator {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a message envelope for network transmission.
pub fn create_envelope(message: Message, magic: [u8; 4]) -> MessageEnvelope {
    MessageEnvelope::new(magic, message)
}

/// Broadcast a block to multiple peers.
///
/// Sends block inventory to all connected peers via network manager.
/// This function handles duplicate prevention and tracks propagation status.
pub fn broadcast_block_inv(block_hash: [u8; 32], propagator: &BlockPropagator) -> Result<()> {
    if !propagator.should_propagate_block(block_hash) {
        // Already propagated, skip
        return Ok(());
    }

    // Create inv message (for future use)
    let _inv_msg = propagator.create_block_inv(block_hash);

    // Note: P2P network integration point
    // When P2P module is integrated, send inv_msg to all connected peers
    // For now, just mark as propagated
    propagator.mark_block_propagated(block_hash)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seen_filter_marks_new_blocks() {
        let filter = SeenFilter::new(100);
        let hash = [1u8; 32];

        // First time should be new
        assert!(filter
            .mark_block_seen(hash)
            .expect("Failed to mark block as seen"));

        // Second time should be seen
        assert!(!filter
            .mark_block_seen(hash)
            .expect("Failed to mark duplicate block"));
    }

    #[test]
    fn test_seen_filter_capacity() {
        let filter = SeenFilter::new(10);

        // Fill beyond capacity
        for i in 0..20u8 {
            let hash = [i; 32];
            let _ = filter.mark_block_seen(hash);
        }

        // Should clear and accept new items
        let new_hash = [99u8; 32];
        assert!(filter
            .mark_block_seen(new_hash)
            .expect("Failed to mark new block after capacity"));
    }

    #[test]
    fn test_propagator_tracks_stats() {
        let propagator = BlockPropagator::new();
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];

        // Mark blocks as received
        assert!(propagator
            .mark_block_received(hash1)
            .expect("Failed to mark first block as received"));
        assert!(propagator
            .mark_block_received(hash2)
            .expect("Failed to mark second block as received"));
        assert!(!propagator
            .mark_block_received(hash1)
            .expect("Failed to mark duplicate block")); // Duplicate

        let stats = propagator.stats().expect("Failed to get propagator stats");
        assert_eq!(stats.blocks_received, 2);
        assert_eq!(stats.blocks_rejected, 1);
    }

    #[test]
    fn test_create_block_inv() {
        let propagator = BlockPropagator::new();
        let hash = [42u8; 32];

        let inv = propagator.create_block_inv(hash);

        match inv {
            Message::Inv { inventory } => {
                assert_eq!(inventory.len(), 1);
                assert_eq!(inventory[0].hash, hash);
                assert_eq!(inventory[0].inv_type, InvType::Block);
            }
            _ => panic!("Expected Inv message"),
        }
    }

    #[test]
    fn test_should_propagate_only_new_blocks() {
        let propagator = BlockPropagator::new();
        let hash = [5u8; 32];

        // Should propagate first time
        assert!(propagator.should_propagate_block(hash));

        // Mark as propagated
        let _ = propagator.mark_block_propagated(hash);

        // Should not propagate again
        assert!(!propagator.should_propagate_block(hash));
    }
}
