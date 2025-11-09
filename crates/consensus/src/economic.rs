//! Economic safeguards for blockchain security
//!
//! This module provides staking and slashing mechanisms to create
//! economic incentives for honest behavior and disincentives for
//! malicious actions in the checkpoint and voting systems.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Economic configuration with security defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfig {
    /// Minimum stake amount to participate
    pub min_stake_amount: u64,
    /// Maximum stake amount per participant
    pub max_stake_amount: u64,
    /// Slash percentage for malicious proposals (0-100)
    pub malicious_proposal_slash_percent: u8,
    /// Slash percentage for false voting (0-100)
    pub false_voting_slash_percent: u8,
    /// Reward percentage for honest participation (0-100)
    pub honest_participation_reward_percent: u8,
    /// Unbonding period in seconds
    pub unbonding_period_seconds: u64,
    /// Minimum time between stake changes
    pub stake_change_cooldown_seconds: u64,
}

impl Default for EconomicConfig {
    fn default() -> Self {
        Self {
            min_stake_amount: 1000,      // Minimum 1000 units
            max_stake_amount: 1000000,    // Maximum 1M units
            malicious_proposal_slash_percent: 50,  // 50% slash
            false_voting_slash_percent: 25,        // 25% slash
            honest_participation_reward_percent: 5,  // 5% reward
            unbonding_period_seconds: 86400 * 7,   // 7 days
            stake_change_cooldown_seconds: 3600,    // 1 hour
        }
    }
}

/// Stake information for a participant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakeInfo {
    /// Participant identifier
    pub participant_id: String,
    /// Current staked amount
    pub staked_amount: u64,
    /// Pending unbonding amount
    pub unbonding_amount: u64,
    /// Unbonding completion time
    pub unbonding_completion_time: Option<u64>,
    /// Last stake change time
    pub last_stake_change_time: u64,
    /// Total rewards earned
    pub total_rewards: u64,
    /// Total slashes incurred
    pub total_slashed: u64,
    /// Reputation score (0-100)
    pub reputation_score: u8,
    /// Whether participant is currently bonded
    pub is_bonded: bool,
}

/// Slash event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashEvent {
    /// Unique slash identifier
    pub id: String,
    /// Participant who was slashed
    pub participant_id: String,
    /// Amount slashed
    pub amount: u64,
    /// Reason for slashing
    pub reason: SlashReason,
    /// Timestamp when slash occurred
    pub timestamp: u64,
    /// Associated proposal or vote ID
    pub related_id: Option<String>,
}

/// Reasons for slashing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlashReason {
    MaliciousProposal,
    FalseVoting,
    DoubleVoting,
    SignatureForgery,
    Inactivity,
    Other(String),
}

/// Reward event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardEvent {
    /// Unique reward identifier
    pub id: String,
    /// Participant who received reward
    pub participant_id: String,
    /// Reward amount
    pub amount: u64,
    /// Reason for reward
    pub reason: RewardReason,
    /// Timestamp when reward was given
    pub timestamp: u64,
    /// Associated proposal or vote ID
    pub related_id: Option<String>,
}

/// Reasons for rewards
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardReason {
    HonestVoting,
    ProposalCreation,
    ActiveParticipation,
    BugBounty,
    Other(String),
}

/// Economic manager for staking and slashing
#[derive(Debug, Clone)]
pub struct EconomicManager {
    /// Configuration
    config: EconomicConfig,
    /// Participant stakes
    stakes: HashMap<String, StakeInfo>,
    /// Slash events
    slash_events: Vec<SlashEvent>,
    /// Reward events
    reward_events: Vec<RewardEvent>,
    /// Total staked amount
    total_staked: u64,
    /// Statistics
    stats: EconomicStats,
}

/// Economic statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EconomicStats {
    pub total_participants: u64,
    pub total_staked: u64,
    pub total_slashed: u64,
    pub total_rewards: u64,
    pub active_proposals: u64,
    pub average_reputation: f64,
}

