//! BitQuan Layer 2 Integration
//!
//! This module provides Layer 2 scaling solutions including rollups and sidechains.

pub mod rollup;
pub mod sidechain;
pub mod bridge;
pub mod batch_processor;
pub mod state_compression;

pub use rollup::{BitQuanRollup, RollupResult, BatchResult};
pub use sidechain::{Sidechain, SidechainConfig};
pub use bridge::{TwoWayPeg, BridgeTransaction};
pub use batch_processor::{BatchProcessor, BatchConfig};
pub use state_compression::{StateCompressor, StateProof};

/// Layer 2 configuration
#[derive(Debug, Clone)]
pub struct Layer2Config {
    /// Enable rollup processing
    pub enable_rollups: bool,
    /// Enable sidechains
    pub enable_sidechains: bool,
    /// Batch size for rollups
    pub batch_size: usize,
    /// Batch timeout
    pub batch_timeout: std::time::Duration,
    /// Max size of compressed state
    pub max_compressed_state_size: usize,
    /// State commitment interval
    pub state_commitment_interval: u64,
    /// Bridge configuration
    pub bridge_config: BridgeConfig,
}

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Minimum confirmations for deposits
    pub min_confirmations: u64,
    /// Maximum time for lock period
    pub lock_period: std::time::Duration,
    /// Fee for bridge transactions
    pub bridge_fee: u64,
    /// Enable automatic settlement
    pub auto_settlement: bool,
}

impl Default for Layer2Config {
    fn default() -> Self {
        Self {
            enable_rollups: true,
            enable_sidechains: true,
            batch_size: 1000,
            batch_timeout: std::time::Duration::from_secs(30),
            max_compressed_state_size: 1024 * 1024, // 1MB
            state_commitment_interval: 100, // Every 100 blocks
            bridge_config: BridgeConfig::default(),
        }
    }
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            min_confirmations: 12,
            lock_period: std::time::Duration::from_secs(3600),
            bridge_fee: 1000,
            auto_settlement: true,
        }
    }
}

/// Layer 2 errors
#[derive(Debug, thiserror::Error)]
pub enum Layer2Error {
    #[error("Rollup error: {0}")]
    RollupError(String),
    #[error("Sidechain error: {0}")]
    SidechainError(String),
    #[error("Bridge error: {0}")]
    BridgeError(String),
    #[error("Batch processing error: {0}")]
    BatchError(String),
    #[error("State compression error: {0}")]
    CompressionError(String),
    #[error("Invalid proof")]
    InvalidProof,
    #[error("Exceeds maximum size")]
    ExceedsMaxSize,
    #[error("Timeout occurred")]
    Timeout,
}

/// Result type for Layer 2 operations
pub type Layer2Result<T> = Result<T, Layer2Error>;

/// Layer 2 statistics
#[derive(Debug, Clone)]
pub struct Layer2Stats {
    pub rollup_batches_processed: u64,
    pub sidechain_transactions: u64,
    pub bridge_transactions: u64,
    pub compressed_state_size: usize,
    pub avg_processing_time: std::time::Duration,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer2_config() {
        let config = Layer2Config::default();
        assert!(config.enable_rollups);
        assert!(config.enable_sidechains);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.batch_timeout.as_secs(), 30);
    }

    #[test]
    fn test_bridge_config() {
        let config = BridgeConfig::default();
        assert_eq!(config.min_confirmations, 12);
        assert_eq!(config.lock_period.as_secs(), 3600);
        assert_eq!(config.bridge_fee, 1000);
        assert!(config.auto_settlement);
    }
}