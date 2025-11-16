use thiserror::Error;

/// BitQuan SDK error types
#[derive(Error, Debug)]
pub enum BitQuanError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Cryptographic error: {0}")]
    Crypto(String),
    
    #[error("Wallet error: {0}")]
    Wallet(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Invalid private key")]
    InvalidPrivateKey,
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("RPC error: {code} - {message}")]
    Rpc { code: i32, message: String },
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Timeout error")]
    Timeout,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BitQuanError>;