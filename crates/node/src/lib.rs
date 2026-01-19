use bitquan_types::error::{Error, Result};
use bitquan_types::NetworkId;
use std::sync::Arc;

// Move necessary imports here
use bitquan_consensus::{ConsensusEngine, ConsensusParams};
use bitquan_storage::InMemoryChainStore;
use bq_crypto::CryptoRegistry;
use tokio::time::sleep;

// Declare all modules
pub mod block_submit;
pub mod chainstate;
pub mod metrics;
pub mod miner;
pub mod pool_template;
pub mod reward_engine;
pub mod stratum_server;
pub mod sync_task;
pub mod vardiff;
pub mod wallet;
pub mod worker;

// Re-export all public types for tests and external usage
pub use block_submit::{BlockSubmitter, SubmitResult};
pub use chainstate::ChainState;
pub use miner::{HybridMiner, MinerMetrics};
pub use pool_template::{BlockTemplate, PoolTemplateManager};
pub use reward_engine::RewardEngine;
pub use stratum_server::*;
pub use vardiff::VarDiff;
pub use wallet::WalletKeypair;

pub async fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    let p2p_addr = p2p_bind.unwrap_or("0.0.0.0:18444");
    let _rpc_addr = rpc_bind.unwrap_or("0.0.0.0:18332");

    println!(
        "Starting BitQuan node with configuration: {config_path}\nP2P listening on {p2p_addr}"
    );

    // Bootstraps placeholder subsystems to illustrate crate integration.
    let registry = CryptoRegistry::default();
    let params = ConsensusParams::phase3_defaults();
    let _engine = ConsensusEngine::new(params, registry);
    let _storage = InMemoryChainStore::new();

    start_p2p_server_async(p2p_addr, network).await
}

pub async fn start_p2p_server_async(addr: &str, network: NetworkId) -> Result<()> {
    use bitquan_network::peer_async::AsyncPeerManager;
    use bitquan_network::server_async::spawn_p2p_server_with_limit;

    // Create async peer manager
    let peer_manager = Arc::new(AsyncPeerManager::new(
        100, // max peers
        network,
    ));

    // Spawn P2P server in background
    spawn_p2p_server_with_limit(
        addr,
        peer_manager.clone(),
        100, // max connections
    )
    .await
    .map_err(|e| Error::Net(e.to_string()))?;

    log::info!("Async P2P server running on {}", addr);

    // Keep running (server is in background task)
    loop {
        sleep(std::time::Duration::from_secs(60)).await;

        // Cleanup dead peers every minute
        peer_manager.cleanup_peers().await;

        let peer_count = peer_manager.ready_peer_count().await;
        log::info!("Active peers: {}", peer_count);
    }
}
