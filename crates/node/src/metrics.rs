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
