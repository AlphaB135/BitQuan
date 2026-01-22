//! Hybrid Mining Controller with weighted algorithm switching and metrics.

use bitquan_consensus::pow::{PowAlgo, PowEngine, RandomXEngine, Sha256dEngine};
use bitquan_types::{BlockHeader, NetworkId, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Hybrid miner capable of mining multiple PoW algorithms concurrently.
pub struct HybridMiner {
    /// Available PoW engines.
    engines: Vec<Arc<dyn PowEngine + Send + Sync>>,
    /// Weight per algorithm (higher = more mining time allocated).
    weights: HashMap<PowAlgo, f32>,
    /// Number of mining threads.
    threads: usize,
    /// Stop signal for graceful shutdown.
    stop_flag: Arc<AtomicBool>,
    /// Metrics counters.
    metrics: MinerMetrics,
}

/// Mining metrics for observability.
#[derive(Clone)]
pub struct MinerMetrics {
    /// Total blocks mined per algorithm.
    pub blocks_mined: HashMap<PowAlgo, Arc<AtomicU64>>,
    /// Total hash attempts per algorithm.
    pub hash_attempts: HashMap<PowAlgo, Arc<AtomicU64>>,
    /// Failed PoW verifications per algorithm.
    pub verify_failures: HashMap<PowAlgo, Arc<AtomicU64>>,
}

impl MinerMetrics {
    /// Create new metrics for given algorithms.
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
        }
    }

    /// Increment blocks mined counter for algorithm.
    pub fn record_block(&self, algo: PowAlgo) {
        if let Some(counter) = self.blocks_mined.get(&algo) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment hash attempt counter for algorithm.
    pub fn record_hash_attempt(&self, algo: PowAlgo) {
        if let Some(counter) = self.hash_attempts.get(&algo) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment verify failure counter for algorithm.
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
}

#[cfg(feature = "pool")]
impl MinerMetrics {
    /// Set total rewards (Phase 8: pool integration).
    pub fn set_total_rewards(&self, _rewards: u64) {
        // TODO: Implement persistent metrics storage
    }

    /// Set pool balance (Phase 8: pool integration).
    pub fn set_pool_balance(&self, _balance: u64) {
        // TODO: Implement persistent metrics storage
    }

    /// Set reward per block (Phase 8: pool integration).
    pub fn set_reward_per_block(&self, _reward: u64) {
        // TODO: Implement persistent metrics storage
    }
}

impl HybridMiner {
    /// Create a new hybrid miner with specified algorithm weights.
    ///
    /// # Arguments
    /// * `weights` - Algorithm weights (higher = more mining time)
    /// * `threads` - Number of mining threads (0 = CPU count)
    /// * `network` - Network ID for validation
    pub fn new(weights: &[(PowAlgo, f32)], threads: usize, network: NetworkId) -> Result<Self> {
        if weights.is_empty() {
            return Err(bitquan_types::Error::Invalid(
                "at least one algorithm required".to_string(),
            ));
        }

        // Validate mainnet restriction
        if network == NetworkId::Mainnet {
            for (_algo, _) in weights {
                #[cfg(feature = "randomx")]
                if *_algo == PowAlgo::RandomX {
                    return Err(bitquan_types::Error::Invalid(
                        "RandomX is not allowed on mainnet".to_string(),
                    ));
                }
            }
        }

        let mut engines: Vec<Arc<dyn PowEngine + Send + Sync>> = Vec::new();
        let mut weight_map = HashMap::new();
        let mut algos = Vec::new();

        for (algo, weight) in weights {
            if *weight <= 0.0 {
                return Err(bitquan_types::Error::Invalid(format!(
                    "weight must be positive for {:?}",
                    algo
                )));
            }

            weight_map.insert(*algo, *weight);
            algos.push(*algo);

            match algo {
                PowAlgo::Sha256d => {
                    engines.push(Arc::new(Sha256dEngine));
                }
                PowAlgo::RandomX => {
                    use bitquan_consensus::pow::{RandomXConfig, RandomXMode};
                    let config = RandomXConfig {
                        mode: RandomXMode::Fast,
                        seed: [0u8; 32], // Will be updated with genesis hash
                    };
                    engines.push(Arc::new(RandomXEngine::new(config)));
                }
                PowAlgo::Ethash => {
                    use bitquan_consensus::pow::{EthashConfig, EthashEngine};
                    let config = EthashConfig::default();
                    engines.push(Arc::new(EthashEngine::new(config)));
                }
            }
        }

        let thread_count = if threads == 0 {
            num_cpus::get()
        } else {
            threads
        };

        Ok(Self {
            engines,
            weights: weight_map,
            threads: thread_count,
            stop_flag: Arc::new(AtomicBool::new(false)),
            metrics: MinerMetrics::new(&algos),
        })
    }

    /// Get reference to metrics.
    pub fn metrics(&self) -> &MinerMetrics {
        &self.metrics
    }

    /// Signal miner to stop gracefully.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Check if miner should stop.
    pub fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }

    /// Select next algorithm based on weighted round-robin.
    pub fn select_algorithm(&self, iteration: u64) -> PowAlgo {
        // Simple weighted selection: accumulate weights and select based on iteration
        let total_weight: f32 = self.weights.values().sum();
        let mut cumulative = 0.0;
        let target = (iteration as f32 % total_weight) + 0.001; // Small epsilon

        for (&algo, &weight) in &self.weights {
            cumulative += weight;
            if target <= cumulative {
                return algo;
            }
        }

        // Fallback to first algorithm
        // SAFETY: weights is guaranteed non-empty due to validation in new()
        // This branch only triggers due to floating-point precision edge cases
        #[allow(clippy::unwrap_used)]
        self.weights.keys().next().copied().unwrap_or_else(|| {
            // Should NEVER happen due to validation in new()
            // If it does (e.g., refactoring bug), use safe fallback
            log::error!("CRITICAL: No mining algorithms available! Using Sha256d fallback.");
            PowAlgo::Sha256d
        })
    }

    /// Get engine for given algorithm.
    pub fn get_engine(&self, algo: PowAlgo) -> Option<Arc<dyn PowEngine + Send + Sync>> {
        for engine in &self.engines {
            if engine.algo() == algo {
                return Some(Arc::clone(engine));
            }
        }
        None
    }

    /// Mine a single block attempt with given header template.
    pub fn mine_block_attempt(
        &self,
        mut header: BlockHeader,
        max_nonce: u64,
        algo: PowAlgo,
    ) -> Result<Option<BlockHeader>> {
        let engine = self
            .get_engine(algo)
            .ok_or_else(|| bitquan_types::Error::Invalid(format!("no engine for {:?}", algo)))?;

        // Set algorithm ID in header
        header.algo_id = algo.to_u8();

        for nonce in 0..max_nonce {
            if self.should_stop() {
                return Ok(None);
            }

            header.nonce = nonce;
            self.metrics.record_hash_attempt(algo);

            match engine.verify(&header) {
                Ok(()) => {
                    self.metrics.record_block(algo);
                    return Ok(Some(header));
                }
                Err(_) => {
                    // Continue mining
                }
            }

            // Yield every 10000 attempts
            if nonce % 10000 == 0 {
                std::thread::yield_now();
            }
        }

        Ok(None)
    }

    /// Get thread count.
    pub fn thread_count(&self) -> usize {
        self.threads
    }

    /// Get algorithm weights.
    pub fn weights(&self) -> &HashMap<PowAlgo, f32> {
        &self.weights
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_miner_creation_sha256d() {
        let weights = vec![(PowAlgo::Sha256d, 1.0)];
        let miner = HybridMiner::new(&weights, 1, NetworkId::Devnet)
            .unwrap_or_else(|e| panic!("Failed to create hybrid miner: {}", e));
        assert_eq!(miner.thread_count(), 1);
        assert_eq!(miner.weights().len(), 1);
    }

    #[test]
    fn mainnet_rejects_randomx() {
        #[cfg(feature = "randomx")]
        {
            let weights = vec![(PowAlgo::RandomX, 1.0)];
            let result = HybridMiner::new(&weights, 1, NetworkId::Mainnet);
            assert!(result.is_err());
            if let Err(e) = result {
                assert!(e.to_string().contains("not allowed on mainnet"));
            }
        }
    }

    #[test]
    fn weighted_selection() {
        let weights = vec![(PowAlgo::Sha256d, 1.0)];
        let miner = HybridMiner::new(&weights, 1, NetworkId::Devnet)
            .unwrap_or_else(|e| panic!("Failed to create hybrid miner: {}", e));

        // Should always select SHA256d with only one option
        for i in 0..10 {
            assert_eq!(miner.select_algorithm(i), PowAlgo::Sha256d);
        }
    }

    #[cfg(feature = "randomx")]
    #[test]
    fn hybrid_weighted_selection() {
        let weights = vec![(PowAlgo::Sha256d, 1.0), (PowAlgo::RandomX, 2.0)];
        let miner = HybridMiner::new(&weights, 1, NetworkId::Devnet)
            .expect("Failed to create hybrid miner");

        // Collect selections to verify distribution
        let mut sha256d_count = 0;
        let mut randomx_count = 0;
        let mut _ethash_count = 0;

        for i in 0..100 {
            match miner.select_algorithm(i) {
                PowAlgo::Sha256d => sha256d_count += 1,
                #[cfg(feature = "randomx")]
                PowAlgo::RandomX => randomx_count += 1,
                PowAlgo::Ethash => _ethash_count += 1,
            }
        }

        // RandomX should be selected more often (roughly 2:1 ratio)
        assert!(randomx_count > sha256d_count);
    }
}
