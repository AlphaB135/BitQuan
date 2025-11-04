//! Integration tests for Stratum mining server.

use bitquan_consensus::pow::PowAlgo;
use bitquan_node::stratum_server::{MinerSession, StratumConfig, StratumMetrics, StratumServer};
use bitquan_types::NetworkId;

#[test]
fn miner_session_lifecycle() {
    let session = MinerSession::new(PowAlgo::Sha256d, "miner1@pool".to_string(), 1.0);
    
    assert_eq!(session.algo, PowAlgo::Sha256d);
    assert_eq!(session.address, "miner1@pool");
    assert_eq!(session.difficulty, 1.0);
    assert_eq!(session.get_accepted(), 0);
    assert_eq!(session.get_rejected(), 0);
    
    // Test share acceptance
    session.accept_share();
    session.accept_share();
    session.accept_share();
    assert_eq!(session.get_accepted(), 3);
    
    // Test share rejection
    session.reject_share();
    assert_eq!(session.get_rejected(), 1);
    
    // Verify counters are independent
    assert_eq!(session.get_accepted(), 3);
}

#[test]
fn stratum_metrics_initialization() {
    let metrics = StratumMetrics::new();
    
    assert_eq!(metrics.get_connections_total(), 0);
    assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 0);
    assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 0);
    
    #[cfg(feature = "randomx")]
    {
        assert_eq!(metrics.get_accepted(PowAlgo::RandomX), 0);
        assert_eq!(metrics.get_rejected(PowAlgo::RandomX), 0);
    }
}

#[test]
fn stratum_metrics_recording() {
    let metrics = StratumMetrics::new();
    
    // Record some SHA-256d shares
    metrics.record_share_accepted(PowAlgo::Sha256d);
    metrics.record_share_accepted(PowAlgo::Sha256d);
    metrics.record_share_accepted(PowAlgo::Sha256d);
    assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 3);
    
    metrics.record_share_rejected(PowAlgo::Sha256d);
    metrics.record_share_rejected(PowAlgo::Sha256d);
    assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 2);
    
    #[cfg(feature = "randomx")]
    {
        // Record some RandomX shares
        metrics.record_share_accepted(PowAlgo::RandomX);
        metrics.record_share_accepted(PowAlgo::RandomX);
        assert_eq!(metrics.get_accepted(PowAlgo::RandomX), 2);
        
        metrics.record_share_rejected(PowAlgo::RandomX);
        assert_eq!(metrics.get_rejected(PowAlgo::RandomX), 1);
    }
}

#[test]
fn stratum_prometheus_format() {
    let metrics = StratumMetrics::new();
    
    metrics.record_share_accepted(PowAlgo::Sha256d);
    metrics.record_share_rejected(PowAlgo::Sha256d);
    
    let output = metrics.format_prometheus(5);
    
    // Check for required metric lines
    assert!(output.contains("stratum_connections_total"));
    assert!(output.contains("stratum_shares_total"));
    assert!(output.contains("stratum_active_miners"));
    assert!(output.contains("algo=\"sha256d\""));
    assert!(output.contains("status=\"ok\""));
    assert!(output.contains("status=\"reject\""));
    assert!(output.contains("stratum_active_miners 5"));
}

#[test]
fn stratum_config_defaults() {
    let config = StratumConfig::default();
    
    assert_eq!(config.bind_addr, "0.0.0.0:3333");
    assert_eq!(config.allow_list, vec!["127.0.0.1"]);
    assert_eq!(config.default_difficulty, 1.0);
    assert_eq!(config.network, NetworkId::Devnet);
}

#[test]
fn stratum_server_creation() {
    let config = StratumConfig {
        bind_addr: "127.0.0.1:13333".to_string(),
        allow_list: vec!["127.0.0.1".to_string(), "192.168.1.0/24".to_string()],
        default_difficulty: 2.0,
        network: NetworkId::Testnet,
        enable_vardiff: true,
        vardiff_target_time: 15.0,
        vardiff_adjust_rate: 0.05,
    };
    
    let server = StratumServer::new(config.clone());
    
    assert_eq!(server.active_miners(), 0);
    let metrics = server.metrics();
    assert_eq!(metrics.get_connections_total(), 0);
}

#[tokio::test]
async fn stratum_server_lifecycle() {
    let config = StratumConfig {
        bind_addr: "127.0.0.1:0".to_string(), // Random port
        allow_list: vec!["127.0.0.1".to_string()],
        default_difficulty: 1.0,
        network: NetworkId::Devnet,
        enable_vardiff: true,
        vardiff_target_time: 15.0,
        vardiff_adjust_rate: 0.05,
    };
    
    let mut server = StratumServer::new(config);
    
    // Start server in background
    let handle = tokio::spawn(async move {
        // Server will run until stopped or error
        let _ = server.start().await;
    });
    
    // Let server initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Abort the server task
    handle.abort();
}

#[cfg(feature = "randomx")]
#[test]
fn randomx_share_metrics() {
    let metrics = StratumMetrics::new();
    
    // Test RandomX-specific metrics
    metrics.record_share_accepted(PowAlgo::RandomX);
    metrics.record_share_accepted(PowAlgo::RandomX);
    metrics.record_share_accepted(PowAlgo::RandomX);
    
    assert_eq!(metrics.get_accepted(PowAlgo::RandomX), 3);
    
    metrics.record_share_rejected(PowAlgo::RandomX);
    assert_eq!(metrics.get_rejected(PowAlgo::RandomX), 1);
    
    let output = metrics.format_prometheus(2);
    assert!(output.contains("algo=\"randomx\""));
}

#[test]
fn multiple_miners_tracking() {
    let session1 = MinerSession::new(PowAlgo::Sha256d, "miner1".to_string(), 1.0);
    let session2 = MinerSession::new(PowAlgo::Sha256d, "miner2".to_string(), 2.0);
    
    #[cfg(feature = "randomx")]
    let session3 = MinerSession::new(PowAlgo::RandomX, "miner3".to_string(), 1.0);
    
    // Simulate activity
    session1.accept_share();
    session1.accept_share();
    session2.accept_share();
    session2.reject_share();
    
    assert_eq!(session1.get_accepted(), 2);
    assert_eq!(session1.get_rejected(), 0);
    assert_eq!(session2.get_accepted(), 1);
    assert_eq!(session2.get_rejected(), 1);
    
    #[cfg(feature = "randomx")]
    {
        session3.accept_share();
        assert_eq!(session3.get_accepted(), 1);
    }
}

#[test]
fn metrics_concurrent_updates() {
    use std::sync::Arc;
    use std::thread;
    
    let metrics = Arc::new(StratumMetrics::new());
    let mut handles = vec![];
    
    // Spawn multiple threads updating metrics
    for _ in 0..10 {
        let metrics_clone = Arc::clone(&metrics);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                metrics_clone.record_share_accepted(PowAlgo::Sha256d);
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify all updates were recorded
    assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 1000);
}
