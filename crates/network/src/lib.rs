//! Peer-to-peer networking scaffolding for BitQuan.
#![warn(missing_docs)]

pub mod io;
pub mod peer;
pub mod protocol;
pub mod relay;

pub use peer::{EclipseConfig, P2PListener, Peer, PeerManager, PeerState};
pub use relay::{create_block_inv, create_tx_inv, RelayManager, RelayPolicy};

use bitquan_types::Block;
use thiserror::Error;

/// Logical peer identifier.
pub type PeerId = String;

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
    pub fn broadcast_block(&self, _block: &Block) -> Result<(), NetworkError> {
        if self.peers.is_empty() {
            return Err(NetworkError::NotConnected);
        }
        Ok(())
    }
}
