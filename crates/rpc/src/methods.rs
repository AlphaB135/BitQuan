//! RPC method implementations

use crate::{error_codes, JsonRpcResponse, RpcError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub value_out: u64,
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

/// RPC method handler trait
pub trait RpcMethods {
    /// Get current block count
    fn getblockcount(&self) -> Result<u64, RpcError>;

    /// Get blockchain info
    fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError>;

    /// Get mining info
    fn getmininginfo(&self) -> Result<MiningInfo, RpcError>;

    /// Get block template for mining
    fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError>;

    /// Get work for mining (simple interface)
    fn getwork(&self) -> Result<WorkData, RpcError>;

    /// Submit mined block
    fn submitblock(&self, block_hex: String) -> Result<bool, RpcError>;

    /// Submit work (getwork style)
    fn submitwork(&self, data: String) -> Result<bool, RpcError>;

    /// Get transaction by txid
    fn gettransaction(&self, txid: String) -> Result<TxInfo, RpcError>;

    /// Submit transaction to network
    fn submittransaction(&self, tx_hex: String) -> Result<String, RpcError>;

    /// Get best block hash
    fn getbestblockhash(&self) -> Result<String, RpcError>;

    /// Get block hash by height
    fn getblockhash(&self, height: u64) -> Result<String, RpcError>;

    /// Get pool statistics
    fn getpoolstats(&self) -> Result<PoolStatsResponse, RpcError>;

    /// Get miner statistics
    fn getminerstats(&self, miner_id: String) -> Result<MinerStatsResponse, RpcError>;

    /// Create payout (mock implementation)
    fn createpayout(&self, request: PayoutRequest) -> Result<PayoutResponse, RpcError>;

    /// Get network status
    fn getnetworkstatus(&self) -> Result<NetworkStatusResponse, RpcError>;
}

/// Dispatch RPC call to appropriate method
pub fn dispatch_call<T: RpcMethods>(
    handler: &T,
    method: &str,
    params: Value,
    id: Value,
) -> JsonRpcResponse {
    match method {
        "getblockcount" => match handler.getblockcount() {
            Ok(count) => JsonRpcResponse::success(id, serde_json::json!(count)),
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getblockchaininfo" => match handler.getblockchaininfo() {
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

        "getmininginfo" => match handler.getmininginfo() {
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

        "getblocktemplate" => match handler.getblocktemplate() {
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

        "getwork" => match handler.getwork() {
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
                match handler.submitblock(block_hex.to_string()) {
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
                match handler.submitwork(data.to_string()) {
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
                match handler.gettransaction(txid.to_string()) {
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
                match handler.submittransaction(tx_hex.to_string()) {
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

        "getbestblockhash" => match handler.getbestblockhash() {
            Ok(hash) => JsonRpcResponse::success(id, serde_json::json!(hash)),
            Err(e) => JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e.to_string()),
        },

        "getblockhash" => {
            if let Some(height) = params
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
            {
                match handler.getblockhash(height) {
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

        "getpoolstats" => match handler.getpoolstats() {
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
                match handler.getminerstats(miner_id.to_string()) {
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
                    match handler.createpayout(request) {
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

        "getnetworkstatus" => match handler.getnetworkstatus() {
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

        _ => JsonRpcResponse::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method '{}' not found", method),
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    struct MockRpc;

    impl RpcMethods for MockRpc {
        fn getblockcount(&self) -> Result<u64, RpcError> {
            Ok(12345)
        }

        fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError> {
            Ok(BlockchainInfo {
                chain: "main".to_string(),
                blocks: 12345,
                bestblockhash: "00000000000000000".to_string(),
                difficulty: 1234.5,
                chainwork: "0000000000000000".to_string(),
            })
        }

        fn getmininginfo(&self) -> Result<MiningInfo, RpcError> {
            Ok(MiningInfo {
                blocks: 12345,
                difficulty: 1234.5,
                networkhashps: 1e12,
            })
        }

        fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError> {
            Err(RpcError::InternalError("not implemented".to_string()))
        }

        fn submitblock(&self, _block_hex: String) -> Result<bool, RpcError> {
            Ok(true)
        }

        fn gettransaction(&self, _txid: String) -> Result<TxInfo, RpcError> {
            Err(RpcError::InternalError("not found".to_string()))
        }

        fn submittransaction(&self, _tx_hex: String) -> Result<String, RpcError> {
            Ok("test_txid_1234567890abcdef".to_string())
        }

        fn getbestblockhash(&self) -> Result<String, RpcError> {
            Ok("00000000000000000".to_string())
        }

        fn getblockhash(&self, _height: u64) -> Result<String, RpcError> {
            Ok("00000000000000000".to_string())
        }

        fn getwork(&self) -> Result<WorkData, RpcError> {
            Ok(WorkData {
                data: "00000000000000000000000000000000".to_string(),
                target: "00000000ffff0000000000000000000000000000000000000000000000000000"
                    .to_string(),
            })
        }

        fn submitwork(&self, _data: String) -> Result<bool, RpcError> {
            Ok(true)
        }

        fn getpoolstats(&self) -> Result<PoolStatsResponse, RpcError> {
            Ok(PoolStatsResponse {
                height: 12345,
                total_rewards: 1000000000,
                miner_count: 5,
                pool_balance: 500000000,
                block_count: 100,
            })
        }

        fn getminerstats(&self, _miner_id: String) -> Result<MinerStatsResponse, RpcError> {
            Ok(MinerStatsResponse {
                miner_id: "test_miner".to_string(),
                total_reward: 50000000,
                blocks_mined: 10,
                recent_blocks: vec![],
            })
        }

        fn createpayout(&self, _request: PayoutRequest) -> Result<PayoutResponse, RpcError> {
            Ok(PayoutResponse {
                payout_id: "payout123".to_string(),
                txid: Some("tx456".to_string()),
            })
        }

        fn getnetworkstatus(&self) -> Result<NetworkStatusResponse, RpcError> {
            Ok(NetworkStatusResponse {
                peers_connected: 5,
                blocks_broadcast: 100,
                blocks_received: 150,
                sync_status: "synced".to_string(),
                local_height: 12345,
                best_height: 12345,
            })
        }
    }

    #[test]
    fn test_dispatch_getblockcount() {
        let handler = MockRpc;
        let response = dispatch_call(
            &handler,
            "getblockcount",
            serde_json::json!([]),
            serde_json::json!(1),
        );
        assert!(response.result.is_some());
        assert_eq!(response.result.expect("Response should have result"), 12345);
    }

    #[test]
    fn test_dispatch_unknown_method() {
        let handler = MockRpc;
        let response = dispatch_call(
            &handler,
            "unknownmethod",
            serde_json::json!([]),
            serde_json::json!(1),
        );
        assert!(response.error.is_some());
        assert_eq!(
            response.error.expect("Response should have error").code,
            error_codes::METHOD_NOT_FOUND
        );
    }
}
