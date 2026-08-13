
// Declare all modules

// Pool-related modules (Phase 8: Stratum mining pool support)
#[cfg(feature = "pool")]
pub mod block_submit;
#[cfg(feature = "pool")]
pub mod pool_template;
#[cfg(feature = "pool")]
pub mod stratum_server;
#[cfg(feature = "pool")]
pub mod vardiff;

// Core modules (always enabled)
pub mod address;
pub mod chainstate;
pub mod metrics;
pub mod miner;
pub mod mnemonic;
pub mod reward_engine;
pub mod rpc;
pub mod sync_task;
pub mod tx_builder;
pub mod wallet;
pub mod worker;

// Re-export all public types for tests and external usage

// Pool-related re-exports (Phase 8)
#[cfg(feature = "pool")]
pub use block_submit::{BlockSubmitter, SubmitResult};
#[cfg(feature = "pool")]
pub use pool_template::{BlockTemplate, PoolTemplateManager};
#[cfg(feature = "pool")]
pub use stratum_server::*;
#[cfg(feature = "pool")]
pub use vardiff::VarDiff;

// Core re-exports (always available)
pub use chainstate::ChainState;
pub use miner::{HybridMiner, MinerMetrics};
pub use mnemonic::{generate_mnemonic, mnemonic_to_seed, parse_mnemonic, MnemonicHelper};
pub use reward_engine::RewardEngine;
pub use wallet::WalletKeypair;


pub mod repro_exploit;
