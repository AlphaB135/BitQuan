//! RPC handler implementation for the BitQuan node.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use bitquan_consensus::header_hash;
use bitquan_rpc::{
    methods::{
        BlockTemplate, BlockchainInfo, MinerStatsResponse, MiningInfo, NetworkStatusResponse,
        PayoutRequest, PayoutResponse, PoolStatsResponse, RpcMethods, SyncResponse, TxInfo,
        WorkData,
    },
    RpcError,
};
use bitquan_storage::{
    async_store::AsyncChainStore, rocksdb_store::RocksDBStore, StorageError,
};
use bitquan_network::async_sync::AsyncSyncManager;
use bitquan_types::{Transaction, NetworkId, GENESIS_BITS};
use hex::FromHex;

/// Node RPC handler backed by an async chain store.
pub struct NodeRpcHandler {
    store: Arc<dyn AsyncChainStore>,
    chain_name: String,
    sync_manager: Option<Arc<AsyncSyncManager>>,
}

impl NodeRpcHandler {
    /// Create a new RPC handler using the given async store.
    pub fn new(store: Arc<dyn AsyncChainStore>, chain_name: impl Into<String>) -> Self {
        Self {
            store,
            chain_name: chain_name.into(),
            sync_manager: None,
        }
    }

    /// Create a new RPC handler with sync manager.
    pub fn with_sync_manager(
        store: Arc<dyn AsyncChainStore>,
        chain_name: impl Into<String>,
        sync_manager: Arc<AsyncSyncManager>,
    ) -> Self {
        Self {
            store,
            chain_name: chain_name.into(),
            sync_manager: Some(sync_manager),
        }
    }

    /// Helper to convert storage errors to RPC errors safely
    fn storage_error_to_rpc(e: bitquan_storage::async_store::AsyncStoreError) -> RpcError {
        match e {
            bitquan_storage::async_store::AsyncStoreError::Storage(se) => {
                RpcError::InternalError(format!("storage error: {}", se))
            }
            bitquan_storage::async_store::AsyncStoreError::TaskSpawn(te) => {
                RpcError::InternalError(format!("task spawn error: {}", te))
            }
            bitquan_storage::async_store::AsyncStoreError::Poisoned(s) => {
                RpcError::InternalError(format!("mutex poisoned during {}", s))
            }
            bitquan_storage::async_store::AsyncStoreError::Cancelled => {
                RpcError::InternalError("operation cancelled".to_string())
            }
        }
    }
}

