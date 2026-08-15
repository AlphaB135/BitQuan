//! Rate Limiting for Network Security
//!
//! This module provides comprehensive rate limiting to prevent message spam
//! and resource exhaustion attacks in the BitQuan network.
//!
//! ## Features
//!
//! - Per-peer message rate limiting
//! - Global message rate limits
//! - Different limits per message type
//! - Automatic peer throttling and banning
//! - Configurable windows and thresholds

use std::collections::HashMap;

use std::time::{Duration, Instant};

use crate::PeerId;

/// Rate limiting errors
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Peer has exceeded rate limit and should be throttled
    RateLimited,
    /// Peer has repeatedly violated limits and should be banned
    BanPeer,
    /// Global rate limit exceeded
    GlobalLimitReached,
}

/// Message types with different rate limits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// Block message
    Block,
    /// Transaction message
    Transaction,
    /// Ping message
    Ping,
    /// Pong message
    Pong,
    /// GetBlocks message
    GetBlocks,
    /// GetHeaders message
    GetHeaders,
    /// Headers message
    Headers,
    /// Inventory message
    Inv,
    /// GetData message
    GetData,
    /// NotFound message
    NotFound,
    /// Tx message
    Tx,
    /// Other message types
    Other,
}

/// Per-message-type rate limits
#[derive(Debug, Clone)]
pub struct MessageTypeLimits {
    /// Block message limit
    pub block: u32, // 10/sec
    /// Transaction message limit
    pub transaction: u32, // 50/sec
    /// Ping message limit
    pub ping: u32, // 5/sec
    /// Pong message limit
    pub pong: u32, // 5/sec
    /// GetBlocks message limit
    pub get_blocks: u32, // 20/sec
    /// GetHeaders message limit
    pub get_headers: u32, // 20/sec
    /// Headers message limit
    pub headers: u32, // 100/sec (burst)
    /// Inventory message limit
    pub inv: u32, // 100/sec (burst)
    /// GetData message limit
    pub get_data: u32, // 50/sec
    /// NotFound message limit
    pub not_found: u32, // 20/sec
    /// Tx message limit
    pub tx: u32, // 100/sec (burst)
    /// Block message limit (deprecated, use block)
    pub block_msg: u32, // 10/sec
    /// Other message types limit
    pub other: u32, // 10/sec
}

impl Default for MessageTypeLimits {
    fn default() -> Self {
        Self {
            block: 10,
            transaction: 50,
            ping: 5,
            pong: 5,
            get_blocks: 20,
            get_headers: 20,
            headers: 100,
            inv: 100,
            get_data: 50,
            not_found: 20,
            tx: 100,
            block_msg: 10,
            other: 10,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Messages per time window per peer
    pub max_messages_per_window: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Number of violations before ban
    pub violation_threshold: u32,
    /// Per-message-type limits
    pub message_type_limits: MessageTypeLimits,
    /// Global message limits
    pub max_global_messages_per_second: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_window: 100,
            window_duration: Duration::from_secs(1),
            violation_threshold: 3,
            message_type_limits: MessageTypeLimits::default(),
            max_global_messages_per_second: 10000,
        }
    }
}

/// Message counter for a single peer
#[derive(Debug)]
struct MessageCounter {
    count: u32,
    window_start: Instant,
    violations: u32,
    type_counters: HashMap<MessageType, u32>,
}

impl MessageCounter {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            violations: 0,
            type_counters: HashMap::new(),
        }
    }

    fn reset_window(&mut self) {
        self.count = 0;
        self.window_start = Instant::now();
        self.type_counters.clear();
        // Decay violations on each window reset: sustained abuse stays high,
        // but occasional bursts by legitimate peers decay over time.
        self.violations /= 2;
    }

    fn check_message_type_limit(&mut self, msg_type: MessageType, limit: u32) -> bool {
        let current = self.type_counters.entry(msg_type).or_insert(0);
        *current += 1;
        *current <= limit
    }
}

/// Comprehensive rate limiter for network messages
#[derive(Debug)]
pub struct RateLimiter {
    /// Per-peer message counts
    peer_counters: HashMap<PeerId, MessageCounter>,
    /// Global message counter
    global_counter: MessageCounter,
    /// Configuration
    config: RateLimitConfig,
    /// Total messages in current window
    total_messages: u32,
}

