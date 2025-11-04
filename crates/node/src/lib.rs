//! BitQuan node library - exposes modules for integration testing.

pub mod miner;
pub mod metrics;

// Re-export commonly used types
pub use miner::{HybridMiner, MinerMetrics};
pub use metrics::MiningMetrics;
