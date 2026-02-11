//! Peer discovery and management.

use crate::protocol::PeerAddr;
use bitquan_types::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Bootstrap seed nodes for testnet.
pub const TESTNET_SEEDS: &[&str] = &[
    "seed.testnet.bitquan.net:18444",
    "node1.testnet.bitquan.org:18444",
    "127.0.0.1:18444", // Localhost for testing
];

/// Bootstrap seed nodes for mainnet (placeholder).
pub const MAINNET_SEEDS: &[&str] = &["seed.bitquan.net:8333", "node1.bitquan.org:8333"];

/// Peer discovery timeout in seconds.
pub const DISCOVERY_TIMEOUT_SECS: u64 = 10;

/// Peer connection timeout in seconds.
pub const PEER_TIMEOUT_SECS: u64 = 120;

/// Peer ping interval in seconds.
pub const PING_INTERVAL_SECS: u64 = 60;

/// Persistent peer information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentPeer {
    /// Peer address.
    pub addr: String,
    /// Last seen timestamp (Unix seconds).
    pub last_seen: u64,
    /// Number of successful connections.
    pub successful_connections: u64,
    /// Number of failed connections.
    pub failed_connections: u64,
    /// Services provided.
    pub services: u64,
    /// Peer's claimed blockchain height (from version message).
    /// Used for Sybil attack protection - only trust heights that can be verified.
    pub claimed_height: Option<u64>,
}

impl PersistentPeer {
    /// Create a new persistent peer record.
    pub fn new(addr: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            addr,
            last_seen: now,
            successful_connections: 0,
            failed_connections: 0,
            services: 0,
            claimed_height: None,
        }
    }

    /// Update last seen timestamp.
    pub fn mark_seen(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Mark connection as successful.
    pub fn mark_success(&mut self) {
        self.successful_connections += 1;
        self.mark_seen();
    }

    /// Mark connection as failed.
    pub fn mark_failure(&mut self) {
        self.failed_connections += 1;
    }

    /// Calculate peer score (higher is better).
    pub fn score(&self) -> f64 {
        let total = (self.successful_connections + self.failed_connections) as f64;
        if total == 0.0 {
            return 0.0;
        }

        let success_rate = self.successful_connections as f64 / total;

        // Age penalty (prefer recently seen peers)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age_secs = now.saturating_sub(self.last_seen);
        let age_penalty = 1.0 / (1.0 + (age_secs as f64 / 3600.0)); // Decay over hours

        success_rate * age_penalty
    }

    /// Check if peer is stale (not seen recently).
    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.last_seen) > timeout_secs
    }
}

/// Peer address book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBook {
    /// Known peers indexed by address.
    peers: HashMap<String, PersistentPeer>,
}

impl PeerBook {
    /// Create a new empty peer book.
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    /// Add a peer to the book.
    pub fn add_peer(&mut self, addr: String) {
        if !self.peers.contains_key(&addr) {
            self.peers.insert(addr.clone(), PersistentPeer::new(addr));
        }
    }

    /// Get a peer by address.
    pub fn get_peer(&self, addr: &str) -> Option<&PersistentPeer> {
        self.peers.get(addr)
    }

    /// Get a mutable peer reference.
    pub fn get_peer_mut(&mut self, addr: &str) -> Option<&mut PersistentPeer> {
        self.peers.get_mut(addr)
    }

    /// Mark peer as seen.
    pub fn mark_peer_seen(&mut self, addr: &str) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.mark_seen();
        }
    }

    /// Mark peer connection as successful.
    pub fn mark_peer_success(&mut self, addr: &str) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.mark_success();
        }
    }

    /// Mark peer connection as failed.
    pub fn mark_peer_failure(&mut self, addr: &str) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.mark_failure();
        }
    }

    /// Get best peers sorted by score.
    pub fn best_peers(&self, limit: usize) -> Vec<String> {
        let mut scored: Vec<_> = self
            .peers
            .values()
            .filter(|p| !p.is_stale(PEER_TIMEOUT_SECS * 2))
            .map(|p| (p.addr.clone(), p.score()))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(limit)
            .map(|(addr, _)| addr)
            .collect()
    }

    /// Remove stale peers.
    pub fn prune_stale(&mut self, timeout_secs: u64) {
        self.peers.retain(|_, peer| !peer.is_stale(timeout_secs));
    }

    /// Get total peer count.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Convert to PeerAddr list for network protocol.
    pub fn to_peer_addrs(&self) -> Vec<PeerAddr> {
        self.peers
            .values()
            .map(|p| PeerAddr {
                timestamp: p.last_seen,
                services: p.services,
                ip: p.addr.clone(),
                port: 18444, // Default port
            })
            .collect()
    }

    /// Import peers from PeerAddr list.
    pub fn import_peer_addrs(&mut self, addrs: Vec<PeerAddr>) {
        for addr in addrs {
            let addr_str = format!("{}:{}", addr.ip, addr.port);

            if let Some(peer) = self.peers.get_mut(&addr_str) {
                peer.last_seen = addr.timestamp;
                peer.services = addr.services;
            } else {
                let mut peer = PersistentPeer::new(addr_str.clone());
                peer.last_seen = addr.timestamp;
                peer.services = addr.services;
                self.peers.insert(addr_str, peer);
            }
        }
    }

    /// Load from JSON file.
    pub fn load_from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(bitquan_types::Error::Io)?;

        let book: PeerBook =
            serde_json::from_str(&contents).map_err(bitquan_types::Error::Serde)?;

        Ok(book)
    }

    /// Save to JSON file.
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(bitquan_types::Error::Serde)?;

        std::fs::write(path, json).map_err(bitquan_types::Error::Io)?;

        Ok(())
    }
}

