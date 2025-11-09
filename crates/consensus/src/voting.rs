//! Secure voting mechanism for blockchain rollback decisions
//!
//! This module provides a decentralized voting system that allows
//! network participants to collectively decide on emergency rollbacks
//! without central authorities or single points of failure.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Voting configuration with security defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingConfig {
    /// Minimum participation percentage (0-100)
    pub min_participation_percent: u8,
    /// Supermajority percentage required (0-100)
    pub supermajority_percent: u8,
    /// Voting window duration in seconds
    pub voting_window_seconds: u64,
    /// Maximum rollback size in blocks
    pub max_rollback_blocks: u64,
    /// Minimum time between votes (seconds)
    pub vote_cooldown_seconds: u64,
}

impl Default for VotingConfig {
    fn default() -> Self {
        Self {
            min_participation_percent: 60,  // 60% must participate
            supermajority_percent: 80,     // 80% must agree
            voting_window_seconds: 3600,    // 1 hour voting window
            max_rollback_blocks: 10000,    // Max 10k blocks rollback
            vote_cooldown_seconds: 1800,    // 30 min cooldown
        }
    }
}

/// Vote options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteOption {
    Approve,
    Reject,
    Abstain,
}

/// Rollback proposal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackProposal {
    /// Unique proposal identifier
    pub id: String,
    /// Target rollback height
    pub target_height: u64,
    /// Current height when proposal was created
    pub current_height: u64,
    /// Reason for rollback
    pub reason: String,
    /// Proposal creator (node identifier)
    pub creator: String,
    /// Timestamp when proposal was created
    pub created_at: u64,
    /// Voting deadline
    pub deadline: u64,
    /// Current vote counts
    pub vote_counts: VoteCounts,
    /// Whether proposal is active
    pub active: bool,
    /// Whether proposal was executed
    pub executed: bool,
}

/// Vote counting structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteCounts {
    pub approve: u64,
    pub reject: u64,
    pub abstain: u64,
    pub total_participants: u64,
}

impl VoteCounts {
    pub fn new() -> Self {
        Self {
            approve: 0,
            reject: 0,
            abstain: 0,
            total_participants: 0,
        }
    }

    pub fn approval_percentage(&self) -> f64 {
        if self.total_participants == 0 {
            0.0
        } else {
            (self.approve as f64 / self.total_participants as f64) * 100.0
        }
    }

    pub fn participation_percentage(&self, total_nodes: u64) -> f64 {
        if total_nodes == 0 {
            0.0
        } else {
            (self.total_participants as f64 / total_nodes as f64) * 100.0
        }
    }
}

/// Individual vote
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    /// Proposal ID
    pub proposal_id: String,
    /// Voter identifier
    pub voter: String,
    /// Vote option
    pub option: VoteOption,
    /// Timestamp when vote was cast
    pub timestamp: u64,
    /// Vote signature (for verification)
    pub signature: String,
}

/// Secure voting manager
#[derive(Debug, Clone)]
pub struct VotingManager {
    /// Configuration
    config: VotingConfig,
    /// Active proposals
    proposals: HashMap<String, RollbackProposal>,
    /// Cast votes
    votes: HashMap<String, Vec<Vote>>,
    /// Network participants
    participants: HashSet<String>,
    /// Last proposal time (for cooldown)
    last_proposal_time: u64,
    /// Statistics
    stats: VotingStats,
}

/// Voting statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VotingStats {
    pub total_proposals: u64,
    pub successful_proposals: u64,
    pub failed_proposals: u64,
    pub total_votes_cast: u64,
    pub active_proposals: u64,
}

impl VotingManager {
    /// Creates a new voting manager with secure defaults
    pub fn new(config: VotingConfig) -> Result<Self, VotingError> {
        // Security: Validate configuration
        Self::validate_config(&config)?;

        Ok(Self {
            config,
            proposals: HashMap::new(),
            votes: HashMap::new(),
            participants: HashSet::new(),
            last_proposal_time: 0,
            stats: VotingStats::default(),
        })
    }

    /// Creates a voting manager with default secure configuration
    pub fn default() -> Result<Self, VotingError> {
        Self::new(VotingConfig::default())
    }

