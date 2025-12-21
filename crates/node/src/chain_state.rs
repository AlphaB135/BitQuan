//! Chain state management for blockchain height and consensus state.

use std::sync::atomic::{AtomicU64, Ordering};

/// Simple in-memory chain state for tracking blockchain height.
/// TODO: Replace with persistent implementation in Phase 8.
pub struct ChainState {
    height: AtomicU64,
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainState {
    /// Create a new chain state starting at height 0.
    pub fn new() -> Self {
        Self {
            height: AtomicU64::new(0),
        }
    }

    /// Get current chain height.
    pub fn get_height(&self) -> u64 {
        self.height.load(Ordering::Relaxed)
    }

    /// Set chain height.
    pub fn set_height(&self, height: u64) {
        self.height.store(height, Ordering::Relaxed);
    }

    /// Increment chain height by 1.
    pub fn increment(&self) -> u64 {
        self.height.fetch_add(1, Ordering::Relaxed) + 1
    }
}
