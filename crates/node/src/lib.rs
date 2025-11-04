//! BitQuan node library - exposes modules for integration testing.

pub mod miner;
pub mod metrics;
pub mod pool_template;
pub mod stratum_server;
pub mod vardiff;
pub mod ws_dashboard;

// Re-export commonly used types
pub use miner::{HybridMiner, MinerMetrics};
pub use metrics::MiningMetrics;
pub use pool_template::{BlockTemplate, PoolTemplateManager};
pub use stratum_server::{StratumServer, StratumConfig, StratumMetrics, MinerSession};
pub use vardiff::VarDiff;
pub use ws_dashboard::{WsDashboard, DashboardConfig, PoolStats, MinerInfo};
