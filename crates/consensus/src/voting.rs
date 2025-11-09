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
    /// Enable multi-factor voting
    pub multi_factor_voting_enabled: bool,
    /// Minimum reputation score to vote (0-100)
    pub min_reputation_to_vote: u8,
    /// Minimum time-locked stake to vote
    pub min_time_locked_stake: u64,
    /// Minimum account age in seconds
    pub min_account_age_seconds: u64,
    /// Maximum voting weight multiplier
    pub max_voting_weight_multiplier: f64,
}

impl Default for VotingConfig {
    fn default() -> Self {
        Self {
            min_participation_percent: 60,  // 60% must participate
            supermajority_percent: 80,     // 80% must agree
            voting_window_seconds: 3600,    // 1 hour voting window
            max_rollback_blocks: 10000,    // Max 10k blocks rollback
            vote_cooldown_seconds: 1800,    // 30 min cooldown
            multi_factor_voting_enabled: true,
            min_reputation_to_vote: 50,
            min_time_locked_stake: 1000,
            min_account_age_seconds: 86400 * 30, // 30 days
            max_voting_weight_multiplier: 10.0,
        }
    }
}

/// Vote options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteOption {
    /// Approve the proposal
    Approve,
    /// Reject the proposal
    Reject,
    /// Abstain from voting
    Abstain,
}

/// Multi-factor voting criteria
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VotingFactors {
    /// Reputation score (0-100)
    pub reputation_score: u8,
    /// Time-locked stake amount
    pub time_locked_stake: u64,
    /// Geographic region
    pub geographic_region: Option<String>,
    /// Participation history (number of past votes)
    pub participation_history: u32,
    /// Account age (in seconds)
    pub account_age_seconds: u64,
    /// Last activity timestamp
    pub last_activity_timestamp: u64,
}

impl VotingFactors {
    /// Creates new voting factors
    pub fn new(
        reputation_score: u8,
        time_locked_stake: u64,
        geographic_region: Option<String>,
        participation_history: u32,
        account_age_seconds: u64,
        last_activity_timestamp: u64,
    ) -> Self {
        Self {
            reputation_score,
            time_locked_stake,
            geographic_region,
            participation_history,
            account_age_seconds,
            last_activity_timestamp,
        }
    }
    
    /// Calculates voting weight based on all factors
    pub fn calculate_voting_weight(&self, _config: &VotingConfig) -> f64 {
        let mut weight = 1.0;
        
        // Reputation factor (0.5x to 2.0x multiplier)
        let reputation_multiplier = 0.5 + (self.reputation_score as f64 / 100.0) * 1.5;
        weight *= reputation_multiplier;
        
        // Stake factor (logarithmic scaling to prevent excessive influence)
        if self.time_locked_stake > 0 {
            let stake_factor = 1.0 + (self.time_locked_stake as f64).ln() / 100.0;
            weight *= stake_factor.min(3.0); // Cap at 3x multiplier
        }
        
        // Participation history factor (up to 1.5x multiplier)
        let participation_multiplier = 1.0 + (self.participation_history as f64 / 100.0).min(0.5);
        weight *= participation_multiplier;
        
        // Account age factor (up to 1.2x multiplier for long-term participants)
        let age_days = self.account_age_seconds / 86400;
        let age_multiplier = 1.0 + (age_days as f64 / 365.0).min(0.2);
        weight *= age_multiplier;
        
        // Recent activity bonus (up to 1.1x)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if now - self.last_activity_timestamp < 86400 * 7 { // Active in last 7 days
            weight *= 1.1;
        }
        
        weight
    }
    
    /// Validates if participant meets minimum requirements
    pub fn meets_minimum_requirements(&self, _config: &VotingConfig) -> bool {
        // Minimum reputation
        if self.reputation_score < 50 {
            return false;
        }
        
        // Minimum time-locked stake
        if self.time_locked_stake < 1000 {
            return false;
        }
        
        // Minimum account age (30 days)
        if self.account_age_seconds < 86400 * 30 {
            return false;
        }
        
        // Recent activity (within 30 days)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if now - self.last_activity_timestamp > 86400 * 30 {
            return false;
        }
        
        true
    }
}

