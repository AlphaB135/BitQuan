//! Peer-to-peer networking scaffolding for BitQuan.
#![warn(missing_docs)]

pub mod discovery;
pub mod dns_bootstrap;
pub mod io;
pub mod peer;
pub mod propagation;
pub mod protocol;
pub mod relay;
pub mod sync;

pub use discovery::{
    bootstrap_peers, discover_from_seeds, PeerBook, PersistentPeer, MAINNET_SEEDS,
    PEER_TIMEOUT_SECS, PING_INTERVAL_SECS, TESTNET_SEEDS,
};
pub use dns_bootstrap::{load_default_seeds, DnsBootstrap, DnsSeed};
pub use peer::{
    handshake, read_frame, EclipseConfig, P2PListener, Peer, PeerManager, PeerState,
    HANDSHAKE_TIMEOUT_MS, MAX_MSG_BYTES,
};
pub use propagation::{
    broadcast_block_inv, create_envelope, BlockPropagator, PropagationStats, SeenFilter,
};
pub use relay::{create_block_inv, create_tx_inv, RelayManager, RelayPolicy};
pub use sync::{process_headers, request_blocks, ChainSync, SyncProgress, SyncStatus};

use bitquan_types::Block;
use thiserror::Error;

/// Logical peer identifier.
pub type PeerId = String;

/// Result type for network operations.
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Configuration values for the networking layer.
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// Address to bind the listening socket to (multiaddr style in the future).
    pub listen_addr: String,
    /// Maximum concurrent peers accepted.
    pub max_peers: usize,
    /// Enable TLS/encryption (placeholder for future)
    pub enable_encryption: bool,
    /// Maximum message size in bytes (10 MB)
    pub max_message_size: usize,
    /// Rate limit: max messages per second per peer
    pub rate_limit_per_peer: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/127.0.0.1/tcp/8333".to_owned(),
            max_peers: 125,
            enable_encryption: true,
            max_message_size: 10_000_000,
            rate_limit_per_peer: 100,
        }
    }
}

/// Errors that can occur during network operations.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Generic I/O failure placeholder.
    #[error("io failure: {0}")]
    Io(String),
    /// Attempted operation requires at least one connected peer.
    #[error("no peers connected")]
    NotConnected,
    /// Lock poisoned error
    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
    /// Invalid message type
    #[error("invalid message type: expected {expected}, got {got}")]
    InvalidMessageType { expected: String, got: String },
}

/// High-level façade for managing peer connections.
pub struct NetworkService {
    config: NetworkConfig,
    peers: Vec<PeerId>,
}

impl NetworkService {
    /// Creates a new service instance with the provided configuration.
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            config,
            peers: Vec::new(),
        }
    }

    /// Returns the configured maximum number of peers.
    pub fn max_peers(&self) -> usize {
        self.config.max_peers
    }

    /// Connects to a new peer (placeholder behaviour).
    pub fn connect(&mut self, peer: PeerId) {
        if !self.peers.contains(&peer) && self.peers.len() < self.config.max_peers {
            self.peers.push(peer);
        }
    }

    /// Disconnects an existing peer.
    pub fn disconnect(&mut self, peer: &PeerId) {
        self.peers.retain(|p| p != peer);
    }

    /// Broadcasts a block to connected peers (no-op placeholder for Phase 3).
    pub fn broadcast_block(&self, _block: &Block) -> Result<()> {
        if self.peers.is_empty() {
            return Err(NetworkError::NotConnected);
        }
        Ok(())
    }
}
