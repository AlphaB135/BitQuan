//! Async TCP-based P2P connection handler with Slowloris protection.
//!
//! This module provides an async version of the Peer struct that properly
//! protects against Slowloris attacks using tokio::time::timeout.

use crate::protocol::{Message, MessageEnvelope, P2pError, PROTOCOL_VERSION};
use bitquan_types::error::{Error, Result as TypesResult};
use bitquan_types::ext::ResultExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::net::SocketAddr;
use std::sync::{Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH, Instant};
use tokio::sync::Mutex;

/// Helper to get current Unix timestamp.
fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Maximum frame size accepted (2 MiB).
pub const MAX_MSG_BYTES: usize = 2 * 1024 * 1024;

/// Handshake timeout.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(1_200);

/// Frame read timeout (Slowloris protection).
/// 
/// CRITICAL: This is the TOTAL time allowed for reading an entire message frame.
/// Unlike sync version, this does NOT reset on partial reads.
pub const FRAME_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Peer connection states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    /// Initial connection established.
    Connected,
    /// Version sent, waiting for verack.
    VersionSent,
    /// Version received, waiting to send verack.
    VersionReceived,
    /// Handshake complete.
    Ready,
    /// Connection closed.
    Disconnected,
}

/// Async peer connection.
pub struct AsyncPeer {
    /// Peer's socket address.
    pub addr: SocketAddr,
    /// Current connection state.
    pub state: PeerState,
    /// Async TCP stream.
    stream: TcpStream,
    /// Peer's protocol version.
    pub version: Option<u32>,
    /// Peer's user agent.
    pub user_agent: Option<String>,
    /// Peer's starting height.
    pub start_height: Option<u64>,
    /// Last seen (for timeout detection).
    pub last_seen: Instant,
    /// Message count (rate limiting).
    pub message_count: u64,
    /// Rate limit window start.
    pub rate_limit_window: Instant,
    /// Ban score.
    pub ban_score: u32,
    /// Network magic.
    pub magic: [u8; 4],
}

impl AsyncPeer {
    /// Creates a new async peer from an accepted connection.
    pub fn new(stream: TcpStream, addr: SocketAddr, magic: [u8; 4]) -> Self {
        AsyncPeer {
            addr,
            state: PeerState::Connected,
            stream,
            version: None,
            user_agent: None,
            start_height: None,
            last_seen: Instant::now(),
            message_count: 0,
            rate_limit_window: Instant::now(),
            ban_score: 0,
            magic,
        }
    }

    /// Performs async handshake with timeout.
    pub async fn handshake(&mut self) -> TypesResult<()> {
        tokio::time::timeout(HANDSHAKE_TIMEOUT, self.do_handshake())
            .await
            .map_err(|_| Error::Timeout("handshake timeout".to_string()))?
    }

    async fn do_handshake(&mut self) -> TypesResult<()> {
        // Exchange magic byte
        self.stream.write_all(&[0x42]).await?;
        let mut response = [0u8; 1];
        self.stream.read_exact(&mut response).await?;
        
        if response[0] != 0x42 {
            return Err(Error::Invalid("invalid handshake token".to_string()));
        }
        Ok(())
    }

    /// Reads a message frame with Slowloris protection.
    ///
    /// CRITICAL SECURITY FIX: Slowloris Attack Protection
    ///
    /// This function uses `tokio::time::timeout` to wrap the ENTIRE read operation.
    /// Unlike the sync version, this timeout does NOT reset on partial reads.
    ///
    /// Attack scenario (BLOCKED):
    /// 1. Attacker connects
    /// 2. Sends 1 byte at t=0s
    /// 3. Sends 1 byte at t=29s
    /// 4. Sends 1 byte at t=58s
    /// 5. At t=30s, tokio::time::timeout fires → connection closed ✅
    ///
    /// Why this works:
    /// - timeout() wraps the ENTIRE async block
    /// - Time is measured from start of timeout(), not from last read
    /// - Partial reads do NOT extend the deadline
    pub async fn read_frame(&mut self) -> TypesResult<Vec<u8>> {
        // SECURITY: Total timeout for entire frame read
        tokio::time::timeout(FRAME_READ_TIMEOUT, self.read_frame_internal())
            .await
            .map_err(|_| Error::Timeout("frame read timeout (slowloris protection)".to_string()))?
    }

