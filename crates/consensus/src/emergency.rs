//! Emergency response system for blockchain security incidents.
//!
//! This module provides tools for handling emergency situations like
//! mining bugs, network attacks, or other critical failures.

use crate::{Checkpoint, CheckpointError, CheckpointManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Emergency response actions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmergencyAction {
    /// Pause all block processing
    PauseProcessing,
    /// Enable checkpoint validation
    EnableCheckpoints,
    /// Add emergency checkpoint
    AddCheckpoint(Checkpoint),
    /// Rollback to specific height
    RollbackTo { height: u64 },
    /// Ban malicious peers
    BanPeers { peer_ids: Vec<String> },
    /// Send network alert
    SendAlert { message: String },
}

/// Emergency response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyConfig {
    /// Whether emergency response is enabled
    pub enabled: bool,
    /// Required signatures for emergency actions
    pub required_signatures: u8,
    /// Time window for emergency response (seconds)
    pub response_window: u64,

}

impl Default for EmergencyConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for security
            required_signatures: 3,
            response_window: 3600, // 1 hour

        }
    }
}

/// Emergency response manager
#[derive(Debug)]
pub struct EmergencyManager {
    /// Configuration
    config: EmergencyConfig,
    /// Checkpoint manager
    checkpoint_manager: CheckpointManager,
    /// Current blockchain height
    current_height: u64,
    /// Emergency actions history
    action_history: Vec<EmergencyAction>,
    /// Banned peers
    banned_peers: HashMap<String, String>,
    /// Whether processing is paused
    processing_paused: bool,
}

impl EmergencyManager {
    /// Creates a new emergency manager
    pub fn new(config: EmergencyConfig) -> Self {
        Self {
            checkpoint_manager: CheckpointManager::new(false),
            current_height: 0,
            action_history: Vec::new(),
            banned_peers: HashMap::new(),
            processing_paused: false,
            config,
        }
    }

    /// Returns true if emergency response is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Updates the current blockchain height
    pub fn update_height(&mut self, height: u64) {
        self.current_height = height;
        self.checkpoint_manager.update_height(height);
    }

    /// Returns true if processing is currently paused
    pub fn is_processing_paused(&self) -> bool {
        self.processing_paused
    }

    /// Executes an emergency action
    pub fn execute_action(
        &mut self,
        action: EmergencyAction,
    ) -> Result<(), EmergencyError> {
        if !self.config.enabled {
            return Err(EmergencyError::Disabled);
        }

        // Execute action
        match action.clone() {
            EmergencyAction::PauseProcessing => {
                self.processing_paused = true;
            }
            EmergencyAction::EnableCheckpoints => {
                self.checkpoint_manager.set_enabled(true);
            }
            EmergencyAction::AddCheckpoint(checkpoint) => {
                self.checkpoint_manager.add_checkpoint(checkpoint)?;
            }
            EmergencyAction::RollbackTo { height } => {
                if height >= self.current_height {
                    return Err(EmergencyError::InvalidRollback {
                        height,
                        current: self.current_height,
                    });
                }
                self.checkpoint_manager.rollback_to(height);
            }
            EmergencyAction::BanPeers { peer_ids } => {
                for peer_id in peer_ids {
                    self.banned_peers.insert(peer_id, "Emergency ban".to_string());
                }
            }
            EmergencyAction::SendAlert { message } => {
                // In a real implementation, this would send alerts to operators
                // Emergency alert - use proper logging in production
            }
        }

        // Record action
        self.action_history.push(action);
        Ok(())
    }

    /// Creates an emergency checkpoint at the specified height
    pub fn create_emergency_checkpoint(
        &mut self,
        height: u64,
        hash: [u8; 32],
        reason: String,
    ) -> Result<(), EmergencyError> {
        if !self.config.enabled {
            return Err(EmergencyError::Disabled);
        }



        // Enable checkpoints if not already enabled
        if !self.checkpoint_manager.is_enabled() {
            self.checkpoint_manager.set_enabled(true);
        }

        // Create checkpoint
        let checkpoint = Checkpoint::new(height, hash, reason);

        // Add checkpoint
        self.checkpoint_manager.add_checkpoint(checkpoint)?;

        Ok(())
    }

    /// Gets the checkpoint manager
    pub fn checkpoint_manager(&self) -> &CheckpointManager {
        &self.checkpoint_manager
    }

    /// Gets mutable checkpoint manager
    pub fn checkpoint_manager_mut(&mut self) -> &mut CheckpointManager {
        &mut self.checkpoint_manager
    }

