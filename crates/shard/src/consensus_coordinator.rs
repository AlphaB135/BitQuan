//! Consensus Coordinator - Coordinates consensus across shards

use crate::{ShardConfig, ShardResult, ShardError};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::time::{Duration, Instant};
use bitquan_types::{Block, Transaction, BlockHeader};

/// Coordinates consensus across multiple shards
pub struct ConsensusCoordinator {
    config: ShardConfig,
    local_shard_id: u16,
    total_shards: u16,

    // Consensus state
    current_round: u64,
    validators: Arc<RwLock<HashMap<u16, Validator>>>,
    shard_leaders: Arc<RwLock<HashMap<u16, LeaderInfo>>>,
    pending_blocks: Arc<RwLock<HashMap<u64, PendingBlock>>>,

    // Message passing
    block_proposals: mpsc::Receiver<BlockProposal>,
    cross_shard_votes: mpsc::Receiver<CrossShardVote>,
    finalization_requests: mspc::Receiver<FinalizationRequest>,

    // Configuration
    block_time: Duration,
    finality_delay: u64,
}

/// Validator information
#[derive(Debug, Clone)]
pub struct Validator {
    pub address: [u8; 32],
    pub public_key: [u8; 32],
    pub stake: u64,
    pub is_active: bool,
    pub uptime: f64,
    pub slashing_events: u32,
}

/// Leader information for each shard
#[derive(Debug, Clone)]
pub struct LeaderInfo {
    pub validator_id: [u8; 32],
    pub public_key: [u8; 32],
    pub start_round: u64,
    pub end_round: u64,
}

/// Block proposal for consensus
#[derive(Debug, Clone)]
pub struct BlockProposal {
    pub shard_id: u16,
    pub block: Block,
    pub proposer: [u8; 32],
    pub round: u64,
    pub signature: [u8; 64],
}

/// Cross-shard vote
#[derive(Debug, Clone)]
pub struct CrossShardVote {
    pub voter_shard: u16,
    pub target_shard: u16,
    pub block_hash: [u8; 32],
    pub vote: VoteType,
    pub round: u64,
    pub signature: [u8; 64],
}

/// Vote types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteType {
    Approve,
    Reject,
    Abstain,
}

/// Finalization request
#[derive(Debug, Clone)]
pub struct FinalizationRequest {
    pub block_hash: [u8; 32],
    pub shard_id: u16,
    pub block_height: u64,
    pub state_root: [u8; 32],
}

/// Pending block for consensus
#[derive(Debug, Clone)]
pub struct PendingBlock {
    pub block: Block,
    pub shard_id: u16,
    pub round: u64,
    pub votes: HashMap<u16, VoteType>,
    pub received_at: Instant,
    pub finality_votes: u64,
}

/// Consensus result
#[derive(Debug)]
pub struct ConsensusResult {
    pub block: Block,
    pub finality_achieved: bool,
    finality_round: u64,
    votes_for: u64,
    votes_against: u64,
    voter_shards: Vec<u16>,
}

impl ConsensusCoordinator {
    /// Create a new consensus coordinator
    pub fn new(config: ShardConfig) -> Result<Self, ShardError> {
        config.validate_shard_id()?;

        let (block_proposals_tx, block_proposals_rx) = mpsc::channel(1000);
        let (cross_shard_votes_tx, cross_shard_votes_rx) = mpsc::channel(1000);
        let (finalization_requests_tx, finalization_requests_rx) = mpsc::channel(100);

        Ok(Self {
            config,
            local_shard_id: config.local_shard_id,
            total_shards: config.total_shards,
            current_round: 0,
            validators: Arc::new(RwLock::new(HashMap::new())),
            shard_leaders: Arc::new(RwLock::new(HashMap::new())),
            pending_blocks: Arc::new(RwLock::new(HashMap::new())),
            block_proposals: block_proposals_rx,
            cross_shard_votes: cross_shard_votes_rx,
            finalization_requests: finalization_requests_rx,
            block_time: Duration::from_secs(12),
            finality_delay: 6, // 6 rounds finality delay
        })
    }

