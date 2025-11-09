//! Automatic recovery system for blockchain anomalies
//!
//! This module provides automatic detection and recovery from blockchain
//! anomalies like mining bugs, attacks, or consensus failures.

use crate::checkpoint::{CheckpointManager, CheckpointError};
use crate::monitoring::{Monitor, MonitorEventType, AlertSeverity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Recovery configuration with security defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Whether auto-recovery is enabled
    pub enabled: bool,
    /// Maximum rollback size in blocks
    pub max_rollback_blocks: u64,
    /// Minimum confirmations before considering state safe
    pub min_confirmations: u64,
    /// Number of signatures required for manual override
    pub override_signatures: u8,
    /// Anomaly detection threshold (deviation percentage)
    pub anomaly_threshold_percent: u8,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for security
            max_rollback_blocks: 10000,
            min_confirmations: 6,
            override_signatures: 3,
            anomaly_threshold_percent: 10,
        }
    }
}

/// Recovery status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStatus {
    /// Normal operation with no anomalies detected
    Normal,
    /// Actively detecting potential anomalies
    Detecting,
    /// In the process of recovering from an anomaly
    Recovering,
    /// Requires manual intervention to proceed
    ManualIntervention,
    /// Auto-recovery system is disabled
    Disabled,
}

/// Blockchain anomaly types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Block hash does not match expected value
    BlockHashMismatch,
    /// Invalid state transition detected
    InvalidStateTransition,
    /// Consensus rules violation
    ConsensusFailure,
    /// Mining-related bug detected
    MiningBug,
    /// Network partition detected
    NetworkPartition,
    /// Other type of anomaly with custom description
    Other(String),
}

/// Anomaly detection result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnomalyDetection {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Block height where anomaly was detected
    pub height: u64,
    /// Expected block hash
    pub expected_hash: [u8; 32],
    /// Actual block hash
    pub actual_hash: [u8; 32],
    /// Detection timestamp
    pub timestamp: u64,
    /// Additional context
    pub context: String,
}

/// Blockchain state snapshot
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: [u8; 32],
    /// State root hash
    pub state_root: [u8; 32],
    /// Timestamp when snapshot was created
    pub timestamp: u64,
    /// Whether state is verified as safe
    pub verified_safe: bool,
}

/// Auto-recovery errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AutoRecoveryError {
    #[error("recovery is disabled")]
    /// Recovery system is disabled
    Disabled,

    #[error("invalid configuration: {reason}")]
    /// Invalid configuration provided
    InvalidConfig { 
        /// Reason for invalid configuration
        reason: String 
    },

    #[error("anomaly detected at height {height}: {anomaly_type:?}")]
    /// Anomaly detected at specific block height
    AnomalyDetected { 
        /// Block height where anomaly was detected
        height: u64, 
        /// Type of anomaly detected
        anomaly_type: AnomalyType 
    },

    #[error("target height {height} not found")]
    /// Target rollback height not found in snapshots
    TargetNotFound { 
        /// Target height that was not found
        height: u64 
    },

    #[error("rollback too large: {blocks} blocks (max: {max})")]
    /// Rollback size exceeds maximum allowed
    RollbackTooLarge { 
        /// Number of blocks to rollback
        blocks: u64, 
        /// Maximum allowed rollback size
        max: u64 
    },

    #[error("insufficient signatures: {have}/{required}")]
    /// Insufficient signatures for manual override
    InsufficientSignatures { 
        /// Number of signatures provided
        have: u8, 
        /// Number of signatures required
        required: u8 
    },

    #[error("checkpoint error: {0}")]
    /// Checkpoint-related error
    CheckpointError(#[from] CheckpointError),

    #[error("invalid state: {reason}")]
    /// Invalid blockchain state
    InvalidState { 
        /// Reason for invalid state
        reason: String 
    },

    #[error("operation not allowed in current status: {status:?}")]
    /// Operation not allowed in current recovery status
    InvalidStatus { 
        /// Current recovery status
        status: RecoveryStatus 
    },
}