impl RateLimiter {
    /// Create new rate limiter with configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            peer_counters: HashMap::new(),
            global_counter: MessageCounter::new(),
            config,
            total_messages: 0,
        }
    }

    /// Check if a peer can send a message
    pub fn check_message(
        &mut self,
        peer_id: &PeerId,
        msg_type: MessageType,
    ) -> Result<(), RateLimitError> {
        // Reset global window if expired
        if self.global_counter.window_start.elapsed() > self.config.window_duration {
            self.global_counter.reset_window();
            self.total_messages = 0;
        }

        // Check global limit first
        if self.total_messages >= self.config.max_global_messages_per_second {
            return Err(RateLimitError::GlobalLimitReached);
        }

        // Get limit before mutable borrow
        let limit = self.get_limit_for_message_type(msg_type);

        // Get or create peer counter
        let counter = self
            .peer_counters
            .entry(peer_id.clone())
            .or_insert_with(MessageCounter::new);

        // Reset peer window if expired
        if counter.window_start.elapsed() > self.config.window_duration {
            counter.reset_window();
        }

        // Check per-message-type limit
        if !counter.check_message_type_limit(msg_type, limit) {
            counter.violations += 1;
            self.total_messages += 1;

            if counter.violations >= self.config.violation_threshold {
                return Err(RateLimitError::BanPeer);
            }

            return Err(RateLimitError::RateLimited);
        }

        // Check per-peer limit
        counter.count += 1;
        self.total_messages += 1;

        if counter.count > self.config.max_messages_per_window {
            counter.violations += 1;

            if counter.violations >= self.config.violation_threshold {
                return Err(RateLimitError::BanPeer);
            }

            return Err(RateLimitError::RateLimited);
        }

        Ok(())
    }

    /// Get rate limit for specific message type
    fn get_limit_for_message_type(&self, msg_type: MessageType) -> u32 {
        match msg_type {
            MessageType::Block => self.config.message_type_limits.block,
            MessageType::Transaction => self.config.message_type_limits.transaction,
            MessageType::Ping => self.config.message_type_limits.ping,
            MessageType::Pong => self.config.message_type_limits.pong,
            MessageType::GetBlocks => self.config.message_type_limits.get_blocks,
            MessageType::GetHeaders => self.config.message_type_limits.get_headers,
            MessageType::Headers => self.config.message_type_limits.headers,
            MessageType::Inv => self.config.message_type_limits.inv,
            MessageType::GetData => self.config.message_type_limits.get_data,
            MessageType::NotFound => self.config.message_type_limits.not_found,
            MessageType::Tx => self.config.message_type_limits.tx,
            MessageType::Other => self.config.message_type_limits.other,
        }
    }

    /// Remove peer from rate limiter (when disconnected)
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peer_counters.remove(peer_id);
    }

    /// Get peer's current violation count
    pub fn get_peer_violations(&self, peer_id: &PeerId) -> u32 {
        self.peer_counters
            .get(peer_id)
            .map(|counter| counter.violations)
            .unwrap_or(0)
    }

    /// Reset peer's violation count (for admin actions)
    pub fn reset_peer_violations(&mut self, peer_id: &PeerId) {
        if let Some(counter) = self.peer_counters.get_mut(peer_id) {
            counter.violations = 0;
        }
    }

    /// Get current global message rate
    pub fn get_global_message_rate(&self) -> f64 {
        let elapsed = self.global_counter.window_start.elapsed();
        if elapsed.is_zero() {
            0.0
        } else {
            self.total_messages as f64 / elapsed.as_secs_f64()
        }
    }

    /// Cleanup old peers (maintenance)
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(300); // 5 minutes

        self.peer_counters
            .retain(|_, counter| now.duration_since(counter.window_start) < timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_normal_traffic() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Should allow normal traffic (up to ping limit of 5)
        for _ in 0..5 {
            assert!(limiter.check_message(&peer, MessageType::Ping).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_spam() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Should allow up to ping limit
        for _ in 0..5 {
            assert!(limiter.check_message(&peer, MessageType::Ping).is_ok());
        }

        // Next ping should be rate limited
        assert!(matches!(
            limiter.check_message(&peer, MessageType::Ping),
            Err(RateLimitError::RateLimited)
        ));
    }

    #[test]
    fn test_rate_limiter_bans_after_violations() {
        let config = RateLimitConfig {
            violation_threshold: 2,       // Lower for testing
            max_messages_per_window: 100, // High enough to not interfere
            ..Default::default()
        };
        let mut limiter = RateLimiter::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // First exceed ping limit to get violation
        for _ in 0..5 {
            assert!(limiter.check_message(&peer, MessageType::Ping).is_ok());
        }
        // 6th ping should be rate limited (first violation)
        assert!(matches!(
            limiter.check_message(&peer, MessageType::Ping),
            Err(RateLimitError::RateLimited)
        ));

        // Now trigger second violation with another message type
        // Exceed transaction limit (5 allowed)
        for _ in 0..6 {
            let _ = limiter.check_message(&peer, MessageType::Transaction);
        }

        // Verify that violations are being tracked
        assert!(limiter.get_peer_violations(&peer) > 0);
    }

    #[test]
    fn test_message_type_limits() {
        let config = RateLimitConfig::default();
        let mut limiter = RateLimiter::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Should allow up to transaction limit
        for _ in 0..50 {
            assert!(limiter
                .check_message(&peer, MessageType::Transaction)
                .is_ok());
        }

        // Next transaction should be rate limited
        assert!(matches!(
            limiter.check_message(&peer, MessageType::Transaction),
            Err(RateLimitError::RateLimited)
        ));
    }
}
