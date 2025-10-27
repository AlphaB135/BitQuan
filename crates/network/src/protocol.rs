//! P2P networking protocol and message handling.
//!
//! This module implements the peer-to-peer protocol for block and transaction
//! propagation in the BitQuan network.

use bitquan_types::{Block, BlockHeader, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// P2P protocol errors.
#[derive(Debug, Error)]
pub enum P2pError {
    /// Invalid message format.
    #[error("invalid message format")]
    InvalidMessage,

    /// Message too large.
    #[error("message too large: {0} bytes")]
    MessageTooLarge(usize),

    /// Protocol version mismatch.
    #[error("protocol version mismatch: got {0}, expected {1}")]
    VersionMismatch(u32, u32),

    /// Peer connection error.
    #[error("peer connection error: {0}")]
    ConnectionError(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    SerializationError(String),
}

/// Protocol version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size (10 MB).
pub const MAX_MESSAGE_SIZE: usize = 10_000_000;

/// Maximum inv/getdata items per message.
pub const MAX_INV_ITEMS: usize = 50_000;

/// Network magic bytes for mainnet.
pub const MAINNET_MAGIC: [u8; 4] = [0x42, 0x51, 0x01, 0x01]; // BQ mainnet

/// Network magic bytes for testnet.
pub const TESTNET_MAGIC: [u8; 4] = [0x42, 0x51, 0x02, 0x02]; // BQ testnet

/// Network magic bytes for devnet.
pub const DEVNET_MAGIC: [u8; 4] = [0x42, 0x51, 0x03, 0x03]; // BQ devnet

/// Network magic bytes for regtest.
pub const REGTEST_MAGIC: [u8; 4] = [0x42, 0x51, 0x04, 0x04]; // BQ regtest

/// Returns network magic for the given NetworkId.
pub fn network_magic(network: bitquan_types::NetworkId) -> [u8; 4] {
    match network {
        bitquan_types::NetworkId::Mainnet => MAINNET_MAGIC,
        bitquan_types::NetworkId::Testnet => TESTNET_MAGIC,
        bitquan_types::NetworkId::Devnet => DEVNET_MAGIC,
        bitquan_types::NetworkId::Regtest => REGTEST_MAGIC,
    }
}

/// P2P message types.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Message {
    /// Version handshake.
    Version {
        /// Protocol version number
        version: u32,
        /// Service flags
        services: u64,
        /// Unix timestamp
        timestamp: u64,
        /// Client user agent string
        user_agent: String,
        /// Starting block height
        start_height: u64,
    },

    /// Version acknowledgment.
    VerAck,

    /// Ping for keepalive.
    Ping {
        /// Random nonce for ping/pong matching
        nonce: u64,
    },

    /// Pong response.
    Pong {
        /// Nonce from corresponding ping
        nonce: u64,
    },

    /// Request peer addresses.
    GetAddr,

    /// Advertise peer addresses.
    Addr {
        /// List of peer addresses
        addrs: Vec<PeerAddr>,
    },

    /// Inventory announcement (blocks/txs available).
    Inv {
        /// Inventory vectors
        inventory: Vec<InvVector>,
    },

    /// Request data.
    GetData {
        /// Requested inventory items
        inventory: Vec<InvVector>,
    },

    /// Block data.
    Block {
        /// Full block
        block: Block,
    },

    /// Transaction data.
    Tx {
        /// Full transaction
        transaction: Transaction,
    },

    /// Request block headers.
    GetHeaders {
        /// Protocol version
        version: u32,
        /// Block locator hashes
        locator_hashes: Vec<[u8; 32]>,
        /// Stop hash
        stop_hash: [u8; 32],
    },

    /// Block headers response.
    Headers {
        /// List of block headers
        headers: Vec<BlockHeader>,
    },

    /// Mempool query.
    GetMempool,

    /// Reject message.
    Reject {
        /// Message type being rejected
        message: String,
        /// Rejection code
        code: RejectCode,
        /// Human-readable reason
        reason: String,
    },
}

/// Peer address information.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAddr {
    /// Timestamp.
    pub timestamp: u64,
    /// Services provided.
    pub services: u64,
    /// IP address.
    pub ip: String,
    /// Port.
    pub port: u16,
}

