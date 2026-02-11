//! Peer-to-peer networking with comprehensive security for BitQuan.
#![warn(missing_docs)]

pub mod ban_manager;
pub mod connection_manager;
pub mod dos_protection;
pub mod height_validation;
pub mod noise; // Noise Protocol encryption for P2P
pub mod rate_limiter;
pub mod reputation;
pub mod security_config;
pub mod security_manager;

pub mod async_sync;
pub mod discovery;
pub mod dns_bootstrap;
pub mod io;
pub mod peer;
pub mod peer_async; // NEW: Async peer implementation with Slowloris protection
pub mod propagation;
pub mod protocol;
pub mod relay;
pub mod server_async; // NEW: Async P2P server
pub mod sync;

pub use ban_manager::{BanConfig, BanError, BanInfo, BanManager, BanReason, BanStats};
pub use connection_manager::{
    ConnectionConfig, ConnectionError, ConnectionManager, ConnectionState, ConnectionStats,
    Direction,
};
pub use dos_protection::{
    AttackInfo, AttackSeverity, DoSConfig, DoSError, DoSProtection, DoSStats,
};
pub use rate_limiter::{
    MessageType, MessageTypeLimits, RateLimitConfig, RateLimitError, RateLimiter,
};
pub use security_manager::{
    PeerSecurityStatus, SecurityError, SecurityEvent, SecurityManager, SecurityStatistics,
};

pub use height_validation::{
    blocks_behind, range_size, sync_progress,
    validate_height_range, validate_peer_height, validate_request_range,
    HeightValidationError, HeightResult, MAX_SANITY_HEIGHT_DIFF,
    MAX_UNVERIFIED_HEIGHT_DIFF,
};


pub use discovery::{
    bootstrap_peers, discover_from_seeds, PeerBook, PersistentPeer, MAINNET_SEEDS,
    PEER_TIMEOUT_SECS, PING_INTERVAL_SECS, TESTNET_SEEDS,
};
pub use dns_bootstrap::{load_default_seeds, DnsBootstrap, DnsSeed};
#[allow(deprecated)] // Re-export deprecated items for backwards compatibility
pub use peer::{
    async_noise_handshake_initiator, async_noise_handshake_responder,
    async_version_handshake_inbound, async_version_handshake_outbound, handshake, read_frame,
    EclipseConfig, P2PListener, Peer, PeerManager, PeerState, HANDSHAKE_TIMEOUT_MS, MAX_MSG_BYTES,
    SOCKET_TIMEOUT_SECS,
};
pub use propagation::{
    broadcast_block_inv, create_envelope, BlockPropagator, PropagationStats, SeenFilter,
};
pub use relay::{create_block_inv, create_tx_inv, RelayManager, RelayPolicy};
pub use sync::{process_headers, request_blocks, ChainSync, SyncProgress, SyncStatus};

// Noise Protocol encryption
pub use noise::{NoiseConfig, NoiseError, NoiseTransport};

pub use bitquan_types::Block;
pub use thiserror::Error;

/// Logical peer identifier.
pub type PeerId = String;

/// Result type for network operations.
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Configuration values for networking layer.
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// Address to bind listening socket to (multiaddr style in future).
    pub listen_addr: String,
    /// Maximum concurrent peers accepted.
    pub max_peers: usize,
    /// Enable Noise Protocol encryption for P2P connections
    pub enable_encryption: bool,
    /// Maximum message size in bytes (10 MB)
    pub max_message_size: usize,
    /// Rate limit: max messages per second per peer
    pub rate_limit_per_peer: usize,
    /// Security configuration
    pub security: SecurityConfig,
    /// Network ID (Mainnet, Testnet, etc.)
    pub network: bitquan_types::NetworkId,
}

/// Re-export SecurityConfig from security_config module
pub use security_config::SecurityConfig;

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/0.0.0.0/tcp/8333".to_owned(),
            max_peers: 125,
            enable_encryption: true,
            max_message_size: 10_000_000,
            rate_limit_per_peer: 100,
            security: SecurityConfig::default(),
            network: bitquan_types::NetworkId::Mainnet,
        }
    }
}

/// Errors that can occur during network operations.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Generic I/O failure placeholder.
    #[error("io failure: {0}")]
    Io(String),
    /// Peer not connected.
    #[error("peer not connected")]
    NotConnected,

    /// Invalid peer address.
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Connection failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(String),
    /// Lock poisoned error
    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
    /// Invalid message type
    #[error("invalid message type: expected {expected}, got {got}")]
    InvalidMessageType {
        /// Expected message type string.
        expected: String,
        /// Actual message type received.
        got: String,
    },
    /// Security error
    #[error("security error: {0}")]
    Security(String),
}

