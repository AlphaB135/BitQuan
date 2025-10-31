//! HTTP server for handling JSON-RPC requests

use crate::{error_codes, methods, JsonRpcRequest, JsonRpcResponse};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

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

    /// Username accessor (primarily for tests).
    #[cfg(test)]
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    /// Password accessor (primarily for tests).
    #[cfg(test)]
    pub(crate) fn password(&self) -> &str {
        &self.password
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
        self.accept_loop(listener, None)
    }

    #[cfg(test)]
    pub(crate) fn serve_with_listener_and_shutdown(
        &self,
        listener: TcpListener,
        shutdown: Option<Receiver<()>>,
    ) -> std::io::Result<()> {
        self.accept_loop(listener, shutdown)
    }

    fn accept_loop(
        &self,
        listener: TcpListener,
        mut shutdown: Option<Receiver<()>>,
    ) -> std::io::Result<()> {
        let bound_addr = listener.local_addr()?;
        println!("RPC server listening on {}", bound_addr);

        if shutdown.is_none() {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => self.spawn_worker(stream),
                    Err(e) => eprintln!("Connection error: {}", e),
                }
            }
            return Ok(());
        }

        listener.set_nonblocking(true)?;

        loop {
            if let Some(rx) = shutdown.as_mut() {
                match rx.try_recv() {
                    Ok(()) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {}
                }
            }

            match listener.accept() {
                Ok((stream, _)) => self.spawn_worker(stream),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }

        Ok(())
    }

    fn spawn_worker(&self, stream: TcpStream) {
        let handler = Arc::clone(&self.handler);
        let auth = self.auth.clone();
        std::thread::spawn(move || {
            if let Err(e) = handle_connection(stream, handler.as_ref(), auth.as_ref()) {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

fn handle_connection<T: methods::RpcMethods>(
    mut stream: TcpStream,
    handler: &T,
    auth: Option<&RpcAuth>,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut buf_reader = BufReader::new(&stream);
    let http_request: Vec<_> = buf_reader
        .by_ref()
        .lines()
        .map_while(Result::ok)
        .take_while(|line| !line.is_empty())
        .collect();

    let request_line = http_request.first().map(|s| s.as_str()).unwrap_or("");

    if request_line.starts_with("GET /health") {
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Length: 2\r\n",
            "Content-Type: text/plain\r\n",
            "Connection: close\r\n",
            "\r\n",
            "ok"
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        let _ = stream.shutdown(Shutdown::Write);
        return Ok(());
    }

    // Read Content-Length header
    let content_length = http_request
        .iter()
        .find(|line| line.to_lowercase().starts_with("content-length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if content_length == 0 {
        let response = concat!(
            "HTTP/1.1 400 Bad Request\r\n",
            "Content-Length: 0\r\n",
            "Connection: close\r\n",
            "\r\n"
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        let _ = stream.shutdown(Shutdown::Write);
        return Ok(());
    }
    if content_length > MAX_REQUEST_SIZE {
        let response = concat!(
            "HTTP/1.1 413 Payload Too Large\r\n",
            "Content-Length: 0\r\n",
            "Connection: close\r\n",
            "\r\n"
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        let _ = stream.shutdown(Shutdown::Write);
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
    buf_reader.read_exact(&mut body)?;

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
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    stream.write_all(http_response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown(Shutdown::Write);
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
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown(Shutdown::Write);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::{BlockTemplate, BlockchainInfo, MiningInfo, RpcMethods, TxInfo};
    use crate::test_util::{basic_auth_header, init_test_tracing, spawn_test_server, wait_ready};
    use crate::RpcError;
    use anyhow::Result;
    use futures::future::try_join_all;
    use reqwest::StatusCode;
    use tokio::time::{timeout, Duration};

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_requires_no_auth_and_closes() -> Result<()> {
        init_test_tracing();
        let server = RpcServer::new(TestHandler, "127.0.0.1:0".to_string());
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let resp = timeout(
            Duration::from_secs(5),
            client.get(format!("{}/health", base_url)).send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.text().await?, "ok");

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_max_body_returns_413() -> Result<()> {
        init_test_tracing();
        let server = RpcServer::new(TestHandler, "127.0.0.1:0".to_string());
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let oversized = vec![b'a'; MAX_REQUEST_SIZE + 1];
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&base_url)
                .header("Content-Type", "application/json")
                .body(oversized)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_without_auth_is_401() -> Result<()> {
        init_test_tracing();
        let auth = RpcAuth::new("user", "pass");
        let server =
            RpcServer::with_auth(TestHandler, "127.0.0.1:0".to_string(), Some(auth.clone()));
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&base_url)
                .header("Content-Type", "application/json")
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_concurrency_smoke() -> Result<()> {
        init_test_tracing();
        let auth = RpcAuth::new("alice", "secret");
        let server =
            RpcServer::with_auth(TestHandler, "127.0.0.1:0".to_string(), Some(auth.clone()));
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let auth_header = basic_auth_header(auth.username(), auth.password());
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;

        let mut requests = Vec::with_capacity(10);
        for _ in 0..10 {
            let client = client.clone();
            let url = base_url.clone();
            let auth_header = auth_header.clone();
            let body = body.to_string();
            requests.push(async move {
                let resp = timeout(
                    Duration::from_secs(5),
                    client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .header("Authorization", &auth_header)
                        .body(body)
                        .send(),
                )
                .await??;
                anyhow::Result::<String>::Ok(resp.text().await?)
            });
        }

        let responses = try_join_all(requests).await?;
        for text in responses {
            assert!(text.contains("100"));
        }

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }
}