#[async_trait]
impl RpcMethods for NodeRpcHandler {
    async fn getblockcount(&self) -> Result<u64, RpcError> {
        self.store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)
    }

    async fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError> {
        let height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;

        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;

        let tip_hash = tip.as_ref().map(|header| hex::encode(header_hash(&header)));

        let difficulty = tip
            .as_ref()
            .map(|header| difficulty_from_bits(header.bits))
            .unwrap_or(1.0);

        Ok(BlockchainInfo {
            chain: self.chain_name.clone(),
            blocks: height,
            bestblockhash: tip_hash.unwrap_or_else(|| String::from("0")),
            difficulty,
            chainwork: String::from("0"),
        })
    }

    async fn getmininginfo(&self) -> Result<MiningInfo, RpcError> {
        let blocks = self.getblockcount().await?;
        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;
        let difficulty = tip
            .map(|header| difficulty_from_bits(header.bits))
            .unwrap_or(1.0);

        Ok(MiningInfo {
            blocks,
            difficulty,
            networkhashps: 0.0,
        })
    }

    async fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError> {
        Err(RpcError::InternalError(
            "getblocktemplate not implemented".into(),
        ))
    }

    async fn getwork(&self) -> Result<WorkData, RpcError> {
        Err(RpcError::InternalError("getwork not implemented".into()))
    }

    async fn submitblock(&self, _block_hex: String) -> Result<bool, RpcError> {
        Err(RpcError::InternalError(
            "submitblock not implemented".into(),
        ))
    }

    async fn submitwork(&self, _data: String) -> Result<bool, RpcError> {
        Err(RpcError::InternalError("submitwork not implemented".into()))
    }

    async fn gettransaction(&self, txid: String) -> Result<TxInfo, RpcError> {
        let bytes = Vec::from_hex(&txid)
            .map_err(|_| RpcError::InvalidParams("txid must be hex-encoded".into()))?;
        if bytes.len() != 32 {
            return Err(RpcError::InvalidParams(
                "txid must be 32 bytes (64 hex chars)".into(),
            ));
        }
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes);

        let tx = self
            .store
            .get_transaction(&id)
            .await
            .map_err(Self::storage_error_to_rpc)?
            .ok_or_else(|| RpcError::InternalError("transaction not found".into()))?;

        let summary = TransactionSummary::from(tx);
        Ok(summary.into())
    }

    async fn submittransaction(&self, tx_hex: String) -> Result<String, RpcError> {
        // Decode transaction from hex
        let tx_bytes = Vec::from_hex(&tx_hex)
            .map_err(|_| RpcError::InvalidParams("transaction must be hex-encoded".into()))?;

        // Parse transaction (for now, assume it's JSON format)
        let tx: Transaction = serde_json::from_slice(&tx_bytes)
            .map_err(|e| RpcError::InvalidParams(format!("failed to parse transaction: {}", e)))?;

        // Calculate transaction ID
        let txid = hex::encode(tx.txid());

        // For now, just validate and return the txid
        // In a full implementation, this would:
        // 1. Validate transaction syntax
        // 2. Verify signatures
        // 3. Check inputs/outputs
        // 4. Add to mempool
        // 5. Broadcast to peers

        Ok(txid)
    }

    async fn getbestblockhash(&self) -> Result<String, RpcError> {
        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;

        Ok(tip
            .map(|header| hex::encode(header_hash(&header)))
            .unwrap_or_else(|| String::from("0")))
    }

    async fn getblockhash(&self, height: u64) -> Result<String, RpcError> {
        let block = self
            .store
            .get_block_by_height(height)
            .await
            .map_err(Self::storage_error_to_rpc)?
            .ok_or_else(|| RpcError::InternalError("block not found".into()))?;

        Ok(hex::encode(header_hash(&block.header)))
    }

    async fn getpoolstats(&self) -> Result<PoolStatsResponse, RpcError> {
        // Pool stats require reward engine integration
        // For now, return placeholder values from chain state
        let height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        Ok(PoolStatsResponse {
            height,
            total_rewards: 0,
            miner_count: 0,
            pool_balance: 0,
            block_count: height,
        })
    }

    async fn getminerstats(&self, miner_id: String) -> Result<MinerStatsResponse, RpcError> {
        // Miner stats require reward engine integration
        // For now, return placeholder
        Ok(MinerStatsResponse {
            miner_id,
            total_reward: 0,
            blocks_mined: 0,
            recent_blocks: vec![],
        })
    }

    async fn createpayout(&self, _request: PayoutRequest) -> Result<PayoutResponse, RpcError> {
        // Payout creation requires reward engine integration
        // For now, return mock success
        Ok(PayoutResponse {
            payout_id: format!("payout_{}", uuid::Uuid::new_v4()),
            txid: None,
        })
    }

    async fn getnetworkstatus(&self) -> Result<NetworkStatusResponse, RpcError> {
        // Network status requires network manager integration
        // For now, return placeholder values
        let local_height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        Ok(NetworkStatusResponse {
            peers_connected: 0,
            blocks_broadcast: 0,
            blocks_received: 0,
            sync_status: "idle".to_string(),
            local_height,
            best_height: local_height,
        })
    }

    async fn sync(&self) -> Result<SyncResponse, RpcError> {
        // Get current chain height safely
        let local_height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;

        // Check if sync manager is available
        if let Some(sync_manager) = &self.sync_manager {
            // Get real sync status from sync manager
            let progress = sync_manager
                .get_sync_progress()
                .await
                .map_err(|e| RpcError::InternalError(format!("sync manager error: {}", e)))?;

            // Determine if syncing is active based on status
            let is_syncing = matches!(progress.status,
            bitquan_network::sync::SyncStatus::Discovering |
            bitquan_network::sync::SyncStatus::DownloadingHeaders |
            bitquan_network::sync::SyncStatus::DownloadingBlocks
        );

            Ok(SyncResponse {
                status: format!("{:?}", progress.status),
                local_height: progress.local_height,
                best_height: progress.best_height,
                blocks_behind: progress.blocks_behind,
                progress: progress.progress,
                syncing: is_syncing,
                last_sync_attempt: progress.last_sync_attempt,
                sync_errors: progress.sync_errors,
            })
        } else {
            // Fallback when sync manager is not initialized
            Ok(SyncResponse {
                status: "sync_manager_unavailable".to_string(),
                local_height,
                best_height: local_height,
                blocks_behind: 0,
                progress: 100.0,
                syncing: false,
                last_sync_attempt: 0,
                sync_errors: 0,
            })
        }
    }
}

fn storage_to_rpc(err: StorageError) -> RpcError {
    RpcError::InternalError(err.to_string())
}

fn difficulty_from_bits(bits: u32) -> f64 {
    let max_target = bitquan_consensus::compact_to_target(GENESIS_BITS);
    let target = bitquan_consensus::compact_to_target(bits);
    if target == 0 {
        return 0.0;
    }
    (max_target / target) as f64
}

struct TransactionSummary {
    txid: String,
    version: i32,
    lock_time: u32,
    vin_count: usize,
    vout_count: usize,
    value_out: u64,
}

impl From<Transaction> for TransactionSummary {
    fn from(tx: Transaction) -> Self {
        // Use saturating_add to prevent overflow when summing output values
        let value_out = tx
            .outputs
            .iter()
            .fold(0u64, |acc, o| acc.saturating_add(o.value));
        Self {
            txid: hex::encode(tx.txid()),
            version: tx.version,
            lock_time: tx.lock_time,
            vin_count: tx.inputs.len(),
            vout_count: tx.outputs.len(),
            value_out,
        }
    }
}

impl From<TransactionSummary> for TxInfo {
    fn from(summary: TransactionSummary) -> Self {
        TxInfo {
            txid: summary.txid,
            version: summary.version,
            locktime: summary.lock_time,
            vin_count: summary.vin_count,
            vout_count: summary.vout_count,
            value_out: summary.value_out,
        }
    }
}
