//! Cross-Shard Communication - Handles inter-shard communication

use crate::{ShardError, ShardResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use std::time::Duration;
use bitquan_types::{Transaction, Block};
use uuid::Uuid;

/// Manages cross-shard communication
pub struct CrossShardComms {
    local_shard_id: u16,
    total_shards: u16,
    timeout: Duration,

    // Message queues
    outgoing_queue: Arc<RwLock<HashMap<u16, mpsc::Sender<CrossShardMessage>>>>,
    incoming_queue: Arc<RwLock<Vec<CrossShardMessage>>>,
    response_channel: Arc<RwLock<HashMap<[u8; 32], mpsc::Sender<CrossShardResponse>>>>,

    // Network layer
    network: Arc<dyn CrossShardNetwork>,

    // State tracking
    pending_operations: Arc<RwLock<HashMap<[u8; 32], PendingOperation>>>,
}

/// Cross-shard message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossShardMessage {
    /// Transaction request to another shard
    TransactionRequest {
        tx: Transaction,
        source_shard: u16,
        target_shard: u16,
        operation_id: [u8; 32],
        nonce: u64,
    },
    /// State query from another shard
    StateQuery {
        key: Vec<u8>,
        source_shard: u16,
        target_shard: u16,
        operation_id: [u8; 32],
        require_proof: bool,
    },
    /// State response
    StateResponse {
        key: Vec<u8>,
        value: Option<Vec<u8>>,
        proof: Option<StateProof>,
        source_shard: u16,
        operation_id: [u8; 32],
    },
    /// Cross-shard block commit
    BlockCommit {
        block: Block,
        source_shard: u16,
        target_shard: u16,
        operation_id: [u8; 32],
        state_root: [u8; 32],
    },
    /// Finalization request
    FinalizationRequest {
        tx_hash: [u8; 32],
        source_shard: u16,
        target_shard: u16,
        operation_id: [u8; 32],
    },
}

/// Cross-shard response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossShardResponse {
    /// Transaction accepted
    TransactionAccepted {
        operation_id: [u8; 32],
        state_root: [u8; 32],
    },
    /// State retrieved successfully
    StateRetrieved {
        operation_id: [u8; 32],
        value: Vec<u8>,
    },
    /// Transaction rejected
    TransactionRejected {
        operation_id: [u8; 32],
        reason: String,
    },
    /// State not found
    StateNotFound {
        operation_id: [u8; 32],
    },
    /// Block committed
    BlockCommitted {
        operation_id: [u8; 32],
        new_height: u64,
    },
    /// Finalized
    Finalized {
        operation_id: [u8; 32],
        status: String,
    },
}

/// State proof for cross-shard verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProof {
    pub proof_type: ProofType,
    pub proof_data: Vec<u8>,
    pub state_root: [u8; 32],
    pub signature: [u8; 64],
}

/// Types of state proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    MerkleProof,
    RangeProof,
    SparseMerkleProof,
}

/// Pending operation tracking
#[derive(Debug, Clone)]
pub struct PendingOperation {
    pub operation_id: [u8; 32],
    pub source_shard: u16,
    pub target_shard: u16,
    pub created_at: Duration,
    pub timeout: Duration,
    pub retry_count: u8,
    pub last_error: Option<String>,
}

/// Network interface for cross-shard communication
#[async_trait]
pub trait CrossShardNetwork: Send + Sync {
    async fn send_message(&self, shard: u16, message: CrossShardMessage) -> Result<(), ShardError>;
    async fn receive_message(&self) -> Result<CrossShardMessage, ShardError>;
    async fn get_peer_addresses(&self, shard: u16) -> Vec<String>;
}

