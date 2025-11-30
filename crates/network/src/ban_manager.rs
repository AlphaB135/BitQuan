//! Ban Management for Network Security
//!
//! This module provides comprehensive ban management to handle
//! temporary and permanent peer bans in BitQuan network.
//!
//! ## Features
//!
//! - Temporary and permanent bans
//! - Ban reason tracking
//! - Automatic ban expiration
//! - IP and peer ID banning
//! - Ban persistence
//! - Ban statistics

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use crate::PeerId;

/// Ban management errors
#[derive(Debug, Clone)]
pub enum BanError {
    /// Peer is already banned
    AlreadyBanned,
    /// Invalid ban duration
    InvalidDuration,
    /// Ban not found
    NotFound,
}

/// Ban information
#[derive(Debug, Clone)]
pub struct BanInfo {
    /// Ban reason
    pub reason: BanReason,
    /// When ban was issued
    pub banned_at: Instant,
    /// When ban expires (None for permanent)
    pub expires_at: Option<Instant>,
    /// Who issued the ban
    pub banned_by: Option<String>,
    /// Additional notes
    pub notes: Option<String>,
}

/// Ban reasons
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BanReason {
    /// Rate limit violations
    RateLimitViolation,
    /// Protocol violations
    ProtocolViolation,
    /// Invalid messages
    InvalidMessages,
    /// Connection abuse
    ConnectionAbuse,
    /// Spam behavior
    SpamBehavior,
    /// Attack behavior
    AttackBehavior,
    /// Manual ban by operator
    ManualBan(String),
    /// Suspicious activity
    SuspiciousActivity,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Sybil attack
    SybilAttack,
    /// Eclipse attack
    EclipseAttack,
}

/// Ban configuration
#[derive(Debug, Clone)]
pub struct BanConfig {
    /// Default temporary ban duration
    pub default_temp_ban_duration: Duration,
    /// Maximum temporary ban duration
    pub max_temp_ban_duration: Duration,
    /// Enable automatic ban expiration
    pub enable_expiration: bool,
    /// Maximum number of active bans
    pub max_active_bans: usize,
    /// Ban history retention period
    pub ban_history_retention: Duration,
    /// Enable ban persistence
    pub enable_persistence: bool,
}

impl Default for BanConfig {
    fn default() -> Self {
        Self {
            default_temp_ban_duration: Duration::from_secs(3600), // 1 hour
            max_temp_ban_duration: Duration::from_secs(86400),    // 24 hours
            enable_expiration: true,
            max_active_bans: 10000,
            ban_history_retention: Duration::from_secs(604800), // 7 days
            enable_persistence: true,
        }
    }
}

/// Ban statistics
#[derive(Debug, Clone, Default)]
pub struct BanStats {
    /// Total number of bans issued
    pub total_bans: u64,
    /// Number of currently active bans
    pub active_bans: usize,
    /// Number of temporary bans issued
    pub temporary_bans: u64,
    /// Number of permanent bans issued
    pub permanent_bans: u64,
    /// Number of expired bans
    pub expired_bans: u64,
    /// Count of bans by reason
    pub bans_by_reason: HashMap<BanReason, u64>,
}

/// Comprehensive ban manager
#[derive(Debug)]
pub struct BanManager {
    /// Banned peers by ID
    banned_peers: HashMap<PeerId, BanInfo>,
    /// Banned IPs
    banned_ips: HashMap<IpAddr, BanInfo>,
    /// Ban history (including expired)
    ban_history: Vec<BanInfo>,
    /// Configuration
    config: BanConfig,
    /// Statistics
    stats: BanStats,
}

impl BanManager {
    /// Create new ban manager
    pub fn new(config: BanConfig) -> Self {
        Self {
            banned_peers: HashMap::new(),
            banned_ips: HashMap::new(),
            ban_history: Vec::new(),
            config,
            stats: BanStats::default(),
        }
    }

