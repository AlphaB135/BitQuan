//! TCP-based P2P connection handler.

use crate::protocol::{Message, MessageEnvelope, P2pError, PROTOCOL_VERSION};
use bitquan_types::error::{Error, Result as TypesResult};
use bitquan_types::ext::ResultExt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Helper to get current Unix timestamp.
/// Returns 0 if system clock is before epoch (extremely unlikely).
fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Maximum frame size accepted by low-level peer helpers (2 MiB).
pub const MAX_MSG_BYTES: usize = 2 * 1024 * 1024;
/// Handshake timeout in milliseconds.
pub const HANDSHAKE_TIMEOUT_MS: u64 = 1_200;

/// Performs a minimal TCP handshake with timeouts applied.
pub fn handshake(stream: &mut TcpStream) -> TypesResult<()> {
    let timeout = Duration::from_millis(HANDSHAKE_TIMEOUT_MS);
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    match do_handshake(stream) {
        Ok(()) => Ok(()),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            Err(Error::Timeout("handshake timeout".to_string()))
        }
        Err(e) => Err(Error::Net(e.to_string())),
    }
}

/// Reads a single length-prefixed message frame.
pub fn read_frame<R: Read>(reader: &mut R) -> TypesResult<Vec<u8>> {
    let mut len_le = [0u8; 4];
    reader.read_exact(&mut len_le).ctx("read len")?;
    let len = u32::from_le_bytes(len_le) as usize;
    if len == 0 {
        return Err(Error::Invalid("empty frame".to_string()));
    }
    if len > MAX_MSG_BYTES {
        return Err(Error::Invalid("message too large".to_string()));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ctx("read frame")?;
    Ok(buf)
}

fn do_handshake(stream: &mut TcpStream) -> io::Result<()> {
    // Exchange a single magic byte to confirm liveness.
    stream.write_all(&[0x42])?;
    let mut response = [0u8; 1];
    stream.read_exact(&mut response)?;
    if response[0] != 0x42 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid handshake token",
        ));
    }
    Ok(())
}

/// Peer connection states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    /// Initial connection established, waiting for version handshake.
    Connected,
    /// Version sent, waiting for verack.
    VersionSent,
    /// Version received, waiting to send verack.
    VersionReceived,
    /// Handshake complete, ready for normal message exchange.
    Ready,
    /// Connection closed or failed.
    Disconnected,
}

/// Represents a single peer connection.
pub struct Peer {
    /// Peer's socket address.
    pub addr: SocketAddr,
    /// Current connection state.
    pub state: PeerState,
    /// TCP stream for communication.
    stream: TcpStream,
    /// Peer's protocol version (from version message).
    pub version: Option<u32>,
    /// Peer's user agent string.
    pub user_agent: Option<String>,
    /// Peer's starting block height.
    pub start_height: Option<u64>,
    /// Last message timestamp (for timeout detection).
    pub last_seen: SystemTime,
    /// Message count for rate limiting.
    pub message_count: u64,
    /// Last rate limit window reset.
    pub rate_limit_window: SystemTime,
    /// Ban score for misbehavior (disconnect at 100).
    pub ban_score: u32,
}

impl Peer {
    /// Creates a new peer from an accepted connection.
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Result<Self, P2pError> {
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        Ok(Peer {
            addr,
            state: PeerState::Connected,
            stream,
            version: None,
            user_agent: None,
            start_height: None,
            last_seen: SystemTime::now(),
            message_count: 0,
            rate_limit_window: SystemTime::now(),
            ban_score: 0,
        })
    }

    /// Adds to ban score and returns true if peer should be disconnected.
    pub fn add_ban_score(&mut self, points: u32) -> bool {
        self.ban_score += points;
        self.ban_score >= 100
    }

    /// Checks if peer should be banned.
    pub fn should_ban(&self) -> bool {
        self.ban_score >= 100
    }

    /// Sends a message to the peer.
    pub fn send_message(&mut self, msg: Message) -> Result<(), P2pError> {
        let envelope = MessageEnvelope::new(msg);
        crate::io::send_envelope(&mut self.stream, &envelope)?;
        Ok(())
    }

    /// Receives a message from the peer (blocking).
    pub fn recv_message(&mut self) -> Result<Message, P2pError> {
        let envelope = crate::io::recv_envelope(&mut self.stream)?;
        self.last_seen = SystemTime::now();

        // Rate limiting check
        // Validate message for memory exhaustion attacks
        crate::protocol::validate_message(&envelope.message)?;

        let now = SystemTime::now();
        if now
            .duration_since(self.rate_limit_window)
            .unwrap_or_default()
            >= Duration::from_secs(1)
        {
            self.message_count = 0;
            self.rate_limit_window = now;
        }

        self.message_count += 1;
        if self.message_count > 100 {
            return Err(P2pError::ConnectionError("rate limit exceeded".into()));
        }

        Ok(envelope.message)
    }

