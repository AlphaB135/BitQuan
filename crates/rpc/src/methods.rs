//! RPC method implementations

use crate::{error_codes, JsonRpcResponse, RpcError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ... [Struct definitions remain the same] ...

/// Block template for mining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTemplate {
    /// Block version
    pub version: i32,
    /// Previous block hash (hex)
    pub previousblockhash: String,
    /// Transactions to include (hex-encoded)
    pub transactions: Vec<String>,
    /// Merkle root (hex)
    pub merkleroot: String,
    /// Target difficulty (hex)
    pub target: String,
    /// Current time
    pub curtime: u32,
    /// Bits (compact target)
    pub bits: u32,
    /// Block height
    pub height: u64,
}

/// Mining work data (getwork style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkData {
    /// Block header data (hex, 80 bytes)
    pub data: String,
    /// Hash target (hex, 32 bytes)
    pub target: String,
}

/// Get blockchain info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    /// Chain name
    pub chain: String,
    /// Current block count
    pub blocks: u64,
    /// Best block hash
    pub bestblockhash: String,
    /// Difficulty
    pub difficulty: f64,
    /// Chain work (hex)
    pub chainwork: String,
}

/// Mining info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningInfo {
    /// Current block count
    pub blocks: u64,
    /// Current difficulty
    pub difficulty: f64,
    /// Network hash rate estimate
    pub networkhashps: f64,
}

/// Transaction info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInfo {
    /// Transaction ID (hex)
    pub txid: String,
    /// Transaction version
    pub version: i32,
    /// Lock time
    pub locktime: u32,
    /// Inputs count
    pub vin_count: usize,
    /// Outputs count
    pub vout_count: usize,
    /// Total output value
    pub value_out: u128,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatsResponse {
    /// Current chain height
    pub height: u64,
    /// Total rewards distributed (satoshis)
    pub total_rewards: u64,
    /// Number of active miners
    pub miner_count: u64,
    /// Pool balance (satoshis)
    pub pool_balance: u64,
    /// Total blocks mined
    pub block_count: u64,
}

/// Miner info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStatsResponse {
    /// Miner ID
    pub miner_id: String,
    /// Total reward earned (satoshis)
    pub total_reward: u64,
    /// Number of blocks mined
    pub blocks_mined: u64,
    /// Recent blocks (limited)
    pub recent_blocks: Vec<MinerBlock>,
}

/// Miner block info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerBlock {
    /// Block hash
    pub hash: String,
    /// Block height
    pub height: u64,
    /// Reward (satoshis)
    pub reward: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Payout request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutRequest {
    /// Miner ID
    pub miner_id: String,
    /// Amount to pay out (satoshis)
    pub amount: u64,
}

/// Payout response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutResponse {
    /// Payout ID
    pub payout_id: String,
    /// Transaction ID (if available)
    pub txid: Option<String>,
}

/// Network status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatusResponse {
    /// Number of connected peers
    pub peers_connected: u64,
    /// Total blocks broadcast
    pub blocks_broadcast: u64,
    /// Total blocks received
    pub blocks_received: u64,
    /// Sync status
    pub sync_status: String,
    /// Local chain height
    pub local_height: u64,
    /// Best known height
    pub best_height: u64,
}

/// Sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Sync status
    pub status: String,
    /// Local chain height
    pub local_height: u64,
    /// Best known height from peers
    pub best_height: u64,
    /// Blocks behind
    pub blocks_behind: u64,
    /// Sync progress percentage
    pub progress: f64,
    /// Whether sync is currently in progress
    pub syncing: bool,
    /// Last sync attempt timestamp
    pub last_sync_attempt: u64,
    /// Sync errors count
    pub sync_errors: u64,
}

/// RPC method handler trait
#[async_trait]
pub trait RpcMethods: Send + Sync {
    /// Get current block count
    async fn getblockcount(&self) -> Result<u64, RpcError>;

    /// Get blockchain info
    async fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError>;

    /// Get mining info
    async fn getmininginfo(&self) -> Result<MiningInfo, RpcError>;

    /// Get block template for mining
    async fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError>;

    /// Get work for mining (simple interface)
    async fn getwork(&self) -> Result<WorkData, RpcError>;

    /// Submit mined block
    async fn submitblock(&self, block_hex: String) -> Result<bool, RpcError>;

    /// Submit work (getwork style)
    async fn submitwork(&self, data: String) -> Result<bool, RpcError>;

    /// Get transaction by txid
    async fn gettransaction(&self, txid: String) -> Result<TxInfo, RpcError>;

    /// Submit transaction to network
    async fn submittransaction(&self, tx_hex: String) -> Result<String, RpcError>;

    /// Get best block hash
    async fn getbestblockhash(&self) -> Result<String, RpcError>;

    /// Get block hash by height
    async fn getblockhash(&self, height: u64) -> Result<String, RpcError>;