    /// Ban a peer by ID
    pub fn ban_peer(
        &mut self,
        peer_id: PeerId,
        reason: BanReason,
        duration: Option<Duration>,
        banned_by: Option<String>,
        notes: Option<String>,
    ) -> Result<(), BanError> {
        // Check if already banned
        if self.banned_peers.contains_key(&peer_id) {
            return Err(BanError::AlreadyBanned);
        }

        // Check active ban limit
        if self.banned_peers.len() >= self.config.max_active_bans {
            return Err(BanError::InvalidDuration);
        }

        let expires_at = duration.map(|d| Instant::now() + d);
        let ban_info = BanInfo {
            reason: reason.clone(),
            banned_at: Instant::now(),
            expires_at,
            banned_by,
            notes,
        };

        self.banned_peers.insert(peer_id, ban_info.clone());
        self.ban_history.push(ban_info.clone());

        self.update_stats(&reason, expires_at.is_none());
        Ok(())
    }

    /// Ban an IP address
    pub fn ban_ip(
        &mut self,
        ip: IpAddr,
        reason: BanReason,
        duration: Option<Duration>,
        banned_by: Option<String>,
        notes: Option<String>,
    ) -> Result<(), BanError> {
        // Check if already banned
        if self.banned_ips.contains_key(&ip) {
            return Err(BanError::AlreadyBanned);
        }

        let expires_at = duration.map(|d| Instant::now() + d);
        let ban_info = BanInfo {
            reason: reason.clone(),
            banned_at: Instant::now(),
            expires_at,
            banned_by,
            notes,
        };

        self.banned_ips.insert(ip, ban_info.clone());
        self.ban_history.push(ban_info.clone());

        self.update_stats(&reason, expires_at.is_none());
        Ok(())
    }

    /// Ban peer with default duration
    pub fn ban_peer_temporarily(
        &mut self,
        peer_id: PeerId,
        reason: BanReason,
    ) -> Result<(), BanError> {
        self.ban_peer(
            peer_id,
            reason,
            Some(self.config.default_temp_ban_duration),
            None,
            None,
        )
    }

    /// Ban peer permanently
    pub fn ban_peer_permanently(
        &mut self,
        peer_id: PeerId,
        reason: BanReason,
        banned_by: Option<String>,
        notes: Option<String>,
    ) -> Result<(), BanError> {
        self.ban_peer(peer_id, reason, None, banned_by, notes)
    }

    /// Check if peer is banned
    pub fn is_peer_banned(&self, peer_id: &PeerId) -> bool {
        self.banned_peers
            .get(peer_id)
            .map(|ban| self.is_ban_active(ban))
            .unwrap_or(false)
    }

    /// Check if IP is banned
    pub fn is_ip_banned(&self, ip: &IpAddr) -> bool {
        self.banned_ips
            .get(ip)
            .map(|ban| self.is_ban_active(ban))
            .unwrap_or(false)
    }

    /// Get ban information for peer
    pub fn get_peer_ban_info(&self, peer_id: &PeerId) -> Option<&BanInfo> {
        self.banned_peers.get(peer_id)
    }

    /// Get ban information for IP
    pub fn get_ip_ban_info(&self, ip: &IpAddr) -> Option<&BanInfo> {
        self.banned_ips.get(ip)
    }

    /// Unban a peer
    pub fn unban_peer(&mut self, peer_id: &PeerId) -> Result<(), BanError> {
        if let Some(ban_info) = self.banned_peers.remove(peer_id) {
            self.stats.expired_bans += 1;
            log::info!("Unbanned peer {:?}: {:?}", peer_id, ban_info.reason);
            Ok(())
        } else {
            Err(BanError::NotFound)
        }
    }

    /// Unban an IP
    pub fn unban_ip(&mut self, ip: &IpAddr) -> Result<(), BanError> {
        if let Some(ban_info) = self.banned_ips.remove(ip) {
            self.stats.expired_bans += 1;
            log::info!("Unbanned IP {}: {:?}", ip, ban_info.reason);
            Ok(())
        } else {
            Err(BanError::NotFound)
        }
    }