    async fn read_frame_internal(&mut self) -> TypesResult<Vec<u8>> {
        // Read length (4 bytes)
        let mut len_le = [0u8; 4];
        self.stream.read_exact(&mut len_le).await.ctx("read len")?;
        
        let len = u32::from_le_bytes(len_le) as usize;
        if len == 0 {
            return Err(Error::Invalid("empty frame".to_string()));
        }
        if len > MAX_MSG_BYTES {
            return Err(Error::Invalid("message too large".to_string()));
        }

        // Read payload
        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf).await.ctx("read frame")?;
        
        Ok(buf)
    }

    /// Sends a message to the peer.
    pub async fn send_message(&mut self, msg: Message) -> Result<(), P2pError> {
        let envelope = MessageEnvelope::new(self.magic, msg);
        self.send_envelope(&envelope).await
    }

    async fn send_envelope(&mut self, envelope: &MessageEnvelope) -> Result<(), P2pError> {
        let data = envelope.serialize()
            .map_err(|e| P2pError::SerializationError(e.to_string()))?;
        
        // Timeout for write as well
        tokio::time::timeout(FRAME_READ_TIMEOUT, self.stream.write_all(&data))
            .await
            .map_err(|_| P2pError::ConnectionError("write timeout".to_string()))?
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
        
        Ok(())
    }

    /// Receives a message from the peer.
    pub async fn recv_message(&mut self) -> Result<Message, P2pError> {
        let frame = self.read_frame().await
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
        
        let envelope = MessageEnvelope::deserialize(&frame, self.magic)?;
        
        self.last_seen = Instant::now();

        // Rate limiting
        let now = Instant::now();
        if now.duration_since(self.rate_limit_window) >= Duration::from_secs(1) {
            self.message_count = 0;
            self.rate_limit_window = now;
        }

        self.message_count += 1;
        if self.message_count > 100 {
            return Err(P2pError::ConnectionError("rate limit exceeded".into()));
        }

        // Validate message for memory exhaustion attacks
        crate::protocol::validate_message(&envelope.message)?;

        Ok(envelope.message)
    }

    /// Performs version handshake (outbound).
    pub async fn handshake_outbound(&mut self, our_height: u64) -> Result<(), P2pError> {
        // Send our version
        let version_msg = Message::Version {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: unix_timestamp(),
            user_agent: "BitQuan/0.1.0-async".to_string(),
            start_height: our_height,
        };

        self.send_message(version_msg).await?;
        self.state = PeerState::VersionSent;

        // Wait for their version
        let msg = self.recv_message().await?;
        match msg {
            Message::Version {
                version,
                user_agent,
                start_height,
                ..
            } => {
                if version != PROTOCOL_VERSION {
                    return Err(P2pError::VersionMismatch(version, PROTOCOL_VERSION));
                }
                self.version = Some(version);
                self.user_agent = Some(user_agent);
                self.start_height = Some(start_height);
                self.state = PeerState::VersionReceived;
            }
            _ => return Err(P2pError::InvalidMessage),
        }

        // Send verack
        self.send_message(Message::VerAck).await?;

        // Wait for their verack
        let msg = self.recv_message().await?;
        match msg {
            Message::VerAck => {
                self.state = PeerState::Ready;
                Ok(())
            }
            _ => Err(P2pError::InvalidMessage),
        }
    }

    /// Performs version handshake (inbound).
    pub async fn handshake_inbound(&mut self, our_height: u64) -> Result<(), P2pError> {
        // Wait for their version
        let msg = self.recv_message().await?;
        match msg {
            Message::Version {
                version,
                user_agent,
                start_height,
                ..
            } => {
                if version != PROTOCOL_VERSION {
                    return Err(P2pError::VersionMismatch(version, PROTOCOL_VERSION));
                }
                self.version = Some(version);
                self.user_agent = Some(user_agent);
                self.start_height = Some(start_height);
                self.state = PeerState::VersionReceived;
            }
            _ => return Err(P2pError::InvalidMessage),
        }

        // Send our version
        let version_msg = Message::Version {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: unix_timestamp(),
            user_agent: "BitQuan/0.1.0-async".to_string(),
            start_height: our_height,
        };

        self.send_message(version_msg).await?;
        self.send_message(Message::VerAck).await?;
        self.state = PeerState::VersionSent;

        // Wait for their verack
        let msg = self.recv_message().await?;
        match msg {
            Message::VerAck => {
                self.state = PeerState::Ready;
                Ok(())
            }
            _ => Err(P2pError::InvalidMessage),
        }
    }

    /// Checks if peer is still alive.
    pub fn is_alive(&self) -> bool {
        self.last_seen.elapsed() < Duration::from_secs(120)
    }

    /// Sends a ping.
    pub async fn send_ping(&mut self, nonce: u64) -> Result<(), P2pError> {
        self.send_message(Message::Ping { nonce }).await
    }

    /// Sends a pong.
    pub async fn send_pong(&mut self, nonce: u64) -> Result<(), P2pError> {
        self.send_message(Message::Pong { nonce }).await
    }

    /// Adds to ban score.
    pub fn add_ban_score(&mut self, points: u32) -> bool {
        self.ban_score += points;
        self.ban_score >= 100
    }

    /// Checks if should be banned.
    pub fn should_ban(&self) -> bool {
        self.ban_score >= 100
    }
}

