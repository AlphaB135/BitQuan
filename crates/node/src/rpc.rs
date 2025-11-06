//! RPC handler implementation for the BitQuan node.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use bitquan_consensus::header_hash;
use bitquan_rpc::{
    methods::{
        BlockTemplate, BlockchainInfo, MinerStatsResponse, MiningInfo, NetworkStatusResponse,
        PayoutRequest, PayoutResponse, PoolStatsResponse, RpcMethods, TxInfo, WorkData,
    },
    RpcError,
};
use bitquan_storage::{rocksdb_store::RocksDBStore, ChainStore, StorageError};
use bitquan_types::{Transaction, GENESIS_BITS};
use hex::FromHex;

/// Node RPC handler backed by a RocksDB chain store.
pub struct NodeRpcHandler {
    store: Arc<Mutex<RocksDBStore>>,
    chain_name: String,
}

impl NodeRpcHandler {
    /// Create a new RPC handler using the given store.
    pub fn new(store: Arc<Mutex<RocksDBStore>>, chain_name: impl Into<String>) -> Self {
        Self {
            store,
            chain_name: chain_name.into(),
        }
    }

    fn with_store<F, R>(&self, f: F) -> Result<R, RpcError>
    where
        F: FnOnce(&RocksDBStore) -> Result<R, StorageError>,
    {
        let guard = self
            .store
            .lock()
            .map_err(|e| RpcError::InternalError(format!("storage poisoned: {e}")))?;
        f(&guard).map_err(storage_to_rpc)
    }
}

impl RpcMethods for NodeRpcHandler {
    fn getblockcount(&self) -> Result<u64, RpcError> {
        self.with_store(|store| store.height())
    }

    fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError> {
        self.with_store(|store| {
            let height = store.height()?;
            let tip_hash = store.tip()?.map(|header| hex::encode(header_hash(&header)));
            let difficulty = tip_hash
                .as_ref()
                .and_then(|_| store.tip().ok())
                .flatten()
                .map(|header| difficulty_from_bits(header.bits))
                .unwrap_or(1.0);

            Ok(BlockchainInfo {
                chain: self.chain_name.clone(),
                blocks: height,
                bestblockhash: tip_hash.unwrap_or_else(|| String::from("0")),
                difficulty,
                chainwork: String::from("0"),
            })
        })
    }

    fn getmininginfo(&self) -> Result<MiningInfo, RpcError> {
        let blocks = self.getblockcount()?;
        let difficulty = self.with_store(|store| {
            Ok(store
                .tip()?
                .map(|header| difficulty_from_bits(header.bits))
                .unwrap_or(1.0))
        })?;
        Ok(MiningInfo {
            blocks,
            difficulty,
            networkhashps: 0.0,
        })
    }

    fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError> {
        Err(RpcError::InternalError(
            "getblocktemplate not implemented".into(),
        ))
    }

    fn getwork(&self) -> Result<WorkData, RpcError> {
        Err(RpcError::InternalError("getwork not implemented".into()))
    }

    fn submitblock(&self, _block_hex: String) -> Result<bool, RpcError> {
        Err(RpcError::InternalError(
            "submitblock not implemented".into(),
        ))
    }

    fn submitwork(&self, _data: String) -> Result<bool, RpcError> {
        Err(RpcError::InternalError("submitwork not implemented".into()))
    }

    fn gettransaction(&self, txid: String) -> Result<TxInfo, RpcError> {
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
            .with_store(|store| store.get_transaction(&id))?
            .ok_or_else(|| RpcError::InternalError("transaction not found".into()))?;

        let summary = TransactionSummary::from(tx);
        Ok(summary.into())
    }

    fn getbestblockhash(&self) -> Result<String, RpcError> {
        self.with_store(|store| {
            let tip = store.tip()?;
            Ok(tip
                .map(|header| hex::encode(header_hash(&header)))
                .unwrap_or_else(|| String::from("0")))
        })
    }

    fn getblockhash(&self, height: u64) -> Result<String, RpcError> {
        self.with_store(|store| {
            let block = store
                .get_block_by_height(height)?
                .ok_or(StorageError::BlockNotFound)?;
            Ok(hex::encode(header_hash(&block.header)))
        })
    }

    fn getpoolstats(&self) -> Result<PoolStatsResponse, RpcError> {
        // Pool stats require reward engine integration
        // For now, return placeholder values from chain state
        let height = self.getblockcount()?;
        Ok(PoolStatsResponse {
            height,
            total_rewards: 0,
            miner_count: 0,
            pool_balance: 0,
            block_count: height,
        })
    }

    fn getminerstats(&self, miner_id: String) -> Result<MinerStatsResponse, RpcError> {
        // Miner stats require reward engine integration
        // For now, return placeholder
        Ok(MinerStatsResponse {
            miner_id,
            total_reward: 0,
            blocks_mined: 0,
            recent_blocks: vec![],
        })
    }

    fn createpayout(&self, _request: PayoutRequest) -> Result<PayoutResponse, RpcError> {
        // Payout creation requires reward engine integration
        // For now, return mock success
        Ok(PayoutResponse {
            payout_id: format!("payout_{}", uuid::Uuid::new_v4()),
            txid: None,
        })
    }

    fn getnetworkstatus(&self) -> Result<NetworkStatusResponse, RpcError> {
        // Network status requires network manager integration
        // For now, return placeholder values
        Ok(NetworkStatusResponse {
            peers_connected: 0,
            blocks_broadcast: 0,
            blocks_received: 0,
            sync_status: "idle".to_string(),
            local_height: self.getblockcount()?,
            best_height: self.getblockcount()?,
        })
    }
}

fn storage_to_rpc(err: StorageError) -> RpcError {
    RpcError::InternalError(err.to_string())
}

fn difficulty_from_bits(bits: u32) -> f64 {
    let max_target = bitquan_consensus::compact_to_target(GENESIS_BITS);
    let target = bitquan_consensus::compact_to_target(bits);
    if target <= 0.0 {
        return 0.0;
    }
    max_target / target
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
