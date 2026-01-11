//! Tests for eclipse attack mitigation

use bitquan_network::{EclipseConfig, NoiseConfig, PeerManager};
use bitquan_types::NetworkId;
use std::net::SocketAddr;
use std::sync::Arc;

/// Create a test NoiseConfig for unit tests.
fn test_noise_config() -> Arc<NoiseConfig> {
    Arc::new(NoiseConfig::generate().expect("Failed to generate test noise config"))
}

#[test]
fn test_subnet_diversity_enforcement() {
    let config = EclipseConfig {
        max_peers_per_subnet: 2,
        anchor_peers: vec![],
        enforce_subnet_diversity: true,
    };

    let noise_config = test_noise_config();
    let pm = PeerManager::with_eclipse_config(10, None, config, NetworkId::Regtest, noise_config);

    // Try to connect multiple peers from same subnet
    // Note: This is a simplified test - real implementation would need actual connections
    assert!(pm.is_subnet_diversity_enforced());
}

#[test]
fn test_anchor_peers_config() {
    let anchor: SocketAddr = "127.0.0.1:8333"
        .parse()
        .expect("Failed to parse socket address");

    let config = EclipseConfig {
        max_peers_per_subnet: 2,
        anchor_peers: vec![anchor],
        enforce_subnet_diversity: true,
    };

    let noise_config = test_noise_config();
    let pm = PeerManager::with_eclipse_config(10, None, config, NetworkId::Regtest, noise_config);

    let anchors = pm.get_anchors();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0], anchor);
}

#[tokio::test]
async fn test_subnet_stats_empty() {
    let noise_config = test_noise_config();
    let pm = PeerManager::new(10, NetworkId::Regtest, noise_config);
    let stats = pm.get_subnet_stats().await;
    assert!(stats.is_empty());
}

#[tokio::test]
async fn test_evict_no_peers() {
    let noise_config = test_noise_config();
    let pm = PeerManager::new(10, NetworkId::Regtest, noise_config);
    let result = pm.evict_lowest_reputation_peer().await;
    assert!(result.is_none());
}
