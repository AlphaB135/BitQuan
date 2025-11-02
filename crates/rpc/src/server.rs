//! HTTP server for handling JSON-RPC requests

use crate::{error_codes, methods, tls::TlsConfig, JsonRpcRequest, JsonRpcResponse, RpcConfig};
use http::StatusCode;
use once_cell::sync::Lazy;
use rustls::{ServerConnection, StreamOwned};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Shutdown, TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Authentication for RPC server (JWT only)
pub type AuthMethod = Arc<crate::jwt::JwtAuth>;

/// Simple HTTP JSON-RPC server
pub struct RpcServer<T> {
    handler: Arc<T>,
    addr: String,
    auth: Option<AuthMethod>,
    config: RpcConfig,
    limiter: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    auth_backoff: Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
    tls: Option<Arc<TlsConfig>>,
    force_tls: bool,
}

impl<T: methods::RpcMethods + Send + Sync + 'static> RpcServer<T> {
    /// Create new RPC server with JWT authentication
    pub fn new(handler: T, addr: String, jwt_auth: crate::jwt::JwtAuth, config: RpcConfig) -> Self {
        let require_tls = config.require_tls;
        Self {
            handler: Arc::new(handler),
            addr,
            auth: Some(Arc::new(jwt_auth)),
            config,
            limiter: Arc::new(Mutex::new(HashMap::new())),
            auth_backoff: Arc::new(Mutex::new(HashMap::new())),
            tls: None,
            force_tls: require_tls,
        }
    }

    /// Create RPC server without authentication (for testing only)
    #[cfg(test)]
    pub fn without_auth(handler: T, addr: String, config: RpcConfig) -> Self {
        let require_tls = config.require_tls;
        Self {
            handler: Arc::new(handler),
            addr,
            auth: None,
            config,
            limiter: Arc::new(Mutex::new(HashMap::new())),
            auth_backoff: Arc::new(Mutex::new(HashMap::new())),
            tls: None,
            force_tls: require_tls,
        }
    }

    /// Attach a TLS configuration to the server (does not automatically enforce TLS).
    pub fn with_tls_config(mut self, tls: TlsConfig) -> Self {
        self.tls = Some(Arc::new(tls));
        self
    }

    /// Require all connections to use TLS (typically for mainnet deployments).
    pub fn require_tls(mut self, required: bool) -> Self {
        self.force_tls = required;
        self
    }

    /// Start serving requests (blocking)
    pub fn serve(&self) -> std::io::Result<()> {
        if self.force_tls && self.tls.is_none() {
            return Err(std::io::Error::other(
                "TLS is required but no TLS configuration was provided",
            ));
        }
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
        println!(
            "RPC config: max_body_bytes={} rl_burst={} rl_refill_per_sec={} conn_cooldown_ms={} require_tls={}",
            self.config.max_body_bytes,
            self.config.rl_burst,
            self.config.rl_refill_per_sec,
            self.config.conn_cooldown_ms,
            self.force_tls
        );
        println!("RPC health endpoint: GET /health (no auth)");

        if shutdown.is_none() {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let peer = stream.peer_addr().ok().map(|addr| addr.ip());
                        self.spawn_worker(stream, peer);
                    }
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
                Ok((stream, peer)) => self.spawn_worker(stream, Some(peer.ip())),
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

    fn spawn_worker(&self, stream: TcpStream, peer: Option<IpAddr>) {
        let handler = Arc::clone(&self.handler);
        let auth = self.auth.clone();
        let config = self.config.clone();
        let limiter = Arc::clone(&self.limiter);
        let auth_backoff = Arc::clone(&self.auth_backoff);
        let tls = self.tls.clone();
        let force_tls = self.force_tls;
        std::thread::spawn(move || {
            let peer_ip = peer
                .or_else(|| stream.peer_addr().ok().map(|addr| addr.ip()))
                .unwrap_or(IpAddr::from([127, 0, 0, 1]));
            let options = ConnectionOptions {
                config: &config,
                limiter: &limiter,
                auth_backoff: &auth_backoff,
                tls: tls.as_ref(),
                force_tls,
            };
            if let Err(e) =
                handle_connection(stream, peer_ip, handler.as_ref(), auth.as_ref(), options)
            {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

struct ConnectionOptions<'a> {
    config: &'a RpcConfig,
    limiter: &'a Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    auth_backoff: &'a Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
    tls: Option<&'a Arc<TlsConfig>>,
    force_tls: bool,
}

/// Abstraction over plain TCP and TLS-encrypted RPC streams.
enum RpcStream {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ServerConnection, TcpStream>>),
}

impl RpcStream {
    fn set_read_timeout(&self, duration: Option<Duration>) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.set_read_timeout(duration),
            RpcStream::Tls(stream) => stream.sock.set_read_timeout(duration),
        }
    }

    fn set_write_timeout(&self, duration: Option<Duration>) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.set_write_timeout(duration),
            RpcStream::Tls(stream) => stream.sock.set_write_timeout(duration),
        }
    }

    fn shutdown(&mut self) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.shutdown(Shutdown::Write),
            RpcStream::Tls(stream) => {
                stream.conn.send_close_notify();
                stream.sock.shutdown(Shutdown::Write)
            }
        }
    }
}

