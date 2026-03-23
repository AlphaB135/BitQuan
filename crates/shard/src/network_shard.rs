//! Shard Network - Handles network communication for sharded architecture

use crate::{ShardError, CrossShardMessage, CrossShardResponse};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::net::{TcpListener, TcpStream};
use futures::stream::StreamExt;

/// Manages network communication for a shard
pub struct ShardNetwork {
    local_shard_id: u16,
    total_shards: u16,
    listener: TcpListener,
    peer_connections: Arc<RwLock<HashMap<u16, PeerConnection>>>,
    message_queue: Arc<RwLock<Vec<InboundMessage>>>,
    network_tx: broadcast::Sender<NetworkMessage>,
}

/// Represents a connection to another shard
pub struct PeerConnection {
    pub shard_id: u16,
    pub address: SocketAddr,
    pub connection: TcpStream,
    pub message_queue: mpsc::Sender<OutboundMessage>,
    pub last_seen: std::time::Instant,
    pub is_connected: bool,
}

/// Inbound message from network
#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub from_shard: u16,
    pub message: CrossShardMessage,
    pub received_at: std::time::Instant,
}

/// Outbound message to network
#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub to_shard: u16,
    pub message: CrossShardMessage,
}

/// Network control message
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    PeerConnected(u16, SocketAddr),
    PeerDisconnected(u16),
    MessageReceived(InboundMessage),
    NetworkError(String),
}

impl ShardNetwork {
    /// Create a new shard network
    pub async fn new(
        local_shard_id: u16,
        total_shards: u16,
        listen_port: u16,
    ) -> Result<Self, ShardError> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))
            .await
            .map_err(|e| ShardError::NetworkError(e.to_string()))?;

        let (network_tx, _) = broadcast::channel(1000);

        Ok(Self {
            local_shard_id,
            total_shards,
            listener,
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            network_tx,
        })
    }

    /// Start the network server
    pub async fn start(&self) {
        let listener = self.listener.clone();
        let message_queue = self.message_queue.clone();
        let network_tx = self.network_tx.clone();

        // Accept incoming connections
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        if let Err(e) = Self::handle_incoming_connection(stream, addr, &message_queue, &network_tx).await {
                            eprintln!("Error handling incoming connection: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
                    }
                }
            }
        });
    }

    /// Connect to another shard
    pub async fn connect_to_peer(&self, shard_id: u16, address: SocketAddr) -> Result<(), ShardError> {
        if shard_id == self.local_shard_id {
            return Err(ShardError::InvalidShardId(shard_id));
        }

        // Check if already connected
        {
            let connections = self.peer_connections.read().await;
            if connections.contains_key(&shard_id) {
                return Ok(());
            }
        }

        // Establish connection
        let stream = TcpStream::connect(address)
            .await
            .map_err(|e| ShardError::NetworkError(e.to_string()))?;

        // Create message queue for this peer
        let (tx, mut rx) = mpsc::channel(1000);

        // Add to peer connections
        let connection = PeerConnection {
            shard_id,
            address,
            connection: stream,
            message_queue: tx,
            last_seen: std::time::Instant::now(),
            is_connected: true,
        };

        {
            let mut connections = self.peer_connections.write().await;
            connections.insert(shard_id, connection);
        }

        // Notify network manager
        let _ = network_tx.send(NetworkMessage::PeerConnected(shard_id, address));

        // Start message receiver for this peer
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                // Send message to peer
                if let Err(e) = Self::send_message_to_peer(&msg).await {
                    eprintln!("Error sending message to peer: {}", e);
                    break;
                }
            }
        });

        Ok(())
    }

    /// Send a message to another shard
    pub async fn send_message(&self, to_shard: u16, message: CrossShardMessage) -> Result<(), ShardError> {
        // Find peer connection
        let connection = {
            let connections = self.peer_connections.read().await;
            connections.get(&to_shard).cloned()
        };

        match connection {
            Some(conn) => {
                // Add to queue
                let outbound = OutboundMessage {
                    to_shard,
                    message,
                };

                if conn.message_queue.send(outbound).await.is_err() {
                    // Connection closed, remove it
                    let mut connections = self.peer_connections.write().await;
                    connections.remove(&to_shard);
                    return Err(ShardError::NetworkError("Connection closed".into()));
                }

                Ok(())
            }
            None => {
                // Try to connect first
                if let Ok(_) = self.connect_to_peer(to_shard, format!("{}:{}", to_shard, 10000 + to_shard).parse().unwrap()) {
                    // Retry sending
                    self.send_message(to_shard, message).await
                } else {
                    Err(ShardError::NetworkError("No such peer".into()))
                }
            }
        }
    }

    /// Broadcast message to all shards
    pub async fn broadcast_message(&self, message: CrossShardMessage) -> Result<(), ShardError> {
        for shard_id in 0..self.total_shards {
            if shard_id != self.local_shard_id {
                if let Err(e) = self.send_message(shard_id, message.clone()).await {
                    eprintln!("Failed to send message to shard {}: {}", shard_id, e);
                }
            }
        }
        Ok(())
    }

    /// Handle incoming connection
    async fn handle_incoming_connection(
        stream: TcpStream,
        addr: SocketAddr,
        message_queue: &Arc<RwLock<Vec<InboundMessage>>>,
        network_tx: &broadcast::Sender<NetworkMessage>,
    ) -> Result<(), ShardError> {
        // In a real implementation, this would:
        // 1. Perform handshake to identify shard
        // 2. Authenticate connection
        // 3. Start message reader/writer tasks

        // For now, just close the connection
        Ok(())
    }

    /// Send message to peer (mock implementation)
    async fn send_message_to_peer(_msg: &OutboundMessage) -> Result<(), ShardError> {
        // In a real implementation, this would serialize and send the message
        Ok(())
    }

    /// Get all peer connections
    pub async fn get_peer_connections(&self) -> HashMap<u16, PeerConnection> {
        self.peer_connections.read().await.clone()
    }

    /// Check if a shard is connected
    pub async fn is_shard_connected(&self, shard_id: u16) -> bool {
        let connections = self.peer_connections.read().await;
        connections.contains_key(&shard_id)
    }

    /// Get network statistics
    pub async fn get_stats(&self) -> NetworkStats {
        let connections = self.peer_connections.read().await;
        let message_queue = self.message_queue.read().await;

        NetworkStats {
            local_shard_id: self.local_shard_id,
            total_shards: self.total_shards,
            connected_peers: connections.len(),
            pending_messages: message_queue.len(),
            is_listening: true, // Would check if listener is active
        }
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub local_shard_id: u16,
    pub total_shards: u16,
    pub connected_peers: usize,
    pub pending_messages: usize,
    pub is_listening: bool,
}