    /// Get pool statistics
    async fn getpoolstats(&self) -> Result<PoolStatsResponse, RpcError>;

    /// Get miner statistics
    async fn getminerstats(&self, miner_id: String) -> Result<MinerStatsResponse, RpcError>;

    /// Create payout (mock implementation)
    async fn createpayout(&self, request: PayoutRequest) -> Result<PayoutResponse, RpcError>;

    /// Get network status
    async fn getnetworkstatus(&self) -> Result<NetworkStatusResponse, RpcError>;

    /// Get sync status or trigger sync
    async fn sync(&self) -> Result<SyncResponse, RpcError>;

    /// Mine blocks immediately (for testing/regtest)
    ///
    /// # Arguments
    /// * `n_blocks` - Number of blocks to mine
    /// * `address` - Optional address for coinbase output (uses default if None)
    async fn generate(
        &self,
        n_blocks: u64,
        address: Option<String>,
    ) -> Result<Vec<String>, RpcError>;

    /// Mine blocks to a specific address (for testing/regtest)
    ///
    /// # Arguments
    /// * `n_blocks` - Number of blocks to mine
    /// * `address` - Address for coinbase output
    async fn generatetoaddress(
        &self,
        n_blocks: u64,
        address: String,
    ) -> Result<Vec<String>, RpcError>;

    /// Send to an address (wallet operation)
    ///
    /// # Arguments
    /// * `address` - Recipient address
    /// * `amount` - Amount to send in qbits
    /// * `comment` - Optional comment
    async fn sendtoaddress(
        &self,
        address: String,
        amount: u128,
        comment: Option<String>,
    ) -> Result<String, RpcError>;
}

