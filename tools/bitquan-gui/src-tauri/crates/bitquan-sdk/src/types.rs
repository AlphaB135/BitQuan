use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Network identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl Default for Network {
    fn default() -> Self {
        Network::Mainnet
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Devnet => write!(f, "devnet"),
        }
    }
}

/// Block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub hash: String,
    pub height: u64,
    pub timestamp: DateTime<Utc>,
    pub transactions: Vec<String>,
    pub previous_hash: String,
    pub merkle_root: String,
    pub difficulty: u64,
    pub nonce: u64,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub stake: Option<u64>,
    pub created_at: DateTime<Utc>,
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// RPC request wrapper
#[derive(Debug, Serialize)]
pub struct RpcRequest<T> {
    pub jsonrpc: String,
    pub method: String,
    pub params: T,
    pub id: u64,
}

impl<T> RpcRequest<T> {
    pub fn new(method: &str, params: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: uuid::Uuid::new_v4().as_u128() as u64,
        }
    }
}

/// RPC response wrapper
#[derive(Debug, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub result: Option<T>,
    pub error: Option<RpcError>,
    pub id: u64,
}

/// RPC error
#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub network: Network,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: crate::DEFAULT_RPC_URL.to_string(),
            network: Network::Mainnet,
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

impl Config {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            ..Default::default()
        }
    }

    pub fn with_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}