impl EconomicManager {
    /// Creates a new economic manager with secure defaults
    pub fn new(config: EconomicConfig) -> Result<Self, EconomicError> {
        // Security: Validate configuration
        Self::validate_config(&config)?;

        Ok(Self {
            config,
            stakes: HashMap::new(),
            slash_events: Vec::new(),
            reward_events: Vec::new(),
            total_staked: 0,
            stats: EconomicStats::default(),
        })
    }

    /// Creates an economic manager with default secure configuration
    pub fn default() -> Result<Self, EconomicError> {
        Self::new(EconomicConfig::default())
    }

    /// Validates economic configuration
    fn validate_config(config: &EconomicConfig) -> Result<(), EconomicError> {
        if config.min_stake_amount == 0 || config.min_stake_amount > config.max_stake_amount {
            return Err(EconomicError::InvalidConfig {
                field: "min_stake_amount".to_string(),
                reason: "Must be greater than 0 and less than max_stake_amount".to_string(),
            });
        }

        if config.malicious_proposal_slash_percent > 100 {
            return Err(EconomicError::InvalidConfig {
                field: "malicious_proposal_slash_percent".to_string(),
                reason: "Must be between 0 and 100".to_string(),
            });
        }

        if config.false_voting_slash_percent > 100 {
            return Err(EconomicError::InvalidConfig {
                field: "false_voting_slash_percent".to_string(),
                reason: "Must be between 0 and 100".to_string(),
            });
        }

        if config.honest_participation_reward_percent > 100 {
            return Err(EconomicError::InvalidConfig {
                field: "honest_participation_reward_percent".to_string(),
                reason: "Must be between 0 and 100".to_string(),
            });
        }

        Ok(())
    }

