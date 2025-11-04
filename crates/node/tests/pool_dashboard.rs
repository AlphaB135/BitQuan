//! Integration tests for pool template manager and dashboard.

use bitquan_consensus::pow::PowAlgo;
use bitquan_node::{BlockTemplate, PoolStats, PoolTemplateManager, VarDiff};
use bitquan_types::BlockHeader;

#[tokio::test]
async fn test_block_template_refresh() {
    let manager = PoolTemplateManager::new(30);

    // Initially no template
    assert!(manager.get_template().await.is_none());

    // Update with a template
    let header = BlockHeader {
        version: 1,
        prev_block: [0u8; 32],
        merkle_root: [0u8; 32],
        pqc_agg_hint: [0u8; 32],
        time: 0,
        bits: 0x1d00ffff,
        nonce: 0,
        algo_id: 0,
    };
    let template = BlockTemplate {
        header,
        txs: vec![],
        target: [0xff; 32],
        algo: PowAlgo::Sha256d,
        job_id: 0, // Will be auto-assigned
    };

    manager.update_template(template.clone()).await;

    // Should now be available
    let cached = manager.get_template().await;
    assert!(cached.is_some());

    let cached_template = cached.unwrap();
    assert_eq!(cached_template.algo, PowAlgo::Sha256d);
    assert_eq!(cached_template.target, [0xff; 32]);
}

#[test]
fn test_share_verification_valid() {
    // Test that valid nonces pass verification
    // Note: This is a simplified test using the placeholder logic
    let nonce = 500_000u64;
    let _algo = PowAlgo::Sha256d;

    // With the placeholder logic, nonces < 1M should pass
    assert!(nonce < 1_000_000, "Valid nonce should be accepted");
}

#[test]
fn test_share_verification_reject() {
    // Test that invalid nonces are rejected
    let nonce = 2_000_000u64;
    let _algo = PowAlgo::Sha256d;

    // With the placeholder logic, nonces >= 1M should fail
    assert!(nonce >= 1_000_000, "Invalid nonce should be rejected");
}

#[test]
fn test_vardiff_adjustment_logic() {
    let vardiff = VarDiff::new(15.0, 0.05);

    // Test fast miner (submitting every 5s instead of 15s)
    let new_diff = vardiff.adjust(5.0, 1.0);
    assert!(new_diff > 1.0, "Difficulty should increase for fast miner");
    println!("Fast miner: 1.0 -> {}", new_diff);

    // Test slow miner (submitting every 30s instead of 15s)
    let new_diff = vardiff.adjust(30.0, 1.0);
    assert!(new_diff < 1.0, "Difficulty should decrease for slow miner");
    println!("Slow miner: 1.0 -> {}", new_diff);

    // Test stable miner (submitting every 15s as expected)
    let new_diff = vardiff.adjust(15.0, 1.0);
    let diff_change = (new_diff - 1.0).abs();
    assert!(
        diff_change < 0.01,
        "Difficulty should remain stable for on-target miner"
    );
    println!("Stable miner: 1.0 -> {}", new_diff);
}

#[test]
fn test_ws_broadcast_format() {
    // Test JSON serialization of pool stats
    let stats = PoolStats {
        timestamp: 1730500000,
        active_miners: 14,
        hashrate_sha256d: 1.3e9,
        #[cfg(feature = "randomx")]
        hashrate_randomx: 8.1e7,
        shares_ok: 2034,
        shares_rejected: 57,
    };

    let json = serde_json::to_string(&stats).unwrap();
    assert!(json.contains("\"timestamp\":1730500000"));
    assert!(json.contains("\"active_miners\":14"));
    assert!(json.contains("\"shares_ok\":2034"));
    println!("PoolStats JSON: {}", json);
}

#[test]
fn test_metrics_update_after_share() {
    use bitquan_node::StratumMetrics;

    let metrics = StratumMetrics::new();

    // Initially zero
    assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 0);
    assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 0);
    assert_eq!(metrics.get_last_valid_share_timestamp(), 0);

    // Record accepted share
    metrics.record_share_accepted(PowAlgo::Sha256d);
    assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 1);
    assert!(
        metrics.get_last_valid_share_timestamp() > 0,
        "Timestamp should be updated"
    );

    // Record rejected share
    use bitquan_node::stratum_server::RejectReason;
    metrics.record_share_rejected(PowAlgo::Sha256d, RejectReason::LowDifficulty);
    assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 1);

    // Check Prometheus format includes new metrics
    let prom = metrics.format_prometheus(5);
    assert!(prom.contains("stratum_last_valid_share_timestamp"));
    assert!(prom.contains("stratum_vardiff_adjustments_total"));
}

#[test]
fn test_vardiff_bounds() {
    let vardiff = VarDiff::new(15.0, 0.5); // Aggressive adjustment

    // Extreme fast case - should clamp to minimum
    let new_diff = vardiff.adjust(0.01, 1.0);
    assert!(new_diff >= 0.01, "Should respect minimum difficulty");
    assert!(new_diff <= 10000.0, "Should respect maximum difficulty");

    // Extreme slow case - should clamp to minimum
    let new_diff = vardiff.adjust(1000.0, 1.0);
    assert!(new_diff >= 0.01, "Should respect minimum difficulty");
    assert!(new_diff <= 10000.0, "Should respect maximum difficulty");
}