    /// Validates voting configuration
    fn validate_config(config: &VotingConfig) -> Result<(), VotingError> {
        if config.min_participation_percent == 0 || config.min_participation_percent > 100 {
            return Err(VotingError::InvalidConfig {
                field: "min_participation_percent".to_string(),
                reason: "Must be between 1 and 100".to_string(),
            });
        }

        if config.supermajority_percent == 0 || config.supermajority_percent > 100 {
            return Err(VotingError::InvalidConfig {
                field: "supermajority_percent".to_string(),
                reason: "Must be between 1 and 100".to_string(),
            });
        }

        if config.supermajority_percent <= config.min_participation_percent {
            return Err(VotingError::InvalidConfig {
                field: "supermajority_percent".to_string(),
                reason: "Must be greater than min_participation_percent".to_string(),
            });
        }

        if config.voting_window_seconds == 0 || config.voting_window_seconds > 86400 {
            return Err(VotingError::InvalidConfig {
                field: "voting_window_seconds".to_string(),
                reason: "Must be between 1 and 86400 seconds".to_string(),
            });
        }

        if config.max_rollback_blocks == 0 || config.max_rollback_blocks > 100000 {
            return Err(VotingError::InvalidConfig {
                field: "max_rollback_blocks".to_string(),
                reason: "Must be between 1 and 100000".to_string(),
            });
        }

        Ok(())
    }

    /// Adds a network participant
    pub fn add_participant(&mut self, participant_id: String) -> Result<(), VotingError> {
        // Security: Validate participant ID
        if participant_id.is_empty() || participant_id.len() > 100 {
            return Err(VotingError::InvalidParticipant {
                id: participant_id,
                reason: "Invalid participant ID length".to_string(),
            });
        }

        self.participants.insert(participant_id);
        Ok(())
    }

    /// Removes a network participant
    pub fn remove_participant(&mut self, participant_id: &str) {
        self.participants.remove(participant_id);
    }

    /// Gets current participant count
    pub fn participant_count(&self) -> u64 {
        self.participants.len() as u64
    }

    /// Creates a new rollback proposal with security validation
    pub fn create_proposal(
        &mut self,
        target_height: u64,
        current_height: u64,
        reason: String,
        creator: String,
    ) -> Result<String, VotingError> {
        // Security: Validate creator is a participant
        if !self.participants.contains(&creator) {
            return Err(VotingError::UnauthorizedCreator {
                creator: creator.clone(),
            });
        }

        // Security: Validate rollback parameters
        if target_height >= current_height {
            return Err(VotingError::InvalidRollback {
                reason: "Target height must be less than current height".to_string(),
            });
        }

        let rollback_size = current_height.saturating_sub(target_height);
        if rollback_size > self.config.max_rollback_blocks {
            return Err(VotingError::InvalidRollback {
                reason: format!("Rollback size {} exceeds maximum {}", 
                    rollback_size, self.config.max_rollback_blocks),
            });
        }

        // Security: Check cooldown period
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now < self.last_proposal_time + self.config.vote_cooldown_seconds {
            return Err(VotingError::CooldownActive);
        }

        // Security: Validate reason
        if reason.is_empty() || reason.len() > 500 {
            return Err(VotingError::InvalidReason);
        }

        // Generate secure proposal ID
        let proposal_id = self.generate_proposal_id();

        let deadline = now + self.config.voting_window_seconds;

        let proposal = RollbackProposal {
            id: proposal_id.clone(),
            target_height,
            current_height,
            reason: Self::sanitize_reason(&reason),
            creator,
            created_at: now,
            deadline,
            vote_counts: VoteCounts::new(),
            active: true,
            executed: false,
        };

        self.proposals.insert(proposal_id.clone(), proposal);
        self.votes.insert(proposal_id.clone(), Vec::new());
        self.last_proposal_time = now;
        self.stats.total_proposals += 1;
        self.stats.active_proposals += 1;

        Ok(proposal_id)
    }

