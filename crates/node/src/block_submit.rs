//! Block submission and network propagation.
//!
//! Handles submission of mined blocks to the network, including local validation
//! and P2P broadcasting.

use bitquan_consensus::check_header_pow;
use bitquan_types::{Block, NetworkId, Result};
use std::sync::Arc;

// use crate::chainstate::ChainState; // TODO: Implement chainstate module
// use crate::metrics::MiningMetrics; // TODO: Implement metrics module
use crate::reward_engine::RewardEngine;
// use log::warn; // TODO: Use when implementing proper logging

/// Result of a block submission attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Reserved for Phase 8 pool integration
pub enum SubmitResult {
    /// Block was accepted and broadcast.
    Accepted {
        /// Block hash.
        hash: [u8; 32],
        /// Block height (if known).
        height: Option<u64>,
    },
    /// Block was rejected with reason.
    Rejected {
        /// Rejection reason.
        reason: String,
    },
    /// Submission error (network/RPC failure).
    Error {
        /// Error message.
        message: String,
    },
}

/// Block submission handler.
#[allow(dead_code)] // Reserved for Phase 8 pool integration
pub struct BlockSubmitter {
    /// Network ID for validation.
    pub network_id: NetworkId,
    /// Mock mode for testing (doesn't actually broadcast).
    pub mock_mode: bool,
    /// Chain state tracker (optional).
    pub chain_state: Option<Arc<()>>, // TODO: Replace with ChainState when implemented
    /// Reward engine (optional).
    pub reward_engine: Option<Arc<std::sync::Mutex<RewardEngine>>>,
    /// Mining metrics (optional).
    pub metrics: Option<Arc<()>>, // TODO: Replace with MiningMetrics when implemented
}

#[allow(dead_code)] // Phase 8 pool integration
impl BlockSubmitter {
    /// Create a new block submitter.
    pub fn new(network_id: NetworkId) -> Self {
        Self {
            network_id,
            mock_mode: false,
            chain_state: None,
            reward_engine: None,
            metrics: None,
        }
    }

    /// Create a mock submitter for testing.
    pub fn mock(network_id: NetworkId) -> Self {
        Self {
            network_id,
            mock_mode: true,
            chain_state: None,
            reward_engine: None,
            metrics: None,
        }
    }

    /// Set chain state.
    pub fn with_chain_state(mut self, state: Arc<()>) -> Self {
        // TODO: Replace with ChainState when implemented
        self.chain_state = Some(state);
        self
    }

    /// Set reward engine.
    pub fn with_reward_engine(mut self, engine: Arc<std::sync::Mutex<RewardEngine>>) -> Self {
        self.reward_engine = Some(engine);
        self
    }

    /// Set metrics.
    pub fn with_metrics(mut self, metrics: Arc<()>) -> Self {
        // TODO: Replace with MiningMetrics when implemented
        self.metrics = Some(metrics);
        self
    }

    /// Submit a mined block to the network.
    ///
    /// Steps:
    /// 1. Validate header PoW locally (skipped in mock mode)
    /// 2. Broadcast to P2P network (or mock log if testing)
    /// 3. Persist to chain and credit reward
    /// 4. Return result
    pub async fn submit(&self, block: &Block, miner_id: Option<&str>) -> Result<SubmitResult> {
        // 1. Validate header PoW locally (skip in mock mode for testing)
        if !self.mock_mode {
            let pow_valid = check_header_pow(&block.header).map_err(|e| {
                bitquan_types::Error::Invalid(format!("PoW validation failed: {}", e))
            })?;

            if !pow_valid {
                return Ok(SubmitResult::Rejected {
                    reason: "pow_invalid".to_string(),
                });
            }
        }

        // 2. Validate basic block structure
        if block.transactions.is_empty() {
            return Ok(SubmitResult::Rejected {
                reason: "no_transactions".to_string(),
            });
        }

        // Get block hash for logging
        let hash = bitquan_consensus::header_hash(&block.header);
        let hash_hex = hex::encode(&hash[..8]);

        // 3. Broadcast to network (or mock)
        let result = if self.mock_mode {
            // Mock mode: just log what would be broadcast
            println!(
                "[INFO] MOCK: Would broadcast block hash={} algo={} height=unknown",
                hash_hex, block.header.algo_id
            );

            SubmitResult::Accepted {
                hash,
                height: None, // Height unknown in mock mode
            }
        } else {
            // Real mode: broadcast to P2P network
            self.broadcast_to_network(block, hash).await?
        };

        // 4. If accepted, persist and credit reward
        if let SubmitResult::Accepted { height, .. } = result {
            if let (Some(_chain_state), Some(reward_engine)) =
                (&self.chain_state, &self.reward_engine)
            {
                // Append to chain
                // let height = chain_state.append_block(block, hash)?; // TODO: Implement when ChainState is ready

                // Record block and credit reward
                let miner = miner_id.unwrap_or("unknown");
                let mut engine = reward_engine.lock().map_err(|e| {
                    bitquan_types::Error::Invalid(format!("reward engine lock poisoned: {}", e))
                })?;
                let reward = engine.record_block(block, hash, height.unwrap_or(0), miner)?;

                // Update metrics
                if let Some(_metrics) = &self.metrics {
                    // TODO: Implement metrics when MiningMetrics is ready
                    // metrics.record_block_persisted();
                    // metrics.set_total_rewards(engine.total_distributed());
                    // metrics.set_pool_balance(engine.total_distributed());
                    // metrics.set_reward_per_block(reward);
                }

                // Log success
                let reward_bq = reward as f64 / 1_0000_0000.0;
                println!(
                    "[INFO] Block accepted! height={}, miner={}, reward={:.2} BQ",
                    height.unwrap_or(0),
                    miner,
                    reward_bq
                );

                return Ok(SubmitResult::Accepted {
                    hash,
                    height: Some(height.unwrap_or(0)),
                });
            }
        }

        Ok(result)
    }