    /// Start the consensus process
    pub async fn start(&self) {
        let mut block_proposals = self.block_proposals.clone();
        let mut cross_shard_votes = self.cross_shard_votes.clone();
        let mut finalization_requests = self.finalization_requests.clone();

        // Process block proposals
        tokio::spawn(async move {
            while let Some(proposal) = block_proposals.recv().await {
                if let Err(e) = self.handle_block_proposal(proposal).await {
                    eprintln!("Error handling block proposal: {}", e);
                }
            }
        });

        // Process cross-shard votes
        tokio::spawn(async move {
            while let Some(vote) = cross_shard_votes.recv().await {
                if let Err(e) = self.handle_cross_shard_vote(vote).await {
                    eprintln!("Error handling cross-shard vote: {}", e);
                }
            }
        });

        // Process finalization requests
        tokio::spawn(async move {
            while let Some(request) = finalization_requests.recv().await {
                if let Err(e) = self.handle_finalization_request(request).await {
                    eprintln!("Error handling finalization request: {}", e);
                }
            }
        });
    }

    /// Propose a block for consensus
    pub async fn propose_block(
        &self,
        block: Block,
        proposer: [u8; 32],
    ) -> Result<ConsensusResult, ShardError> {
        let round = self.current_round;
        let signature = self.sign_block(&block, &proposer);

        let proposal = BlockProposal {
            shard_id: self.local_shard_id,
            block: block.clone(),
            proposer,
            round,
            signature,
        };

        // Store pending block
        {
            let mut pending = self.pending_blocks.write().await;
            pending.insert(round, PendingBlock {
                block: block.clone(),
                shard_id: self.local_shard_id,
                round,
                votes: HashMap::new(),
                received_at: Instant::now(),
                finality_votes: 0,
            });
        }

        // Broadcast proposal to other shards
        self.broadcast_block_proposal(proposal).await?;

        // Wait for consensus
        self.wait_for_consensus(round).await
    }

    /// Handle incoming block proposal
    async fn handle_block_proposal(&self, proposal: BlockProposal) -> Result<(), ShardError> {
        // Validate proposal
        if !self.validate_block_proposal(&proposal)? {
            return Ok(()); // Invalid proposal, ignore
        }

        // Check if we should vote
        let should_vote = self.should_vote_on_block(&proposal).await?;

        if should_vote {
            let vote_type = self.evaluate_block(&proposal.block).await?;
            let vote = CrossShardVote {
                voter_shard: self.local_shard_id,
                target_shard: proposal.shard_id,
                block_hash: block_hash(&proposal.block.header),
                vote: vote_type,
                round: proposal.round,
                signature: self.sign_vote(&vote_type, &proposal.block),
            };

            // Send vote
            self.send_cross_shard_vote(vote).await?;
        }

        Ok(())
    }

    /// Handle cross-shard vote
    async fn handle_cross_shard_vote(&self, vote: CrossShardVote) -> Result<(), ShardError> {
        // Validate vote
        if !self.validate_vote(&vote)? {
            return Ok(());
        }

        // Update pending block
        let mut pending = self.pending_blocks.write().await;
        if let Some(pending_block) = pending.get_mut(&vote.round) {
            if pending_block.block_hash == vote.block_hash {
                pending_block.votes.insert(vote.voter_shard, vote.vote.clone());
                pending_block.finality_votes += 1;
            }
        }

        // Check for finality
        if let Some(pending_block) = pending.get(&vote.round) {
            if pending_block.finality_votes >= self.required_finality_votes() {
                self.finalize_block(pending_block).await?;
            }
        }

        Ok(())
    }

    /// Handle finalization request
    async fn handle_finalization_request(&self, request: FinalizationRequest) -> Result<(), ShardError> {
        // Validate the request
        if !self.validate_finalization_request(&request).await {
            return Ok(());
        }

        // Update shard state with finalized block
        // In a real implementation, this would update the local chain state
        println!("Block {} finalized by shard {}",
                hex::encode(request.block_hash), request.shard_id);

        Ok(())
    }