    /// Casts a vote on a proposal with security validation
    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter: String,
        option: VoteOption,
        signature: String,
    ) -> Result<(), VotingError> {
        // Security: Validate voter is a participant
        if !self.participants.contains(&voter) {
            return Err(VotingError::UnauthorizedVoter {
                voter: voter.clone(),
            });
        }

        // Security: Check if proposal exists and is active
        let proposal = self.proposals.get(proposal_id)
            .ok_or_else(|| VotingError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?;

        if !proposal.active {
            return Err(VotingError::ProposalNotActive {
                id: proposal_id.to_string(),
            });
        }

        // Security: Check voting deadline
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > proposal.deadline {
            return Err(VotingError::VotingExpired {
                id: proposal_id.to_string(),
            });
        }

        // Security: Validate signature format
        if signature.is_empty() || signature.len() > 200 {
            return Err(VotingError::InvalidSignature);
        }

        // Check if voter already voted
        let votes = self.votes.get_mut(proposal_id).unwrap();
        if votes.iter().any(|v| v.voter == voter) {
            return Err(VotingError::AlreadyVoted {
                voter,
                proposal_id: proposal_id.to_string(),
            });
        }

        // Create and store vote
        let vote = Vote {
            proposal_id: proposal_id.to_string(),
            voter: voter.clone(),
            option: option.clone(),
            timestamp: now,
            signature,
        };

        votes.push(vote);
        self.stats.total_votes_cast += 1;

        // Update proposal vote counts
        self.update_proposal_vote_counts(proposal_id);

        Ok(())
    }

    /// Updates vote counts for a proposal
    fn update_proposal_vote_counts(&mut self, proposal_id: &str) {
        if let (Some(proposal), Some(votes)) = 
            (self.proposals.get_mut(proposal_id), self.votes.get(proposal_id)) {
            
            let mut counts = VoteCounts::new();
            
            for vote in votes {
                match vote.option {
                    VoteOption::Approve => counts.approve += 1,
                    VoteOption::Reject => counts.reject += 1,
                    VoteOption::Abstain => counts.abstain += 1,
                }
                counts.total_participants += 1;
            }
            
            proposal.vote_counts = counts;
        }
    }

    /// Checks if a proposal has passed
    pub fn check_proposal_result(&self, proposal_id: &str) -> Result<ProposalResult, VotingError> {
        let proposal = self.proposals.get(proposal_id)
            .ok_or_else(|| VotingError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?;

        let total_nodes = self.participant_count() as u64;
        let counts = &proposal.vote_counts;

        let participation = counts.participation_percentage(total_nodes);
        let approval = counts.approval_percentage();

        // Check if voting is still active
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let voting_expired = now > proposal.deadline;

        if participation >= self.config.min_participation_percent as f64 &&
           approval >= self.config.supermajority_percent as f64 {
            Ok(ProposalResult::Approved)
        } else if voting_expired {
            Ok(ProposalResult::Rejected)
        } else {
            Ok(ProposalResult::Pending)
        }
    }

    /// Executes a successful proposal
    pub fn execute_proposal(&mut self, proposal_id: &str) -> Result<(), VotingError> {
        // Check if proposal can be executed
        let result = self.check_proposal_result(proposal_id)?;
        if result != ProposalResult::Approved {
            return Err(VotingError::ProposalNotApproved {
                id: proposal_id.to_string(),
            });
        }

        // Mark proposal as executed
        if let Some(proposal) = self.proposals.get_mut(proposal_id) {
            proposal.executed = true;
            proposal.active = false;
        }

        self.stats.successful_proposals += 1;
        self.stats.active_proposals -= 1;

        Ok(())
    }

    /// Gets a proposal by ID
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&RollbackProposal> {
        self.proposals.get(proposal_id)
    }

    /// Gets all active proposals
    pub fn get_active_proposals(&self) -> Vec<&RollbackProposal> {
        self.proposals
            .values()
            .filter(|p| p.active)
            .collect()
    }

    /// Gets votes for a proposal
    pub fn get_proposal_votes(&self, proposal_id: &str) -> Option<&Vec<Vote>> {
        self.votes.get(proposal_id)
    }

    /// Gets voting statistics
    pub fn get_stats(&self) -> &VotingStats {
        &self.stats
    }

    /// Cleans up expired proposals
    pub fn cleanup_expired_proposals(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut expired_proposals = Vec::new();

        for (id, proposal) in &self.proposals {
            if proposal.active && now > proposal.deadline {
                expired_proposals.push(id.clone());
            }
        }

        for id in expired_proposals {
            if let Some(proposal) = self.proposals.get_mut(&id) {
                proposal.active = false;
                self.stats.active_proposals -= 1;
                self.stats.failed_proposals += 1;
            }
        }
    }

    /// Generates secure proposal ID
    fn generate_proposal_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .hash(&mut hasher);
        
        format!("prop_{:x}", hasher.finish())
    }

    /// Sanitizes proposal reason
    fn sanitize_reason(reason: &str) -> String {
        reason
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control())
            .take(500)
            .collect()
    }
}

/// Proposal result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalResult {
    Approved,
    Rejected,
    Pending,
}

