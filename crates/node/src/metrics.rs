//! Prometheus metrics for hybrid mining observability.

use bitquan_consensus::pow::PowAlgo;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Global mining metrics collector.
pub struct MiningMetrics {
    /// Blocks mined per algorithm.
    blocks_mined: HashMap<PowAlgo, Arc<AtomicU64>>,
    /// Hash attempts per algorithm.
    hash_attempts: HashMap<PowAlgo, Arc<AtomicU64>>,
    /// Failed PoW verifications per algorithm.
    verify_failures: HashMap<PowAlgo, Arc<AtomicU64>>,
    /// Last block timestamp per algorithm.
    last_block_time: Arc<RwLock<HashMap<PowAlgo, Instant>>>,
    /// Block time durations for calculating averages.
    block_durations: Arc<RwLock<HashMap<PowAlgo, Vec<Duration>>>>,
    /// Total blocks persisted to chain.
    blocks_persisted: Arc<AtomicU64>,
    /// Total rewards distributed (satoshis).
    total_rewards: Arc<AtomicU64>,
    /// Current pool balance (satoshis).
    pool_balance: Arc<AtomicU64>,
    /// Total payouts completed.
    payouts_total: Arc<AtomicU64>,
    /// Current reward per block (satoshis).
    reward_per_block: Arc<AtomicU64>,
    /// Network peers connected.
    network_peers_connected: Arc<AtomicU64>,
    /// Network blocks broadcast.
    network_blocks_broadcast: Arc<AtomicU64>,
    /// Network blocks received.
    network_blocks_received: Arc<AtomicU64>,
    /// Network sync active flag.
    network_sync_active: Arc<AtomicU64>,
}

