use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// System-wide metrics collector
#[derive(Clone)]
pub struct Metrics {
    pub blocks_total: Arc<AtomicU64>,
    pub transactions_total: Arc<AtomicU64>,
    pub mempool_size: Arc<AtomicU64>,
    pub peers_connected: Arc<AtomicU64>,
    pub sync_height: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            blocks_total: Arc::new(AtomicU64::new(0)),
            transactions_total: Arc::new(AtomicU64::new(0)),
            mempool_size: Arc::new(AtomicU64::new(0)),
            peers_connected: Arc::new(AtomicU64::new(0)),
            sync_height: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment_blocks(&self) {
        self.blocks_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_transactions(&self, count: u64) {
        self.transactions_total.fetch_add(count, Ordering::Relaxed);
    }

    pub fn set_mempool_size(&self, size: u64) {
        self.mempool_size.store(size, Ordering::Relaxed);
    }

    pub fn set_peers_connected(&self, count: u64) {
        self.peers_connected.store(count, Ordering::Relaxed);
    }

    pub fn set_sync_height(&self, height: u64) {
        self.sync_height.store(height, Ordering::Relaxed);
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self, network: &str) -> String {
        format!(
            r#"# HELP bitquan_blocks_total Total number of blocks processed
# TYPE bitquan_blocks_total counter
bitquan_blocks_total{{network="{}"}} {}

# HELP bitquan_transactions_total Total number of transactions processed
# TYPE bitquan_transactions_total counter
bitquan_transactions_total{{network="{}"}} {}

# HELP bitquan_mempool_size Current number of transactions in mempool
# TYPE bitquan_mempool_size gauge
bitquan_mempool_size{{network="{}"}} {}

# HELP bitquan_peers_connected Current number of connected peers
# TYPE bitquan_peers_connected gauge
bitquan_peers_connected{{network="{}"}} {}

# HELP bitquan_sync_height Current blockchain height
# TYPE bitquan_sync_height gauge
bitquan_sync_height{{network="{}"}} {}
"#,
            network,
            self.blocks_total.load(Ordering::Relaxed),
            network,
            self.transactions_total.load(Ordering::Relaxed),
            network,
            self.mempool_size.load(Ordering::Relaxed),
            network,
            self.peers_connected.load(Ordering::Relaxed),
            network,
            self.sync_height.load(Ordering::Relaxed),
        )
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_increment() {
        let metrics = Metrics::new();

        metrics.increment_blocks();
        assert_eq!(metrics.blocks_total.load(Ordering::Relaxed), 1);

        metrics.increment_transactions(5);
        assert_eq!(metrics.transactions_total.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_metrics_set() {
        let metrics = Metrics::new();

        metrics.set_mempool_size(100);
        assert_eq!(metrics.mempool_size.load(Ordering::Relaxed), 100);

        metrics.set_peers_connected(42);
        assert_eq!(metrics.peers_connected.load(Ordering::Relaxed), 42);

        metrics.set_sync_height(1000);
        assert_eq!(metrics.sync_height.load(Ordering::Relaxed), 1000);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Metrics::new();
        metrics.increment_blocks();
        metrics.increment_transactions(10);
        metrics.set_mempool_size(5);
        metrics.set_peers_connected(3);
        metrics.set_sync_height(100);

        let output = metrics.export_prometheus("mainnet");

        assert!(output.contains("bitquan_blocks_total{network=\"mainnet\"} 1"));
        assert!(output.contains("bitquan_transactions_total{network=\"mainnet\"} 10"));
        assert!(output.contains("bitquan_mempool_size{network=\"mainnet\"} 5"));
        assert!(output.contains("bitquan_peers_connected{network=\"mainnet\"} 3"));
        assert!(output.contains("bitquan_sync_height{network=\"mainnet\"} 100"));
    }
}