    /// Validate block proposal
    fn validate_block_proposal(&self, proposal: &BlockProposal) -> Result<bool, ShardError> {
        // Check if proposer is valid leader
        let leaders = self.shard_leaders.read().await;
        if let Some(leader) = leaders.get(&proposal.shard_id) {
            // Check if proposer matches leader
            if leader.validator_id != proposal.proposer {
                return Ok(false);
            }

            // Check if round is valid
            if proposal.round < leader.start_round || proposal.round > leader.end_round {
                return Ok(false);
            }
        }

        // Validate block structure
        self.validate_block_structure(&proposal.block)?;

        Ok(true)
    }

    /// Determine if we should vote on a block
    async fn should_vote_on_block(&self, proposal: &BlockProposal) -> Result<bool, ShardError> {
        // Simple voting strategy: vote on blocks from shards we know
        let validators = self.validators.read().await;
        validators.contains_key(&proposal.shard_id)
    }

    /// Evaluate a block and determine vote
    async fn evaluate_block(&self, block: &Block) -> Result<VoteType, ShardError> {
        // Implement block evaluation logic
        // This would check:
        // - Block validity
        // - Transaction validity
        // - State transitions
        // - Consensus rules

        // For now, always approve
        Ok(VoteType::Approve)
    }

    /// Sign a block
    fn sign_block(&self, block: &Block, signer: &[u8; 32]) -> [u8; 64] {
        // In a real implementation, this would use cryptographic signatures
        // For now, return a dummy signature
        [0u8; 64]
    }

    /// Sign a vote
    fn sign_vote(&self, vote: &VoteType, block: &Block) -> [u8; 64] {
        // In a real implementation, this would use cryptographic signatures
        [0u8; 64]
    }

    /// Broadcast block proposal to other shards
    async fn broadcast_block_proposal(&self, proposal: BlockProposal) -> Result<(), ShardError> {
        // In a real implementation, this would send to other shards via network
        println!("Broadcasting block proposal to other shards");
        Ok(())
    }

    /// Send cross-shard vote
    async fn send_cross_shard_vote(&self, vote: CrossShardVote) -> Result<(), ShardError> {
        // In a real implementation, this would send to target shard
        println!("Sending vote from {} to {}", vote.voter_shard, vote.target_shard);
        Ok(())
    }

