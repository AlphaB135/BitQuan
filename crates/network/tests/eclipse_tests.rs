//! Tests for eclipse attack mitigation

use bitquan_network::{EclipseConfig, PeerManager};
use std::net::SocketAddr;

#[test]
fn test_subnet_diversity_enforcement() {
    let config = EclipseConfig {
        max_peers_per_subnet: 2,
        anchor_peers: vec![],
        enforce_subnet_diversity: true,
    };

    let pm = PeerManager::with_eclipse_config(10, None, config);

    // Try to connect multiple peers from same subnet
    // Note: This is a simplified test - real implementation would need actual connections
    assert!(pm.is_subnet_diversity_enforced());
}

#[test]
fn test_anchor_peers_config() {
    let anchor: SocketAddr = "127.0.0.1:8333".parse().expect("Failed to parse socket address");

    let config = EclipseConfig {
        max_peers_per_subnet: 2,
        anchor_peers: vec![anchor],
        enforce_subnet_diversity: true,
    };

    let pm = PeerManager::with_eclipse_config(10, None, config);

    let anchors = pm.get_anchors();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0], anchor);
}

#[test]
fn test_subnet_stats_empty() {
    let pm = PeerManager::new(10);
    let stats = pm.get_subnet_stats();
    assert!(stats.is_empty());
}

#[test]
fn test_evict_no_peers() {
    let pm = PeerManager::new(10);
    let result = pm.evict_lowest_reputation_peer();
    assert!(result.is_none());
}