    /// Stakes tokens for a participant with security validation
    pub fn stake(&mut self, participant_id: String, amount: u64) -> Result<(), EconomicError> {
        // Security: Validate participant ID
        if participant_id.is_empty() || participant_id.len() > 100 {
            return Err(EconomicError::InvalidParticipant {
                id: participant_id,
                reason: "Invalid participant ID length".to_string(),
            });
        }

        // Security: Validate amount
        if amount == 0 {
            return Err(EconomicError::InvalidAmount {
                reason: "Stake amount must be greater than 0".to_string(),
            });
        }

        if amount < self.config.min_stake_amount {
            return Err(EconomicError::InsufficientStake {
                required: self.config.min_stake_amount,
                provided: amount,
            });
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let stake_info = self.stakes.entry(participant_id.clone()).or_insert_with(|| StakeInfo {
            participant_id: participant_id.clone(),
            staked_amount: 0,
            unbonding_amount: 0,
            unbonding_completion_time: None,
            last_stake_change_time: now,
            total_rewards: 0,
            total_slashed: 0,
            reputation_score: 100, // Start with perfect reputation
            is_bonded: false,
        });

        // Security: Check cooldown period
        if now < stake_info.last_stake_change_time + self.config.stake_change_cooldown_seconds {
            return Err(EconomicError::CooldownActive);
        }

        // Security: Check maximum stake
        let new_total = stake_info.staked_amount + amount;
        if new_total > self.config.max_stake_amount {
            return Err(EconomicError::ExcessiveStake {
                maximum: self.config.max_stake_amount,
                attempted: new_total,
            });
        }

        // Update stake
        stake_info.staked_amount = new_total;
        stake_info.last_stake_change_time = now;
        stake_info.is_bonded = true;

        self.total_staked += amount;
        self.update_stats();

        Ok(())
    }

    /// Unbonds tokens for a participant
    pub fn unbond(&mut self, participant_id: &str, amount: u64) -> Result<(), EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id)
            .ok_or_else(|| EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            })?;

        // Security: Validate amount
        if amount == 0 {
            return Err(EconomicError::InvalidAmount {
                reason: "Unbond amount must be greater than 0".to_string(),
            });
        }

        if amount > stake_info.staked_amount {
            return Err(EconomicError::InsufficientStake {
                required: amount,
                provided: stake_info.staked_amount,
            });
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Security: Check cooldown period
        if now < stake_info.last_stake_change_time + self.config.stake_change_cooldown_seconds {
            return Err(EconomicError::CooldownActive);
        }

        // Start unbonding process
        stake_info.staked_amount -= amount;
        stake_info.unbonding_amount += amount;
        stake_info.unbonding_completion_time = Some(now + self.config.unbonding_period_seconds);
        stake_info.last_stake_change_time = now;

        if stake_info.staked_amount == 0 {
            stake_info.is_bonded = false;
        }

        self.total_staked -= amount;
        self.update_stats();

        Ok(())
    }

    /// Withdraws completed unbonding tokens
    pub fn withdraw_unbonded(&mut self, participant_id: &str) -> Result<u64, EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id)
            .ok_or_else(|| EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if unbonding is complete
        let completion_time = stake_info.unbonding_completion_time
            .ok_or_else(|| EconomicError::NoUnbondingInProgress {
                id: participant_id.to_string(),
            })?;

        if now < completion_time {
            return Err(EconomicError::UnbondingNotComplete {
                id: participant_id.to_string(),
                completion_time,
            });
        }

        let amount = stake_info.unbonding_amount;
        stake_info.unbonding_amount = 0;
        stake_info.unbonding_completion_time = None;

        Ok(amount)
    }

    /// Slashes a participant for misbehavior
    pub fn slash(
        &mut self,
        participant_id: &str,
        reason: SlashReason,
        related_id: Option<String>,
    ) -> Result<u64, EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id)
            .ok_or_else(|| EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            })?;

        // Calculate slash amount based on reason
        let slash_percent = match reason {
            SlashReason::MaliciousProposal => self.config.malicious_proposal_slash_percent,
            SlashReason::FalseVoting => self.config.false_voting_slash_percent,
            SlashReason::DoubleVoting => 50, // 50% for double voting
            SlashReason::SignatureForgery => 100, // 100% for signature forgery
            SlashReason::Inactivity => 10, // 10% for inactivity
            SlashReason::Other(_) => 25, // Default 25%
        };

        let slash_amount = (stake_info.staked_amount * slash_percent as u64) / 100;

        if slash_amount == 0 {
            return Ok(0);
        }

        // Apply slash
        stake_info.staked_amount -= slash_amount;
        stake_info.total_slashed += slash_amount;
        
        // Update reputation
        stake_info.reputation_score = stake_info.reputation_score.saturating_sub(10);
        if stake_info.reputation_score == 0 {
            stake_info.is_bonded = false;
        }

        self.total_staked -= slash_amount;

        // Record slash event
        let slash_event = SlashEvent {
            id: self.generate_event_id("slash"),
            participant_id: participant_id.to_string(),
            amount: slash_amount,
            reason: reason.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            related_id,
        };

        self.slash_events.push(slash_event);
        self.stats.total_slashed += slash_amount;
        self.update_stats();

        Ok(slash_amount)
    }

    /// Rewards a participant for honest behavior
    pub fn reward(
        &mut self,
        participant_id: &str,
        reason: RewardReason,
        related_id: Option<String>,
    ) -> Result<u64, EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id)
            .ok_or_else(|| EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            })?;

        // Calculate reward amount
        let reward_percent = match reason {
            RewardReason::HonestVoting => self.config.honest_participation_reward_percent,
            RewardReason::ProposalCreation => 10, // 10% for proposal creation
            RewardReason::ActiveParticipation => 5, // 5% for active participation
            RewardReason::BugBounty => 20, // 20% for bug bounty
            RewardReason::Other(_) => 5, // Default 5%
        };

        let reward_amount = (stake_info.staked_amount * reward_percent as u64) / 100;

        if reward_amount == 0 {
            return Ok(0);
        }

        // Apply reward
        stake_info.total_rewards += reward_amount;
        
        // Update reputation
        stake_info.reputation_score = (stake_info.reputation_score + 1).min(100);

        // Record reward event
        let reward_event = RewardEvent {
            id: self.generate_event_id("reward"),
            participant_id: participant_id.to_string(),
            amount: reward_amount,
            reason: reason.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            related_id,
        };

        self.reward_events.push(reward_event);
        self.stats.total_rewards += reward_amount;

        Ok(reward_amount)
    }

    /// Gets stake information for a participant
    pub fn get_stake_info(&self, participant_id: &str) -> Option<&StakeInfo> {
        self.stakes.get(participant_id)
    }

    /// Gets all participants
    pub fn get_all_participants(&self) -> Vec<&StakeInfo> {
        self.stakes.values().collect()
    }

    /// Gets bonded participants
    pub fn get_bonded_participants(&self) -> Vec<&StakeInfo> {
        self.stakes
            .values()
            .filter(|s| s.is_bonded)
            .collect()
    }

    /// Gets slash events for a participant
    pub fn get_slash_events(&self, participant_id: &str) -> Vec<&SlashEvent> {
        self.slash_events
            .iter()
            .filter(|e| e.participant_id == participant_id)
            .collect()
    }

    /// Gets reward events for a participant
    pub fn get_reward_events(&self, participant_id: &str) -> Vec<&RewardEvent> {
        self.reward_events
            .iter()
            .filter(|e| e.participant_id == participant_id)
            .collect()
    }

    /// Gets economic statistics
    pub fn get_stats(&self) -> &EconomicStats {
        &self.stats
    }

    /// Updates economic statistics
    fn update_stats(&mut self) {
        self.stats.total_participants = self.stakes.len() as u64;
        self.stats.total_staked = self.total_staked;

        if !self.stakes.is_empty() {
            let total_reputation: u32 = self.stakes
                .values()
                .map(|s| s.reputation_score as u32)
                .sum();
            self.stats.average_reputation = total_reputation as f64 / self.stakes.len() as f64;
        }
    }

    /// Generates unique event ID
    fn generate_event_id(&self, prefix: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .hash(&mut hasher);
        
        format!("{}_{}_{:x}", prefix, std::process::id(), hasher.finish())
    }

    /// Processes unbonding completions
    pub fn process_unbonding_completions(&mut self) -> Vec<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut completed = Vec::new();

        for (participant_id, stake_info) in &mut self.stakes {
            if let Some(completion_time) = stake_info.unbonding_completion_time {
                if now >= completion_time {
                    completed.push(participant_id.clone());
                }
            }
        }

        completed
    }

    /// Validates participant can vote
    pub fn can_vote(&self, participant_id: &str) -> bool {
        if let Some(stake_info) = self.stakes.get(participant_id) {
            stake_info.is_bonded && 
            stake_info.staked_amount >= self.config.min_stake_amount &&
            stake_info.reputation_score >= 50 // Minimum reputation to vote
        } else {
            false
        }
    }

    /// Validates participant can create proposals
    pub fn can_create_proposal(&self, participant_id: &str) -> bool {
        if let Some(stake_info) = self.stakes.get(participant_id) {
            stake_info.is_bonded && 
            stake_info.staked_amount >= self.config.min_stake_amount * 2 && // Higher stake for proposals
            stake_info.reputation_score >= 70 // Higher reputation for proposals
        } else {
            false
        }
    }
}

