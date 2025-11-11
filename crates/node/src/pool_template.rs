//! Block template generation for mining pool operations.
//!
//! Provides fresh templates for miners with automatic refresh mechanism.
//!
//! Reserved for Phase 8 pool integration.

#![allow(dead_code)]

use bitquan_consensus::pow::PowAlgo;
use bitquan_types::{BlockHeader, Result, Transaction};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Block template for miners.
#[derive(Clone, Debug)]
pub struct BlockTemplate {
    /// Block header template with fields pre-filled.
    pub header: BlockHeader,
    /// Transactions to include in the block.
    pub txs: Vec<Transaction>,
    /// Target hash (32 bytes, little-endian).
    pub target: [u8; 32],
    /// Mining algorithm for this template.
    pub algo: PowAlgo,
    /// Job ID for Stratum (increments on each refresh).
    pub job_id: u64,
}

/// Pool template manager with automatic refresh.
pub struct PoolTemplateManager {
    /// Cached template, protected by RwLock.
    cache: Arc<RwLock<Option<BlockTemplate>>>,
    /// Last template refresh timestamp.
    last_refresh: Arc<RwLock<Instant>>,
    /// Refresh interval in seconds.
    refresh_interval: u64,
    /// Next job_id counter.
    next_job_id: Arc<RwLock<u64>>,
}

impl PoolTemplateManager {
    /// Create a new template manager with specified refresh interval.
    ///
    /// # Arguments
    /// * `refresh_interval` - How often to refresh templates (in seconds)
    pub fn new(refresh_interval: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            last_refresh: Arc::new(RwLock::new(Instant::now())),
            refresh_interval,
            next_job_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Get the current cached template.
    pub async fn get_template(&self) -> Option<BlockTemplate> {
        let cache = self.cache.read().await;
        cache.clone()
    }

    /// Manually update the template cache with auto-incremented job_id.
    pub async fn update_template(&self, mut template: BlockTemplate) {
        // Assign next job_id
        let mut job_id = self.next_job_id.write().await;
        template.job_id = *job_id;
        *job_id = job_id.wrapping_add(1);

        let mut cache = self.cache.write().await;
        *cache = Some(template);

        let mut last_refresh = self.last_refresh.write().await;
        *last_refresh = Instant::now();
    }

    /// Start the automatic refresh loop.
    ///
    /// This spawns a background task that periodically generates fresh templates.
    /// The `build_fn` callback is invoked to generate new templates.
    pub fn start_refresh_loop<F>(&self, build_fn: F)
    where
        F: Fn() -> Result<BlockTemplate> + Send + Sync + 'static,
    {
        let cache = Arc::clone(&self.cache);
        let last_refresh = Arc::clone(&self.last_refresh);
        let interval_secs = self.refresh_interval;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;

                match build_fn() {
                    Ok(template) => {
                        let mut cache_guard = cache.write().await;
                        *cache_guard = Some(template);

                        let mut refresh_guard = last_refresh.write().await;
                        *refresh_guard = Instant::now();

                        println!("PoolTemplate: Refreshed block template");
                    }
                    Err(e) => {
                        eprintln!("PoolTemplate: Failed to build template: {}", e);
                    }
                }
            }
        });
    }

    /// Get time since last refresh.
    pub async fn time_since_refresh(&self) -> Duration {
        let last = self.last_refresh.read().await;
        last.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_types::BlockHeader;

    #[tokio::test]
    async fn test_template_cache() {
        let manager = PoolTemplateManager::new(30);

        // Initially empty
        assert!(manager.get_template().await.is_none());

        // Update template
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
        assert_eq!(
            cached.expect("Template should be available").algo,
            PowAlgo::Sha256d
        );
    }
}
