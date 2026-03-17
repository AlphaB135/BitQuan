//! Shard Manager - Core orchestrator for horizontal scaling

use crate::{ShardConfig, ShardResult, ShardError, PartitioningStrategy};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use bitquan_types::{Transaction, Block, NetworkId};
use crate::cross_shard::CrossShardComms;
use crate::state_partition::StatePartitioner;

/// Orchestrates shard operations and coordinates cross-shard communication
pub struct ShardManager {
    config: ShardConfig,
    state_partitioner: StatePartitioner,
    cross_shard_comms: Arc<CrossShardComms>,
    local_state: Arc<RwLock<LocalState>>,
    network_id: NetworkId,
    genesis_hash: [u8; 32],
}

/// Local state maintained by each shard
#[derive(Debug, Default)]
pub struct LocalState {
    pub current_height: u64,
    pub pending_transactions: HashMap<[u8; 32], Transaction>,
    pub cross_shard_operations: HashMap<[u8; 32], CrossShardOperation>,
    pub validator_set: Vec<Validator>,
}

/// Validator information
#[derive(Debug, Clone)]
pub struct Validator {
    pub address: [u8; 32],
    pub public_key: [u8; 32],
    pub is_active: bool,
    pub reputation_score: f64,
}

impl ShardManager {
    /// Create a new shard manager
    pub fn new(
        config: ShardConfig,
        network_id: NetworkId,
        genesis_hash: [u8; 32],
    ) -> Result<Self, ShardError> {
        config.validate_shard_id()?;

        let state_partitioner = StatePartitioner::new(config.local_shard_id, config.total_shards);
        let cross_shard_comms = Arc::new(CrossShardComms::new(
            config.local_shard_id,
            config.total_shards,
            config.cross_shard_timeout,
        ));

        Ok(Self {
            config,
            state_partitioner,
            cross_shard_comms,
            local_state: Arc::new(RwLock::new(LocalState::default())),
            network_id,
            genesis_hash,
        })
    }

    /// Process a transaction - routes to appropriate shard
    pub async fn process_transaction(&self, tx: Transaction) -> ShardResult<TransactionResult> {
        let target_shard = self.route_transaction(&tx);

        if target_shard == self.config.local_shard_id {
            // Local processing
            self.process_local_transaction(tx).await
        } else {
            // Cross-shard processing
            let op_id = self.generate_operation_id();
            let cross_shard_op = CrossShardOperation {
                target_shard,
                operation_id: op_id,
                data: bincode::serialize(&tx).unwrap_or_default(),
                timeout: self.config.cross_shard_timeout,
            };

            self.cross_shard_comms
                .send_transaction(tx, target_shard, op_id)
                .await?;

            ShardResult::CrossShard(cross_shard_op)
        }
    }

    /// Route transaction to appropriate shard
    pub fn route_transaction(&self, tx: &Transaction) -> u16 {
        match self.config.partitioning {
            PartitioningStrategy::Hash => self.route_by_hash(tx),
            PartitioningStrategy::Range => self.route_by_range(tx),
            PartitioningStrategy::Consistent => self.route_by_consistent_hash(tx),
        }
    }

    /// Route by hash of sender address
    fn route_by_hash(&self, tx: &Transaction) -> u16 {
        let hash = blake3::hash(&tx.sender);
        let hash_bytes = hash.as_bytes();
        // Use first 2 bytes to determine shard (0-65535)
        let shard_value = u16::from_be_bytes([hash_bytes[0], hash_bytes[1]]);
        shard_value % self.config.total_shards
    }

    /// Route by address range
    fn route_by_range(&self, tx: &Transaction) -> u16 {
        // First byte determines shard
        let shard_value = tx.sender[0] as u16;
        shard_value % self.config.total_shards
    }

    /// Route by consistent hashing
    fn route_by_consistent_hash(&self, tx: &Transaction) -> u16 {
        // Simplified consistent hashing
        let hash = blake3::hash(&tx.sender);
        let shard_value = (hash.as_bytes()[0] as u16) * 256 + (hash.as_bytes()[1] as u16);
        shard_value % self.config.total_shards
    }

