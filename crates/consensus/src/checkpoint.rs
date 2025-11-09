//! Automatic checkpoint system for emergency blockchain recovery.
//!
//! This module provides a simple, secure checkpoint mechanism that automatically
//! creates checkpoints and allows rollback to a known good state
//! during emergency situations like mining bugs or attacks.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Maximum number of checkpoints allowed (prevents abuse)
pub const MAX_CHECKPOINTS: usize = 100;

/// Auto-checkpoint interval (blocks)
pub const AUTO_CHECKPOINT_INTERVAL: u64 = 1000;

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

/// Checkpoint manager for automatic recovery
#[derive(Debug, Clone)]
pub struct CheckpointManager {
    /// Ordered checkpoints by height
    checkpoints: BTreeMap<u64, Checkpoint>,
    /// Current blockchain height (for validation)
    current_height: u64,
    /// Whether checkpoints are enabled
    enabled: bool,
    /// Auto-checkpoint interval (blocks)
    auto_checkpoint_interval: u64,
    /// Last auto-checkpoint height
    last_auto_checkpoint: u64,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager
    pub fn new(enabled: bool) -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            current_height: 0,
            enabled,
            auto_checkpoint_interval: AUTO_CHECKPOINT_INTERVAL,
            last_auto_checkpoint: 0,
        }
    }

    /// Creates a new checkpoint manager with custom interval
    pub fn new_with_interval(enabled: bool, interval: u64) -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            current_height: 0,
            enabled,
            auto_checkpoint_interval: interval,
            last_auto_checkpoint: 0,
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
        // Security: Height should never decrease
        if height < self.current_height {
            return;
        }
        
        self.current_height = height;
    }

    /// Updates height and creates auto-checkpoint if needed with verified block hash
    pub fn update_height_with_block(&mut self, height: u64, block_hash: [u8; 32]) -> Result<(), CheckpointError> {
        // Security: Height should never decrease
        if height < self.current_height {
            return Err(CheckpointError::InvalidData {
                reason: "Block height cannot decrease".to_string(),
            });
        }
        
        self.current_height = height;
        
        // Auto-checkpoint logic with security checks
        if self.enabled && self.should_create_auto_checkpoint(height) {
            self.create_auto_checkpoint_with_hash(height, block_hash)?;
        }
        
        Ok(())
    }

    /// Checks if auto-checkpoint should be created with security validation
    fn should_create_auto_checkpoint(&self, height: u64) -> bool {
        // Security: Never checkpoint at genesis
        if height == 0 {
            return false;
        }
        
        // Security: Never checkpoint in the future
        if height > self.current_height {
            return false;
        }
        
        // Security: Respect minimum interval
        if self.last_auto_checkpoint > 0 {
            if height < self.last_auto_checkpoint + self.auto_checkpoint_interval {
                return false;
            }
        }
        
        // First checkpoint logic
        if self.last_auto_checkpoint == 0 {
            return height >= self.auto_checkpoint_interval;
        }
        
        true
    }

    /// Creates an automatic checkpoint with security validation
    fn create_auto_checkpoint(&mut self, height: u64) -> Result<(), CheckpointError> {
        // Security: Never create checkpoint without actual block hash
        // This function should only be called with verified block data
        return Err(CheckpointError::InvalidData {
            reason: "Auto-checkpoint requires actual block hash - use create_auto_checkpoint_with_hash()".to_string(),
        });
    }

    /// Creates an automatic checkpoint with verified block hash
    pub fn create_auto_checkpoint_with_hash(&mut self, height: u64, block_hash: [u8; 32]) -> Result<(), CheckpointError> {
        // Security validations
        if height == 0 {
            return Err(CheckpointError::InvalidData {
                reason: "Cannot create checkpoint at genesis block".to_string(),
            });
        }

        if height > self.current_height {
            return Err(CheckpointError::CheckpointInFuture {
                height,
                current: self.current_height,
            });
        }

        if self.checkpoints.len() >= MAX_CHECKPOINTS {
            return Err(CheckpointError::TooManyCheckpoints {
                max: MAX_CHECKPOINTS,
            });
        }

        // Prevent duplicate checkpoints
        if self.checkpoints.contains_key(&height) {
            return Err(CheckpointError::InvalidData {
                reason: "Checkpoint already exists at this height".to_string(),
            });
        }

        let checkpoint = Checkpoint::new(
            height,
            block_hash,
            format!("Auto-checkpoint at height {}", height),
        );
        
        self.add_checkpoint(checkpoint)?;
        self.last_auto_checkpoint = height;
        
        Ok(())
    }

    /// Adds a new checkpoint with comprehensive security validation
    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<(), CheckpointError> {
        // Security: Must be enabled
        if !self.enabled {
            return Err(CheckpointError::InvalidData {
                reason: "Checkpoints are disabled".to_string(),
            });
        }

        // Security: Validate checkpoint data structure
        checkpoint.validate()?;

        // Security: Prevent checkpoint overflow
        if self.checkpoints.len() >= MAX_CHECKPOINTS {
            return Err(CheckpointError::TooManyCheckpoints {
                max: MAX_CHECKPOINTS,
            });
        }

        // Security: No future checkpoints
        if checkpoint.height > self.current_height {
            return Err(CheckpointError::CheckpointInFuture {
                height: checkpoint.height,
                current: self.current_height,
            });
        }

        // Security: Prevent duplicate checkpoints
        if self.checkpoints.contains_key(&checkpoint.height) {
            return Err(CheckpointError::InvalidData {
                reason: "Checkpoint already exists at this height".to_string(),
            });
        }

        // Security: Validate checkpoint reason
        if checkpoint.reason.is_empty() || checkpoint.reason.len() > 200 {
            return Err(CheckpointError::InvalidData {
                reason: "Invalid checkpoint reason length".to_string(),
            });
        }

        self.checkpoints.insert(checkpoint.height, checkpoint);
        Ok(())
    }

    /// Validates a block against checkpoints with security checks
    pub fn validate_block(&self, height: u64, hash: &[u8; 32]) -> Result<(), CheckpointError> {
        // Security: Skip validation if disabled
        if !self.enabled {
            return Ok(());
        }

        // Security: Validate input parameters
        if height == 0 {
            return Ok(()); // Genesis block has no checkpoint
        }

        // Security: Check hash length
        if hash.len() != 32 {
            return Err(CheckpointError::InvalidData {
                reason: "Invalid block hash length".to_string(),
            });
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

    /// Gets auto-checkpoint interval
    pub fn auto_checkpoint_interval(&self) -> u64 {
        self.auto_checkpoint_interval
    }

    /// Sets auto-checkpoint interval
    pub fn set_auto_checkpoint_interval(&mut self, interval: u64) {
        self.auto_checkpoint_interval = interval;
    }

    /// Gets last auto-checkpoint height
    pub fn last_auto_checkpoint(&self) -> u64 {
        self.last_auto_checkpoint
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

    /// Imports checkpoints from backup with comprehensive validation
    pub fn import(&mut self, checkpoints: Vec<Checkpoint>) -> Result<(), CheckpointError> {
        // Security: Must be enabled
        if !self.enabled {
            return Err(CheckpointError::InvalidData {
                reason: "Checkpoints are disabled".to_string(),
            });
        }

        // Security: Validate input size
        if checkpoints.is_empty() {
            return Err(CheckpointError::InvalidData {
                reason: "Cannot import empty checkpoint list".to_string(),
            });
        }

        if checkpoints.len() > MAX_CHECKPOINTS {
            return Err(CheckpointError::TooManyCheckpoints {
                max: MAX_CHECKPOINTS,
            });
        }

        // Security: Clear existing checkpoints only after validation
        let mut temp_checkpoints = BTreeMap::new();

        // Validate each checkpoint before importing
        for checkpoint in &checkpoints {
            // Validate checkpoint structure
            checkpoint.validate()?;

            // Security: No future checkpoints
            if checkpoint.height > self.current_height {
                return Err(CheckpointError::CheckpointInFuture {
                    height: checkpoint.height,
                    current: self.current_height,
                });
            }

            // Security: Prevent duplicates
            if temp_checkpoints.contains_key(&checkpoint.height) {
                return Err(CheckpointError::InvalidData {
                    reason: "Duplicate checkpoint height in import data".to_string(),
                });
            }

            temp_checkpoints.insert(checkpoint.height, checkpoint.clone());
        }

        // Security: Only clear after successful validation
        self.checkpoints = temp_checkpoints;
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
    fn test_auto_checkpoint_with_hash() {
        let mut manager = CheckpointManager::new_with_interval(true, 100);
        let test_hash = [1u8; 32];
        
        // Should create auto-checkpoint at height 100
        let result = manager.update_height_with_block(100, test_hash);
        assert!(result.is_ok());
        assert_eq!(manager.last_auto_checkpoint(), 100);
        assert_eq!(manager.count(), 1);
        
        // Should create auto-checkpoint at height 200
        let result = manager.update_height_with_block(200, test_hash);
        assert!(result.is_ok());
        assert_eq!(manager.last_auto_checkpoint(), 200);
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_auto_checkpoint_security() {
        let mut manager = CheckpointManager::new_with_interval(true, 100);
        let test_hash = [1u8; 32];
        
        // Should not create checkpoint at genesis
        let result = manager.update_height_with_block(0, test_hash);
        assert!(result.is_ok());
        assert_eq!(manager.last_auto_checkpoint(), 0);
        assert_eq!(manager.count(), 0);
        
        // Should not create checkpoint when disabled
        let mut disabled_manager = CheckpointManager::new_with_interval(false, 100);
        let result = disabled_manager.update_height_with_block(100, test_hash);
        assert!(result.is_ok());
        assert_eq!(disabled_manager.last_auto_checkpoint(), 0);
        assert_eq!(disabled_manager.count(), 0);
    }

    #[test]
    fn test_import_security() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(1000);
        
        // Should reject empty import
        let result = manager.import(vec![]);
        assert!(result.is_err());
        
        // Should reject import with too many checkpoints
        let mut checkpoints = Vec::new();
        for i in 0..101 {
            checkpoints.push(make_checkpoint(i as u64, i));
        }
        let result = manager.import(checkpoints);
        assert!(result.is_err());
        
        // Should reject import with duplicate heights
        let duplicate_checkpoints = vec![
            make_checkpoint(100, 1),
            make_checkpoint(100, 2),
        ];
        let result = manager.import(duplicate_checkpoints);
        assert!(result.is_err());
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
                // Log checkpoint creation failure for debugging
                // In production, this should use proper logging
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