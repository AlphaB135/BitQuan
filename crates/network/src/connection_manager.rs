//! Connection Management for Network Security
//!
//! This module provides comprehensive connection management to prevent
//! resource exhaustion and enforce connection limits.
//!
//! ## Features
//!
//! - Global connection limits
//! - Per-IP connection limits
//! - Inbound/outbound connection separation
//! - Connection timeout management
//! - Idle connection cleanup
//! - Connection tracking and monitoring

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use crate::PeerId;

/// Connection management errors
#[derive(Debug, Clone)]
pub enum ConnectionError {
    /// Global connection limit reached
    GlobalLimitReached,
    /// Inbound connection limit reached
    InboundLimitReached,
    /// Outbound connection limit reached
    OutboundLimitReached,
    /// Per-IP connection limit reached
    IpLimitReached,
    /// Connection timeout
    Timeout,
    /// Invalid connection request
    InvalidRequest,
}

/// Connection direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Inbound connection
    Inbound,
    /// Outbound connection
    Outbound,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct Connection {
    /// Peer ID
    pub peer_id: PeerId,
    /// IP address
    pub ip: IpAddr,
    /// Connection direction
    pub direction: Direction,
    /// When connection was established
    pub connected_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// User agent (if provided)
    pub user_agent: Option<String>,
    /// Connection state
    pub state: ConnectionState,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    /// Connection is established
    Connected,
    /// Connection is being closed
    Disconnecting,
    /// Connection is closed
    Disconnected,
}

/// Connection management configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Global connection limits
    pub max_total_connections: usize,
    /// Maximum number of inbound connections
    pub max_inbound_connections: usize,
    /// Maximum number of outbound connections
    pub max_outbound_connections: usize,

    /// Per-IP limits
    pub max_connections_per_ip: usize,

    /// Timeouts
    pub connection_timeout: Duration,
    /// Idle timeout duration
    pub idle_timeout: Duration,

    /// Rate limiting
    pub enable_rate_limiting: bool,

    /// Security settings
    pub allow_only_outbound: bool,
    /// Maximum connection attempts per minute
    pub max_connection_attempts_per_minute: u32,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_total_connections: 125,
            max_inbound_connections: 100,
            max_outbound_connections: 25,
            max_connections_per_ip: 3,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            enable_rate_limiting: true,
            allow_only_outbound: false,
            max_connection_attempts_per_minute: 10,
        }
    }
}

/// Comprehensive connection manager
#[derive(Debug)]
pub struct ConnectionManager {
    /// Active connections by peer ID
    active_connections: HashMap<PeerId, Connection>,
    /// Connections grouped by IP address
    ip_connections: HashMap<IpAddr, Vec<PeerId>>,
    /// Connection attempts by IP (for rate limiting)
    connection_attempts: HashMap<IpAddr, Vec<Instant>>,
    /// Configuration
    config: ConnectionConfig,
    /// Statistics
    stats: ConnectionStats,
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total number of connections handled
    pub total_connections: u64,
    /// Number of inbound connections
    pub inbound_connections: u64,
    /// Number of outbound connections
    pub outbound_connections: u64,
    /// Number of rejected connections
    pub rejected_connections: u64,
    /// Number of timed out connections
    pub timed_out_connections: u64,
    /// Current number of active connections
    pub current_connections: usize,
    /// Number of unique IPs connected
    pub unique_ips: usize,
}