    /// Performs version handshake (outbound connection).
    pub fn handshake_outbound(&mut self, our_height: u64) -> Result<(), P2pError> {
        // Send our version
        let version_msg = Message::Version {
            version: PROTOCOL_VERSION,
            services: 1, // NODE_NETWORK
            timestamp: unix_timestamp(),
            user_agent: "BitQuan/0.1.0".to_string(),
            start_height: our_height,
        };

        self.send_message(version_msg)?;
        self.state = PeerState::VersionSent;

        // Wait for their version
        let msg = self.recv_message()?;
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
        self.send_message(Message::VerAck)?;

        // Wait for their verack
        let msg = self.recv_message()?;
        match msg {
            Message::VerAck => {
                self.state = PeerState::Ready;
                Ok(())
            }
            _ => Err(P2pError::InvalidMessage),
        }
    }

    /// Handles incoming version handshake (inbound connection).
    pub fn handshake_inbound(&mut self, our_height: u64) -> Result<(), P2pError> {
        // Wait for their version
        let msg = self.recv_message()?;
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
            user_agent: "BitQuan/0.1.0".to_string(),
            start_height: our_height,
        };

        self.send_message(version_msg)?;
        self.send_message(Message::VerAck)?;
        self.state = PeerState::VersionSent;

        // Wait for their verack
        let msg = self.recv_message()?;
        match msg {
            Message::VerAck => {
                self.state = PeerState::Ready;
                Ok(())
            }
            _ => Err(P2pError::InvalidMessage),
        }
    }

    /// Checks if peer is still alive (hasn't timed out).
    pub fn is_alive(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_seen)
            .unwrap_or_default()
            < Duration::from_secs(120)
    }

    /// Sends a ping to keep connection alive.
    pub fn send_ping(&mut self, nonce: u64) -> Result<(), P2pError> {
        self.send_message(Message::Ping { nonce })
    }

    /// Sends a pong response.
    pub fn send_pong(&mut self, nonce: u64) -> Result<(), P2pError> {
        self.send_message(Message::Pong { nonce })
    }
}

/// Eclipse attack mitigation configuration
#[derive(Debug, Clone)]
pub struct EclipseConfig {
    /// Maximum peers from same /24 subnet
    pub max_peers_per_subnet: usize,
    /// Anchor peers (hardcoded, never evicted)
    pub anchor_peers: Vec<SocketAddr>,
    /// Enable subnet diversity checks
    pub enforce_subnet_diversity: bool,
}

impl Default for EclipseConfig {
    fn default() -> Self {
        Self {
            max_peers_per_subnet: 2,
            anchor_peers: vec![],
            enforce_subnet_diversity: true,
        }
    }
}

/// Manages multiple peer connections.
pub struct PeerManager {
    /// Active peer connections.
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Maximum number of peers.
    max_peers: usize,
    /// Current blockchain height.
    current_height: Arc<Mutex<u64>>,
    /// Relay manager for tracking announced items.
    relay_manager: Option<Arc<crate::relay::RelayManager>>,
    /// Eclipse attack mitigation config
    eclipse_config: EclipseConfig,
}