impl MiningMetrics {
    /// Create new metrics collector.
    pub fn new(algos: &[PowAlgo]) -> Self {
        let mut blocks_mined = HashMap::new();
        let mut hash_attempts = HashMap::new();
        let mut verify_failures = HashMap::new();

        for &algo in algos {
            blocks_mined.insert(algo, Arc::new(AtomicU64::new(0)));
            hash_attempts.insert(algo, Arc::new(AtomicU64::new(0)));
            verify_failures.insert(algo, Arc::new(AtomicU64::new(0)));
        }

        Self {
            blocks_mined,
            hash_attempts,
            verify_failures,
            last_block_time: Arc::new(RwLock::new(HashMap::new())),
            block_durations: Arc::new(RwLock::new(HashMap::new())),
            blocks_persisted: Arc::new(AtomicU64::new(0)),
            total_rewards: Arc::new(AtomicU64::new(0)),
            pool_balance: Arc::new(AtomicU64::new(0)),
            payouts_total: Arc::new(AtomicU64::new(0)),
            reward_per_block: Arc::new(AtomicU64::new(0)),
            network_peers_connected: Arc::new(AtomicU64::new(0)),
            network_blocks_broadcast: Arc::new(AtomicU64::new(0)),
            network_blocks_received: Arc::new(AtomicU64::new(0)),
            network_sync_active: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a successfully mined block.
    pub fn record_block_mined(&self, algo: PowAlgo) {
        if let Some(counter) = self.blocks_mined.get(&algo) {
            counter.fetch_add(1, Ordering::Relaxed);
        }

        let now = Instant::now();
        let mut last_times = self.last_block_time.write().unwrap();
        
        if let Some(last_time) = last_times.get(&algo) {
            let duration = now.duration_since(*last_time);
            let mut durations = self.block_durations.write().unwrap();
            durations.entry(algo).or_insert_with(Vec::new).push(duration);
            
            // Keep only last 100 durations
            if let Some(list) = durations.get_mut(&algo) {
                if list.len() > 100 {
                    list.remove(0);
                }
            }
        }
        
        last_times.insert(algo, now);
    }

    /// Record hash attempts.
    pub fn record_hash_attempts(&self, algo: PowAlgo, count: u64) {
        if let Some(counter) = self.hash_attempts.get(&algo) {
            counter.fetch_add(count, Ordering::Relaxed);
        }
    }

    /// Record PoW verification failure.
    pub fn record_verify_failure(&self, algo: PowAlgo) {
        if let Some(counter) = self.verify_failures.get(&algo) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get total blocks mined for algorithm.
    pub fn get_blocks_mined(&self, algo: PowAlgo) -> u64 {
        self.blocks_mined
            .get(&algo)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get total hash attempts for algorithm.
    pub fn get_hash_attempts(&self, algo: PowAlgo) -> u64 {
        self.hash_attempts
            .get(&algo)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get verify failures for algorithm.
    pub fn get_verify_failures(&self, algo: PowAlgo) -> u64 {
        self.verify_failures
            .get(&algo)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get average block time for algorithm in seconds.
    pub fn get_avg_block_time(&self, algo: PowAlgo) -> Option<f64> {
        let durations = self.block_durations.read().unwrap();
        if let Some(list) = durations.get(&algo) {
            if list.is_empty() {
                return None;
            }
            let sum: Duration = list.iter().sum();
            Some(sum.as_secs_f64() / list.len() as f64)
        } else {
            None
        }
    }

    /// Get estimated hashrate for algorithm (hashes/sec).
    pub fn get_hashrate(&self, algo: PowAlgo) -> f64 {
        let attempts = self.get_hash_attempts(algo);
        if let Some(avg_time) = self.get_avg_block_time(algo) {
            if avg_time > 0.0 {
                return attempts as f64 / avg_time;
            }
        }
        0.0
    }

    /// Record a block persisted to chain.
    pub fn record_block_persisted(&self) {
        self.blocks_persisted.fetch_add(1, Ordering::Relaxed);
    }

    /// Update total rewards distributed.
    pub fn set_total_rewards(&self, amount: u64) {
        self.total_rewards.store(amount, Ordering::Relaxed);
    }

    /// Update pool balance.
    pub fn set_pool_balance(&self, amount: u64) {
        self.pool_balance.store(amount, Ordering::Relaxed);
    }

    /// Record a payout.
    pub fn record_payout(&self) {
        self.payouts_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Set current reward per block.
    pub fn set_reward_per_block(&self, amount: u64) {
        self.reward_per_block.store(amount, Ordering::Relaxed);
    }

    /// Get blocks persisted.
    pub fn get_blocks_persisted(&self) -> u64 {
        self.blocks_persisted.load(Ordering::Relaxed)
    }

    /// Get total rewards.
    pub fn get_total_rewards(&self) -> u64 {
        self.total_rewards.load(Ordering::Relaxed)
    }

    /// Get pool balance.
    pub fn get_pool_balance(&self) -> u64 {
        self.pool_balance.load(Ordering::Relaxed)
    }

    /// Get total payouts.
    pub fn get_payouts_total(&self) -> u64 {
        self.payouts_total.load(Ordering::Relaxed)
    }

    /// Set network peers connected.
    pub fn set_network_peers(&self, count: u64) {
        self.network_peers_connected.store(count, Ordering::Relaxed);
    }

    /// Record network block broadcast.
    pub fn record_network_block_broadcast(&self) {
        self.network_blocks_broadcast.fetch_add(1, Ordering::Relaxed);
    }

    /// Record network block received.
    pub fn record_network_block_received(&self) {
        self.network_blocks_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Set network sync active status.
    pub fn set_network_sync_active(&self, active: bool) {
        self.network_sync_active.store(if active { 1 } else { 0 }, Ordering::Relaxed);
    }

    /// Get network peers count.
    pub fn get_network_peers(&self) -> u64 {
        self.network_peers_connected.load(Ordering::Relaxed)
    }

    /// Get network blocks broadcast.
    pub fn get_network_blocks_broadcast(&self) -> u64 {
        self.network_blocks_broadcast.load(Ordering::Relaxed)
    }

    /// Get network blocks received.
    pub fn get_network_blocks_received(&self) -> u64 {
        self.network_blocks_received.load(Ordering::Relaxed)
    }

    /// Format metrics as Prometheus text format.
    pub fn format_prometheus(&self) -> String {
        let mut output = String::new();
        
        output.push_str("# HELP pow_mined_blocks_total Total blocks mined per algorithm\n");
        output.push_str("# TYPE pow_mined_blocks_total counter\n");
        for (algo, counter) in &self.blocks_mined {
            let value = counter.load(Ordering::Relaxed);
            output.push_str(&format!(
                "pow_mined_blocks_total{{algo=\"{}\"}} {}\n",
                algo.name(),
                value
            ));
        }
        
        output.push_str("# HELP pow_hash_attempts_total Total hash attempts per algorithm\n");
        output.push_str("# TYPE pow_hash_attempts_total counter\n");
        for (algo, counter) in &self.hash_attempts {
            let value = counter.load(Ordering::Relaxed);
            output.push_str(&format!(
                "pow_hash_attempts_total{{algo=\"{}\"}} {}\n",
                algo.name(),
                value
            ));
        }
        
        output.push_str("# HELP pow_verify_failures_total Total PoW verification failures\n");
        output.push_str("# TYPE pow_verify_failures_total counter\n");
        for (algo, counter) in &self.verify_failures {
            let value = counter.load(Ordering::Relaxed);
            output.push_str(&format!(
                "pow_verify_failures_total{{algo=\"{}\"}} {}\n",
                algo.name(),
                value
            ));
        }
        
        output.push_str("# HELP pow_hashrate_gauge Estimated hashrate per algorithm\n");
        output.push_str("# TYPE pow_hashrate_gauge gauge\n");
        for &algo in self.blocks_mined.keys() {
            let hashrate = self.get_hashrate(algo);
            output.push_str(&format!(
                "pow_hashrate_gauge{{algo=\"{}\"}} {:.2}\n",
                algo.name(),
                hashrate
            ));
        }
        
        output.push_str("# HELP pow_block_time_seconds Average block time per algorithm\n");
        output.push_str("# TYPE pow_block_time_seconds gauge\n");
        for &algo in self.blocks_mined.keys() {
            if let Some(avg_time) = self.get_avg_block_time(algo) {
                output.push_str(&format!(
                    "pow_block_time_seconds{{algo=\"{}\"}} {:.2}\n",
                    algo.name(),
                    avg_time
                ));
            }
        }
        
        output.push_str("# HELP stratum_blocks_persisted_total Total blocks persisted to chain\n");
        output.push_str("# TYPE stratum_blocks_persisted_total counter\n");
        output.push_str(&format!("stratum_blocks_persisted_total {}\n", self.get_blocks_persisted()));
        
        output.push_str("# HELP stratum_total_rewards_distributed Total rewards distributed in satoshis\n");
        output.push_str("# TYPE stratum_total_rewards_distributed counter\n");
        output.push_str(&format!("stratum_total_rewards_distributed {}\n", self.get_total_rewards()));
        
        output.push_str("# HELP stratum_pool_balance_gauge Current pool balance in satoshis\n");
        output.push_str("# TYPE stratum_pool_balance_gauge gauge\n");
        output.push_str(&format!("stratum_pool_balance_gauge {}\n", self.get_pool_balance()));
        
        output.push_str("# HELP stratum_payouts_total Total payouts completed\n");
        output.push_str("# TYPE stratum_payouts_total counter\n");
        output.push_str(&format!("stratum_payouts_total {}\n", self.get_payouts_total()));
        
        output.push_str("# HELP reward_per_block_gauge Current reward per block in satoshis\n");
        output.push_str("# TYPE reward_per_block_gauge gauge\n");
        output.push_str(&format!("reward_per_block_gauge {}\n", self.reward_per_block.load(Ordering::Relaxed)));
        
        output.push_str("# HELP network_peers_connected Number of connected peers\n");
        output.push_str("# TYPE network_peers_connected gauge\n");
        output.push_str(&format!("network_peers_connected {}\n", self.get_network_peers()));
        
        output.push_str("# HELP network_blocks_broadcast_total Total blocks broadcast to network\n");
        output.push_str("# TYPE network_blocks_broadcast_total counter\n");
        output.push_str(&format!("network_blocks_broadcast_total {}\n", self.get_network_blocks_broadcast()));
        
        output.push_str("# HELP network_blocks_received_total Total blocks received from network\n");
        output.push_str("# TYPE network_blocks_received_total counter\n");
        output.push_str(&format!("network_blocks_received_total {}\n", self.get_network_blocks_received()));
        
        output.push_str("# HELP network_sync_active_gauge Network sync active status (0=idle, 1=syncing)\n");
        output.push_str("# TYPE network_sync_active_gauge gauge\n");
        output.push_str(&format!("network_sync_active_gauge {}\n", self.network_sync_active.load(Ordering::Relaxed)));
        
        output
    }

    /// Print metrics summary to stdout.
    pub fn print_summary(&self) {
        println!("\n=== Mining Metrics ===");
        for &algo in self.blocks_mined.keys() {
            let blocks = self.get_blocks_mined(algo);
            let attempts = self.get_hash_attempts(algo);
            let failures = self.get_verify_failures(algo);
            let hashrate = self.get_hashrate(algo);
            let avg_time = self.get_avg_block_time(algo);

            println!("\nAlgorithm: {}", algo.name());
            println!("  Blocks mined: {}", blocks);
            println!("  Hash attempts: {}", attempts);
            println!("  Verify failures: {}", failures);
            println!("  Estimated hashrate: {:.2} H/s", hashrate);
            if let Some(time) = avg_time {
                println!("  Avg block time: {:.2}s", time);
            }
        }
        println!("======================\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_basic_operations() {
        let metrics = MiningMetrics::new(&[PowAlgo::Sha256d]);
        
        metrics.record_block_mined(PowAlgo::Sha256d);
        assert_eq!(metrics.get_blocks_mined(PowAlgo::Sha256d), 1);
        
        metrics.record_hash_attempts(PowAlgo::Sha256d, 1000);
        assert_eq!(metrics.get_hash_attempts(PowAlgo::Sha256d), 1000);
        
        metrics.record_verify_failure(PowAlgo::Sha256d);
        assert_eq!(metrics.get_verify_failures(PowAlgo::Sha256d), 1);
    }

    #[test]
    fn prometheus_format() {
        let metrics = MiningMetrics::new(&[PowAlgo::Sha256d]);
        metrics.record_block_mined(PowAlgo::Sha256d);
        
        let output = metrics.format_prometheus();
        assert!(output.contains("pow_mined_blocks_total"));
        assert!(output.contains("sha256d"));
    }
}
