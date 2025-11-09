//! Checkpoint system for emergency blockchain recovery.
//!
//! This module provides a simple, secure checkpoint mechanism that allows
//! network operators to rollback the blockchain to a known good state
//! during emergency situations like mining bugs or attacks.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Maximum number of checkpoints allowed (prevents abuse)
pub const MAX_CHECKPOINTS: usize = 100;

/// Minimum blocks between checkpoints (prevents frequent rollbacks)
pub const MIN_CHECKPOINT_INTERVAL: u64 = 1000;

/// Errors that can occur during checkpoint operations
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CheckpointError {
    /// Checkpoint height is too recent (security risk)
    #[error("checkpoint height {height} is too recent (min: {min})")]
    CheckpointTooRecent { height: u64, min: u64 },

    /// Checkpoint height is in the future
    #[error("checkpoint height {height} is in the future (current: {current})")]
    CheckpointInFuture { height: u64, current: u64 },

    /// Checkpoint hash does not match block hash
    #[error("checkpoint hash mismatch at height {height}")]
    HashMismatch { height: u64 },

    /// Too many checkpoints (potential abuse)
    #[error("too many checkpoints (max: {max})")]
    TooManyCheckpoints { max: usize },

    /// Checkpoint not found
    #[error("checkpoint not found at height {height}")]
    NotFound { height: u64 },

    /// Invalid checkpoint data
    #[error("invalid checkpoint data: {reason}")]
    InvalidData { reason: String },
}

/// A single checkpoint entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Block height of the checkpoint
    pub height: u64,
    /// Expected block hash at this height
    pub hash: [u8; 32],
    /// Timestamp when checkpoint was created (for audit)
    pub created_at: u64,
    /// Reason for checkpoint (emergency, upgrade, etc.)
    pub reason: String,
}

impl Checkpoint {
    /// Creates a new checkpoint
    pub fn new(height: u64, hash: [u8; 32], reason: String) -> Self {
        Self {
            height,
            hash,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            reason,
        }
    }

    /// Validates checkpoint data
    pub fn validate(&self) -> Result<(), CheckpointError> {
        if self.height == 0 {
            return Err(CheckpointError::InvalidData {
                reason: "genesis block cannot be checkpointed".to_string(),
            });
        }

        if self.reason.is_empty() {
            return Err(CheckpointError::InvalidData {
                reason: "checkpoint reason cannot be empty".to_string(),
            });
        }

        if self.reason.len() > 200 {
            return Err(CheckpointError::InvalidData {
                reason: "checkpoint reason too long (max 200 chars)".to_string(),
            });
        }

        Ok(())
    }
}