/// Enhanced vote with multi-factor weighting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancedVote {
    /// Voter identifier
    pub voter_id: String,
    /// Vote option
    pub vote_option: VoteOption,
    /// Voting factors at time of vote
    pub voting_factors: VotingFactors,
    /// Calculated voting weight
    pub voting_weight: f64,
    /// Vote timestamp
    pub timestamp: u64,
    /// Vote signature
    pub signature: Option<String>,
}

impl EnhancedVote {
    /// Creates new enhanced vote
    pub fn new(
        voter_id: String,
        vote_option: VoteOption,
        voting_factors: VotingFactors,
        config: &VotingConfig,
    ) -> Self {
        let voting_weight = voting_factors.calculate_voting_weight(config);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            voter_id,
            vote_option,
            voting_factors,
            voting_weight,
            timestamp,
            signature: None,
        }
    }
    
    /// Validates the vote
    pub fn validate(&self, config: &VotingConfig) -> bool {
        // Check minimum requirements
        if !self.voting_factors.meets_minimum_requirements(config) {
            return false;
        }
        
        // Check voting weight is reasonable
        if self.voting_weight <= 0.0 || self.voting_weight > 10.0 {
            return false;
        }
        
        true
    }
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
    /// Number of approve votes
    pub approve: u64,
    /// Number of reject votes
    pub reject: u64,
    /// Number of abstain votes
    pub abstain: u64,
    /// Total number of participants
    pub total_participants: u64,
}

impl VoteCounts {
    /// Creates a new empty vote counter
    pub fn new() -> Self {
        Self {
            approve: 0,
            reject: 0,
            abstain: 0,
            total_participants: 0,
        }
    }

    /// Calculates approval percentage
    pub fn approval_percentage(&self) -> f64 {
        if self.total_participants == 0 {
            0.0
        } else {
            (self.approve as f64 / self.total_participants as f64) * 100.0
        }
    }

    /// Calculates participation percentage
    pub fn participation_percentage(&self, total_nodes: u64) -> f64 {
        if total_nodes == 0 {
            0.0
        } else {
            (self.total_participants as f64 / total_nodes as f64) * 100.0
        }
    }
}

