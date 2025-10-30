//! HTTP server for handling JSON-RPC requests

use crate::{error_codes, methods, JsonRpcRequest, JsonRpcResponse};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

const MAX_REQUEST_SIZE: usize = 1_048_576; // 1 MiB

/// Basic authentication configuration for RPC server.
#[derive(Clone, Debug)]
pub struct RpcAuth {
    username: String,
    password: String,
}

impl RpcAuth {
    /// Creates a new auth configuration from username/password strings.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    fn matches(&self, provided: &str) -> bool {
        provided == format!("{}:{}", self.username, self.password)
    }
}

/// Simple HTTP JSON-RPC server
pub struct RpcServer<T> {
    handler: Arc<T>,
    addr: String,
    auth: Option<RpcAuth>,
}

impl<T: methods::RpcMethods + Send + Sync + 'static> RpcServer<T> {
    /// Create new RPC server
    pub fn new(handler: T, addr: String) -> Self {
        Self::with_auth(handler, addr, None)
    }

    /// Create RPC server with optional authentication.
    pub fn with_auth(handler: T, addr: String, auth: Option<RpcAuth>) -> Self {
        Self {
            handler: Arc::new(handler),
            addr,
            auth,
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
                    let auth = self.auth.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, handler.as_ref(), auth.as_ref()) {
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
    auth: Option<&RpcAuth>,
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
    if content_length > MAX_REQUEST_SIZE {
        let response = "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    if let Some(auth_cfg) = auth {
        if !is_authorized(&http_request, auth_cfg) {
            send_unauthorized(&mut stream)?;
            return Ok(());
        }
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

fn is_authorized(request_headers: &[String], auth: &RpcAuth) -> bool {
    let header = request_headers
        .iter()
        .find(|line| line.to_ascii_lowercase().starts_with("authorization:"));

    let Some(header) = header else {
        return false;
    };

    let mut parts = header.splitn(2, ':');
    parts.next(); // skip "Authorization"
    let value = parts.next().map(str::trim).unwrap_or_default();

    if !value.to_ascii_lowercase().starts_with("basic ") {
        return false;
    }

    let encoded = value[5..].trim();
    use base64::{engine::general_purpose::STANDARD, Engine};

    let decoded = match STANDARD.decode(encoded) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let credential = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return false,
    };

    auth.matches(&credential)
}

fn send_unauthorized(stream: &mut TcpStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 401 Unauthorized\r\n",
        "WWW-Authenticate: Basic realm=\"BitQuan RPC\"\r\n",
        "Content-Length: 0\r\n",
        "Content-Type: text/plain\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::{BlockTemplate, BlockchainInfo, MiningInfo, RpcMethods, TxInfo};
    use crate::RpcError;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

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

        fn getwork(&self) -> Result<crate::methods::WorkData, RpcError> {
            Ok(crate::methods::WorkData {
                data: "00000000".to_string(),
                target: "00000000ffff".to_string(),
            })
        }

        fn submitwork(&self, _: String) -> Result<bool, RpcError> {
            Ok(true)
        }
    }

    #[test]
    fn test_server_creation() {
        let handler = TestHandler;
        let _server = RpcServer::new(handler, "127.0.0.1:0".to_string());
    }

    #[test]
    fn rejects_request_exceeding_max_body() {
        let handler = TestHandler;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                handle_connection(stream, &handler, None).unwrap();
            }
        });

        let mut stream = TcpStream::connect(addr).unwrap();
        let request = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            super::MAX_REQUEST_SIZE + 1
        );
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with("HTTP/1.1 413"),
            "unexpected response: {}",
            response
        );

        server_thread.join().unwrap();
    }

    #[test]
    fn rejects_request_without_auth() {
        let handler = TestHandler;
        let auth = RpcAuth::new("user", "pass");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                handle_connection(stream, &handler, Some(&auth)).unwrap();
            }
        });

        let mut stream = TcpStream::connect(addr).unwrap();
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let request = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut buf = [0u8; 512];
        let n = stream.read(&mut buf).unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(
            response.starts_with("HTTP/1.1 401"),
            "unexpected response: {}",
            response
        );

        server_thread.join().unwrap();
    }

    #[test]
    fn accepts_request_with_valid_auth() {
        let handler = TestHandler;
        let auth = RpcAuth::new("alice", "secret");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                handle_connection(stream, &handler, Some(&auth)).unwrap();
            }
        });

        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let credential = STANDARD.encode(b"alice:secret");

        let mut stream = TcpStream::connect(addr).unwrap();
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let request = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nAuthorization: Basic {}\r\nContent-Length: {}\r\n\r\n{}",
            credential,
            body.len(),
            body
        );
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut buf = [0u8; 512];
        let n = stream.read(&mut buf).unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "unexpected response: {}",
            response
        );

        server_thread.join().unwrap();
    }
}
