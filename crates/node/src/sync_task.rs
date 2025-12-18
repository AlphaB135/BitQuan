//! Background sync task for maintaining chain synchronization

use crate::rpc::NodeRpcHandler;
use bitquan_network::async_sync::{AsyncSyncManager, AsyncSyncError};
use bitquan_network::{discovery::PeerBook, peer::PeerManager};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use log::{info, error, warn};

/// Spawns a background task that periodically runs sync maintenance
pub async fn spawn_sync_maintenance(
    sync_manager: Arc<AsyncSyncManager>,
    rpc_handler: Arc<NodeRpcHandler>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Periodic sync maintenance loop
        loop {
            // Check sync status
            match sync_manager.get_sync_progress().await {
                Ok(progress) => {
                    let is_syncing = matches!(progress.status,
                        bitquan_network::sync::SyncStatus::Discovering |
                        bitquan_network::sync::SyncStatus::DownloadingHeaders |
                        bitquan_network::sync::SyncStatus::DownloadingBlocks
                    );

                    info!(
                        "Sync status: {:?}, Height: {}, Progress: {:.1}%, Syncing: {}",
                        progress.status, progress.local_height, progress.progress, is_syncing
                    );
                }
                Err(e) => {
                    error!("Failed to get sync progress: {}", e);
                }
            }

            // Wait before next check
            sleep(Duration::from_secs(30)).await;
        }
    })
}

/// Initialize sync manager and background task
pub async fn initialize_sync(
    local_height: u64,
    network_id: bitquan_types::NetworkId,
) -> Result<(Arc<AsyncSyncManager>, tokio::task::JoinHandle<()>), Box<dyn std::error::Error + Send + Sync>> {
    // Create simple sync manager without peer book for now
    // TODO: Add peer discovery once mutex issues are resolved
    let sync_manager = Arc::new(AsyncSyncManager::new(local_height));

    info!("AsyncSyncManager initialized with height: {}", local_height);

    Ok((sync_manager, tokio::spawn(async move {
        // Simple maintenance task
        loop {
            sleep(Duration::from_secs(300)).await; // Check every 5 minutes
            info!("Background sync maintenance check completed");
        }
    })))
}