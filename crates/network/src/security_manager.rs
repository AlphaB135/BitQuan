//! Security Integration Module
//!
//! This module provides the main integration point for all network
//! security features, offering a unified interface for managing
//! rate limiting, connections, reputation, bans, and DoS protection.
//!
//! ## Features
//!
//! - Unified security management
//! - Coordinated response to attacks
//! - Centralized configuration
//! - Comprehensive monitoring
//! - Event-driven architecture
//! - Performance optimized

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::{
    ban_manager::{BanError, BanManager, BanReason, BanStats},
    connection_manager::{ConnectionError, ConnectionManager, ConnectionStats},
    dos_protection::{AttackInfo, DoSError, DoSProtection, DoSStats},
    rate_limiter::{MessageType, RateLimitError, RateLimiter},
    reputation::{ReputationAction, ReputationManager, ReputationStats, Violation},
    security_config::{SecurityConfig, SecurityLevel},
};
use crate::PeerId;

/// Security management errors
#[derive(Debug, Clone)]
pub enum SecurityError {
    /// Rate limiting error
    RateLimit(RateLimitError),
    /// Connection management error
    Connection(ConnectionError),
    /// Reputation management error
    Reputation(String),
    /// Ban management error
    Ban(BanError),
    /// DoS protection error
    DoS(DoSError),
    /// Configuration error
    Configuration(String),
}

impl From<RateLimitError> for SecurityError {
    fn from(error: RateLimitError) -> Self {
        SecurityError::RateLimit(error)
    }
}

impl From<ConnectionError> for SecurityError {
    fn from(error: ConnectionError) -> Self {
        SecurityError::Connection(error)
    }
}

impl From<BanError> for SecurityError {
    fn from(error: BanError) -> Self {
        SecurityError::Ban(error)
    }
}

impl From<DoSError> for SecurityError {
    fn from(error: DoSError) -> Self {
        SecurityError::DoS(error)
    }
}

/// Security events
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    /// Peer violated rate limits
    RateLimitViolation {
        /// The peer ID
        peer_id: PeerId,
        /// The type of message
        message_type: MessageType,
        /// Number of violations
        violation_count: u32,
    },
    /// Peer connection state changed
    ConnectionStateChanged {
        /// The peer ID
        peer_id: PeerId,
        /// Previous state
        old_state: String,
        /// New state
        new_state: String,
    },
    /// Peer reputation changed
    ReputationChanged {
        /// The peer ID
        peer_id: PeerId,
        /// Previous score
        old_score: i32,
        /// New score
        new_score: i32,
        /// Action taken
        action: ReputationAction,
    },
    /// Peer was banned
    PeerBanned {
        /// The peer ID
        peer_id: PeerId,
        /// Reason for ban
        reason: BanReason,
        /// Duration of ban
        duration: Option<Duration>,
        /// Whether ban is permanent
        is_permanent: bool,
    },
    /// Peer was unbanned
    PeerUnbanned {
        /// The peer ID
        peer_id: PeerId,
    },
    /// Attack detected
    AttackDetected {
        /// Information about the attack
        attack: AttackInfo,
    },
    /// Security statistics updated
    StatisticsUpdated {
        /// Update interval
        interval: Duration,
    },
}

/// Security statistics
#[derive(Debug, Clone, Default)]
pub struct SecurityStatistics {
    /// Rate limiting statistics
    pub rate_limiting: RateLimitStats,
    /// Connection statistics
    pub connections: ConnectionStats,
    /// Reputation statistics
    pub reputation: ReputationStats,
    /// Ban statistics
    pub bans: BanStats,
    /// DoS protection statistics
    pub dos_protection: DoSStats,
    /// Overall security score
    pub security_score: f64,
}

/// Rate limiting statistics
#[derive(Debug, Clone, Default)]
pub struct RateLimitStats {
    /// Total messages processed
    pub total_messages: u64,
    /// Total messages rejected due to rate limiting
    pub rejected_messages: u64,
    /// Number of peers currently rate limited
    pub rate_limited_peers: usize,
    /// Number of peers currently banned via rate limiting
    pub banned_peers: usize,
}

