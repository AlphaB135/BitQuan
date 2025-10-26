//! HTTP server for handling JSON-RPC requests

use crate::{error_codes, methods, JsonRpcRequest, JsonRpcResponse};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

/// Simple HTTP JSON-RPC server
pub struct RpcServer<T> {
    handler: Arc<T>,
    addr: String,
}

impl<T: methods::RpcMethods + Send + Sync + 'static> RpcServer<T> {
    /// Create new RPC server
    pub fn new(handler: T, addr: String) -> Self {
        Self {
            handler: Arc::new(handler),
            addr,
        }
    }

    /// Start serving requests (blocking)
    pub fn serve(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.addr)?;
        println!("RPC server listening on {}", self.addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handler = Arc::clone(&self.handler);
                    std::thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, handler.as_ref()) {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Connection error: {}", e),
            }
        }
        Ok(())
    }
}

fn handle_connection<T: methods::RpcMethods>(
    mut stream: TcpStream,
    handler: &T,
) -> std::io::Result<()> {
    let buf_reader = BufReader::new(&stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map_while(Result::ok)
        .take_while(|line| !line.is_empty())
        .collect();

    // Read Content-Length header
    let content_length = http_request
        .iter()
        .find(|line| line.to_lowercase().starts_with("content-length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if content_length == 0 {
        let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    // Read body
    let mut body = vec![0u8; content_length];
    stream.read_exact(&mut body)?;

    // Parse JSON-RPC request
    let json_response = match serde_json::from_slice::<JsonRpcRequest>(&body) {
        Ok(request) => {
            if request.jsonrpc != "2.0" {
                JsonRpcResponse::error(
                    request.id,
                    error_codes::INVALID_REQUEST,
                    "Invalid JSON-RPC version".to_string(),
                )
            } else {
                methods::dispatch_call(handler, &request.method, request.params, request.id)
            }
        }
        Err(e) => JsonRpcResponse::error(
            serde_json::Value::Null,
            error_codes::PARSE_ERROR,
            format!("Parse error: {}", e),
        ),
    };

    // Serialize response
    let response_body = serde_json::to_string(&json_response).unwrap();

    // Send HTTP response
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    stream.write_all(http_response.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::{BlockchainInfo, MiningInfo, RpcMethods, BlockTemplate, TxInfo};
    use crate::RpcError;

    struct TestHandler;

    impl RpcMethods for TestHandler {
        fn getblockcount(&self) -> Result<u64, RpcError> {
            Ok(100)
        }

        fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError> {
            Ok(BlockchainInfo {
                chain: "test".to_string(),
                blocks: 100,
                bestblockhash: "test".to_string(),
                difficulty: 1.0,
                chainwork: "0".to_string(),
            })
        }

        fn getmininginfo(&self) -> Result<MiningInfo, RpcError> {
            Ok(MiningInfo {
                blocks: 100,
                difficulty: 1.0,
                networkhashps: 1000.0,
            })
        }

        fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError> {
            Err(RpcError::InternalError("not implemented".to_string()))
        }

        fn submitblock(&self, _: String) -> Result<bool, RpcError> {
            Ok(true)
        }

        fn gettransaction(&self, _: String) -> Result<TxInfo, RpcError> {
            Err(RpcError::InternalError("not found".to_string()))
        }

        fn getbestblockhash(&self) -> Result<String, RpcError> {
            Ok("test".to_string())
        }

        fn getblockhash(&self, _: u64) -> Result<String, RpcError> {
            Ok("test".to_string())
        }
    }

    #[test]
    fn test_server_creation() {
        let handler = TestHandler;
        let _server = RpcServer::new(handler, "127.0.0.1:0".to_string());
    }
}