    /// Wait for consensus on a block
    async fn wait_for_consensus(&self, round: u64) -> Result<ConsensusResult, ShardError> {
        // Wait for sufficient votes
        let mut timeout = tokio::time::timeout(self.block_time * 2, async {
            loop {
                let pending = self.pending_blocks.read().await;
                if let Some(block) = pending.get(&round) {
                    if block.votes.len() >= self.required_votes() {
                        return Some(block.clone());
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }).await;

        match timeout {
            Ok(Some(block)) => {
                let votes_for = block.votes.values().filter(|v| *v == &VoteType::Approve).count() as u64;
                let votes_against = block.votes.len() as u64 - votes_for;
                let finality_achieved = block.finality_votes >= self.required_finality_votes();

                Ok(ConsensusResult {
                    block: block.block,
                    finality_achieved,
                    finality_round: round,
                    votes_for,
                    votes_against,
                    voter_shards: block.votes.keys().cloned().collect(),
                })
            }
            Ok(None) => Err(ShardError::ConsensusError("Timeout waiting for consensus".into())),
            Err(_) => Err(ShardError::ConsensusError("Timeout waiting for consensus".into())),
        }
    }

    /// Finalize a block
    async fn finalize_block(&self, pending_block: &PendingBlock) -> Result<(), ShardError> {
        // Move block to finalized state
        // Update chain state
        // Notify other shards

        println!("Finalizing block {} from shard {}",
                block_hash(&pending_block.block.header), pending_block.shard_id);

        // Remove from pending
        let mut pending = self.pending_blocks.write().await;
        pending.remove(&pending_block.round);

        Ok(())
    }

    /// Validate vote
    fn validate_vote(&self, vote: &CrossShardVote) -> Result<bool, ShardError> {
        // Validate signature, round, etc.
        // For now, always valid
        Ok(true)
    }

    /// Validate finalization request
    async fn validate_finalization_request(&self, request: &FinalizationRequest) -> bool {
        // Validate that the requesting shard can finalize this block
        // For now, always true
        true
    }

    /// Calculate required votes for consensus
    fn required_votes(&self) -> usize {
        // Simple majority
        (self.total_shards as usize / 2) + 1
    }

    /// Calculate required finality votes
    fn required_finality_votes(&self) -> u64 {
        // 2/3 majority
        (self.total_shards as u64 * 2 / 3) + 1
    }

    /// Validate block structure
    fn validate_block_structure(&self, block: &Block) -> Result<(), ShardError> {
        // Basic validation
        if block.transactions.is_empty() {
            return Err(ShardError::ConsensusError("Empty block".into()));
        }

        // Check coinbase transaction
        if block.transactions.len() > 0 {
            let coinbase = &block.transactions[0];
            if coinbase.inputs.len() == 1 && coinbase.inputs[0].prev_txid == [0u8; 32] {
                // Valid coinbase
            } else {
                return Err(ShardError::ConsensusError("Invalid coinbase".into()));
            }
        }

        Ok(())
    }

    /// Initialize validator set
    pub async fn initialize_validators(&self, validators: Vec<Validator>) {
        let mut validator_map = self.validators.write().await;
        validator_map.clear();

        for validator in validators {
            validator_map.insert(validator.address[0] as u16, validator);
        }
    }

    /// Update shard leaders
    pub async fn update_leaders(&self, leaders: HashMap<u16, LeaderInfo>) {
        *self.shard_leaders.write().await = leaders;
    }

    /// Get consensus statistics
    pub async fn get_stats(&self) -> ConsensusStats {
        let pending = self.pending_blocks.read().await;
        let validators = self.validators.read().await;

        ConsensusStats {
            current_round: self.current_round,
            pending_blocks: pending.len(),
            active_validators: validators.len(),
            block_time_ms: self.block_time.as_millis() as u64,
            finality_delay_rounds: self.finality_delay,
        }
    }
}

/// Calculate block hash
fn block_hash(header: &BlockHeader) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let bytes = header.to_bytes();
    let hash = Sha256::digest(bytes);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    result
}

/// Consensus statistics
#[derive(Debug, Clone)]
pub struct ConsensusStats {
    pub current_round: u64,
    pub pending_blocks: usize,
    pub active_validators: usize,
    pub block_time_ms: u64,
    pub finality_delay_rounds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_votes() {
        let config = ShardConfig {
            total_shards: 4,
            local_shard_id: 0,
            ..Default::default()
        };
        let coordinator = ConsensusCoordinator::new(config).unwrap();

        assert_eq!(coordinator.required_votes(), 3);
        assert_eq!(coordinator.required_finality_votes(), 3);
    }

    #[test]
    fn test_required_votes_odd() {
        let config = ShardConfig {
            total_shards: 5,
            local_shard_id: 0,
            ..Default::default()
        };
        let coordinator = ConsensusCoordinator::new(config).unwrap();

        assert_eq!(coordinator.required_votes(), 3);
        assert_eq!(coordinator.required_finality_votes(), 4);
    }

    #[tokio::test]
    async fn test_validator_initialization() {
        let config = ShardConfig::default();
        let coordinator = ConsensusCoordinator::new(config).unwrap();

        let validators = vec![
            Validator {
                address: [0u8; 32],
                public_key: [1u8; 32],
                stake: 1000,
                is_active: true,
                uptime: 1.0,
                slashing_events: 0,
            }
        ];

        coordinator.initialize_validators(validators).await;

        let stats = coordinator.get_stats().await;
        assert_eq!(stats.active_validators, 1);
    }
}