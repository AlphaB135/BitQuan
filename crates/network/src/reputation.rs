//! Peer Reputation Management for Network Security
//!
//! This module provides comprehensive peer reputation tracking to identify
//! and ban malicious peers in the BitQuan network.
//!
//! ## Features
//!
//! - Peer behavior scoring
//! - Violation tracking
//! - Automatic reputation decay
//! - Temporary and permanent bans
//! - Configurable thresholds

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::PeerId;

/// Reputation management actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReputationAction {
    /// No action required
    None,
    /// Throttle peer (reduce rate limits)
    Throttle,
    /// Temporarily ban peer
    TemporaryBan(Duration),
    /// Permanently ban peer
    PermanentBan,
}

/// Reputation violation types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Violation {
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Invalid message format
    InvalidMessage,
    /// Protocol violation
    ProtocolViolation,
    /// Double spend attempt
    DoubleSpend,
    /// Spam transaction
    SpamTransaction,
    /// Connection timeout
    Timeout,
    /// Invalid block
    InvalidBlock,
    /// Fork attempt
    ForkAttempt,
    /// Sybil attack behavior
    SybilBehavior,
    /// Eclipse attack behavior
    EclipseBehavior,
}

/// Peer reputation information
#[derive(Debug, Clone)]
pub struct PeerReputation {
    /// Current reputation score
    pub score: i32,
    /// List of violations
    pub violations: Vec<Violation>,
    /// Last time reputation was updated
    pub last_updated: Instant,
    /// Number of good actions
    pub good_actions: u32,
    /// Number of bad actions
    pub bad_actions: u32,
    /// Peer is currently throttled
    pub is_throttled: bool,
    /// Peer is currently banned
    pub is_banned: bool,
    /// Ban expiration time
    pub ban_expires: Option<Instant>,
}

/// Reputation management configuration
#[derive(Debug, Clone)]
pub struct ReputationConfig {
    /// Initial reputation score for new peers
    pub initial_score: i32,
    /// Minimum score before banning
    pub min_score: i32,
    /// Maximum score
    pub max_score: i32,
    /// Score threshold for temporary ban
    pub temp_ban_threshold: i32,
    /// Score threshold for permanent ban
    pub perm_ban_threshold: i32,
    /// Reputation decay rate (points per hour)
    pub decay_rate: i32,
    /// Good action reward
    pub good_action_reward: u32,
    /// Violation penalties by type
    pub violation_penalties: HashMap<Violation, i32>,
    /// Temporary ban duration
    pub temp_ban_duration: Duration,
    /// Score for throttling
    pub throttle_threshold: i32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        let mut penalties = HashMap::new();
        penalties.insert(Violation::RateLimitExceeded, -10);
        penalties.insert(Violation::InvalidMessage, -20);
        penalties.insert(Violation::ProtocolViolation, -50);
        penalties.insert(Violation::DoubleSpend, -100);
        penalties.insert(Violation::SpamTransaction, -30);
        penalties.insert(Violation::Timeout, -5);
        penalties.insert(Violation::InvalidBlock, -40);
        penalties.insert(Violation::ForkAttempt, -80);
        penalties.insert(Violation::SybilBehavior, -60);
        penalties.insert(Violation::EclipseBehavior, -90);

        Self {
            initial_score: 50,
            min_score: -100,
            max_score: 100,
            temp_ban_threshold: -30,
            perm_ban_threshold: -50,
            decay_rate: 1,
            good_action_reward: 1,
            violation_penalties: penalties,
            temp_ban_duration: Duration::from_secs(3600), // 1 hour
            throttle_threshold: 0,
        }
    }
}

/// Comprehensive peer reputation manager
#[derive(Debug)]
pub struct ReputationManager {
    /// Peer reputation scores
    reputations: HashMap<PeerId, PeerReputation>,
    /// Configuration
    config: ReputationConfig,
    /// Statistics
    stats: ReputationStats,
}

/// Reputation statistics
#[derive(Debug, Clone, Default)]
pub struct ReputationStats {
    /// Total number of tracked peers
    pub total_peers: usize,
    /// Number of banned peers
    pub banned_peers: usize,
    /// Number of throttled peers
    pub throttled_peers: usize,
    /// Average reputation score
    pub average_score: f64,
    /// Total number of violations recorded
    pub total_violations: u64,
}