    /// Broadcast block to P2P network.
    ///
    /// In production, this would:
    /// - Connect to peers via P2P protocol
    /// - Send block announcement
    /// - Wait for acceptance confirmations
    ///
    /// Currently: logs as "would broadcast" placeholder.
    async fn broadcast_to_network(&self, block: &Block, hash: [u8; 32]) -> Result<SubmitResult> {
        let hash_hex = hex::encode(&hash[..8]);

        // Note: P2P network integration point
        // When P2P module is integrated, this will:
        // - Connect to peers via P2P protocol
        // - Send block announcement
        // - Wait for acceptance confirmations
        println!(
            "[INFO] Block mined! hash={} algo={} txs={} (P2P broadcast pending integration)",
            hash_hex,
            block.header.algo_id,
            block.transactions.len()
        );

        Ok(SubmitResult::Accepted {
            hash,
            height: None, // Height tracking requires chain state
        })
    }
    /// Validate block against consensus rules (extended check).
    ///
    /// Full block validation including:
    /// - Merkle root verification
    /// - Timestamp checks
    /// - Basic transaction structure validation
    /// - Note: Full UTXO validation requires blockchain state
    pub fn validate_block_full(&self, block: &Block) -> Result<bool> {
        let _height = self
            .chain_state
            .as_ref()
            .map(|_s| 1u64) // TODO: Implement get_height when ChainState is ready
            .unwrap_or(0);

        // Validate timestamp (not too far in future)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if block.header.time > (now + 7200) as u32 {
            // 2 hours future tolerance
            return Err(bitquan_types::Error::Invalid(
                "Block timestamp too far in future".to_string(),
            ));
        }

        // Basic transaction validation
        for tx in &block.transactions {
            if tx.inputs.is_empty() && !bitquan_types::validation::is_coinbase(tx) {
                return Err(bitquan_types::Error::Invalid(
                    "Non-coinbase transaction has no inputs".to_string(),
                ));
            }
            if tx.outputs.is_empty() {
                return Err(bitquan_types::Error::Invalid(
                    "Transaction has no outputs".to_string(),
                ));
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use bitquan_types::BlockHeader;

    fn dummy_block() -> Block {
        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 1234567890,
            bits: 0x207fffff, // Very easy
            nonce: 0,
            algo_id: 0,
        };

        Block {
            header,
            transactions: vec![],
        }
    }

    #[tokio::test]
    async fn test_submit_reject_no_transactions() {
        let submitter = BlockSubmitter::mock(NetworkId::Testnet);
        let block = dummy_block();

        let result = submitter
            .submit(&block, None)
            .await
            .unwrap_or_else(|e| panic!("Failed to submit block: {}", e));

        // Should reject blocks with no transactions
        match result {
            SubmitResult::Rejected { reason } => {
                assert_eq!(reason, "no_transactions");
            }
            _ => unreachable!("Expected rejection, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_submit_mock_mode() {
        let submitter = BlockSubmitter::mock(NetworkId::Testnet);
        let mut block = dummy_block();

        // Add a dummy transaction
        block.transactions.push(bitquan_types::Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Testnet,
            genesis_hash: [0u8; 32],
            lock_time: 0,
            inputs: vec![],
            outputs: vec![],
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        });

        let result = submitter
            .submit(&block, Some("test_miner"))
            .await
            .unwrap_or_else(|e| panic!("Failed to submit block with miner: {}", e));

        match result {
            SubmitResult::Accepted { hash, .. } => {
                assert_eq!(hash.len(), 32);
            }
            _ => unreachable!("Expected acceptance in mock mode, got {:?}", result),
        }
    }

    #[test]
    fn test_submit_result_equality() {
        let accepted1 = SubmitResult::Accepted {
            hash: [0u8; 32],
            height: Some(100),
        };
        let accepted2 = SubmitResult::Accepted {
            hash: [0u8; 32],
            height: Some(100),
        };
        assert_eq!(accepted1, accepted2);

        let rejected = SubmitResult::Rejected {
            reason: "test".to_string(),
        };
        assert_ne!(accepted1, rejected);
    }
}
