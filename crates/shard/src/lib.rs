//! BitQuan Sharding Module
//!
//! This module provides horizontal scaling capabilities through blockchain sharding.
//! It enables parallel processing of transactions across multiple shards while maintaining
//! security through cross-shard communication protocols.

pub mod shard_manager;
pub mod state_partition;
pub mod cross_shard;
pub mod consensus_coordinator;
pub mod network_shard;

pub use shard_manager::{ShardManager, ShardConfig, ShardResult};
pub use state_partition::{StatePartitioner, StateColumn};
pub use cross_shard::{CrossShardComms, CrossShardMessage, CrossShardResponse};
pub use consensus_coordinator::{ConsensusCoordinator, ShardConsensus};
pub use network_shard::{ShardNetwork, ShardPeer};

/// Configuration for sharding
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// Total number of shards in the network
    pub total_shards: u16,
    /// Current shard ID (0 to total_shards-1)
    pub local_shard_id: u16,
    /// Minimum number of validators per shard
    pub min_validators: usize,
    /// Maximum number of validators per shard
    pub max_validators: usize,
    /// Cross-shard message timeout
    pub cross_shard_timeout: std::time::Duration,
    /// State partitioning strategy
    pub partitioning: PartitioningStrategy,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            total_shards: 4,
            local_shard_id: 0,
            min_validators: 4,
            max_validators: 16,
            cross_shard_timeout: std::time::Duration::from_secs(30),
            partitioning: PartitioningStrategy::Hash,
        }
    }
}

/// State partitioning strategies
#[derive(Debug, Clone, Copy)]
pub enum PartitioningStrategy {
    /// Hash-based partitioning (default)
    Hash,
    /// Range-based partitioning
    Range,
    /// Consistent hashing
    Consistent,
}

/// Result type for shard operations
#[derive(Debug)]
pub enum ShardResult<T> {
    /// Operation completed successfully on local shard
    Local(T),
    /// Operation requires cross-shard communication
    CrossShard(CrossShardOperation),
    /// Operation failed
    Error(ShardError),
}

/// Cross-shard operation details
#[derive(Debug)]
pub struct CrossShardOperation {
    pub target_shard: u16,
    pub operation_id: [u8; 32],
    pub data: Vec<u8>,
    pub timeout: std::time::Duration,
}

/// Shard-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ShardError {
    #[error("Invalid shard ID: {0}")]
    InvalidShardId(u16),
    #[error("Cross-shard communication timeout")]
    CrossShardTimeout,
    #[error("State access denied for key: {0}")]
    StateAccessDenied(String),
    #[error("Consensus error: {0}")]
    ConsensusError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl<T> From<ShardError> for ShardResult<T> {
    fn from(err: ShardError) -> Self {
        ShardResult::Error(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_config_default() {
        let config = ShardConfig::default();
        assert_eq!(config.total_shards, 4);
        assert_eq!(config.local_shard_id, 0);
        assert_eq!(config.min_validators, 4);
        assert_eq!(config.max_validators, 16);
    }

    #[test]
    fn test_shard_id_validation() {
        let mut config = ShardConfig::default();
        assert!(config.validate_shard_id().is_ok());

        config.local_shard_id = 4;
        assert!(config.validate_shard_id().is_err());
    }

    impl ShardConfig {
        pub fn validate_shard_id(&self) -> Result<(), ShardError> {
            if self.local_shard_id >= self.total_shards {
                Err(ShardError::InvalidShardId(self.local_shard_id))
            } else {
                Ok(())
            }
        }
    }
}