/// Comprehensive security manager
pub struct SecurityManager {
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Connection manager
    connection_manager: ConnectionManager,
    /// Reputation manager
    reputation_manager: ReputationManager,
    /// Ban manager
    ban_manager: BanManager,
    /// DoS protection
    dos_protection: DoSProtection,
    /// Configuration
    config: SecurityConfig,
    /// Event handlers
    event_handlers: Vec<Box<dyn SecurityEventHandler>>,
    /// Statistics
    stats: Arc<RwLock<SecurityStatistics>>,
    /// Last cleanup time
    last_cleanup: Instant,
}

/// Security event handler trait
///
/// -- Linus: Changed to take &SecurityEvent instead of SecurityEvent.
/// This avoids cloning the event for every handler in emit_event().
/// If a handler needs to store the event, IT can clone it.
pub trait SecurityEventHandler: Send + Sync {
    /// Handle a security event (by reference to avoid cloning)
    fn handle_security_event(&self, event: &SecurityEvent);
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            rate_limiter: RateLimiter::new(config.rate_limiting.clone()),
            connection_manager: ConnectionManager::new(config.connections.clone()),
            reputation_manager: ReputationManager::new(config.reputation.clone()),
            ban_manager: BanManager::new(config.bans.clone()),
            dos_protection: DoSProtection::new(config.dos_protection.clone()),
            config,
            event_handlers: Vec::new(),
            stats: Arc::new(RwLock::new(SecurityStatistics::default())),
            last_cleanup: Instant::now(),
        }
    }

    /// Process incoming message
    pub fn process_message(
        &mut self,
        peer_id: &PeerId,
        message_type: MessageType,
    ) -> Result<(), SecurityError> {
        // Check rate limits first
        self.rate_limiter
            .check_message(peer_id, message_type)
            .map_err(SecurityError::RateLimit)?;

        // Check DoS protection
        self.dos_protection
            .analyze_pattern(peer_id.clone(), 1, 0)
            .map_err(SecurityError::DoS)?;

        Ok(())
    }

    /// Handle new connection attempt
    pub fn handle_connection_attempt(
        &mut self,
        peer_id: PeerId,
        ip: std::net::IpAddr,
        user_agent: Option<String>,
        is_inbound: bool,
    ) -> Result<(), SecurityError> {
        // Check SYN flood protection
        if is_inbound {
            if let Some(syn_cookie) = self
                .dos_protection
                .handle_syn_packet(ip, Some(peer_id.clone()))?
            {
                if !self
                    .dos_protection
                    .validate_syn_cookie(ip, syn_cookie.value())?
                {
                    return Err(SecurityError::DoS(DoSError::SynFlood));
                }
            } else {
                return Err(SecurityError::DoS(DoSError::SynFlood));
            }
        }

        // Check connection limits
        if is_inbound {
            self.connection_manager
                .accept_inbound_connection(peer_id.clone(), ip, user_agent)
                .map_err(SecurityError::Connection)?;
        } else {
            self.connection_manager
                .initiate_outbound_connection(peer_id.clone(), ip, user_agent)
                .map_err(SecurityError::Connection)?;
        }

        // Track connection for DoS detection
        self.dos_protection
            .track_connection(peer_id.clone(), ip)
            .map_err(SecurityError::DoS)?;

        Ok(())
    }

    /// Handle connection established
    pub fn handle_connection_established(&mut self, peer_id: PeerId) {
        // Update connection state
        if let Some(connection) = self.connection_manager.get_connection(&peer_id) {
            let old_state = format!("{:?}", connection.state);
            self.connection_manager.update_connection_state(
                &peer_id,
                super::connection_manager::ConnectionState::Connected,
            );

            self.emit_event(SecurityEvent::ConnectionStateChanged {
                peer_id: peer_id.clone(),
                old_state,
                new_state: "Connected".to_string(),
            });
        }

        // Report good behavior to reputation
        self.reputation_manager.report_good_behavior(&peer_id);
    }

    /// Handle connection closed
    pub fn handle_connection_closed(&mut self, peer_id: PeerId, _reason: Option<String>) {
        // Remove from connection manager
        self.connection_manager.remove_connection(&peer_id);

        // Remove from rate limiter
        self.rate_limiter.remove_peer(&peer_id);

        // Emit event
        self.emit_event(SecurityEvent::ConnectionStateChanged {
            peer_id: peer_id.clone(),
            old_state: "Connected".to_string(),
            new_state: "Disconnected".to_string(),
        });
    }

    /// Handle protocol violation
    pub fn handle_protocol_violation(
        &mut self,
        peer_id: PeerId,
        violation: Violation,
    ) -> ReputationAction {
        let action = self
            .reputation_manager
            .report_violation(&peer_id, violation);

        // Emit event
        self.emit_event(SecurityEvent::ReputationChanged {
            peer_id: peer_id.clone(),
            old_score: self.reputation_manager.get_score(&peer_id).unwrap_or(0),
            new_score: self.reputation_manager.get_score(&peer_id).unwrap_or(0),
            action: action.clone(),
        });

        action
    }

    /// Handle bandwidth usage
    pub fn handle_bandwidth_usage(
        &mut self,
        peer_id: PeerId,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Result<(), SecurityError> {
        // Track bandwidth
        self.dos_protection
            .track_bandwidth(peer_id.clone(), bytes_sent, bytes_received)
            .map_err(SecurityError::DoS)?;

        Ok(())
    }

    /// Ban a peer
    pub fn ban_peer(
        &mut self,
        peer_id: PeerId,
        reason: BanReason,
        duration: Option<Duration>,
        banned_by: Option<String>,
    ) -> Result<(), SecurityError> {
        // Ban in ban manager
        self.ban_manager
            .ban_peer(
                peer_id.clone(),
                reason.clone(),
                duration,
                banned_by.clone(),
                None,
            )
            .map_err(SecurityError::Ban)?;

        // Remove from connection manager
        self.connection_manager.remove_connection(&peer_id);

        // Remove from rate limiter
        self.rate_limiter.remove_peer(&peer_id);

        // Emit event
        self.emit_event(SecurityEvent::PeerBanned {
            peer_id: peer_id.clone(),
            reason: reason.clone(),
            duration,
            is_permanent: duration.is_none(),
        });

        Ok(())
    }

    /// Unban a peer
    pub fn unban_peer(&mut self, peer_id: PeerId) -> Result<(), SecurityError> {
        // Unban in ban manager
        self.ban_manager
            .unban_peer(&peer_id)
            .map_err(SecurityError::Ban)?;

        // Emit event
        self.emit_event(SecurityEvent::PeerUnbanned {
            peer_id: peer_id.clone(),
        });

        Ok(())
    }

    /// Check if peer is allowed to connect
    pub fn can_connect(&mut self, ip: &std::net::IpAddr) -> bool {
        // Check connection limits
        if !self.connection_manager.can_accept_connection(ip) {
            return false;
        }

        // Check if IP is banned
        if self.ban_manager.is_ip_banned(ip) {
            return false;
        }

        // Check DoS protection
        if self.dos_protection.is_ip_under_attack(ip) {
            return false;
        }

        true
    }

    /// Check if peer is banned
    pub fn is_peer_banned(&self, peer_id: &PeerId) -> bool {
        self.ban_manager.is_peer_banned(peer_id) || self.reputation_manager.is_banned(peer_id)
    }

    /// Check if IP is banned
    pub fn is_ip_banned(&self, ip: &std::net::IpAddr) -> bool {
        self.ban_manager.is_ip_banned(ip)
    }

    /// Get peer's current security status
    pub fn get_peer_security_status(&self, peer_id: &PeerId) -> PeerSecurityStatus {
        PeerSecurityStatus {
            is_banned: self.is_peer_banned(peer_id),
            is_throttled: self.reputation_manager.is_throttled(peer_id),
            reputation_score: self.reputation_manager.get_score(peer_id),
            violation_count: self.rate_limiter.get_peer_violations(peer_id),
            ban_duration_remaining: self.ban_manager.get_ban_duration_remaining(peer_id),
        }
    }

    /// Get comprehensive security statistics
    pub fn get_statistics(&self) -> SecurityStatistics {
        // Recover from poisoned lock if a thread panicked while holding it
        let mut stats = self.stats.write().unwrap_or_else(|e| e.into_inner());

        // Update statistics from all components
        stats.rate_limiting = RateLimitStats {
            total_messages: self.rate_limiter.get_global_message_rate() as u64,
            rejected_messages: 0, // Would need to track this
            rate_limited_peers: self.reputation_manager.get_throttled_peers().len(),
            banned_peers: self.ban_manager.get_banned_peers().count(),
        };

        stats.connections = self.connection_manager.get_stats().clone();
        stats.reputation = self.reputation_manager.get_stats().clone();
        stats.bans = self.ban_manager.get_stats().clone();
        stats.dos_protection = self.dos_protection.get_stats().clone();

        // Calculate overall security score
        stats.security_score = self.calculate_security_score(&stats);

        stats.clone()
    }

    /// Add event handler
    pub fn add_event_handler(&mut self, handler: Box<dyn SecurityEventHandler>) {
        self.event_handlers.push(handler);
    }

    /// Perform maintenance cleanup
    pub fn cleanup(&mut self) -> Vec<PeerId> {
        let now = Instant::now();

        // Only cleanup if enough time has passed
        if now.duration_since(self.last_cleanup) < Duration::from_secs(60) {
            return Vec::new();
        }

        let mut disconnected_peers = Vec::new();

        // Cleanup rate limiter
        self.rate_limiter.cleanup();

        // Cleanup connection manager
        let connection_disconnected = self.connection_manager.cleanup_connections();
        disconnected_peers.extend(connection_disconnected.clone());

        // Cleanup reputation manager
        self.reputation_manager.apply_decay();

        // Clear expired bans
        let bans_cleared = self.ban_manager.clear_expired_bans();

        // Cleanup DoS protection
        self.dos_protection.cleanup();

        // Update last cleanup time
        self.last_cleanup = now;

        // Emit statistics update event
        if bans_cleared > 0 || !connection_disconnected.is_empty() {
            self.emit_event(SecurityEvent::StatisticsUpdated {
                interval: Duration::from_secs(60),
            });
        }

        disconnected_peers
    }

    /// Emit security event to all handlers
    ///
    /// -- Linus: Now passes &event instead of event.clone() per handler.
    /// Before: O(n) clones where n = number of handlers
    /// After:  0 clones
    fn emit_event(&self, event: SecurityEvent) {
        for handler in &self.event_handlers {
            handler.handle_security_event(&event);
        }
    }

    /// Calculate overall security score
    fn calculate_security_score(&self, stats: &SecurityStatistics) -> f64 {
        let base_score = match self.config.global.security_level {
            SecurityLevel::Minimal => 20.0,
            SecurityLevel::Standard => 50.0,
            SecurityLevel::High => 80.0,
            SecurityLevel::Maximum => 95.0,
        };

        // Adjust based on current threats
        let threat_penalty = (stats.dos_protection.total_attacks_detected as f64 * 5.0)
            + (stats.bans.temporary_bans as f64 * 2.0)
            + (stats.bans.permanent_bans as f64 * 10.0);

        let reputation_factor = if stats.reputation.total_peers > 0 {
            stats.reputation.average_score / 100.0
        } else {
            0.0
        };

        (base_score - threat_penalty).max(0.0) + reputation_factor
    }

    /// Get security configuration summary
    pub fn get_config_summary(&self) -> String {
        self.config.get_summary()
    }

    /// Export security state
    pub fn export_security_state(&self) -> String {
        let stats = self.get_statistics();

        format!(
            "=========================\n\
             Security Report\n\
             =========================\n\
             Security Level: {:?}\n\
             Overall Score: {:.1}\n\
             \n\
             Rate Limiting:\n\
             - Total Messages: {}\n\
             - Rejected: {}\n\
             - Rate Limited Peers: {}\n\
             \n\
             Connections:\n\
             - Total: {}\n\
             - Current: {}\n\
             - Inbound: {}\n\
             - Outbound: {}\n\
             \n\
             Reputation:\n\
             - Total Peers: {}\n\
             - Average Score: {:.1}\n\
             \n\
             Bans:\n\
             - Total Bans: {}\n\
             - Temporary: {}\n\
             - Permanent: {}\n\
             \n\
             DoS Protection:\n\
             - SYN Floods: {}\n\
             - Connection Floods: {}\n\
             - Bandwidth Attacks: {}\n\
             - Pattern Attacks: {}\n\
             - Total Attacks: {}\n\
             \n\
             =========================\n",
            self.config.global.security_level,
            stats.security_score,
            stats.rate_limiting.total_messages,
            stats.rate_limiting.rejected_messages,
            stats.rate_limiting.rate_limited_peers,
            stats.connections.total_connections,
            stats.connections.current_connections,
            stats.connections.inbound_connections,
            stats.connections.outbound_connections,
            stats.reputation.total_peers,
            stats.reputation.average_score,
            stats.bans.total_bans,
            stats.bans.temporary_bans,
            stats.bans.permanent_bans,
            stats.dos_protection.syn_floods_detected,
            stats.dos_protection.connection_floods_detected,
            stats.dos_protection.bandwidth_attacks_detected,
            stats.dos_protection.pattern_attacks_detected,
            stats.dos_protection.total_attacks_detected
        )
    }
}