impl ConnectionManager {
    /// Create new connection manager
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            active_connections: HashMap::new(),
            ip_connections: HashMap::new(),
            connection_attempts: HashMap::new(),
            config,
            stats: ConnectionStats::default(),
        }
    }

    /// Accept a new inbound connection
    pub fn accept_inbound_connection(
        &mut self,
        peer_id: PeerId,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<(), ConnectionError> {
        // Check rate limiting on connection attempts
        if self.config.allow_only_outbound {
            return Err(ConnectionError::InvalidRequest);
        }

        if self.is_rate_limited(&ip) {
            return Err(ConnectionError::InvalidRequest);
        }

        // Check global limit
        if self.active_connections.len() >= self.config.max_total_connections {
            self.stats.rejected_connections += 1;
            return Err(ConnectionError::GlobalLimitReached);
        }

        // Check inbound limit
        let inbound_count = self.count_inbound_connections();
        if inbound_count >= self.config.max_inbound_connections {
            self.stats.rejected_connections += 1;
            return Err(ConnectionError::InboundLimitReached);
        }

        // Check per-IP limit
        let ip_count = self.count_connections_from_ip(&ip);
        if ip_count >= self.config.max_connections_per_ip {
            self.stats.rejected_connections += 1;
            return Err(ConnectionError::IpLimitReached);
        }

        // Accept connection
        let connection = Connection {
            peer_id: peer_id.clone(),
            ip,
            direction: Direction::Inbound,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            user_agent,
            state: ConnectionState::Connected,
        };

        self.active_connections
            .insert(peer_id.clone(), connection.clone());
        self.ip_connections.entry(ip).or_default().push(peer_id);

        self.update_stats(&connection);
        Ok(())
    }

    /// Initiate an outbound connection
    pub fn initiate_outbound_connection(
        &mut self,
        peer_id: PeerId,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<(), ConnectionError> {
        // Check global limit
        if self.active_connections.len() >= self.config.max_total_connections {
            self.stats.rejected_connections += 1;
            return Err(ConnectionError::GlobalLimitReached);
        }

        // Check outbound limit
        let outbound_count = self.count_outbound_connections();
        if outbound_count >= self.config.max_outbound_connections {
            self.stats.rejected_connections += 1;
            return Err(ConnectionError::OutboundLimitReached);
        }

        // Check per-IP limit
        let ip_count = self.count_connections_from_ip(&ip);
        if ip_count >= self.config.max_connections_per_ip {
            self.stats.rejected_connections += 1;
            return Err(ConnectionError::IpLimitReached);
        }

        // Create connection
        let connection = Connection {
            peer_id: peer_id.clone(),
            ip,
            direction: Direction::Outbound,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            user_agent,
            state: ConnectionState::Connecting,
        };

        self.active_connections
            .insert(peer_id.clone(), connection.clone());
        self.ip_connections.entry(ip).or_default().push(peer_id);

        self.update_stats(&connection);
        Ok(())
    }

    /// Remove a connection (disconnection)
    pub fn remove_connection(&mut self, peer_id: &PeerId) {
        if let Some(connection) = self.active_connections.remove(peer_id) {
            // Remove from IP tracking
            if let Some(peers) = self.ip_connections.get_mut(&connection.ip) {
                peers.retain(|p| *p != *peer_id);
                if peers.is_empty() {
                    self.ip_connections.remove(&connection.ip);
                }
            }

            // Update stats
            self.stats.current_connections = self.active_connections.len();
            self.stats.unique_ips = self.ip_connections.len();
        }
    }

    /// Update connection activity
    pub fn update_activity(&mut self, peer_id: &PeerId, bytes_sent: u64, bytes_received: u64) {
        if let Some(connection) = self.active_connections.get_mut(peer_id) {
            connection.last_activity = Instant::now();
            connection.bytes_sent += bytes_sent;
            connection.bytes_received += bytes_received;
        }
    }

    /// Update connection state
    pub fn update_connection_state(&mut self, peer_id: &PeerId, new_state: ConnectionState) {
        if let Some(connection) = self.active_connections.get_mut(peer_id) {
            connection.state = new_state;
        }
    }

    /// Clean up idle and timed out connections
    pub fn cleanup_connections(&mut self) -> Vec<PeerId> {
        let now = Instant::now();
        let mut to_disconnect = Vec::new();

        // Check for idle connections
        for (peer_id, connection) in &self.active_connections {
            let idle_duration = now.duration_since(connection.last_activity);

            if idle_duration > self.config.idle_timeout {
                to_disconnect.push(peer_id.clone());
            }
        }

        // Remove timed out connections
        for peer_id in &to_disconnect {
            if let Some(connection) = self.active_connections.get(peer_id) {
                let connection_duration = now.duration_since(connection.connected_at);

                if connection.state == ConnectionState::Connecting
                    && connection_duration > self.config.connection_timeout
                {
                    self.stats.timed_out_connections += 1;
                }
            }

            self.remove_connection(peer_id);
        }

        to_disconnect
    }

    /// Get connection by peer ID
    pub fn get_connection(&self, peer_id: &PeerId) -> Option<&Connection> {
        self.active_connections.get(peer_id)
    }

    /// Get all connections
    pub fn get_all_connections(&self) -> impl Iterator<Item = &Connection> {
        self.active_connections.values()
    }

    /// Get connections by IP address
    pub fn get_connections_by_ip(&self, ip: &IpAddr) -> impl Iterator<Item = &PeerId> {
        self.ip_connections
            .get(ip)
            .into_iter()
            .flat_map(|peers| peers.iter())
    }

    /// Check if IP is rate limited for connections
    fn is_rate_limited(&mut self, ip: &IpAddr) -> bool {
        if !self.config.enable_rate_limiting {
            return false;
        }

        let now = Instant::now();
        let attempts = self.connection_attempts.entry(*ip).or_default();

        // Remove old attempts (older than 1 minute)
        attempts.retain(|&timestamp| now.duration_since(timestamp) < Duration::from_secs(60));

        // Check if too many attempts
        if attempts.len() >= self.config.max_connection_attempts_per_minute as usize {
            true
        } else {
            attempts.push(now);
            false
        }
    }

    /// Count inbound connections
    fn count_inbound_connections(&self) -> usize {
        self.active_connections
            .values()
            .filter(|conn| conn.direction == Direction::Inbound)
            .count()
    }

    /// Count outbound connections
    fn count_outbound_connections(&self) -> usize {
        self.active_connections
            .values()
            .filter(|conn| conn.direction == Direction::Outbound)
            .count()
    }

    /// Count connections from specific IP
    fn count_connections_from_ip(&self, ip: &IpAddr) -> usize {
        self.ip_connections
            .get(ip)
            .map(|peers| peers.len())
            .unwrap_or(0)
    }

    /// Update connection statistics
    fn update_stats(&mut self, connection: &Connection) {
        self.stats.total_connections += 1;
        self.stats.current_connections = self.active_connections.len();
        self.stats.unique_ips = self.ip_connections.len();

        match connection.direction {
            Direction::Inbound => self.stats.inbound_connections += 1,
            Direction::Outbound => self.stats.outbound_connections += 1,
        }
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &ConnectionStats {
        &self.stats
    }

    /// Get connection count by direction
    pub fn get_connection_counts(&self) -> (usize, usize, usize) {
        let inbound = self.count_inbound_connections();
        let outbound = self.count_outbound_connections();
        let total = self.active_connections.len();

        (inbound, outbound, total)
    }

    /// Check if can accept more connections
    pub fn can_accept_connection(&mut self, ip: &IpAddr) -> bool {
        self.active_connections.len() < self.config.max_total_connections
            && self.count_inbound_connections() < self.config.max_inbound_connections
            && self.count_connections_from_ip(ip) < self.config.max_connections_per_ip
            && !self.is_rate_limited(ip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_basic_operations() {
        let mut config = ConnectionConfig::default();
        config.max_connections_per_ip = 1; // Set to 1 for testing
        let mut manager = ConnectionManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());
        let ip = "127.0.0.1".parse().unwrap();

        // Should accept inbound connection
        assert!(manager.accept_inbound_connection(peer, ip, None).is_ok());
        assert_eq!(manager.get_connection_counts(), (1, 0, 1));

        // Should reject duplicate from same IP
        let peer2 = format!("test_peer_{}", rand::random::<u64>());
        assert!(matches!(
            manager.accept_inbound_connection(peer2, ip, None),
            Err(ConnectionError::IpLimitReached)
        ));
    }

    #[test]
    fn test_connection_limits() {
        let mut config = ConnectionConfig::default();
        config.max_total_connections = 2;
        let mut manager = ConnectionManager::new(config);

        let peer1 = format!("test_peer_{}", rand::random::<u64>());
        let peer2 = format!("test_peer_{}", rand::random::<u64>());
        let ip1 = "127.0.0.1".parse().unwrap();
        let ip2 = "127.0.0.2".parse().unwrap();

        // Should accept first two connections
        assert!(manager.accept_inbound_connection(peer1, ip1, None).is_ok());
        assert!(manager.accept_inbound_connection(peer2, ip2, None).is_ok());

        // Should reject third connection
        let peer3 = format!("test_peer_{}", rand::random::<u64>());
        let ip3 = "127.0.0.3".parse().unwrap();
        assert!(matches!(
            manager.accept_inbound_connection(peer3, ip3, None),
            Err(ConnectionError::GlobalLimitReached)
        ));
    }

    #[test]
    fn test_outbound_connections() {
        let config = ConnectionConfig::default();
        let mut manager = ConnectionManager::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());
        let ip = "127.0.0.1".parse().unwrap();

        // Should accept outbound connection
        assert!(manager.initiate_outbound_connection(peer, ip, None).is_ok());
        assert_eq!(manager.get_connection_counts(), (0, 1, 1));
    }

    #[test]
    fn test_connection_cleanup() {
        let mut config = ConnectionConfig::default();
        config.idle_timeout = Duration::from_millis(100);
        let mut manager = ConnectionManager::new(config);

        let peer = format!("test_peer_{}", rand::random::<u64>());
        let ip = "127.0.0.1".parse().unwrap();

        assert!(manager
            .accept_inbound_connection(peer.clone(), ip, None)
            .is_ok());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Should clean up idle connection
        let disconnected = manager.cleanup_connections();
        assert_eq!(disconnected.len(), 1);
        assert_eq!(disconnected[0], peer);
    }
}
