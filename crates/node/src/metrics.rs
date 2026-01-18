// Prometheus metric registration uses expect() because duplicate registration
// indicates a bug in the code (conflicting metric names).
#![allow(clippy::expect_used)]

use lazy_static::lazy_static;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, IntCounter, IntCounterVec,
    IntGauge,
};
use std::sync::OnceLock;
use warp::Filter;

/// Mining metrics (stub for test compatibility)
/// TODO: Implement actual mining metrics tracking
#[derive(Debug, Clone)]
#[allow(dead_code)] // Placeholder for future implementation
pub struct MiningMetrics {
    _phantom: std::marker::PhantomData<()>,
}

#[allow(dead_code)] // Placeholder for future implementation
impl MiningMetrics {
    pub fn new(_algos: &[bitquan_consensus::pow::PowAlgo]) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    // Stub methods for test compatibility
    pub fn record_block_persisted(&self, _height: u64) {}
    pub fn set_total_rewards(&self, _rewards: u128) {}
    pub fn set_pool_balance(&self, _balance: u128) {}
    pub fn set_reward_per_block(&self, _reward: u128) {}
    pub fn record_block_mined(&self) {}
    pub fn record_hash_attempts(&self, _attempts: u64) {}
    pub fn get_blocks_mined(&self) -> u64 {
        0
    }
    pub fn get_hash_attempts(&self) -> u64 {
        0
    }
}

// Prometheus metric registration uses expect() because duplicate registration
// indicates a bug in the code (conflicting metric names).
lazy_static! {
    // Global metrics
    static ref BLOCK_HEIGHT: IntGauge = register_int_gauge!(
        "block_height",
        "Current blockchain height"
    ).expect("failed to register block_height metric");

    static ref CONNECTED_PEERS: IntGauge = register_int_gauge!(
        "connected_peers",
        "Number of connected peers"
    ).expect("failed to register connected_peers metric");

    static ref MEMPOOL_SIZE: IntGauge = register_int_gauge!(
        "mempool_size",
        "Number of transactions in mempool"
    ).expect("failed to register mempool_size metric");

    static ref TOTAL_REORGS: IntCounter = register_int_counter!(
        "total_reorgs",
        "Total number of chain reorganizations"
    ).expect("failed to register total_reorgs metric");

    static ref BAN_SCORE_EVENTS: IntCounterVec = register_int_counter_vec!(
        "ban_score_events",
        "Peer ban events by reason",
        &["reason"]
    ).expect("failed to register ban_score_events metric");
}

/// Global OnceLock to track running metrics servers per port
#[allow(dead_code)]
static METRICS_SERVERS: OnceLock<std::sync::Mutex<std::collections::HashMap<u16, bool>>> =
    OnceLock::new();

#[allow(dead_code)]
pub fn start_metrics_server(port: u16) -> Result<tokio::task::JoinHandle<()>, String> {
    // Get or initialize the servers map
    let servers =
        METRICS_SERVERS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));

    // Check if server is already running on this port
    {
        let mut running = servers
            .lock()
            .map_err(|e| format!("lock poisoned: {}", e))?;
        if running.contains_key(&port) {
            return Err(format!("Metrics server already running on port {}", port));
        }
        running.insert(port, true);
    }

    let metrics_route = warp::path!("metrics").map(|| {
        use prometheus::TextEncoder;
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let metric_string = encoder
            .encode_to_string(&metric_families)
            .unwrap_or_else(|e| format!("Error encoding metrics: {}", e));

        // Build custom response with correct content-type
        warp::http::Response::builder()
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .body(metric_string)
            .expect("failed to build HTTP response for metrics endpoint")
    });

    Ok(tokio::spawn(async move {
        warp::serve(metrics_route).run(([127, 0, 0, 1], port)).await;
    }))
}

// Helper functions for updating metrics
pub fn update_block_height(height: u64) {
    BLOCK_HEIGHT.set(height as i64);
}

pub fn update_connected_peers(count: usize) {
    CONNECTED_PEERS.set(count as i64);
}

pub fn update_mempool_size(size: usize) {
    MEMPOOL_SIZE.set(size as i64);
}

pub fn increment_reorg_counter() {
    TOTAL_REORGS.inc();
}

pub fn increment_ban_event(reason: &str) {
    BAN_SCORE_EVENTS.with_label_values(&[reason]).inc();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_update() {
        update_block_height(100);
        update_connected_peers(5);
        update_mempool_size(10);
        increment_reorg_counter();
        increment_ban_event("misbehavior");

        // Verify metrics are updated (basic check)
        assert_eq!(BLOCK_HEIGHT.get(), 100);
        assert_eq!(CONNECTED_PEERS.get(), 5);
        assert_eq!(MEMPOOL_SIZE.get(), 10);
        assert!(TOTAL_REORGS.get() > 0);
        assert!(BAN_SCORE_EVENTS.with_label_values(&["misbehavior"]).get() > 0);
    }
}