impl ReputationManager {
    /// Create new reputation manager
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            reputations: HashMap::new(),
            config,
            stats: ReputationStats::default(),
        }
    }

    /// Report a violation for a peer
    pub fn report_violation(&mut self, peer_id: &PeerId, violation: Violation) -> ReputationAction {
        let rep = self
            .reputations
            .entry(peer_id.clone())
            .or_insert_with(|| PeerReputation {
                score: self.config.initial_score,
                violations: Vec::new(),
                last_updated: Instant::now(),
                good_actions: 0,
                bad_actions: 0,
                is_throttled: false,
                is_banned: false,
                ban_expires: None,
            });

        // Apply penalty
        let penalty = self
            .config
            .violation_penalties
            .get(&violation)
            .copied()
            .unwrap_or(-10); // Default penalty

        rep.score = (rep.score + penalty).max(self.config.min_score);
        rep.violations.push(violation);
        rep.bad_actions += 1;
        rep.last_updated = Instant::now();

        // Update statistics
        self.stats.total_violations += 1;

        // Determine action based on new score
        if rep.score <= self.config.perm_ban_threshold {
            rep.is_banned = true;
            rep.ban_expires = None; // Permanent ban
            self.stats.banned_peers += 1;
            ReputationAction::PermanentBan
        } else if rep.score <= self.config.temp_ban_threshold {
            rep.is_banned = true;
            rep.ban_expires = Some(Instant::now() + self.config.temp_ban_duration);
            self.stats.banned_peers += 1;
            ReputationAction::TemporaryBan(self.config.temp_ban_duration)
        } else if rep.score <= self.config.throttle_threshold {
            rep.is_throttled = true;
            self.stats.throttled_peers += 1;
            ReputationAction::Throttle
        } else {
            ReputationAction::None
        }
    }

    /// Report good behavior for a peer
    pub fn report_good_behavior(&mut self, peer_id: &PeerId) {
        if let Some(rep) = self.reputations.get_mut(peer_id) {
            rep.score =
                (rep.score + self.config.good_action_reward as i32).min(self.config.max_score);
            rep.good_actions += 1;
            rep.last_updated = Instant::now();

            // Remove throttling if score improves
            if rep.is_throttled && rep.score > self.config.throttle_threshold {
                rep.is_throttled = false;
            }
        }
    }

    /// Get peer's current reputation
    pub fn get_reputation(&self, peer_id: &PeerId) -> Option<&PeerReputation> {
        self.reputations.get(peer_id)
    }

    /// Check if peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.reputations
            .get(peer_id)
            .map(|rep| {
                if rep.is_banned {
                    if let Some(expires) = rep.ban_expires {
                        Instant::now() < expires
                    } else {
                        true // Permanent ban
                    }
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Check if peer is throttled
    pub fn is_throttled(&self, peer_id: &PeerId) -> bool {
        self.reputations
            .get(peer_id)
            .map(|rep| rep.is_throttled)
            .unwrap_or(false)
    }

    /// Get peer's current score
    pub fn get_score(&self, peer_id: &PeerId) -> Option<i32> {
        self.reputations.get(peer_id).map(|rep| rep.score)
    }

    /// Remove peer from reputation tracking
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.reputations.remove(peer_id);
        self.update_stats();
    }

    /// Apply reputation decay (periodic maintenance)
    pub fn apply_decay(&mut self) {
        let now = Instant::now();
        let decay_duration = Duration::from_secs(3600); // 1 hour

        for rep in self.reputations.values_mut() {
            if now.duration_since(rep.last_updated) >= decay_duration {
                // Decay score towards initial score
                if rep.score > self.config.initial_score {
                    rep.score = (rep.score - self.config.decay_rate).max(self.config.initial_score);
                } else if rep.score < self.config.initial_score {
                    rep.score = (rep.score + self.config.decay_rate).min(self.config.initial_score);
                }
                rep.last_updated = now;
            }
        }
    }

    /// Clear expired temporary bans
    pub fn clear_expired_bans(&mut self) -> usize {
        let now = Instant::now();
        let mut cleared = 0;

        for rep in self.reputations.values_mut() {
            if let Some(expires) = rep.ban_expires {
                if now >= expires {
                    rep.is_banned = false;
                    rep.ban_expires = None;
                    cleared += 1;
                }
            }
        }

        cleared
    }

    /// Get reputation statistics
    pub fn get_stats(&self) -> &ReputationStats {
        &self.stats
    }

    /// Update internal statistics
    fn update_stats(&mut self) {
        self.stats.total_peers = self.reputations.len();

        if self.stats.total_peers > 0 {
            let total_score: i64 = self.reputations.values().map(|rep| rep.score as i64).sum();

            self.stats.average_score = total_score as f64 / self.stats.total_peers as f64;
        }
    }

    /// Get peers with specific reputation range
    pub fn get_peers_by_score_range(&self, min: i32, max: i32) -> Vec<PeerId> {
        self.reputations
            .iter()
            .filter(|(_, rep)| rep.score >= min && rep.score <= max)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }

    /// Get banned peers
    pub fn get_banned_peers(&self) -> Vec<PeerId> {
        self.reputations
            .iter()
            .filter(|(_, rep)| rep.is_banned)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }

    /// Get throttled peers
    pub fn get_throttled_peers(&self) -> Vec<PeerId> {
        self.reputations
            .iter()
            .filter(|(_, rep)| rep.is_throttled)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }

    /// Manually ban a peer
    pub fn ban_peer(&mut self, peer_id: &PeerId, _reason: String, duration: Option<Duration>) {
        if let Some(rep) = self.reputations.get_mut(peer_id) {
            rep.is_banned = true;
            rep.ban_expires = duration.map(|d| Instant::now() + d);
            rep.violations.push(Violation::ProtocolViolation); // Admin action
            rep.bad_actions += 1;
            rep.last_updated = Instant::now();

            self.stats.banned_peers += 1;
        }
    }

    /// Manually unban a peer
    pub fn unban_peer(&mut self, peer_id: &PeerId) {
        if let Some(rep) = self.reputations.get_mut(peer_id) {
            rep.is_banned = false;
            rep.ban_expires = None;
            rep.last_updated = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_initial_score() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        let score = manager.get_score(&peer);
        // New peer has no score until first interaction (get_score doesn't create entry)
        assert_eq!(score, None);
    }

    #[test]
    fn test_reputation_violation_penalties() {
        let config = ReputationConfig::default();
        let mut manager = ReputationManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Report rate limit violation
        let action = manager.report_violation(&peer, Violation::RateLimitExceeded);
        assert_eq!(action, ReputationAction::None); // Score still above threshold

        let score = manager.get_score(&peer);
        assert_eq!(score, Some(40)); // 50 - 10
    }

    #[test]
    fn test_reputation_banning() {
        let config = ReputationConfig::default();
        let mut manager = ReputationManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Report multiple violations to trigger ban
        for _ in 0..5 {
            manager.report_violation(&peer, Violation::ProtocolViolation);
        }

        let action = manager.report_violation(&peer, Violation::ProtocolViolation);
        assert!(matches!(action, ReputationAction::PermanentBan));

        assert!(manager.is_banned(&peer));
    }

    #[test]
    fn test_reputation_good_behavior() {
        let config = ReputationConfig::default();
        let mut manager = ReputationManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Report violation
        manager.report_violation(&peer, Violation::RateLimitExceeded);
        let score = manager.get_score(&peer);
        assert_eq!(score, Some(40));

        // Report good behavior
        manager.report_good_behavior(&peer);
        let score = manager.get_score(&peer);
        assert_eq!(score, Some(41)); // 40 + 1
    }

    #[test]
    fn test_reputation_decay() {
        let config = ReputationConfig {
            decay_rate: 10,
            ..Default::default()
        }; // Higher decay rate for testing
        let mut manager = ReputationManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Report violation
        manager.report_violation(&peer, Violation::ProtocolViolation);
        let score = manager.get_score(&peer);
        assert_eq!(score, Some(0)); // 50 - 50

        // Manually set last_updated to simulate time passing
        if let Some(rep) = manager.reputations.get_mut(&peer) {
            rep.last_updated = Instant::now() - Duration::from_secs(3601); // More than 1 hour ago
        }

        manager.apply_decay();
        let score = manager.get_score(&peer);
        // Score decays towards initial (50), so from 0 it increases by decay_rate (10)
        assert_eq!(score, Some(10));
    }

    #[test]
    fn test_temporary_ban_expiry() {
        let config = ReputationConfig::default();
        let mut manager = ReputationManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Trigger temporary ban (score between -30 and -50)
        // Initial score: 50, temp_ban_threshold: -30
        // Need to reach score <= -30 but > -50
        // Use InvalidMessage (-20 each): 50 - 20 - 20 - 20 - 20 = -30
        for _ in 0..4 {
            manager.report_violation(&peer, Violation::InvalidMessage); // -20 each
        }

        // One more violation to get into temporary ban range
        let action = manager.report_violation(&peer, Violation::RateLimitExceeded); // -10 more = -40 (temporary ban)
        assert!(matches!(action, ReputationAction::TemporaryBan(_)));

        assert!(manager.is_banned(&peer));

        // Manually unban to test
        manager.unban_peer(&peer);
        assert!(!manager.is_banned(&peer));
    }
}
