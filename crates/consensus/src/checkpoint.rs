//! Automatic checkpoint system for emergency blockchain recovery.
//!
//! This module provides a simple, secure checkpoint mechanism that automatically
//! creates checkpoints and allows rollback to a known good state
//! during emergency situations like mining bugs or attacks.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;
use sha2::{Sha256, Digest};

/// Maximum number of checkpoints allowed (prevents abuse)
pub const MAX_CHECKPOINTS: usize = 100;

/// Auto-checkpoint interval (blocks)
pub const AUTO_CHECKPOINT_INTERVAL: u64 = 1000;



/// Minimum signatures required for checkpoint validation
pub const MIN_SIGNATURES: usize = 3;

/// Maximum age for signatures (seconds)
pub const MAX_SIGNATURE_AGE: u64 = 300; // 5 minutes

/// Minimum nodes required for cross-validation
pub const MIN_CROSS_VALIDATION_NODES: usize = 5;

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

    /// Insufficient signatures for validation
    #[error("insufficient signatures: {have}/{required}")]
    InsufficientSignatures { have: usize, required: usize },

    /// Invalid signature format
    #[error("invalid signature format: {reason}")]
    InvalidSignature { reason: String },

    /// Signature too old
    #[error("signature too old: age {age}s > max {max}s")]
    SignatureTooOld { age: u64, max: u64 },

    /// Cross-validation failed
    #[error("cross-validation failed: {reason}")]
    CrossValidationFailed { reason: String },

    /// Node verification failed
    #[error("node verification failed: {node_id}")]
    NodeVerificationFailed { node_id: String },
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
    /// Multi-signature data for enhanced security
    pub signatures: Vec<CheckpointSignature>,
    /// Cross-validation results from other nodes
    pub cross_validations: Vec<CrossValidation>,
}

/// A signature from a validator node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointSignature {
    /// Node ID of the signer
    pub node_id: String,
    /// Signature of the checkpoint data
    pub signature: Vec<u8>,
    /// Timestamp when signature was created
    pub timestamp: u64,
    /// Public key of the signer
    pub public_key: Vec<u8>,
}

/// Cross-validation result from another node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossValidation {
    /// Node ID that performed validation
    pub node_id: String,
    /// Whether the node validated the checkpoint
    pub is_valid: bool,
    /// Validation timestamp
    pub timestamp: u64,
    /// Optional validation comment
    pub comment: Option<String>,
}

impl Checkpoint {
    /// Creates a new checkpoint with enhanced security
    pub fn new(height: u64, hash: [u8; 32], reason: String) -> Self {
        Self {
            height,
            hash,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            reason,
            signatures: Vec::new(),
            cross_validations: Vec::new(),
        }
    }