/// Automatic recovery manager
#[derive(Debug, Clone)]
pub struct AutoRecoveryManager {
    /// Configuration
    config: RecoveryConfig,
    /// Current recovery status
    status: RecoveryStatus,
    /// Checkpoint manager for rollback targets
    checkpoint_manager: CheckpointManager,
    /// Monitoring system for alerts
    monitor: Monitor,
    /// State snapshots by height
    snapshots: BTreeMap<u64, StateSnapshot>,
    /// Last known safe height
    last_safe_height: u64,
    /// Last known safe hash
    last_safe_hash: [u8; 32],
    /// Manual override signatures
    override_signatures: HashMap<String, String>,
    /// Recovery history
    recovery_history: Vec<AnomalyDetection>,
}

impl AutoRecoveryManager {
    /// Creates a new auto-recovery manager
    pub fn new(
        config: RecoveryConfig,
        checkpoint_manager: CheckpointManager,
        monitor: Monitor,
    ) -> Self {
        Self {
            config,
            status: RecoveryStatus::Normal,
            checkpoint_manager,
            monitor,
            snapshots: BTreeMap::new(),
            last_safe_height: 0,
            last_safe_hash: [0u8; 32],
            override_signatures: HashMap::new(),
            recovery_history: Vec::new(),
        }
    }

    /// Gets current recovery status
    pub fn status(&self) -> RecoveryStatus {
        self.status.clone()
    }

