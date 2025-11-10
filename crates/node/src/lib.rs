//! BitQuan node library - exposes modules for integration testing.

#![allow(dead_code)] // Many builder/infra APIs are not yet used but will be in production

pub mod block_submit;
pub mod chainstate;
pub mod metrics;
pub mod miner;
pub mod monitoring;
pub mod pool_db;
pub mod pool_dashboard;
pub mod pool_template;
pub mod reward_engine;
pub mod stratum_server;
pub mod vardiff;
pub mod ws_dashboard;

// Re-export commonly used types
pub use block_submit::{BlockSubmitter, SubmitResult};
pub use chainstate::ChainState;
pub use metrics::MiningMetrics;
pub use miner::{HybridMiner, MinerMetrics};
pub use monitoring::{MonitoringSystem, HealthStatus, PerformanceMetrics};
pub use pool_db::{BlockRecord, PayoutRecord, PoolDatabase};
pub use pool_dashboard::{PoolDashboard};
pub use pool_template::{BlockTemplate, PoolTemplateManager};
pub use reward_engine::{PoolStats, RewardEngine};
pub use stratum_server::{MinerSession, StratumConfig, StratumMetrics, StratumServer};
pub use vardiff::VarDiff;
pub use ws_dashboard::{DashboardConfig, MinerInfo, PoolStats as WsPoolStats, WsDashboard};