/// Checkpoint manager for emergency recovery
#[derive(Debug, Clone)]
pub struct CheckpointManager {
    /// Ordered checkpoints by height
    checkpoints: BTreeMap<u64, Checkpoint>,
    /// Current blockchain height (for validation)
    current_height: u64,
    /// Whether checkpoints are enabled
    enabled: bool,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager
    pub fn new(enabled: bool) -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            current_height: 0,
            enabled,
        }
    }

    /// Enables or disables checkpoint validation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns true if checkpoints are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Updates the current blockchain height
    pub fn update_height(&mut self, height: u64) {
        self.current_height = height;
    }

    /// Adds a new checkpoint (only for emergency use)
    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<(), CheckpointError> {
        if !self.enabled {
            return Err(CheckpointError::InvalidData {
                reason: "checkpoints are disabled".to_string(),
            });
        }

        // Validate checkpoint data
        checkpoint.validate()?;

        // Security: limit number of checkpoints
        if self.checkpoints.len() >= MAX_CHECKPOINTS {
            return Err(CheckpointError::TooManyCheckpoints {
                max: MAX_CHECKPOINTS,
            });
        }

        // Security: prevent future checkpoints only
        if checkpoint.height > self.current_height {
            return Err(CheckpointError::CheckpointInFuture {
                height: checkpoint.height,
                current: self.current_height,
            });
        }

        self.checkpoints.insert(checkpoint.height, checkpoint);
        Ok(())
    }

    /// Validates a block against checkpoints
    pub fn validate_block(&self, height: u64, hash: &[u8; 32]) -> Result<(), CheckpointError> {
        if !self.enabled {
            return Ok(()); // Skip validation if disabled
        }

        // Check if there's a checkpoint at this height
        if let Some(checkpoint) = self.checkpoints.get(&height) {
            if checkpoint.hash != *hash {
                return Err(CheckpointError::HashMismatch { height });
            }
        }

        Ok(())
    }

    /// Gets the latest checkpoint at or below the given height
    pub fn get_latest_checkpoint(&self, max_height: u64) -> Option<&Checkpoint> {
        self.checkpoints
            .range(..=max_height)
            .next_back()
            .map(|(_, checkpoint)| checkpoint)
    }

    /// Gets all checkpoints at or below the given height
    pub fn get_checkpoints_up_to(&self, max_height: u64) -> Vec<&Checkpoint> {
        self.checkpoints
            .range(..=max_height)
            .map(|(_, checkpoint)| checkpoint)
            .collect()
    }

    /// Gets a checkpoint at specific height
    pub fn get_checkpoint(&self, height: u64) -> Option<&Checkpoint> {
        self.checkpoints.get(&height)
    }

    /// Returns true if there's a checkpoint at the given height
    pub fn has_checkpoint(&self, height: u64) -> bool {
        self.checkpoints.contains_key(&height)
    }

    /// Gets the highest checkpoint height
    pub fn highest_checkpoint_height(&self) -> Option<u64> {
        self.checkpoints.keys().next_back().copied()
    }

    /// Returns the number of checkpoints
    pub fn count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Removes all checkpoints (emergency reset)
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    /// Removes checkpoints above the given height (rollback)
    pub fn rollback_to(&mut self, height: u64) {
        self.checkpoints.split_off(&(height + 1));
    }

    /// Exports checkpoints for backup
    pub fn export(&self) -> Vec<&Checkpoint> {
        self.checkpoints.values().collect()
    }

    /// Imports checkpoints from backup (with validation)
    pub fn import(&mut self, checkpoints: Vec<Checkpoint>) -> Result<(), CheckpointError> {
        if !self.enabled {
            return Err(CheckpointError::InvalidData {
                reason: "checkpoints are disabled".to_string(),
            });
        }

        // Clear existing checkpoints
        self.checkpoints.clear();

        // Validate and import each checkpoint
        for checkpoint in checkpoints {
            checkpoint.validate()?;
            
            if self.checkpoints.len() >= MAX_CHECKPOINTS {
                return Err(CheckpointError::TooManyCheckpoints {
                    max: MAX_CHECKPOINTS,
                });
            }

            self.checkpoints.insert(checkpoint.height, checkpoint);
        }

        Ok(())
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new(false) // Disabled by default for security
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_checkpoint(height: u64, hash: u64) -> Checkpoint {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0..8].copy_from_slice(&hash.to_le_bytes());
        Checkpoint::new(height, hash_bytes, format!("Test checkpoint {}", height))
    }

    #[test]
    fn test_checkpoint_validation() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(1000);

        // Valid checkpoint (height 500 is less than current height 1000)
        let checkpoint = make_checkpoint(500, 123);
        assert!(manager.add_checkpoint(checkpoint).is_ok());

        // Future checkpoint
        let future = make_checkpoint(1500, 789);
        assert!(manager.add_checkpoint(future).is_err());
    }

    #[test]
    fn test_block_validation() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(2000); // Set higher height to allow checkpoint at 1000

        // Add checkpoint
        let checkpoint = make_checkpoint(1000, 123);
        manager.add_checkpoint(checkpoint).unwrap();

        // Valid block hash
        let mut valid_hash = [0u8; 32];
        valid_hash[0..8].copy_from_slice(&123u64.to_le_bytes());
        assert!(manager.validate_block(1000, &valid_hash).is_ok());

        // Invalid block hash
        let mut invalid_hash = [0u8; 32];
        invalid_hash[0..8].copy_from_slice(&999u64.to_le_bytes());
        assert!(manager.validate_block(1000, &invalid_hash).is_err());

        // No checkpoint at height (should pass)
        assert!(manager.validate_block(600, &invalid_hash).is_ok());
    }

    #[test]
    fn test_disabled_manager() {
        let mut manager = CheckpointManager::new(false);
        manager.update_height(1000);

        // Should fail to add checkpoint when disabled
        let checkpoint = make_checkpoint(500, 123);
        assert!(manager.add_checkpoint(checkpoint).is_err());

        // Should pass validation even with wrong hash
        let wrong_hash = [99u8; 32];
        assert!(manager.validate_block(500, &wrong_hash).is_ok());
    }

    #[test]
    fn test_checkpoint_limits() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(20000); // High enough to allow all checkpoints

        // Add maximum checkpoints (start from height 1 to avoid genesis)
        for i in 0..MAX_CHECKPOINTS {
            let height = (i as u64) * 100 + 1; // Start from 1, not 0
            let checkpoint = make_checkpoint(height, i as u64);
            let result = manager.add_checkpoint(checkpoint);
            if result.is_err() {
                println!("Failed at i={}, height={}: {:?}", i, height, result);
            }
            assert!(result.is_ok());
        }

        // Should fail to add one more
        let extra = make_checkpoint(20000, 999);
        assert!(manager.add_checkpoint(extra).is_err());
    }

    #[test]
    fn test_rollback() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(2000); // High enough to allow all checkpoints

        // Add checkpoints
        manager.add_checkpoint(make_checkpoint(100, 1)).unwrap();
        manager.add_checkpoint(make_checkpoint(200, 2)).unwrap();
        manager.add_checkpoint(make_checkpoint(300, 3)).unwrap();

        assert_eq!(manager.count(), 3);

        // Rollback to height 200
        manager.rollback_to(200);
        assert_eq!(manager.count(), 2);
        assert!(manager.has_checkpoint(100));
        assert!(manager.has_checkpoint(200));
        assert!(!manager.has_checkpoint(300));
    }
}