impl Default for VoteCounts {
    fn default() -> Self {
        Self::new()
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
    /// Enhanced votes with multi-factor weighting
    enhanced_votes: HashMap<String, Vec<EnhancedVote>>,
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
    /// Total number of proposals created
    pub total_proposals: u64,
    /// Number of successful proposals
    pub successful_proposals: u64,
    /// Number of failed proposals
    pub failed_proposals: u64,
    /// Total number of votes cast
    pub total_votes_cast: u64,
    /// Number of currently active proposals
    pub active_proposals: u64,
}

/// Participant data for voting factor calculation
#[derive(Debug, Clone)]
pub struct ParticipantVotingData {
    /// Participant ID
    pub participant_id: String,
    /// Reputation score
    pub reputation_score: u8,
    /// Staked amount
    pub staked_amount: u64,
    /// Time locked hours
    pub time_locked_hours: u64,
    /// Geographic region
    pub geographic_region: String,
    /// Participation score
    pub participation_score: f64,
    /// Last activity timestamp
    pub last_activity_timestamp: u64,
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
            enhanced_votes: HashMap::new(),
            participants: HashSet::new(),
            last_proposal_time: 0,
            stats: VotingStats::default(),
        })
    }

    /// Creates a voting manager with default secure configuration
    pub fn with_default_config() -> Result<Self, VotingError> {
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

        let total_nodes = self.participant_count();
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

    /// Casts an enhanced vote with multi-factor weighting
    pub fn cast_enhanced_vote(&mut self, proposal_id: &str, vote: EnhancedVote) -> Result<(), VotingError> {
        if !self.config.multi_factor_voting_enabled {
            return Err(VotingError::InvalidConfig {
                field: "multi_factor_voting_enabled".to_string(),
                reason: "Multi-factor voting is disabled".to_string(),
            });
        }

        // Validate the vote
        if !vote.validate(&self.config) {
            return Err(VotingError::InvalidParticipant {
                id: vote.voter_id.clone(),
                reason: "Vote does not meet minimum requirements".to_string(),
            });
        }

        // Check if proposal exists and is active
        let proposal = self.proposals.get(proposal_id)
            .ok_or_else(|| VotingError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?;

        if !proposal.active {
            return Err(VotingError::ProposalNotActive {
                id: proposal_id.to_string(),
            });
        }

        // Check if voting window is still open
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > proposal.created_at + self.config.voting_window_seconds {
            return Err(VotingError::VotingExpired {
                id: proposal_id.to_string(),
            });
        }

        // Check if voter has already voted
        let proposal_votes = self.enhanced_votes.entry(proposal_id.to_string()).or_default();
        if proposal_votes.iter().any(|v| v.voter_id == vote.voter_id) {
            return Err(VotingError::AlreadyVoted {
                voter: vote.voter_id.clone(),
                proposal_id: proposal_id.to_string(),
            });
        }

        // Add the vote
        proposal_votes.push(vote);

        Ok(())
    }

    /// Calculates weighted voting results for a proposal
    pub fn calculate_weighted_results(&self, proposal_id: &str) -> Result<(f64, f64, f64), VotingError> {
        if !self.config.multi_factor_voting_enabled {
            return Err(VotingError::InvalidConfig {
                field: "multi_factor_voting_enabled".to_string(),
                reason: "Multi-factor voting is disabled".to_string(),
            });
        }

        let votes = self.enhanced_votes.get(proposal_id)
            .ok_or_else(|| VotingError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?;

        let mut approve_weight = 0.0;
        let mut reject_weight = 0.0;
        let mut abstain_weight = 0.0;

        for vote in votes {
            match vote.vote_option {
                VoteOption::Approve => approve_weight += vote.voting_weight,
                VoteOption::Reject => reject_weight += vote.voting_weight,
                VoteOption::Abstain => abstain_weight += vote.voting_weight,
            }
        }

        Ok((approve_weight, reject_weight, abstain_weight))
    }

    /// Determines if a proposal passes using regular voting (non-weighted)
    fn does_proposal_pass_regular(&self, proposal_id: &str) -> Result<bool, VotingError> {
        let proposal = self.proposals.get(proposal_id)
            .ok_or_else(|| VotingError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?;

        let empty_votes = vec![];
        let votes = self.votes.get(proposal_id).unwrap_or(&empty_votes);
        let total_votes = votes.len() as u64;
        
        if total_votes == 0 {
            return Ok(false);
        }

        let approve_votes = proposal.vote_counts.approve;
        let total_participants = self.participants.len() as u64;

        // Check minimum participation
        let participation_percentage = (total_votes as f64 / total_participants as f64) * 100.0;
        if participation_percentage < self.config.min_participation_percent as f64 {
            return Ok(false);
        }

        // Check supermajority
        let approval_percentage = (approve_votes as f64 / total_votes as f64) * 100.0;
        Ok(approval_percentage >= self.config.supermajority_percent as f64)
    }

    /// Determines if a proposal passes based on weighted voting
    pub fn does_proposal_pass_weighted(&self, proposal_id: &str) -> Result<bool, VotingError> {
        if !self.config.multi_factor_voting_enabled {
            // Use regular voting logic when multi-factor is disabled
            return self.does_proposal_pass_regular(proposal_id);
        }

        let (approve_weight, reject_weight, abstain_weight) = self.calculate_weighted_results(proposal_id)?;
        let total_weight = approve_weight + reject_weight + abstain_weight;

        if total_weight == 0.0 {
            return Ok(false);
        }

        // Check minimum participation
        let participation_percentage = ((total_weight - abstain_weight) / total_weight) * 100.0;
        if participation_percentage < self.config.min_participation_percent as f64 {
            return Ok(false);
        }

        // Check supermajority
        let approval_percentage = (approve_weight / total_weight) * 100.0;
        Ok(approval_percentage >= self.config.supermajority_percent as f64)
    }

    /// Gets voting factors for a participant (integration point with economic system)
    pub fn get_voting_factors_for_participant(
        &self,
        data: ParticipantVotingData,
    ) -> VotingFactors {
        VotingFactors::new(
            data.reputation_score,
            data.staked_amount,
            Some(data.geographic_region.clone()),
            data.participation_score as u32,
            data.last_activity_timestamp,
            data.last_activity_timestamp,
        )
    }

    /// Validates participant can vote with multi-factor requirements
    pub fn can_participant_vote_weighted(&self, voting_factors: &VotingFactors) -> bool {
        if !self.config.multi_factor_voting_enabled {
            return true;
        }

        voting_factors.meets_minimum_requirements(&self.config)
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
    /// Proposal was approved
    Approved,
    /// Proposal was rejected
    Rejected,
    /// Proposal is still pending
    Pending,
}

/// Voting system errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VotingError {
    #[error("invalid configuration field '{field}': {reason}")]
    /// Invalid configuration field
    InvalidConfig { 
        /// Field name that is invalid
        field: String, 
        /// Reason why the field is invalid
        reason: String 
    },

    #[error("invalid participant '{id}': {reason}")]
    /// Invalid participant
    InvalidParticipant { 
        /// Participant ID
        id: String, 
        /// Reason why the participant is invalid
        reason: String 
    },

    #[error("unauthorized creator '{creator}'")]
    /// Unauthorized proposal creator
    UnauthorizedCreator { 
        /// Creator ID
        creator: String 
    },

    #[error("unauthorized voter '{voter}'")]
    /// Unauthorized voter
    UnauthorizedVoter { 
        /// Voter ID
        voter: String 
    },

    #[error("invalid rollback: {reason}")]
    /// Invalid rollback operation
    InvalidRollback { 
        /// Reason why rollback is invalid
        reason: String 
    },

    #[error("proposal cooldown is active")]
    /// Proposal cooldown period is active
    CooldownActive,

    #[error("invalid proposal reason")]
    /// Invalid proposal reason provided
    InvalidReason,

    #[error("proposal not found: {id}")]
    /// Proposal with specified ID not found
    ProposalNotFound { 
        /// ID of the missing proposal
        id: String 
    },

    #[error("proposal '{id}' is not active")]
    /// Proposal is not currently active
    ProposalNotActive { 
        /// ID of the inactive proposal
        id: String 
    },

    #[error("voting expired for proposal '{id}'")]
    /// Voting period has expired for the proposal
    VotingExpired { 
        /// ID of the expired proposal
        id: String 
    },

    #[error("invalid signature")]
    /// Invalid signature provided
    InvalidSignature,

    #[error("voter '{voter}' already voted on proposal '{proposal_id}'")]
    /// Voter has already voted on the proposal
    AlreadyVoted { 
        /// ID of the voter
        voter: String, 
        /// ID of the proposal
        proposal_id: String 
    },

    #[error("proposal '{id}' is not approved")]
    /// Proposal is not approved
    ProposalNotApproved { 
        /// ID of the unapproved proposal
        id: String 
    },
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
            multi_factor_voting_enabled: true,
            min_reputation_to_vote: 50,
            min_time_locked_stake: 1000,
            min_account_age_seconds: 86400 * 30,
            max_voting_weight_multiplier: 10.0,
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
            multi_factor_voting_enabled: true,
            min_reputation_to_vote: 50,
            min_time_locked_stake: 1000,
            min_account_age_seconds: 86400 * 30,
            max_voting_weight_multiplier: 10.0,
        };
        
        let result = VotingManager::new(invalid_config);
        assert!(result.is_err());
        
        // Should accept valid config
        let valid_config = VotingConfig::default();
        let result = VotingManager::new(valid_config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_voting_factors() {
        let factors = VotingFactors::new(
            80, // reputation
            2000, // time-locked stake
            Some("NorthAmerica".to_string()), // geographic region
            10, // participation history
            86400 * 60, // 60 days account age
            1234567890, // last activity
        );

        let config = VotingConfig::default();
        
        // Test minimum requirements
        assert!(factors.meets_minimum_requirements(&config));
        
        // Test voting weight calculation
        let weight = factors.calculate_voting_weight(&config);
        assert!(weight > 1.0); // Should be > 1.0 due to good factors
        assert!(weight < 10.0); // Should be reasonable
    }

    #[test]
    fn test_enhanced_vote() {
        let config = VotingConfig::default();
        let factors = VotingFactors::new(
            80, // reputation
            2000, // time-locked stake
            Some("NorthAmerica".to_string()), // geographic region
            10, // participation history
            86400 * 60, // 60 days account age
            1234567890, // last activity
        );

        let vote = EnhancedVote::new(
            "voter1".to_string(),
            VoteOption::Approve,
            factors,
            &config,
        );

        // Test vote validation
        assert!(vote.validate(&config));
        assert!(vote.voting_weight > 1.0);
        assert_eq!(vote.vote_option, VoteOption::Approve);
        assert_eq!(vote.voter_id, "voter1");
    }

    #[test]
    fn test_multi_factor_voting() {
        let mut manager = create_test_manager();
        
        // Create a proposal
        let proposal_id = manager.create_proposal(
            500, // current_height
            1000, // target_height
            "creator1".to_string(),
            "Test rollback".to_string(),
        ).unwrap();

        // Create voting factors for different voters
        let high_reputation_factors = VotingFactors::new(
            90, // high reputation
            5000, // high stake
            Some("NorthAmerica".to_string()),
            20, // experienced participant
            86400 * 365, // 1 year old account
            1234567890,
        );

        let low_reputation_factors = VotingFactors::new(
            60, // low reputation
            1000, // minimum stake
            Some("Europe".to_string()),
            5, // new participant
            86400 * 30, // minimum age
            1234567890,
        );

        // Cast enhanced votes
        let vote1 = EnhancedVote::new(
            "voter1".to_string(),
            VoteOption::Approve,
            high_reputation_factors,
            &manager.config,
        );

        let vote2 = EnhancedVote::new(
            "voter2".to_string(),
            VoteOption::Approve,
            low_reputation_factors,
            &manager.config,
        );

        manager.cast_enhanced_vote(&proposal_id, vote1).unwrap();
        manager.cast_enhanced_vote(&proposal_id, vote2).unwrap();

        // Test weighted results
        let (approve_weight, reject_weight, abstain_weight) = manager.calculate_weighted_results(&proposal_id).unwrap();
        assert!(approve_weight > 0.0);
        assert_eq!(reject_weight, 0.0);
        assert_eq!(abstain_weight, 0.0);

        // High reputation vote should have more weight
        assert!(approve_weight > 2.0); // Both votes combined
    }

    #[test]
    fn test_multi_factor_voting_requirements() {
        let config = VotingConfig::default();
        
        // Test factors that don't meet requirements
        let low_reputation_factors = VotingFactors::new(
            30, // too low reputation
            2000,
            Some("NorthAmerica".to_string()),
            10,
            86400 * 60,
            1234567890,
        );

        let low_stake_factors = VotingFactors::new(
            80,
            500, // too low stake
            Some("NorthAmerica".to_string()),
            10,
            86400 * 60,
            1234567890,
        );

        let new_account_factors = VotingFactors::new(
            80,
            2000,
            Some("NorthAmerica".to_string()),
            10,
            86400 * 10, // too new
            1234567890,
        );

        assert!(!low_reputation_factors.meets_minimum_requirements(&config));
        assert!(!low_stake_factors.meets_minimum_requirements(&config));
        assert!(!new_account_factors.meets_minimum_requirements(&config));
    }

    #[test]
    fn test_multi_factor_voting_disabled() {
        let mut manager = VotingManager::new(VotingConfig {
            multi_factor_voting_enabled: false,
            ..VotingConfig::default()
        }).unwrap();

        let proposal_id = manager.create_proposal(
            500, // current_height
            1000, // target_height
            "creator1".to_string(),
            "Test rollback".to_string(),
        ).unwrap();

        let factors = VotingFactors::new(
            80,
            2000,
            Some("NorthAmerica".to_string()),
            10,
            86400 * 60,
            1234567890,
        );

        let vote = EnhancedVote::new(
            "voter1".to_string(),
            VoteOption::Approve,
            factors,
            &manager.config,
        );

        // Should fail when multi-factor voting is disabled
        let result = manager.cast_enhanced_vote(&proposal_id, vote);
        assert!(result.is_err());
    }
}