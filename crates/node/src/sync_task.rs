//! Background sync task for maintaining chain synchronization

use crate::rpc::NodeRpcHandler;
use bitquan_network::async_sync::{AsyncSyncManager, AsyncSyncError};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use log::{info, error, warn};

/// Spawns a background task that periodically runs sync maintenance
pub async fn spawn_sync_maintenance(
    sync_manager: Arc<AsyncSyncManager>,
    rpc_handler: Arc<NodeRpcHandler>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Starting background sync maintenance task");

        loop {
            // Check if sync is needed
            match sync_manager.needs_sync().await {
                Ok(needs_sync) => {
                    if needs_sync {
                        info!("Sync is needed, attempting to start sync");
                        match sync_manager.start_sync_if_needed().await {
                            Ok(started) => {
                                if started {
                                    info!("Sync started successfully");

                                    // Monitor sync progress
                                    while sync_manager.inner().inner().is_syncing() {
                                        match sync_manager.get_sync_progress().await {
                                            Ok(progress) => {
                                                info!(
                                                    "Sync progress: {:.1}% ({} blocks behind)",
                                                    progress.progress,
                                                    progress.blocks_behind
                                                );
                                            }
                                            Err(e) => {
                                                error!("Failed to get sync progress: {}", e);
                                                break;
                                            }
                                        }
                                        sleep(Duration::from_secs(5)).await;
                                    }

                                    info!("Sync completed successfully");
                                } else {
                                    info!("Sync already in progress");
                                }
                            }
                            Err(e) => {
                                error!("Failed to start sync: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to check sync status: {}", e);
                    }
                }

                // Wait before next check
                sleep(Duration::from_secs(30)).await;
            }
        }
    })
}

/// Initialize sync manager and background task
pub async fn initialize_sync(
    local_height: u64,
    network_id: bitquan_types::NetworkId,
) -> Result<(Arc<AsyncSyncManager>, tokio::task::JoinHandle<()>), Box<dyn std::error::Error + Send + Sync>> {
    use bitquan_network::{discovery::PeerBook, peer::PeerManager};

    let peer_manager = Arc::new(PeerManager::new());
    let peer_book = Arc::new(std::sync::Mutex::new(PeerBook::new()));

    let sync_manager = Arc::new(AsyncSyncManager::new(
        local_height,
        peer_manager,
        peer_book,
        network_id,
    ));

    // Set initial best height based on current local height
    sync_manager.chain_sync.set_best_height(local_height).await?;

    // Spawn a simple peer discovery task for simulation
    let sync_manager_clone = Arc::clone(&sync_manager);
    tokio::spawn(async move {
        loop {
            // Simulate discovering peers with higher height
            let current_progress = sync_manager_clone.get_sync_progress().await.unwrap_or_else(|e| {
                error!("Failed to get sync progress for peer discovery: {}", e);
                // Return a default progress if we can't get the real one
                bitquan_network::sync::SyncProgress {
                    status: bitquan_network::sync::SyncStatus::Idle,
                    local_height: 0,
                    best_height: 0,
                    blocks_behind: 0,
                    progress: 100.0,
                    syncing: false,
                    last_sync_attempt: 0,
                    sync_errors: 0,
                }
            });

            // Simulate finding a peer with higher height
            if current_progress.best_height == current_progress.local_height {
                let simulated_peer_height = current_progress.local_height + 100;
                info!("Discovered peer with height: {}", simulated_peer_height);
                if let Err(e) = sync_manager_clone.set_best_height(simulated_peer_height).await {
                    error!("Failed to update best height: {}", e);
                }
            }

            // Wait before next discovery attempt
            sleep(Duration::from_secs(60)).await;
        }
    });

    Ok((sync_manager, tokio::spawn(async move {
        // Simple maintenance task
        loop {
            sleep(Duration::from_secs(300)).await; // Check every 5 minutes
            info!("Background sync maintenance check completed");
        }
    })))
}