    /// Enables or disables auto-recovery
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        if !enabled {
            self.status = RecoveryStatus::Disabled;
        } else if self.status == RecoveryStatus::Disabled {
            self.status = RecoveryStatus::Normal;
        }
    }

    /// Updates configuration with validation
    pub fn update_config(&mut self, config: RecoveryConfig) -> Result<(), AutoRecoveryError> {
        // Validate configuration
        if config.max_rollback_blocks == 0 || config.max_rollback_blocks > 100000 {
            return Err(AutoRecoveryError::InvalidConfig {
                reason: "Invalid max_rollback_blocks".to_string(),
            });
        }

        if config.min_confirmations == 0 || config.min_confirmations > 100 {
            return Err(AutoRecoveryError::InvalidConfig {
                reason: "Invalid min_confirmations".to_string(),
            });
        }

        if config.override_signatures == 0 || config.override_signatures > 10 {
            return Err(AutoRecoveryError::InvalidConfig {
                reason: "Invalid override_signatures".to_string(),
            });
        }

        self.config = config;
        Ok(())
    }

    /// Processes a new block for anomaly detection
    pub fn process_block(
        &mut self,
        height: u64,
        hash: [u8; 32],
        state_root: [u8; 32],
    ) -> Result<(), AutoRecoveryError> {
        // Security: Skip if disabled
        if !self.config.enabled {
            return Ok(());
        }

        // Security: Only process in normal status
        if self.status != RecoveryStatus::Normal {
            return Err(AutoRecoveryError::InvalidStatus {
                status: self.status.clone(),
            });
        }

        // Create state snapshot
        let snapshot = StateSnapshot {
            height,
            hash,
            state_root,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            verified_safe: false,
        };

        // Store snapshot
        self.snapshots.insert(height, snapshot);

        // Validate against checkpoints
        if let Err(e) = self.checkpoint_manager.validate_block(height, &hash) {
            let anomaly = AnomalyDetection {
                anomaly_type: AnomalyType::BlockHashMismatch,
                height,
                expected_hash: self.last_safe_hash,
                actual_hash: hash,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                context: format!("Checkpoint validation failed: {}", e),
            };

            return self.handle_anomaly(anomaly);
        }

        // Update safe state after sufficient confirmations
        if height > self.last_safe_height + self.config.min_confirmations {
            self.last_safe_height = height - self.config.min_confirmations;
            self.last_safe_hash = hash;

            // Mark snapshots as safe
            if let Some(snapshot) = self.snapshots.get_mut(&self.last_safe_height) {
                snapshot.verified_safe = true;
            }
        }

        // Cleanup old snapshots
        self.cleanup_old_snapshots();

        Ok(())
    }

    /// Handles detected anomaly
    fn handle_anomaly(&mut self, anomaly: AnomalyDetection) -> Result<(), AutoRecoveryError> {
        self.status = RecoveryStatus::Detecting;

        // Add to recovery history
        self.recovery_history.push(anomaly.clone());

        // Send alert
        let _ = self.monitor.create_alert(
            AlertSeverity::Critical,
            MonitorEventType::AnomalyDetected,
            format!("Anomaly detected: {:?}", anomaly.anomaly_type),
            Some(anomaly.height),
        );

        // Attempt automatic recovery
        if self.can_auto_recover(&anomaly) {
            self.status = RecoveryStatus::Recovering;
            return self.execute_auto_rollback(&anomaly);
        }

        // Require manual intervention
        self.status = RecoveryStatus::ManualIntervention;
        Err(AutoRecoveryError::AnomalyDetected {
            height: anomaly.height,
            anomaly_type: anomaly.anomaly_type,
        })
    }

    /// Checks if automatic recovery is possible
    fn can_auto_recover(&self, anomaly: &AnomalyDetection) -> bool {
        // Check rollback size
        let rollback_size = anomaly.height.saturating_sub(self.last_safe_height);
        if rollback_size > self.config.max_rollback_blocks {
            return false;
        }

        // Check if we have a safe checkpoint
        if self.last_safe_height == 0 {
            return false;
        }

        true
    }

    /// Executes automatic rollback
    fn execute_auto_rollback(&mut self, anomaly: &AnomalyDetection) -> Result<(), AutoRecoveryError> {
        // Calculate rollback size
        let rollback_size = anomaly.height.saturating_sub(self.last_safe_height);
        if rollback_size > self.config.max_rollback_blocks {
            return Err(AutoRecoveryError::RollbackTooLarge {
                blocks: rollback_size,
                max: self.config.max_rollback_blocks,
            });
        }

        // Perform rollback
        self.rollback_to(self.last_safe_height)?;

        // Send recovery alert
        let _ = self.monitor.create_alert(
            AlertSeverity::Warning,
            MonitorEventType::SystemError,
            format!("Auto-recovery completed: rolled back {} blocks", rollback_size),
            Some(self.last_safe_height),
        );

        // Return to normal status
        self.status = RecoveryStatus::Normal;

        Ok(())
    }

    /// Manual override for canceling auto-rollback
    pub fn manual_override(
        &mut self,
        signature: &str,
        reason: &str,
    ) -> Result<(), AutoRecoveryError> {
        // Add signature
        self.override_signatures.insert("manual_override".to_string(), signature.to_string());

        // Check if we have enough signatures
        if self.override_signatures.len() >= self.config.override_signatures as usize {
            self.status = RecoveryStatus::ManualIntervention;

            // Send alert
            let _ = self.monitor.create_alert(
                AlertSeverity::Warning,
                MonitorEventType::SecurityViolation,
                format!("Manual override triggered: {}", reason),
                None,
            );
        }

        Ok(())
    }

    /// Executes manual rollback to specified height
    pub fn manual_rollback(
        &mut self,
        target_height: u64,
        signatures: Vec<String>,
    ) -> Result<(), AutoRecoveryError> {
        // Verify signatures
        if signatures.len() < self.config.override_signatures as usize {
            return Err(AutoRecoveryError::InsufficientSignatures {
                have: signatures.len() as u8,
                required: self.config.override_signatures,
            });
        }

        // Check if target height exists
        if !self.snapshots.contains_key(&target_height) {
            return Err(AutoRecoveryError::TargetNotFound { height: target_height });
        }

        // Perform rollback
        self.rollback_to(target_height)?;

        // Send alert
        let _ = self.monitor.create_alert(
            AlertSeverity::Warning,
            MonitorEventType::SystemError,
            format!("Manual rollback to height {} completed", target_height),
            Some(target_height),
        );

        Ok(())
    }

    /// Rolls back to specified height
    fn rollback_to(&mut self, target_height: u64) -> Result<(), AutoRecoveryError> {
        // Remove snapshots above target height
        self.snapshots.split_off(&(target_height + 1));

        // Update checkpoint manager
        self.checkpoint_manager.rollback_to(target_height);

        // Update safe state
        if let Some(snapshot) = self.snapshots.get(&target_height) {
            self.last_safe_height = target_height;
            self.last_safe_hash = snapshot.hash;
        }

        Ok(())
    }

    /// Cleans up old snapshots
    fn cleanup_old_snapshots(&mut self) {
        let cutoff_height = self.last_safe_height.saturating_sub(self.config.max_rollback_blocks);
        self.snapshots.split_off(&cutoff_height);
    }

    /// Gets recovery statistics
    pub fn get_recovery_stats(&self) -> RecoveryStats {
        RecoveryStats {
            status: self.status.clone(),
            last_safe_height: self.last_safe_height,
            snapshot_count: self.snapshots.len(),
            recovery_count: self.recovery_history.len(),
            override_signatures: self.override_signatures.len() as u8,
        }
    }

    /// Gets recovery history
    pub fn get_recovery_history(&self) -> &[AnomalyDetection] {
        &self.recovery_history
    }

    /// Clears recovery history
    pub fn clear_history(&mut self) {
        self.recovery_history.clear();
    }
}