impl Read for RpcStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            RpcStream::Plain(stream) => stream.read(buf),
            RpcStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for RpcStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            RpcStream::Plain(stream) => stream.write(buf),
            RpcStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.flush(),
            RpcStream::Tls(stream) => stream.flush(),
        }
    }
}

fn upgrade_to_tls(stream: TcpStream, tls_config: &TlsConfig) -> std::io::Result<RpcStream> {
    let server_config = tls_config.server_config();
    let connection = ServerConnection::new(server_config).map_err(std::io::Error::other)?;
    let mut tls_stream = StreamOwned::new(connection, stream);
    while tls_stream.conn.is_handshaking() {
        tls_stream.conn.complete_io(&mut tls_stream.sock)?;
    }
    Ok(RpcStream::Tls(Box::new(tls_stream)))
}

fn handle_connection<T: methods::RpcMethods>(
    stream: TcpStream,
    peer_ip: IpAddr,
    handler: &T,
    auth: Option<&AuthMethod>,
    options: ConnectionOptions<'_>,
) -> std::io::Result<()> {
    let start = Instant::now();
    let config = options.config;
    let limiter = options.limiter;
    let auth_backoff = options.auth_backoff;
    let tls = options.tls;
    let force_tls = options.force_tls;
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_millis(config.header_read_timeout_ms)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    // Check TLS enforcement with self-signed validation
    let mut channel = match (tls, force_tls) {
        (Some(tls_config), _) => {
            // Validate self-signed cert not allowed on mainnet
            if tls_config.is_self_signed() && !config.allow_self_signed {
                return Err(std::io::Error::other(
                    "Self-signed certificates not allowed in production",
                ));
            }

            // Warn if certificate expires soon
            if tls_config.expires_soon(30) {
                eprintln!("⚠️  WARNING: TLS certificate expires in less than 30 days!");
            }

            upgrade_to_tls(stream, tls_config.as_ref())?
        }
        (None, true) => {
            // TLS required but not provided - send upgrade required response
            return send_upgrade_required(stream, peer_ip, start);
        }
        (None, false) => RpcStream::Plain(stream),
    };

    channel.set_read_timeout(Some(Duration::from_millis(config.header_read_timeout_ms)))?;
    channel.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut buf_reader = BufReader::new(&mut channel);
    let mut total_header_bytes: usize = 0;
    let mut request_line = String::new();
    let mut headers: Vec<String> = Vec::new();

    loop {
        let mut line = String::new();
        let bytes = buf_reader.read_line(&mut line)?;
        if bytes == 0 {
            let stream = buf_reader.get_mut();
            send_bad_request(stream)?;
            record_response(ResponseContext {
                method: "INVALID",
                path: "invalid",
                status: StatusCode::BAD_REQUEST,
                content_length: 0,
                start,
                client_ip: peer_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            return Ok(());
        }

        total_header_bytes += bytes;
        if total_header_bytes > config.max_header_bytes {
            let stream = buf_reader.get_mut();
            send_header_too_large(stream)?;
            record_response(ResponseContext {
                method: "INVALID",
                path: "header",
                status: StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                content_length: 0,
                start,
                client_ip: peer_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: true,
            });
            apply_cooldown(config);
            return Ok(());
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if request_line.is_empty() {
            if trimmed.is_empty() {
                let stream = buf_reader.get_mut();
                send_bad_request(stream)?;
                record_response(ResponseContext {
                    method: "INVALID",
                    path: "invalid",
                    status: StatusCode::BAD_REQUEST,
                    content_length: 0,
                    start,
                    client_ip: peer_ip,
                    rate_limited: false,
                    body_limit: false,
                    auth_fail: false,
                    header_limit: false,
                });
                apply_cooldown(config);
                return Ok(());
            }
            request_line = trimmed.to_string();
            continue;
        }

        if trimmed.is_empty() {
            break;
        }

        headers.push(trimmed.to_string());
    }

    (**buf_reader.get_mut()).set_read_timeout(Some(Duration::from_secs(5)))?;

    if request_line.is_empty() {
        let stream = buf_reader.get_mut();
        send_bad_request(stream)?;
        record_response(ResponseContext {
            method: "INVALID",
            path: "invalid",
            status: StatusCode::BAD_REQUEST,
            content_length: 0,
            start,
            client_ip: peer_ip,
            rate_limited: false,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        apply_cooldown(config);
        return Ok(());
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let path_owned = path.to_string();

    let client_ip = resolve_client_ip(peer_ip, &headers, config);

    let content_length = headers
        .iter()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    let host_header = headers.iter().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("host") {
            Some(value.trim().to_string())
        } else {
            None
        }
    });

    if !validate_host_header(host_header.as_deref(), config) {
        let stream = buf_reader.get_mut();
        send_forbidden(stream)?;
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::FORBIDDEN,
            content_length,
            start,
            client_ip,
            rate_limited: false,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        apply_cooldown(config);
        return Ok(());
    }

    let origin_header = headers.iter().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("origin") {
            Some(value.trim().to_string())
        } else {
            None
        }
    });

    if !validate_origin_header(origin_header.as_deref(), config) {
        let stream = buf_reader.get_mut();
        send_forbidden(stream)?;
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::FORBIDDEN,
            content_length,
            start,
            client_ip,
            rate_limited: false,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        apply_cooldown(config);
        return Ok(());
    }

    let is_health = method.eq_ignore_ascii_case("GET") && path == "/health";
    let is_metrics = method.eq_ignore_ascii_case("GET") && path == "/metrics";
    let is_login = method.eq_ignore_ascii_case("POST") && path == "/auth/login";
    let is_refresh = method.eq_ignore_ascii_case("POST") && path == "/auth/refresh";

    if is_metrics {
        if !client_ip.is_loopback() {
            let stream = buf_reader.get_mut();
            send_forbidden(stream)?;
            record_response(ResponseContext {
                method,
                path: &path_owned,
                status: StatusCode::FORBIDDEN,
                content_length: 0,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            return Ok(());
        }

        let body = render_metrics();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        {
            let stream = buf_reader.get_mut();
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();
        }
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::OK,
            content_length: 0,
            start,
            client_ip,
            rate_limited: false,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        return Ok(());
    }

    if is_health {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nok";
        {
            let stream = buf_reader.get_mut();
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();
        }
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::OK,
            content_length: 0,
            start,
            client_ip,
            rate_limited: false,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        return Ok(());
    }

    // JWT Login endpoint - no auth required
    if is_login {
        if let Some(jwt_auth) = auth {
            return handle_login_endpoint(
                &mut buf_reader,
                jwt_auth,
                RequestContext {
                    method,
                    path: &path_owned,
                    content_length,
                    config,
                    start,
                    client_ip,
                },
            );
        } else {
            // JWT not configured
            let stream = buf_reader.get_mut();
            let error_body = r#"{"error":"JWT not configured"}"#;
            let response = format!(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_body.len(),
                error_body
            );
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();
            return Ok(());
        }
    }

    // JWT Refresh endpoint - no auth required
    if is_refresh {
        if let Some(jwt_auth) = auth {
            return handle_refresh_endpoint(
                &mut buf_reader,
                jwt_auth,
                RequestContext {
                    method,
                    path: &path_owned,
                    content_length,
                    config,
                    start,
                    client_ip,
                },
            );
        } else {
            // JWT not configured
            let stream = buf_reader.get_mut();
            let error_body = r#"{"error":"JWT not configured"}"#;
            let response = format!(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_body.len(),
                error_body
            );
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();
            return Ok(());
        }
    }

    if content_length == 0 {
        let stream = buf_reader.get_mut();
        send_bad_request(stream)?;
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::BAD_REQUEST,
            content_length,
            start,
            client_ip,
            rate_limited: false,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        apply_cooldown(config);
        return Ok(());
    }

    if content_length > config.max_body_bytes {
        let stream = buf_reader.get_mut();
        send_payload_too_large(stream)?;
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::PAYLOAD_TOO_LARGE,
            content_length,
            start,
            client_ip,
            rate_limited: false,
            body_limit: true,
            auth_fail: false,
            header_limit: false,
        });
        apply_cooldown(config);
        return Ok(());
    }

    if !take_token(client_ip, limiter, config) {
        let stream = buf_reader.get_mut();
        send_too_many_requests(stream)?;
        record_response(ResponseContext {
            method,
            path: &path_owned,
            status: StatusCode::TOO_MANY_REQUESTS,
            content_length,
            start,
            client_ip,
            rate_limited: true,
            body_limit: false,
            auth_fail: false,
            header_limit: false,
        });
        apply_cooldown(config);
        return Ok(());
    }

    if let Some(auth_cfg) = auth {
        if !is_authorized_new(&headers, auth_cfg.as_ref()) {
            let stream = buf_reader.get_mut();
            send_unauthorized(stream)?;
            record_response(ResponseContext {
                method,
                path: &path_owned,
                status: StatusCode::UNAUTHORIZED,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: true,
                header_limit: false,
            });
            apply_auth_backoff(client_ip, auth_backoff);
            apply_cooldown(config);
            return Ok(());
        } else {
            reset_auth_backoff(client_ip, auth_backoff);
        }
    }

    (**buf_reader.get_mut()).set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut body = vec![0u8; content_length];
    buf_reader.read_exact(&mut body)?;

    // Release the buffered borrow so we can reuse the channel for writing.
    let stream = buf_reader.into_inner();

    let json_request = match serde_json::from_slice::<JsonRpcRequest>(&body) {
        Ok(req) => req,
        Err(e) => {
            let error_response = JsonRpcResponse::error(
                serde_json::Value::Null,
                error_codes::PARSE_ERROR,
                format!("Parse error: {e}"),
            );
            respond_json(stream, &error_response, config)?;
            record_response(ResponseContext {
                method,
                path: &path_owned,
                status: StatusCode::OK,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            return Ok(());
        }
    };

    let json_response = if json_request.jsonrpc != "2.0" {
        JsonRpcResponse::error(
            json_request.id,
            error_codes::INVALID_REQUEST,
            "Invalid JSON-RPC version".to_string(),
        )
    } else {
        methods::dispatch_call(
            handler,
            &json_request.method,
            json_request.params,
            json_request.id,
        )
    };

    respond_json(stream, &json_response, config)?;
    record_response(ResponseContext {
        method,
        path: &path_owned,
        status: StatusCode::OK,
        content_length,
        start,
        client_ip,
        rate_limited: false,
        body_limit: false,
        auth_fail: false,
        header_limit: false,
    });
    apply_cooldown(config);
    Ok(())
}

/// Check JWT authorization (Bearer token)
fn is_authorized_new(request_headers: &[String], jwt_auth: &crate::jwt::JwtAuth) -> bool {
    let header = request_headers
        .iter()
        .find(|line| line.to_ascii_lowercase().starts_with("authorization:"));

    let Some(header) = header else {
        return false;
    };

    let mut parts = header.splitn(2, ':');
    parts.next(); // skip "Authorization"
    let value = parts.next().map(str::trim).unwrap_or_default();

    if !value.to_ascii_lowercase().starts_with("bearer ") {
        return false;
    }

    let token = value[7..].trim();
    match jwt_auth.verify_token(token) {
        Ok(_claims) => true,
        Err(e) => {
            eprintln!("JWT verification failed: {}", e);
            false
        }
    }
}

fn respond_json(
    stream: &mut RpcStream,
    response: &JsonRpcResponse,
    config: &RpcConfig,
) -> std::io::Result<()> {
    let response_body = serde_json::to_string(response).unwrap();
    let security_headers = build_security_headers(config);
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
        response_body.len(),
        security_headers,
        response_body
    );
    stream.write_all(http_response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_bad_request(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 400 Bad Request\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_payload_too_large(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 413 Payload Too Large\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_header_too_large(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 431 Request Header Fields Too Large\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_forbidden(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 403 Forbidden\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_unauthorized(stream: &mut RpcStream) -> std::io::Result<()> {
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
    let _ = stream.shutdown();
    Ok(())
}

fn send_too_many_requests(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 429 Too Many Requests\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn apply_cooldown(config: &RpcConfig) {
    if config.conn_cooldown_ms > 0 {
        std::thread::sleep(Duration::from_millis(config.conn_cooldown_ms));
    }
}

/// Handle JWT login endpoint
struct RequestContext<'a> {
    method: &'a str,
    path: &'a str,
    content_length: usize,
    config: &'a RpcConfig,
    start: Instant,
    client_ip: IpAddr,
}

fn handle_login_endpoint(
    buf_reader: &mut BufReader<&mut RpcStream>,
    jwt_auth: &Arc<crate::jwt::JwtAuth>,
    ctx: RequestContext<'_>,
) -> std::io::Result<()> {
    let RequestContext {
        method,
        path,
        content_length,
        config,
        start,
        client_ip,
    } = ctx;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct LoginRequest {
        username: String,
        password: String,
    }

    #[derive(Serialize)]
    struct LoginResponse {
        access_token: String,
        token_type: String,
        expires_in: u64,
    }

    #[derive(Serialize)]
    struct ErrorResponse {
        error: String,
        message: String,
    }

    // Read body
    let mut body = vec![0u8; content_length];
    buf_reader.read_exact(&mut body)?;

    // Parse login request
    let login_req: LoginRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            let stream = buf_reader.get_mut();
            let error = ErrorResponse {
                error: "invalid_request".to_string(),
                message: format!("Invalid JSON: {}", e),
            };
            let error_json = serde_json::to_string(&error).unwrap();
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_json.len(),
                error_json
            );
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();

            record_response(ResponseContext {
                method,
                path,
                status: StatusCode::BAD_REQUEST,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            return Ok(());
        }
    };

    // Attempt login
    match jwt_auth.login(&login_req.username, &login_req.password) {
        Ok(token) => {
            let response_data = LoginResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
            };
            let response_json = serde_json::to_string(&response_data).unwrap();
            let security_headers = build_security_headers(config);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
                response_json.len(),
                security_headers,
                response_json
            );

            let stream = buf_reader.get_mut();
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();

            record_response(ResponseContext {
                method,
                path,
                status: StatusCode::OK,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            Ok(())
        }
        Err(e) => {
            let stream = buf_reader.get_mut();
            let error = ErrorResponse {
                error: "invalid_credentials".to_string(),
                message: e,
            };
            let error_json = serde_json::to_string(&error).unwrap();
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_json.len(),
                error_json
            );
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();

            record_response(ResponseContext {
                method,
                path,
                status: StatusCode::UNAUTHORIZED,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: true,
                header_limit: false,
            });
            apply_cooldown(config);
            Ok(())
        }
    }
}

/// Handle JWT refresh endpoint
fn handle_refresh_endpoint(
    buf_reader: &mut BufReader<&mut RpcStream>,
    jwt_auth: &Arc<crate::jwt::JwtAuth>,
    ctx: RequestContext<'_>,
) -> std::io::Result<()> {
    let RequestContext {
        method,
        path,
        content_length,
        config,
        start,
        client_ip,
    } = ctx;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct RefreshRequest {
        refresh_token: String,
    }

    #[derive(Serialize)]
    struct RefreshResponse {
        access_token: String,
        token_type: String,
        expires_in: u64,
    }

    #[derive(Serialize)]
    struct ErrorResponse {
        error: String,
        message: String,
    }

    // Read body
    let mut body = vec![0u8; content_length];
    buf_reader.read_exact(&mut body)?;

    // Parse refresh request
    let refresh_req: RefreshRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            let stream = buf_reader.get_mut();
            let error = ErrorResponse {
                error: "invalid_request".to_string(),
                message: format!("Invalid JSON: {}", e),
            };
            let error_json = serde_json::to_string(&error).unwrap();
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_json.len(),
                error_json
            );
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();

            record_response(ResponseContext {
                method,
                path,
                status: StatusCode::BAD_REQUEST,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            return Ok(());
        }
    };

    // Attempt token refresh
    match jwt_auth.refresh_token(&refresh_req.refresh_token) {
        Ok(new_token) => {
            let response_data = RefreshResponse {
                access_token: new_token,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
            };
            let response_json = serde_json::to_string(&response_data).unwrap();
            let security_headers = build_security_headers(config);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
                response_json.len(),
                security_headers,
                response_json
            );

            let stream = buf_reader.get_mut();
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();

            record_response(ResponseContext {
                method,
                path,
                status: StatusCode::OK,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: false,
                header_limit: false,
            });
            apply_cooldown(config);
            Ok(())
        }
        Err(e) => {
            let stream = buf_reader.get_mut();
            let error = ErrorResponse {
                error: "invalid_token".to_string(),
                message: e,
            };
            let error_json = serde_json::to_string(&error).unwrap();
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error_json.len(),
                error_json
            );
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();

            record_response(ResponseContext {
                method,
                path,
                status: StatusCode::UNAUTHORIZED,
                content_length,
                start,
                client_ip,
                rate_limited: false,
                body_limit: false,
                auth_fail: true,
                header_limit: false,
            });
            apply_cooldown(config);
            Ok(())
        }
    }
}

/// Send HTTP 426 Upgrade Required response for non-TLS connections when TLS is mandatory
fn send_upgrade_required(
    mut stream: TcpStream,
    peer_ip: IpAddr,
    start: Instant,
) -> std::io::Result<()> {
    let body = r#"{"error":"TLS Required","message":"This server requires HTTPS. Please upgrade your connection to HTTPS."}"#;
    let response = format!(
        concat!(
            "HTTP/1.1 426 Upgrade Required\r\n",
            "Upgrade: TLS/1.3, HTTP/1.1\r\n",
            "Connection: Upgrade\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: {}\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    stream.shutdown(Shutdown::Write)?;

    record_response(ResponseContext {
        method: "UPGRADE",
        path: "required",
        status: StatusCode::UPGRADE_REQUIRED,
        content_length: body.len(),
        start,
        client_ip: peer_ip,
        rate_limited: false,
        body_limit: false,
        auth_fail: true, // TLS required acts like auth failure
        header_limit: false,
    });

    Ok(())
}

/// Add security headers to HTTP response
fn build_security_headers(config: &RpcConfig) -> String {
    let mut headers = String::new();

    // HSTS (HTTP Strict Transport Security)
    if config.enable_hsts {
        headers.push_str(&format!(
            "Strict-Transport-Security: max-age={}{}preload\r\n",
            config.hsts_max_age,
            if config.hsts_include_subdomains {
                "; includeSubDomains; "
            } else {
                "; "
            }
        ));
    }

    // Security headers (always enabled)
    headers.push_str("X-Content-Type-Options: nosniff\r\n");
    headers.push_str("X-Frame-Options: DENY\r\n");
    headers.push_str("X-XSS-Protection: 1; mode=block\r\n");
    headers.push_str("Referrer-Policy: no-referrer\r\n");
    headers.push_str("Content-Security-Policy: default-src 'none'\r\n");

    headers
}

fn take_token(
    ip: IpAddr,
    limiter: &Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    config: &RpcConfig,
) -> bool {
    let mut map = limiter.lock().unwrap();
    let bucket = map.entry(ip).or_insert_with(|| TokenBucket {
        tokens: config.rl_burst as f64,
        last: Instant::now(),
    });

    let now = Instant::now();
    let elapsed = now.duration_since(bucket.last).as_secs_f64();
    bucket.last = now;

    if config.rl_refill_per_sec > 0 {
        bucket.tokens =
            (bucket.tokens + elapsed * config.rl_refill_per_sec as f64).min(config.rl_burst as f64);
    }

    if bucket.tokens >= 1.0 {
        bucket.tokens -= 1.0;
        true
    } else {
        false
    }
}

struct TokenBucket {
    tokens: f64,
    last: Instant,
}

struct BackoffState {
    fails: u32,
    last: Instant,
}

fn resolve_client_ip(peer_ip: IpAddr, headers: &[String], config: &RpcConfig) -> IpAddr {
    if !config.trust_proxy {
        return peer_ip;
    }

    if !config
        .trusted_proxies
        .iter()
        .any(|cidr| cidr.contains(peer_ip))
    {
        return peer_ip;
    }

    for line in headers {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("x-forwarded-for") {
                if let Some(first) = value.split(',').next() {
                    if let Ok(addr) = IpAddr::from_str(first.trim()) {
                        return addr;
                    }
                }
            }
        }
    }

    peer_ip
}

fn apply_auth_backoff(ip: IpAddr, backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>) {
    let mut map = backoff.lock().unwrap();
    let state = map.entry(ip).or_insert(BackoffState {
        fails: 0,
        last: Instant::now(),
    });
    state.fails = state.fails.saturating_add(1);
    state.last = Instant::now();
    let exponent = (state.fails - 1).min(4);
    let delay_ms = 100u64.saturating_mul(1u64 << exponent);
    drop(map);
    std::thread::sleep(Duration::from_millis(delay_ms));
}

fn reset_auth_backoff(ip: IpAddr, backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>) {
    let mut map = backoff.lock().unwrap();
    map.remove(&ip);
}

struct RpcMetrics {
    requests_total: AtomicU64,
    status_200: AtomicU64,
    status_400: AtomicU64,
    status_401: AtomicU64,
    status_403: AtomicU64,
    status_413: AtomicU64,
    status_429: AtomicU64,
    status_431: AtomicU64,
    status_other: AtomicU64,
    rl_drops_total: AtomicU64,
    auth_fail_total: AtomicU64,
    body_too_large_total: AtomicU64,
    header_limit_total: AtomicU64,
    latency_sum_ms: AtomicU64,
    latency_count: AtomicU64,
    latency_bucket_50: AtomicU64,
    latency_bucket_100: AtomicU64,
    latency_bucket_250: AtomicU64,
    latency_bucket_500: AtomicU64,
    latency_bucket_1000: AtomicU64,
    latency_bucket_inf: AtomicU64,
}

impl Default for RpcMetrics {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            status_200: AtomicU64::new(0),
            status_400: AtomicU64::new(0),
            status_401: AtomicU64::new(0),
            status_403: AtomicU64::new(0),
            status_413: AtomicU64::new(0),
            status_429: AtomicU64::new(0),
            status_431: AtomicU64::new(0),
            status_other: AtomicU64::new(0),
            rl_drops_total: AtomicU64::new(0),
            auth_fail_total: AtomicU64::new(0),
            body_too_large_total: AtomicU64::new(0),
            header_limit_total: AtomicU64::new(0),
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            latency_bucket_50: AtomicU64::new(0),
            latency_bucket_100: AtomicU64::new(0),
            latency_bucket_250: AtomicU64::new(0),
            latency_bucket_500: AtomicU64::new(0),
            latency_bucket_1000: AtomicU64::new(0),
            latency_bucket_inf: AtomicU64::new(0),
        }
    }
}

impl RpcMetrics {
    fn record(
        &self,
        status: StatusCode,
        latency_ms: u64,
        rate_limited: bool,
        body_limit: bool,
        auth_fail: bool,
        header_limit: bool,
    ) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        match status {
            StatusCode::OK => self.status_200.fetch_add(1, Ordering::Relaxed),
            StatusCode::BAD_REQUEST => self.status_400.fetch_add(1, Ordering::Relaxed),
            StatusCode::UNAUTHORIZED => self.status_401.fetch_add(1, Ordering::Relaxed),
            StatusCode::FORBIDDEN => self.status_403.fetch_add(1, Ordering::Relaxed),
            StatusCode::PAYLOAD_TOO_LARGE => self.status_413.fetch_add(1, Ordering::Relaxed),
            StatusCode::TOO_MANY_REQUESTS => self.status_429.fetch_add(1, Ordering::Relaxed),
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE => {
                self.status_431.fetch_add(1, Ordering::Relaxed)
            }
            _ => self.status_other.fetch_add(1, Ordering::Relaxed),
        };

        if rate_limited {
            self.rl_drops_total.fetch_add(1, Ordering::Relaxed);
        }
        if body_limit {
            self.body_too_large_total.fetch_add(1, Ordering::Relaxed);
        }
        if auth_fail {
            self.auth_fail_total.fetch_add(1, Ordering::Relaxed);
        }
        if header_limit {
            self.header_limit_total.fetch_add(1, Ordering::Relaxed);
        }

        self.latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);

        let bucket = if latency_ms <= 50 {
            &self.latency_bucket_50
        } else if latency_ms <= 100 {
            &self.latency_bucket_100
        } else if latency_ms <= 250 {
            &self.latency_bucket_250
        } else if latency_ms <= 500 {
            &self.latency_bucket_500
        } else if latency_ms <= 1000 {
            &self.latency_bucket_1000
        } else {
            &self.latency_bucket_inf
        };
        bucket.fetch_add(1, Ordering::Relaxed);
    }

    fn render(&self) -> String {
        let total = self.requests_total.load(Ordering::Relaxed);
        let status_line = |code: &str, value| {
            format!("rpc_requests_status_total{{code=\"{}\"}} {}\n", code, value)
        };
        let body = format!(
            "rpc_requests_total {}\n{}{}{}{}{}{}{}{}rpc_rl_drops_total {}\nrpc_body_413_total {}\nrpc_header_limit_total {}\nrpc_auth_401_total {}\nrpc_latency_ms_sum {}\nrpc_latency_ms_count {}\nrpc_latency_ms_bucket{{le=\"50\"}} {}\nrpc_latency_ms_bucket{{le=\"100\"}} {}\nrpc_latency_ms_bucket{{le=\"250\"}} {}\nrpc_latency_ms_bucket{{le=\"500\"}} {}\nrpc_latency_ms_bucket{{le=\"1000\"}} {}\nrpc_latency_ms_bucket{{le=\"+Inf\"}} {}\n",
            total,
            status_line("200", self.status_200.load(Ordering::Relaxed)),
            status_line("400", self.status_400.load(Ordering::Relaxed)),
            status_line("401", self.status_401.load(Ordering::Relaxed)),
            status_line("403", self.status_403.load(Ordering::Relaxed)),
            status_line("413", self.status_413.load(Ordering::Relaxed)),
            status_line("429", self.status_429.load(Ordering::Relaxed)),
            status_line("431", self.status_431.load(Ordering::Relaxed)),
            status_line("other", self.status_other.load(Ordering::Relaxed)),
            self.rl_drops_total.load(Ordering::Relaxed),
            self.body_too_large_total.load(Ordering::Relaxed),
            self.header_limit_total.load(Ordering::Relaxed),
            self.auth_fail_total.load(Ordering::Relaxed),
            self.latency_sum_ms.load(Ordering::Relaxed),
            self.latency_count.load(Ordering::Relaxed),
            self.latency_bucket_50.load(Ordering::Relaxed),
            self.latency_bucket_100.load(Ordering::Relaxed),
            self.latency_bucket_250.load(Ordering::Relaxed),
            self.latency_bucket_500.load(Ordering::Relaxed),
            self.latency_bucket_1000.load(Ordering::Relaxed),
            self.latency_bucket_inf.load(Ordering::Relaxed),
        );
        body
    }
}

static METRICS: Lazy<RpcMetrics> = Lazy::new(RpcMetrics::default);

fn render_metrics() -> String {
    METRICS.render()
}

struct ResponseContext<'a> {
    method: &'a str,
    path: &'a str,
    status: StatusCode,
    content_length: usize,
    start: Instant,
    client_ip: IpAddr,
    rate_limited: bool,
    body_limit: bool,
    auth_fail: bool,
    header_limit: bool,
}