    /// Checks if a peer is banned
    pub fn is_peer_banned(&self, peer_id: &str) -> bool {
        self.banned_peers.contains_key(peer_id)
    }

    /// Gets ban reason for a peer
    pub fn get_ban_reason(&self, peer_id: &str) -> Option<&str> {
        self.banned_peers.get(peer_id).map(|s| s.as_str())
    }

    /// Unbans a peer
    pub fn unban_peer(&mut self, peer_id: &str) -> bool {
        self.banned_peers.remove(peer_id).is_some()
    }

    /// Gets all banned peers
    pub fn get_banned_peers(&self) -> Vec<(&String, &String)> {
        self.banned_peers.iter().collect()
    }

    /// Gets action history
    pub fn get_action_history(&self) -> &Vec<EmergencyAction> {
        &self.action_history
    }

    /// Clears action history
    pub fn clear_history(&mut self) {
        self.action_history.clear();
    }

    /// Validates a block header against emergency rules
    pub fn validate_block_emergency(&self, height: u64, hash: &[u8; 32]) -> Result<(), EmergencyError> {
        // Check if processing is paused
        if self.processing_paused {
            return Err(EmergencyError::ProcessingPaused);
        }

        // Validate against checkpoints
        self.checkpoint_manager
            .validate_block(height, hash)
            .map_err(EmergencyError::Checkpoint)?;

        Ok(())
    }

    /// Gets emergency status summary
    pub fn get_status(&self) -> EmergencyStatus {
        EmergencyStatus {
            enabled: self.config.enabled,
            processing_paused: self.processing_paused,
            checkpoints_enabled: self.checkpoint_manager.is_enabled(),
            checkpoint_count: self.checkpoint_manager.count(),
            banned_peers_count: self.banned_peers.len(),
            action_history_count: self.action_history.len(),
            current_height: self.current_height,
        }
    }
}

/// Emergency status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyStatus {
    /// Whether emergency response is enabled
    pub enabled: bool,
    /// Whether processing is paused
    pub processing_paused: bool,
    /// Whether checkpoints are enabled
    pub checkpoints_enabled: bool,
    /// Number of checkpoints
    pub checkpoint_count: usize,
    /// Number of banned peers
    pub banned_peers_count: usize,
    /// Number of actions in history
    pub action_history_count: usize,
    /// Current blockchain height
    pub current_height: u64,
}

/// Emergency response errors
#[derive(Debug, Error)]
pub enum EmergencyError {
    /// Emergency response is disabled
    #[error("emergency response is disabled")]
    Disabled,

    /// Operator not authorized


    /// Checkpoint error
    #[error("checkpoint error: {0}")]
    Checkpoint(#[from] CheckpointError),

    /// Invalid rollback height
    #[error("invalid rollback height {height} (current: {current})")]
    InvalidRollback { height: u64, current: u64 },

    /// Processing is paused
    #[error("block processing is paused due to emergency")]
    ProcessingPaused,

    /// Invalid action
    #[error("invalid emergency action: {reason}")]
    InvalidAction { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> EmergencyConfig {
        EmergencyConfig {
            enabled: true,
            required_signatures: 1,
            response_window: 3600,
        }
    }

    #[test]
    fn test_emergency_checkpoint() {
        let mut manager = EmergencyManager::new(make_config());
        manager.update_height(2000); // High enough to allow checkpoint at 1000

        let hash = [123u8; 32];
        let result = manager.create_emergency_checkpoint(
            1000,
            hash,
            "Test emergency".to_string(),
        );

        assert!(result.is_ok());
        assert!(manager.checkpoint_manager().has_checkpoint(1000));
    }



    #[test]
    fn test_pause_processing() {
        let mut manager = EmergencyManager::new(make_config());
        
        let action = EmergencyAction::PauseProcessing;
        let result = manager.execute_action(action, "operator1");
        
        assert!(result.is_ok());
        assert!(manager.is_processing_paused());
    }

    #[test]
    fn test_ban_peers() {
        let mut manager = EmergencyManager::new(make_config());
        
        let action = EmergencyAction::BanPeers {
            peer_ids: vec!["peer1".to_string(), "peer2".to_string()],
        };
        
        manager.execute_action(action).unwrap();
        
        assert!(manager.is_peer_banned("peer1"));
        assert!(manager.is_peer_banned("peer2"));
        assert!(!manager.is_peer_banned("peer3"));
    }
}