    /// Process transaction locally
    async fn process_local_transaction(&self, tx: Transaction) -> ShardResult<TransactionResult> {
        // Validate transaction
        if !self.validate_transaction(&tx) {
            return Err(ShardError::ConsensusError("Invalid transaction".into()));
        }

        // Add to pending transactions
        {
            let mut state = self.local_state.write().await;
            state.pending_transactions.insert(tx.txid(), tx.clone());
        }

        // Process through local consensus
        let result = self.process_through_consensus(tx).await?;

        ShardResult::Local(TransactionResult {
            success: true,
            tx_hash: tx.txid(),
            local_state_update: Some(result),
            cross_shard_updates: vec![],
        })
    }

    /// Process through local consensus
    async fn process_through_consensus(&self, tx: Transaction) -> ConsensusResult {
        // This would integrate with the BitQuan consensus engine
        // For now, simulate a simple processing result
        ConsensusResult {
            state_root: [0u8; 32], // Would be actual state root
            gas_used: 21000,
            status: "success".into(),
            logs: vec![],
        }
    }

    /// Generate operation ID for cross-shard operations
    fn generate_operation_id(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&self.config.local_shard_id.to_le_bytes());
        // Fill rest with random data
        for i in 4..32 {
            bytes[i] = (i * 137) as u8;
        }
        bytes
    }

    /// Validate transaction before processing
    fn validate_transaction(&self, tx: &Transaction) -> bool {
        // Basic validation - would be more comprehensive in production
        tx.inputs.len() > 0 &&
        tx.outputs.len() > 0 &&
        tx.lock_time >= 0
    }

    /// Get current shard statistics
    pub async fn get_stats(&self) -> ShardStats {
        let state = self.local_state.read().await;
        ShardStats {
            shard_id: self.config.local_shard_id,
            total_shards: self.config.total_shards,
            current_height: state.current_height,
            pending_transactions: state.pending_transactions.len(),
            active_validators: state.validator_set.len(),
            cross_shard_operations: state.cross_shard_operations.len(),
        }
    }
}

/// Result of transaction processing
#[derive(Debug)]
pub struct TransactionResult {
    pub success: bool,
    pub tx_hash: [u8; 32],
    pub local_state_update: Option<ConsensusResult>,
    pub cross_shard_updates: Vec<CrossShardUpdate>,
}

/// Cross-shard update notification
#[derive(Debug, Clone)]
pub struct CrossShardUpdate {
    pub from_shard: u16,
    pub operation_id: [u8; 32],
    pub status: String,
    pub new_state_root: [u8; 32],
}

/// Consensus processing result
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub state_root: [u8; 32],
    pub gas_used: u64,
    pub status: String,
    pub logs: Vec<String>,
}

/// Shard statistics
#[derive(Debug, Clone)]
pub struct ShardStats {
    pub shard_id: u16,
    pub total_shards: u16,
    pub current_height: u64,
    pub pending_transactions: usize,
    pub active_validators: usize,
    pub cross_shard_operations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_routing() {
        let config = ShardConfig {
            total_shards: 4,
            local_shard_id: 0,
            ..Default::default()
        };

        let manager = ShardManager::new(
            config,
            NetworkId::Devnet,
            [0u8; 32]
        ).unwrap();

        // Test routing by hash
        let mut tx = Transaction::default();
        tx.sender = [0x12, 0x34, 0x56, 0x78, 0x00, 0x00, 0x00, 0x00];
        let shard = manager.route_transaction(&tx);
        assert!(shard < 4);

        // Test routing by range
        let mut tx2 = Transaction::default();
        tx2.sender = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let shard2 = manager.route_transaction(&tx2);
        assert_eq!(shard2, 1);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ShardConfig::default();
        assert!(config.validate_shard_id().is_ok());

        config.local_shard_id = 4;
        assert!(config.validate_shard_id().is_err());
    }
}