impl CrossShardComms {
    /// Create new cross-shard communication manager
    pub fn new(
        local_shard_id: u16,
        total_shards: u16,
        timeout: Duration,
    ) -> Self {
        Self {
            local_shard_id,
            total_shards,
            timeout,
            outgoing_queue: Arc::new(RwLock::new(HashMap::new())),
            incoming_queue: Arc::new(RwLock::new(Vec::new())),
            response_channel: Arc::new(RwLock::new(HashMap::new())),
            network: Arc::new(MockCrossShardNetwork::new()),
            pending_operations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set network implementation
    pub fn with_network<N: CrossShardNetwork + 'static>(self, network: N) -> Self {
        Self {
            network: Arc::new(network),
            ..self
        }
    }

    /// Send a transaction to another shard
    pub async fn send_transaction(
        &self,
        tx: Transaction,
        target_shard: u16,
        operation_id: [u8; 32],
    ) -> Result<(), ShardError> {
        if target_shard == self.local_shard_id {
            return Err(ShardError::InvalidShardId(target_shard));
        }

        let message = CrossShardMessage::TransactionRequest {
            tx: tx.clone(),
            source_shard: self.local_shard_id,
            target_shard,
            operation_id,
            nonce: self.generate_nonce(),
        };

        self.send_message_to_shard(target_shard, message).await
    }

    /// Query state from another shard
    pub async fn query_state(
        &self,
        key: Vec<u8>,
        target_shard: u16,
        require_proof: bool,
    ) -> Result<CrossShardResponse, ShardError> {
        let operation_id = self.generate_operation_id();

        // Store response channel
        let (tx, mut rx) = mpsc::channel(1);
        {
            let mut channels = self.response_channel.write().await;
            channels.insert(operation_id, tx);
        }

        let message = CrossShardMessage::StateQuery {
            key,
            source_shard: self.local_shard_id,
            target_shard,
            operation_id,
            require_proof,
        };

        self.send_message_to_shard(target_shard, message).await?;

        // Wait for response with timeout
        match tokio::time::timeout(self.timeout, rx.recv()).await {
            Ok(Some(response)) => Ok(response),
            Ok(None) => Err(ShardError::CrossShardTimeout),
            Err(_) => Err(ShardError::CrossShardTimeout),
        }
    }

    /// Commit a block to another shard
    pub async fn commit_block(
        &self,
        block: Block,
        target_shard: u16,
        state_root: [u8; 32],
    ) -> Result<CrossShardResponse, ShardError> {
        let operation_id = self.generate_operation_id();

        let message = CrossShardMessage::BlockCommit {
            block: block.clone(),
            source_shard: self.local_shard_id,
            target_shard,
            operation_id,
            state_root,
        };

        self.send_message_to_shard(target_shard, message).await?;

        // Wait for response
        tokio::time::sleep(self.timeout).await;
        Err(ShardError::CrossShardTimeout)
    }

    /// Send a message to a specific shard
    async fn send_message_to_shard(
        &self,
        shard_id: u16,
        message: CrossShardMessage,
    ) -> Result<(), ShardError> {
        let operation_id = self.extract_operation_id(&message);

        // Track pending operation
        let pending_op = PendingOperation {
            operation_id,
            source_shard: self.local_shard_id,
            target_shard: shard_id,
            created_at: Duration::from_secs(0), // Would be actual timestamp
            timeout: self.timeout,
            retry_count: 0,
            last_error: None,
        };

        {
            let mut pending = self.pending_operations.write().await;
            pending.insert(operation_id, pending_op);
        }

        // Send message via network
        self.network.send_message(shard_id, message).await?;

        Ok(())
    }

    /// Handle incoming message
    pub async fn handle_incoming_message(&self, message: CrossShardMessage) {
        match message {
            CrossShardMessage::TransactionRequest { tx, operation_id, .. } => {
                self.handle_transaction_request(tx, operation_id).await;
            }
            CrossShardMessage::StateQuery { key, operation_id, .. } => {
                self.handle_state_query(key, operation_id).await;
            }
            CrossShardMessage::StateResponse { operation_id, .. } => {
                self.handle_state_response(operation_id).await;
            }
            CrossShardMessage::BlockCommit { block, operation_id, .. } => {
                self.handle_block_commit(block, operation_id).await;
            }
            CrossShardMessage::FinalizationRequest { operation_id, .. } => {
                self.handle_finalization_request(operation_id).await;
            }
        }
    }

    /// Handle transaction request from another shard
    async fn handle_transaction_request(&self, tx: Transaction, operation_id: [u8; 32]) {
        // In a real implementation, this would:
        // 1. Validate the transaction
        // 2. Process it through local consensus
        // 3. Send back response

        // For now, just send a mock acceptance
        let response = CrossShardResponse::TransactionAccepted {
            operation_id,
            state_root: [0u8; 32],
        };

        self.send_response(operation_id, response).await;
    }

    /// Handle state query from another shard
    async fn handle_state_query(&self, key: Vec<u8>, operation_id: [u8; 32]) {
        // In a real implementation, this would:
        // 1. Look up the state key
        // 2. Generate proof if required
        // 3. Send back response

        let value = Some(b"mock_value".to_vec());
        let proof = Some(StateProof {
            proof_type: ProofType::MerkleProof,
            proof_data: vec![],
            state_root: [0u8; 32],
            signature: [0u8; 64],
        });

        let response = CrossShardResponse::StateRetrieved {
            operation_id,
            value: value.unwrap(),
        };

        self.send_response(operation_id, response).await;
    }

    /// Handle state response
    async fn handle_state_response(&self, operation_id: [u8; 32]) {
        // Forward response to waiting caller
        if let Some(mut tx) = {
            let channels = self.response_channel.read().await;
            channels.get(&operation_id).cloned()
        } {
            let _ = tx.send(CrossShardResponse::StateRetrieved {
                operation_id,
                value: vec![],
            }).await;
        }
    }

    /// Handle block commit
    async fn handle_block_commit(&self, block: Block, operation_id: [u8; 32]) {
        // In a real implementation, this would:
        // 1. Verify the block
        // 2. Add to local blockchain
        // 3. Notify other shards

        // For now, just acknowledge
        let response = CrossShardResponse::BlockCommitted {
            operation_id,
            new_height: 100, // Would be actual new height
        };

        self.send_response(operation_id, response).await;
    }

    /// Handle finalization request
    async fn handle_finalization_request(&self, operation_id: [u8; 32]) {
        let response = CrossShardResponse::Finalized {
            operation_id,
            status: "success".to_string(),
        };

        self.send_response(operation_id, response).await;
    }

    /// Send response to operation ID
    async fn send_response(&self, operation_id: [u8; 32], response: CrossShardResponse) {
        if let Some(mut tx) = {
            let channels = self.response_channel.write().await;
            channels.remove(&operation_id)
        } {
            let _ = tx.send(response).await;
        }
    }

    /// Generate operation ID
    fn generate_operation_id(&self) -> [u8; 32] {
        let mut uuid = Uuid::new_v4();
        let mut bytes = [0u8; 32];
        uuid.as_bytes().copy_into_slice(&mut bytes);
        bytes
    }

    /// Generate nonce
    fn generate_nonce(&self) -> u64 {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Extract operation ID from message
    fn extract_operation_id(&self, message: &CrossShardMessage) -> [u8; 32] {
        match message {
            CrossShardMessage::TransactionRequest { operation_id, .. } => *operation_id,
            CrossShardMessage::StateQuery { operation_id, .. } => *operation_id,
            CrossShardMessage::StateResponse { operation_id, .. } => *operation_id,
            CrossShardMessage::BlockCommit { operation_id, .. } => *operation_id,
            CrossShardMessage::FinalizationRequest { operation_id, .. } => *operation_id,
        }
    }

    /// Check for timed out operations
    pub async fn check_timeouts(&self) -> Vec<[u8; 32]> {
        let mut timed_out = Vec::new();
        let now = Duration::from_secs(0); // Would be actual time

        {
            let mut pending = self.pending_operations.write().await;
            for (op_id, op) in pending.iter_mut() {
                if now.duration_since(op.created_at) > op.timeout {
                    timed_out.push(*op_id);
                    op.retry_count += 1;
                    op.last_error = Some("Timeout".to_string());
                }
            }

            // Remove old operations
            timed_out.retain(|op_id| {
                if let Some(op) = pending.get(op_id) {
                    op.retry_count < 3 // Retry up to 3 times
                } else {
                    false
                }
            });
        }

        timed_out
    }

    /// Get statistics
    pub async fn get_stats(&self) -> CrossShardStats {
        let pending = self.pending_operations.read().await;
        let timed_out = self.check_timeouts().await;

        CrossShardStats {
            local_shard_id: self.local_shard_id,
            total_shards: self.total_shards,
            pending_operations: pending.len(),
            timed_out_operations: timed_out.len(),
            avg_response_time: Duration::from_millis(0), // Would be calculated
        }
    }
}

/// Mock network implementation for testing
struct MockCrossShardNetwork;

impl MockCrossShardNetwork {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CrossShardNetwork for MockCrossShardNetwork {
    async fn send_message(&self, _shard: u16, _message: CrossShardMessage) -> Result<(), ShardError> {
        Ok(())
    }

    async fn receive_message(&self) -> Result<CrossShardMessage, ShardError> {
        Err(ShardError::NetworkError("Not implemented".to_string()))
    }

    async fn get_peer_addresses(&self, _shard: u16) -> Vec<String> {
        vec!["127.0.0.1:1234".to_string()]
    }
}

/// Cross-shard statistics
#[derive(Debug, Clone)]
pub struct CrossShardStats {
    pub local_shard_id: u16,
    pub total_shards: u16,
    pub pending_operations: usize,
    pub timed_out_operations: usize,
    pub avg_response_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cross_shard_comm() {
        let comms = CrossShardComms::new(0, 4, Duration::from_secs(5));

        // Test message ID extraction
        let msg = CrossShardMessage::TransactionRequest {
            tx: Transaction::default(),
            source_shard: 0,
            target_shard: 1,
            operation_id: [1u8; 32],
            nonce: 123,
        };

        let op_id = comms.extract_operation_id(&msg);
        assert_eq!(op_id, [1u8; 32]);
    }

    #[test]
    fn test_operation_id_generation() {
        let comms = CrossShardComms::new(0, 4, Duration::from_secs(5));
        let op_id1 = comms.generate_operation_id();
        let op_id2 = comms.generate_operation_id();

        assert_ne!(op_id1, op_id2);
    }

    #[tokio::test]
    async fn test_timeout_checking() {
        let comms = CrossShardComms::new(0, 4, Duration::from_millis(100));

        // This would need mock time to test properly
        // For now, just verify the function doesn't panic
        let timed_out = comms.check_timeouts().await;
        assert!(timed_out.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // Re-export serde for tests
    use serde_json;
}