fn record_response(ctx: ResponseContext<'_>) {
    let latency_ms = ctx.start.elapsed().as_millis() as u64;
    METRICS.record(
        ctx.status,
        latency_ms,
        ctx.rate_limited,
        ctx.body_limit,
        ctx.auth_fail,
        ctx.header_limit,
    );

    if ctx.rate_limited || ctx.body_limit || ctx.auth_fail || ctx.header_limit {
        let reason = if ctx.rate_limited {
            "rate_limit"
        } else if ctx.body_limit {
            "body_limit"
        } else if ctx.header_limit {
            "header_limit"
        } else {
            "auth_failure"
        };

        let log = json!({
            "event": "rpc_guard",
            "reason": reason,
            "status": ctx.status.as_u16(),
            "method": ctx.method,
            "route": ctx.path,
            "ip": ctx.client_ip.to_string(),
            "bytes": ctx.content_length,
            "latency_ms": latency_ms,
        });
        println!("{}", log);
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]
    use super::*;
    use crate::methods::{BlockTemplate, BlockchainInfo, MiningInfo, RpcMethods, TxInfo};
    use crate::test_util::{init_test_tracing, spawn_test_server, wait_ready};
    use crate::{RpcConfig, RpcError};
    use anyhow::Result;
    use futures::future::try_join_all;
    use reqwest::StatusCode;
    use tokio::time::{sleep, timeout, Duration};

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

    fn base_config() -> RpcConfig {
        RpcConfig {
            conn_cooldown_ms: 0,
            trusted_proxies: Vec::new(),
            ..RpcConfig::default()
        }
    }

    #[test]
    fn test_server_creation() {
        let handler = TestHandler;
        let jwt_auth = crate::jwt::JwtAuth::new("test-secret-key");
        let _server = RpcServer::new(
            handler,
            "127.0.0.1:0".to_string(),
            jwt_auth,
            RpcConfig::default(),
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_not_rate_limited_under_load() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 1;
        config.rl_refill_per_sec = 0;
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let health_url = format!("{}/health", base_url);

        let mut tasks = Vec::with_capacity(50);
        for _ in 0..50 {
            let client = client.clone();
            let url = health_url.clone();
            tasks.push(async move {
                let resp = timeout(Duration::from_secs(5), client.get(url).send()).await??;
                anyhow::Result::<StatusCode>::Ok(resp.status())
            });
        }

        let statuses = try_join_all(tasks).await?;
        assert!(statuses.into_iter().all(|code| code == StatusCode::OK));

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_requires_no_auth_and_closes() -> Result<()> {
        init_test_tracing();
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), base_config());
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
    async fn rpc_body_within_limit_allows_200() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.max_body_bytes = 131_072;
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let body = vec![b'a'; 64 * 1024];
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_max_body_returns_413() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.max_body_bytes = 131_072;
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let oversized = vec![b'a'; 256 * 1024];
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
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
    async fn rpc_without_auth_passes() -> Result<()> {
        init_test_tracing();
        let config = base_config();
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_concurrency_smoke() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 20;
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#.to_string();

        let mut tasks = Vec::with_capacity(10);
        for _ in 0..10 {
            let client = client.clone();
            let url = rpc_endpoint.clone();
            let body = body.clone();
            tasks.push(async move {
                let resp = timeout(
                    Duration::from_secs(5),
                    client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .body(body)
                        .send(),
                )
                .await??;
                anyhow::Result::<String>::Ok(resp.text().await?)
            });
        }

        let responses = try_join_all(tasks).await?;
        for text in responses {
            assert!(text.contains("100"));
        }

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_rate_limit_429_when_exceeded() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 5;
        config.rl_refill_per_sec = 0;
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let mut statuses = Vec::new();
        for _ in 0..10 {
            let resp = timeout(
                Duration::from_secs(5),
                client
                    .post(&rpc_endpoint)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send(),
            )
            .await??;
            statuses.push(resp.status());
        }

        let ok = statuses.iter().filter(|&&s| s == StatusCode::OK).count();
        let limited = statuses
            .iter()
            .filter(|&&s| s == StatusCode::TOO_MANY_REQUESTS)
            .count();
        assert_eq!(ok, 5);
        assert_eq!(limited, 5);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_rate_limit_recovers_after_time() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 2;
        config.rl_refill_per_sec = 4;
        let server = RpcServer::without_auth(TestHandler, "127.0.0.1:0".to_string(), config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;

        for _ in 0..2 {
            let resp = timeout(
                Duration::from_secs(5),
                client
                    .post(&rpc_endpoint)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send(),
            )
            .await??;
            assert_eq!(resp.status(), StatusCode::OK);
        }

        let third = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);

        sleep(Duration::from_millis(300)).await;

        let fourth = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(fourth.status(), StatusCode::OK);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }
}

/// Validate Host header against whitelist (DNS rebinding protection)
fn validate_host_header(host: Option<&str>, config: &RpcConfig) -> bool {
    if !config.enforce_host_validation {
        return true;
    }

    let Some(host_value) = host else {
        return false; // Missing Host header
    };

    // Extract hostname without port
    let hostname = host_value.split(':').next().unwrap_or(host_value);

    config
        .allowed_hosts
        .iter()
        .any(|allowed| allowed == hostname || allowed == host_value)
}

/// Validate Origin header (CORS protection)
fn validate_origin_header(origin: Option<&str>, config: &RpcConfig) -> bool {
    if config.allowed_origins.is_empty() {
        return true; // No restrictions if list is empty
    }

    let Some(origin_value) = origin else {
        return true; // Allow if no Origin header (same-origin requests)
    };

    config
        .allowed_origins
        .iter()
        .any(|allowed| allowed == origin_value)
}

#[cfg(test)]
mod dns_rebinding_tests {
    use super::*;

    #[test]
    fn test_validate_host_allowed() {
        let config = RpcConfig {
            allowed_hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            enforce_host_validation: true,
            ..Default::default()
        };

        assert!(validate_host_header(Some("localhost"), &config));
        assert!(validate_host_header(Some("127.0.0.1"), &config));
        assert!(validate_host_header(Some("localhost:8332"), &config));
    }

    #[test]
    fn test_validate_host_rejected() {
        let config = RpcConfig {
            allowed_hosts: vec!["localhost".to_string()],
            enforce_host_validation: true,
            ..Default::default()
        };

        assert!(!validate_host_header(Some("evil.com"), &config));
        assert!(!validate_host_header(None, &config));
    }

    #[test]
    fn test_validate_origin() {
        let config = RpcConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };

        assert!(validate_origin_header(Some("https://example.com"), &config));
        assert!(!validate_origin_header(Some("https://evil.com"), &config));
        assert!(validate_origin_header(None, &config)); // Same-origin OK
    }
}