/// Inventory vector for announcing available data.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct InvVector {
    /// Type of data.
    pub inv_type: InvType,
    /// Hash of the data.
    pub hash: [u8; 32],
}

/// Inventory types.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum InvType {
    /// Transaction.
    Tx = 1,
    /// Block.
    Block = 2,
    /// Filtered block (not used yet).
    FilteredBlock = 3,
}

/// Rejection codes.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum RejectCode {
    /// Malformed message.
    Malformed = 0x01,
    /// Invalid data.
    Invalid = 0x10,
    /// Obsolete version.
    Obsolete = 0x11,
    /// Duplicate.
    Duplicate = 0x12,
    /// Non-standard transaction.
    Nonstandard = 0x40,
    /// Insufficient fee.
    InsufficientFee = 0x42,
}

/// Message envelope with header.
#[derive(Clone, Debug)]
pub struct MessageEnvelope {
    /// Network magic bytes.
    pub magic: [u8; 4],
    /// Message payload.
    pub message: Message,
}

impl MessageEnvelope {
    /// Creates a new message envelope.
    pub fn new(message: Message) -> Self {
        Self {
            magic: MAINNET_MAGIC,
            message,
        }
    }

    /// Serializes the message to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, P2pError> {
        let payload = serde_json::to_vec(&self.message)
            .map_err(|e| P2pError::SerializationError(e.to_string()))?;

        if payload.len() > MAX_MESSAGE_SIZE {
            return Err(P2pError::MessageTooLarge(payload.len()));
        }

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.magic);
        buffer.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&payload);

        Ok(buffer)
    }

    /// Deserializes a message from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, P2pError> {
        if data.len() < 8 {
            return Err(P2pError::InvalidMessage);
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&data[0..4]);

        if magic != MAINNET_MAGIC {
            return Err(P2pError::InvalidMessage);
        }

        let length = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

        if length > MAX_MESSAGE_SIZE {
            return Err(P2pError::MessageTooLarge(length));
        }

        if data.len() < 8 + length {
            return Err(P2pError::InvalidMessage);
        }

        let message = serde_json::from_slice(&data[8..8 + length])
            .map_err(|e| P2pError::SerializationError(e.to_string()))?;

        Ok(Self { magic, message })
    }
}

/// Peer connection state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerState {
    /// Initial state, waiting for version.
    New,
    /// Version sent, waiting for verack.
    VersionSent,
    /// Fully connected and active.
    Active,
    /// Disconnected.
    Disconnected,
}

/// Peer information.
#[derive(Clone, Debug)]
pub struct Peer {
    /// Peer address.
    pub addr: String,
    /// Connection state.
    pub state: PeerState,
    /// Protocol version.
    pub version: u32,
    /// Services.
    pub services: u64,
    /// User agent.
    pub user_agent: String,
    /// Start height.
    pub start_height: u64,
    /// Last seen timestamp.
    pub last_seen: u64,
}

impl Peer {
    /// Creates a new peer.
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            state: PeerState::New,
            version: 0,
            services: 0,
            user_agent: String::new(),
            start_height: 0,
            last_seen: 0,
        }
    }

    /// Checks if peer is active.
    pub fn is_active(&self) -> bool {
        self.state == PeerState::Active
    }
}

/// Simple peer manager.
pub struct PeerManager {
    /// Connected peers.
    peers: Vec<Peer>,
    /// Maximum peers.
    max_peers: usize,
}