impl Default for PeerBook {
    fn default() -> Self {
        Self::new()
    }
}

/// Discover peers from DNS seeds.
///
/// This is a placeholder for actual DNS resolution.
/// In production, this would query DNS seeds and return resolved addresses.
pub fn discover_from_seeds(seeds: &[&str]) -> Result<Vec<String>> {
    // For now, just return the seeds as-is
    // DNS resolution implementation pending
    Ok(seeds.iter().map(|s| s.to_string()).collect())
}

/// Bootstrap the peer book with seed nodes.
pub fn bootstrap_peers(is_testnet: bool) -> PeerBook {
    let mut book = PeerBook::new();

    let seeds = if is_testnet {
        TESTNET_SEEDS
    } else {
        MAINNET_SEEDS
    };

    for seed in seeds {
        book.add_peer(seed.to_string());
    }

    book
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistent_peer_scoring() {
        let mut peer = PersistentPeer::new("127.0.0.1:18444".to_string());

        // Initial score should be 0
        assert_eq!(peer.score(), 0.0);

        // After some successes
        peer.mark_success();
        peer.mark_success();
        peer.mark_success();

        // Score should be positive
        assert!(peer.score() > 0.0);

        // After failure
        peer.mark_failure();

        // Score should decrease
        let score_after_failure = peer.score();
        assert!(score_after_failure < 1.0);
    }

    #[test]
    fn test_peer_book_best_peers() {
        let mut book = PeerBook::new();

        book.add_peer("peer1:18444".to_string());
        book.add_peer("peer2:18444".to_string());
        book.add_peer("peer3:18444".to_string());

        // Mark peer2 as successful multiple times
        book.mark_peer_success("peer2:18444");
        book.mark_peer_success("peer2:18444");
        book.mark_peer_success("peer2:18444");

        // Mark peer1 as successful once
        book.mark_peer_success("peer1:18444");

        // Best peers should include both peer2 and peer1
        let best = book.best_peers(3);
        assert!(best.len() >= 2);
        assert!(best.contains(&"peer2:18444".to_string()));
        assert!(best.contains(&"peer1:18444".to_string()));
    }

    #[test]
    fn test_peer_book_persistence() {
        let mut book = PeerBook::new();
        book.add_peer("test:18444".to_string());
        book.mark_peer_success("test:18444");

        let temp_file = if cfg!(windows) {
            std::env::temp_dir()
                .join("bitquan_peers_test.json")
                .to_string_lossy()
                .to_string()
        } else {
            "/tmp/bitquan_peers_test.json".to_string()
        };

        // Save
        book.save_to_file(&temp_file)
            .expect("Failed to save peer book");

        // Load
        let loaded = PeerBook::load_from_file(&temp_file).expect("Failed to load peer book");

        assert_eq!(loaded.peer_count(), 1);
        assert!(loaded.get_peer("test:18444").is_some());

        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_bootstrap_peers() {
        let book = bootstrap_peers(true);
        assert!(book.peer_count() > 0);
    }

    #[test]
    fn test_import_peer_addrs() {
        let mut book = PeerBook::new();

        let addrs = vec![
            PeerAddr {
                timestamp: 1234567890,
                services: 1,
                ip: "192.168.1.1".to_string(),
                port: 18444,
            },
            PeerAddr {
                timestamp: 1234567891,
                services: 1,
                ip: "192.168.1.2".to_string(),
                port: 18444,
            },
        ];

        book.import_peer_addrs(addrs);

        assert_eq!(book.peer_count(), 2);
        assert!(book.get_peer("192.168.1.1:18444").is_some());
    }
}
