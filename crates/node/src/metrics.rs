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
