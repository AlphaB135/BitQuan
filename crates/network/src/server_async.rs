//! Async P2P server implementation with tokio.
//!
//! This module provides an async TCP listener for accepting peer connections
//! using tokio's async runtime.

use crate::peer_async::AsyncPeerManager;
use crate::protocol::P2pError;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Async P2P listener for accepting incoming peer connections.
pub struct AsyncP2PListener {
    listener: TcpListener,
    peer_manager: Arc<AsyncPeerManager>,
}

impl AsyncP2PListener {
    /// Creates a new async P2P listener bound to the specified address.
    pub async fn bind(addr: &str, peer_manager: Arc<AsyncPeerManager>) -> Result<Self, P2pError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        Ok(AsyncP2PListener {
            listener,
            peer_manager,
        })
    }

    /// Returns the local address the listener is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, P2pError> {
        self.listener
            .local_addr()
            .map_err(|e| P2pError::ConnectionError(e.to_string()))
    }

    /// Accepts a single incoming connection.
    ///
    /// This method is async and will wait for a connection.
    pub async fn accept_one(&self) -> Result<(), P2pError> {
        let (stream, addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        self.peer_manager.add_peer_inbound(stream, addr).await?;
        Ok(())
    }

    /// Runs the accept loop, spawning a new task for each peer.
    ///
    /// CRITICAL: This function spawns tokio tasks for each peer connection.
    /// Each peer is handled concurrently without blocking the listener.
    ///
    /// This solves the Slowloris attack because:
    /// 1. Each peer runs in its own lightweight task (4KB memory)
    /// 2. Slow peers don't block the listener from accepting new connections
    /// 3. tokio::time::timeout in each task enforces total read timeout
    pub async fn run_accept_loop(self: Arc<Self>) {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    let peer_manager = Arc::clone(&self.peer_manager);

                    // Spawn a new task for this peer connection
                    tokio::spawn(async move {
                        match peer_manager.add_peer_inbound(stream, addr).await {
                            Ok(peer_height) => {
                                if let Some(height) = peer_height {
                                    log::info!(
                                        "Peer {} connected with claimed height {}",
                                        addr,
                                        height
                                    );
                                } else {
                                    log::info!("Peer {} connected (no height claimed)", addr);
                                }
                                // Note: PeerBook is automatically updated if set via set_peer_book()
                            }
                            Err(e) => {
                                log::warn!("Failed to add peer {}: {}", addr, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to accept connection: {}", e);
                    // Continue accepting despite errors
                }
            }
        }
    }

    /// Runs the accept loop with a connection limit check.
    ///
    /// This version checks peer count before accepting to prevent
    /// resource exhaustion from too many connections.
    pub async fn run_accept_loop_with_limit(self: Arc<Self>, max_connections: usize) {
        loop {
            // Check current peer count
            let current_peers = self.peer_manager.peer_count().await;

            if current_peers >= max_connections {
                // Wait a bit before checking again
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    let peer_manager = Arc::clone(&self.peer_manager);

                    tokio::spawn(async move {
                        match peer_manager.add_peer_inbound(stream, addr).await {
                            Ok(peer_height) => {
                                if let Some(height) = peer_height {
                                    log::info!(
                                        "Peer {} connected with claimed height {}",
                                        addr,
                                        height
                                    );
                                } else {
                                    log::info!("Peer {} connected (no height claimed)", addr);
                                }
                                // Note: PeerBook is automatically updated if set via set_peer_book()
                            }
                            Err(e) => {
                                log::warn!("Failed to add peer {}: {}", addr, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// Spawns the P2P server as a background task.
///
/// This is the recommended way to run the P2P server in production.
/// It returns immediately, allowing the main task to continue.
///
/// Example:
/// ```no_run
/// use bitquan_network::server_async::spawn_p2p_server;
/// use bitquan_network::peer_async::AsyncPeerManager;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let peer_manager = Arc::new(AsyncPeerManager::new(100, bitquan_types::NetworkId::Mainnet));
///
///     // Spawn P2P server in background
///     spawn_p2p_server("0.0.0.0:8333", peer_manager.clone()).await.unwrap();
///
///     // Server is now running in background
///     // Main task can continue with other work
/// }
/// ```
pub async fn spawn_p2p_server(
    addr: &str,
    peer_manager: Arc<AsyncPeerManager>,
) -> Result<(), P2pError> {
    let listener = Arc::new(AsyncP2PListener::bind(addr, peer_manager).await?);

    log::info!("P2P server listening on {}", listener.local_addr()?);

    // Spawn the accept loop as a background task
    tokio::spawn(async move {
        listener.run_accept_loop().await;
    });

    Ok(())
}

/// Spawns the P2P server with a connection limit.
pub async fn spawn_p2p_server_with_limit(
    addr: &str,
    peer_manager: Arc<AsyncPeerManager>,
    max_connections: usize,
) -> Result<(), P2pError> {
    let listener = Arc::new(AsyncP2PListener::bind(addr, peer_manager).await?);

    log::info!(
        "P2P server listening on {} (max {} connections)",
        listener.local_addr()?,
        max_connections
    );

    tokio::spawn(async move {
        listener.run_accept_loop_with_limit(max_connections).await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::NetworkId;

    #[tokio::test]
    async fn test_listener_bind() {
        let peer_manager = Arc::new(AsyncPeerManager::new(10, NetworkId::Devnet));
        let listener = AsyncP2PListener::bind("127.0.0.1:0", peer_manager).await;
        assert!(listener.is_ok());
    }

    #[tokio::test]
    async fn test_local_addr() {
        let peer_manager = Arc::new(AsyncPeerManager::new(10, NetworkId::Devnet));
        let listener = AsyncP2PListener::bind("127.0.0.1:0", peer_manager)
            .await
            .expect("Failed to bind listener in test");
        let addr = listener
            .local_addr()
            .expect("Failed to get local address in test");
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }
}