/// Recovery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    /// Current recovery status
    pub status: RecoveryStatus,
    /// Last known safe block height
    pub last_safe_height: u64,
    /// Number of stored snapshots
    pub snapshot_count: usize,
    /// Number of recovery operations performed
    pub recovery_count: usize,
    /// Number of override signatures collected
    pub override_signatures: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::Monitor;

    fn create_test_manager() -> AutoRecoveryManager {
        let config = RecoveryConfig {
            enabled: true,
            max_rollback_blocks: 1000,
            min_confirmations: 6,
            override_signatures: 3,
            anomaly_threshold_percent: 10,
        };

        let checkpoint_manager = CheckpointManager::new(true);
        let monitor = Monitor::default();

        AutoRecoveryManager::new(config, checkpoint_manager, monitor)
    }

    #[test]
    fn test_auto_recovery_disabled() {
        let mut manager = create_test_manager();
        manager.set_enabled(false);

        let result = manager.process_block(100, [1u8; 32], [2u8; 32]);
        assert!(result.is_ok()); // Should succeed without processing
    }

    #[test]
    fn test_normal_block_processing() {
        let mut manager = create_test_manager();

        // Process normal blocks
        for i in 1..=10 {
            let hash = [i as u8; 32];
            let state_root = [(i + 100) as u8; 32];
            let result = manager.process_block(i, hash, state_root);
            assert!(result.is_ok());
        }

        let stats = manager.get_recovery_stats();
        assert_eq!(stats.status, RecoveryStatus::Normal);
        assert_eq!(stats.snapshot_count, 10);
    }

    #[test]
    fn test_manual_override() {
        let mut manager = create_test_manager();

        // Add signatures
        let result = manager.manual_override("sig1", "Test override");
        assert!(result.is_ok());

        let result = manager.manual_override("sig2", "Test override");
        assert!(result.is_ok());

        let result = manager.manual_override("sig3", "Test override");
        assert!(result.is_ok());

        let stats = manager.get_recovery_stats();
        assert_eq!(stats.override_signatures, 3);
    }

    #[test]
    fn test_config_validation() {
        let mut manager = create_test_manager();

        // Invalid config
        let invalid_config = RecoveryConfig {
            enabled: true,
            max_rollback_blocks: 0, // Invalid
            min_confirmations: 6,
            override_signatures: 3,
            anomaly_threshold_percent: 10,
        };

        let result = manager.update_config(invalid_config);
        assert!(result.is_err());
    }
}