impl PeerManager {
    /// Creates a new peer manager.
    pub fn new(max_peers: usize) -> Self {
        PeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            relay_manager: None,
            eclipse_config: EclipseConfig::default(),
        }
    }

    /// Creates a new peer manager with relay support.
    pub fn with_relay(max_peers: usize, relay_manager: Arc<crate::relay::RelayManager>) -> Self {
        PeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            relay_manager: Some(relay_manager),
            eclipse_config: EclipseConfig::default(),
        }
    }

    /// Creates a new peer manager with eclipse attack mitigation
    pub fn with_eclipse_config(
        max_peers: usize,
        relay_manager: Option<Arc<crate::relay::RelayManager>>,
        eclipse_config: EclipseConfig,
    ) -> Self {
        PeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            relay_manager,
            eclipse_config,
        }
    }

    /// Helper to lock peers mutex.
    fn lock_peers(&self) -> Result<std::sync::MutexGuard<'_, Vec<Peer>>, P2pError> {
        self.peers
            .lock()
            .map_err(|_| P2pError::ConnectionError("peer list mutex poisoned".to_string()))
    }

    /// Helper to lock height mutex.
    fn lock_height(&self) -> Result<std::sync::MutexGuard<'_, u64>, P2pError> {
        self.current_height
            .lock()
            .map_err(|_| P2pError::ConnectionError("height mutex poisoned".to_string()))
    }

    /// Updates the current blockchain height.
    pub fn update_height(&self, height: u64) {
        if let Ok(mut h) = self.lock_height() {
            *h = height;
        }
    }

    /// Extract /24 subnet from IP address
    fn get_subnet_24(addr: &SocketAddr) -> Option<[u8; 3]> {
        match addr.ip() {
            std::net::IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                Some([octets[0], octets[1], octets[2]])
            }
            std::net::IpAddr::V6(_) => {
                // For IPv6, we'd use /64 or /48, simplified here
                None
            }
        }
    }

    /// Count peers from the same /24 subnet
    fn count_peers_in_subnet(&self, peers: &[Peer], subnet: [u8; 3]) -> usize {
        peers
            .iter()
            .filter(|p| {
                Self::get_subnet_24(&p.addr)
                    .map(|s| s == subnet)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Check if peer is an anchor (hardcoded, never evicted)
    fn is_anchor(&self, addr: &SocketAddr) -> bool {
        self.eclipse_config
            .anchor_peers
            .iter()
            .any(|anchor| anchor == addr)
    }

    /// Adds a new peer connection (inbound).
    pub fn add_peer_inbound(&self, stream: TcpStream, addr: SocketAddr) -> Result<(), P2pError> {
        let mut peers = self.lock_peers()?;

        if peers.len() >= self.max_peers {
            return Err(P2pError::ConnectionError("max peers reached".into()));
        }

        // Eclipse attack mitigation: check subnet diversity
        if self.eclipse_config.enforce_subnet_diversity {
            if let Some(subnet) = Self::get_subnet_24(&addr) {
                let count = self.count_peers_in_subnet(&peers, subnet);
                if count >= self.eclipse_config.max_peers_per_subnet && !self.is_anchor(&addr) {
                    return Err(P2pError::ConnectionError(format!(
                        "too many peers from same subnet: {} (max: {})",
                        count, self.eclipse_config.max_peers_per_subnet
                    )));
                }
            }
        }

        let mut peer = Peer::new(stream, addr)?;
        let height = *self.lock_height()?;
        peer.handshake_inbound(height)?;

        peers.push(peer);
        Ok(())
    }

    /// Connects to a new peer (outbound).
    pub fn connect_peer(&self, addr: SocketAddr) -> Result<(), P2pError> {
        let mut peers = self.lock_peers()?;

        if peers.len() >= self.max_peers {
            return Err(P2pError::ConnectionError("max peers reached".into()));
        }

        let stream =
            TcpStream::connect(addr).map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        let mut peer = Peer::new(stream, addr)?;
        let height = *self.lock_height()?;
        peer.handshake_outbound(height)?;

        peers.push(peer);
        Ok(())
    }

    /// Broadcasts a message to all ready peers.
    pub fn broadcast(&self, msg: Message) -> Result<usize, P2pError> {
        let mut peers = self.lock_peers()?;
        let mut sent_count = 0;

        for peer in peers.iter_mut() {
            if peer.state == PeerState::Ready {
                if let Ok(()) = peer.send_message(msg.clone()) {
                    sent_count += 1;
                }
            }
        }

        Ok(sent_count)
    }

    /// Broadcasts inventory to all peers (with relay tracking).
    pub fn broadcast_inv(&self, inv: crate::protocol::InvVector) -> Result<usize, P2pError> {
        use crate::protocol::Message;

        // Track announcement if relay manager exists
        if let Some(relay) = &self.relay_manager {
            let _ = relay.announce(&inv);
        }

        let msg = Message::Inv {
            inventory: vec![inv],
        };

        self.broadcast(msg)
    }

    /// Handles incoming inventory announcement.
    pub fn handle_inv(
        &self,
        peer_id: &str,
        inventory: Vec<crate::protocol::InvVector>,
    ) -> Vec<crate::protocol::InvVector> {
        let mut needed = Vec::new();

        if let Some(relay) = &self.relay_manager {
            for inv in inventory {
                // Only request if we haven't seen it
                let announced = relay.has_announced(&inv.hash).unwrap_or(false);
                let relayed = relay.was_relayed(&inv.hash).unwrap_or(false);
                if !announced && !relayed {
                    let _ = relay.add_request(inv.hash, peer_id.to_string());
                    needed.push(inv);
                }
            }
        } else {
            // No relay manager, request everything
            needed = inventory;
        }

        needed
    }

    /// Marks data as relayed.
    pub fn mark_relayed(&self, hash: [u8; 32]) {
        if let Some(relay) = &self.relay_manager {
            let _ = relay.mark_relayed(hash);
        }
    }

    /// Removes disconnected peers.
    pub fn cleanup_peers(&self) {
        if let Ok(mut peers) = self.lock_peers() {
            peers.retain(|p| p.is_alive() && p.state != PeerState::Disconnected);
        }
    }

    /// Returns the current number of peers.
    pub fn peer_count(&self) -> usize {
        self.lock_peers().map(|p| p.len()).unwrap_or(0)
    }

    /// Returns the number of ready peers.
    pub fn ready_peer_count(&self) -> usize {
        self.peers
            .lock()
            .ok()
            .map(|peers| {
                peers
                    .iter()
                    .filter(|p| p.state == PeerState::Ready)
                    .count()
            })
            .unwrap_or(0)
    }

    /// Get subnet diversity statistics
    pub fn get_subnet_stats(&self) -> std::collections::HashMap<[u8; 3], usize> {
        let Ok(peers) = self.lock_peers() else {
            return std::collections::HashMap::new();
        };
        let mut subnet_counts = std::collections::HashMap::new();

        for peer in peers.iter() {
            if let Some(subnet) = Self::get_subnet_24(&peer.addr) {
                *subnet_counts.entry(subnet).or_insert(0) += 1;
            }
        }

        subnet_counts
    }

    /// Evict lowest-reputation non-anchor peer
    pub fn evict_lowest_reputation_peer(&self) -> Option<SocketAddr> {
        let Ok(mut peers) = self.lock_peers() else {
            return None;
        };

        let result = peers
            .iter()
            .enumerate()
            .filter(|(_, p)| !self.is_anchor(&p.addr))
            .max_by_key(|(_, p)| p.ban_score)
            .map(|(i, p)| (i, p.addr));

        if let Some((idx, addr)) = result {
            peers.remove(idx);
            Some(addr)
        } else {
            None
        }
    }

    /// Get list of anchor peers
    pub fn get_anchors(&self) -> Vec<SocketAddr> {
        self.eclipse_config.anchor_peers.clone()
    }

    /// Check if subnet diversity is enforced
    pub fn is_subnet_diversity_enforced(&self) -> bool {
        self.eclipse_config.enforce_subnet_diversity
    }
}

/// TCP listener for accepting incoming peer connections.
pub struct P2PListener {
    listener: TcpListener,
    peer_manager: Arc<PeerManager>,
}

impl P2PListener {
    /// Creates a new P2P listener bound to the specified address.
    pub fn bind(addr: &str, peer_manager: Arc<PeerManager>) -> Result<Self, P2pError> {
        let listener =
            TcpListener::bind(addr).map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        listener
            .set_nonblocking(false)
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        Ok(P2PListener {
            listener,
            peer_manager,
        })
    }

    /// Accepts a single incoming connection.
    pub fn accept_one(&self) -> Result<(), P2pError> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                self.peer_manager.add_peer_inbound(stream, addr)?;
                Ok(())
            }
            Err(e) => Err(P2pError::ConnectionError(e.to_string())),
        }
    }

    /// Returns the local address the listener is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, P2pError> {
        self.listener
            .local_addr()
            .map_err(|e| P2pError::ConnectionError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_manager_creation() {
        let pm = PeerManager::new(10);
        assert_eq!(pm.peer_count(), 0);
        assert_eq!(pm.max_peers, 10);
    }

    #[test]
    fn test_peer_manager_height_update() {
        let pm = PeerManager::new(10);
        pm.update_height(42);
        assert_eq!(*pm.current_height.lock().unwrap(), 42);
    }

    #[test]
    fn test_peer_state_transitions() {
        let state = PeerState::Connected;
        assert_eq!(state, PeerState::Connected);

        let state = PeerState::Ready;
        assert_eq!(state, PeerState::Ready);
    }

    #[test]
    fn test_ban_score_threshold() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                drop(stream);
            }
        });

        let stream = TcpStream::connect(addr).unwrap();
        let mut peer = Peer::new(stream, addr).unwrap();

        assert_eq!(peer.ban_score, 0);
        assert!(!peer.should_ban());

        assert!(!peer.add_ban_score(50));
        assert_eq!(peer.ban_score, 50);
        assert!(!peer.should_ban());

        assert!(peer.add_ban_score(60));
        assert_eq!(peer.ban_score, 110);
        assert!(peer.should_ban());
    }
}