    /// Clear expired bans
    pub fn clear_expired_bans(&mut self) -> usize {
        let now = Instant::now();
        let mut cleared = 0;

        // Clear expired peer bans
        self.banned_peers.retain(|_, ban| {
            if let Some(expires) = ban.expires_at {
                if now >= expires {
                    cleared += 1;
                    false
                } else {
                    true
                }
            } else {
                true // Permanent ban
            }
        });

        // Clear expired IP bans
        self.banned_ips.retain(|_, ban| {
            if let Some(expires) = ban.expires_at {
                if now >= expires {
                    cleared += 1;
                    false
                } else {
                    true
                }
            } else {
                true // Permanent ban
            }
        });

        self.stats.expired_bans += cleared;
        cleared as usize
    }

    /// Get all banned peers
    pub fn get_banned_peers(&self) -> impl Iterator<Item = (&PeerId, &BanInfo)> {
        self.banned_peers.iter()
    }

    /// Get all banned IPs
    pub fn get_banned_ips(&self) -> impl Iterator<Item = (&IpAddr, &BanInfo)> {
        self.banned_ips.iter()
    }

    /// Get ban statistics
    pub fn get_stats(&self) -> &BanStats {
        &self.stats
    }

    /// Get ban history
    pub fn get_ban_history(&self) -> &[BanInfo] {
        &self.ban_history
    }

    /// Clear old ban history
    pub fn cleanup_history(&mut self) {
        let cutoff = Instant::now() - self.config.ban_history_retention;
        self.ban_history.retain(|ban| ban.banned_at > cutoff);
    }

    /// Check if ban is currently active
    fn is_ban_active(&self, ban: &BanInfo) -> bool {
        if !self.config.enable_expiration {
            return !ban.expires_at.is_some(); // Permanent bans always active
        }

        match ban.expires_at {
            Some(expires) => Instant::now() < expires,
            None => true, // Permanent ban
        }
    }

    /// Update ban statistics
    fn update_stats(&mut self, reason: &BanReason, is_permanent: bool) {
        self.stats.total_bans += 1;
        self.stats.active_bans = self.banned_peers.len() + self.banned_ips.len();

        if is_permanent {
            self.stats.permanent_bans += 1;
        } else {
            self.stats.temporary_bans += 1;
        }

        *self.stats.bans_by_reason.entry(reason.clone()).or_insert(0) += 1;
    }

    /// Get ban duration remaining
    pub fn get_ban_duration_remaining(&self, peer_id: &PeerId) -> Option<Duration> {
        self.banned_peers.get(peer_id).and_then(|ban| {
            ban.expires_at.map(|expires| {
                let now = Instant::now();
                if expires > now {
                    expires.duration_since(now)
                } else {
                    Duration::ZERO
                }
            })
        })
    }

    /// Get bans by reason
    pub fn get_bans_by_reason(&self, reason: &BanReason) -> Vec<&BanInfo> {
        self.banned_peers
            .values()
            .filter(|ban| ban.reason == *reason)
            .collect()
    }