/// Dispatch RPC call to appropriate method
pub async fn dispatch_call<T: RpcMethods>(
    handler: &T,
    method: &str,
    params: Value,
    id: Value,
) -> JsonRpcResponse {
    match method {
        "getblockcount" => match handler.getblockcount().await {
            Ok(count) => JsonRpcResponse::success(id, serde_json::json!(count)),
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getblockchaininfo" => match handler.getblockchaininfo().await {
            Ok(info) => match serde_json::to_value(info) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getmininginfo" => match handler.getmininginfo().await {
            Ok(info) => match serde_json::to_value(info) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getblocktemplate" => match handler.getblocktemplate().await {
            Ok(template) => match serde_json::to_value(template) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getwork" => match handler.getwork().await {
            Ok(work) => match serde_json::to_value(work) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "submitblock" => {
            if let Some(block_hex) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
            {
                match handler.submitblock(block_hex.to_string()).await {
                    Ok(accepted) => JsonRpcResponse::success(id, serde_json::json!(accepted)),
                    Err(e) => {
                        JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                    }
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "expected block hex".to_string(),
                )
            }
        }

        "submitwork" => {
            if let Some(data) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
            {
                match handler.submitwork(data.to_string()).await {
                    Ok(accepted) => JsonRpcResponse::success(id, serde_json::json!(accepted)),
                    Err(e) => {
                        JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                    }
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "expected work data".to_string(),
                )
            }
        }

        "gettransaction" => {
            if let Some(txid) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
            {
                match handler.gettransaction(txid.to_string()).await {
                    Ok(tx) => match serde_json::to_value(tx) {
                        Ok(v) => JsonRpcResponse::success(id, v),
                        Err(e) => JsonRpcResponse::error(
                            id,
                            error_codes::INTERNAL_ERROR,
                            format!("serialization error: {}", e),
                        ),
                    },
                    Err(e) => {
                        JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                    }
                }
            } else {
                JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "expected txid".to_string())
            }
        }

        "submittransaction" => {
            if let Some(tx_hex) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
            {
                match handler.submittransaction(tx_hex.to_string()).await {
                    Ok(txid) => JsonRpcResponse::success(id, serde_json::json!(txid)),
                    Err(e) => {
                        JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                    }
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "expected transaction hex".to_string(),
                )
            }
        }

        "getbestblockhash" => match handler.getbestblockhash().await {
            Ok(hash) => JsonRpcResponse::success(id, serde_json::json!(hash)),
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getblockhash" => {
            if let Some(height) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
            {
                match handler.getblockhash(height).await {
                    Ok(hash) => JsonRpcResponse::success(id, serde_json::json!(hash)),
                    Err(e) => {
                        JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                    }
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "expected height".to_string(),
                )
            }
        }

        "getpoolstats" => match handler.getpoolstats().await {
            Ok(stats) => match serde_json::to_value(stats) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getminerstats" => {
            if let Some(miner_id) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
            {
                match handler.getminerstats(miner_id.to_string()).await {
                    Ok(stats) => match serde_json::to_value(stats) {
                        Ok(v) => JsonRpcResponse::success(id, v),
                        Err(e) => JsonRpcResponse::error(
                            id,
                            error_codes::INTERNAL_ERROR,
                            format!("serialization error: {}", e),
                        ),
                    },
                    Err(e) => {
                        JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                    }
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "expected miner_id".to_string(),
                )
            }
        }

        "createpayout" => {
            if let Some(params_obj) = params.as_object() {
                if let (Some(miner_id), Some(amount)) = (
                    params_obj.get("miner_id").and_then(|v| v.as_str()),
                    params_obj.get("amount").and_then(|v| v.as_u64()),
                ) {
                    let request = PayoutRequest {
                        miner_id: miner_id.to_string(),
                        amount,
                    };
                    match handler.createpayout(request).await {
                        Ok(response) => match serde_json::to_value(response) {
                            Ok(v) => JsonRpcResponse::success(id, v),
                            Err(e) => JsonRpcResponse::error(
                                id,
                                error_codes::INTERNAL_ERROR,
                                format!("serialization error: {}", e),
                            ),
                        },
                        Err(e) => {
                            JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string())
                        }
                    }
                } else {
                    JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "expected miner_id and amount".to_string(),
                    )
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "expected object with miner_id and amount".to_string(),
                )
            }
        }

        "getnetworkstatus" => match handler.getnetworkstatus().await {
            Ok(status) => match serde_json::to_value(status) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "sync" => match handler.sync().await {
            Ok(sync_info) => match serde_json::to_value(sync_info) {
                Ok(v) => JsonRpcResponse::success(id, v),
                Err(e) => JsonRpcResponse::error(
                    id,
                    error_codes::INTERNAL_ERROR,
                    format!("serialization error: {}", e),
                ),
            },
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "generate" => {
            // Parse params: [n_blocks, address?] or [n_blocks]
            let n_blocks = match params.as_array() {
                Some(arr) if !arr.is_empty() => match arr[0].as_u64() {
                    Some(n) => n,
                    None => {
                        return JsonRpcResponse::error(
                            id,
                            error_codes::INVALID_PARAMS,
                            "n_blocks must be a number".to_string(),
                        )
                    }
                },
                _ => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "generate requires at least n_blocks parameter".to_string(),
                    )
                }
            };

            let address = match params.as_array() {
                Some(arr) if arr.len() > 1 => {
                    arr.get(1).and_then(|v| v.as_str()).map(|s| s.to_string())
                }
                _ => None,
            };

            match handler.generate(n_blocks, address).await {
                Ok(block_hashes) => JsonRpcResponse::success(id, serde_json::json!(block_hashes)),
                Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
            }
        }

        "generatetoaddress" => {
            // Parse params: [n_blocks, address]
            let arr = match params.as_array() {
                Some(a) if !a.is_empty() => a,
                _ => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "generatetoaddress requires [n_blocks, address] parameters".to_string(),
                    )
                }
            };

            let n_blocks = match arr.first().and_then(|v| v.as_u64()) {
                Some(n) => n,
                None => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "n_blocks must be a number".to_string(),
                    )
                }
            };

            let address = match arr.get(1).and_then(|v| v.as_str()) {
                Some(addr) => addr.to_string(),
                None => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "address parameter is required".to_string(),
                    )
                }
            };

            match handler.generatetoaddress(n_blocks, address).await {
                Ok(block_hashes) => JsonRpcResponse::success(id, serde_json::json!(block_hashes)),
                Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
            }
        }

        "sendtoaddress" => {
            // Parse params: [address, amount, comment?]
            let arr = match params.as_array() {
                Some(a) if a.len() >= 2 => a,
                _ => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "sendtoaddress requires [address, amount] parameters".to_string(),
                    )
                }
            };

            let address = match arr.first().and_then(|v| v.as_str()) {
                Some(addr) => addr.to_string(),
                None => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "address parameter is required".to_string(),
                    )
                }
            };

            let amount = match arr.get(1) {
                Some(v) => {
                    // Try u64 first for smaller values
                    if let Some(amt) = v.as_u64() {
                        amt as u128
                    } else if let Some(s) = v.as_str() {
                        // Parse from string for larger values (u128)
                        match s.parse::<u128>() {
                            Ok(val) => val,
                            Err(_) => {
                                return JsonRpcResponse::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    "amount parameter must be a valid u128 number".to_string(),
                                )
                            }
                        }
                    } else {
                        return JsonRpcResponse::error(
                            id,
                            error_codes::INVALID_PARAMS,
                            "amount parameter is required (u128)".to_string(),
                        );
                    }
                }
                None => {
                    return JsonRpcResponse::error(
                        id,
                        error_codes::INVALID_PARAMS,
                        "amount parameter is required (u128)".to_string(),
                    )
                }
            };

            let comment = arr.get(2).and_then(|v| v.as_str()).map(|s| s.to_string());

            match handler.sendtoaddress(address, amount, comment).await {
                Ok(txid) => JsonRpcResponse::success(id, serde_json::json!(txid)),
                Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
            }
        }

        _ => JsonRpcResponse::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method '{}' not found", method),
        ),
    }
}