/// Voting system errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VotingError {
    #[error("invalid configuration field '{field}': {reason}")]
    InvalidConfig { field: String, reason: String },

    #[error("invalid participant '{id}': {reason}")]
    InvalidParticipant { id: String, reason: String },

    #[error("unauthorized creator '{creator}'")]
    UnauthorizedCreator { creator: String },

    #[error("unauthorized voter '{voter}'")]
    UnauthorizedVoter { voter: String },

    #[error("invalid rollback: {reason}")]
    InvalidRollback { reason: String },

    #[error("proposal cooldown is active")]
    CooldownActive,

    #[error("invalid proposal reason")]
    InvalidReason,

    #[error("proposal not found: {id}")]
    ProposalNotFound { id: String },

    #[error("proposal '{id}' is not active")]
    ProposalNotActive { id: String },

    #[error("voting expired for proposal '{id}'")]
    VotingExpired { id: String },

    #[error("invalid signature")]
    InvalidSignature,

    #[error("voter '{voter}' already voted on proposal '{proposal_id}'")]
    AlreadyVoted { voter: String, proposal_id: String },

    #[error("proposal '{id}' is not approved")]
    ProposalNotApproved { id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> VotingManager {
        VotingManager::new(VotingConfig {
            min_participation_percent: 50,
            supermajority_percent: 75,
            voting_window_seconds: 3600,
            max_rollback_blocks: 1000,
            vote_cooldown_seconds: 300,
        }).unwrap()
    }

    #[test]
    fn test_voting_manager_creation() {
        let manager = create_test_manager();
        assert_eq!(manager.participant_count(), 0);
        assert_eq!(manager.get_stats().total_proposals, 0);
    }

    #[test]
    fn test_participant_management() {
        let mut manager = create_test_manager();
        
        // Should add participant
        manager.add_participant("node1".to_string()).unwrap();
        assert_eq!(manager.participant_count(), 1);
        
        // Should reject invalid participant
        let result = manager.add_participant("".to_string());
        assert!(result.is_err());
        
        // Should remove participant
        manager.remove_participant("node1");
        assert_eq!(manager.participant_count(), 0);
    }

    #[test]
    fn test_proposal_creation() {
        let mut manager = create_test_manager();
        manager.add_participant("creator".to_string()).unwrap();
        
        // Should create proposal
        let result = manager.create_proposal(
            900,
            1000,
            "Test rollback".to_string(),
            "creator".to_string(),
        );
        assert!(result.is_ok());
        
        let proposal_id = result.unwrap();
        let proposal = manager.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.target_height, 900);
        assert_eq!(proposal.current_height, 1000);
        assert!(proposal.active);
    }

    #[test]
    fn test_proposal_security() {
        let mut manager = create_test_manager();
        manager.add_participant("creator".to_string()).unwrap();
        
        // Should reject unauthorized creator
        let result = manager.create_proposal(
            900,
            1000,
            "Test rollback".to_string(),
            "unauthorized".to_string(),
        );
        assert!(result.is_err());
        
        // Should reject invalid rollback
        let result = manager.create_proposal(
            1000, // Same as current
            1000,
            "Test rollback".to_string(),
            "creator".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_voting_process() {
        let mut manager = create_test_manager();
        
        // Add participants
        manager.add_participant("creator".to_string()).unwrap();
        manager.add_participant("voter1".to_string()).unwrap();
        manager.add_participant("voter2".to_string()).unwrap();
        
        // Create proposal
        let proposal_id = manager.create_proposal(
            900,
            1000,
            "Test rollback".to_string(),
            "creator".to_string(),
        ).unwrap();
        
        // Cast votes
        manager.cast_vote(
            &proposal_id,
            "voter1".to_string(),
            VoteOption::Approve,
            "sig1".to_string(),
        ).unwrap();
        
        manager.cast_vote(
            &proposal_id,
            "voter2".to_string(),
            VoteOption::Approve,
            "sig2".to_string(),
        ).unwrap();
        
        // Check proposal result
        let result = manager.check_proposal_result(&proposal_id).unwrap();
        assert!(matches!(result, ProposalResult::Pending)); // Not enough participation yet
    }

    #[test]
    fn test_vote_security() {
        let mut manager = create_test_manager();
        manager.add_participant("creator".to_string()).unwrap();
        manager.add_participant("voter1".to_string()).unwrap();
        
        let proposal_id = manager.create_proposal(
            900,
            1000,
            "Test rollback".to_string(),
            "creator".to_string(),
        ).unwrap();
        
        // Should reject unauthorized voter
        let result = manager.cast_vote(
            &proposal_id,
            "unauthorized".to_string(),
            VoteOption::Approve,
            "sig1".to_string(),
        );
        assert!(result.is_err());
        
        // Should reject duplicate votes
        manager.cast_vote(
            &proposal_id,
            "voter1".to_string(),
            VoteOption::Approve,
            "sig1".to_string(),
        ).unwrap();
        
        let result = manager.cast_vote(
            &proposal_id,
            "voter1".to_string(),
            VoteOption::Reject,
            "sig2".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation() {
        // Should reject invalid config
        let invalid_config = VotingConfig {
            min_participation_percent: 0, // Invalid
            supermajority_percent: 75,
            voting_window_seconds: 3600,
            max_rollback_blocks: 1000,
            vote_cooldown_seconds: 300,
        };
        
        let result = VotingManager::new(invalid_config);
        assert!(result.is_err());
        
        // Should accept valid config
        let valid_config = VotingConfig::default();
        let result = VotingManager::new(valid_config);
        assert!(result.is_ok());
    }
}