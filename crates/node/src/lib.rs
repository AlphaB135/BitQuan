use bitquan_types::error::{Error, Result};
use bitquan_types::NetworkId;
use std::sync::Arc;

use bitquan_consensus::{ConsensusEngine, ConsensusParams};
use bitquan_mempool::Mempool;
use bitquan_network::peer_async::AsyncPeerManager;
use bitquan_network::server_async::spawn_p2p_server_with_limit;
use bitquan_storage::InMemoryChainStore;
use bq_crypto::CryptoRegistry;
use tokio::sync::Mutex;
use tokio::time::sleep;

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

pub async fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    let p2p_addr = p2p_bind.unwrap_or("0.0.0.0:18444");
    let rpc_addr = rpc_bind.unwrap_or("127.0.0.1:18332");

    log::info!("Starting BitQuan node | config={config_path} | p2p={p2p_addr} | rpc={rpc_addr}");

    // 1. Crypto registry (Dilithium5 provider)
    let registry = CryptoRegistry::default();

    // 2. Consensus engine
    let params = match network {
        NetworkId::Mainnet => ConsensusParams::phase3_defaults(),
        NetworkId::Testnet => ConsensusParams::testnet_hybrid(),
        _ => ConsensusParams::devnet_hybrid(),
    };
    let consensus = Arc::new(Mutex::new(ConsensusEngine::new(params, registry)));

    // 3. Storage (in-memory for now; replace with RocksDB for production)
    let store = Arc::new(Mutex::new(InMemoryChainStore::new()));

    // 4. Mempool
    let mempool = Arc::new(Mutex::new(
        Mempool::new().map_err(|e| Error::Internal(e.to_string()))?,
    ));

    // 5. P2P server (background task)
    let peer_manager = Arc::new(AsyncPeerManager::new(100, network));
    spawn_p2p_server_with_limit(p2p_addr, peer_manager.clone(), 100)
        .await
        .map_err(|e| Error::Net(e.to_string()))?;
    log::info!("P2P server running on {p2p_addr}");

    // 6. Subsystems wired
    log::info!("Node subsystems wired. Entering main loop.");
    // NOTE: consensus and mempool are created but not yet wired to the P2P
    // message handler. SyncTask integration is tracked in issue #143.
    // For now, the node connects to peers and maintains the heartbeat.
    // Block processing will be added in the next iteration.
    let _consensus = consensus;
    let _mempool = mempool;

    // 7. Main loop — heartbeat and peer maintenance
    loop {
        sleep(std::time::Duration::from_secs(30)).await;
        peer_manager.cleanup_peers().await;
        let peers = peer_manager.ready_peer_count().await;
        let height = store.lock().await.height();
        log::info!("height={height} peers={peers}");
    }
}

pub async fn start_p2p_server_async(addr: &str, network: NetworkId) -> Result<()> {
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
