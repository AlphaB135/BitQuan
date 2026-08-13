//! Economic safeguards for blockchain security
//!
//! This module provides staking and slashing mechanisms to create
//! economic incentives for honest behavior and disincentives for
//! malicious actions in consensus validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::time::{SystemTime, UNIX_EPOCH};
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
    /// Minimum reputation score to create rollback proposals (0-100)
    pub min_reputation_for_proposal: u8,
    /// Reputation penalty for malicious proposals
    pub malicious_proposal_reputation_penalty: u8,
    /// Reputation reward for honest voting
    pub honest_voting_reputation_reward: u8,
    /// Reputation recovery cooldown period in seconds
    pub reputation_recovery_cooldown_seconds: u64,
    /// Minimum time-lock period for voting eligibility (seconds)
    pub min_stake_time_lock_seconds: u64,
}

impl Default for EconomicConfig {
    fn default() -> Self {
        Self {
            min_stake_amount: 1000,                           // Minimum 1000 units
            max_stake_amount: 1000000,                        // Maximum 1M units
            malicious_proposal_slash_percent: 50,             // 50% slash
            false_voting_slash_percent: 25,                   // 25% slash
            honest_participation_reward_percent: 5,           // 5% reward
            unbonding_period_seconds: 86400 * 7,              // 7 days
            stake_change_cooldown_seconds: 3600,              // 1 hour
            min_reputation_for_proposal: 80,                  // 80 reputation required
            malicious_proposal_reputation_penalty: 20,        // -20 reputation penalty
            honest_voting_reputation_reward: 2,               // +2 reputation reward
            reputation_recovery_cooldown_seconds: 86400 * 30, // 30 days
            min_stake_time_lock_seconds: 86400 * 30,          // 30 days time-lock
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
    /// Time-locked staked amount (eligible for voting)
    pub time_locked_amount: u64,
    /// Pending unbonding amount
    pub unbonding_amount: u64,
    /// Unbonding completion time
    pub unbonding_completion_time: Option<u64>,
    /// Last stake change time
    pub last_stake_change_time: u64,
    /// Time when stake was locked (for time-lock calculation)
    pub stake_lock_time: Option<u64>,
    /// Total rewards earned
    pub total_rewards: u64,
    /// Total slashes incurred
    pub total_slashed: u64,
    /// Reputation score (0-100)
    pub reputation_score: u8,
    /// Last reputation change time
    pub last_reputation_change_time: u64,
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
    /// Created malicious proposals
    MaliciousProposal,
    /// Provided false voting information
    FalseVoting,
    /// Voted multiple times on the same proposal
    DoubleVoting,
    /// Forged signatures
    SignatureForgery,
    /// Inactive for extended period
    Inactivity,
    /// Other reason with custom description
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
    /// Rewarded for honest voting behavior
    HonestVoting,
    /// Rewarded for creating proposals
    ProposalCreation,
    /// Rewarded for active participation
    ActiveParticipation,
    /// Rewarded for bug bounty contributions
    BugBounty,
    /// Other reason with custom description
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
    /// Reputation recovery events
    pub reputation_recovery_events: Vec<ReputationRecoveryEvent>,
    /// Total staked amount
    total_staked: u64,
    /// Statistics
    stats: EconomicStats,
}

/// Reputation recovery event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationRecoveryEvent {
    /// Unique recovery identifier
    pub id: String,
    /// Participant who recovered reputation
    pub participant_id: String,
    /// Amount of reputation recovered
    pub amount_recovered: u8,
    /// Recovery reason
    pub reason: String,
    /// Timestamp when recovery occurred
    pub timestamp: u64,
}

/// Economic statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EconomicStats {
    /// Total number of participants
    pub total_participants: u64,
    /// Total amount staked by all participants
    pub total_staked: u64,
    /// Total amount slashed from participants
    pub total_slashed: u64,
    /// Total rewards distributed
    pub total_rewards: u64,
    /// Number of currently active proposals
    pub active_proposals: u64,
    /// Average reputation score across all participants
    pub average_reputation: f64,
    /// Total number of reputation recoveries
    pub total_reputation_recoveries: u64,
    /// Number of participants above reputation threshold
    pub participants_above_reputation_threshold: u64,
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
            reputation_recovery_events: Vec::new(),
            total_staked: 0,
            stats: EconomicStats::default(),
        })
    }

    /// Creates an economic manager with default secure configuration
    pub fn with_default_config() -> Result<Self, EconomicError> {
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

        let stake_info = self
            .stakes
            .entry(participant_id.clone())
            .or_insert_with(|| StakeInfo {
                participant_id: participant_id.clone(),
                staked_amount: 0,
                time_locked_amount: 0,
                unbonding_amount: 0,
                unbonding_completion_time: None,
                last_stake_change_time: now,
                stake_lock_time: None,
                total_rewards: 0,
                total_slashed: 0,
                reputation_score: 100, // Start with perfect reputation
                last_reputation_change_time: now,
                is_bonded: false,
            });

        // Security: Check cooldown period
        if now < stake_info.last_stake_change_time + self.config.stake_change_cooldown_seconds {
            return Err(EconomicError::CooldownActive);
        }

        // Security: Check for u64 overflow before adding stake (H-8)
        let new_total =
            stake_info
                .staked_amount
                .checked_add(amount)
                .ok_or(EconomicError::StakeOverflow {
                    current: stake_info.staked_amount,
                    additional: amount,
                })?;
        if new_total > self.config.max_stake_amount {
            return Err(EconomicError::ExcessiveStake {
                maximum: self.config.max_stake_amount,
                attempted: new_total,
            });
        }

        // Update stake with time-lock
        stake_info.staked_amount = new_total;
        stake_info.last_stake_change_time = now;
        stake_info.stake_lock_time = Some(now); // Start time-lock for new stake
        stake_info.is_bonded = true;

        // Security: Check for u64 overflow on total_staked (H-8)
        self.total_staked =
            self.total_staked
                .checked_add(amount)
                .ok_or(EconomicError::StakeOverflow {
                    current: self.total_staked,
                    additional: amount,
                })?;
        self.update_time_locked_stakes();
        self.update_stats();

        Ok(())
    }

    /// Unbonds tokens for a participant
    pub fn unbond(&mut self, participant_id: &str, amount: u64) -> Result<(), EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
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
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if unbonding is complete
        let completion_time = stake_info.unbonding_completion_time.ok_or_else(|| {
            EconomicError::NoUnbondingInProgress {
                id: participant_id.to_string(),
            }
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
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
        })?;

        // Calculate slash amount based on reason
        let slash_percent = match reason {
            SlashReason::MaliciousProposal => self.config.malicious_proposal_slash_percent,
            SlashReason::FalseVoting => self.config.false_voting_slash_percent,
            SlashReason::DoubleVoting => 50, // 50% for double voting
            SlashReason::SignatureForgery => 100, // 100% for signature forgery
            SlashReason::Inactivity => 10,   // 10% for inactivity
            SlashReason::Other(_) => 25,     // Default 25%
        };

        // Calculate slash amount based on total locked stake (staked + unbonding)
        let total_locked_stake = stake_info.staked_amount + stake_info.unbonding_amount;
        let slash_amount = (total_locked_stake * slash_percent as u64) / 100;

        if slash_amount == 0 {
            return Ok(0);
        }

        // Apply slash proportionally from staked and unbonding amounts
        let slash_from_staked = std::cmp::min(slash_amount, stake_info.staked_amount);
        let slash_from_unbonding = slash_amount - slash_from_staked;

        stake_info.staked_amount -= slash_from_staked;
        stake_info.unbonding_amount -= slash_from_unbonding;
        stake_info.total_slashed += slash_amount;

        // Update reputation
        stake_info.reputation_score = stake_info.reputation_score.saturating_sub(10);
        if stake_info.reputation_score == 0 {
            stake_info.is_bonded = false;
        }

        // Only reduce total_staked by the amount slashed from staked_amount
        self.total_staked -= slash_from_staked;

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
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
        })?;

        // Calculate reward amount
        let reward_percent = match reason {
            RewardReason::HonestVoting => self.config.honest_participation_reward_percent,
            RewardReason::ProposalCreation => 10, // 10% for proposal creation
            RewardReason::ActiveParticipation => 5, // 5% for active participation
            RewardReason::BugBounty => 20,        // 20% for bug bounty
            RewardReason::Other(_) => 5,          // Default 5%
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
        self.stakes.values().filter(|s| s.is_bonded).collect()
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
            let total_reputation: u32 = self
                .stakes
                .values()
                .map(|s| s.reputation_score as u32)
                .sum();
            self.stats.average_reputation = total_reputation as f64 / self.stakes.len() as f64;

            // Count participants above reputation threshold
            self.stats.participants_above_reputation_threshold = self
                .stakes
                .values()
                .filter(|s| s.reputation_score >= self.config.min_reputation_for_proposal)
                .count() as u64;

            // Count reputation recoveries
            self.stats.total_reputation_recoveries = self.reputation_recovery_events.len() as u64;

        }
    }

    /// Generates unique event ID using blake3 for collision resistance.
    ///
    /// # Security (L-10)
    /// Previously used `DefaultHasher` (SipHash-1-3) which is not
    /// collision-resistant under adversarial conditions. Replaced with
    /// blake3 to ensure event IDs are cryptographically unique.
    fn generate_event_id(&self, prefix: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&pid.to_le_bytes());
        hasher.update(prefix.as_bytes());
        // Include total_staked as additional entropy source
        hasher.update(&self.total_staked.to_le_bytes());
        let hash = hasher.finalize();

        // Use first 16 hex chars (64 bits) for the ID — sufficient for uniqueness
        format!("{}_{}_{}", prefix, pid, &hash.to_hex()[..16])
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

    /// Gets voting power for a participant based on time-locked stake
    pub fn get_voting_power(&self, participant_id: &str) -> Result<u64, EconomicError> {
        let stake_info =
            self.stakes
                .get(participant_id)
                .ok_or_else(|| EconomicError::ParticipantNotFound {
                    id: participant_id.to_string(),
                })?;

        // Only time-locked stake counts for voting power
        Ok(stake_info.time_locked_amount)
    }

    /// Validates participant can vote with enhanced security
    pub fn can_vote(&self, participant_id: &str) -> bool {
        if let Some(stake_info) = self.stakes.get(participant_id) {
            stake_info.is_bonded
                && stake_info.staked_amount >= self.config.min_stake_amount
                && stake_info.reputation_score >= 50 // Minimum reputation to vote
                && self.get_time_locked_stake(participant_id) >= self.config.min_stake_amount
        // Must have time-locked stake
        } else {
            false
        }
    }

    /// Validates participant can create proposals with enhanced security
    pub fn can_create_proposal(&self, participant_id: &str) -> bool {
        if let Some(stake_info) = self.stakes.get(participant_id) {
            stake_info.is_bonded
                && stake_info.staked_amount >= self.config.min_stake_amount * 2 // Higher stake for proposals
                && stake_info.reputation_score >= self.config.min_reputation_for_proposal // Use configurable threshold
                && self.get_time_locked_stake(participant_id) >= self.config.min_stake_amount * 2
        // Must have time-locked stake
        } else {
            false
        }
    }

    /// Gets time-locked stake amount for voting power calculation
    pub fn get_time_locked_stake(&self, participant_id: &str) -> u64 {
        if let Some(stake_info) = self.stakes.get(participant_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Calculate time-locked amount based on lock time
            if let Some(lock_time) = stake_info.stake_lock_time {
                if now >= lock_time + self.config.min_stake_time_lock_seconds {
                    stake_info.staked_amount // Fully time-locked
                } else {
                    // Partially time-locked based on time elapsed (deterministic integer math)
                    let elapsed = now.saturating_sub(lock_time);
                    let progress =
                        (elapsed as u128 * 100) / self.config.min_stake_time_lock_seconds as u128;
                    ((stake_info.staked_amount as u128 * progress) / 100) as u64
                }
            } else {
                0 // No time-lock
            }
        } else {
            0
        }
    }

    /// Updates time-locked stakes for all participants
    fn update_time_locked_stakes(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for stake_info in self.stakes.values_mut() {
            stake_info.time_locked_amount = if let Some(lock_time) = stake_info.stake_lock_time {
                if now >= lock_time + self.config.min_stake_time_lock_seconds {
                    stake_info.staked_amount // Fully time-locked
                } else {
                    // Partially time-locked based on time elapsed (deterministic integer math)
                    let elapsed = now.saturating_sub(lock_time);
                    let progress =
                        (elapsed as u128 * 100) / self.config.min_stake_time_lock_seconds as u128;
                    ((stake_info.staked_amount as u128 * progress) / 100) as u64
                }
            } else {
                0
            };
        }
    }

    /// Applies reputation penalty for malicious proposal
    pub fn apply_reputation_penalty(
        &mut self,
        participant_id: &str,
        _reason: &str,
    ) -> Result<(), EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check cooldown period
        if now
            < stake_info.last_reputation_change_time
                + self.config.reputation_recovery_cooldown_seconds
        {
            return Err(EconomicError::CooldownActive);
        }

        // Apply penalty
        let new_score = stake_info
            .reputation_score
            .saturating_sub(self.config.malicious_proposal_reputation_penalty);
        stake_info.reputation_score = new_score;
        stake_info.last_reputation_change_time = now;

        self.update_stats();
        Ok(())
    }

    /// Applies reputation reward for honest voting
    pub fn apply_reputation_reward(
        &mut self,
        participant_id: &str,
        _reason: &str,
    ) -> Result<(), EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Apply reward
        let new_score = stake_info
            .reputation_score
            .saturating_add(self.config.honest_voting_reputation_reward);
        stake_info.reputation_score = new_score.min(100); // Cap at 100
        stake_info.last_reputation_change_time = now;

        self.update_stats();
        Ok(())
    }

    /// Attempts reputation recovery for eligible participants
    pub fn attempt_reputation_recovery(
        &mut self,
        participant_id: &str,
    ) -> Result<u8, EconomicError> {
        let stake_info = self.stakes.get_mut(participant_id).ok_or_else(|| {
            EconomicError::ParticipantNotFound {
                id: participant_id.to_string(),
            }
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if recovery cooldown has passed
        if now
            < stake_info.last_reputation_change_time
                + self.config.reputation_recovery_cooldown_seconds
        {
            return Err(EconomicError::CooldownActive);
        }

        // Only recover if reputation is below threshold
        if stake_info.reputation_score >= 80 {
            return Err(EconomicError::InvalidAmount {
                reason: "Reputation is already above recovery threshold".to_string(),
            });
        }

        // Recover reputation (gradual recovery)
        let recovery_amount = std::cmp::min(10, 80 - stake_info.reputation_score);
        stake_info.reputation_score += recovery_amount;
        stake_info.last_reputation_change_time = now;

        // Record recovery event
        let recovery_event = ReputationRecoveryEvent {
            id: format!("recovery_{}_{}", participant_id, now),
            participant_id: participant_id.to_string(),
            amount_recovered: recovery_amount,
            reason: "Automatic recovery after cooldown period".to_string(),
            timestamp: now,
        };

        self.reputation_recovery_events.push(recovery_event);
        self.update_stats();

        Ok(recovery_amount)
    }
}

/// Economic system errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EconomicError {
    /// Invalid configuration error
    #[error("invalid configuration field '{field}': {reason}")]
    InvalidConfig {
        /// The configuration field name
        field: String,
        /// The reason for invalidity
        reason: String,
    },

    /// Invalid participant error
    #[error("invalid participant '{id}': {reason}")]
    InvalidParticipant {
        /// The participant ID
        id: String,
        /// The reason for invalidity
        reason: String,
    },

    /// Participant not found error
    #[error("participant not found: {id}")]
    ParticipantNotFound {
        /// The participant ID that was not found
        id: String,
    },

    /// Invalid amount error
    #[error("invalid amount: {reason}")]
    InvalidAmount {
        /// The reason the amount is invalid
        reason: String,
    },

    /// Insufficient stake error
    #[error("insufficient stake: required {required}, provided {provided}")]
    InsufficientStake {
        /// The required stake amount
        required: u64,
        /// The provided stake amount
        provided: u64,
    },

    /// Excessive stake error
    #[error("excessive stake: maximum {maximum}, attempted {attempted}")]
    ExcessiveStake {
        /// The maximum allowed stake amount
        maximum: u64,
        /// The attempted stake amount
        attempted: u64,
    },

    /// Cooldown period active error
    #[error("cooldown period is active")]
    CooldownActive,

    /// No unbonding in progress error
    #[error("no unbonding in progress for participant: {id}")]
    NoUnbondingInProgress {
        /// The participant ID
        id: String,
    },

    /// Unbonding not complete error
    #[error("unbonding not complete for participant: {id}, completion time: {completion_time}")]
    UnbondingNotComplete {
        /// The participant ID
        id: String,
        /// The completion time for unbonding
        completion_time: u64,
    },

    /// Stake arithmetic overflow (H-8)
    #[error("stake overflow: current {current} + additional {additional} exceeds u64::MAX")]
    StakeOverflow {
        /// The current stake amount before the operation
        current: u64,
        /// The additional amount that caused the overflow
        additional: u64,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn create_test_manager() -> EconomicManager {
        EconomicManager::new(EconomicConfig {
            min_stake_amount: 100,
            max_stake_amount: 10000,
            malicious_proposal_slash_percent: 50,
            false_voting_slash_percent: 25,
            honest_participation_reward_percent: 5,
            unbonding_period_seconds: 3600,   // 1 hour for tests
            stake_change_cooldown_seconds: 0, // No cooldown for tests
            min_reputation_for_proposal: 80,
            malicious_proposal_reputation_penalty: 20,
            honest_voting_reputation_reward: 2,
            reputation_recovery_cooldown_seconds: 0, // No cooldown for tests
            min_stake_time_lock_seconds: 0,          // No time lock for tests
            max_voting_power_per_region_percent: 30,
        })
        .expect("Failed to create test manager")
    }

    #[test]
    fn test_staking() {
        let mut manager = create_test_manager();

        // Should stake successfully
        let result = manager.stake("participant1".to_string(), 1000);
        assert!(result.is_ok());

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
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
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Should start unbonding
        let result = manager.unbond("participant1", 500);
        assert!(result.is_ok());

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.staked_amount, 500);
        assert_eq!(stake_info.unbonding_amount, 500);
        assert!(stake_info.unbonding_completion_time.is_some());
    }

    #[test]
    fn test_slashing() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Should slash for malicious proposal
        let slash_amount = manager
            .slash(
                "participant1",
                SlashReason::MaliciousProposal,
                Some("prop123".to_string()),
            )
            .expect("Failed to slash");

        assert_eq!(slash_amount, 500); // 50% of 1000

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.staked_amount, 500);
        assert_eq!(stake_info.total_slashed, 500);
        assert_eq!(stake_info.reputation_score, 90);
    }

    #[test]
    fn test_slashing_with_unbonding_amount() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Start unbonding 400 tokens
        manager
            .unbond("participant1", 400)
            .expect("Failed to unbond");

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.staked_amount, 600);
        assert_eq!(stake_info.unbonding_amount, 400);

        // Should slash based on total locked stake (600 + 400 = 1000)
        let slash_amount = manager
            .slash(
                "participant1",
                SlashReason::MaliciousProposal,
                Some("prop123".to_string()),
            )
            .expect("Failed to slash");

        assert_eq!(slash_amount, 500); // 50% of total locked stake (1000)

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        // Should deduct from staked first (500), then from unbonding (0)
        assert_eq!(stake_info.staked_amount, 100); // 600 - 500
        assert_eq!(stake_info.unbonding_amount, 400); // unchanged
        assert_eq!(stake_info.total_slashed, 500);
    }

    #[test]
    fn test_rewards() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Should reward for honest voting
        let reward_amount = manager
            .reward(
                "participant1",
                RewardReason::HonestVoting,
                Some("vote123".to_string()),
            )
            .expect("Failed to reward");

        assert_eq!(reward_amount, 50); // 5% of 1000

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.total_rewards, 50);
        assert_eq!(stake_info.reputation_score, 100); // Capped at 100
    }

    #[test]
    fn test_reputation_threshold() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Should be able to create proposal with high reputation
        assert!(manager.can_create_proposal("participant1"));

        // Apply reputation penalty
        manager
            .apply_reputation_penalty("participant1", "malicious proposal")
            .expect("Failed to apply reputation penalty");

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.reputation_score, 80); // 100 - 20

        // Should still be able to create proposal (exactly at threshold)
        assert!(manager.can_create_proposal("participant1"));

        // Apply another penalty
        manager
            .apply_reputation_penalty("participant1", "another malicious proposal")
            .expect("Failed to apply reputation penalty");

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.reputation_score, 60); // 80 - 20

        // Should not be able to create proposal anymore
        assert!(!manager.can_create_proposal("participant1"));
    }

    #[test]
    fn test_reputation_rewards() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Apply penalty first
        manager
            .apply_reputation_penalty("participant1", "malicious proposal")
            .expect("Failed to apply reputation penalty");
        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.reputation_score, 80);

        // Apply reward for honest voting
        manager
            .apply_reputation_reward("participant1", "honest voting")
            .expect("Failed to apply reputation reward");
        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.reputation_score, 82); // 80 + 2

        // Should cap at 100
        for _ in 0..20 {
            manager
                .apply_reputation_reward("participant1", "honest voting")
                .expect("Failed to apply reputation reward");
        }
        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.reputation_score, 100); // Capped
    }

    #[test]
    fn test_time_locked_stakes() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Initially should have time-locked stake (since time lock is disabled in tests)
        assert_eq!(manager.get_time_locked_stake("participant1"), 1000);

        // Should be able to vote (since time lock is disabled in tests)
        assert!(manager.can_vote("participant1"));

        // Should be able to create proposal (since time lock is disabled in tests)
        assert!(manager.can_create_proposal("participant1"));

        // Simulate time passing (5 minutes for test)
        // Note: In tests, we'll manually advance time instead of sleeping
        // std::thread::sleep(std::time::Duration::from_secs(6)); // Slightly more than test time-lock

        // Update time-locked stakes
        manager.update_time_locked_stakes();

        // Now should have time-locked stake (since time lock is disabled in tests)
        assert_eq!(manager.get_time_locked_stake("participant1"), 1000);

        // Should be able to vote and create proposal
        assert!(manager.can_vote("participant1"));
        assert!(manager.can_create_proposal("participant1"));
    }

    #[test]
    fn test_reputation_recovery() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");

        // Apply severe penalty
        manager
            .apply_reputation_penalty("participant1", "malicious proposal")
            .expect("Failed to apply reputation penalty");
        manager
            .apply_reputation_penalty("participant1", "malicious proposal")
            .expect("Failed to apply reputation penalty");

        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert_eq!(stake_info.reputation_score, 60);

        // Should recover if reputation is low (since cooldown is disabled in tests)
        let result = manager.attempt_reputation_recovery("participant1");
        assert!(result.is_ok());

        // Check that reputation increased
        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        assert!(stake_info.reputation_score > 60);

        // Should not be able to recover if reputation is already high (>= 80)
        // Apply more penalties to get above threshold
        for _ in 0..5 {
            let _ = manager.apply_reputation_penalty("participant1", "test penalty");
        }
        let stake_info = manager
            .get_stake_info("participant1")
            .expect("Failed to get stake info");
        if stake_info.reputation_score >= 80 {
            let result = manager.attempt_reputation_recovery("participant1");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_enhanced_voting_eligibility() {
        let mut manager = create_test_manager();
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake participant1");
        manager
            .stake("participant2".to_string(), 100)
            .expect("Failed to stake participant2"); // Minimum stake

        // participant1 should be able to vote (has enough stake and reputation)
        assert!(manager.can_vote("participant1"));

        // participant2 should be able to vote (minimum stake)
        assert!(manager.can_vote("participant2"));

        // participant1 should be able to create proposal (high stake and reputation)
        assert!(manager.can_create_proposal("participant1"));

        // participant2 should not be able to create proposal (insufficient stake)
        assert!(!manager.can_create_proposal("participant2"));

        // Apply penalty to participant1
        manager
            .apply_reputation_penalty("participant1", "malicious proposal")
            .expect("Failed to apply reputation penalty");
        manager
            .apply_reputation_penalty("participant1", "malicious proposal")
            .expect("Failed to apply reputation penalty");

        // participant1 should still be able to vote (reputation >= 50)
        assert!(manager.can_vote("participant1"));

        // participant1 should not be able to create proposal (reputation < 80)
        assert!(!manager.can_create_proposal("participant1"));
    }

    #[test]
    fn test_participation_validation() {
        let mut manager = create_test_manager();

        // Should not be able to vote without staking
        assert!(!manager.can_vote("participant1"));
        assert!(!manager.can_create_proposal("participant1"));

        // Should be able to vote after staking
        manager
            .stake("participant1".to_string(), 1000)
            .expect("Failed to stake");
        assert!(manager.can_vote("participant1"));
        // Note: With 1000 stake, should be able to create proposal (min is 100)
        assert!(manager.can_create_proposal("participant1"));

        // Should be able to create proposals with higher stake
        manager
            .stake("participant1".to_string(), 100)
            .expect("Failed to stake additional"); // Total 1100
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
            min_reputation_for_proposal: 80,
            malicious_proposal_reputation_penalty: 20,
            honest_voting_reputation_reward: 2,
            reputation_recovery_cooldown_seconds: 86400 * 30,
            min_stake_time_lock_seconds: 86400 * 30,
            max_voting_power_per_region_percent: 30,
        };

        let result = EconomicManager::new(invalid_config);
        assert!(result.is_err());

        // Should accept valid config
        let valid_config = EconomicConfig::default();
        let result = EconomicManager::new(valid_config);
        assert!(result.is_ok());
    }


}