/// Async peer manager.
pub struct AsyncPeerManager {
    /// Active peers.
    peers: Arc<Mutex<Vec<AsyncPeer>>>,
    /// Max peers.
    max_peers: usize,
    /// Current height.
    current_height: Arc<Mutex<u64>>,
    /// Network magic.
    magic: [u8; 4],
}

impl AsyncPeerManager {
    /// Creates a new async peer manager.
    pub fn new(max_peers: usize, network: bitquan_types::NetworkId) -> Self {
        AsyncPeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            magic: crate::protocol::network_magic(network),
        }
    }

    /// Updates blockchain height.
    pub async fn update_height(&self, height: u64) {
        let mut h = self.current_height.lock().await;
        *h = height;
    }

    /// Adds an inbound peer.
    pub async fn add_peer_inbound(&self, stream: TcpStream, addr: SocketAddr) -> Result<(), P2pError> {
        let mut peers = self.peers.lock().await;

        if peers.len() >= self.max_peers {
            return Err(P2pError::ConnectionError("max peers reached".into()));
        }

        let mut peer = AsyncPeer::new(stream, addr, self.magic);
        let height = *self.current_height.lock().await;
        peer.handshake_inbound(height).await?;

        peers.push(peer);
        Ok(())
    }

    /// Connects to a peer (outbound).
    pub async fn connect_peer(&self, addr: SocketAddr) -> Result<(), P2pError> {
        let mut peers = self.peers.lock().await;

        if peers.len() >= self.max_peers {
            return Err(P2pError::ConnectionError("max peers reached".into()));
        }

        let stream = TcpStream::connect(addr).await
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        let mut peer = AsyncPeer::new(stream, addr, self.magic);
        let height = *self.current_height.lock().await;
        peer.handshake_outbound(height).await?;

        peers.push(peer);
        Ok(())
    }

    /// Broadcasts a message to all ready peers.
    pub async fn broadcast(&self, msg: Message) -> Result<usize, P2pError> {
        let mut peers = self.peers.lock().await;
        let mut sent_count = 0;

        for peer in peers.iter_mut() {
            if peer.state == PeerState::Ready {
                if peer.send_message(msg.clone()).await.is_ok() {
                    sent_count += 1;
                }
            }
        }

        Ok(sent_count)
    }

    /// Cleans up dead peers.
    pub async fn cleanup_peers(&self) {
        let mut peers = self.peers.lock().await;
        peers.retain(|p| p.is_alive() && p.state != PeerState::Disconnected);
    }

    /// Returns peer count.
    pub async fn peer_count(&self) -> usize {
        self.peers.lock().await.len()
    }

    /// Returns ready peer count.
    pub async fn ready_peer_count(&self) -> usize {
        self.peers.lock().await
            .iter()
            .filter(|p| p.state == PeerState::Ready)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_peer_manager() {
        let pm = AsyncPeerManager::new(10, bitquan_types::NetworkId::Devnet);
        assert_eq!(pm.peer_count().await, 0);
        assert_eq!(pm.max_peers, 10);
    }

    #[tokio::test]
    async fn test_height_update() {
        let pm = AsyncPeerManager::new(10, bitquan_types::NetworkId::Devnet);
        pm.update_height(42).await;
        assert_eq!(*pm.current_height.lock().await, 42);
    }
}