/// Peer management for sharded network
pub struct PeerManager {
    network: Arc<ShardNetwork>,
    known_peers: Arc<RwLock<HashMap<u16, Vec<SocketAddr>>>>,
    health_check_interval: std::time::Duration,
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(network: Arc<ShardNetwork>) -> Self {
        Self {
            network,
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            health_check_interval: std::time::Duration::from_secs(30),
        }
    }

    /// Add known peer addresses for a shard
    pub async fn add_peer_addresses(&self, shard_id: u16, addresses: Vec<SocketAddr>) {
        let mut peers = self.known_peers.write().await;
        peers.insert(shard_id, addresses);
    }

    /// Connect to known peers
    pub async fn connect_to_known_peers(&self) -> Result<(), ShardError> {
        let peers = self.known_peers.read().await;
        let mut tasks = Vec::new();

        for (shard_id, addresses) in peers.iter() {
            // Try each address
            for addr in addresses {
                let network = self.network.clone();
                let shard_id = *shard_id;
                let addr = *addr;

                let task = tokio::spawn(async move {
                    if let Err(e) = network.connect_to_peer(shard_id, addr).await {
                        eprintln!("Failed to connect to shard {} at {}: {}", shard_id, addr, e);
                    }
                });

                tasks.push(task);
            }
        }

        // Wait for all connection attempts
        for task in tasks {
            task.await?;
        }

        Ok(())
    }

    /// Start health check routine
    pub async fn start_health_checks(&self) {
        let network = self.network.clone();
        let known_peers = self.known_peers.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(network.health_check_interval).await;

                // Check peer connectivity
                let connections = network.get_peer_connections().await;
                let peers = known_peers.read().await;

                for (shard_id, _) in peers.iter() {
                    if !connections.contains_key(shard_id) {
                        // Try to reconnect
                        if let Some(addresses) = peers.get(shard_id) {
                            for addr in addresses {
                                if let Err(e) = network.connect_to_peer(*shard_id, *addr).await {
                                    eprintln!("Failed to reconnect to shard {}: {}", shard_id, e);
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Get peer statistics
    pub async fn get_peer_stats(&self) -> PeerStats {
        let network_stats = self.network.get_stats().await;
        let known_peers = self.known_peers.read().await;

        PeerStats {
            local_shard_id: network_stats.local_shard_id,
            total_shards: network_stats.total_shards,
            known_peers: known_peers.len(),
            connected_peers: network_stats.connected_peers,
            total_peer_addresses: known_peers.values().map(|addrs| addrs.len()).sum(),
        }
    }
}

/// Peer statistics
#[derive(Debug, Clone)]
pub struct PeerStats {
    pub local_shard_id: u16,
    pub total_shards: u16,
    pub known_peers: usize,
    pub connected_peers: usize,
    pub total_peer_addresses: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_creation() {
        let network = ShardNetwork::new(0, 4, 12345).await.unwrap();
        assert_eq!(network.local_shard_id, 0);
        assert_eq!(network.total_shards, 4);
    }

    #[test]
    fn test_peer_manager() {
        let network = Arc::new(ShardNetwork::new(0, 4, 12345).await.unwrap());
        let manager = PeerManager::new(network.clone());

        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        tokio::spawn(async move {
            manager.add_peer_addresses(1, vec![addr]).await;
            manager.connect_to_known_peers().await.unwrap();
        });
    }
}