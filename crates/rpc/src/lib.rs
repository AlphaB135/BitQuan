//! JSON-RPC interface for BitQuan node operations.
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use ipnetwork::IpNetwork;

pub mod methods;
pub mod server;

/// Runtime configuration options for the RPC server.
#[derive(Clone, Debug)]
pub struct RpcConfig {
    /// Maximum request body size in bytes (default: 1 MiB).
    pub max_body_bytes: usize,
    /// Token bucket burst size per IP.
    pub rl_burst: u32,
    /// Token refill rate (tokens per second) per IP.
    pub rl_refill_per_sec: u32,
    /// Cooldown applied after each request (milliseconds).
    pub conn_cooldown_ms: u64,
    /// Whether to trust proxy headers for client IP detection.
    pub trust_proxy: bool,
    /// List of trusted proxy CIDR ranges.
    pub trusted_proxies: Vec<IpNetwork>,
    /// Maximum allowed size of HTTP headers in bytes.
    pub max_header_bytes: usize,
    /// Header read timeout in milliseconds.
    pub header_read_timeout_ms: u64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            max_body_bytes: 1_048_576,
            rl_burst: 20,
            rl_refill_per_sec: 10,
            conn_cooldown_ms: 10,
             trust_proxy: false,
             trusted_proxies: Vec::new(),
             max_header_bytes: 8 * 1024,
             header_read_timeout_ms: 1_000,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util;

/// RPC error types
#[derive(Debug, Error)]
pub enum RpcError {
    /// Invalid parameters
    #[error("invalid params: {0}")]
    InvalidParams(String),
    /// Method not found
    #[error("method not found: {0}")]
    MethodNotFound(String),
    /// Internal error
    #[error("internal error: {0}")]
    InternalError(String),
    /// Parse error
    #[error("parse error: {0}")]
    ParseError(String),
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Parameters (optional)
    #[serde(default)]
    pub params: serde_json::Value,
    /// Request ID
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Result (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Request ID
    pub id: serde_json::Value,
}

/// JSON-RPC Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// Create success response
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create error response
    pub fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}

/// RPC error codes (JSON-RPC 2.0 standard + custom)
pub mod error_codes {
    /// Parse error
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "getblockcount");
        assert_eq!(req.id, 1);
    }

    #[test]
    fn test_response_serialization() {
        let resp = JsonRpcResponse::success(
            serde_json::Value::Number(1.into()),
            serde_json::json!({"height": 12345}),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(json.contains("12345"));
    }

    #[test]
    fn test_error_response() {
        let resp = JsonRpcResponse::error(
            serde_json::Value::Number(1.into()),
            error_codes::METHOD_NOT_FOUND,
            "Method not found".to_string(),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32601"));
    }
}