/// Economic system errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EconomicError {
    #[error("invalid configuration field '{field}': {reason}")]
    InvalidConfig { field: String, reason: String },

    #[error("invalid participant '{id}': {reason}")]
    InvalidParticipant { id: String, reason: String },

    #[error("participant not found: {id}")]
    ParticipantNotFound { id: String },

    #[error("invalid amount: {reason}")]
    InvalidAmount { reason: String },

    #[error("insufficient stake: required {required}, provided {provided}")]
    InsufficientStake { required: u64, provided: u64 },

    #[error("excessive stake: maximum {maximum}, attempted {attempted}")]
    ExcessiveStake { maximum: u64, attempted: u64 },

    #[error("cooldown period is active")]
    CooldownActive,

    #[error("no unbonding in progress for participant: {id}")]
    NoUnbondingInProgress { id: String },

    #[error("unbonding not complete for participant: {id}, completion time: {completion_time}")]
    UnbondingNotComplete { id: String, completion_time: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> EconomicManager {
        EconomicManager::new(EconomicConfig {
            min_stake_amount: 100,
            max_stake_amount: 10000,
            malicious_proposal_slash_percent: 50,
            false_voting_slash_percent: 25,
            honest_participation_reward_percent: 5,
            unbonding_period_seconds: 3600, // 1 hour for tests
            stake_change_cooldown_seconds: 60, // 1 minute for tests
        }).unwrap()
    }

    #[test]
    fn test_staking() {
        let mut manager = create_test_manager();
        
        // Should stake successfully
        let result = manager.stake("participant1".to_string(), 1000);
        assert!(result.is_ok());
        
        let stake_info = manager.get_stake_info("participant1").unwrap();
        assert_eq!(stake_info.staked_amount, 1000);
        assert!(stake_info.is_bonded);
        assert_eq!(stake_info.reputation_score, 100);
    }

    #[test]
    fn test_staking_security() {
        let mut manager = create_test_manager();
        
        // Should reject insufficient stake
        let result = manager.stake("participant1".to_string(), 50);
        assert!(result.is_err());
        
        // Should reject invalid participant ID
        let result = manager.stake("".to_string(), 1000);
        assert!(result.is_err());
        
        // Should reject excessive stake
        let result = manager.stake("participant1".to_string(), 20000);
        assert!(result.is_err());
    }

    #[test]
    fn test_unbonding() {
        let mut manager = create_test_manager();
        manager.stake("participant1".to_string(), 1000).unwrap();
        
        // Should start unbonding
        let result = manager.unbond("participant1", 500);
        assert!(result.is_ok());
        
        let stake_info = manager.get_stake_info("participant1").unwrap();
        assert_eq!(stake_info.staked_amount, 500);
        assert_eq!(stake_info.unbonding_amount, 500);
        assert!(stake_info.unbonding_completion_time.is_some());
    }

    #[test]
    fn test_slashing() {
        let mut manager = create_test_manager();
        manager.stake("participant1".to_string(), 1000).unwrap();
        
        // Should slash for malicious proposal
        let slash_amount = manager.slash(
            "participant1",
            SlashReason::MaliciousProposal,
            Some("prop123".to_string()),
        ).unwrap();
        
        assert_eq!(slash_amount, 500); // 50% of 1000
        
        let stake_info = manager.get_stake_info("participant1").unwrap();
        assert_eq!(stake_info.staked_amount, 500);
        assert_eq!(stake_info.total_slashed, 500);
        assert_eq!(stake_info.reputation_score, 90);
    }

    #[test]
    fn test_rewards() {
        let mut manager = create_test_manager();
        manager.stake("participant1".to_string(), 1000).unwrap();
        
        // Should reward for honest voting
        let reward_amount = manager.reward(
            "participant1",
            RewardReason::HonestVoting,
            Some("vote123".to_string()),
        ).unwrap();
        
        assert_eq!(reward_amount, 50); // 5% of 1000
        
        let stake_info = manager.get_stake_info("participant1").unwrap();
        assert_eq!(stake_info.total_rewards, 50);
        assert_eq!(stake_info.reputation_score, 100); // Capped at 100
    }

    #[test]
    fn test_participation_validation() {
        let mut manager = create_test_manager();
        
        // Should not be able to vote without staking
        assert!(!manager.can_vote("participant1"));
        assert!(!manager.can_create_proposal("participant1"));
        
        // Should be able to vote after staking
        manager.stake("participant1".to_string(), 1000).unwrap();
        assert!(manager.can_vote("participant1"));
        assert!(!manager.can_create_proposal("participant1")); // Need higher stake
        
        // Should be able to create proposals with higher stake
        manager.stake("participant1".to_string(), 100).unwrap(); // Total 1100
        assert!(manager.can_create_proposal("participant1"));
    }

    #[test]
    fn test_config_validation() {
        // Should reject invalid config
        let invalid_config = EconomicConfig {
            min_stake_amount: 0, // Invalid
            max_stake_amount: 10000,
            malicious_proposal_slash_percent: 50,
            false_voting_slash_percent: 25,
            honest_participation_reward_percent: 5,
            unbonding_period_seconds: 86400,
            stake_change_cooldown_seconds: 3600,
        };
        
        let result = EconomicManager::new(invalid_config);
        assert!(result.is_err());
        
        // Should accept valid config
        let valid_config = EconomicConfig::default();
        let result = EconomicManager::new(valid_config);
        assert!(result.is_ok());
    }
}