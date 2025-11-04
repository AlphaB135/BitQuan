//! BitQuan node library - exposes modules for integration testing.

pub mod block_submit;
pub mod chainstate;
pub mod miner;
pub mod metrics;
pub mod pool_db;
pub mod pool_template;
pub mod reward_engine;
pub mod stratum_server;
pub mod vardiff;
pub mod ws_dashboard;

// Re-export commonly used types
pub use block_submit::{BlockSubmitter, SubmitResult};
pub use chainstate::ChainState;
pub use miner::{HybridMiner, MinerMetrics};
pub use metrics::MiningMetrics;
pub use pool_db::{BlockRecord, PayoutRecord, PoolDatabase};
pub use pool_template::{BlockTemplate, PoolTemplateManager};
pub use reward_engine::{RewardEngine, PoolStats};
pub use stratum_server::{StratumServer, StratumConfig, StratumMetrics, MinerSession};
pub use vardiff::VarDiff;
pub use ws_dashboard::{WsDashboard, DashboardConfig, PoolStats as WsPoolStats, MinerInfo};