    /// Export bans to string format
    pub fn export_bans(&self) -> String {
        let mut output = String::new();

        output.push_str("# BitQuan Network Bans Export\n");
        output.push_str(&format!("Generated: {:?}\n\n", Instant::now()));

        for (peer_id, ban_info) in &self.banned_peers {
            output.push_str(&format!(
                "Peer: {:?}\n  Reason: {:?}\n  Banned: {:?}\n  Expires: {}\n  By: {:?}\n\n",
                peer_id,
                ban_info.reason,
                ban_info.banned_at,
                ban_info
                    .expires_at
                    .map(|e| format!("{:?}", e))
                    .unwrap_or_else(|| "Permanent".to_string()),
                ban_info.banned_by
            ));
        }

        for (ip, ban_info) in &self.banned_ips {
            output.push_str(&format!(
                "IP: {}\n  Reason: {:?}\n  Banned: {:?}\n  Expires: {}\n  By: {:?}\n\n",
                ip,
                ban_info.reason,
                ban_info.banned_at,
                ban_info
                    .expires_at
                    .map(|e| format!("{:?}", e))
                    .unwrap_or_else(|| "Permanent".to_string()),
                ban_info.banned_by
            ));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ban_manager_basic_operations() {
        let config = BanConfig::default();
        let mut manager = BanManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());
        let reason = BanReason::RateLimitViolation;

        // Should ban peer
        assert!(manager
            .ban_peer_temporarily(peer.clone(), reason.clone())
            .is_ok());
        assert!(manager.is_peer_banned(&peer));

        // Should get ban info
        let ban_info = manager.get_peer_ban_info(&peer);
        assert!(ban_info.is_some());
        assert_eq!(ban_info.unwrap().reason, reason);
    }

    #[test]
    fn test_ban_expiration() {
        let mut config = BanConfig::default();
        config.default_temp_ban_duration = Duration::from_millis(100);
        let mut manager = BanManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Ban with short duration
        assert!(manager
            .ban_peer_temporarily(peer.clone(), BanReason::ProtocolViolation)
            .is_ok());
        assert!(manager.is_peer_banned(&peer));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Clear expired bans
        let cleared = manager.clear_expired_bans();
        assert_eq!(cleared, 1);
        assert!(!manager.is_peer_banned(&peer));
    }

    #[test]
    fn test_permanent_ban() {
        let config = BanConfig::default();
        let mut manager = BanManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Permanent ban
        assert!(manager
            .ban_peer_permanently(peer.clone(), BanReason::AttackBehavior, None, None)
            .is_ok());
        assert!(manager.is_peer_banned(&peer));

        // Should not expire
        std::thread::sleep(Duration::from_millis(100));
        let cleared = manager.clear_expired_bans();
        assert_eq!(cleared, 0);
        assert!(manager.is_peer_banned(&peer));
    }

    #[test]
    fn test_ip_banning() {
        let config = BanConfig::default();
        let mut manager = BanManager::new(config);
        let ip = "192.168.1.100".parse().unwrap();

        // Ban IP
        assert!(manager
            .ban_ip(ip, BanReason::SpamBehavior, None, None, None)
            .is_ok());
        assert!(manager.is_ip_banned(&ip));

        // Should get ban info
        let ban_info = manager.get_ip_ban_info(&ip);
        assert!(ban_info.is_some());
        assert_eq!(ban_info.unwrap().reason, BanReason::SpamBehavior);
    }

    #[test]
    fn test_unban_operations() {
        let config = BanConfig::default();
        let mut manager = BanManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Ban peer
        assert!(manager
            .ban_peer_temporarily(peer.clone(), BanReason::ProtocolViolation)
            .is_ok());
        assert!(manager.is_peer_banned(&peer));

        // Unban peer
        assert!(manager.unban_peer(&peer).is_ok());
        assert!(!manager.is_peer_banned(&peer));

        // Unban should fail if not banned
        assert!(matches!(manager.unban_peer(&peer), Err(BanError::NotFound)));
    }

    #[test]
    fn test_ban_statistics() {
        let config = BanConfig::default();
        let mut manager = BanManager::new(config);
        let peer1 = format!("test_peer_{}", rand::random::<u64>());
        let peer2 = format!("test_peer_{}", rand::random::<u64>());

        // Create different types of bans
        assert!(manager
            .ban_peer_temporarily(peer1, BanReason::RateLimitViolation)
            .is_ok());
        assert!(manager
            .ban_peer_permanently(peer2, BanReason::AttackBehavior, None, None)
            .is_ok());

        let stats = manager.get_stats();
        assert_eq!(stats.total_bans, 2);
        assert_eq!(stats.active_bans, 2);
        assert_eq!(stats.temporary_bans, 1);
        assert_eq!(stats.permanent_bans, 1);
    }
}