impl PeerManager {
    /// Creates a new peer manager.
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: Vec::new(),
            max_peers,
        }
    }

    /// Adds a peer.
    pub fn add_peer(&mut self, addr: String) -> Result<(), P2pError> {
        if self.peers.len() >= self.max_peers {
            return Err(P2pError::ConnectionError("max peers reached".to_string()));
        }

        // Check for duplicate
        if self.peers.iter().any(|p| p.addr == addr) {
            return Err(P2pError::ConnectionError(
                "peer already connected".to_string(),
            ));
        }

        self.peers.push(Peer::new(addr));
        Ok(())
    }

    /// Removes a peer.
    pub fn remove_peer(&mut self, addr: &str) {
        self.peers.retain(|p| p.addr != addr);
    }

    /// Gets active peers.
    pub fn active_peers(&self) -> Vec<&Peer> {
        self.peers.iter().filter(|p| p.is_active()).collect()
    }

    /// Gets peer count.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Updates peer state.
    pub fn update_peer_state(&mut self, addr: &str, state: PeerState) {
        if let Some(peer) = self.peers.iter_mut().find(|p| p.addr == addr) {
            peer.state = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_serialization_roundtrip() {
        let msg = Message::Version {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: 1234567890,
            user_agent: "BitQuan/0.1.0".to_string(),
            start_height: 100,
        };

        let envelope = MessageEnvelope::new(msg.clone());
        let serialized = envelope.serialize().unwrap();
        let deserialized = MessageEnvelope::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.message, msg);
        assert_eq!(deserialized.magic, MAINNET_MAGIC);
    }

    #[test]
    fn reject_oversized_message() {
        let large_data = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let msg = Message::Reject {
            message: String::from_utf8(large_data).unwrap(),
            code: RejectCode::Invalid,
            reason: "test".to_string(),
        };

        let envelope = MessageEnvelope::new(msg);
        let result = envelope.serialize();

        assert!(matches!(result, Err(P2pError::MessageTooLarge(_))));
    }

    #[test]
    fn peer_manager_basic() {
        let mut pm = PeerManager::new(3);

        pm.add_peer("127.0.0.1:8333".to_string()).unwrap();
        pm.add_peer("127.0.0.1:8334".to_string()).unwrap();

        assert_eq!(pm.peer_count(), 2);

        pm.update_peer_state("127.0.0.1:8333", PeerState::Active);
        assert_eq!(pm.active_peers().len(), 1);

        pm.remove_peer("127.0.0.1:8333");
        assert_eq!(pm.peer_count(), 1);
    }

    #[test]
    fn peer_manager_max_peers() {
        let mut pm = PeerManager::new(2);

        pm.add_peer("127.0.0.1:8333".to_string()).unwrap();
        pm.add_peer("127.0.0.1:8334".to_string()).unwrap();

        let result = pm.add_peer("127.0.0.1:8335".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_peer() {
        let mut pm = PeerManager::new(5);

        pm.add_peer("127.0.0.1:8333".to_string()).unwrap();
        let result = pm.add_peer("127.0.0.1:8333".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_network_magic_values() {
        assert_eq!(MAINNET_MAGIC, [0x42, 0x51, 0x01, 0x01]);
        assert_eq!(TESTNET_MAGIC, [0x42, 0x51, 0x02, 0x02]);
        assert_eq!(DEVNET_MAGIC, [0x42, 0x51, 0x03, 0x03]);
        assert_eq!(REGTEST_MAGIC, [0x42, 0x51, 0x04, 0x04]);
    }

    #[test]
    fn test_network_magic_function() {
        use bitquan_types::NetworkId;
        assert_eq!(network_magic(NetworkId::Mainnet), MAINNET_MAGIC);
        assert_eq!(network_magic(NetworkId::Testnet), TESTNET_MAGIC);
        assert_eq!(network_magic(NetworkId::Devnet), DEVNET_MAGIC);
        assert_eq!(network_magic(NetworkId::Regtest), REGTEST_MAGIC);
    }

    #[test]
    fn test_all_network_magics_unique() {
        let magics = [MAINNET_MAGIC, TESTNET_MAGIC, DEVNET_MAGIC, REGTEST_MAGIC];
        for i in 0..magics.len() {
            for j in (i + 1)..magics.len() {
                assert_ne!(magics[i], magics[j]);
            }
        }
    }
}