/// High-level network service with comprehensive security.
pub struct NetworkService {
    config: NetworkConfig,
    peers: Vec<PeerId>,
    security: SecurityManager,
    propagator: BlockPropagator,
}

impl NetworkService {
    /// Creates a new service instance with the provided configuration.
    pub fn new(config: NetworkConfig) -> Self {
        let security = SecurityManager::new(config.security.clone());
        let propagator = BlockPropagator::new();

        Self {
            config,
            peers: Vec::new(),
            security,
            propagator,
        }
    }

    /// Returns the configured maximum number of peers.
    pub fn max_peers(&self) -> usize {
        self.config.max_peers
    }

    /// Get security manager reference
    pub fn security(&self) -> &SecurityManager {
        &self.security
    }

    /// Get mutable security manager reference
    pub fn security_mut(&mut self) -> &mut SecurityManager {
        &mut self.security
    }

    /// Connects to a new peer with security checks.
    pub fn connect(&mut self, peer: PeerId, ip: std::net::IpAddr) -> Result<()> {
        // Check bans
        if self.security.is_peer_banned(&peer) {
            return Err(NetworkError::Security("Peer banned".to_string()));
        }

        if self.security.is_ip_banned(&ip) {
            return Err(NetworkError::Security("IP banned".to_string()));
        }

        // Original logic
        if !self.peers.contains(&peer) && self.peers.len() < self.config.max_peers {
            self.peers.push(peer);
        }

        Ok(())
    }

    /// Get list of connected peers
    pub fn peers(&self) -> &[PeerId] {
        &self.peers
    }

    /// Disconnects an existing peer.
    pub fn disconnect(&mut self, peer: &PeerId) {
        self.peers.retain(|p| p != peer);
    }

    /// Broadcasts a block to all connected peers.
    pub fn broadcast_block(&mut self, block: &Block) -> Result<()> {
        if self.peers.is_empty() {
            return Err(NetworkError::NotConnected);
        }

        // Compute block hash
        let hash = bitquan_consensus::header_hash(&block.header);

        // Check if we should broadcast (prevent duplicates)
        if !self.propagator.should_propagate_block(hash) {
            // Already propagated
            return Ok(());
        }

        // Create block message
        let message = protocol::Message::Block {
            block: block.clone(),
        };

        // Send to all connected peers
        for peer in &self.peers {
            // Send block to peer via TCP
            if let Err(e) = self.send_to_peer(peer, &message) {
                // Log error but continue to other peers
                log::error!(
                    "[P2P] Failed to send block {} to peer {}: {}",
                    hex::encode(&hash[..8]),
                    peer,
                    e
                );
            }
        }

        // Mark as propagated
        self.propagator.mark_block_propagated(hash)?;

        Ok(())
    }

    /// Sends a message to a peer via TCP.
    ///
    /// This is a simple, synchronous TCP implementation with timeout.
    /// The message is serialized using JSON and sent with a length prefix.
    ///
    /// # Arguments
    /// * `peer_addr` - Peer address (e.g., "127.0.0.1:8333")
    /// * `message` - Message to send
    ///
    /// # Errors
    /// Returns error if connection fails, serialization fails, or send fails.
    fn send_to_peer(&self, peer_addr: &str, message: &protocol::Message) -> Result<()> {
        use std::io::Write;
        use std::net::TcpStream;
        use std::time::Duration;

        // Create message envelope
        let magic = protocol::network_magic(self.config.network);
        let envelope = protocol::MessageEnvelope::new(magic, message.clone());

        // Serialize message
        let data = envelope
            .serialize()
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;

        // Connect to peer with timeout (5 seconds)
        let mut stream = TcpStream::connect_timeout(
            &peer_addr
                .parse()
                .map_err(|e| NetworkError::InvalidAddress(format!("Invalid address: {}", e)))?,
            Duration::from_secs(5),
        )
        .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to connect: {}", e)))?;

        // Set write timeout
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| NetworkError::IoError(format!("Failed to set timeout: {}", e)))?;

        // Send data
        stream
            .write_all(&data)
            .map_err(|e| NetworkError::IoError(format!("Failed to send data: {}", e)))?;

        // Flush to ensure data is sent
        stream
            .flush()
            .map_err(|e| NetworkError::IoError(format!("Failed to flush: {}", e)))?;

        Ok(())
    }

    /// Get propagation statistics
    pub fn propagation_stats(&self) -> Result<PropagationStats> {
        self.propagator.stats()
    }
}