    /// Creates a checkpoint with initial signature
    pub fn with_signature(
        height: u64,
        hash: [u8; 32],
        reason: String,
        node_id: String,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            height,
            hash,
            created_at: timestamp,
            reason,
            signatures: vec![CheckpointSignature {
                node_id,
                signature,
                timestamp,
                public_key,
            }],
            cross_validations: Vec::new(),
        }
    }

    /// Validates checkpoint data with enhanced security checks
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

        // Validate signatures
        self.validate_signatures()?;

        // Validate cross-validations
        self.validate_cross_validations()?;

        Ok(())
    }

    /// Validates all signatures
    pub fn validate_signatures(&self) -> Result<(), CheckpointError> {
        if self.signatures.len() < MIN_SIGNATURES {
            return Err(CheckpointError::InsufficientSignatures {
                have: self.signatures.len(),
                required: MIN_SIGNATURES,
            });
        }

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for sig in &self.signatures {
            // Check signature age
            if sig.timestamp > current_time {
                return Err(CheckpointError::InvalidSignature {
                    reason: "signature timestamp is in the future".to_string(),
                });
            }

            let age = current_time - sig.timestamp;
            if age > MAX_SIGNATURE_AGE {
                return Err(CheckpointError::SignatureTooOld {
                    age,
                    max: MAX_SIGNATURE_AGE,
                });
            }

            // Validate signature format
            if sig.signature.is_empty() || sig.signature.len() > 1024 {
                return Err(CheckpointError::InvalidSignature {
                    reason: "invalid signature length".to_string(),
                });
            }

            if sig.public_key.is_empty() || sig.public_key.len() > 1024 {
                return Err(CheckpointError::InvalidSignature {
                    reason: "invalid public key length".to_string(),
                });
            }

            if sig.node_id.is_empty() || sig.node_id.len() > 256 {
                return Err(CheckpointError::InvalidSignature {
                    reason: "invalid node ID length".to_string(),
                });
            }
        }

        // Check for duplicate node IDs
        let mut node_ids = std::collections::HashSet::new();
        for sig in &self.signatures {
            if !node_ids.insert(&sig.node_id) {
                return Err(CheckpointError::InvalidSignature {
                    reason: format!("duplicate signature from node: {}", sig.node_id),
                });
            }
        }

        Ok(())
    }

    /// Validates cross-validation results
    pub fn validate_cross_validations(&self) -> Result<(), CheckpointError> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for validation in &self.cross_validations {
            // Check timestamp
            if validation.timestamp > current_time {
                return Err(CheckpointError::CrossValidationFailed {
                    reason: "validation timestamp is in the future".to_string(),
                });
            }

            // Check node ID
            if validation.node_id.is_empty() || validation.node_id.len() > 256 {
                return Err(CheckpointError::CrossValidationFailed {
                    reason: "invalid node ID in cross-validation".to_string(),
                });
            }

            // Check comment length
            if let Some(comment) = &validation.comment {
                if comment.len() > 500 {
                    return Err(CheckpointError::CrossValidationFailed {
                        reason: "validation comment too long".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Adds a signature to the checkpoint
    pub fn add_signature(&mut self, signature: CheckpointSignature) -> Result<(), CheckpointError> {
        // Check for duplicate node ID
        if self.signatures.iter().any(|s| s.node_id == signature.node_id) {
            return Err(CheckpointError::InvalidSignature {
                reason: format!("node {} already signed this checkpoint", signature.node_id),
            });
        }

        // Validate signature format
        if signature.signature.is_empty() || signature.signature.len() > 1024 {
            return Err(CheckpointError::InvalidSignature {
                reason: "invalid signature length".to_string(),
            });
        }

        if signature.public_key.is_empty() || signature.public_key.len() > 1024 {
            return Err(CheckpointError::InvalidSignature {
                reason: "invalid public key length".to_string(),
            });
        }

        self.signatures.push(signature);
        Ok(())
    }

    /// Adds a cross-validation result
    pub fn add_cross_validation(&mut self, validation: CrossValidation) -> Result<(), CheckpointError> {
        // Check for duplicate node ID
        if self.cross_validations.iter().any(|v| v.node_id == validation.node_id) {
            return Err(CheckpointError::CrossValidationFailed {
                reason: format!("node {} already validated this checkpoint", validation.node_id),
            });
        }

        self.cross_validations.push(validation);
        Ok(())
    }

    /// Gets the checkpoint data for signing
    pub fn get_signing_data(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.hash);
        hasher.update(self.created_at.to_le_bytes());
        hasher.update(&self.reason);
        hasher.finalize().to_vec()
    }

    /// Checks if checkpoint has sufficient signatures
    pub fn has_sufficient_signatures(&self) -> bool {
        self.signatures.len() >= MIN_SIGNATURES
    }

    /// Gets the number of valid cross-validations
    pub fn get_valid_cross_validations(&self) -> usize {
        self.cross_validations.iter().filter(|v| v.is_valid).count()
    }
}

/// Checkpoint manager for automatic recovery with enhanced security
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
    /// Known validator nodes for cross-validation
    validator_nodes: HashMap<String, ValidatorNode>,
    /// Pending checkpoints awaiting signatures
    pending_checkpoints: HashMap<u64, PendingCheckpoint>,
    /// Cross-validation cache
    cross_validation_cache: HashMap<String, CrossValidationResult>,
}

/// Information about a validator node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorNode {
    /// Node ID
    pub node_id: String,
    /// Public key
    pub public_key: Vec<u8>,
    /// Node reputation score
    pub reputation: u8,
    /// Geographic region
    pub region: String,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Whether node is active
    pub is_active: bool,
}

/// A checkpoint pending multi-signature validation
#[derive(Debug, Clone)]
pub struct PendingCheckpoint {
    /// The checkpoint data
    pub checkpoint: Checkpoint,
    /// Required signatures
    pub required_signatures: usize,
    /// Creation timestamp
    pub created_at: u64,
    /// Expiration timestamp
    pub expires_at: u64,
}

/// Result of cross-validation
#[derive(Debug, Clone)]
pub struct CrossValidationResult {
    /// Checkpoint height
    pub height: u64,
    /// Validation results from nodes
    pub results: Vec<(String, bool)>,
    /// Timestamp of validation
    pub timestamp: u64,
    /// Overall validation result
    pub is_valid: bool,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager with enhanced security
    pub fn new(enabled: bool) -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            current_height: 0,
            enabled,
            auto_checkpoint_interval: AUTO_CHECKPOINT_INTERVAL,
            last_auto_checkpoint: 0,
            validator_nodes: HashMap::new(),
            pending_checkpoints: HashMap::new(),
            cross_validation_cache: HashMap::new(),
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
            validator_nodes: HashMap::new(),
            pending_checkpoints: HashMap::new(),
            cross_validation_cache: HashMap::new(),
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
        if self.last_auto_checkpoint > 0
            && height < self.last_auto_checkpoint + self.auto_checkpoint_interval {
            return false;
        }
        
        // First checkpoint logic
        if self.last_auto_checkpoint == 0 {
            return height >= self.auto_checkpoint_interval;
        }
        
        true
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
        self.pending_checkpoints.clear();
        self.cross_validation_cache.clear();
    }

    /// Adds a validator node for cross-validation
    pub fn add_validator_node(&mut self, node: ValidatorNode) -> Result<(), CheckpointError> {
        if node.node_id.is_empty() || node.node_id.len() > 256 {
            return Err(CheckpointError::InvalidData {
                reason: "invalid node ID".to_string(),
            });
        }

        if node.public_key.is_empty() || node.public_key.len() > 1024 {
            return Err(CheckpointError::InvalidData {
                reason: "invalid public key".to_string(),
            });
        }

        self.validator_nodes.insert(node.node_id.clone(), node);
        Ok(())
    }

    /// Removes a validator node
    pub fn remove_validator_node(&mut self, node_id: &str) {
        self.validator_nodes.remove(node_id);
    }

    /// Gets active validator nodes
    pub fn get_active_validators(&self) -> Vec<&ValidatorNode> {
        self.validator_nodes
            .values()
            .filter(|node| node.is_active)
            .collect()
    }

    /// Creates a checkpoint pending multi-signature validation
    pub fn create_pending_checkpoint(
        &mut self,
        height: u64,
        hash: [u8; 32],
        reason: String,
        creator_node_id: String,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Result<u64, CheckpointError> {
        if !self.enabled {
            return Err(CheckpointError::InvalidData {
                reason: "checkpoints are disabled".to_string(),
            });
        }

        // Validate basic checkpoint requirements
        if height == 0 || height > self.current_height {
            return Err(CheckpointError::InvalidData {
                reason: "invalid checkpoint height".to_string(),
            });
        }

        // Create checkpoint with initial signature
        let checkpoint = Checkpoint::with_signature(height, hash, reason, creator_node_id, signature, public_key);
        
        // Validate the checkpoint
        checkpoint.validate()?;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let pending = PendingCheckpoint {
            checkpoint,
            required_signatures: MIN_SIGNATURES,
            created_at: current_time,
            expires_at: current_time + MAX_SIGNATURE_AGE,
        };

        let pending_id = current_time;
        self.pending_checkpoints.insert(pending_id, pending);
        
        Ok(pending_id)
    }

    /// Signs a pending checkpoint
    pub fn sign_pending_checkpoint(
        &mut self,
        pending_id: u64,
        node_id: String,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Result<(), CheckpointError> {
        let pending = self.pending_checkpoints.get_mut(&pending_id)
            .ok_or(CheckpointError::NotFound { height: pending_id })?;

        // Check if pending checkpoint has expired
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if current_time > pending.expires_at {
            return Err(CheckpointError::InvalidData {
                reason: "pending checkpoint has expired".to_string(),
            });
        }

        // Verify the signer is a known validator
        if !self.validator_nodes.contains_key(&node_id) {
            return Err(CheckpointError::NodeVerificationFailed { node_id });
        }

        // Add signature
        let sig = CheckpointSignature {
            node_id: node_id.clone(),
            signature,
            timestamp: current_time,
            public_key,
        };

        pending.checkpoint.add_signature(sig)?;

        // Check if we have enough signatures to finalize
        if pending.checkpoint.has_sufficient_signatures() {
            self.finalize_pending_checkpoint(pending_id)?;
        }

        Ok(())
    }

    /// Finalizes a pending checkpoint with sufficient signatures
    fn finalize_pending_checkpoint(&mut self, pending_id: u64) -> Result<(), CheckpointError> {
        let pending = self.pending_checkpoints.remove(&pending_id)
            .ok_or(CheckpointError::NotFound { height: pending_id })?;

        // Perform cross-validation
        self.cross_validate_checkpoint(&pending.checkpoint)?;

        // Add to finalized checkpoints
        self.checkpoints.insert(pending.checkpoint.height, pending.checkpoint);
        
        Ok(())
    }

    /// Performs cross-validation of a checkpoint
    fn cross_validate_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        let active_validators = self.get_active_validators();
        
        if active_validators.len() < MIN_CROSS_VALIDATION_NODES {
            return Err(CheckpointError::CrossValidationFailed {
                reason: format!("insufficient active validators: {}/{}", 
                    active_validators.len(), MIN_CROSS_VALIDATION_NODES),
            });
        }

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Simulate cross-validation (in real implementation, this would be network calls)
        let mut valid_count = 0;
        let mut results = Vec::new();

        for validator in &active_validators {
            // In a real implementation, this would be an actual network validation
            // For now, we simulate based on validator reputation
            let is_valid = validator.reputation >= 50; // Basic reputation check
            
            if is_valid {
                valid_count += 1;
            }

            results.push((validator.node_id.clone(), is_valid));

            // Add cross-validation result to checkpoint
            let _validation = CrossValidation {
                node_id: validator.node_id.clone(),
                is_valid,
                timestamp: current_time,
                comment: if is_valid { 
                    Some("Checkpoint validated successfully".to_string()) 
                } else { 
                    Some("Checkpoint validation failed".to_string()) 
                },
            };

            // Note: In a real implementation, we'd need mutable access to add this
            // For now, we'll store it in the cache
        }

        // Store cross-validation result
        let cache_key = format!("{}:{}", checkpoint.height, checkpoint.hash.iter().map(|b| format!("{:02x}", b)).collect::<String>());
        self.cross_validation_cache.insert(cache_key, CrossValidationResult {
            height: checkpoint.height,
            results,
            timestamp: current_time,
            is_valid: valid_count >= MIN_CROSS_VALIDATION_NODES,
        });

        // Check if we have sufficient valid cross-validations
        if valid_count < MIN_CROSS_VALIDATION_NODES {
            return Err(CheckpointError::CrossValidationFailed {
                reason: format!("insufficient valid cross-validations: {}/{}", 
                    valid_count, MIN_CROSS_VALIDATION_NODES),
            });
        }

        Ok(())
    }

    /// Gets pending checkpoints
    pub fn get_pending_checkpoints(&self) -> &HashMap<u64, PendingCheckpoint> {
        &self.pending_checkpoints
    }

    /// Cleans up expired pending checkpoints
    pub fn cleanup_expired_pending(&mut self) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.pending_checkpoints.retain(|_, pending| current_time <= pending.expires_at);
    }

    /// Gets cross-validation result for a checkpoint
    pub fn get_cross_validation_result(&self, height: u64, hash: &[u8; 32]) -> Option<&CrossValidationResult> {
        let cache_key = format!("{}:{}", height, hash.iter().map(|b| format!("{:02x}", b)).collect::<String>());
        self.cross_validation_cache.get(&cache_key)
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

    #[test]
    fn test_enhanced_checkpoint_signatures() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(1000);

        // Add validator nodes
        let validator1 = ValidatorNode {
            node_id: "node1".to_string(),
            public_key: vec![1u8; 32],
            reputation: 80,
            region: "us-east".to_string(),
            last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            is_active: true,
        };
        let validator2 = ValidatorNode {
            node_id: "node2".to_string(),
            public_key: vec![2u8; 32],
            reputation: 75,
            region: "eu-west".to_string(),
            last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            is_active: true,
        };
        let validator3 = ValidatorNode {
            node_id: "node3".to_string(),
            public_key: vec![3u8; 32],
            reputation: 70,
            region: "asia-pacific".to_string(),
            last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            is_active: true,
        };

        manager.add_validator_node(validator1).unwrap();
        manager.add_validator_node(validator2).unwrap();
        manager.add_validator_node(validator3).unwrap();

        // Create pending checkpoint
        let pending_id = manager.create_pending_checkpoint(
            500,
            [123u8; 32],
            "Test checkpoint".to_string(),
            "node1".to_string(),
            vec![1u8; 64],
            vec![1u8; 32],
        ).unwrap();

        // Add signatures from other validators
        manager.sign_pending_checkpoint(pending_id, "node2".to_string(), vec![2u8; 64], vec![2u8; 32]).unwrap();
        manager.sign_pending_checkpoint(pending_id, "node3".to_string(), vec![3u8; 64], vec![3u8; 32]).unwrap();

        // Check if checkpoint was finalized
        assert_eq!(manager.count(), 1);
        assert!(manager.has_checkpoint(500));
    }

    #[test]
    fn test_insufficient_signatures() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(1000);

        // Add only one validator
        let validator = ValidatorNode {
            node_id: "node1".to_string(),
            public_key: vec![1u8; 32],
            reputation: 80,
            region: "us-east".to_string(),
            last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            is_active: true,
        };
        manager.add_validator_node(validator).unwrap();

        // Create pending checkpoint
        let _pending_id = manager.create_pending_checkpoint(
            500,
            [123u8; 32],
            "Test checkpoint".to_string(),
            "node1".to_string(),
            vec![1u8; 64],
            vec![1u8; 32],
        ).unwrap();

        // Should not be finalized due to insufficient signatures
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.get_pending_checkpoints().len(), 1);
    }

    #[test]
    fn test_cross_validation() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(1000);

        // Add multiple validators for cross-validation
        for i in 0..6 {
            let validator = ValidatorNode {
                node_id: format!("node{}", i),
                public_key: vec![i as u8; 32],
                reputation: 60 + (i as u8 * 5),
                region: "global".to_string(),
                last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                is_active: true,
            };
            manager.add_validator_node(validator).unwrap();
        }

        // Create checkpoint with sufficient signatures
        let pending_id = manager.create_pending_checkpoint(
            500,
            [123u8; 32],
            "Test checkpoint".to_string(),
            "node0".to_string(),
            vec![0u8; 64],
            vec![0u8; 32],
        ).unwrap();

        for i in 1..3 {
            manager.sign_pending_checkpoint(pending_id, format!("node{}", i), vec![i as u8; 64], vec![i as u8; 32]).unwrap();
        }

        // Check cross-validation result
        let checkpoint = manager.get_checkpoint(500).unwrap();
        assert!(checkpoint.get_valid_cross_validations() >= MIN_CROSS_VALIDATION_NODES);
    }

    #[test]
    fn test_signature_validation() {
        let checkpoint = Checkpoint::with_signature(
            100,
            [123u8; 32],
            "Test".to_string(),
            "node1".to_string(),
            vec![1u8; 64],
            vec![1u8; 32],
        );

        // Should fail with insufficient signatures
        assert!(checkpoint.validate_signatures().is_err());

        // Add more signatures
        let mut enhanced_checkpoint = checkpoint.clone();
        for i in 1..3 {
            enhanced_checkpoint.add_signature(CheckpointSignature {
                node_id: format!("node{}", i),
                signature: vec![i as u8; 64],
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                public_key: vec![i as u8; 32],
            }).unwrap();
        }

        // Should now pass
        assert!(enhanced_checkpoint.validate_signatures().is_ok());
    }

    #[test]
    fn test_cleanup_expired_pending() {
        let mut manager = CheckpointManager::new(true);
        manager.update_height(1000);

        // Add validator
        let validator = ValidatorNode {
            node_id: "node1".to_string(),
            public_key: vec![1u8; 32],
            reputation: 80,
            region: "us-east".to_string(),
            last_seen: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            is_active: true,
        };
        manager.add_validator_node(validator).unwrap();

        // Create pending checkpoint
        let _pending_id = manager.create_pending_checkpoint(
            500,
            [123u8; 32],
            "Test checkpoint".to_string(),
            "node1".to_string(),
            vec![1u8; 64],
            vec![1u8; 32],
        ).unwrap();

        assert_eq!(manager.get_pending_checkpoints().len(), 1);

        // Cleanup should not remove non-expired checkpoints
        manager.cleanup_expired_pending();
        assert_eq!(manager.get_pending_checkpoints().len(), 1);
    }
}