use crate::error::{BitQuanError, Result};
use crate::types::*;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;


/// BitQuan blockchain client
#[derive(Debug, Clone)]
pub struct BitQuanClient {
    client: Client,
    config: Config,
}

impl BitQuanClient {
    /// Create a new client with default configuration
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    /// Create a new client with custom RPC URL
    pub fn new_with_url(rpc_url: impl Into<String>) -> Self {
        Self::with_config(Config::new(rpc_url))
    }

    /// Create a new client with custom configuration
    pub fn with_config(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Make an RPC call
    async fn rpc_call<T, R>(&self, method: &str, params: T) -> Result<R>
    where
        T: serde::Serialize,
        R: DeserializeOwned,
    {
        let request = RpcRequest::new(method, params);
        
        let mut retries = 0;
        loop {
            let response = self.client
                .post(&self.config.rpc_url)
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let rpc_response: RpcResponse<R> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(BitQuanError::Rpc {
                        code: error.code,
                        message: error.message,
                    });
                }
                
                return rpc_response.result
                    .ok_or_else(|| BitQuanError::Internal("No result in RPC response".to_string()));
            } else {
                retries += 1;
                if retries >= self.config.max_retries {
                    return Err(BitQuanError::Internal("Network error after retries".to_string()));
                }
                tokio::time::sleep(Duration::from_millis(1000 * retries as u64)).await;
            }
        }
    }

    /// Get blockchain information
    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        self.rpc_call("getblockchaininfo", ()).await
    }

    /// Get block by height
    pub async fn get_block_by_height(&self, height: u64) -> Result<Block> {
        self.rpc_call("getblockbyheight", (height,)).await
    }

    /// Get block by hash
    pub async fn get_block_by_hash(&self, hash: &str) -> Result<Block> {
        self.rpc_call("getblockbyhash", (hash,)).await
    }

    /// Get account information
    pub async fn get_account(&self, address: &str) -> Result<Account> {
        self.rpc_call("getaccount", (address,)).await
    }

    /// Get balance for an address
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        self.rpc_call("getbalance", (address,)).await
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &str) -> Result<TransactionInfo> {
        self.rpc_call("gettransaction", (tx_hash,)).await
    }

    /// Send raw transaction
    pub async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String> {
        self.rpc_call("sendrawtransaction", (tx_hex,)).await
    }

    /// Get current block height
    pub async fn get_block_height(&self) -> Result<u64> {
        self.rpc_call("getblockheight", ()).await
    }

    /// Get network hash rate
    pub async fn get_network_hash_rate(&self) -> Result<f64> {
        self.rpc_call("getnetworkhashps", ()).await
    }

    /// Get peer information
    pub async fn get_peer_info(&self) -> Result<Vec<PeerInfo>> {
        self.rpc_call("getpeerinfo", ()).await
    }

    /// Estimate transaction fee
    pub async fn estimate_fee(&self, tx_size: u64) -> Result<u64> {
        self.rpc_call("estimatefee", (tx_size,)).await
    }

    /// Validate address
    pub async fn validate_address(&self, address: &str) -> Result<bool> {
        self.rpc_call("validateaddress", (address,)).await
    }

    /// Get mempool information
    pub async fn get_mempool_info(&self) -> Result<MempoolInfo> {
        self.rpc_call("getmempoolinfo", ()).await
    }

    /// Get raw mempool transactions
    pub async fn get_raw_mempool(&self) -> Result<Vec<String>> {
        self.rpc_call("getrawmempool", ()).await
    }
}

impl Default for BitQuanClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Additional types for RPC responses
#[derive(Debug, serde::Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub best_block_hash: String,
    pub difficulty: f64,
    pub median_time: u64,
    pub verification_progress: f64,
    pub chainwork: String,
    pub size_on_disk: u64,
    pub pruned: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct TransactionInfo {
    pub txid: String,
    pub hash: String,
    pub version: i32,
    pub size: u64,
    pub vsize: u64,
    pub weight: u64,
    pub locktime: u64,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
    pub blockhash: Option<String>,
    pub confirmations: u64,
    pub time: u64,
    pub blocktime: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Vin {
    pub txid: String,
    pub vout: u32,
    pub script_sig: Option<ScriptSig>,
    pub sequence: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScriptSig {
    pub asm: String,
    pub hex: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Vout {
    pub value: u64,
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    pub req_sigs: Option<u32>,
    #[serde(rename = "type")]
    pub script_type: String,
    pub addresses: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PeerInfo {
    pub id: i32,
    pub addr: String,
    pub addr_local: Option<String>,
    pub services: String,
    pub relaytxes: bool,
    pub lastsend: u64,
    pub lastrecv: u64,
    pub pingtime: Option<f64>,
    pub version: u32,
    pub subver: String,
    pub inbound: bool,
    pub startingheight: i64,
    pub banscore: i32,
    pub synced_headers: i64,
    pub synced_blocks: i64,
    pub inflight: Vec<String>,
    pub whitelisted: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct MempoolInfo {
    pub size: u64,
    pub bytes: u64,
    pub usage: u64,
    pub maxmempool: u64,
    pub mempoolminfee: f64,
    pub minrelaytxfee: f64,
}