/// Peer security status
#[derive(Debug, Clone)]
pub struct PeerSecurityStatus {
    /// Whether peer is currently banned
    pub is_banned: bool,
    /// Whether peer is currently throttled
    pub is_throttled: bool,
    /// Current reputation score
    pub reputation_score: Option<i32>,
    /// Number of violations
    pub violation_count: u32,
    /// Time remaining on ban (if any)
    pub ban_duration_remaining: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_integration() {
        let config = SecurityConfig::for_security_level(SecurityLevel::Standard);
        let mut security = SecurityManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());
        let ip = "127.0.0.1".parse().expect("Invalid IP address in test");

        // Should allow connection
        assert!(security
            .handle_connection_attempt(peer.clone(), ip, None, true)
            .is_ok());
        assert!(security.can_connect(&ip));

        // Handle connection established
        security.handle_connection_established(peer.clone());

        // Process messages (within rate limits)
        for _ in 0..5 {
            assert!(security.process_message(&peer, MessageType::Ping).is_ok());
        }

        // Check status
        let status = security.get_peer_security_status(&peer);
        assert!(!status.is_banned);
        assert!(!status.is_throttled);
        // Reputation score may or may not be set depending on implementation
        // (score is created on first violation, not on connection)
    }

    #[test]
    fn test_security_ban_flow() {
        let config = SecurityConfig::for_security_level(SecurityLevel::Standard);
        let mut security = SecurityManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());
        let ip = "127.0.0.1".parse().expect("Invalid IP address in test");

        // Ban peer directly
        assert!(security
            .ban_peer(
                peer.clone(),
                BanReason::ProtocolViolation,
                Some(Duration::from_secs(3600)),
                None
            )
            .is_ok());
        assert!(security.is_peer_banned(&peer));

        // Ban the IP directly to test can_connect
        security
            .ban_manager
            .ban_ip(
                ip,
                BanReason::ProtocolViolation,
                Some(Duration::from_secs(3600)),
                None,
                None,
            )
            .unwrap();

        // Should not be able to connect (IP is now banned)
        assert!(!security.can_connect(&ip));
    }

    #[test]
    fn test_security_statistics() {
        let config = SecurityConfig::for_security_level(SecurityLevel::Standard);
        let security = SecurityManager::new(config);

        let stats = security.get_statistics();
        assert!(stats.security_score > 0.0);
        assert_eq!(stats.connections.current_connections, 0);
        assert_eq!(stats.reputation.total_peers, 0);
    }

    #[test]
    fn test_security_cleanup() {
        let config = SecurityConfig::for_security_level(SecurityLevel::Standard);
        let mut security = SecurityManager::new(config);

        // Initial cleanup should do nothing
        let disconnected = security.cleanup();
        assert!(disconnected.is_empty());
    }
}
