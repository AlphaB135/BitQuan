// Prometheus metrics with lazy_static initialization.
//
// CLIPPY EXPECT_USED JUSTIFICATION:
// - Duplicate metric registration is a programmer error (conflicting metric names)
// - lazy_static! doesn't support fallible initialization with ? operator
// - This runs once at startup; panic here is appropriate for configuration bugs
// - All expect() messages describe exactly what went wrong for debugging
//
// Therefore, clippy::expect_used is allowed for this module.

#![expect(clippy::expect_used)]

use lazy_static::lazy_static;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, IntCounter, IntCounterVec,
    IntGauge,
};

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

    // Traction metrics
    static ref TOTAL_TRANSACTIONS: IntCounter = register_int_counter!(
        "bitquan_total_transactions",
        "Total number of transactions processed"
    ).expect("failed to register total_transactions metric");

    static ref BLOCKS_PER_HOUR: IntGauge = register_int_gauge!(
        "bitquan_blocks_per_hour",
        "Blocks mined in the last hour"
    ).expect("failed to register blocks_per_hour metric");

    static ref AVG_BLOCK_TIME_SECONDS: IntGauge = register_int_gauge!(
        "bitquan_avg_block_time_seconds",
        "Average block time in seconds (last 100 blocks)"
    ).expect("failed to register avg_block_time_seconds metric");

    static ref ACTIVE_MINERS: IntGauge = register_int_gauge!(
        "bitquan_active_miners",
        "Number of unique miners in the last 24 hours"
    ).expect("failed to register active_miners metric");

    static ref NETWORK_HASHRATE: IntGauge = register_int_gauge!(
        "bitquan_network_hashrate_hps",
        "Estimated network hashrate in hashes per second"
    ).expect("failed to register network_hashrate metric");

    static ref TOTAL_BLOCKS_MINED: IntCounter = register_int_counter!(
        "bitquan_total_blocks_mined",
        "Total number of blocks mined since genesis"
    ).expect("failed to register total_blocks_mined metric");

    static ref UPTIME_SECONDS: IntGauge = register_int_gauge!(
        "bitquan_uptime_seconds",
        "Node uptime in seconds"
    ).expect("failed to register uptime_seconds metric");
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

// Traction metric helpers
pub fn increment_total_transactions() {
    TOTAL_TRANSACTIONS.inc();
}

pub fn update_blocks_per_hour(count: u64) {
    BLOCKS_PER_HOUR.set(count as i64);
}

pub fn update_avg_block_time(seconds: u64) {
    AVG_BLOCK_TIME_SECONDS.set(seconds as i64);
}

pub fn update_active_miners(count: u64) {
    ACTIVE_MINERS.set(count as i64);
}

pub fn update_network_hashrate(hps: u64) {
    NETWORK_HASHRATE.set(hps as i64);
}

pub fn increment_total_blocks_mined() {
    TOTAL_BLOCKS_MINED.inc();
}

pub fn update_uptime(seconds: u64) {
    UPTIME_SECONDS.set(seconds as i64);
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

        // Traction metrics
        increment_total_transactions();
        update_blocks_per_hour(6);
        update_avg_block_time(600);
        update_active_miners(3);
        update_network_hashrate(1_000_000);
        increment_total_blocks_mined();
        update_uptime(3600);

        // Verify core metrics are updated
        assert_eq!(BLOCK_HEIGHT.get(), 100);
        assert_eq!(CONNECTED_PEERS.get(), 5);
        assert_eq!(MEMPOOL_SIZE.get(), 10);
        assert!(TOTAL_REORGS.get() > 0);
        assert!(BAN_SCORE_EVENTS.with_label_values(&["misbehavior"]).get() > 0);

        // Verify traction metrics
        assert_eq!(BLOCKS_PER_HOUR.get(), 6);
        assert_eq!(AVG_BLOCK_TIME_SECONDS.get(), 600);
        assert_eq!(ACTIVE_MINERS.get(), 3);
        assert_eq!(NETWORK_HASHRATE.get(), 1_000_000);
        assert_eq!(UPTIME_SECONDS.get(), 3600);
    }
}
