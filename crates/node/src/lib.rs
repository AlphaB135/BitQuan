//! BitQuan node library - exposes modules for integration testing.

pub mod miner;
pub mod metrics;
pub mod stratum_server;

// Re-export commonly used types
pub use miner::{HybridMiner, MinerMetrics};
pub use metrics::MiningMetrics;
pub use stratum_server::{StratumServer, StratumConfig, StratumMetrics, MinerSession};
